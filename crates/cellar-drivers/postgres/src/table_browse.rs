use std::time::Instant;

use cellar_core::error::{CellarError, CellarResult};
use cellar_core::query::{
    NoticeCapture, QueryResult, SortDirection, TableBrowseRequest, TableFilterClause,
    TableFilterOperator,
};
use cellar_core::schema::{Column, Table};
use cellar_core::value::{ColumnMeta, Row};
use futures::TryStreamExt;
use sqlx::{Column as _, Postgres, QueryBuilder, Row as _, TypeInfo as _};
use thiserror::Error;

use crate::connect::PgConnection;
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
    conn: &PgConnection,
    request: &TableBrowseRequest,
    table: &Table,
) -> CellarResult<QueryResult> {
    let max_rows = normalized_limit(request).map_err(to_query_err)?;
    let database = request
        .database
        .as_deref()
        .unwrap_or(conn.config().database.as_str());
    let pool = conn.pool_for_database(database).await?;
    let pool = &pool;

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
            "Postgres server notices are parsed by sqlx, but the current PgPool query path consumes NoticeResponse frames internally and exposes only log/tracing output without SQLSTATE, detail, hint, or query correlation.",
        ),
        rows_affected: None,
        duration_ms: started.elapsed().as_millis() as u64,
        truncated,
    })
}

fn build_table_browse_query<'args>(
    request: &TableBrowseRequest,
    table: &Table,
) -> Result<QueryBuilder<'args, Postgres>, TableBrowseError> {
    validate_table_request(request, table)?;

    let mut builder = QueryBuilder::<Postgres>::new("SELECT * FROM ");
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

fn push_filter<'args>(
    builder: &mut QueryBuilder<'args, Postgres>,
    filter: &TableFilterClause,
    table: &Table,
) -> Result<(), TableBrowseError> {
    let column = column_for(table, &filter.column)?;
    let kind = ColumnKind::from_pg_type(&column.data_type);

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
            builder.push(" ILIKE ('%' || ");
            builder.push_bind(value);
            builder.push(" || '%')");
        }
        TableFilterOperator::Equals | TableFilterOperator::NotEquals => {
            let value = require_value(filter)?;
            let operator = if filter.operator == TableFilterOperator::Equals {
                " = "
            } else {
                " <> "
            };
            push_typed_comparison(builder, column, kind, operator, value, filter.operator)?;
        }
        TableFilterOperator::GreaterThan
        | TableFilterOperator::GreaterThanOrEqual
        | TableFilterOperator::LessThan
        | TableFilterOperator::LessThanOrEqual => {
            let value = require_value(filter)?;
            if !kind.supports_ordering() {
                return Err(unsupported(column, filter.operator));
            }
            push_typed_comparison(
                builder,
                column,
                kind,
                comparison_operator(filter.operator),
                value,
                filter.operator,
            )?;
        }
    }

    Ok(())
}

fn push_typed_comparison<'args>(
    builder: &mut QueryBuilder<'args, Postgres>,
    column: &Column,
    kind: ColumnKind,
    operator: &str,
    value: String,
    filter_operator: TableFilterOperator,
) -> Result<(), TableBrowseError> {
    if matches!(kind, ColumnKind::Unsupported) {
        return Err(unsupported(column, filter_operator));
    }

    push_ident(builder, &column.name);
    builder.push(operator);
    builder.push_bind(value);
    if let Some(cast) = kind.parameter_cast() {
        builder.push("::");
        builder.push(cast);
    }
    Ok(())
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

fn comparison_operator(operator: TableFilterOperator) -> &'static str {
    match operator {
        TableFilterOperator::GreaterThan => " > ",
        TableFilterOperator::GreaterThanOrEqual => " >= ",
        TableFilterOperator::LessThan => " < ",
        TableFilterOperator::LessThanOrEqual => " <= ",
        _ => unreachable!("comparison_operator called for non-comparison filter"),
    }
}

fn column_for<'a>(table: &'a Table, name: &str) -> Result<&'a Column, TableBrowseError> {
    table
        .columns
        .iter()
        .find(|column| column.name == name)
        .ok_or_else(|| TableBrowseError::UnknownColumn(name.to_string()))
}

fn unsupported(column: &Column, operator: TableFilterOperator) -> TableBrowseError {
    TableBrowseError::UnsupportedOperator {
        column: column.name.clone(),
        data_type: column.data_type.clone(),
        operator,
    }
}

fn push_qualified_table(builder: &mut QueryBuilder<'_, Postgres>, schema: &str, table: &str) {
    push_ident(builder, schema);
    builder.push(".");
    push_ident(builder, table);
}

