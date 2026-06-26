use cellar_core::error::{CellarError, CellarResult};
use cellar_core::value::CellValue;
use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
use sqlx::mysql::{MySqlRow, MySqlValueRef};
use sqlx::{Row, TypeInfo, ValueRef};

/// Decode one cell from a MySQL row into our typed [`CellValue`].
///
/// We dispatch on the type name reported by `MySqlTypeInfo::name()`. The
/// important quirks of sqlx-mysql's naming, which earlier drafts of this file
/// got wrong:
///
/// * Unsigned integers are named `"INT UNSIGNED"`, `"BIGINT UNSIGNED"`, … —
///   the qualifier is a *suffix*, not a prefix. They also fail sqlx's signed
///   `int_compatible` check, so they must be decoded through `u64`.
/// * Every signed integer width is compatible with `i64` (sqlx widens through
///   `i64` internally), so decoding all signed ints as `i64` avoids a
///   `MEDIUMINT`-into-`i16` overflow.
/// * `DECIMAL` is *not* string-compatible and we don't build with the
///   `bigdecimal`/`rust_decimal` features, so the checked `try_get::<String>`
///   fails. MySQL transmits decimals as ASCII in both protocols, so we use the
///   *unchecked* decode (which calls `String::decode` → reads the bytes as
///   UTF-8) to preserve full precision, mirroring how Postgres keeps `numeric`.
pub fn decode_cell(row: &MySqlRow, ordinal: usize) -> CellarResult<CellValue> {
    let raw: MySqlValueRef<'_> = row
        .try_get_raw(ordinal)
        .map_err(|e| CellarError::decode(e.to_string()))?;
    if raw.is_null() {
        return Ok(CellValue::Null);
    }

    let type_name = raw.type_info().name().to_uppercase();
    match type_name.as_str() {
        // TINYINT(1) surfaces as BOOLEAN; treat it as a boolean.
        "BOOLEAN" | "BOOL" => row
            .try_get::<bool, _>(ordinal)
            .map(CellValue::Bool)
            .map_err(decode_err),

        // All signed widths decode cleanly through i64.
        "TINYINT" | "SMALLINT" | "MEDIUMINT" | "INT" | "INTEGER" | "BIGINT" => row
            .try_get::<i64, _>(ordinal)
            .map(CellValue::Int)
            .map_err(decode_err),

        // Unsigned columns need u64. A BIGINT UNSIGNED can exceed i64::MAX, so
        // keep those as Numeric text rather than silently wrapping.
        "TINYINT UNSIGNED" | "SMALLINT UNSIGNED" | "MEDIUMINT UNSIGNED" | "INT UNSIGNED"
        | "INTEGER UNSIGNED" | "BIGINT UNSIGNED" | "YEAR" | "BIT" => row
            .try_get::<u64, _>(ordinal)
            .map(uint_to_cell)
            .map_err(decode_err),

        "FLOAT" => row
            .try_get::<f32, _>(ordinal)
            .map(|v| CellValue::Float(v as f64))
            .map_err(decode_err),
        "DOUBLE" => row
            .try_get::<f64, _>(ordinal)
            .map(CellValue::Float)
            .map_err(decode_err),

        // DECIMAL is not String-compatible in sqlx and we have no decimal
        // feature; the unchecked decode reads its ASCII bytes and keeps
        // arbitrary precision.
        "DECIMAL" | "NUMERIC" | "NEWDECIMAL" => {
            unchecked_text(row, ordinal).map(CellValue::Numeric)
        }

        "CHAR" | "VARCHAR" | "TEXT" | "TINYTEXT" | "MEDIUMTEXT" | "LONGTEXT" | "ENUM" | "SET" => {
            // ENUM is String-compatible but SET is not, so fall back to the
            // unchecked UTF-8 decode when the checked one is rejected.
            match row.try_get::<String, _>(ordinal) {
                Ok(s) => Ok(CellValue::Text(s)),
                Err(_) => unchecked_text(row, ordinal).map(CellValue::Text),
            }
        }

        "BINARY" | "VARBINARY" | "BLOB" | "TINYBLOB" | "MEDIUMBLOB" | "LONGBLOB" | "GEOMETRY" => {
            row.try_get::<Vec<u8>, _>(ordinal)
                .map(CellValue::Bytes)
                .map_err(decode_err)
        }

        "DATE" => row
            .try_get::<NaiveDate, _>(ordinal)
            .map(CellValue::Date)
            .map_err(decode_err),
        // MySQL TIME is a signed duration (±838h) and can fall outside
        // chrono::NaiveTime's clock range; fall back to the raw string so the
        // value is still shown rather than erroring the whole row.
        "TIME" => match row.try_get::<NaiveTime, _>(ordinal) {
            Ok(t) => Ok(CellValue::Time(t)),
            Err(_) => unchecked_text(row, ordinal).map(CellValue::Text),
        },
        "DATETIME" | "TIMESTAMP" => row
            .try_get::<NaiveDateTime, _>(ordinal)
            .map(CellValue::Timestamp)
            .map_err(decode_err),

        "JSON" => row
            .try_get::<serde_json::Value, _>(ordinal)
            .map(CellValue::Json)
            .map_err(decode_err),

        // Unknown type: try a typed string, then an unchecked UTF-8 read, then
        // bytes, before giving up — better to show the user something than to
        // drop the whole row.
        _ => match row.try_get::<String, _>(ordinal) {
            Ok(s) => Ok(CellValue::Text(s)),
            Err(_) => match row.try_get_unchecked::<String, _>(ordinal) {
                Ok(s) => Ok(CellValue::Text(s)),
                Err(_) => match row.try_get::<Vec<u8>, _>(ordinal) {
                    Ok(b) => Ok(CellValue::Bytes(b)),
                    Err(_) => Err(CellarError::UnsupportedType(type_name)),
                },
            },
        },
    }
}

/// Map an unsigned integer to a cell, keeping values above `i64::MAX` (only
/// possible for `BIGINT UNSIGNED`) as exact decimal text instead of wrapping.
fn uint_to_cell(v: u64) -> CellValue {
    match i64::try_from(v) {
        Ok(i) => CellValue::Int(i),
        Err(_) => CellValue::Numeric(v.to_string()),
    }
}

/// Decode a column as a UTF-8 string via the *unchecked* path, bypassing
/// sqlx's type-compatibility gate. Used for types MySQL transmits as ASCII
/// (DECIMAL, SET) that the checked `String` decode rejects.
fn unchecked_text(row: &MySqlRow, ordinal: usize) -> CellarResult<String> {
    row.try_get_unchecked::<String, _>(ordinal)
        .map_err(decode_err)
}

fn decode_err(e: sqlx::Error) -> CellarError {
    CellarError::Decode(e.to_string())
}
