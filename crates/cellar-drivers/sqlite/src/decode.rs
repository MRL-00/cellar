use cellar_core::error::{CellarError, CellarResult};
use cellar_core::value::CellValue;
use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
use sqlx::sqlite::{SqliteRow, SqliteValueRef};
use sqlx::{Row, TypeInfo, ValueRef};

/// Decode one cell from a SQLite row into our typed [`CellValue`].
///
/// SQLite is dynamically typed: a column declared INTEGER can still hold
/// text, so the reported type name is a hint, not a guarantee. Every arm
/// falls back to a plain text read (then bytes) instead of erroring the row.
pub fn decode_cell(row: &SqliteRow, ordinal: usize) -> CellarResult<CellValue> {
    let raw: SqliteValueRef<'_> = row
        .try_get_raw(ordinal)
        .map_err(|e| CellarError::decode(e.to_string()))?;
    if raw.is_null() {
        return Ok(CellValue::Null);
    }

    let type_name = raw.type_info().name().to_uppercase();
    let cell = match type_name.as_str() {
        "INTEGER" | "INT" | "INT4" | "INT8" | "BIGINT" => {
            row.try_get::<i64, _>(ordinal).map(CellValue::Int).ok()
        }
        "REAL" | "FLOAT" | "DOUBLE" => row.try_get::<f64, _>(ordinal).map(CellValue::Float).ok(),
        "BOOLEAN" | "BOOL" => row.try_get::<bool, _>(ordinal).map(CellValue::Bool).ok(),
        // NUMERIC affinity stores whatever fits; try the concrete types in
        // order of fidelity.
        "NUMERIC" => row
            .try_get::<i64, _>(ordinal)
            .map(CellValue::Int)
            .or_else(|_| row.try_get::<f64, _>(ordinal).map(CellValue::Float))
            .ok(),
        "DATE" => row
            .try_get::<NaiveDate, _>(ordinal)
            .map(CellValue::Date)
            .ok(),
        "TIME" => row
            .try_get::<NaiveTime, _>(ordinal)
            .map(CellValue::Time)
            .ok(),
        "DATETIME" | "TIMESTAMP" => row
            .try_get::<NaiveDateTime, _>(ordinal)
            .map(CellValue::Timestamp)
            .ok(),
        "BLOB" => row
            .try_get::<Vec<u8>, _>(ordinal)
            .map(CellValue::Bytes)
            .ok(),
        "TEXT" => row.try_get::<String, _>(ordinal).map(CellValue::Text).ok(),
        _ => None,
    };

    if let Some(cell) = cell {
        return Ok(cell);
    }

    // Dynamic-typing fallback: show the user something rather than dropping
    // the whole row.
    match row.try_get::<String, _>(ordinal) {
        Ok(s) => Ok(CellValue::Text(s)),
        Err(_) => match row.try_get::<Vec<u8>, _>(ordinal) {
            Ok(b) => Ok(CellValue::Bytes(b)),
            Err(_) => Err(CellarError::UnsupportedType(type_name)),
        },
    }
}
