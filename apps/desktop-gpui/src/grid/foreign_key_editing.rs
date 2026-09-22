use std::collections::BTreeSet;

use cellar_core::{
    query::{ForeignKeyLookupRequest, QueryResult},
    schema::ForeignKey,
    value::CellValue,
};

use super::editing::EditableGrid;

#[derive(Clone)]
pub(super) struct ForeignKeyNavigationOption {
    pub(super) label: String,
    pub(super) lookup: Result<(ForeignKeyLookupRequest, String), String>,
}

impl EditableGrid {
    pub(super) fn foreign_key_columns(&self, result: &QueryResult) -> BTreeSet<usize> {
        result
            .columns
            .iter()
            .enumerate()
            .filter_map(|(index, column)| {
                self.table
                    .foreign_keys
                    .iter()
                    .any(|key| key.columns.iter().any(|name| name == &column.name))
                    .then_some(index)
            })
            .collect()
    }

    pub(super) fn foreign_key_unavailable_columns(&self, result: &QueryResult) -> BTreeSet<usize> {
        result
            .columns
            .iter()
            .enumerate()
            .filter_map(|(index, column)| {
                let keys = self
                    .table
                    .foreign_keys
                    .iter()
                    .filter(|key| key.columns.iter().any(|name| name == &column.name))
                    .collect::<Vec<_>>();
                (!keys.is_empty()
                    && keys
                        .iter()
                        .all(|key| self.navigation_unavailable_reason(key).is_some()))
                .then_some(index)
            })
            .collect()
    }

    pub(super) fn foreign_key_navigation_options(
        &self,
        result: &QueryResult,
        row: usize,
        column: usize,
    ) -> Result<Vec<ForeignKeyNavigationOption>, String> {
        let selected = result
            .columns
            .get(column)
            .ok_or_else(|| "The selected cell is outside the result columns".to_owned())?;
        let keys = self
            .table
            .foreign_keys
            .iter()
            .filter(|key| key.columns.iter().any(|name| name == &selected.name))
            .collect::<Vec<_>>();
        if keys.is_empty() {
            return Err("This column is not a foreign key".into());
        }

        Ok(keys
            .into_iter()
            .map(|key| ForeignKeyNavigationOption {
                label: foreign_key_label(key),
                lookup: self.foreign_key_lookup_for_key(result, row, key),
            })
            .collect())
    }

    fn foreign_key_lookup_for_key(
        &self,
        result: &QueryResult,
        row: usize,
        key: &ForeignKey,
    ) -> Result<(ForeignKeyLookupRequest, String), String> {
        if let Some(reason) = self.navigation_unavailable_reason(key) {
            return Err(reason);
        }
        if key.columns.is_empty() || key.columns.len() != key.referenced_columns.len() {
            return Err("Foreign-key metadata is incomplete for this relationship".into());
        }
        if self.inserted_rows().contains(&row) {
            return Err("Commit the inserted row before navigating a foreign key".into());
        }
        if self.deleted_rows().contains(&row) {
            return Err("Revert the pending row delete before navigating a foreign key".into());
        }

        let mut values = Vec::with_capacity(key.columns.len());
        for (local, referenced) in key.columns.iter().zip(&key.referenced_columns) {
            if referenced.trim().is_empty() {
                return Err(
                    "Referenced-column metadata is unavailable for this foreign key".into(),
                );
            }
            let local_index = result
                .columns
                .iter()
                .position(|column| &column.name == local)
                .ok_or_else(|| {
                    format!("Foreign-key column {local} is not present in this result")
                })?;
            if self.display_value(row, local_index).is_some() {
                return Err(
                    "Commit or revert pending edits to the foreign-key columns before navigating"
                        .into(),
                );
            }
            let value = result
                .rows
                .get(row)
                .and_then(|cells| cells.get(local_index))
                .ok_or_else(|| "The selected row is no longer available".to_owned())?;
            if value.is_null() {
                return Err("NULL foreign-key values do not identify a referenced row".into());
            }
            if matches!(value, CellValue::Bytes(_) | CellValue::Json(_)) {
                return Err(
                    "This foreign-key value type cannot be carried into a bounded table filter"
                        .into(),
                );
            }
            if matches!(value, CellValue::Float(value) if !value.is_finite()) {
                return Err(
                    "Non-finite foreign-key values cannot identify a referenced row".into(),
                );
            }
            values.push(value.clone());
        }

        Ok((
            ForeignKeyLookupRequest {
                connection_id: self.target.connection_id.clone(),
                database: self.target.database.clone(),
                schema: key.referenced_schema.clone(),
                table: key.referenced_table.clone(),
                columns: key.referenced_columns.clone(),
                values,
            },
            key.referenced_columns[0].clone(),
        ))
    }

    fn navigation_unavailable_reason(&self, key: &ForeignKey) -> Option<String> {
        (self.source_engine() == cellar_core::driver::Engine::MySql
            && key.referenced_schema != self.target.schema)
        .then(|| {
            format!(
                "Cross-catalog MySQL navigation is unavailable; referenced catalog {} is not loaded",
                key.referenced_schema
            )
        })
    }
}

fn foreign_key_label(key: &ForeignKey) -> String {
    let target = if key.referenced_schema.trim().is_empty() {
        key.referenced_table.clone()
    } else {
        format!("{}.{}", key.referenced_schema, key.referenced_table)
    };
    if key.name.trim().is_empty() {
        format!("Open {target}")
    } else {
        format!("{} → {target}", key.name)
    }
}
