//! Column widths: fitting a column to its content and keeping widths the user
//! chose. Widths live here so the grid's rendering code stays small.
use cellar_core::query::QueryResult;
use gpui::Context;

use super::{row::cell_edit_text, DataGrid};

/// Columns fit their content, but stay inside bounds that keep headers legible
/// and stop a single wide `text` value from swallowing the viewport.
pub(super) const MIN_COLUMN_WIDTH: f32 = 64.;
pub(super) const MAX_COLUMN_WIDTH: f32 = 600.;
/// Values considered when fitting a column: enough to settle on a width without
/// walking a whole page of rows on every reload.
const AUTOFIT_SAMPLE_ROWS: usize = 200;

/// Width that fits a header, its type, and the longest sampled value.
pub(super) fn auto_fit_width(
    name: &str,
    data_type: &str,
    value_lengths: impl Iterator<Item = usize>,
) -> f32 {
    let header = (name.chars().count() + data_type.chars().count()) as f32 * 7.8 + 58.;
    value_lengths
        .map(|length| length as f32 * 7.8 + 32.)
        .fold(header, f32::max)
        .clamp(MIN_COLUMN_WIDTH, MAX_COLUMN_WIDTH)
}

/// Starting widths for a fresh grid: each column fits its header, type, and the
/// longest of the first [`AUTOFIT_SAMPLE_ROWS`] loaded values.
pub(super) fn content_column_widths(result: &QueryResult) -> Vec<f32> {
    result
        .columns
        .iter()
        .enumerate()
        .map(|(index, meta)| {
            let lengths = result
                .rows
                .iter()
                .take(AUTOFIT_SAMPLE_ROWS)
                .filter_map(|row| row.get(index))
                .map(|value| cell_edit_text(value).chars().count());
            auto_fit_width(&meta.name, &meta.data_type, lengths)
        })
        .collect()
}

impl DataGrid {
    /// Fits one column to its content after a double-click on its edge, and
    /// remembers it as a width the user chose.
    pub(super) fn fit_column_to_content(&mut self, column: usize, cx: &mut Context<Self>) {
        let Some(meta) = self.result.columns.get(column) else {
            return;
        };
        let lengths = self.sampled_value_lengths(column);
        std::sync::Arc::make_mut(&mut self.column_widths)[column] =
            auto_fit_width(&meta.name, &meta.data_type, lengths);
        self.widths_user_set.insert(column);
        self.resizing = None;
        self.suppress_sort = true;
        cx.emit(super::DataGridEvent::LayoutChanged);
        cx.notify();
    }

    /// Longest of the sampled values for `column`, used to size it.
    pub(super) fn sampled_value_lengths(&self, column: usize) -> impl Iterator<Item = usize> + '_ {
        self.result
            .rows
            .iter()
            .take(AUTOFIT_SAMPLE_ROWS)
            .filter_map(move |row| row.get(column))
            .map(|value| cell_edit_text(value).chars().count())
    }

    /// Keeps untouched columns sized to their content as pages arrive. Columns
    /// the user resized keep the width they chose.
    pub(super) fn refit_automatic_columns(&mut self) {
        if self.result.columns.is_empty() || self.widths_user_set.len() >= self.column_widths.len()
        {
            return;
        }
        let mut widths = std::sync::Arc::clone(&self.column_widths);
        for (index, meta) in self.result.columns.iter().enumerate() {
            if self.widths_user_set.contains(&index) {
                continue;
            }
            let lengths = self.sampled_value_lengths(index);
            let width = auto_fit_width(&meta.name, &meta.data_type, lengths);
            if widths[index] != width {
                std::sync::Arc::make_mut(&mut widths)[index] = width;
            }
        }
        self.column_widths = widths;
    }
}

#[cfg(test)]
mod tests {
    use cellar_core::{
        query::{NoticeCapture, QueryResult},
        value::{CellValue, ColumnMeta},
    };

    use super::{auto_fit_width, content_column_widths};

    fn column(name: &str, data_type: &str) -> ColumnMeta {
        ColumnMeta {
            name: name.into(),
            data_type: data_type.into(),
            nullable: true,
        }
    }

    fn result_fixture(rows: Vec<Vec<CellValue>>) -> QueryResult {
        QueryResult {
            columns: vec![
                column("mssite", "nvarchar(64)"),
                column("Email", "nvarchar(128)"),
            ],
            rows,
            notices: Vec::new(),
            notice_capture: NoticeCapture::unsupported("test"),
            rows_affected: None,
            duration_ms: 0,
            truncated: false,
            total_rows: None,
        }
    }

    #[test]
    fn autofit_covers_headers_and_values_with_safe_bounds() {
        assert!(auto_fit_width("customer_id", "uuid", [3].into_iter()) > 150.);
        assert_eq!(auto_fit_width("x", "text", [1_000].into_iter()), 600.);
        assert!(auto_fit_width("x", "text", [].into_iter()) >= 64.);
    }

    #[test]
    fn content_widths_fit_the_longest_value_in_each_column() {
        let uuid = "2f0c0fc6-9a7c-4f3b-9d10-6d0a1f3e8b21";
        let result = result_fixture(vec![
            vec![
                CellValue::Text(uuid.into()),
                CellValue::Text("jack@epiczone.me".into()),
            ],
            vec![CellValue::Null, CellValue::Null],
        ]);
        let widths = content_column_widths(&result);
        assert_eq!(
            widths[0],
            auto_fit_width("mssite", "nvarchar(64)", [uuid.len()].into_iter())
        );
        assert!(widths[0] > widths[1]);
        assert!(widths[1] >= 64.);
    }

    #[test]
    fn content_widths_stop_at_the_cap_for_very_long_values() {
        let result = result_fixture(vec![vec![
            CellValue::Text("x".repeat(4_000)),
            CellValue::Text("a@b.c".into()),
        ]]);
        assert_eq!(content_column_widths(&result)[0], 600.);
    }
}
