mod context_menu;
mod controls;
mod date_picker;
mod editing;
mod export;
mod keyboard;
mod layout;
mod rich;
mod row;
mod view;

pub use layout::{GridLayout, PortableGridLayout};

use std::{ops::Range, sync::Arc};

use cellar_core::{
    query::{NoticeCapture, QueryResult, QueryResultPage, QueryResultSummary, SortDirection},
    schema::Table,
};
use cellar_diff::TableChangeRequest;
use chrono::Datelike as _;
use gpui::{
    point, prelude::*, px, ClipboardItem, Context, Entity, EventEmitter, FocusHandle, Focusable,
    ScrollHandle, ScrollStrategy, UniformListScrollHandle, Window,
};
use gpui_component::input::{InputEvent, InputState};

use date_picker::{date_editor_kind, DateEditor};
use editing::EditableGrid;
use row::{cell_edit_text, clipboard_text};

use crate::model::TableTarget;

const ROW_NUMBER_WIDTH: f32 = 36.;
const CELL_WIDTH: f32 = 160.;
const COLUMN_OVERSCAN: usize = 2;
const FROZEN_COLUMNS: usize = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct CellPosition {
    row: usize,
    column: usize,
}

struct ActiveEditor {
    position: CellPosition,
    state: Entity<InputState>,
    date: Option<DateEditor>,
    time: Option<Entity<InputState>>,
}

#[derive(Clone)]
struct DragColumn(usize);

#[derive(Clone)]
pub enum DataGridEvent {
    ImportCsv,
    ReviewChanges {
        connection_id: String,
        request: TableChangeRequest,
    },
    SortColumn {
        column: String,
        direction: Option<SortDirection>,
    },
    FindUsages {
        target: TableTarget,
        column: Option<String>,
    },
}

pub struct DataGrid {
    result: Arc<QueryResult>,
    visible_rows: Range<usize>,
    vertical_scroll: UniformListScrollHandle,
    horizontal_scroll: ScrollHandle,
    focus_handle: FocusHandle,
    selection: Option<CellPosition>,
    editable: Option<EditableGrid>,
    active_editor: Option<ActiveEditor>,
    sort: Option<(usize, SortDirection)>,
    column_widths: Arc<Vec<f32>>,
    resizing: Option<(usize, f32, f32)>,
    suppress_sort: bool,
    edit_error: Option<String>,
    export_message: Option<Result<String, String>>,
    null_display: Arc<str>,
    stripe_rows: bool,
}

impl DataGrid {
    pub fn new(result: QueryResult, cx: &mut Context<Self>) -> Self {
        let column_widths = Arc::new(vec![CELL_WIDTH; result.columns.len()]);
        Self {
            result: Arc::new(result),
            visible_rows: 0..0,
            vertical_scroll: UniformListScrollHandle::new(),
            horizontal_scroll: ScrollHandle::new(),
            focus_handle: cx.focus_handle(),
            selection: None,
            editable: None,
            active_editor: None,
            sort: None,
            column_widths,
            resizing: None,
            suppress_sort: false,
            edit_error: None,
            export_message: None,
            null_display: Arc::from("NULL"),
            stripe_rows: false,
        }
    }

    pub fn set_display_preferences(
        &mut self,
        null_display: impl Into<Arc<str>>,
        stripe_rows: bool,
        cx: &mut Context<Self>,
    ) {
        self.null_display = null_display.into();
        self.stripe_rows = stripe_rows;
        cx.notify();
    }

    pub fn new_table(
        result: QueryResult,
        target: TableTarget,
        table: Table,
        sort: Option<(usize, SortDirection)>,
        cx: &mut Context<Self>,
    ) -> Self {
        let editable = EditableGrid::new(target, table, &result);
        Self {
            editable: Some(editable),
            sort,
            ..Self::new(result, cx)
        }
    }

