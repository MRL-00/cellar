use chrono::{Datelike, Local, NaiveDate};
use gpui::{div, prelude::*, px, AnyElement, Entity, SharedString, WeakEntity};
use gpui_component::{input::Input, input::InputState, Icon};

use super::DataGrid;
use crate::theme::{
    ACCENT, ACCENT_FG, BG, BORDER, FG, FG_MUTED, FG_SECONDARY, PANEL, PANEL_RAISED,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum DateEditorKind {
    Date,
    DateTime,
    Time,
}

impl DateEditorKind {
    pub(super) fn has_time(self) -> bool {
        matches!(self, Self::DateTime | Self::Time)
    }
}

#[derive(Clone)]
pub(super) struct DateEditor {
    pub(super) kind: DateEditorKind,
    pub(super) month: NaiveDate,
    pub(super) selected: Option<NaiveDate>,
}

impl DateEditor {
    pub(super) fn new(kind: DateEditorKind, raw: &str) -> Self {
        let selected = parse_date(raw);
        let base = selected.unwrap_or_else(|| Local::now().date_naive());
        Self {
            kind,
            month: base.with_day(1).expect("valid date has a first day"),
            selected,
        }
    }

    pub(super) fn shift_month(&mut self, delta: i32) {
        let month = self.month.month0() as i32 + delta;
        let year = self.month.year() + month.div_euclid(12);
        let month = month.rem_euclid(12) as u32 + 1;
        self.month = NaiveDate::from_ymd_opt(year, month, 1).expect("normalized month is valid");
    }

    pub(super) fn value(&self, time: &str) -> Option<String> {
        match self.kind {
            DateEditorKind::Date => self
                .selected
                .map(|date| date.format("%Y-%m-%d").to_string()),
            DateEditorKind::DateTime => self
                .selected
                .map(|date| format!("{}T{}", date.format("%Y-%m-%d"), normalized_time(time))),
            DateEditorKind::Time => Some(normalized_time(time)),
        }
    }

    pub(super) fn picker_height(&self) -> f32 {
        match self.kind {
            DateEditorKind::Time => 112.,
            DateEditorKind::DateTime => 328.,
            DateEditorKind::Date => 284.,
        }
    }
}

pub(super) fn date_editor_kind(data_type: &str) -> Option<DateEditorKind> {
    let kind = data_type.trim().to_ascii_lowercase();
    if kind == "date" {
        Some(DateEditorKind::Date)
    } else if kind.starts_with("timestamp") || kind.contains("datetime") {
        Some(DateEditorKind::DateTime)
    } else if kind == "time" || kind.starts_with("time(") || kind.starts_with("time ") {
        Some(DateEditorKind::Time)
    } else {
        None
    }
}

pub(super) fn parse_time(raw: &str) -> String {
    raw.as_bytes()
        .windows(5)
        .position(|window| {
            window[0].is_ascii_digit()
                && window[1].is_ascii_digit()
                && window[2] == b':'
                && window[3].is_ascii_digit()
                && window[4].is_ascii_digit()
        })
        .map_or_else(
            || "00:00:00".into(),
            |start| {
                let end = (start + 8).min(raw.len());
                let value = &raw[start..end];
                if value.len() >= 8 && value.as_bytes().get(5) == Some(&b':') {
                    value[..8].to_owned()
                } else {
                    format!("{}:00", &raw[start..start + 5])
                }
            },
        )
}

pub(super) fn picker(
    editor: DateEditor,
    time: Option<Entity<InputState>>,
    left: f32,
    top: f32,
    grid: WeakEntity<DataGrid>,
) -> AnyElement {
    let picker_height = editor.picker_height();
    div()
        .absolute()
        .left(px(left))
        .top(px(top))
        .w(px(300.))
        .h(px(picker_height))
        .flex()
        .flex_col()
        .rounded(px(6.))
        .border_1()
        .border_color(BORDER)
        .bg(PANEL)
        .shadow_lg()
        .text_size(px(12.))
        .text_color(FG_SECONDARY)
        .when(editor.kind != DateEditorKind::Time, |panel| {
            panel
                .child(month_header(editor.month, grid.clone()))
                .child(weekday_header())
                .child(month_grid(&editor, grid.clone()))
        })
        .when_some(time, |panel, time| {
            panel.child(
                div()
                    .h(px(44.))
                    .flex_shrink_0()
                    .flex()
                    .items_center()
                    .gap_2()
                    .px_3()
                    .border_t_1()
                    .border_color(BORDER)
                    .child(div().w(px(34.)).text_color(FG_MUTED).child("Time"))
                    .child(
                        div()
                            .h(px(25.))
                            .w(px(116.))
                            .rounded(px(4.))
                            .border_1()
                            .border_color(BORDER)
                            .bg(BG)
                            .px_1()
                            .font_family(crate::theme::mono_font())
                            .child(Input::new(&time).h_full().appearance(false)),
                    ),
            )
        })
        .child(actions(grid))
        .into_any_element()
}

fn month_header(month: NaiveDate, grid: WeakEntity<DataGrid>) -> AnyElement {
    div()
        .h(px(38.))
        .flex_shrink_0()
        .flex()
        .items_center()
        .justify_between()
        .px_2()
        .child(month_button(
            "previous-month",
            "icons/chevron-left.svg",
            -1,
            grid.clone(),
        ))
        .child(
            div()
                .font_weight(gpui::FontWeight::MEDIUM)
                .text_color(FG)
                .child(month.format("%B %Y").to_string()),
        )
        .child(month_button(
            "next-month",
            "icons/chevron-right.svg",
            1,
            grid,
        ))
        .into_any_element()
}

fn month_button(
    id: &'static str,
    icon: &'static str,
    delta: i32,
    grid: WeakEntity<DataGrid>,
) -> AnyElement {
    div()
        .id(id)
        .tab_index(0)
        .cursor_pointer()
        .size(px(24.))
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(4.))
        .text_color(FG_MUTED)
        .hover(|style| style.bg(PANEL_RAISED).text_color(FG))
        .child(Icon::empty().path(icon).size(px(11.)))
        .on_click(move |_, _, cx| {
            grid.update(cx, |grid, cx| grid.shift_editor_month(delta, cx))
                .ok();
        })
        .into_any_element()
}

