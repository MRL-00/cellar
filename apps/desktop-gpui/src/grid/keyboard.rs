use gpui::{Context, KeyDownEvent, Window};

use super::DataGrid;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum GridKeyAction {
    Copy,
    Paste,
    Review,
    RevertAll,
    SetNull,
    Edit,
    CancelOrRevert,
    DeleteRow,
    Move(isize, isize),
}

impl DataGrid {
    pub(super) fn key_down(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
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
            GridKeyAction::Edit => {
                if let Some(position) = self.selection {
                    self.begin_edit(position, window, cx);
                }
            }
            GridKeyAction::CancelOrRevert => {
                if self.active_editor.is_some() {
                    self.cancel_editor(cx);
                } else if let (Some(position), Some(editable)) =
                    (self.selection, self.editable.as_mut())
                {
                    if editable.revert_cell(position.row, position.column) {
                        self.edit_error = None;
                        cx.notify();
                    }
                }
            }
            GridKeyAction::DeleteRow => self.delete_selected_row(cx),
            GridKeyAction::Move(row, column) => self.move_selection(row, column, cx),
        }
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
            ("backspace", false) | ("delete", false) => Some(GridKeyAction::SetNull),
            _ => None,
        };
    }
    match key {
        "enter" => Some(GridKeyAction::Edit),
        "escape" => Some(GridKeyAction::CancelOrRevert),
        "backspace" | "delete" => Some(GridKeyAction::DeleteRow),
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
            Some(GridKeyAction::DeleteRow)
        );
    }
}