    pub fn from_page(page: QueryResultPage, cx: &mut Context<Self>) -> Self {
        Self::new(
            QueryResult {
                columns: page.columns,
                rows: page.rows,
                notices: Vec::new(),
                notice_capture: NoticeCapture::unsupported("query is still running"),
                rows_affected: None,
                duration_ms: 0,
                truncated: false,
                total_rows: None,
            },
            cx,
        )
    }

    pub fn append_page(
        &mut self,
        page: QueryResultPage,
        cx: &mut Context<Self>,
    ) -> Result<(), String> {
        let result = Arc::make_mut(&mut self.result);
        if page.offset != result.rows.len() as u64 {
            return Err(format!(
                "query page arrived out of order: expected {}, got {}",
                result.rows.len(),
                page.offset
            ));
        }
        if result.columns.is_empty() {
            result.columns = page.columns;
        } else if result.columns != page.columns {
            return Err("query columns changed between result pages".into());
        }
        result.rows.extend(page.rows);
        cx.notify();
        Ok(())
    }

    pub fn complete(&mut self, summary: &QueryResultSummary, cx: &mut Context<Self>) {
        let result = Arc::make_mut(&mut self.result);
        result.notices = summary.notices.clone();
        result.notice_capture = summary.notice_capture.clone();
        result.rows_affected = summary.rows_affected;
        result.duration_ms = summary.duration_ms;
        result.truncated = summary.truncated;
        result.total_rows = summary.total_rows;
        cx.notify();
    }

    pub fn clear_pending(&mut self, cx: &mut Context<Self>) {
        if let Some(editable) = &mut self.editable {
            let inserted = editable.clear();
            if !inserted.is_empty() {
                self.selection = None;
            }
            for row in inserted.into_iter().rev() {
                Arc::make_mut(&mut self.result).rows.remove(row);
            }
        }
        self.active_editor = None;
        self.edit_error = None;
        cx.notify();
    }

    pub fn prepare_for_reload(&mut self, cx: &mut Context<Self>) -> bool {
        self.commit_editor(cx);
        let pending = self
            .editable
            .as_ref()
            .is_some_and(|editable| editable.pending_count() > 0);
        if pending && self.edit_error.is_none() {
            self.edit_error = Some("Commit or revert pending edits before reloading data".into());
            cx.notify();
        }
        !pending && self.edit_error.is_none()
    }

    pub fn scroll_to_cell(&mut self, row: usize, column: usize, cx: &mut Context<Self>) {
        self.selection = Some(CellPosition { row, column });
        self.vertical_scroll
            .scroll_to_item(row, ScrollStrategy::Center);
        self.reveal_column(column);
        cx.notify();
    }

    fn visible_columns(&self) -> Range<usize> {
        visible_column_range(
            &self.column_widths,
            (-f32::from(self.horizontal_scroll.offset().x)).max(0.),
            f32::from(self.horizontal_scroll.bounds().size.width).max(800.),
        )
    }

    fn select(&mut self, position: CellPosition, window: &mut Window, cx: &mut Context<Self>) {
        self.selection = Some(position);
        window.focus(&self.focus_handle);
        cx.notify();
    }