fn weekday_header() -> AnyElement {
    div()
        .h(px(22.))
        .flex_shrink_0()
        .flex()
        .px(px(8.))
        .children(["Mo", "Tu", "We", "Th", "Fr", "Sa", "Su"].map(|day| {
            div()
                .w(px(40.))
                .flex()
                .items_center()
                .justify_center()
                .text_size(px(10.))
                .text_color(FG_MUTED)
                .child(day)
        }))
        .into_any_element()
}

fn month_grid(editor: &DateEditor, grid: WeakEntity<DataGrid>) -> AnyElement {
    let offset = editor.month.weekday().num_days_from_monday() as i64;
    let first = editor.month - chrono::Duration::days(offset);
    div()
        .h(px(180.))
        .flex_shrink_0()
        .flex()
        .flex_wrap()
        .px(px(8.))
        .children((0..42).map(|index| {
            let date = first + chrono::Duration::days(index);
            let selected = editor.selected == Some(date);
            let current_month = date.month() == editor.month.month();
            let day_grid = grid.clone();
            div()
                .id(SharedString::from(format!("calendar-day:{date}")))
                .tab_index(0)
                .cursor_pointer()
                .w(px(40.))
                .h(px(30.))
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(4.))
                .bg(if selected { ACCENT } else { PANEL })
                .text_color(if selected {
                    ACCENT_FG
                } else if current_month {
                    FG_SECONDARY
                } else {
                    FG_MUTED
                })
                .when(!selected, |day| {
                    day.hover(|style| style.bg(PANEL_RAISED).text_color(FG))
                })
                .child(date.day().to_string())
                .on_click(move |_, _, cx| {
                    day_grid
                        .update(cx, |grid, cx| grid.select_editor_date(date, cx))
                        .ok();
                })
        }))
        .into_any_element()
}

fn actions(grid: WeakEntity<DataGrid>) -> AnyElement {
    let cancel = grid.clone();
    div()
        .h(px(44.))
        .flex_shrink_0()
        .flex()
        .items_center()
        .justify_end()
        .gap_2()
        .px_3()
        .border_t_1()
        .border_color(BORDER)
        .child(
            action_button("date-cancel", "Cancel", false).on_click(move |_, _, cx| {
                cancel.update(cx, DataGrid::cancel_editor).ok();
            }),
        )
        .child(
            action_button("date-apply", "Apply", true).on_click(move |_, _, cx| {
                grid.update(cx, DataGrid::apply_date_editor).ok();
            }),
        )
        .into_any_element()
}

fn action_button(
    id: &'static str,
    label: &'static str,
    primary: bool,
) -> gpui::Stateful<gpui::Div> {
    div()
        .id(id)
        .tab_index(0)
        .cursor_pointer()
        .h(px(24.))
        .flex()
        .items_center()
        .px_3()
        .rounded(px(4.))
        .border_1()
        .border_color(if primary { ACCENT } else { BORDER })
        .bg(if primary { ACCENT } else { PANEL_RAISED })
        .text_color(if primary { ACCENT_FG } else { FG_SECONDARY })
        .hover(|style| {
            if primary {
                style.text_color(ACCENT_FG)
            } else {
                style.text_color(FG)
            }
        })
        .child(label)
}

fn parse_date(raw: &str) -> Option<NaiveDate> {
    (raw.len() >= 10)
        .then(|| &raw[..10])
        .and_then(|value| NaiveDate::parse_from_str(value, "%Y-%m-%d").ok())
}

fn normalized_time(raw: &str) -> String {
    let parsed = parse_time(raw);
    let mut parts = parsed
        .split(':')
        .filter_map(|part| part.parse::<u32>().ok());
    let hour = parts.next().unwrap_or(0).min(23);
    let minute = parts.next().unwrap_or(0).min(59);
    let second = parts.next().unwrap_or(0).min(59);
    format!("{hour:02}:{minute:02}:{second:02}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn date_editor_recognizes_and_formats_classic_temporal_types() {
        assert_eq!(date_editor_kind("date"), Some(DateEditorKind::Date));
        assert_eq!(
            date_editor_kind("timestamp with time zone"),
            Some(DateEditorKind::DateTime)
        );
        assert_eq!(date_editor_kind("time(6)"), Some(DateEditorKind::Time));
        assert_eq!(parse_time("2026-08-15T05:04"), "05:04:00");
        let editor = DateEditor::new(DateEditorKind::DateTime, "2026-08-15 05:04:03");
        assert_eq!(
            editor.value("25:99:99").as_deref(),
            Some("2026-08-15T23:59:59")
        );
    }
}
