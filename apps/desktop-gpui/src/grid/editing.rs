use std::{collections::BTreeMap, sync::Arc};

use cellar_core::{query::QueryResult, schema::Table, value::CellValue};
use cellar_diff::{CellAssignment, DiffColumn, DiffValue, RowChange, TableChangeRequest};

use crate::model::TableTarget;

#[derive(Clone)]
struct PendingCell {
    value: DiffValue,
}

#[derive(Clone)]
struct PendingRow {
    keys: Vec<CellAssignment>,
    edits: BTreeMap<usize, PendingCell>,
    inserted: bool,
    deleted: bool,
}

pub(super) struct EditableGrid {
    target: TableTarget,
    table: Table,
    row_keys: Arc<Vec<Option<(String, Vec<CellAssignment>)>>>,
    pending: Arc<BTreeMap<String, PendingRow>>,
    next_insert: u64,
}

impl EditableGrid {
    pub(super) fn new(target: TableTarget, table: Table, result: &QueryResult) -> Self {
        let row_keys = result
            .rows
            .iter()
            .map(|row| row_key(&table, result, row))
            .collect();
        Self {
            target,
            table,
            row_keys: Arc::new(row_keys),
            pending: Arc::new(BTreeMap::new()),
            next_insert: 1,
        }
    }

    pub(super) fn can_edit(&self) -> bool {
        !self.table.primary_key.is_empty()
    }

    pub(super) fn column_flags(&self, name: &str) -> (bool, bool) {
        (
            self.table.primary_key.iter().any(|column| column == name),
            self.table
                .foreign_keys
                .iter()
                .any(|key| key.columns.iter().any(|column| column == name)),
        )
    }

    pub(super) fn set_value(
        &mut self,
        row: usize,
        column: usize,
        value: Option<String>,
        result: &QueryResult,
    ) -> Result<(), String> {
        let Some((row_key, keys)) = self.row_keys.get(row).and_then(Option::as_ref) else {
            return Ok(());
        };
        let Some(original) = result.rows.get(row).and_then(|cells| cells.get(column)) else {
            return Ok(());
        };
        if let Some(value) = value.as_deref() {
            validate_value(&result.columns[column].data_type, value)?;
        }
        let pending = Arc::make_mut(&mut self.pending);
        if pending.get(row_key).is_some_and(|row| row.deleted) {
            return Ok(());
        }
        let inserted = pending.get(row_key).is_some_and(|row| row.inserted);
        if !inserted && value == diff_value(original) {
            if let Some(change) = pending.get_mut(row_key) {
                change.edits.remove(&column);
                if change.edits.is_empty() && !change.inserted {
                    pending.remove(row_key);
                }
            }
            return Ok(());
        }
        pending
            .entry(row_key.clone())
            .or_insert_with(|| PendingRow {
                keys: keys.clone(),
                edits: BTreeMap::new(),
                inserted: false,
                deleted: false,
            })
            .edits
            .insert(
                column,
                PendingCell {
                    value: DiffValue { value },
                },
            );
        Ok(())
    }

    pub(super) fn display_value(&self, row: usize, column: usize) -> Option<Option<String>> {
        let row_key = self
            .row_keys
            .get(row)
            .and_then(Option::as_ref)
            .map(|(key, _)| key)?;
        self.pending
            .get(row_key)
            .and_then(|change| change.edits.get(&column))
            .map(|cell| cell.value.value.clone())
    }

    pub(super) fn revert_cell(&mut self, row: usize, column: usize) -> bool {
        let Some(row_key) = self
            .row_keys
            .get(row)
            .and_then(Option::as_ref)
            .map(|(key, _)| key.clone())
        else {
            return false;
        };
        let pending = Arc::make_mut(&mut self.pending);
        let Some(change) = pending.get_mut(&row_key) else {
            return false;
        };
        let reverted = change.edits.remove(&column).is_some();
        if change.edits.is_empty() && !change.inserted && !change.deleted {
            pending.remove(&row_key);
        }
        reverted
    }

    pub(super) fn pending_count(&self) -> usize {
        self.pending
            .values()
            .map(|change| {
                if change.deleted || change.inserted {
                    1
                } else {
                    change.edits.len()
                }
            })
            .sum()
    }

    pub(super) fn insert_row(&mut self, row: usize) {
        if row != self.row_keys.len() {
            return;
        }
        let row_id = format!("insert:{}", self.next_insert);
        self.next_insert += 1;
        Arc::make_mut(&mut self.row_keys).push(Some((row_id.clone(), Vec::new())));
        Arc::make_mut(&mut self.pending).insert(
            row_id,
            PendingRow {
                keys: Vec::new(),
                edits: BTreeMap::new(),
                inserted: true,
                deleted: false,
            },
        );
    }

