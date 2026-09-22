use cellar_core::query::ForeignKeyLookupRequest;
use gpui::Context;

use super::{foreign_key_editing::ForeignKeyNavigationOption, DataGrid, DataGridEvent};

#[derive(Clone)]
pub(super) struct ForeignKeyPicker {
    pub(super) row: usize,
    pub(super) column: usize,
    pub(super) options: Vec<ForeignKeyNavigationOption>,
}

impl DataGrid {
    pub fn set_navigation_error(&mut self, error: impl Into<String>, cx: &mut Context<Self>) {
        self.edit_error = Some(error.into());
        cx.notify();
    }

    pub(super) fn foreign_key_navigation_options(
        &self,
        row: usize,
        column: usize,
    ) -> Result<Vec<ForeignKeyNavigationOption>, String> {
        self.editable
            .as_ref()
            .ok_or_else(|| "Foreign-key navigation is available for table rows only".to_owned())?
            .foreign_key_navigation_options(&self.result, row, column)
    }

    pub(super) fn select_foreign_key_option(
        &mut self,
        option: Result<(ForeignKeyLookupRequest, String), String>,
        cx: &mut Context<Self>,
    ) {
        self.foreign_key_picker = None;
        cx.notify();
        match option {
            Ok((lookup, focus_column)) => {
                let Some(source) = self
                    .editable
                    .as_ref()
                    .map(|editable| editable.target().clone())
                else {
                    self.set_navigation_error(
                        "Foreign-key navigation is available for table rows only",
                        cx,
                    );
                    return;
                };
                cx.emit(DataGridEvent::NavigateForeignKey {
                    source,
                    lookup,
                    focus_column,
                });
            }
            Err(error) => self.set_navigation_error(error, cx),
        }
    }

    pub(super) fn open_foreign_key_cell(
        &mut self,
        row: usize,
        column: usize,
        cx: &mut Context<Self>,
    ) {
        if self.reloading {
            self.set_navigation_error("Wait for the table reload to finish before navigating", cx);
            return;
        }
        match self.foreign_key_navigation_options(row, column) {
            Ok(options) if options.len() == 1 => {
                self.select_foreign_key_option(options[0].lookup.clone(), cx);
            }
            Ok(options) => {
                self.foreign_key_picker = Some(ForeignKeyPicker {
                    row,
                    column,
                    options,
                });
                self.edit_error = None;
                cx.notify();
            }
            Err(error) => self.set_navigation_error(error, cx),
        }
    }
}
