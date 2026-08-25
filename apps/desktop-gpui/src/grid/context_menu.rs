use cellar_core::query::QueryResult;
use cellar_runtime::export::{export_result, ExportFormat};
use gpui::{App, ClipboardItem, WeakEntity};
use gpui_component::{
    menu::{PopupMenu, PopupMenuItem},
    Icon,
};

use super::{row::cell_edit_text, DataGrid, DataGridEvent};

impl DataGrid {
    pub(super) fn header_context_menu(
        &self,
        mut menu: PopupMenu,
        column: usize,
        grid: WeakEntity<Self>,
    ) -> PopupMenu {
        let Some(meta) = self.result.columns.get(column) else {
            return menu;
        };
        if let Some(editable) = &self.editable {
            let target = editable.target().clone();
            let column_name = meta.name.clone();
            let column_target = target.clone();
            let column_grid = grid.clone();
            menu = menu
                .item(
                    PopupMenuItem::new(format!("Find Usages of {}", meta.name))
                        .icon(Icon::empty().path("icons/search.svg"))
                        .on_click(move |_, _, cx| {
                            column_grid
                                .update(cx, |_, cx| {
                                    cx.emit(DataGridEvent::FindUsages {
                                        target: column_target.clone(),
                                        column: Some(column_name.clone()),
                                    });
                                })
                                .ok();
                        }),
                )
                .item(
                    PopupMenuItem::new(format!("Find Usages of {}", target.table))
                        .icon(Icon::empty().path("icons/search.svg"))
                        .on_click(move |_, _, cx| {
                            grid.update(cx, |_, cx| {
                                cx.emit(DataGridEvent::FindUsages {
                                    target: target.clone(),
                                    column: None,
                                });
                            })
                            .ok();
                        }),
                );
        }
        menu.item(copy_item("Copy column name", meta.name.clone()))
    }

    pub(super) fn cell_context_menu(
        &self,
        mut menu: PopupMenu,
        row: usize,
        column: usize,
        grid: WeakEntity<Self>,
    ) -> PopupMenu {
        menu = menu.item(copy_item("Copy cell", self.displayed_cell(row, column)));
        let guid = self
            .result
            .columns
            .get(column)
            .is_some_and(|column| is_guid_type(&column.data_type));
        let deleted = self
            .editable
            .as_ref()
            .is_some_and(|editable| editable.deleted_rows().contains(&row));
        if guid && self.editable.is_some() && !deleted {
            menu = menu.item(
                PopupMenuItem::new("Generate new GUID").on_click(move |_, _, cx| {
                    grid.update(cx, |grid, cx| {
                        grid.set_cell_value(row, column, Some(uuid::Uuid::new_v4().to_string()), cx)
                    })
                    .ok();
                }),
            );
        }
        menu
    }

    pub(super) fn row_context_menu(
        &self,
        mut menu: PopupMenu,
        row: usize,
        grid: WeakEntity<Self>,
    ) -> PopupMenu {
        for (label, format) in [
            ("Copy row as CSV", ExportFormat::Csv),
            ("Copy row as TSV", ExportFormat::Tsv),
            ("Copy row as JSON", ExportFormat::Json),
            ("Copy row as SQL INSERT", ExportFormat::Sql),
        ] {
            menu = menu.item(copy_item(label, self.formatted_rows(&[row], format, false)));
        }
        if let Some(editable) = &self.editable {
            let label = if editable.deleted_rows().contains(&row) {
                "Unmark row for delete"
            } else if editable.inserted_rows().contains(&row) {
                "Cancel insert"
            } else {
                "Delete row"
            };
            menu = menu.item(PopupMenuItem::separator()).item(
                PopupMenuItem::new(label)
                    .icon(Icon::empty().path("icons/trash.svg"))
                    .on_click(move |_, _, cx| {
                        grid.update(cx, |grid, cx| grid.toggle_row_delete(row, cx))
                            .ok();
                    }),
            );
        } else {
            menu = menu.item(PopupMenuItem::separator());
            for (label, format) in [
                ("Copy all rows as CSV", ExportFormat::Csv),
                ("Copy all rows as JSON", ExportFormat::Json),
                ("Copy all rows as SQL INSERT", ExportFormat::Sql),
            ] {
                menu = menu.item(copy_item(
                    label,
                    self.formatted_rows(
                        &(0..self.result.rows.len()).collect::<Vec<_>>(),
                        format,
                        true,
                    ),
                ));
            }
        }
        menu
    }

    fn displayed_cell(&self, row: usize, column: usize) -> String {
        if let Some(value) = self
            .editable
            .as_ref()
            .and_then(|editable| editable.display_value(row, column))
        {
            return value.unwrap_or_default();
        }
        self.result
            .rows
            .get(row)
            .and_then(|row| row.get(column))
            .filter(|value| !value.is_null())
            .map(cell_edit_text)
            .unwrap_or_default()
    }

    fn formatted_rows(&self, rows: &[usize], format: ExportFormat, header: bool) -> String {
        let mut result: QueryResult = (*self.result).clone();
        result.rows = rows
            .iter()
            .filter_map(|row| self.result.rows.get(*row).cloned())
            .collect();
        let table = self
            .editable
            .as_ref()
            .map(|editable| (editable.schema_name(), editable.table_name()));
        let mut bytes = Vec::new();
        if export_result(&mut bytes, &result, format, table).is_err() {
            return String::new();
        }
        let mut text = String::from_utf8(bytes).unwrap_or_default();
        if !header && matches!(format, ExportFormat::Csv | ExportFormat::Tsv) {
            text = text
                .split_once("\r\n")
                .map_or(String::new(), |(_, rows)| rows.to_owned());
        }
        text.trim_end().to_owned()
    }

    fn set_cell_value(
        &mut self,
        row: usize,
        column: usize,
        value: Option<String>,
        cx: &mut gpui::Context<Self>,
    ) {
        if let Some(editable) = &mut self.editable {
            self.edit_error = editable.set_value(row, column, value, &self.result).err();
            cx.notify();
        }
    }
}

fn copy_item(label: &'static str, text: String) -> PopupMenuItem {
    PopupMenuItem::new(label)
        .icon(Icon::empty().path("icons/copy.svg"))
        .on_click(move |_, _, cx: &mut App| {
            cx.write_to_clipboard(ClipboardItem::new_string(text.clone()));
        })
}

fn is_guid_type(data_type: &str) -> bool {
    matches!(
        data_type
            .split(['(', '['])
            .next()
            .unwrap_or(data_type)
            .trim()
            .to_ascii_lowercase()
            .as_str(),
        "uuid" | "guid" | "uniqueidentifier"
    )
}

#[cfg(test)]
mod tests {
    use super::is_guid_type;

    #[test]
    fn guid_types_match_the_classic_grid() {
        assert!(is_guid_type("uuid"));
        assert!(is_guid_type("GUID"));
        assert!(is_guid_type("uniqueidentifier"));
        assert!(!is_guid_type("text"));
    }
}