    pub(super) fn remove_insert(&mut self, row: usize) -> bool {
        let Some((row_id, _)) = self.row_keys.get(row).and_then(Option::as_ref) else {
            return false;
        };
        if !self
            .pending
            .get(row_id)
            .is_some_and(|change| change.inserted)
        {
            return false;
        }
        let row_id = row_id.clone();
        Arc::make_mut(&mut self.row_keys).remove(row);
        Arc::make_mut(&mut self.pending).remove(&row_id);
        true
    }

    pub(super) fn inserted_rows(&self) -> Vec<usize> {
        self.changed_rows(|change| change.inserted)
    }

    /// Matches the classic grid: inserts are cancelled, existing deletes are
    /// unmarked, and all other rows become pending deletes.
    pub(super) fn toggle_delete(&mut self, row: usize) -> bool {
        if self.remove_insert(row) {
            return true;
        }
        let Some((row_key, keys)) = self.row_keys.get(row).and_then(Option::as_ref) else {
            return false;
        };
        let pending = Arc::make_mut(&mut self.pending);
        if pending.get(row_key).is_some_and(|change| change.deleted) {
            pending.remove(row_key);
        } else {
            pending.insert(
                row_key.clone(),
                PendingRow {
                    keys: keys.clone(),
                    edits: BTreeMap::new(),
                    inserted: false,
                    deleted: true,
                },
            );
        }
        false
    }

    pub(super) fn deleted_rows(&self) -> Vec<usize> {
        self.changed_rows(|change| change.deleted)
    }

    fn changed_rows(&self, predicate: impl Fn(&PendingRow) -> bool) -> Vec<usize> {
        self.row_keys
            .iter()
            .enumerate()
            .filter_map(|(row, key)| {
                key.as_ref()
                    .and_then(|(key, _)| self.pending.get(key))
                    .is_some_and(&predicate)
                    .then_some(row)
            })
            .collect()
    }

    pub(super) fn display_values(&self) -> BTreeMap<(usize, usize), Option<String>> {
        self.row_keys
            .iter()
            .enumerate()
            .flat_map(|(row, key)| {
                let edits = key
                    .as_ref()
                    .and_then(|(key, _)| self.pending.get(key))
                    .map(|change| &change.edits);
                edits.into_iter().flat_map(move |edits| {
                    edits
                        .iter()
                        .map(move |(column, cell)| ((row, *column), cell.value.value.clone()))
                })
            })
            .collect()
    }

    pub(super) fn move_column(&mut self, source: usize, target: usize) {
        for change in Arc::make_mut(&mut self.pending).values_mut() {
            change.edits = std::mem::take(&mut change.edits)
                .into_iter()
                .map(|(column, edit)| (super::moved_index(column, source, target), edit))
                .collect();
        }
    }

    pub(super) fn clear(&mut self) -> Vec<usize> {
        let inserted = self.inserted_rows();
        for row in inserted.iter().rev() {
            Arc::make_mut(&mut self.row_keys).remove(*row);
        }
        self.pending = Arc::new(BTreeMap::new());
        inserted
    }

    pub(super) fn request(&self, result: &QueryResult) -> TableChangeRequest {
        let changes = self
            .pending
            .iter()
            .map(|(row_id, change)| {
                if change.inserted {
                    RowChange::Insert {
                        row_id: row_id.clone(),
                        values: change
                            .edits
                            .iter()
                            .filter_map(|(column, edit)| {
                                result.columns.get(*column).map(|meta| CellAssignment {
                                    column: meta.name.clone(),
                                    value: edit.value.clone(),
                                })
                            })
                            .collect(),
                    }
                } else if change.deleted {
                    RowChange::Delete {
                        row_id: row_id.clone(),
                        keys: change.keys.clone(),
                    }
                } else {
                    RowChange::Update {
                        row_id: row_id.clone(),
                        keys: change.keys.clone(),
                        edits: change
                            .edits
                            .iter()
                            .filter_map(|(column, edit)| {
                                result.columns.get(*column).map(|meta| CellAssignment {
                                    column: meta.name.clone(),
                                    value: edit.value.clone(),
                                })
                            })
                            .collect(),
                    }
                }
            })
            .collect();
        TableChangeRequest {
            database: Some(self.target.database.clone()),
            schema: self.target.schema.clone(),
            table: self.target.table.clone(),
            primary_key: self.table.primary_key.clone(),
            columns: self
                .table
                .columns
                .iter()
                .map(|column| DiffColumn {
                    name: column.name.clone(),
                    data_type: column.data_type.clone(),
                    nullable: column.nullable,
                })
                .collect(),
            changes,
        }
    }

