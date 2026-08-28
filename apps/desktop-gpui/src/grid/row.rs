use std::{collections::BTreeMap, ops::Range, sync::Arc};

use cellar_core::{
    query::{QueryResult, SortDirection},
    value::{CellValue, ColumnMeta},
};
use gpui::{
    div, prelude::*, px, App, Context, IntoElement, MouseButton, Pixels, Point, Render, RenderOnce,
    SharedString, WeakEntity, Window,
};
use gpui_component::menu::ContextMenuExt;
use gpui_component::Icon;

use super::rich::rich_cell_content;
use super::{width_sum, CellPosition, DataGrid, DragColumn, FROZEN_COLUMNS, ROW_NUMBER_WIDTH};
use crate::theme::{
    accent, accent_soft, ACCENT, BORDER_DIVIDER, DELETE_SOFT, FG, FG_MUTED, FG_SECONDARY,
    GRID_LINE, INSERT_SOFT, PANEL, PANEL_MUTED, PANEL_RAISED, PROD, UPDATE_SOFT, WARN,
};

struct DragPreview {
    label: String,
    position: Point<Pixels>,
}

impl Render for DragPreview {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().pl(self.position.x).pt(self.position.y).child(
            div()
                .px_2()
                .py_1()
                .bg(PANEL_RAISED)
                .border_1()
                .border_color(ACCENT)
                .child(self.label.clone()),
        )
    }
}

#[derive(IntoElement)]
pub(super) struct GridRow {
    pub result: Arc<QueryResult>,
    pub row: usize,
    pub columns: Range<usize>,
    pub horizontal_offset: f32,
    pub selection: Option<CellPosition>,
    pub pending: Arc<BTreeMap<(usize, usize), Option<String>>>,
    pub inserted: bool,
    pub deleted: bool,
    pub editable: bool,
    pub null_display: Arc<str>,
    pub stripe_rows: bool,
    pub grid: WeakEntity<DataGrid>,
    pub column_widths: Arc<Vec<f32>>,
}

impl RenderOnce for GridRow {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let total_columns = self.result.columns.len();
        let frozen = FROZEN_COLUMNS.min(total_columns);
        let row_background = row_background(self.stripe_rows, self.row);
        div()
            .flex()
            .h(px(crate::theme::row_height()))
            .w(px(
                ROW_NUMBER_WIDTH + width_sum(&self.column_widths, 0..total_columns)
            ))
            .bg(if self.deleted {
                DELETE_SOFT.rgba()
            } else if self.inserted {
                INSERT_SOFT.rgba()
            } else {
                row_background
            })
            .border_b_1()
            .border_color(BORDER_DIVIDER)
            .child(
                div()
                    .relative()
                    .left(px(self.horizontal_offset))
                    .flex()
                    .flex_shrink_0()
                    .bg(if self.deleted {
                        DELETE_SOFT.rgba()
                    } else if self.inserted {
                        INSERT_SOFT.rgba()
                    } else {
                        row_background
                    })
                    .child(
                        div()
                            .w(px(ROW_NUMBER_WIDTH))
                            .flex_shrink_0()
                            .flex()
                            .items_center()
                            .justify_center()
                            .text_size(px(11.))
                            .text_color(FG_MUTED)
                            .child((self.row + 1).to_string())
                            .context_menu({
                                let grid = self.grid.clone();
                                let row = self.row;
                                move |menu, _, cx| {
                                    let Some(entity) = grid.upgrade() else {
                                        return menu;
                                    };
                                    entity.update(cx, |this, _| {
                                        this.row_context_menu(menu, row, grid.clone())
                                    })
                                }
                            }),
                    )
                    .children((0..frozen).map(|column| self.cell(column))),
            )
            .child(
                div()
                    .w(px(width_sum(
                        &self.column_widths,
                        frozen..self.columns.start,
                    )))
                    .flex_shrink_0(),
            )
            .children(self.columns.clone().map(|column| self.cell(column)))
            .child(
                div()
                    .w(px(width_sum(
                        &self.column_widths,
                        self.columns.end..total_columns,
                    )))
                    .flex_shrink_0(),
            )
    }
}

fn row_background(stripe_rows: bool, row: usize) -> gpui::Rgba {
    if !stripe_rows || row.is_multiple_of(2) {
        PANEL.rgba()
    } else {
        PANEL_MUTED.rgba()
    }
}

impl GridRow {
    fn cell(&self, column: usize) -> impl IntoElement {
        grid_cell(
            Arc::clone(&self.result),
            self.row,
            column,
            self.selection
                == Some(CellPosition {
                    row: self.row,
                    column,
                }),
            self.pending.get(&(self.row, column)).cloned(),
            self.inserted,
            self.deleted,
            self.editable,
            Arc::clone(&self.null_display),
            row_background(self.stripe_rows, self.row),
            self.column_widths[column],
            self.grid.clone(),
        )
    }
}

