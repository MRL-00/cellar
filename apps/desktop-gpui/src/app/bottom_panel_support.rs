use cellar_core::query::{DatabaseNotice, NoticeSeverity, QueryResultSummary};
use chrono::{DateTime, Local};
use gpui::{div, prelude::*, px, AnyElement, SharedString};
use gpui_component::Icon;

use super::bottom_panel_views::{MessageFilter, PanelMessage};
use cellar_desktop_gpui::{
    model::{TabKind, WorkspaceTab},
    theme::{
        accent_soft, ACCENT, BORDER_DIVIDER, FG, FG_MUTED, FG_SECONDARY, FG_TERTIARY, INSERT,
        INSET, PANEL, PANEL_RAISED, PROD, WARN, WARN_SOFT,
    },
};

pub(super) fn message_header() -> AnyElement {
    div()
        .h(px(25.))
        .flex_shrink_0()
        .flex()
        .items_center()
        .border_b_1()
        .border_color(BORDER_DIVIDER)
        .bg(INSET)
        .text_size(px(12.))
        .text_color(FG_MUTED)
        .children([
            message_cell("time", 130., true),
            message_cell("level", 88., true),
            message_cell("source", 108., true),
            message_cell("message", 480., true),
            message_cell("metrics", 180., false),
        ])
        .into_any_element()
}

pub(super) fn message_row(message: &PanelMessage) -> AnyElement {
    div()
        .flex()
        .items_start()
        .border_b_1()
        .border_color(BORDER_DIVIDER)
        .hover(|style| style.bg(PANEL))
        .child(message_value(&message.time, 130., FG_MUTED, true))
        .child(message_value(
            message.level.label(),
            88.,
            message_level_color(message.level),
            true,
        ))
        .child(message_value(message.source, 108., FG_SECONDARY, true))
        .child(message_value(&message.text, 480., FG, true))
        .child(message_value(&message.metrics, 180., FG_MUTED, false))
        .into_any_element()
}

fn message_cell(label: &'static str, width: f32, border: bool) -> AnyElement {
    div()
        .w(px(width))
        .flex_shrink_0()
        .px(px(10.))
        .when(border, |cell| {
            cell.border_r_1().border_color(BORDER_DIVIDER)
        })
        .child(label)
        .into_any_element()
}

fn message_value(
    value: impl Into<SharedString>,
    width: f32,
    color: cellar_desktop_gpui::theme::DynamicColor,
    border: bool,
) -> AnyElement {
    div()
        .w(px(width))
        .flex_shrink_0()
        .overflow_hidden()
        .px(px(10.))
        .py(px(6.))
        .line_height(px(17.))
        .text_color(color)
        .when(border, |cell| {
            cell.border_r_1().border_color(BORDER_DIVIDER)
        })
        .child(value.into())
        .into_any_element()
}

pub(super) fn panel_messages(
    summary: &QueryResultSummary,
    query: bool,
    row_limit: u64,
) -> Vec<PanelMessage> {
    let duration = format_duration(summary.duration_ms as i64);
    let count = summary.rows_affected.unwrap_or(summary.row_count);
    let mut messages = query
        .then(|| PanelMessage {
            time: "—".into(),
            level: MessageFilter::Info,
            source: "client",
            text: format!(
                "Running statement with row limit {}.",
                format_number(row_limit)
            ),
            metrics: "-".into(),
        })
        .into_iter()
        .collect::<Vec<_>>();
    messages.push(PanelMessage {
        time: "—".into(),
        level: MessageFilter::Success,
        source: "execution",
        text: if summary.rows_affected.is_some() {
            format!("Query OK: {} affected in {duration}.", format_rows(count))
        } else {
            format!("Returned {} in {duration}.", format_rows(count))
        },
        metrics: format!("{} ms | {}", summary.duration_ms, format_rows(count)),
    });
    if summary.truncated {
        messages.push(PanelMessage {
            time: "—".into(),
            level: MessageFilter::Warning,
            source: "execution",
            text: format!(
                "Result hit row limit {}; showing first {}.",
                format_number(row_limit),
                format_rows(summary.row_count)
            ),
            metrics: format!(
                "{} ms | {}",
                summary.duration_ms,
                format_rows(summary.row_count)
            ),
        });
    }
    messages
}

fn format_rows(count: u64) -> String {
    format!(
        "{} row{}",
        format_number(count),
        if count == 1 { "" } else { "s" }
    )
}

