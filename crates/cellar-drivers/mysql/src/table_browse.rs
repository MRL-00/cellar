use std::time::Instant;

use cellar_core::error::CellarError;
use cellar_core::error::CellarResult;
use cellar_core::query::{
    NoticeCapture, QueryResult, SortDirection, TableBrowseRequest, TableFilterClause,
    TableFilterOperator,
};
use cellar_core::schema::{Column, Table};
use cellar_core::value::{ColumnMeta, Row};
use futures::TryStreamExt;
use sqlx::{Column as _, MySql, QueryBuilder, Row as _, TypeInfo as _};
use thiserror::Error;

use crate::connect::MySqlConnection;
use crate::decode::decode_cell;

const DEFAULT_TABLE_BROWSE_ROWS: u32 = 500;
const MAX_TABLE_BROWSE_ROWS: u32 = 2000;

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
    #[error("table browse limit cannot exceed {MAX_TABLE_BROWSE_ROWS} rows per page")]
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
    conn: &MySqlConnection,
    request: &TableBrowseRequest,
    table: &Table,
) -> CellarResult<QueryResult> {
    let max_rows = normalized_limit(request).map_err(to_query_err)?;
    let pool = conn.pool();

    let total_rows: Option<u64> = if request.include_total {
        let count: i64 = build_table_count_query(request, table)
            .map_err(to_query_err)?
            .build_query_scalar()
            .fetch_one(pool)
            .await
            .map_err(|e| CellarError::query(e.to_string()))?;
        Some(count.max(0) as u64)
    } else {
        None
    };

    let mut builder = build_table_browse_query(request, table).map_err(to_query_err)?;
    let started = Instant::now();
    let mut stream = builder.build().fetch(pool);
    let mut columns: Option<Vec<ColumnMeta>> = None;
    let mut materialized: Vec<Row> = Vec::with_capacity(max_rows as usize);
    let mut truncated = false;

    while let Some(r) = stream
        .try_next()
        .await
        .map_err(|e| CellarError::query(e.to_string()))?
    {
        if columns.is_none() {
            columns = Some(
                r.columns()
                    .iter()
                    .map(|c| ColumnMeta {
                        name: c.name().to_string(),
                        data_type: c.type_info().name().to_string().to_lowercase(),
                        nullable: true,
                    })
                    .collect(),
            );
        }

        if materialized.len() >= max_rows as usize {
            truncated = true;
            break;
        }

        let mut cells: Row = Vec::with_capacity(r.columns().len());
        for i in 0..r.columns().len() {
            cells.push(decode_cell(&r, i)?);
        }
        materialized.push(cells);
    }

    Ok(QueryResult {
        columns: columns.unwrap_or_default(),
        rows: materialized,
        notices: Vec::new(),
        notice_capture: NoticeCapture::unsupported(
            "MySQL does not expose server notices through the sqlx query path.",
        ),
        rows_affected: None,
        duration_ms: started.elapsed().as_millis() as u64,
        truncated,
        total_rows,
    })
}

fn build_table_count_query<'args>(
    request: &'args TableBrowseRequest,
    table: &Table,
) -> Result<QueryBuilder<'args, MySql>, TableBrowseError> {
    for filter in &request.filters {
        column_for(table, &filter.column)?;
    }

    let mut builder = QueryBuilder::<MySql>::new("SELECT count(*) FROM ");
    push_qualified_table(&mut builder, &request.schema, &request.table);

    if !request.filters.is_empty() {
        builder.push(" WHERE ");
        for (i, filter) in request.filters.iter().enumerate() {
            if i > 0 {
                builder.push(" AND ");
            }
            push_filter(&mut builder, filter, table)?;
        }
    }

    Ok(builder)
}

