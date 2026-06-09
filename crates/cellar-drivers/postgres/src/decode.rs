use cellar_core::error::{CellarError, CellarResult};
use cellar_core::value::CellValue;
use chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime, Utc};
use sqlx::postgres::{PgRow, PgValueRef};
use sqlx::types::BigDecimal;
use sqlx::{Row, TypeInfo, ValueRef};
use uuid::Uuid;

/// Decode one cell from a Postgres row into our typed [`CellValue`].
///
/// We dispatch on the Postgres type name reported by `PgTypeInfo::name()`.
/// Unknown types are surfaced as `Text` after a best-effort `SELECT` raw
/// string decode so the user still sees *something* in the grid — better
/// than dropping the column.
pub fn decode_cell(row: &PgRow, ordinal: usize) -> CellarResult<CellValue> {
    let raw: PgValueRef<'_> = row
        .try_get_raw(ordinal)
        .map_err(|e| CellarError::decode(e.to_string()))?;
    if raw.is_null() {
        return Ok(CellValue::Null);
    }

    let type_name = raw.type_info().name().to_uppercase();
    match type_name.as_str() {
        "BOOL" => row
            .try_get::<bool, _>(ordinal)
            .map(CellValue::Bool)
            .map_err(decode_err),

        "INT2" => row
            .try_get::<i16, _>(ordinal)
            .map(|v| CellValue::Int(v as i64))
            .map_err(decode_err),
        "INT4" => row
            .try_get::<i32, _>(ordinal)
            .map(|v| CellValue::Int(v as i64))
            .map_err(decode_err),
        "INT8" => row
            .try_get::<i64, _>(ordinal)
            .map(CellValue::Int)
            .map_err(decode_err),
        "OID" => row
            .try_get::<i64, _>(ordinal)
            .map(CellValue::Int)
            .map_err(decode_err),

        "FLOAT4" => row
            .try_get::<f32, _>(ordinal)
            .map(|v| CellValue::Float(v as f64))
            .map_err(decode_err),
        "FLOAT8" => row
            .try_get::<f64, _>(ordinal)
            .map(CellValue::Float)
            .map_err(decode_err),

        // `numeric` keeps arbitrary precision. BigDecimal → string preserves it.
        "NUMERIC" => row
            .try_get::<BigDecimal, _>(ordinal)
            .map(|v| CellValue::Numeric(v.to_string()))
            .map_err(decode_err),

        "TEXT" | "VARCHAR" | "CHAR" | "BPCHAR" | "NAME" | "CITEXT" => row
            .try_get::<String, _>(ordinal)
            .map(CellValue::Text)
            .map_err(decode_err),

        "UUID" => row
            .try_get::<Uuid, _>(ordinal)
            .map(CellValue::Uuid)
            .map_err(decode_err),

        "TIMESTAMPTZ" => row
            .try_get::<DateTime<Utc>, _>(ordinal)
            .map(CellValue::TimestampTz)
            .map_err(decode_err),
        "TIMESTAMP" => row
            .try_get::<NaiveDateTime, _>(ordinal)
            .map(CellValue::Timestamp)
            .map_err(decode_err),
        "DATE" => row
            .try_get::<NaiveDate, _>(ordinal)
            .map(CellValue::Date)
            .map_err(decode_err),
        "TIME" | "TIMETZ" => row
            .try_get::<NaiveTime, _>(ordinal)
            .map(CellValue::Time)
            .map_err(decode_err),

        "JSON" | "JSONB" => row
            .try_get::<serde_json::Value, _>(ordinal)
            .map(CellValue::Json)
            .map_err(decode_err),

        "BYTEA" => row
            .try_get::<Vec<u8>, _>(ordinal)
            .map(CellValue::Bytes)
            .map_err(decode_err),

        // Fall back to text. `try_get::<String>` works for many implicit
        // string casts (e.g. inet, cidr, mac, money) when sqlx supports the
        // decode. User-defined enums are the common case here: sqlx rejects the
        // custom type OID in `try_get`, but Postgres transmits an enum's *label*
        // as the value bytes in both wire formats, so a raw UTF-8 read recovers
        // it. Only if that also fails do we surface the type name.
        other => match row.try_get::<String, _>(ordinal) {
            Ok(s) => Ok(CellValue::Text(s)),
            Err(_) => match raw.as_str() {
                Ok(s) => Ok(CellValue::Text(s.to_string())),
                Err(_) => Err(CellarError::UnsupportedType(other.to_string())),
            },
        },
    }
}

fn decode_err(e: sqlx::Error) -> CellarError {
    CellarError::Decode(e.to_string())
}
