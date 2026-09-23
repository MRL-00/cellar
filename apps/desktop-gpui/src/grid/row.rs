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

use super::json::JsonPalette;
use super::rich::rich_cell_content;
use super::{
    width_sum, CellPosition, CellRange, DataGrid, DragColumn, FROZEN_COLUMNS, ROW_NUMBER_WIDTH,
};
use crate::theme::{
    accent, accent_soft, opaque_over, ACCENT, ACCENT_FG, BORDER_DIVIDER, DELETE_SOFT, FG, FG_MUTED,
    FG_SECONDARY, GRID_LINE, INSERT_SOFT, PANEL, PANEL_MUTED, PANEL_RAISED, PROD, UPDATE_SOFT,
    WARN,
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
    pub cell_selection: Option<CellRange>,
    pub row_selected: bool,
    pub pending: Arc<BTreeMap<(usize, usize), Option<String>>>,
    pub inserted: bool,
    pub deleted: bool,
    pub editable: bool,
    pub null_display: Arc<str>,
    pub stripe_rows: bool,
    pub grid: WeakEntity<DataGrid>,
    pub column_widths: Arc<Vec<f32>>,
    pub json_palette: JsonPalette,
}

impl RenderOnce for GridRow {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let total_columns = self.result.columns.len();
        let frozen = FROZEN_COLUMNS.min(total_columns);
        let row_background = if self.row_selected {
            accent_soft()
        } else {
            row_background(self.stripe_rows, self.row)
        };
        let row_tint = if self.deleted {
            DELETE_SOFT.rgba()
        } else if self.inserted {
            INSERT_SOFT.rgba()
        } else {
            row_background
        };
        // The frozen pane covers the columns that scroll beneath it, so it needs
        // an opaque version of the row background: every tint this row can carry
        // flattened onto the grid's panel color.
        let pane_background = pane_background(row_tint);
        div()
            .flex()
            .h(px(crate::theme::row_height()))
            .w(px(
                ROW_NUMBER_WIDTH + width_sum(&self.column_widths, 0..total_columns)
            ))
            .bg(row_tint)
            .border_b_1()
            .border_color(BORDER_DIVIDER)
            .child(
                div()
                    .w(px(width_sum(&self.column_widths, 0..self.columns.start)))
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
            .child(
                div()
                    .absolute()
                    .left(px(self.horizontal_offset))
                    .top_0()
                    .bottom_0()
                    .flex()
                    .flex_shrink_0()
                    .bg(pane_background)
                    .child(
                        div()
                            .w(px(ROW_NUMBER_WIDTH))
                            .flex_shrink_0()
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor_pointer()
                            .text_size(px(11.))
                            .text_color(FG_MUTED)
                            .when(self.row_selected, |element| {
                                element.bg(ACCENT).text_color(ACCENT_FG)
                            })
                            .when(
                                !self.row_selected
                                    && self
                                        .selection
                                        .is_some_and(|selection| selection.row == self.row),
                                |element| element.bg(accent_soft()),
                            )
                            .child((self.row + 1).to_string())
                            .on_mouse_down(MouseButton::Left, {
                                let grid = self.grid.clone();
                                let row = self.row;
                                move |event, window, cx| {
                                    grid.update(cx, |grid, cx| {
                                        grid.select_row(
                                            row,
                                            event.modifiers.secondary(),
                                            event.modifiers.shift,
                                            window,
                                            cx,
                                        );
                                    })
                                    .ok();
                                }
                            })
                            .context_menu({
                                let grid = self.grid.clone();
                                let row = self.row;
                                move |menu, window, cx| {
                                    let Some(entity) = grid.upgrade() else {
                                        return menu;
                                    };
                                    entity.update(cx, |this, grid_cx| {
                                        if !this.selected_rows.contains(&row) {
                                            this.select_row(row, false, false, window, grid_cx);
                                        }
                                        this.row_context_menu(menu, row, grid.clone())
                                    })
                                }
                            }),
                    )
                    .children((0..frozen).map(|column| self.cell(column))),
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

/// Rows paint translucent tints (`accent_soft`, delete/insert highlights) over
/// the grid's panel color. The frozen pane hides the columns that scroll
/// underneath it, so it has to paint the same result as one opaque color.
fn pane_background(row_tint: gpui::Rgba) -> gpui::Rgba {
    opaque_over(PANEL.rgba(), row_tint)
}

impl GridRow {
    fn cell(&self, column: usize) -> impl IntoElement {
        grid_cell(
            Arc::clone(&self.result),
            self.row,
            column,
            self.cell_selection.is_some_and(|selection| {
                selection.contains(CellPosition {
                    row: self.row,
                    column,
                })
            }),
            self.pending.get(&(self.row, column)).cloned(),
            self.inserted,
            self.deleted,
            self.editable,
            Arc::clone(&self.null_display),
            if self.row_selected {
                accent_soft()
            } else {
                row_background(self.stripe_rows, self.row)
            },
            self.column_widths[column],
            self.json_palette,
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

/// The glyph and tint a column header uses for its engine-native type. Shared
/// with the schema tree, which lists columns under a table.
pub fn column_type_icon(
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
    json_palette: JsonPalette,
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
    let text = inline_text(&text);
    let content = if is_pending {
        div().truncate().child(text).into_any_element()
    } else {
        rich_cell_content(&result, row, column, selected, text, json_palette)
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
        .on_mouse_down(MouseButton::Left, move |event, window, cx| {
            grid.update(cx, |grid, cx| {
                let position = CellPosition { row, column };
                if editable && event.click_count >= 2 {
                    grid.begin_edit(position, window, cx);
                } else {
                    grid.select(position, event.modifiers.shift, window, cx);
                }
            })
            .ok();
        })
        .on_mouse_move({
            let grid = menu_grid.clone();
            move |event, _, cx| {
                grid.update(cx, |grid, cx| {
                    if event.dragging() {
                        grid.extend_cell_selection(CellPosition { row, column }, cx)
                    } else {
                        grid.finish_cell_selection(cx);
                    }
                })
                .ok();
            }
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

fn inline_text(text: &str) -> String {
    if !text.contains(['\n', '\r']) {
        return text.to_owned();
    }
    text.replace("\r\n", "⏎")
        .replace('\n', "⏎")
        .replace('\r', "⏎")
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

#[cfg(test)]
mod tests {
    use cellar_core::value::CellValue;

    use super::{cell_text, column_type_icon, inline_text, pane_background, row_background};
    use crate::theme::{accent_soft, DELETE_SOFT, INSERT_SOFT, PANEL, PANEL_MUTED};

    #[test]
    fn grid_display_preferences_control_nulls_and_stripes() {
        assert_eq!(cell_text(&CellValue::Null, "∅"), "∅");
        assert_eq!(row_background(false, 1), PANEL.rgba());
        assert_eq!(row_background(true, 1), PANEL_MUTED.rgba());
    }

    #[test]
    fn frozen_pane_background_hides_the_columns_underneath_it() {
        // Row highlights are translucent; the pane has to flatten them so the
        // scrolled columns cannot show through a selected or edited row.
        assert_eq!(pane_background(accent_soft()).a, 1.);
        assert_eq!(pane_background(DELETE_SOFT.rgba()).a, 1.);
        assert_eq!(pane_background(INSERT_SOFT.rgba()).a, 1.);
        // Already-opaque stripe colors are unchanged.
        assert_eq!(pane_background(PANEL_MUTED.rgba()), PANEL_MUTED.rgba());
    }

    #[test]
    fn inline_text_keeps_cells_on_one_line() {
        assert_eq!(inline_text("plain"), "plain");
        assert_eq!(inline_text("a\nb\nc"), "a⏎b⏎c");
        assert_eq!(inline_text("a\r\nb\rc"), "a⏎b⏎c");
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