fn build_table_browse_query<'args>(
    request: &'args TableBrowseRequest,
    table: &Table,
) -> Result<QueryBuilder<'args, MySql>, TableBrowseError> {
    validate_table_request(request, table)?;

    let mut builder = QueryBuilder::<MySql>::new("SELECT * FROM ");
    push_qualified_table(&mut builder, &request.schema, &request.table);

    if !request.filters.is_empty() {
        builder.push(" WHERE ");
        for (i, filter) in request.filters.iter().enumerate() {
            if i > 0 {
                builder.push(" AND ");
            }
            push_filter(&mut builder, filter, table)?;
        }
    }

    if !request.sorts.is_empty() {
        builder.push(" ORDER BY ");
        for (i, sort) in request.sorts.iter().enumerate() {
            if i > 0 {
                builder.push(", ");
            }
            let column = column_for(table, &sort.column)?;
            push_ident(&mut builder, &column.name);
            match sort.direction {
                SortDirection::Asc => builder.push(" ASC"),
                SortDirection::Desc => builder.push(" DESC"),
            };
        }
    } else if request.primary_key_fallback_ordering && !table.primary_key.is_empty() {
        builder.push(" ORDER BY ");
        for (i, column_name) in table.primary_key.iter().enumerate() {
            if i > 0 {
                builder.push(", ");
            }
            let column = column_for(table, column_name)?;
            push_ident(&mut builder, &column.name);
        }
    }

    let fetch_limit = normalized_limit(request)? + 1;
    builder.push(" LIMIT ");
    builder.push_bind(fetch_limit as i64);
    if let Some(offset) = normalized_offset(request)? {
        builder.push(" OFFSET ");
        builder.push_bind(offset as i64);
    }

    Ok(builder)
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

fn push_filter<'args>(
    builder: &mut QueryBuilder<'args, MySql>,
    filter: &'args TableFilterClause,
    table: &Table,
) -> Result<(), TableBrowseError> {
    let column = column_for(table, &filter.column)?;
    let kind = ColumnKind::from_mysql_type(&column.data_type);

    match filter.operator {
        TableFilterOperator::IsNull => {
            reject_value(filter)?;
            push_ident(builder, &column.name);
            builder.push(" IS NULL");
        }
        TableFilterOperator::IsNotNull => {
            reject_value(filter)?;
            push_ident(builder, &column.name);
            builder.push(" IS NOT NULL");
        }
        TableFilterOperator::Contains => {
            let value = require_value(filter)?;
            if !matches!(kind, ColumnKind::Text) {
                return Err(unsupported(column, filter.operator));
            }
            push_ident(builder, &column.name);
            builder.push(" LIKE CONCAT('%', ");
            builder.push_bind(value);
            builder.push(", '%')");
        }
        TableFilterOperator::Equals | TableFilterOperator::NotEquals => {
            let value = require_value(filter)?;
            let operator = if filter.operator == TableFilterOperator::Equals {
                " = "
            } else {
                " <> "
            };
            push_ident(builder, &column.name);
            builder.push(operator);
            builder.push_bind(value);
        }
        TableFilterOperator::GreaterThan
        | TableFilterOperator::GreaterThanOrEqual
        | TableFilterOperator::LessThan
        | TableFilterOperator::LessThanOrEqual => {
            let value = require_value(filter)?;
            // Range comparisons are valid for numeric and temporal columns;
            // MySQL coerces the bound string for DATE/TIME/DATETIME.
            if !matches!(kind, ColumnKind::Numeric | ColumnKind::Temporal) {
                return Err(unsupported(column, filter.operator));
            }
            let operator = match filter.operator {
                TableFilterOperator::GreaterThan => " > ",
                TableFilterOperator::GreaterThanOrEqual => " >= ",
                TableFilterOperator::LessThan => " < ",
                TableFilterOperator::LessThanOrEqual => " <= ",
                _ => unreachable!(),
            };
            push_ident(builder, &column.name);
            builder.push(operator);
            builder.push_bind(value);
        }
    }

    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ColumnKind {
    Numeric,
    Text,
    Temporal,
    Other,
}

impl ColumnKind {
    /// Classify a column from its `information_schema.COLUMN_TYPE`, e.g.
    /// `"varchar(255)"`, `"int(10) unsigned"`, `"decimal(10,2)"`,
    /// `"enum('a','b')"`. We key off the leading base-type token, ignoring the
    /// length/enum-value parenthetical and the trailing `unsigned`/`zerofill`
    /// attributes.
    fn from_mysql_type(type_name: &str) -> Self {
        let lowered = type_name.to_ascii_lowercase();
        let base = lowered
            .split(['(', ' '])
            .next()
            .unwrap_or(lowered.as_str());
        match base {
            "tinyint" | "smallint" | "mediumint" | "int" | "integer" | "bigint" | "float"
            | "double" | "real" | "decimal" | "numeric" | "dec" | "fixed" | "bit" | "year" => {
                ColumnKind::Numeric
            }
            "char" | "varchar" | "text" | "tinytext" | "mediumtext" | "longtext" | "enum"
            | "set" => ColumnKind::Text,
            "date" | "time" | "datetime" | "timestamp" => ColumnKind::Temporal,
            _ => ColumnKind::Other,
        }
    }
}

fn column_for<'a>(table: &'a Table, column_name: &str) -> Result<&'a Column, TableBrowseError> {
    table
        .columns
        .iter()
        .find(|c| c.name == column_name)
        .ok_or_else(|| TableBrowseError::UnknownColumn(column_name.to_string()))
}

