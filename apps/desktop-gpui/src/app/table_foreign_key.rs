use std::sync::Arc;

use cellar_core::{
    driver::Engine,
    query::{ForeignKeyLookupRequest, TableFilterClause, TableFilterOperator},
    value::CellValue,
};
use gpui::{Context, Entity};

use cellar_desktop_gpui::{
    grid::DataGrid,
    model::{TableLookupContext, TableTarget},
};

use super::CellarApp;

impl CellarApp {
    pub(super) fn navigate_foreign_key(
        &mut self,
        source_tab_id: u64,
        source: TableTarget,
        lookup: ForeignKeyLookupRequest,
        focus_column: String,
        cx: &mut Context<Self>,
    ) {
        let Some(source_grid) = self.grids.get(&source_tab_id).cloned() else {
            return;
        };
        let error = |grid: &Entity<DataGrid>, message: String, cx: &mut Context<Self>| {
            grid.update(cx, |grid, cx| grid.set_navigation_error(message, cx));
        };
        if lookup.connection_id != source.connection_id || lookup.database != source.database {
            error(
                &source_grid,
                "Foreign-key navigation must stay on the source connection and database".into(),
                cx,
            );
            return;
        }
        if self
            .model
            .connections()
            .iter()
            .find(|connection| connection.id == source.connection_id)
            .is_some_and(|connection| {
                connection.engine.family() == Engine::MySql && lookup.schema != source.schema
            })
        {
            error(
                &source_grid,
                format!(
                    "Cross-catalog MySQL foreign-key navigation is unavailable; referenced catalog {} is not loaded",
                    lookup.schema
                ),
                cx,
            );
            return;
        }
        let destination = TableTarget {
            connection_id: lookup.connection_id.clone(),
            database: lookup.database.clone(),
            schema: lookup.schema.clone(),
            table: lookup.table.clone(),
        };
        if self.model.table(&destination).is_none() {
            error(
                &source_grid,
                "Referenced table metadata is unavailable; refresh the connection schema first"
                    .into(),
                cx,
            );
            return;
        }
        let Some(filters) = lookup
            .columns
            .iter()
            .zip(&lookup.values)
            .map(|(column, value)| {
                lookup_value_text(value).map(|value| TableFilterClause {
                    column: column.clone(),
                    operator: TableFilterOperator::Equals,
                    value: Some(value),
                })
            })
            .collect::<Option<Vec<_>>>()
        else {
            error(
                &source_grid,
                "This foreign-key value cannot be represented by the table filter".into(),
                cx,
            );
            return;
        };
        let registry = Arc::clone(&self.registry);
        let runtime = Arc::clone(&self.runtime);
        cx.spawn(async move |this, cx| {
            let result = runtime
                .spawn(async move { registry.lookup_foreign_key(lookup).await })
                .await
                .map_err(|error| format!("foreign-key lookup task failed: {error}"))
                .and_then(|result| result.map_err(|error| error.to_string()));
            this.update(cx, |this, cx| {
                match result {
                Ok(result) if result.rows.is_empty() => error(
                    &source_grid,
                    "No referenced row matches this foreign-key value".into(),
                    cx,
                ),
                Ok(result) if result.truncated || result.rows.len() > 1 => error(
                    &source_grid,
                    "Multiple referenced rows match this foreign-key value; navigation is ambiguous"
                        .into(),
                    cx,
                ),
                Ok(_) => {
                    this.pending_foreign_key_navigation = Some((
                        destination,
                        TableLookupContext {
                            filters,
                            focus_column,
                        },
                    ));
                    cx.notify();
                }
                Err(error_message) => error(&source_grid, error_message, cx),
            }
            })
            .ok();
        })
        .detach();
    }
}

fn lookup_value_text(value: &CellValue) -> Option<String> {
    match value {
        CellValue::Null => None,
        CellValue::Bool(value) => Some(value.to_string()),
        CellValue::Int(value) => Some(value.to_string()),
        CellValue::Float(value) if value.is_finite() => Some(value.to_string()),
        CellValue::Float(_) => None,
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
