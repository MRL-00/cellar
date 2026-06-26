use std::time::Instant;

use cellar_core::error::{CellarError, CellarResult};
use cellar_core::query::{
    NoticeCapture, QueryResult, SortDirection, TableBrowseRequest, TableFilterClause,
    TableFilterOperator,
};
use cellar_core::schema::Table;
use cellar_core::table_browse::{
    column_for, normalized_limit, normalized_offset, reject_value, require_value, unsupported,
    validate_table_request, TableBrowseError,
};
use cellar_core::value::{ColumnMeta, Row};
use futures_util::TryStreamExt;
use tiberius::QueryItem;

use crate::connect::{map_tiberius_runtime_err, SqlServerConnection};
use crate::decode::decode_cell;

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
                        // SQL Server browse uses server-side OFFSET/FETCH, so
                        // the server already caps the result set. This branch
                        // can only be reached when the client fetched limit+1
                        // rows (to detect truncation). Break rather than
                        // continue to avoid transferring stray rows.
                        break;
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
            // TODO: add include_total support for SQL Server once the shared
            // filter-building code is refactored to support parameterized
            // count queries (tiberius does not use sqlx QueryBuilder).
            total_rows: None,
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
    use cellar_core::schema::Column;

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
            include_total: false,
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
            include_total: false,
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
            include_total: false,
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