fn require_value(filter: &TableFilterClause) -> Result<&str, TableBrowseError> {
    filter.value.as_deref().ok_or_else(|| TableBrowseError::MissingValue {
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

fn push_ident<'args>(builder: &mut QueryBuilder<'args, MySql>, ident: &str) {
    builder.push('`');
    builder.push(ident.replace('`', "``"));
    builder.push('`');
}

fn push_qualified_table<'args>(
    builder: &mut QueryBuilder<'args, MySql>,
    schema: &str,
    table: &str,
) {
    push_ident(builder, schema);
    builder.push('.');
    push_ident(builder, table);
}

fn to_query_err(err: TableBrowseError) -> CellarError {
    CellarError::query(err.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use cellar_core::query::TableSortClause;

    fn request() -> TableBrowseRequest {
        TableBrowseRequest {
            connection_id: "conn".into(),
            database: Some("app".into()),
            schema: "app".into(),
            table: "users".into(),
            limit: Some(500),
            offset: None,
            sorts: Vec::new(),
            filters: Vec::new(),
            primary_key_fallback_ordering: true,
            include_total: false,
        }
    }

    fn table() -> Table {
        Table {
            name: "users".into(),
            schema: "app".into(),
            row_count: None,
            primary_key: vec!["id".into()],
            foreign_keys: Vec::new(),
            indexes: Vec::new(),
            columns: vec![
                column("id", "bigint(20)", true),
                column("email", "varchar(255)", false),
                column("age", "int(11)", false),
                column("created_at", "datetime", false),
            ],
        }
    }

    fn column(name: &str, data_type: &str, primary: bool) -> Column {
        Column {
            name: name.into(),
            data_type: data_type.into(),
            nullable: true,
            default: None,
            is_primary_key: primary,
            ordinal: 1,
            comment: None,
        }
    }

    fn sql_for(request: &TableBrowseRequest) -> Result<String, TableBrowseError> {
        Ok(build_table_browse_query(request, &table())?
            .sql()
            .to_string())
    }

    #[test]
    fn backtick_quotes_and_uses_primary_key_fallback_ordering() {
        let sql = sql_for(&request()).expect("sql");
        assert_eq!(sql, "SELECT * FROM `app`.`users` ORDER BY `id` LIMIT ?");
    }

    #[test]
    fn doubles_embedded_backticks() {
        let mut req = request();
        req.schema = "odd`schema".into();
        req.table = "user`data".into();
        let mut meta = table();
        meta.schema = req.schema.clone();
        meta.name = req.table.clone();

        let sql = build_table_browse_query(&req, &meta)
            .expect("sql")
            .sql()
            .to_string();
        assert_eq!(
            sql,
            "SELECT * FROM `odd``schema`.`user``data` ORDER BY `id` LIMIT ?"
        );
    }

    #[test]
    fn binds_contains_value_instead_of_inlining() {
        let mut req = request();
        req.filters.push(TableFilterClause {
            column: "email".into(),
            operator: TableFilterOperator::Contains,
            value: Some("o'reilly".into()),
        });
        let sql = sql_for(&req).expect("sql");
        assert_eq!(
            sql,
            "SELECT * FROM `app`.`users` WHERE `email` LIKE CONCAT('%', ?, '%') ORDER BY `id` LIMIT ?"
        );
        assert!(!sql.contains("o'reilly"));
    }

    #[test]
    fn allows_range_comparison_on_temporal_column() {
        let mut req = request();
        req.filters.push(TableFilterClause {
            column: "created_at".into(),
            operator: TableFilterOperator::GreaterThan,
            value: Some("2024-01-01".into()),
        });
        let sql = sql_for(&req).expect("sql");
        assert_eq!(
            sql,
            "SELECT * FROM `app`.`users` WHERE `created_at` > ? ORDER BY `id` LIMIT ?"
        );
    }

    #[test]
    fn comparison_on_numeric_column_with_is_null_and_sort() {
        let mut req = request();
        req.filters.push(TableFilterClause {
            column: "age".into(),
            operator: TableFilterOperator::GreaterThanOrEqual,
            value: Some("21".into()),
        });
        req.filters.push(TableFilterClause {
            column: "created_at".into(),
            operator: TableFilterOperator::IsNull,
            value: None,
        });
        req.sorts.push(TableSortClause {
            column: "email".into(),
            direction: SortDirection::Desc,
        });
        let sql = sql_for(&req).expect("sql");
        assert_eq!(
            sql,
            "SELECT * FROM `app`.`users` WHERE `age` >= ? AND `created_at` IS NULL ORDER BY `email` DESC LIMIT ?"
        );
    }

    #[test]
    fn rejects_comparison_on_text_column() {
        let mut req = request();
        req.filters.push(TableFilterClause {
            column: "email".into(),
            operator: TableFilterOperator::GreaterThan,
            value: Some("a".into()),
        });
        let err = match build_table_browse_query(&req, &table()) {
            Err(e) => e,
            Ok(_) => panic!("expected the comparison to be rejected"),
        };
        assert!(matches!(
            err,
            TableBrowseError::UnsupportedOperator { .. }
        ));
    }

    #[test]
    fn rejects_unknown_column() {
        let mut req = request();
        req.filters.push(TableFilterClause {
            column: "ghost".into(),
            operator: TableFilterOperator::Equals,
            value: Some("x".into()),
        });
        let err = match build_table_browse_query(&req, &table()) {
            Err(e) => e,
            Ok(_) => panic!("expected the unknown column to be rejected"),
        };
        assert!(matches!(err, TableBrowseError::UnknownColumn(c) if c == "ghost"));
    }

    #[test]
    fn count_query_omits_limit_and_order() {
        let mut req = request();
        req.filters.push(TableFilterClause {
            column: "age".into(),
            operator: TableFilterOperator::Equals,
            value: Some("21".into()),
        });
        let sql = build_table_count_query(&req, &table())
            .expect("sql")
            .sql()
            .to_string();
        assert_eq!(
            sql,
            "SELECT count(*) FROM `app`.`users` WHERE `age` = ?"
        );
        assert!(!sql.contains("LIMIT"));
        assert!(!sql.contains("ORDER BY"));
    }

    #[test]
    fn column_kind_parses_column_type_modifiers() {
        assert_eq!(ColumnKind::from_mysql_type("varchar(255)"), ColumnKind::Text);
        assert_eq!(
            ColumnKind::from_mysql_type("int(10) unsigned"),
            ColumnKind::Numeric
        );
        assert_eq!(
            ColumnKind::from_mysql_type("decimal(10,2)"),
            ColumnKind::Numeric
        );
        assert_eq!(
            ColumnKind::from_mysql_type("enum('a','b')"),
            ColumnKind::Text
        );
        assert_eq!(ColumnKind::from_mysql_type("datetime"), ColumnKind::Temporal);
    }
}