    fn begin_edit(&mut self, position: CellPosition, window: &mut Window, cx: &mut Context<Self>) {
        if !self.editable.as_ref().is_some_and(EditableGrid::can_edit) {
            return;
        }
        self.commit_editor(cx);
        let current = self
            .editable
            .as_ref()
            .and_then(|editable| editable.display_value(position.row, position.column))
            .flatten()
            .or_else(|| {
                self.result
                    .rows
                    .get(position.row)
                    .and_then(|row| row.get(position.column))
                    .map(cell_edit_text)
            })
            .unwrap_or_default();
        let date = self
            .result
            .columns
            .get(position.column)
            .and_then(|column| date_editor_kind(&column.data_type))
            .map(|kind| DateEditor::new(kind, &current));
        let time = date.as_ref().and_then(|date| {
            date.kind.has_time().then(|| {
                cx.new(|cx| {
                    InputState::new(window, cx).default_value(date_picker::parse_time(&current))
                })
            })
        });
        let state = cx.new(|cx| InputState::new(window, cx).default_value(current));
        let commit_on_blur = date.is_none();
        cx.subscribe_in(&state, window, move |this, _, event: &InputEvent, _, cx| {
            if matches!(event, InputEvent::PressEnter { .. })
                || commit_on_blur && matches!(event, InputEvent::Blur)
            {
                this.commit_editor(cx);
            }
        })
        .detach();
        window.focus(&state.focus_handle(cx));
        self.selection = Some(position);
        self.active_editor = Some(ActiveEditor {
            position,
            state,
            date,
            time,
        });
        cx.notify();
    }

    fn select_editor_date(&mut self, date: chrono::NaiveDate, cx: &mut Context<Self>) {
        if let Some(editor) = self
            .active_editor
            .as_mut()
            .and_then(|editor| editor.date.as_mut())
        {
            editor.selected = Some(date);
            editor.month = date.with_day(1).expect("valid date has a first day");
            cx.notify();
        }
    }

    fn shift_editor_month(&mut self, delta: i32, cx: &mut Context<Self>) {
        if let Some(editor) = self
            .active_editor
            .as_mut()
            .and_then(|editor| editor.date.as_mut())
        {
            editor.shift_month(delta);
            cx.notify();
        }
    }

    fn apply_date_editor(&mut self, cx: &mut Context<Self>) {
        let Some(editor) = self.active_editor.take() else {
            return;
        };
        let Some(date) = editor.date else {
            self.active_editor = Some(editor);
            self.commit_editor(cx);
            return;
        };
        let time = editor.time.as_ref().map_or_else(
            || "00:00:00".to_owned(),
            |state| state.read(cx).value().to_string(),
        );
        let value = date.value(&time);
        if let Some(editable) = &mut self.editable {
            self.edit_error = editable
                .set_value(
                    editor.position.row,
                    editor.position.column,
                    value,
                    &self.result,
                )
                .err();
        }
        cx.notify();
    }

    fn cancel_editor(&mut self, cx: &mut Context<Self>) {
        self.active_editor = None;
        cx.notify();
    }

    fn commit_editor(&mut self, cx: &mut Context<Self>) {
        let Some(editor) = self.active_editor.take() else {
            return;
        };
        let value = editor.state.read(cx).value().to_string();
        if let Some(editable) = &mut self.editable {
            self.edit_error = editable
                .set_value(
                    editor.position.row,
                    editor.position.column,
                    Some(value),
                    &self.result,
                )
                .err();
        }
        cx.notify();
    }

    fn set_selected_null(&mut self, cx: &mut Context<Self>) {
        self.commit_editor(cx);
        let Some(position) = self.selection else {
            return;
        };
        if let Some(editable) = &mut self.editable {
            self.edit_error = editable
                .set_value(position.row, position.column, None, &self.result)
                .err();
            cx.notify();
        }
    }

    fn toggle_selected_bool(&mut self, cx: &mut Context<Self>) {
        self.commit_editor(cx);
        let Some(position) = self.selection else {
            return;
        };
        if !self.result.columns[position.column]
            .data_type
            .eq_ignore_ascii_case("bool")
            && !self.result.columns[position.column]
                .data_type
                .eq_ignore_ascii_case("boolean")
        {
            self.edit_error = Some("Select a boolean column first".into());
            cx.notify();
            return;
        }
        let current = self
            .editable
            .as_ref()
            .and_then(|editable| editable.display_value(position.row, position.column))
            .flatten()
            .and_then(|value| value.parse::<bool>().ok())
            .or_else(|| match &self.result.rows[position.row][position.column] {
                cellar_core::value::CellValue::Bool(value) => Some(*value),
                _ => None,
            })
            .unwrap_or(false);
        if let Some(editable) = &mut self.editable {
            self.edit_error = editable
                .set_value(
                    position.row,
                    position.column,
                    Some((!current).to_string()),
                    &self.result,
                )
                .err();
        }
        cx.notify();
    }

