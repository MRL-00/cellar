use std::sync::Arc;

use cellar_runtime::export::ExportFormat;
use gpui::{ClipboardItem, Context, ScrollStrategy, Window};

use super::{editing, CellPosition, CellRange, DataGrid, EditableGrid};

impl DataGrid {
    pub(super) fn select(
        &mut self,
        position: CellPosition,
        extend: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !extend || self.selection_anchor.is_none() {
            self.selection_anchor = Some(position);
        }
        self.selection = Some(position);
        self.selecting_cells = true;
        self.clear_row_selection();
        window.focus(&self.focus_handle);
        cx.notify();
    }

    pub(super) fn extend_cell_selection(&mut self, position: CellPosition, cx: &mut Context<Self>) {
        if !self.selecting_cells || self.selection == Some(position) {
            return;
        }
        self.selection = Some(position);
        cx.notify();
    }

    pub(super) fn finish_cell_selection(&mut self, cx: &mut Context<Self>) {
        if self.selecting_cells {
            self.selecting_cells = false;
            cx.notify();
        }
    }

    pub(super) fn cell_selection(&self) -> Option<CellRange> {
        self.selection
            .map(|head| CellRange::between(self.selection_anchor.unwrap_or(head), head))
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
        self.selection_anchor = None;
        self.selecting_cells = false;
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
        self.selection_anchor = self.selection;
        self.selecting_cells = false;
        self.clear_row_selection();
        self.vertical_scroll
            .scroll_to_item(row, ScrollStrategy::Center);
        self.reveal_column(column);
        cx.notify();
    }

    pub(super) fn copy_selection(&self, cx: &mut Context<Self>) {
        if !self.selected_rows.is_empty() {
            let rows = self.selected_rows.iter().copied().collect::<Vec<_>>();
            // JSON objects carry column names. GPUI writes one plain-text
            // clipboard entry, so a second TSV flavor would be dropped.
            let text = self.formatted_rows(&rows, ExportFormat::Json, true);
            if !text.is_empty() {
                cx.write_to_clipboard(ClipboardItem::new_string(text));
            }
            return;
        }
        let Some(selection) = self.cell_selection() else {
            return;
        };
        let text = self.formatted_cells(selection);
        if !text.is_empty() {
            cx.write_to_clipboard(ClipboardItem::new_string(text));
        }
    }

    pub(super) fn paste_selection(&mut self, cx: &mut Context<Self>) {
        if self.reloading {
            return;
        }
        let Some(start) = self.selection else {
            return;
        };
        if !self.editable.as_ref().is_some_and(EditableGrid::can_edit) {
            return;
        }
        let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) else {
            return;
        };
        self.clear_row_selection();
        let Some(editable) = &mut self.editable else {
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
    /// above it. When every selected row is already marked for delete the
    /// action unmarks them all; otherwise it marks every unmarked row so a
    /// mixed selection never un-deletes rows.
    pub(super) fn toggle_row_deletes(&mut self, rows: &[usize], cx: &mut Context<Self>) {
        if self.reloading {
            return;
        }
        let Some(editable) = &mut self.editable else {
            return;
        };
        let deleted = editable
            .deleted_rows()
            .into_iter()
            .collect::<std::collections::BTreeSet<_>>();
        let all_deleted = !rows.is_empty() && rows.iter().all(|row| deleted.contains(row));
        let mut removed = false;
        for &row in rows.iter().rev() {
            if !all_deleted && deleted.contains(&row) {
                continue;
            }
            if editable.toggle_delete(row) {
                Arc::make_mut(&mut self.result).rows.remove(row);
                removed = true;
            }
        }
        if removed {
            self.selection = None;
            self.selection_anchor = None;
            self.selecting_cells = false;
            self.clear_row_selection();
        }
        cx.notify();
    }
}

#[cfg(test)]
mod tests {
    use super::{CellPosition, CellRange};

    #[test]
    fn cell_range_normalizes_drag_direction() {
        let range = CellRange::between(
            CellPosition { row: 5, column: 3 },
            CellPosition { row: 2, column: 1 },
        );

        assert_eq!(range.start, CellPosition { row: 2, column: 1 });
        assert_eq!(range.end, CellPosition { row: 5, column: 3 });
        assert!(range.contains(CellPosition { row: 4, column: 2 }));
        assert!(!range.contains(CellPosition { row: 4, column: 4 }));
    }
}
