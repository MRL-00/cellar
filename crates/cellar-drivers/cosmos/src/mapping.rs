//! Document → Cellar column/row mapping.

use std::collections::{BTreeMap, BTreeSet};

use cellar_core::schema::Column;
use cellar_core::value::{CellValue, Row};
use serde_json::{Map, Value};

pub(crate) fn infer_columns(documents: &[Map<String, Value>]) -> Vec<Column> {
    let mut fields: BTreeMap<String, BTreeSet<&'static str>> = BTreeMap::new();
    for doc in documents {
        for (name, value) in doc {
            if name == "id" || name.starts_with('_') {
                continue;
            }
            fields
                .entry(name.clone())
                .or_default()
                .insert(json_type_name(value));
        }
    }

    let mut columns = vec![column("id", "string", false, true, 1)];
    for (name, data_type, ordinal) in [
        ("_ts", "integer", 2u32),
        ("_etag", "string", 3),
        ("_rid", "string", 4),
    ] {
        if documents.iter().any(|d| d.contains_key(name)) {
            columns.push(column(name, data_type, true, false, ordinal));
        }
    }
    let start = columns.len() as u32 + 1;
    for (idx, (name, types)) in fields.into_iter().enumerate() {
        let data_type = if types.len() == 1 {
            types.into_iter().next().unwrap_or("unknown")
        } else {
            "mixed"
        };
        columns.push(column(&name, data_type, true, false, start + idx as u32));
    }
    columns
}

pub(crate) fn columns_for_browse(
    table: &cellar_core::schema::Table,
    documents: &[Map<String, Value>],
) -> Vec<Column> {
    let mut columns = table.columns.clone();
    let existing = columns
        .iter()
        .map(|c| c.name.as_str())
        .collect::<BTreeSet<_>>();
    let mut extra = BTreeMap::new();
    for doc in documents {
        for (name, value) in doc {
            if !existing.contains(name.as_str()) && !name.starts_with('_') {
                extra.entry(name.clone()).or_insert(json_type_name(value));
            }
        }
    }
    let start = columns.len() as u32 + 1;
    for (idx, (name, data_type)) in extra.into_iter().enumerate() {
        columns.push(column(&name, data_type, true, false, start + idx as u32));
    }
    columns
}

pub(crate) fn column(
    name: &str,
    data_type: &str,
    nullable: bool,
    is_primary_key: bool,
    ordinal: u32,
) -> Column {
    Column {
        name: name.into(),
        data_type: data_type.into(),
        nullable,
        default: None,
        is_primary_key,
        ordinal,
        comment: None,
    }
}

pub(crate) fn row_for_document(document: &Map<String, Value>, columns: &[Column]) -> Row {
    columns
        .iter()
        .map(|column| {
            document
                .get(&column.name)
                .map(to_cell_value)
                .unwrap_or(CellValue::Null)
        })
        .collect()
}

pub(crate) fn json_type_name(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(n) if n.is_i64() || n.is_u64() => "integer",
        Value::Number(_) => "double",
        Value::String(_) => "string",
        Value::Array(_) | Value::Object(_) => "json",
    }
}

pub(crate) fn to_cell_value(value: &Value) -> CellValue {
    match value {
        Value::Null => CellValue::Null,
        Value::Bool(v) => CellValue::Bool(*v),
        Value::Number(n) => n
            .as_i64()
            .map(CellValue::Int)
            .or_else(|| n.as_u64().map(|_| CellValue::Numeric(n.to_string())))
            .or_else(|| n.as_f64().map(CellValue::Float))
            .unwrap_or_else(|| CellValue::Numeric(n.to_string())),
        Value::String(v) => CellValue::Text(v.clone()),
        Value::Array(_) | Value::Object(_) => CellValue::Json(value.clone()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn infers_columns_and_maps_rows_from_documents() {
        let documents = vec![json!({
            "id": "ada",
            "_ts": 1_700_000_000,
            "_etag": "\"0000\"",
            "active": true,
            "age": 37,
            "name": "Ada",
            "prefs": { "theme": "dark" }
        })
        .as_object()
        .cloned()
        .unwrap()];

        let columns = infer_columns(&documents);
        assert!(columns.iter().any(|c| c.name == "id" && c.is_primary_key));
        assert!(columns
            .iter()
            .any(|c| c.name == "active" && c.data_type == "boolean"));
        assert!(columns
            .iter()
            .any(|c| c.name == "age" && c.data_type == "integer"));
        assert!(columns.iter().any(|c| c.name == "_ts"));

        let row = row_for_document(&documents[0], &columns);
        assert_eq!(row[0], CellValue::Text("ada".into()));
        let name_pos = columns.iter().position(|c| c.name == "name").unwrap();
        assert_eq!(row[name_pos], CellValue::Text("Ada".into()));
        let prefs_pos = columns.iter().position(|c| c.name == "prefs").unwrap();
        assert_eq!(row[prefs_pos], CellValue::Json(json!({ "theme": "dark" })));
    }

    #[test]
    fn preserves_large_unsigned_integers() {
        let value = json!(u64::MAX);
        assert_eq!(
            to_cell_value(&value),
            CellValue::Numeric(u64::MAX.to_string())
        );
        assert_eq!(to_cell_value(&json!(42)), CellValue::Int(42));
        assert_eq!(to_cell_value(&json!(1.5)), CellValue::Float(1.5));
    }
}
