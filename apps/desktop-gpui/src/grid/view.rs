use std::{collections::BTreeSet, ops::Range, sync::Arc};

use gpui::{
    div, prelude::*, px, uniform_list, Context, IntoElement, MouseButton, Render, ScrollWheelEvent,
    WeakEntity, Window,
};
use gpui_component::Icon;

use super::{
    controls, date_picker, row::header_cell, row::GridRow, width_sum, DataGrid, EditableGrid,
    FROZEN_COLUMNS, ROW_NUMBER_WIDTH,
};
use crate::theme::{ui_px, ui_scale, ACCENT, BORDER, FG_MUTED, PANEL, PANEL_RAISED};

impl DataGrid {
    fn header(
        &self,
        columns: Range<usize>,
        horizontal_offset: f32,
        grid: WeakEntity<DataGrid>,
    ) -> impl IntoElement {
        let total_columns = self.result.columns.len();
        let frozen = FROZEN_COLUMNS.min(total_columns);
        let total_width = ROW_NUMBER_WIDTH + width_sum(&self.column_widths, 0..total_columns);
        div()
            .flex()
            .h(ui_px(26.))
            .w(px(total_width))
            .bg(PANEL)
            .border_b_1()
            .border_color(BORDER)
            .child(
                div()
                    .relative()
                    .left(px(horizontal_offset))
                    .flex()
                    .flex_shrink_0()
                    .bg(PANEL)
                    .child(
                        div()
                            .w(px(ROW_NUMBER_WIDTH))
                            .flex_shrink_0()
                            .flex()
                            .items_center()
                            .justify_center()
                            .text_color(FG_MUTED)
                            .child(Icon::empty().path("icons/type-hash.svg").size(ui_px(9.))),
                    )
                    .children(self.result.columns.iter().take(frozen).enumerate().map(
                        |(index, column)| {
                            let (primary_key, foreign_key) = self
                                .editable
                                .as_ref()
                                .map(|editable| editable.column_flags(&column.name))
                                .unwrap_or_default();
                            header_cell(
                                column,
                                index,
                                self.column_widths[index],
                                self.sort,
                                primary_key,
                                foreign_key,
                                grid.clone(),
                            )
                        },
                    )),
            )
            .child(
                div()
                    .w(px(width_sum(&self.column_widths, frozen..columns.start)))
                    .flex_shrink_0(),
            )
            .children(
                self.result
                    .columns
                    .iter()
                    .skip(columns.start)
                    .take(columns.end.saturating_sub(columns.start))
                    .enumerate()
                    .map(|(offset, column)| {
                        let index = columns.start + offset;
                        let (primary_key, foreign_key) = self
                            .editable
                            .as_ref()
                            .map(|editable| editable.column_flags(&column.name))
                            .unwrap_or_default();
                        header_cell(
                            column,
                            index,
                            self.column_widths[index],
                            self.sort,
                            primary_key,
                            foreign_key,
                            grid.clone(),
                        )
                    }),
            )
            .child(
                div()
                    .w(px(width_sum(
                        &self.column_widths,
                        columns.end..total_columns,
                    )))
                    .flex_shrink_0(),
            )
    }
}

impl Render for DataGrid {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let columns = self.visible_columns();
        let horizontal_offset = (-f32::from(self.horizontal_scroll.offset().x)).max(0.);
        let result = Arc::clone(&self.result);
        let pending = Arc::new(
            self.editable
                .as_ref()
                .map(EditableGrid::display_values)
                .unwrap_or_default(),
        );
        let pending_count = self
            .editable
            .as_ref()
            .map(EditableGrid::pending_count)
            .unwrap_or(0);
        let deleted = Arc::new(
            self.editable
                .as_ref()
                .map(EditableGrid::deleted_rows)
                .unwrap_or_default()
                .into_iter()
                .collect::<BTreeSet<_>>(),
        );
        let inserted = Arc::new(
            self.editable
                .as_ref()
                .map(EditableGrid::inserted_rows)
                .unwrap_or_default()
                .into_iter()
                .collect::<BTreeSet<_>>(),
        );
        let editable = self.editable.as_ref().is_some_and(EditableGrid::can_edit);
        let selection = self.selection;
        let grid = cx.weak_entity();
        let row_grid = grid.clone();
        let row_deleted = Arc::clone(&deleted);
        let row_inserted = Arc::clone(&inserted);
        let null_display = Arc::clone(&self.null_display);
        let stripe_rows = self.stripe_rows;
        let resize_grid = grid.clone();
        let column_widths = Arc::clone(&self.column_widths);
        let total_width = ROW_NUMBER_WIDTH + width_sum(&column_widths, 0..result.columns.len());
        let editor = self.active_editor.as_ref().map(|editor| {
            let column_left =
                ROW_NUMBER_WIDTH + width_sum(&column_widths, 0..editor.position.column);
            let left = if editor.position.column < FROZEN_COLUMNS {
                column_left
            } else {
                column_left - horizontal_offset
            };
            let vertical_offset = f32::from(self.vertical_scroll.0.borrow().base_handle.offset().y);
            let top = 26. * ui_scale()
                + editor.position.row as f32 * crate::theme::row_height()
                + vertical_offset;
            (
                editor.state.clone(),
                left,
                top,
                column_widths[editor.position.column],
                editor.date.clone(),
                editor.time.clone(),
            )
        });

