use gpui::{Context, Focusable as _, KeyDownEvent, Window};
use gpui_component::input::Copy;

use super::DataGrid;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum GridKeyAction {
    Copy,
    Paste,
    Review,
    RevertAll,
    SetNull,
    NavigateForeignKey,
    Edit,
    CancelOrRevert,
    /// Backspace/Delete: marks gutter-selected rows for delete, otherwise
    /// clears the selected cell into an empty inline editor.
    ClearOrDelete,
    Move(isize, isize),
}

impl DataGrid {
    pub(super) fn key_down(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.editor_has_focus(window, cx) {
            self.editor_key_down(event, window, cx);
            return;
        }
        if self.foreign_key_picker.is_some() && event.keystroke.key == "escape" {
            self.foreign_key_picker = None;
            cx.notify();
            cx.stop_propagation();
            return;
        }
        let modifiers = event.keystroke.modifiers;
        let Some(action) = grid_key_action(
            event.keystroke.key.as_str(),
            modifiers.secondary(),
            modifiers.shift,
        ) else {
            return;
        };
        match action {
            GridKeyAction::Copy => self.copy_selection(cx),
            GridKeyAction::Paste => self.paste_selection(cx),
            GridKeyAction::Review => self.request_review(cx),
            GridKeyAction::RevertAll => self.clear_pending(cx),
            GridKeyAction::SetNull => self.set_selected_null(cx),
            GridKeyAction::NavigateForeignKey => {
                if let Some(position) = self.selection {
                    self.open_foreign_key_cell(position.row, position.column, cx);
                }
            }
            GridKeyAction::Edit => {
                if let Some(position) = self.selection {
                    self.begin_edit(position, None, window, cx);
                }
            }
            GridKeyAction::CancelOrRevert => {
                // A date/time editor stays open after its input blurs.
                if self.active_editor.is_some() {
                    self.cancel_editor(cx);
                } else if !self.selected_rows.is_empty() {
                    self.clear_row_selection();
                    cx.notify();
                } else if let (Some(position), Some(editable)) =
                    (self.selection, self.editable.as_mut())
                {
                    if editable.revert_cell(position.row, position.column) {
                        self.edit_error = None;
                        cx.notify();
                    }
                }
            }
            GridKeyAction::ClearOrDelete => {
                if !self.selected_rows.is_empty() {
                    self.delete_selected_row(cx);
                } else if let Some(position) = self.selection {
                    self.begin_edit(position, Some(String::new()), window, cx);
                }
            }
            GridKeyAction::Move(row, column) => self.move_selection(row, column, cx),
        }
        cx.stop_propagation();
    }

    /// The Edit menu's Copy item and any Cmd/Ctrl+C key binding resolve to
    /// GPUI's [`Copy`] action. Handle it while the grid itself holds focus, so
    /// the menu bar copies the selected cells and rows instead of doing
    /// nothing. A focused input inside the grid (a cell editor) keeps its own
    /// copy behaviour because the grid is no longer the focused element.
    pub(super) fn copy_action(&mut self, _: &Copy, window: &mut Window, cx: &mut Context<Self>) {
        if !self.focus_handle.is_focused(window) {
            return;
        }
        self.copy_selection(cx);
        cx.stop_propagation();
    }

    /// Whether keyboard focus is inside the open cell editor (its text input
    /// or a date/time editor's time input). Date/time editors do not commit on
    /// blur, so an editor can stay open while the grid itself has focus; the
    /// grid shortcuts must keep working then.
    fn editor_has_focus(&self, window: &Window, cx: &gpui::App) -> bool {
        self.active_editor.as_ref().is_some_and(|editor| {
            editor.state.focus_handle(cx).is_focused(window)
                || editor
                    .time
                    .as_ref()
                    .is_some_and(|time| time.focus_handle(cx).is_focused(window))
        })
    }

    /// Keys the inline input lets bubble (Enter, Escape, arrows) arrive here.
    /// Only commit/cancel are meaningful; everything else must not reach the
    /// grid shortcuts, or editing a cell could move or delete rows.
    fn editor_key_down(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match event.keystroke.key.as_str() {
            "enter" => self.commit_editor(cx),
            "escape" => self.cancel_editor(cx),
            _ => return,
        }
        window.focus(&self.focus_handle);
        cx.stop_propagation();
    }
}

fn grid_key_action(key: &str, secondary: bool, shift: bool) -> Option<GridKeyAction> {
    if secondary {
        return match (key, shift) {
            ("c", false) => Some(GridKeyAction::Copy),
            ("v", false) => Some(GridKeyAction::Paste),
            ("s", false) => Some(GridKeyAction::Review),
            ("z", true) => Some(GridKeyAction::RevertAll),
            ("g", false) => Some(GridKeyAction::NavigateForeignKey),
            ("backspace", false) | ("delete", false) => Some(GridKeyAction::SetNull),
            _ => None,
        };
    }
    match key {
        "enter" => Some(GridKeyAction::Edit),
        "escape" => Some(GridKeyAction::CancelOrRevert),
        "backspace" | "delete" => Some(GridKeyAction::ClearOrDelete),
        "left" => Some(GridKeyAction::Move(0, -1)),
        "right" => Some(GridKeyAction::Move(0, 1)),
        "up" => Some(GridKeyAction::Move(-1, 0)),
        "down" => Some(GridKeyAction::Move(1, 0)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keymap_routes_canonical_grid_shortcuts_without_shadowing_plain_delete() {
        assert_eq!(
            grid_key_action("enter", false, false),
            Some(GridKeyAction::Edit)
        );
        assert_eq!(
            grid_key_action("s", true, false),
            Some(GridKeyAction::Review)
        );
        assert_eq!(
            grid_key_action("z", true, true),
            Some(GridKeyAction::RevertAll)
        );
        assert_eq!(
            grid_key_action("backspace", true, false),
            Some(GridKeyAction::SetNull)
        );
        assert_eq!(
            grid_key_action("delete", false, false),
            Some(GridKeyAction::ClearOrDelete)
        );
        assert_eq!(
            grid_key_action("backspace", false, false),
            Some(GridKeyAction::ClearOrDelete)
        );
        assert_eq!(grid_key_action("k", true, false), None);
        assert_eq!(grid_key_action("n", true, false), None);
        assert_eq!(grid_key_action("t", true, false), None);
        assert_eq!(grid_key_action("f", true, false), None);
        assert_eq!(grid_key_action("enter", true, false), None);
    }
}
