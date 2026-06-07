use std::time::Instant;

use cellar_core::error::{CellarError, CellarResult};
use cellar_core::query::{
    NoticeCapture, QueryResult, SortDirection, TableBrowseRequest, TableFilterClause,
    TableFilterOperator,
};
use cellar_core::schema::{Column, Table};
use cellar_core::value::{ColumnMeta, Row};
use futures_util::TryStreamExt;
use thiserror::Error;
use tiberius::QueryItem;

use crate::connect::{map_tiberius_runtime_err, SqlServerConnection};
use crate::decode::decode_cell;

const DEFAULT_TABLE_BROWSE_ROWS: u32 = 500;
const MAX_TABLE_BROWSE_ROWS: u32 = 500;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum TableBrowseError {
    #[error("schema name is empty")]
    EmptySchema,
    #[error("table name is empty")]
    EmptyTable,
    #[error("table metadata does not match requested table")]
    TableMetadataMismatch,
    #[error("table browse limit must be greater than zero")]
    EmptyLimit,
    #[error("table browse limit cannot exceed {MAX_TABLE_BROWSE_ROWS} rows")]
    LimitTooLarge,
    #[error("table browse offset is too large")]
    OffsetTooLarge,
    #[error("unknown column {0}")]
    UnknownColumn(String),
    #[error("operator {operator:?} requires a value for column {column}")]
    MissingValue {
        column: String,
        operator: TableFilterOperator,
    },
    #[error("operator {operator:?} does not accept a value for column {column}")]
    UnexpectedValue {
        column: String,
        operator: TableFilterOperator,
    },
    #[error("operator {operator:?} is not supported for column {column} ({data_type})")]
    UnsupportedOperator {
        column: String,
        data_type: String,
        operator: TableFilterOperator,
    },
}

pub async fn browse_table(
    conn: &SqlServerConnection,
    request: &TableBrowseRequest,
    table: &Table,
) -> CellarResult<QueryResult> {
    let max_rows = normalized_limit(request).map_err(to_query_err)?;
    let sql = build_table_browse_query(request, table).map_err(to_query_err)?;
    let started = Instant::now();

    conn.with_client(async |client| {
        let mut stream = client
            .simple_query(sql)
            .await
            .map_err(|e| map_tiberius_runtime_err(e, "table browse"))?;
        let mut columns: Option<Vec<ColumnMeta>> = None;
        let mut rows: Vec<Row> = Vec::with_capacity(max_rows as usize);
        let mut truncated = false;

        while let Some(item) = stream
            .try_next()
            .await
            .map_err(|e| map_tiberius_runtime_err(e, "table browse"))?
        {
            match item {
                QueryItem::Metadata(meta) if columns.is_none() => {
                    columns = Some(
                        meta.columns()
                            .iter()
                            .map(|c| ColumnMeta {
                                name: c.name().to_string(),
                                data_type: format!("{:?}", c.column_type()).to_lowercase(),
                                nullable: true,
                            })
                            .collect(),
                    );
                }
                QueryItem::Row(row) => {
                    if rows.len() >= max_rows as usize {
                        truncated = true;
                        continue;
                    }
                    rows.push(row.into_iter().map(decode_cell).collect());
                }
                _ => {}
            }
        }

        Ok(QueryResult {
            columns: columns.unwrap_or_default(),
            rows,
            notices: Vec::new(),
            notice_capture: NoticeCapture::unsupported(
                "SQL Server informational messages are not exposed through the current tiberius query path.",
            ),
            rows_affected: None,
            duration_ms: started.elapsed().as_millis() as u64,
            truncated,
        })
    })
    .await
}

fn build_table_browse_query(
    request: &TableBrowseRequest,
    table: &Table,
) -> Result<String, TableBrowseError> {
    validate_table_request(request, table)?;

    let mut sql = format!(
        "SELECT * FROM {}.{}",
        quote_ident(&request.schema),
        quote_ident(&request.table)
    );

    if !request.filters.is_empty() {
        sql.push_str(" WHERE ");
        let filters = request
            .filters
            .iter()
            .map(|filter| filter_sql(filter, table))
            .collect::<Result<Vec<_>, _>>()?;
        sql.push_str(&filters.join(" AND "));
    }

    if !request.sorts.is_empty() {
        sql.push_str(" ORDER BY ");
        let sorts = request
            .sorts
            .iter()
            .map(|sort| {
                let column = column_for(table, &sort.column)?;
                let direction = match sort.direction {
                    SortDirection::Asc => "ASC",
                    SortDirection::Desc => "DESC",
                };
                Ok(format!("{} {direction}", quote_ident(&column.name)))
            })
            .collect::<Result<Vec<_>, TableBrowseError>>()?;
        sql.push_str(&sorts.join(", "));
    } else if request.primary_key_fallback_ordering && !table.primary_key.is_empty() {
        sql.push_str(" ORDER BY ");
        let pk = table
            .primary_key
            .iter()
            .map(|column| column_for(table, column).map(|c| quote_ident(&c.name)))
            .collect::<Result<Vec<_>, _>>()?;
        sql.push_str(&pk.join(", "));
    } else {
        sql.push_str(" ORDER BY (SELECT 0)");
    }

    let fetch_limit = normalized_limit(request)? + 1;
    let offset = normalized_offset(request)?.unwrap_or(0);
    sql.push_str(&format!(
        " OFFSET {offset} ROWS FETCH NEXT {fetch_limit} ROWS ONLY"
    ));
    Ok(sql)
}