    pub(super) fn connection_id(&self) -> &str {
        &self.target.connection_id
    }

    pub(super) fn target(&self) -> &TableTarget {
        &self.target
    }

    pub(super) fn table_name(&self) -> &str {
        &self.target.table
    }

    pub(super) fn schema_name(&self) -> &str {
        &self.target.schema
    }
}

fn validate_value(data_type: &str, value: &str) -> Result<(), String> {
    let kind = data_type.to_ascii_lowercase();
    if matches!(kind.as_str(), "bool" | "boolean") {
        value
            .parse::<bool>()
            .map(|_| ())
            .map_err(|_| "Boolean values must be true or false".into())
    } else if kind == "date" {
        chrono::NaiveDate::parse_from_str(value, "%Y-%m-%d")
            .map(|_| ())
            .map_err(|_| "Date values must use YYYY-MM-DD".into())
    } else if ["int", "serial", "oid"]
        .iter()
        .any(|needle| kind.contains(needle))
        && !kind.contains("interval")
    {
        value
            .parse::<i64>()
            .map(|_| ())
            .map_err(|_| "Integer columns need a whole number".into())
    } else if ["numeric", "decimal"]
        .iter()
        .any(|needle| kind.contains(needle))
    {
        valid_decimal(value)
            .then_some(())
            .ok_or_else(|| "Numeric columns need a valid number".into())
    } else if ["float", "double", "real"]
        .iter()
        .any(|needle| kind.contains(needle))
    {
        value
            .parse::<f64>()
            .map(|_| ())
            .map_err(|_| "Floating-point columns need a valid finite or exponential number".into())
    } else {
        Ok(())
    }
}

fn valid_decimal(value: &str) -> bool {
    let value = value.trim();
    let value = value
        .strip_prefix('+')
        .or_else(|| value.strip_prefix('-'))
        .unwrap_or(value);
    let mut parts = value.split(['e', 'E']);
    let Some(mantissa) = parts.next() else {
        return false;
    };
    let exponent = parts.next();
    if parts.next().is_some() || exponent.is_some_and(|value| value.parse::<i32>().is_err()) {
        return false;
    }
    let mut dot = false;
    let mut digit = false;
    mantissa.chars().all(|character| {
        if character == '.' && !dot {
            dot = true;
            true
        } else if character.is_ascii_digit() {
            digit = true;
            true
        } else {
            false
        }
    }) && digit
}

fn row_key(
    table: &Table,
    result: &QueryResult,
    row: &[CellValue],
) -> Option<(String, Vec<CellAssignment>)> {
    if table.primary_key.is_empty() {
        return None;
    }
    let keys = table
        .primary_key
        .iter()
        .map(|name| {
            let column = result
                .columns
                .iter()
                .position(|column| &column.name == name)?;
            let value = row.get(column)?;
            Some(CellAssignment {
                column: name.clone(),
                value: DiffValue {
                    value: diff_value(value),
                },
            })
        })
        .collect::<Option<Vec<_>>>()?;
    let id = keys
        .iter()
        .map(|key| format!("{}={:?}", key.column, key.value.value))
        .collect::<Vec<_>>()
        .join("\u{1f}");
    Some((id, keys))
}

fn diff_value(value: &CellValue) -> Option<String> {
    match value {
        CellValue::Null => None,
        CellValue::Bool(value) => Some(value.to_string()),
        CellValue::Int(value) => Some(value.to_string()),
        CellValue::Float(value) => Some(value.to_string()),
        CellValue::Numeric(value) | CellValue::Text(value) => Some(value.clone()),
        CellValue::Bytes(value) => Some(format!(
            "\\x{}",
            value
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>()
        )),
        CellValue::Json(value) => Some(value.to_string()),
        CellValue::Uuid(value) => Some(value.to_string()),
        CellValue::Date(value) => Some(value.to_string()),
        CellValue::Time(value) => Some(value.to_string()),
        CellValue::Timestamp(value) => Some(value.to_string()),
        CellValue::TimestampTz(value) => Some(value.to_rfc3339()),
    }
}

#[cfg(test)]
mod tests {
    use cellar_core::{
        query::{NoticeCapture, QueryResult},
        schema::{Column, Table},
        value::{CellValue, ColumnMeta},
    };

    use super::{validate_value, EditableGrid};
    use crate::model::TableTarget;
    use cellar_diff::RowChange;

