use cellar_core::value::CellValue;
use chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime, Utc};
use tiberius::{ColumnData, FromSql};

pub fn decode_cell(value: ColumnData<'static>) -> CellValue {
    // Temporal types: decode via tiberius's chrono FromSql impls into the typed
    // CellValue variants. The inner ColumnData::DateTime2 etc. only implement
    // Debug, so formatting them directly yields "DateTime2 { date: Date(0) }".
    match &value {
        ColumnData::DateTime(_) | ColumnData::SmallDateTime(_) | ColumnData::DateTime2(_) => {
            return NaiveDateTime::from_sql(&value)
                .ok()
                .flatten()
                .map(CellValue::Timestamp)
                .unwrap_or(CellValue::Null);
        }
        ColumnData::DateTimeOffset(_) => {
            return DateTime::<Utc>::from_sql(&value)
                .ok()
                .flatten()
                .map(CellValue::TimestampTz)
                .unwrap_or(CellValue::Null);
        }
        ColumnData::Date(_) => {
            return NaiveDate::from_sql(&value)
                .ok()
                .flatten()
                .map(CellValue::Date)
                .unwrap_or(CellValue::Null);
        }
        ColumnData::Time(_) => {
            return NaiveTime::from_sql(&value)
                .ok()
                .flatten()
                .map(CellValue::Time)
                .unwrap_or(CellValue::Null);
        }
        _ => {}
    }
    match value {
        ColumnData::U8(v) => v
            .map(|v| CellValue::Int(v as i64))
            .unwrap_or(CellValue::Null),
        ColumnData::I16(v) => v
            .map(|v| CellValue::Int(v as i64))
            .unwrap_or(CellValue::Null),
        ColumnData::I32(v) => v
            .map(|v| CellValue::Int(v as i64))
            .unwrap_or(CellValue::Null),
        ColumnData::I64(v) => v.map(CellValue::Int).unwrap_or(CellValue::Null),
        ColumnData::F32(v) => v
            .map(|v| CellValue::Float(v as f64))
            .unwrap_or(CellValue::Null),
        ColumnData::F64(v) => v.map(CellValue::Float).unwrap_or(CellValue::Null),
        ColumnData::Bit(v) => v.map(CellValue::Bool).unwrap_or(CellValue::Null),
        ColumnData::String(v) => v
            .map(|v| CellValue::Text(v.into_owned()))
            .unwrap_or(CellValue::Null),
        ColumnData::Guid(v) => v.map(CellValue::Uuid).unwrap_or(CellValue::Null),
        ColumnData::Binary(v) => v
            .map(|v| CellValue::Bytes(v.into_owned()))
            .unwrap_or(CellValue::Null),
        ColumnData::Numeric(v) => v
            .map(|v| CellValue::Numeric(v.to_string()))
            .unwrap_or(CellValue::Null),
        ColumnData::Xml(v) => v
            .map(|v| CellValue::Text(format!("{v:?}")))
            .unwrap_or(CellValue::Null),
        // Temporal variants handled above via FromSql.
        ColumnData::DateTime(_)
        | ColumnData::SmallDateTime(_)
        | ColumnData::Time(_)
        | ColumnData::Date(_)
        | ColumnData::DateTime2(_)
        | ColumnData::DateTimeOffset(_) => unreachable!("handled above"),
    }
}