fn filter_sql(filter: &TableFilterClause, table: &Table) -> Result<String, TableBrowseError> {
    let column = column_for(table, &filter.column)?;
    let ident = quote_ident(&column.name);
    match filter.operator {
        TableFilterOperator::IsNull => {
            reject_value(filter)?;
            Ok(format!("{ident} IS NULL"))
        }
        TableFilterOperator::IsNotNull => {
            reject_value(filter)?;
            Ok(format!("{ident} IS NOT NULL"))
        }
        TableFilterOperator::Contains => {
            let value = require_value(filter)?;
            if !is_text_type(&column.data_type) {
                return Err(unsupported(column, filter.operator));
            }
            Ok(format!(
                "{ident} LIKE {}",
                quote_literal(&format!("%{value}%"))
            ))
        }
        TableFilterOperator::Equals | TableFilterOperator::NotEquals => {
            let value = require_value(filter)?;
            let op = if filter.operator == TableFilterOperator::Equals {
                "="
            } else {
                "<>"
            };
            Ok(format!("{ident} {op} {}", quote_literal(&value)))
        }
        TableFilterOperator::GreaterThan
        | TableFilterOperator::GreaterThanOrEqual
        | TableFilterOperator::LessThan
        | TableFilterOperator::LessThanOrEqual => {
            let value = require_value(filter)?;
            if !supports_ordering(&column.data_type) {
                return Err(unsupported(column, filter.operator));
            }
            Ok(format!(
                "{ident} {} {}",
                comparison_operator(filter.operator),
                quote_literal(&value)
            ))
        }
    }
}

fn validate_table_request(
    request: &TableBrowseRequest,
    table: &Table,
) -> Result<(), TableBrowseError> {
    if request.schema.trim().is_empty() {
        return Err(TableBrowseError::EmptySchema);
    }
    if request.table.trim().is_empty() {
        return Err(TableBrowseError::EmptyTable);
    }
    if table.schema != request.schema || table.name != request.table {
        return Err(TableBrowseError::TableMetadataMismatch);
    }
    normalized_limit(request)?;
    normalized_offset(request)?;
    for sort in &request.sorts {
        column_for(table, &sort.column)?;
    }
    for filter in &request.filters {
        column_for(table, &filter.column)?;
    }
    Ok(())
}

fn normalized_limit(request: &TableBrowseRequest) -> Result<u32, TableBrowseError> {
    let limit = request.limit.unwrap_or(DEFAULT_TABLE_BROWSE_ROWS);
    if limit == 0 {
        return Err(TableBrowseError::EmptyLimit);
    }
    if limit > MAX_TABLE_BROWSE_ROWS {
        return Err(TableBrowseError::LimitTooLarge);
    }
    Ok(limit)
}

fn normalized_offset(request: &TableBrowseRequest) -> Result<Option<u32>, TableBrowseError> {
    match request.offset {
        Some(offset) if offset > i32::MAX as u32 => Err(TableBrowseError::OffsetTooLarge),
        Some(0) | None => Ok(None),
        Some(offset) => Ok(Some(offset)),
    }
}

fn column_for<'a>(table: &'a Table, name: &str) -> Result<&'a Column, TableBrowseError> {
    table
        .columns
        .iter()
        .find(|column| column.name == name)
        .ok_or_else(|| TableBrowseError::UnknownColumn(name.to_string()))
}

fn require_value(filter: &TableFilterClause) -> Result<String, TableBrowseError> {
    filter
        .value
        .clone()
        .ok_or_else(|| TableBrowseError::MissingValue {
            column: filter.column.clone(),
            operator: filter.operator,
        })
}

fn reject_value(filter: &TableFilterClause) -> Result<(), TableBrowseError> {
    if filter.value.is_some() {
        return Err(TableBrowseError::UnexpectedValue {
            column: filter.column.clone(),
            operator: filter.operator,
        });
    }
    Ok(())
}