        div()
            .id("native-data-grid")
            .size_full()
            .min_h_0()
            .relative()
            .overflow_hidden()
            .flex()
            .flex_col()
            .font_family(crate::theme::mono_font())
            .text_size(ui_px(13.))
            .bg(PANEL)
            .track_focus(&self.focus_handle)
            .on_key_down(cx.listener(Self::key_down))
            .on_mouse_move(cx.listener(Self::resize_column))
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|this, _, _, cx| this.finish_resize(cx)),
            )
            .on_mouse_up_out(MouseButton::Left, move |_, _, cx| {
                resize_grid
                    .update(cx, |grid, cx| grid.finish_resize(cx))
                    .ok();
            })
            .child(
                div()
                    .id("native-grid-scroller")
                    .flex_1()
                    .min_h_0()
                    .overflow_x_scroll()
                    .track_scroll(&self.horizontal_scroll)
                    .on_scroll_wheel(cx.listener(|_, event: &ScrollWheelEvent, window, cx| {
                        let delta = event.delta.pixel_delta(window.line_height());
                        if !super::wheel_scrolls_horizontally(delta) {
                            cx.stop_propagation();
                        }
                        cx.notify();
                    }))
                    .child(self.header(columns.clone(), horizontal_offset, grid.clone()))
                    .child(
                        uniform_list(
                            "native-grid-rows",
                            result.rows.len(),
                            cx.processor(move |this, range: Range<usize>, _, _| {
                                this.visible_rows = range.clone();
                                range
                                    .map(|row| GridRow {
                                        result: Arc::clone(&result),
                                        row,
                                        columns: columns.clone(),
                                        horizontal_offset,
                                        selection,
                                        pending: Arc::clone(&pending),
                                        deleted: row_deleted.contains(&row),
                                        inserted: row_inserted.contains(&row),
                                        editable,
                                        null_display: Arc::clone(&null_display),
                                        stripe_rows,
                                        grid: row_grid.clone(),
                                        column_widths: Arc::clone(&column_widths),
                                    })
                                    .collect::<Vec<_>>()
                            }),
                        )
                        .h_full()
                        .w(px(total_width))
                        .track_scroll(self.vertical_scroll.clone()),
                    ),
            )
            .child(controls::export_bar(
                grid.clone(),
                editable,
                self.export_message.as_ref(),
            ))
            .when(editable, |element| {
                element.child(controls::pending_bar(
                    grid.clone(),
                    pending_count,
                    self.edit_error.as_deref(),
                ))
            })
            .when_some(editor, |element, (state, left, top, width, date, time)| {
                let viewport_width =
                    f32::from(self.horizontal_scroll.bounds().size.width).max(300.);
                let viewport_height = f32::from(
                    self.vertical_scroll
                        .0
                        .borrow()
                        .base_handle
                        .bounds()
                        .size
                        .height,
                )
                .max(330.);
                element
                    .child(
                        div()
                            .absolute()
                            .left(px(left))
                            .top(px(top))
                            .w(px(width))
                            .h(px(crate::theme::row_height()))
                            .bg(PANEL_RAISED)
                            .border_1()
                            .border_color(ACCENT)
                            .child(crate::widgets::compact_input(&state)),
                    )
                    .when_some(date, |element, date| {
                        let picker_height = date.picker_height();
                        let picker_top = if top + crate::theme::row_height() + picker_height
                            <= viewport_height
                        {
                            top + crate::theme::row_height()
                        } else {
                            (top - picker_height).max(26. * ui_scale())
                        };
                        element.child(date_picker::picker(
                            date,
                            time,
                            left.min((viewport_width - 300.).max(0.)),
                            picker_top,
                            grid.clone(),
                        ))
                    })
            })
    }
}
