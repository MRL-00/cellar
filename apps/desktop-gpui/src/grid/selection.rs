use std::sync::Arc;

use cellar_runtime::export::ExportFormat;
use gpui::{ClipboardItem, Context, ScrollStrategy, Window};

use super::row::clipboard_text;
use super::{editing, CellPosition, DataGrid};

impl DataGrid {
    pub(super) fn select(
        &mut self,
        position: CellPosition,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.selection = Some(position);
        self.clear_row_selection();
        window.focus(&self.focus_handle);
        cx.notify();
    }

    /// Row-number gutter clicks: plain click selects just the row, the
    /// secondary modifier (Cmd on macOS, Ctrl elsewhere) toggles it in the
    /// multi-selection, and Shift extends a range from the last anchor row.
    pub(super) fn select_row(
        &mut self,
        row: usize,
        additive: bool,
        range: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if range {
            let anchor = *self.row_anchor.get_or_insert(row);
            if !additive {
                self.selected_rows.clear();
            }
            self.selected_rows.extend(anchor.min(row)..=anchor.max(row));
        } else if additive {
            if !self.selected_rows.remove(&row) {
                self.selected_rows.insert(row);
            }
            self.row_anchor = Some(row);
        } else {
            self.selected_rows.clear();
            self.selected_rows.insert(row);
            self.row_anchor = Some(row);
        }
        self.selection = Some(CellPosition { row, column: 0 });
        window.focus(&self.focus_handle);
        cx.notify();
    }

    pub(super) fn clear_row_selection(&mut self) {
        self.selected_rows.clear();
        self.row_anchor = None;
    }

    pub(super) fn move_selection(
        &mut self,
        row_delta: isize,
        column_delta: isize,
        cx: &mut Context<Self>,
    ) {
        if self.result.rows.is_empty() || self.result.columns.is_empty() {
            return;
        }
        let current = self.selection.unwrap_or(CellPosition { row: 0, column: 0 });
        let row = current
            .row
            .saturating_add_signed(row_delta)
            .min(self.result.rows.len() - 1);
        let column = current
            .column
            .saturating_add_signed(column_delta)
            .min(self.result.columns.len() - 1);
        self.selection = Some(CellPosition { row, column });
        self.clear_row_selection();
        self.vertical_scroll
            .scroll_to_item(row, ScrollStrategy::Center);
        self.reveal_column(column);
        cx.notify();
    }

    pub(super) fn copy_selection(&self, cx: &mut Context<Self>) {
        if !self.selected_rows.is_empty() {
            let rows = self.selected_rows.iter().copied().collect::<Vec<_>>();
            let text = self.formatted_rows(&rows, ExportFormat::Tsv, false);
            if !text.is_empty() {
                cx.write_to_clipboard(ClipboardItem::new_string(text));
            }
            return;
        }
        let Some(position) = self.selection else {
            return;
        };
        let text = self
            .editable
            .as_ref()
            .and_then(|editable| editable.display_value(position.row, position.column))
            .map(|value| value.unwrap_or_else(|| "NULL".into()))
            .or_else(|| {
                self.result
                    .rows
                    .get(position.row)
                    .and_then(|row| row.get(position.column))
                    .map(clipboard_text)
            });
        if let Some(text) = text {
            cx.write_to_clipboard(ClipboardItem::new_string(text));
        }
    }

    pub(super) fn paste_selection(&mut self, cx: &mut Context<Self>) {
        let Some(start) = self.selection else {
            return;
        };
        let Some(editable) = &mut self.editable else {
            return;
        };
        if !editable.can_edit() {
            return;
        }
        let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) else {
            return;
        };
        for (row_offset, values) in editing::clipboard_rows(&text).into_iter().enumerate() {
            let row = start.row.saturating_add(row_offset);
            if row >= self.result.rows.len() {
                break;
            }
            for (column_offset, value) in values.into_iter().enumerate() {
                let column = start.column.saturating_add(column_offset);
                if column >= self.result.columns.len() {
                    break;
                }
                if let Err(error) = editable.set_value(row, column, Some(value), &self.result) {
                    self.edit_error = Some(error);
                    cx.notify();
                    return;
                }
            }
        }
        cx.notify();
    }

    pub fn delete_selected_row(&mut self, cx: &mut Context<Self>) {
        self.commit_editor(cx);
        if self.edit_error.is_some() {
            return;
        }
        if self.selected_rows.is_empty() {
            let Some(position) = self.selection else {
                return;
            };
            self.toggle_row_delete(position.row, cx);
            return;
        }
        let rows = self.selected_rows.iter().copied().collect::<Vec<_>>();
        self.toggle_row_deletes(&rows, cx);
    }

    pub(super) fn toggle_row_delete(&mut self, row: usize, cx: &mut Context<Self>) {
        self.toggle_row_deletes(&[row], cx);
    }

    /// Rows must be processed in descending order: cancelling an inserted row
    /// removes it from `row_keys` and `result.rows`, which shifts every index
    /// above it.
    pub(super) fn toggle_row_deletes(&mut self, rows: &[usize], cx: &mut Context<Self>) {
        let Some(editable) = &mut self.editable else {
            return;
        };
        let mut removed = false;
        for &row in rows.iter().rev() {
            if editable.toggle_delete(row) {
                Arc::make_mut(&mut self.result).rows.remove(row);
                removed = true;
            }
        }
        if removed {
            self.selection = None;
            self.clear_row_selection();
        }
        cx.notify();
    }
}