fn unsupported(column: &Column, operator: TableFilterOperator) -> TableBrowseError {
    TableBrowseError::UnsupportedOperator {
        column: column.name.clone(),
        data_type: column.data_type.clone(),
        operator,
    }
}

fn comparison_operator(operator: TableFilterOperator) -> &'static str {
    match operator {
        TableFilterOperator::GreaterThan => ">",
        TableFilterOperator::GreaterThanOrEqual => ">=",
        TableFilterOperator::LessThan => "<",
        TableFilterOperator::LessThanOrEqual => "<=",
        _ => unreachable!("comparison_operator called for non-comparison filter"),
    }
}

fn quote_ident(ident: &str) -> String {
    format!("[{}]", ident.replace(']', "]]"))
}

fn quote_literal(value: &str) -> String {
    format!("N'{}'", value.replace('\'', "''"))
}

fn is_text_type(data_type: &str) -> bool {
    let lower = data_type.to_lowercase();
    lower.contains("char") || lower == "text" || lower == "ntext" || lower == "xml"
}

fn supports_ordering(data_type: &str) -> bool {
    let lower = data_type.to_lowercase();
    !matches!(
        lower.as_str(),
        "bit" | "image" | "text" | "ntext" | "xml" | "geography" | "geometry" | "hierarchyid"
    )
}

fn to_query_err(err: TableBrowseError) -> CellarError {
    CellarError::query(err.to_string())
}

#[cfg(test)]
mod tests {
    use cellar_core::query::{SortDirection, TableFilterClause, TableSortClause};

    use super::*;

    #[test]
    fn quotes_identifiers_and_uses_primary_key_fallback_ordering() {
        let table = table();
        let request = TableBrowseRequest {
            connection_id: "conn".into(),
            database: Some("main".into()),
            schema: "dbo".into(),
            table: "orders]archive".into(),
            limit: Some(50),
            offset: None,
            sorts: Vec::new(),
            filters: Vec::new(),
            primary_key_fallback_ordering: true,
        };

        let sql = build_table_browse_query(&request, &table).unwrap();

        assert_eq!(
            sql,
            "SELECT * FROM [dbo].[orders]]archive] ORDER BY [id] OFFSET 0 ROWS FETCH NEXT 51 ROWS ONLY"
        );
    }

    #[test]
    fn escapes_filter_literals() {
        let table = table();
        let request = TableBrowseRequest {
            connection_id: "conn".into(),
            database: Some("main".into()),
            schema: "dbo".into(),
            table: "orders]archive".into(),
            limit: Some(25),
            offset: Some(10),
            sorts: vec![TableSortClause {
                column: "name".into(),
                direction: SortDirection::Desc,
            }],
            filters: vec![TableFilterClause {
                column: "name".into(),
                operator: TableFilterOperator::Contains,
                value: Some("O'Hara".into()),
            }],
            primary_key_fallback_ordering: true,
        };

        let sql = build_table_browse_query(&request, &table).unwrap();

        assert_eq!(
            sql,
            "SELECT * FROM [dbo].[orders]]archive] WHERE [name] LIKE N'%O''Hara%' ORDER BY [name] DESC OFFSET 10 ROWS FETCH NEXT 26 ROWS ONLY"
        );
    }

    #[test]
    fn rejects_unknown_columns() {
        let mut request = TableBrowseRequest {
            connection_id: "conn".into(),
            database: Some("main".into()),
            schema: "dbo".into(),
            table: "orders]archive".into(),
            limit: Some(25),
            offset: None,
            sorts: Vec::new(),
            filters: Vec::new(),
            primary_key_fallback_ordering: true,
        };
        request.sorts.push(TableSortClause {
            column: "missing".into(),
            direction: SortDirection::Asc,
        });

        let err = build_table_browse_query(&request, &table()).unwrap_err();

        assert_eq!(err, TableBrowseError::UnknownColumn("missing".into()));
    }

    fn table() -> Table {
        Table {
            name: "orders]archive".into(),
            schema: "dbo".into(),
            row_count: None,
            columns: vec![
                Column {
                    name: "id".into(),
                    data_type: "int".into(),
                    nullable: false,
                    default: None,
                    is_primary_key: true,
                    ordinal: 1,
                    comment: None,
                },
                Column {
                    name: "name".into(),
                    data_type: "nvarchar(100)".into(),
                    nullable: true,
                    default: None,
                    is_primary_key: false,
                    ordinal: 2,
                    comment: None,
                },
            ],
            primary_key: vec!["id".into()],
            foreign_keys: Vec::new(),
            indexes: Vec::new(),
        }
    }
}