fn format_number(value: u64) -> String {
    let digits = value.to_string();
    let mut grouped = String::with_capacity(digits.len() + digits.len() / 3);
    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            grouped.push(',');
        }
        grouped.push(digit);
    }
    grouped
}

pub(super) fn notice_filter(severity: &NoticeSeverity) -> MessageFilter {
    match severity {
        NoticeSeverity::Panic | NoticeSeverity::Fatal | NoticeSeverity::Error => {
            MessageFilter::Error
        }
        NoticeSeverity::Warning => MessageFilter::Warning,
        _ => MessageFilter::Info,
    }
}

pub(super) fn message_level_color(
    level: MessageFilter,
) -> cellar_desktop_gpui::theme::DynamicColor {
    match level {
        MessageFilter::Success => INSERT,
        MessageFilter::Warning => WARN,
        MessageFilter::Error => PROD,
        MessageFilter::Info | MessageFilter::All => ACCENT,
    }
}

pub(super) fn notice_counts(notices: &[DatabaseNotice]) -> Vec<(NoticeSeverity, usize)> {
    [
        NoticeSeverity::Panic,
        NoticeSeverity::Fatal,
        NoticeSeverity::Error,
        NoticeSeverity::Warning,
        NoticeSeverity::Notice,
        NoticeSeverity::Info,
        NoticeSeverity::Log,
        NoticeSeverity::Debug,
        NoticeSeverity::Unknown,
    ]
    .into_iter()
    .filter_map(|severity| {
        let count = notices
            .iter()
            .filter(|notice| notice.severity == severity)
            .count();
        (count > 0).then_some((severity, count))
    })
    .take(4)
    .collect()
}

pub(super) fn notice_severity_label(severity: &NoticeSeverity) -> &'static str {
    match severity {
        NoticeSeverity::Panic => "panic",
        NoticeSeverity::Fatal => "fatal",
        NoticeSeverity::Error => "error",
        NoticeSeverity::Warning => "warning",
        NoticeSeverity::Notice => "notice",
        NoticeSeverity::Info => "info",
        NoticeSeverity::Log => "log",
        NoticeSeverity::Debug => "debug",
        NoticeSeverity::Unknown => "unknown",
    }
}

pub(super) fn notice_state(
    title: impl Into<SharedString>,
    body: impl Into<SharedString>,
) -> AnyElement {
    div()
        .flex_1()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap(px(6.))
        .bg(PANEL)
        .p_6()
        .text_center()
        .child(
            div()
                .text_size(px(12.))
                .font_weight(gpui::FontWeight::MEDIUM)
                .text_color(FG_SECONDARY)
                .child(title.into()),
        )
        .child(
            div()
                .max_w(px(520.))
                .text_size(px(10.5))
                .line_height(px(16.))
                .text_color(FG_MUTED)
                .child(body.into()),
        )
        .into_any_element()
}

pub(super) fn notice_row(notice: &DatabaseNotice) -> AnyElement {
    div()
        .flex()
        .items_start()
        .px_2()
        .py(px(6.))
        .border_b_1()
        .border_color(BORDER_DIVIDER)
        .font_family(cellar_desktop_gpui::theme::mono_font())
        .text_size(px(10.5))
        .line_height(px(15.))
        .child(
            div()
                .w(px(76.))
                .flex_shrink_0()
                .text_color(FG_MUTED)
                .child(format_notice_time(&notice.timestamp)),
        )
        .child(
            div().w(px(84.)).flex_shrink_0().child(
                div()
                    .rounded(px(4.))
                    .px(px(6.))
                    .bg(notice_soft_color(&notice.severity))
                    .text_color(message_level_color(notice_filter(&notice.severity)))
                    .child(format!("{:?}", notice.severity).to_lowercase()),
            ),
        )
        .child(
            div()
                .w(px(82.))
                .flex_shrink_0()
                .text_color(FG_MUTED)
                .child(notice.code.clone().unwrap_or_default()),
        )
        .child(
            div()
                .min_w_0()
                .flex_1()
                .text_color(FG_SECONDARY)
                .child(notice_text(notice)),
        )
        .into_any_element()
}

fn notice_text(notice: &DatabaseNotice) -> String {
    let mut text = notice.message.clone();
    if let Some(detail) = &notice.detail {
        text.push_str("  detail: ");
        text.push_str(detail);
    }
    if let Some(hint) = &notice.hint {
        text.push_str("  hint: ");
        text.push_str(hint);
    }
    text
}

