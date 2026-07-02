//! Shared table-browse types and validation helpers used by every SQL driver.
//!
//! The query-builder and filter-push code stays per-driver because each driver
//! uses a different backend (`QueryBuilder<Postgres>`, `QueryBuilder<MySql>`,
//! or plain `String` for tiberius). Only the pieces that are pure-Rust and
//! backend-agnostic live here.

use thiserror::Error;

use crate::query::{TableBrowseRequest, TableFilterClause, TableFilterOperator};
use crate::schema::{Column, Table};

pub const DEFAULT_TABLE_BROWSE_ROWS: u32 = 500;
/// Hard ceiling raised to 2 000 so that users can request larger pages without
/// hitting the old 500-row wall. The default page size remains 500.
pub const MAX_TABLE_BROWSE_ROWS: u32 = 2000;

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

pub fn validate_table_request(
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

pub fn normalized_limit(request: &TableBrowseRequest) -> Result<u32, TableBrowseError> {
    let limit = request.limit.unwrap_or(DEFAULT_TABLE_BROWSE_ROWS);
    if limit == 0 {
        return Err(TableBrowseError::EmptyLimit);
    }
    if limit > MAX_TABLE_BROWSE_ROWS {
        return Err(TableBrowseError::LimitTooLarge);
    }
    Ok(limit)
}

pub fn normalized_offset(request: &TableBrowseRequest) -> Result<Option<u32>, TableBrowseError> {
    match request.offset {
        Some(offset) if offset > i32::MAX as u32 => Err(TableBrowseError::OffsetTooLarge),
        Some(0) | None => Ok(None),
        Some(offset) => Ok(Some(offset)),
    }
}

pub fn column_for<'a>(table: &'a Table, name: &str) -> Result<&'a Column, TableBrowseError> {
    table
        .columns
        .iter()
        .find(|column| column.name == name)
        .ok_or_else(|| TableBrowseError::UnknownColumn(name.to_string()))
}

pub fn require_value(filter: &TableFilterClause) -> Result<String, TableBrowseError> {
    filter
        .value
        .clone()
        .ok_or_else(|| TableBrowseError::MissingValue {
            column: filter.column.clone(),
            operator: filter.operator,
        })
}

pub fn reject_value(filter: &TableFilterClause) -> Result<(), TableBrowseError> {
    if filter.value.is_some() {
        return Err(TableBrowseError::UnexpectedValue {
            column: filter.column.clone(),
            operator: filter.operator,
        });
    }
    Ok(())
}

/// Escape LIKE wildcards (`\`, `%`, `_`) in user text so contains/starts
/// with/ends with match it literally. Backslash is the escape character in
/// Postgres and MySQL by default; SQL Server drivers must add `ESCAPE '\'`.
/// Not used for the `like` operator, where the user controls the wildcards.
pub fn escape_like_wildcards(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

pub fn unsupported(column: &Column, operator: TableFilterOperator) -> TableBrowseError {
    TableBrowseError::UnsupportedOperator {
        column: column.name.clone(),
        data_type: column.data_type.clone(),
        operator,
    }
}

/// Mark columns whose names appear in `pk` as primary key columns.
/// Used by all SQL drivers during schema introspection.
pub fn mark_primary_keys(mut cols: Vec<Column>, pk: &[String]) -> Vec<Column> {
    for c in cols.iter_mut() {
        if pk.iter().any(|p| p == &c.name) {
            c.is_primary_key = true;
        }
    }
    cols
}

/// Build an empty [`crate::schema::Schema`] with the given name.
/// Used by drivers that assemble schemas incrementally during introspection.
pub fn schema_with(name: String) -> crate::schema::Schema {
    crate::schema::Schema {
        name,
        tables: Vec::new(),
        views: Vec::new(),
    }
}