pub(super) fn header_cell(
    column: &ColumnMeta,
    index: usize,
    width: f32,
    sort: Option<(usize, SortDirection)>,
    primary_key: bool,
    foreign_key: bool,
    grid: WeakEntity<DataGrid>,
) -> impl IntoElement {
    let sorted = sort.is_some_and(|(sorted, _)| sorted == index);
    let sort_icon = if matches!(sort, Some((sorted, SortDirection::Desc)) if sorted == index) {
        "icons/sort-desc.svg"
    } else {
        "icons/sort-asc.svg"
    };
    let (type_icon, type_color) = column_type_icon(&column.data_type, primary_key, foreign_key);
    let resize_grid = grid.clone();
    let drop_grid = grid.clone();
    let menu_grid = grid.clone();
    let drag_label = column.name.clone();
    div()
        .id(SharedString::from(format!("header:{index}")))
        .cursor_pointer()
        .relative()
        .w(px(width))
        .h_full()
        .flex_shrink_0()
        .flex()
        .items_center()
        .gap(px(6.))
        .px(px(8.))
        .border_l_1()
        .border_color(GRID_LINE)
        .bg(if sorted { accent(0.06) } else { PANEL.rgba() })
        .text_color(FG)
        .font_weight(gpui::FontWeight::MEDIUM)
        .whitespace_nowrap()
        .child(
            div()
                .flex_shrink_0()
                .text_color(type_color)
                .child(Icon::empty().path(type_icon).size(px(10.))),
        )
        .child(
            div()
                .min_w_0()
                .flex_1()
                .truncate()
                .child(column.name.clone()),
        )
        .child(
            div()
                .flex_shrink_0()
                .ml_auto()
                .text_size(px(10.5))
                .font_weight(gpui::FontWeight::NORMAL)
                .text_color(FG_MUTED)
                .child(column.data_type.to_lowercase()),
        )
        .child(
            div()
                .flex_shrink_0()
                .text_color(if sorted { ACCENT } else { FG_MUTED })
                .opacity(if sorted { 1. } else { 0.35 })
                .child(Icon::empty().path(sort_icon).size(px(10.))),
        )
        .child(
            div()
                .absolute()
                .right_0()
                .top_0()
                .bottom_0()
                .w(px(6.))
                .cursor_col_resize()
                .on_mouse_down(MouseButton::Left, move |event, _, cx| {
                    cx.stop_propagation();
                    resize_grid
                        .update(cx, |grid, cx| {
                            if event.click_count >= 2 {
                                grid.auto_fit_column(index, cx);
                            } else {
                                grid.begin_resize(index, f32::from(event.position.x), cx);
                            }
                        })
                        .ok();
                }),
        )
        .on_drag(DragColumn(index), move |_, position, _, cx| {
            cx.new(|_| DragPreview {
                label: drag_label.clone(),
                position,
            })
        })
        .drag_over::<DragColumn>(|style, _, _, _| style.border_l_2().border_color(ACCENT))
        .on_drop(move |drag: &DragColumn, _, cx| {
            drop_grid
                .update(cx, |grid, cx| grid.move_column(drag.0, index, cx))
                .ok();
        })
        .on_click(move |_, _, cx| {
            grid.update(cx, |grid, cx| grid.toggle_sort(index, cx)).ok();
        })
        .context_menu(move |menu, _, cx| {
            let Some(entity) = menu_grid.upgrade() else {
                return menu;
            };
            entity.update(cx, |this, _| {
                this.header_context_menu(menu, index, menu_grid.clone())
            })
        })
}

fn column_type_icon(
    data_type: &str,
    primary_key: bool,
    foreign_key: bool,
) -> (&'static str, crate::theme::DynamicColor) {
    if primary_key {
        return ("icons/type-key.svg", WARN);
    }
    if foreign_key {
        return ("icons/type-link.svg", ACCENT);
    }
    let data_type = data_type.to_ascii_lowercase();
    if [
        "int", "serial", "numeric", "decimal", "real", "double", "float", "uuid",
    ]
    .iter()
    .any(|kind| data_type.contains(kind))
    {
        ("icons/type-hash.svg", FG_MUTED)
    } else if ["date", "time"].iter().any(|kind| data_type.contains(kind)) {
        ("icons/type-calendar.svg", FG_MUTED)
    } else if data_type.contains("bool") {
        ("icons/type-bool.svg", FG_MUTED)
    } else if ["json", "object", "array", "map"]
        .iter()
        .any(|kind| data_type.contains(kind))
    {
        ("icons/type-json.svg", FG_MUTED)
    } else {
        ("icons/type-text.svg", FG_MUTED)
    }
}