pub(super) fn notice_soft_color(severity: &NoticeSeverity) -> gpui::Rgba {
    match notice_filter(severity) {
        MessageFilter::Error => gpui::Rgba {
            a: 0.12,
            ..PROD.rgba()
        },
        MessageFilter::Warning => WARN_SOFT.rgba(),
        _ => accent_soft(),
    }
}

pub(super) fn panel_icon_button(id: &'static str, path: &'static str) -> gpui::Stateful<gpui::Div> {
    div()
        .id(id)
        .tab_index(0)
        .cursor_pointer()
        .size(px(22.))
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(4.))
        .text_color(FG_TERTIARY)
        .hover(|style| style.bg(PANEL_RAISED).text_color(FG))
        .child(Icon::empty().path(path).size(px(11.)))
}

fn format_notice_time(timestamp: &str) -> String {
    DateTime::parse_from_rfc3339(timestamp).map_or_else(
        |_| timestamp.chars().take(12).collect(),
        |time| {
            time.with_timezone(&Local)
                .format("%H:%M:%S%.3f")
                .to_string()
        },
    )
}

pub(super) fn tab_database(tab: &WorkspaceTab) -> Option<&str> {
    match &tab.kind {
        TabKind::Table { target, .. } => Some(&target.database),
        TabKind::Query { target, .. } => Some(&target.database),
        TabKind::ErDiagram { target, .. } => Some(&target.database),
        TabKind::SchemaCompare { config, .. } => config
            .target
            .database()
            .or_else(|| config.source.database()),
    }
}

pub(super) fn format_duration(ms: i64) -> String {
    if ms < 1_000 {
        format!("{ms} ms")
    } else if ms < 10_000 {
        format!("{:.2} s", ms as f64 / 1_000.)
    } else {
        format!("{:.1} s", ms as f64 / 1_000.)
    }
}

pub(super) fn format_history_time(ms: i64) -> String {
    DateTime::from_timestamp_millis(ms).map_or_else(
        || ms.to_string(),
        |time| {
            time.with_timezone(&Local)
                .format("%b %-d, %H:%M")
                .to_string()
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notice_severities_use_the_same_message_filter_groups_as_the_react_app() {
        assert_eq!(notice_filter(&NoticeSeverity::Fatal), MessageFilter::Error);
        assert_eq!(
            notice_filter(&NoticeSeverity::Warning),
            MessageFilter::Warning
        );
        assert_eq!(notice_filter(&NoticeSeverity::Notice), MessageFilter::Info);
    }

    #[test]
    fn history_durations_match_the_classic_display() {
        assert_eq!(format_duration(42), "42 ms");
        assert_eq!(format_duration(1_234), "1.23 s");
        assert_eq!(format_duration(12_345), "12.3 s");
    }

    #[test]
    fn completion_messages_match_classic_wording_and_separate_notices() {
        let messages = panel_messages(
            &QueryResultSummary {
                notices: vec![DatabaseNotice {
                    severity: NoticeSeverity::Notice,
                    code: None,
                    message: "hello".into(),
                    detail: None,
                    hint: None,
                    timestamp: String::new(),
                    connection_id: None,
                    database: None,
                    query_id: None,
                }],
                notice_capture: cellar_core::query::NoticeCapture::supported(),
                rows_affected: None,
                duration_ms: 1_234,
                truncated: false,
                total_rows: None,
                row_count: 1_000,
            },
            true,
            10_000,
        );
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].text, "Running statement with row limit 10,000.");
        assert_eq!(messages[1].text, "Returned 1,000 rows in 1.23 s.");
    }

    #[test]
    fn notice_header_keeps_canonical_severity_order_and_limit() {
        let notice = |severity| DatabaseNotice {
            severity,
            code: None,
            message: String::new(),
            detail: None,
            hint: None,
            timestamp: String::new(),
            connection_id: None,
            database: None,
            query_id: None,
        };
        let counts = notice_counts(&[
            notice(NoticeSeverity::Info),
            notice(NoticeSeverity::Error),
            notice(NoticeSeverity::Warning),
            notice(NoticeSeverity::Fatal),
            notice(NoticeSeverity::Notice),
        ]);
        assert_eq!(counts.len(), 4);
        assert_eq!(counts[0].0, NoticeSeverity::Fatal);
        assert_eq!(counts[1].0, NoticeSeverity::Error);
        assert_eq!(counts[2].0, NoticeSeverity::Warning);
        assert_eq!(counts[3].0, NoticeSeverity::Notice);
    }
}