fn push_ident(builder: &mut QueryBuilder<'_, Postgres>, ident: &str) {
    builder.push("\"");
    builder.push(ident.replace('"', "\"\""));
    builder.push("\"");
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ColumnKind {
    Text,
    Typed(&'static str, bool),
    Unsupported,
}

impl ColumnKind {
    fn from_pg_type(data_type: &str) -> Self {
        match data_type.to_ascii_lowercase().as_str() {
            "text" | "varchar" | "bpchar" | "char" | "name" | "citext" => Self::Text,
            "int2" => Self::Typed("int2", true),
            "int4" => Self::Typed("int4", true),
            "int8" => Self::Typed("int8", true),
            "oid" => Self::Typed("oid", true),
            "float4" => Self::Typed("float4", true),
            "float8" => Self::Typed("float8", true),
            "numeric" => Self::Typed("numeric", true),
            "bool" => Self::Typed("boolean", false),
            "uuid" => Self::Typed("uuid", false),
            "date" => Self::Typed("date", true),
            "time" => Self::Typed("time", true),
            "timetz" => Self::Typed("timetz", true),
            "timestamp" => Self::Typed("timestamp", true),
            "timestamptz" => Self::Typed("timestamptz", true),
            _ => Self::Unsupported,
        }
    }

    fn parameter_cast(self) -> Option<&'static str> {
        match self {
            Self::Text | Self::Unsupported => None,
            Self::Typed(cast, _) => Some(cast),
        }
    }

    fn supports_ordering(self) -> bool {
        match self {
            Self::Text => true,
            Self::Typed(_, supports_ordering) => supports_ordering,
            Self::Unsupported => false,
        }
    }
}

fn to_query_err(err: TableBrowseError) -> CellarError {
    CellarError::query(err.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request() -> TableBrowseRequest {
        TableBrowseRequest {
            connection_id: "conn".into(),
            database: Some("app".into()),
            schema: "public".into(),
            table: "users".into(),
            limit: Some(500),
            sorts: Vec::new(),
            filters: Vec::new(),
            primary_key_fallback_ordering: true,
        }
    }

    fn table() -> Table {
        Table {
            name: "users".into(),
            schema: "public".into(),
            row_count: None,
            primary_key: vec!["id".into()],
            foreign_keys: Vec::new(),
            indexes: Vec::new(),
            columns: vec![
                column("id", "int8", true),
                column("email", "text", false),
                column("age", "int4", false),
                column("deleted_at", "timestamptz", false),
                column("payload", "jsonb", false),
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
    fn quotes_identifiers_and_uses_primary_key_fallback_ordering() {
        let sql = sql_for(&request()).expect("sql");

        assert_eq!(
            sql,
            r#"SELECT * FROM "public"."users" ORDER BY "id" LIMIT $1"#
        );
    }

    #[test]
    fn doubles_embedded_identifier_quotes() {
        let mut req = request();
        req.schema = r#"odd"schema"#.into();
        req.table = r#"user"data"#.into();
        let mut meta = table();
        meta.schema = req.schema.clone();
        meta.name = req.table.clone();

        let sql = build_table_browse_query(&req, &meta)
            .expect("sql")
            .sql()
            .to_string();

        assert_eq!(
            sql,
            r#"SELECT * FROM "odd""schema"."user""data" ORDER BY "id" LIMIT $1"#
        );
    }

    #[test]
    fn binds_filter_values_instead_of_inlining_literals() {
        let mut req = request();
        req.filters.push(TableFilterClause {
            column: "email".into(),
            operator: TableFilterOperator::Contains,
            value: Some("o'reilly_%".into()),
        });

        let sql = sql_for(&req).expect("sql");

        assert_eq!(
            sql,
            r#"SELECT * FROM "public"."users" WHERE "email" ILIKE ('%' || $1 || '%') ORDER BY "id" LIMIT $2"#
        );
        assert!(!sql.contains("o'reilly"));
    }

    #[test]
    fn casts_typed_values_for_safe_comparisons() {
        let mut req = request();
        req.filters.push(TableFilterClause {
            column: "age".into(),
            operator: TableFilterOperator::GreaterThanOrEqual,
            value: Some("21".into()),
        });
        req.filters.push(TableFilterClause {
            column: "deleted_at".into(),
            operator: TableFilterOperator::IsNull,
            value: None,
        });
        req.sorts.push(super::cellar_core_sort("email"));

        let sql = sql_for(&req).expect("sql");

        assert_eq!(
            sql,
            r#"SELECT * FROM "public"."users" WHERE "age" >= $1::int4 AND "deleted_at" IS NULL ORDER BY "email" ASC LIMIT $2"#
        );
    }

    #[test]
    fn rejects_unknown_columns() {
        let mut req = request();
        req.sorts.push(super::cellar_core_sort("missing"));

        assert_eq!(
            sql_for(&req).unwrap_err(),
            TableBrowseError::UnknownColumn("missing".into())
        );
    }

    #[test]
    fn rejects_unsupported_operator_for_column_type() {
        let mut req = request();
        req.filters.push(TableFilterClause {
            column: "payload".into(),
            operator: TableFilterOperator::Contains,
            value: Some("x".into()),
        });

        assert!(matches!(
            sql_for(&req),
            Err(TableBrowseError::UnsupportedOperator { column, .. }) if column == "payload"
        ));
    }

    #[test]
    fn rejects_excessive_limits() {
        let mut req = request();
        req.limit = Some(MAX_TABLE_BROWSE_ROWS + 1);

        assert_eq!(sql_for(&req).unwrap_err(), TableBrowseError::LimitTooLarge);
    }
}

#[cfg(test)]
fn cellar_core_sort(column: &str) -> cellar_core::query::TableSortClause {
    cellar_core::query::TableSortClause {
        column: column.into(),
        direction: SortDirection::Asc,
    }
}
