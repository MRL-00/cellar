use cellar_runtime::export::ExportFormat;
use gpui::{div, prelude::*, px, IntoElement, WeakEntity};

use super::DataGrid;
use crate::theme::{ACCENT, BORDER, FG_MUTED, PANEL, PANEL_RAISED, PROD, WARN};

pub(super) fn pending_bar(
    grid: WeakEntity<DataGrid>,
    pending_count: usize,
    error: Option<&str>,
) -> impl IntoElement {
    let add = grid.clone();
    let set_null = grid.clone();
    let delete = grid.clone();
    let toggle_bool = grid.clone();
    let revert = grid.clone();
    let review = grid;
    div()
        .h(px(34.))
        .flex_shrink_0()
        .flex()
        .items_center()
        .gap_3()
        .px_3()
        .bg(PANEL_RAISED)
        .border_t_1()
        .border_color(BORDER)
        .child(
            div()
                .text_color(WARN)
                .child(format!("{pending_count} pending edits")),
        )
        .child(action("Add Row").on_click(move |_, window, cx| {
            add.update(cx, |grid, cx| grid.add_row(window, cx)).ok();
        }))
        .child(action("Set NULL").on_click(move |_, _, cx| {
            set_null.update(cx, DataGrid::set_selected_null).ok();
        }))
        .child(action("Toggle Bool").on_click(move |_, _, cx| {
            toggle_bool.update(cx, DataGrid::toggle_selected_bool).ok();
        }))
        .child(
            action("Delete Row")
                .text_color(WARN)
                .on_click(move |_, _, cx| {
                    delete.update(cx, DataGrid::delete_selected_row).ok();
                }),
        )
        .child(action("Revert").on_click(move |_, _, cx| {
            revert.update(cx, DataGrid::clear_pending).ok();
        }))
        .child(
            div()
                .id("review-grid-edits")
                .px_2()
                .py_1()
                .bg(if pending_count > 0 { ACCENT } else { PANEL })
                .text_color(if pending_count > 0 { PANEL } else { FG_MUTED })
                .child("Review & Commit")
                .when(pending_count > 0, |element| {
                    element
                        .tab_index(0)
                        .cursor_pointer()
                        .on_click(move |_, _, cx| {
                            review.update(cx, DataGrid::review_changes).ok();
                        })
                }),
        )
        .when_some(error.map(str::to_owned), |element, error| {
            element.child(div().text_color(PROD).child(error))
        })
}

pub(super) fn export_bar(
    grid: WeakEntity<DataGrid>,
    editable: bool,
    message: Option<&Result<String, String>>,
) -> impl IntoElement {
    let csv = grid.clone();
    let tsv = grid.clone();
    let json = grid.clone();
    let sql = grid.clone();
    let import = grid;
    div()
        .h(px(30.))
        .flex_shrink_0()
        .flex()
        .items_center()
        .gap_3()
        .px_3()
        .bg(PANEL_RAISED)
        .border_t_1()
        .border_color(BORDER)
        .child(div().text_color(FG_MUTED).child("Export"))
        .child(action("CSV").on_click(move |_, _, cx| {
            csv.update(cx, |grid, cx| grid.begin_export(ExportFormat::Csv, cx))
                .ok();
        }))
        .child(action("TSV").on_click(move |_, _, cx| {
            tsv.update(cx, |grid, cx| grid.begin_export(ExportFormat::Tsv, cx))
                .ok();
        }))
        .child(action("JSON").on_click(move |_, _, cx| {
            json.update(cx, |grid, cx| grid.begin_export(ExportFormat::Json, cx))
                .ok();
        }))
        .child(action("SQL").on_click(move |_, _, cx| {
            sql.update(cx, |grid, cx| grid.begin_export(ExportFormat::Sql, cx))
                .ok();
        }))
        .when(editable, |element| {
            element.child(action("Import CSV").on_click(move |_, _, cx| {
                import.update(cx, DataGrid::request_csv_import).ok();
            }))
        })
        .when_some(message.cloned(), |element, message| match message {
            Ok(message) => {
                element.child(div().flex_1().truncate().text_color(ACCENT).child(message))
            }
            Err(message) => {
                element.child(div().flex_1().truncate().text_color(PROD).child(message))
            }
        })
}

fn action(label: &'static str) -> gpui::Stateful<gpui::Div> {
    div()
        .id(label)
        .tab_index(0)
        .cursor_pointer()
        .text_color(FG_MUTED)
        .child(label)
}
