use cellar_core::error::{CellarError, CellarResult};
use cellar_core::value::CellValue;
use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
use sqlx::mysql::{MySqlRow, MySqlValueRef};
use sqlx::{Row, TypeInfo, ValueRef};

/// Decode one cell from a MySQL row into our typed [`CellValue`].
pub fn decode_cell(row: &MySqlRow, ordinal: usize) -> CellarResult<CellValue> {
    let raw: MySqlValueRef<'_> = row
        .try_get_raw(ordinal)
        .map_err(|e| CellarError::decode(e.to_string()))?;
    if raw.is_null() {
        return Ok(CellValue::Null);
    }

    let type_name = raw.type_info().name().to_uppercase();
    match type_name.as_str() {
        "BOOLEAN" | "BOOL" => {
            row.try_get::<bool, _>(ordinal)
                .map(CellValue::Bool)
                .map_err(decode_err)
        }
        "TINYINT" => {
            row.try_get::<i8, _>(ordinal)
                .map(|v| CellValue::Int(v as i64))
                .map_err(decode_err)
        }
        "SMALLINT" | "MEDIUMINT" => row
            .try_get::<i16, _>(ordinal)
            .map(|v| CellValue::Int(v as i64))
            .map_err(decode_err),
        "INT" | "INTEGER" => row
            .try_get::<i32, _>(ordinal)
            .map(|v| CellValue::Int(v as i64))
            .map_err(decode_err),
        "BIGINT" => row
            .try_get::<i64, _>(ordinal)
            .map(CellValue::Int)
            .map_err(decode_err),
        "UNSIGNED TINYINT" => row
            .try_get::<u8, _>(ordinal)
            .map(|v| CellValue::Int(v as i64))
            .map_err(decode_err),
        "UNSIGNED SMALLINT" => row
            .try_get::<u16, _>(ordinal)
            .map(|v| CellValue::Int(v as i64))
            .map_err(decode_err),
        "UNSIGNED INT" | "UNSIGNED INTEGER" => row
            .try_get::<u32, _>(ordinal)
            .map(|v| CellValue::Int(v as i64))
            .map_err(decode_err),
        "UNSIGNED BIGINT" => row
            .try_get::<u64, _>(ordinal)
            .map(|v| CellValue::Int(v as i64))
            .map_err(decode_err),

        "FLOAT" => row
            .try_get::<f32, _>(ordinal)
            .map(|v| CellValue::Float(v as f64))
            .map_err(decode_err),
        "DOUBLE" => row
            .try_get::<f64, _>(ordinal)
            .map(CellValue::Float)
            .map_err(decode_err),

        "DECIMAL" | "NUMERIC" | "NEWDECIMAL" => row
            .try_get::<String, _>(ordinal)
            .map(CellValue::Numeric)
            .map_err(decode_err),

        "CHAR" | "VARCHAR" | "TEXT" | "TINYTEXT" | "MEDIUMTEXT" | "LONGTEXT"
        | "ENUM" | "SET" => row
            .try_get::<String, _>(ordinal)
            .map(CellValue::Text)
            .map_err(decode_err),

        "BINARY" | "VARBINARY" | "BLOB" | "TINYBLOB" | "MEDIUMBLOB" | "LONGBLOB" => row
            .try_get::<Vec<u8>, _>(ordinal)
            .map(CellValue::Bytes)
            .map_err(decode_err),

        "DATE" => row
            .try_get::<NaiveDate, _>(ordinal)
            .map(CellValue::Date)
            .map_err(decode_err),
        "TIME" => row
            .try_get::<NaiveTime, _>(ordinal)
            .map(CellValue::Time)
            .map_err(decode_err),
        "DATETIME" => row
            .try_get::<NaiveDateTime, _>(ordinal)
            .map(CellValue::Timestamp)
            .map_err(decode_err),
        "TIMESTAMP" => row
            .try_get::<NaiveDateTime, _>(ordinal)
            .map(CellValue::Timestamp)
            .map_err(decode_err),

        "JSON" => row
            .try_get::<serde_json::Value, _>(ordinal)
            .map(CellValue::Json)
            .map_err(decode_err),

        _ => match row.try_get::<String, _>(ordinal) {
            Ok(s) => Ok(CellValue::Text(s)),
            Err(_) => Err(CellarError::UnsupportedType(type_name)),
        },
    }
}

fn decode_err(e: sqlx::Error) -> CellarError {
    CellarError::Decode(e.to_string())
}
