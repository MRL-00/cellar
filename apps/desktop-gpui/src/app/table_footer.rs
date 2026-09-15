use std::time::Duration;

use gpui::{
    div, percentage, prelude::*, Animation, AnimationExt, Context, Div, Entity, SharedString,
    Transformation, Window,
};
use gpui_component::{
    button::{Button, ButtonVariants},
    menu::{DropdownMenu, PopupMenuItem},
    Disableable, Icon, Sizable,
};

use super::CellarApp;
use cellar_desktop_gpui::{
    grid::DataGrid,
    model::TablePage,
    theme::{
        ui_px, DynamicColor, ACCENT, BORDER, BORDER_DIVIDER, FG, FG_MUTED, FG_SECONDARY,
        PANEL_RAISED, PROD, WARN,
    },
};
use cellar_runtime::export::ExportFormat;

impl CellarApp {
    pub(super) fn table_footer(
        &self,
        tab_id: u64,
        page: TablePage,
        grid: Entity<DataGrid>,
        reloading: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let editable = grid.read(cx).can_edit();
        let pending_count = grid.read(cx).pending_count();
        let status = grid
            .read(cx)
            .edit_error()
            .map(|error| (error.to_owned(), PROD))
            .or_else(|| {
                grid.read(cx).export_message().map(|message| match message {
                    Ok(message) => (message.clone(), ACCENT),
                    Err(message) => (message.clone(), PROD),
                })
            });
        let export_grid = grid.clone();
        let mut bar = div()
            .h(ui_px(36.))
            .flex_shrink_0()
            .flex()
            .items_center()
            .gap(ui_px(2.))
            .px_3()
            .border_t_1()
            .border_color(BORDER)
            .bg(PANEL_RAISED)
            .text_size(ui_px(11.5))
            .text_color(FG_SECONDARY)
            .child(
                Button::new(SharedString::from(format!("grid-export:{tab_id}")))
                    .child(footer_icon("icons/download.svg", FG))
                    .tooltip("Export")
                    .ghost()
                    .small()
                    .compact()
                    .dropdown_caret(true)
                    .dropdown_menu(move |menu, _, _| {
                        menu.item(export_item("CSV", ExportFormat::Csv, export_grid.clone()))
                            .item(export_item("TSV", ExportFormat::Tsv, export_grid.clone()))
                            .item(export_item("JSON", ExportFormat::Json, export_grid.clone()))
                            .item(export_item("SQL", ExportFormat::Sql, export_grid.clone()))
                    }),
            );
        if editable {
            bar = bar
                .child(
                    Button::new(SharedString::from(format!("grid-import:{tab_id}")))
                        .child(footer_icon("icons/upload.svg", FG))
                        .tooltip("Import CSV")
                        .ghost()
                        .small()
                        .compact()
                        .on_click({
                            let grid = grid.clone();
                            move |_, _, cx| {
                                grid.update(cx, |grid, cx| grid.request_csv_import(cx));
                            }
                        }),
                )
                .child(footer_divider())
                .child(
                    div()
                        .px_1()
                        .text_color(if pending_count > 0 { WARN } else { FG_MUTED })
                        .child(format!("{pending_count} pending")),
                )
                .child(grid_action(
                    format!("grid-add:{tab_id}"),
                    "icons/plus.svg",
                    "Add row",
                    FG,
                    grid.clone(),
                    |grid, window, cx| grid.add_row(window, cx),
                ))
                .child(grid_action(
                    format!("grid-bool:{tab_id}"),
                    "icons/type-bool.svg",
                    "Toggle boolean",
                    FG,
                    grid.clone(),
                    |grid, _, cx| grid.toggle_selected_bool(cx),
                ))
                .child(grid_action(
                    format!("grid-delete:{tab_id}"),
                    "icons/trash.svg",
                    "Delete row",
                    WARN,
                    grid.clone(),
                    |grid, _, cx| grid.delete_selected_row(cx),
                ))
                .child(grid_action(
                    format!("grid-revert:{tab_id}"),
                    "icons/undo.svg",
                    "Revert pending edits",
                    FG,
                    grid.clone(),
                    |grid, _, cx| grid.clear_pending(cx),
                ))
                .child(
                    Button::new(SharedString::from(format!("grid-review:{tab_id}")))
                        .child(footer_icon(
                            "icons/commit.svg",
                            if pending_count > 0 { ACCENT } else { FG_MUTED },
                        ))
                        .tooltip("Review & commit")
                        .ghost()
                        .small()
                        .compact()
                        .disabled(pending_count == 0)
                        .on_click(move |_, _, cx| {
                            grid.update(cx, |grid, cx| grid.request_review(cx));
                        }),
                );
        }
        bar = bar
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .justify_end()
                    .gap_2()
                    .when_some(status, |element, (message, color)| {
                        element.child(div().min_w_0().truncate().text_color(color).child(message))
                    }),
            )
            .when(reloading, |element| {
                element.child(
                    Icon::empty()
                        .path("icons/spinner.svg")
                        .size(ui_px(13.))
                        .text_color(ACCENT)
                        .with_animation(
                            "table-reload-spinner",
                            Animation::new(Duration::from_millis(900)).repeat(),
                            |icon, delta| icon.transform(Transformation::rotate(percentage(delta))),
                        ),
                )
            })
            .child(div().flex_shrink_0().child(page_label(page)))
            .child(
                Button::new(SharedString::from(format!("grid-prev:{tab_id}")))
                    .child(footer_icon("icons/chevron-left.svg", FG))
                    .tooltip("Previous page")
                    .ghost()
                    .small()
                    .compact()
                    .disabled(reloading || !page.has_previous())
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.change_table_page(tab_id, false, cx);
                    })),
            )
            .child(
                Button::new(SharedString::from(format!("grid-next:{tab_id}")))
                    .child(footer_icon("icons/chevron-right.svg", FG))
                    .tooltip("Next page")
                    .ghost()
                    .small()
                    .compact()
                    .disabled(reloading || !page.has_next())
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.change_table_page(tab_id, true, cx);
                    })),
            );
        bar
    }
}

pub(super) fn page_label(page: TablePage) -> String {
    if page.rows == 0 {
        return "0 rows".into();
    }
    let first = u64::from(page.offset) + 1;
    let last = u64::from(page.offset) + u64::from(page.rows);
    match page.total_rows {
        Some(total) => format!("{first}–{last} of {total}"),
        None => format!("{first}–{last}"),
    }
}

fn footer_icon(path: &'static str, color: DynamicColor) -> Icon {
    Icon::empty()
        .path(path)
        .size(ui_px(17.))
        .text_color(color)
}

fn footer_divider() -> Div {
    div().w(ui_px(1.)).h(ui_px(14.)).mx_1().bg(BORDER_DIVIDER)
}

fn export_item(
    label: &'static str,
    format: ExportFormat,
    grid: Entity<DataGrid>,
) -> PopupMenuItem {
    PopupMenuItem::new(label).on_click(move |_, _, cx| {
        grid.update(cx, |grid, cx| grid.begin_export(format, cx));
    })
}

fn grid_action(
    id: String,
    icon: &'static str,
    tooltip: &'static str,
    color: DynamicColor,
    grid: Entity<DataGrid>,
    action: impl Fn(&mut DataGrid, &mut Window, &mut Context<DataGrid>) + 'static,
) -> Button {
    Button::new(SharedString::from(id))
        .child(footer_icon(icon, color))
        .tooltip(tooltip)
        .ghost()
        .small()
        .compact()
        .on_click(move |_, window, cx| {
            grid.update(cx, |grid, cx| action(grid, window, cx));
        })
}