fn grid_cell(
    result: Arc<QueryResult>,
    row: usize,
    column: usize,
    selected: bool,
    pending: Option<Option<String>>,
    inserted: bool,
    deleted: bool,
    editable: bool,
    null_display: Arc<str>,
    row_background: gpui::Rgba,
    width: f32,
    grid: WeakEntity<DataGrid>,
) -> impl IntoElement {
    let value = result.rows.get(row).and_then(|row| row.get(column));
    let menu_grid = grid.clone();
    let is_pending = pending.is_some();
    let (text, is_null) = match pending {
        Some(Some(value)) => (value, false),
        Some(None) => (null_display.to_string(), true),
        None => (
            value
                .map(|value| cell_text(value, &null_display))
                .unwrap_or_else(|| null_display.to_string()),
            value.is_none_or(CellValue::is_null),
        ),
    };
    let content = if is_pending {
        div().truncate().child(text).into_any_element()
    } else {
        rich_cell_content(
            row,
            column,
            selected,
            result.columns.get(column),
            value,
            text,
        )
    };
    div()
        .id(SharedString::from(format!("cell:{row}:{column}")))
        .group("grid-cell")
        .cursor_pointer()
        .w(px(width))
        .h_full()
        .flex_shrink_0()
        .flex()
        .items_center()
        .px(px(10.))
        .border_l_1()
        .border_color(GRID_LINE)
        .bg(if deleted {
            DELETE_SOFT.rgba()
        } else if inserted {
            INSERT_SOFT.rgba()
        } else if is_pending {
            UPDATE_SOFT.rgba()
        } else if selected {
            accent_soft()
        } else {
            row_background
        })
        .text_color(if deleted {
            PROD
        } else if is_null {
            FG_MUTED
        } else {
            FG_SECONDARY
        })
        .whitespace_nowrap()
        .truncate()
        .child(content)
        .on_click(move |event, window, cx| {
            grid.update(cx, |grid, cx| {
                let position = CellPosition { row, column };
                if editable && event.click_count() >= 2 {
                    grid.begin_edit(position, window, cx);
                } else {
                    grid.select(position, window, cx);
                }
            })
            .ok();
        })
        .context_menu(move |menu, _, cx| {
            let Some(entity) = menu_grid.upgrade() else {
                return menu;
            };
            entity.update(cx, |this, _| {
                this.cell_context_menu(menu, row, column, menu_grid.clone())
            })
        })
}

fn cell_text(value: &CellValue, null_display: &str) -> String {
    match value {
        CellValue::Null => null_display.into(),
        CellValue::Bool(value) => value.to_string(),
        CellValue::Int(value) => value.to_string(),
        CellValue::Float(value) => value.to_string(),
        CellValue::Numeric(value) | CellValue::Text(value) => value.clone(),
        CellValue::Bytes(value) => format!("<{} bytes>", value.len()),
        CellValue::Json(value) => value.to_string(),
        CellValue::Uuid(value) => value.to_string(),
        CellValue::Date(value) => value.to_string(),
        CellValue::Time(value) => value.to_string(),
        CellValue::Timestamp(value) => value.to_string(),
        CellValue::TimestampTz(value) => value.to_rfc3339(),
    }
}

pub(super) fn cell_edit_text(value: &CellValue) -> String {
    match value {
        CellValue::Null => String::new(),
        CellValue::Bytes(value) => {
            let mut text = String::from("\\x");
            for byte in value {
                text.push_str(&format!("{byte:02x}"));
            }
            text
        }
        value => cell_text(value, "NULL"),
    }
}

pub(super) fn clipboard_text(value: &CellValue) -> String {
    if value.is_null() {
        "NULL".into()
    } else {
        cell_edit_text(value)
    }
}

#[cfg(test)]
mod tests {
    use cellar_core::value::CellValue;

    use super::{cell_text, column_type_icon, row_background};
    use crate::theme::{PANEL, PANEL_MUTED};

    #[test]
    fn grid_display_preferences_control_nulls_and_stripes() {
        assert_eq!(cell_text(&CellValue::Null, "∅"), "∅");
        assert_eq!(row_background(false, 1), PANEL.rgba());
        assert_eq!(row_background(true, 1), PANEL_MUTED.rgba());
    }

    #[test]
    fn grid_header_uses_canonical_type_icons() {
        assert_eq!(
            column_type_icon("text", true, false).0,
            "icons/type-key.svg"
        );
        assert_eq!(
            column_type_icon("text", false, true).0,
            "icons/type-link.svg"
        );
        assert_eq!(
            column_type_icon("bigint", false, false).0,
            "icons/type-hash.svg"
        );
        assert_eq!(
            column_type_icon("timestamp", false, false).0,
            "icons/type-calendar.svg"
        );
        assert_eq!(
            column_type_icon("jsonb", false, false).0,
            "icons/type-json.svg"
        );
    }
}