    fn add_row(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(editable) = &mut self.editable else {
            return;
        };
        let row = self.result.rows.len();
        let columns = self.result.columns.len();
        editable.insert_row(row);
        Arc::make_mut(&mut self.result)
            .rows
            .push(vec![cellar_core::value::CellValue::Null; columns]);
        self.vertical_scroll
            .scroll_to_item(row, ScrollStrategy::Center);
        self.begin_edit(CellPosition { row, column: 0 }, window, cx);
    }

    fn delete_selected_row(&mut self, cx: &mut Context<Self>) {
        self.commit_editor(cx);
        if self.edit_error.is_some() {
            return;
        }
        let Some(position) = self.selection else {
            return;
        };
        self.toggle_row_delete(position.row, cx);
    }

    fn toggle_row_delete(&mut self, row: usize, cx: &mut Context<Self>) {
        if let Some(editable) = &mut self.editable {
            if editable.toggle_delete(row) {
                Arc::make_mut(&mut self.result).rows.remove(row);
                self.selection = None;
            }
            cx.notify();
        }
    }

    fn review_changes(&mut self, cx: &mut Context<Self>) {
        self.commit_editor(cx);
        let Some(editable) = &self.editable else {
            return;
        };
        if editable.pending_count() == 0 {
            return;
        }
        cx.emit(DataGridEvent::ReviewChanges {
            connection_id: editable.connection_id().to_owned(),
            request: editable.request(&self.result),
        });
    }

    pub fn request_review(&mut self, cx: &mut Context<Self>) {
        self.review_changes(cx);
    }

    pub fn pending_count(&self) -> usize {
        self.editable
            .as_ref()
            .map_or(0, EditableGrid::pending_count)
    }

    fn request_csv_import(&mut self, cx: &mut Context<Self>) {
        if self.editable.is_some() {
            cx.emit(DataGridEvent::ImportCsv);
        }
    }

    fn toggle_sort(&mut self, column: usize, cx: &mut Context<Self>) {
        if std::mem::take(&mut self.suppress_sort) {
            return;
        }
        if self.editable.is_none() {
            return;
        }
        if !self.prepare_for_reload(cx) {
            return;
        }
        let direction = next_sort_direction(self.sort, column);
        self.sort = direction.map(|direction| (column, direction));
        cx.emit(DataGridEvent::SortColumn {
            column: self.result.columns[column].name.clone(),
            direction,
        });
        cx.notify();
    }

    fn move_column(&mut self, source: usize, target: usize, cx: &mut Context<Self>) {
        if source == target
            || source >= self.result.columns.len()
            || target >= self.result.columns.len()
        {
            return;
        }
        self.commit_editor(cx);
        let result = Arc::make_mut(&mut self.result);
        let column = result.columns.remove(source);
        result.columns.insert(target, column);
        for row in &mut result.rows {
            let value = row.remove(source);
            row.insert(target, value);
        }
        let widths = Arc::make_mut(&mut self.column_widths);
        let width = widths.remove(source);
        widths.insert(target, width);
        if let Some(editable) = &mut self.editable {
            editable.move_column(source, target);
        }
        self.selection = self.selection.map(|position| CellPosition {
            row: position.row,
            column: moved_index(position.column, source, target),
        });
        self.sort = self
            .sort
            .map(|(column, direction)| (moved_index(column, source, target), direction));
        self.suppress_sort = true;
        cx.notify();
    }

    fn move_selection(&mut self, row_delta: isize, column_delta: isize, cx: &mut Context<Self>) {
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
        self.vertical_scroll
            .scroll_to_item(row, ScrollStrategy::Center);
        self.reveal_column(column);
        cx.notify();
    }

