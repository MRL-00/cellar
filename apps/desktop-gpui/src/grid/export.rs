use std::{path::PathBuf, sync::Arc};

use cellar_runtime::export::{export_result_to_path, ExportFormat};
use gpui::{AppContext, Context};

use super::DataGrid;

impl DataGrid {
    pub fn begin_export(&mut self, format: ExportFormat, cx: &mut Context<Self>) {
        if self
            .editable
            .as_ref()
            .is_some_and(|editable| editable.pending_count() > 0)
        {
            self.export_message = Some(Err("Commit or revert pending edits before export".into()));
            cx.notify();
            return;
        }
        let result = Arc::clone(&self.result);
        let table = self.editable.as_ref().map(|editable| {
            (
                editable.schema_name().to_owned(),
                editable.table_name().to_owned(),
            )
        });
        let filename = format!(
            "{}.{}",
            table
                .as_ref()
                .map(|(_, table)| table.as_str())
                .unwrap_or("results"),
            format.extension()
        );
        let directory = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let receiver = cx.prompt_for_new_path(&directory, Some(&filename));
        self.export_message = Some(Ok("Choose a destination…".into()));
        cx.spawn(async move |this, cx| {
            let path = match receiver.await {
                Ok(Ok(Some(path))) => path,
                Ok(Ok(None)) => {
                    this.update(cx, |this, cx| {
                        this.export_message = None;
                        cx.notify();
                    })
                    .ok();
                    return;
                }
                Ok(Err(error)) => {
                    this.update(cx, |this, cx| {
                        this.export_message = Some(Err(error.to_string()));
                        cx.notify();
                    })
                    .ok();
                    return;
                }
                Err(error) => {
                    this.update(cx, |this, cx| {
                        this.export_message = Some(Err(error.to_string()));
                        cx.notify();
                    })
                    .ok();
                    return;
                }
            };
            let exported_path = path.clone();
            let task = cx.background_spawn(async move {
                export_result_to_path(
                    &path,
                    &result,
                    format,
                    table
                        .as_ref()
                        .map(|(schema, table)| (schema.as_str(), table.as_str())),
                )
                .map_err(|error| error.to_string())
            });
            let outcome = task.await;
            this.update(cx, |this, cx| {
                this.export_message =
                    Some(outcome.map(|_| format!("Saved {}", exported_path.to_string_lossy())));
                cx.notify();
            })
            .ok();
        })
        .detach();
        cx.notify();
    }
}
