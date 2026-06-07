use cellar_core::value::CellValue;
use tiberius::ColumnData;

pub fn decode_cell(value: ColumnData<'static>) -> CellValue {
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
        ColumnData::DateTime(v) => v
            .map(|v| CellValue::Text(format!("{v:?}")))
            .unwrap_or(CellValue::Null),
        ColumnData::SmallDateTime(v) => v
            .map(|v| CellValue::Text(format!("{v:?}")))
            .unwrap_or(CellValue::Null),
        ColumnData::Time(v) => v
            .map(|v| CellValue::Text(format!("{v:?}")))
            .unwrap_or(CellValue::Null),
        ColumnData::Date(v) => v
            .map(|v| CellValue::Text(format!("{v:?}")))
            .unwrap_or(CellValue::Null),
        ColumnData::DateTime2(v) => v
            .map(|v| CellValue::Text(format!("{v:?}")))
            .unwrap_or(CellValue::Null),
        ColumnData::DateTimeOffset(v) => v
            .map(|v| CellValue::Text(format!("{v:?}")))
            .unwrap_or(CellValue::Null),
    }
}