    fn copy_selection(&self, cx: &mut Context<Self>) {
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

    fn paste_selection(&mut self, cx: &mut Context<Self>) {
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
        for (row_offset, values) in clipboard_rows(&text).into_iter().enumerate() {
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

    fn reveal_column(&self, column: usize) {
        if column < FROZEN_COLUMNS {
            return;
        }
        let viewport = f32::from(self.horizontal_scroll.bounds().size.width).max(800.);
        let current = (-f32::from(self.horizontal_scroll.offset().x)).max(0.);
        let frozen_width = ROW_NUMBER_WIDTH + width_sum(&self.column_widths, 0..FROZEN_COLUMNS);
        let left = ROW_NUMBER_WIDTH + width_sum(&self.column_widths, 0..column);
        let right = left + self.column_widths[column];
        let next = if left < current + frozen_width {
            (left - frozen_width).max(0.)
        } else if right > current + viewport {
            right - viewport
        } else {
            current
        };
        let offset = self.horizontal_scroll.offset();
        self.horizontal_scroll
            .set_offset(point(px(-next), offset.y));
    }

    fn begin_resize(&mut self, column: usize, cursor_x: f32, cx: &mut Context<Self>) {
        self.resizing = Some((column, cursor_x, self.column_widths[column]));
        cx.notify();
    }

    fn auto_fit_column(&mut self, column: usize, cx: &mut Context<Self>) {
        let Some(meta) = self.result.columns.get(column) else {
            return;
        };
        // ponytail: character-width estimate; use GPUI text shaping if mixed-width grid fonts need exact sizing.
        let lengths = self
            .result
            .rows
            .iter()
            .take(200)
            .filter_map(|row| row.get(column))
            .map(|value| cell_edit_text(value).chars().count());
        Arc::make_mut(&mut self.column_widths)[column] =
            auto_fit_width(&meta.name, &meta.data_type, lengths);
        self.resizing = None;
        self.suppress_sort = true;
        cx.notify();
    }

    fn resize_column(
        &mut self,
        event: &gpui::MouseMoveEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some((column, start_x, start_width)) = self.resizing else {
            return;
        };
        let width = (start_width + f32::from(event.position.x) - start_x).clamp(64., 600.);
        Arc::make_mut(&mut self.column_widths)[column] = width;
        cx.notify();
    }

    fn finish_resize(&mut self, cx: &mut Context<Self>) {
        if self.resizing.take().is_some() {
            cx.notify();
        }
    }
}

impl EventEmitter<DataGridEvent> for DataGrid {}

fn width_sum(widths: &[f32], range: Range<usize>) -> f32 {
    widths
        .iter()
        .skip(range.start)
        .take(range.end.saturating_sub(range.start))
        .sum()
}

fn auto_fit_width(name: &str, data_type: &str, value_lengths: impl Iterator<Item = usize>) -> f32 {
    let header = (name.chars().count() + data_type.chars().count()) as f32 * 7.8 + 58.;
    value_lengths
        .map(|length| length as f32 * 7.8 + 32.)
        .fold(header, f32::max)
        .clamp(64., 600.)
}

fn visible_column_range(widths: &[f32], offset: f32, viewport: f32) -> Range<usize> {
    let total = widths.len();
    let frozen = FROZEN_COLUMNS.min(total);
    let frozen_width = ROW_NUMBER_WIDTH + width_sum(widths, 0..frozen);
    let mut first = frozen;
    let mut position = frozen_width;
    while first < total && position + widths[first] < offset + frozen_width {
        position += widths[first];
        first += 1;
    }
    first = first.saturating_sub(COLUMN_OVERSCAN).max(frozen);
    position = ROW_NUMBER_WIDTH + width_sum(widths, 0..first);
    let mut last = first;
    while last < total && position < offset + viewport {
        position += widths[last];
        last += 1;
    }
    last = (last + COLUMN_OVERSCAN).min(total);
    first..last
}

fn next_sort_direction(
    sort: Option<(usize, SortDirection)>,
    column: usize,
) -> Option<SortDirection> {
    match sort {
        Some((sorted, SortDirection::Asc)) if sorted == column => Some(SortDirection::Desc),
        Some((sorted, SortDirection::Desc)) if sorted == column => None,
        _ => Some(SortDirection::Asc),
    }
}

fn moved_index(index: usize, source: usize, target: usize) -> usize {
    if index == source {
        target
    } else if source < target && (source + 1..=target).contains(&index) {
        index - 1
    } else if target < source && (target..source).contains(&index) {
        index + 1
    } else {
        index
    }
}

fn clipboard_rows(text: &str) -> Vec<Vec<String>> {
    text.trim_end_matches(['\r', '\n'])
        .split('\n')
        .map(|row| {
            row.trim_end_matches('\r')
                .split('\t')
                .map(str::to_owned)
                .collect()
        })
        .collect()
}

pub(super) fn wheel_scrolls_horizontally(delta: gpui::Point<gpui::Pixels>) -> bool {
    f32::from(delta.x).abs() > f32::from(delta.y).abs()
}

#[cfg(test)]
mod tests {
    use cellar_core::query::SortDirection;

    use super::{
        auto_fit_width, clipboard_rows, moved_index, next_sort_direction, visible_column_range,
        wheel_scrolls_horizontally,
    };
    use gpui::{point, px};

    #[test]
    fn column_autofit_covers_headers_and_values_with_safe_bounds() {
        assert!(auto_fit_width("customer_id", "uuid", [3].into_iter()) > 150.);
        assert_eq!(auto_fit_width("x", "text", [1_000].into_iter()), 600.);
    }

    #[test]
    fn horizontal_virtualization_stays_bounded() {
        let widths = vec![160.; 500];
        assert_eq!(visible_column_range(&widths, 0., 800.), 1..7);
        let scrolled = visible_column_range(&widths, 30_000., 800.);
        assert!(scrolled.start > 180);
        assert!(scrolled.len() <= 10);
        assert_eq!(visible_column_range(&[], 0., 800.), 0..0);
    }

    #[test]
    fn table_sort_cycles_ascending_descending_off() {
        assert_eq!(next_sort_direction(None, 2), Some(SortDirection::Asc));
        assert_eq!(
            next_sort_direction(Some((2, SortDirection::Asc)), 2),
            Some(SortDirection::Desc)
        );
        assert_eq!(next_sort_direction(Some((2, SortDirection::Desc)), 2), None);
    }

    #[test]
    fn clipboard_tsv_preserves_empty_cells() {
        assert_eq!(
            clipboard_rows("a\tb\n\tc\n"),
            vec![
                vec!["a".to_string(), "b".to_string()],
                vec!["".to_string(), "c".to_string()],
            ]
        );
    }

    #[test]
    fn moving_a_column_remaps_every_affected_index() {
        assert_eq!(
            (0..5).map(|i| moved_index(i, 1, 3)).collect::<Vec<_>>(),
            vec![0, 3, 1, 2, 4]
        );
        assert_eq!(
            (0..5).map(|i| moved_index(i, 3, 1)).collect::<Vec<_>>(),
            vec![0, 2, 3, 1, 4]
        );
    }

    #[test]
    fn vertical_wheel_does_not_count_as_horizontal() {
        assert!(!wheel_scrolls_horizontally(point(px(0.), px(40.))));
        assert!(!wheel_scrolls_horizontally(point(px(4.), px(40.))));
        assert!(wheel_scrolls_horizontally(point(px(40.), px(4.))));
        assert!(!wheel_scrolls_horizontally(point(px(10.), px(10.))));
    }
}