    fn fixture() -> (TableTarget, Table, QueryResult) {
        let target = TableTarget {
            connection_id: "one".into(),
            database: "cellar".into(),
            schema: "public".into(),
            table: "users".into(),
        };
        let table = Table {
            name: "users".into(),
            schema: "public".into(),
            row_count: Some(1),
            columns: vec![
                Column {
                    name: "id".into(),
                    data_type: "int8".into(),
                    nullable: false,
                    default: None,
                    is_primary_key: true,
                    ordinal: 1,
                    comment: None,
                },
                Column {
                    name: "name".into(),
                    data_type: "text".into(),
                    nullable: true,
                    default: None,
                    is_primary_key: false,
                    ordinal: 2,
                    comment: None,
                },
            ],
            primary_key: vec!["id".into()],
            foreign_keys: Vec::new(),
            indexes: Vec::new(),
        };
        let result = QueryResult {
            columns: vec![
                ColumnMeta {
                    name: "id".into(),
                    data_type: "int8".into(),
                    nullable: false,
                },
                ColumnMeta {
                    name: "name".into(),
                    data_type: "text".into(),
                    nullable: true,
                },
            ],
            rows: vec![vec![CellValue::Int(7), CellValue::Text("old".into())]],
            notices: Vec::new(),
            notice_capture: NoticeCapture::unsupported("test"),
            rows_affected: None,
            duration_ms: 0,
            truncated: false,
            total_rows: Some(1),
        };
        (target, table, result)
    }

    #[test]
    fn pending_edit_builds_safe_request_and_reverting_drops_it() {
        let (target, table, result) = fixture();
        let mut edits = EditableGrid::new(target, table, &result);
        edits
            .set_value(0, 1, Some("new".into()), &result)
            .expect("valid text edit");
        let request = edits.request(&result);
        assert_eq!(edits.pending_count(), 1);
        assert_eq!(request.changes.len(), 1);
        let debug = format!("{:?}", request.changes[0]);
        assert!(debug.contains("id") && debug.contains("new"));

        assert!(edits.revert_cell(0, 1));
        assert_eq!(edits.pending_count(), 0);
        edits
            .set_value(0, 1, Some("new".into()), &result)
            .expect("valid text edit");

        edits
            .set_value(0, 1, Some("old".into()), &result)
            .expect("valid text edit");
        assert_eq!(edits.pending_count(), 0);
    }

    #[test]
    fn pending_delete_builds_safe_request_and_reverting_drops_it() {
        let (target, table, result) = fixture();
        let mut edits = EditableGrid::new(target, table, &result);
        edits.toggle_delete(0);

        assert_eq!(edits.pending_count(), 1);
        assert_eq!(edits.deleted_rows(), vec![0]);
        assert!(matches!(
            &edits.request(&result).changes[0],
            RowChange::Delete { row_id, keys }
                if row_id.contains("id") && keys[0].column == "id"
        ));

        edits.clear();
        assert_eq!(edits.pending_count(), 0);
    }

    #[test]
    fn delete_toggle_unmarks_an_existing_pending_delete() {
        let (target, table, result) = fixture();
        let mut edits = EditableGrid::new(target, table, &result);
        assert!(!edits.toggle_delete(0));
        assert_eq!(edits.deleted_rows(), vec![0]);
        assert!(!edits.toggle_delete(0));
        assert!(edits.deleted_rows().is_empty());
    }

    #[test]
    fn pending_insert_builds_values_and_can_be_removed() {
        let (target, table, mut result) = fixture();
        let mut edits = EditableGrid::new(target, table, &result);
        let row = result.rows.len();
        result
            .rows
            .push(vec![CellValue::Null; result.columns.len()]);
        edits.insert_row(row);
        edits
            .set_value(row, 0, Some("8".into()), &result)
            .expect("valid integer edit");

        assert_eq!(edits.pending_count(), 1);
        assert_eq!(edits.inserted_rows(), vec![1]);
        assert!(matches!(
            &edits.request(&result).changes[0],
            RowChange::Insert { row_id, values }
                if row_id.starts_with("insert:")
                    && values[0].column == "id"
                    && values[0].value.value.as_deref() == Some("8")
        ));

        assert!(edits.remove_insert(row));
        assert_eq!(edits.pending_count(), 0);
    }

    #[test]
    fn typed_edits_reject_values_the_database_cannot_cast_safely() {
        assert!(validate_value("int8", "42").is_ok());
        assert!(validate_value("int8", "4.2").is_err());
        assert!(validate_value("boolean", "true").is_ok());
        assert!(validate_value("numeric", "999999999999999999999999.25").is_ok());
        assert!(validate_value("numeric", "--1").is_err());
        assert!(validate_value("date", "2026-08-14").is_ok());
        assert!(validate_value("date", "14/08/2026").is_err());
    }
}
