use cellar_runtime::history::QueryHistoryRecord;
use gpui::{div, prelude::*, px, AnyElement, ClipboardItem, Context, SharedString};
use gpui_component::{input::Input, scroll::ScrollableElement, Icon};

use super::{bottom_panel_support::*, shell_widgets::bottom_empty, CellarApp};
use cellar_desktop_gpui::model::{QueryState, TabKind, TableLoadState, WorkspaceTab};
use cellar_desktop_gpui::theme::{
    accent_soft, ACCENT, ACCENT_FG, BORDER, BORDER_DIVIDER, DELETE_SOFT, FG, FG_DISABLED, FG_MUTED,
    FG_SECONDARY, INSERT, INSERT_SOFT, INSET, PANEL, PANEL_MUTED, PROD, WARN,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum MessageFilter {
    All,
    Success,
    Warning,
    Error,
    Info,
}

impl MessageFilter {
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Success => "success",
            Self::Warning => "warning",
            Self::Error => "error",
            Self::Info => "info",
        }
    }
}

pub(super) struct PanelMessage {
    pub(super) time: String,
    pub(super) level: MessageFilter,
    pub(super) source: &'static str,
    pub(super) text: String,
    pub(super) metrics: String,
}

impl CellarApp {
    pub(super) fn bottom_messages_panel(&self, cx: &mut Context<Self>) -> AnyElement {
        let Some(tab) = self.model.active_tab() else {
            return bottom_empty(
                "No active tab",
                "Open a table or query tab to see execution feedback.",
            );
        };
        let messages = if let Some(summary) = self.query_summaries.get(&tab.id) {
            let (query, row_limit) = match &tab.kind {
                TabKind::Query { .. } => (true, 10_000),
                TabKind::Table { page, .. } => (false, u64::from(page.limit)),
                TabKind::ErDiagram { .. } | TabKind::SchemaCompare { .. } => (false, 10_000),
            };
            panel_messages(summary, query, row_limit)
        } else if let Some((source, text)) = tab_error_message(tab) {
            vec![PanelMessage {
                time: "—".into(),
                level: MessageFilter::Error,
                source,
                text,
                metrics: "-".into(),
            }]
        } else if matches!(
            &tab.kind,
            TabKind::Query {
                state: QueryState::Running { .. },
                ..
            }
        ) {
            vec![PanelMessage {
                time: "—".into(),
                level: MessageFilter::Info,
                source: "client",
                text: "Running statement with row limit 10,000.".into(),
                metrics: "-".into(),
            }]
        } else {
            return bottom_empty(
                "No execution messages",
                "Run or refresh the active tab to populate status, warnings, and errors.",
            );
        };
        let visible = messages
            .iter()
            .filter(|message| {
                self.bottom_message_filter == MessageFilter::All
                    || message.level == self.bottom_message_filter
            })
            .collect::<Vec<_>>();

        div()
            .flex_1()
            .min_h_0()
            .flex()
            .flex_col()
            .bg(INSET)
            .child(
                div()
                    .h(px(32.))
                    .flex_shrink_0()
                    .flex()
                    .items_center()
                    .justify_between()
                    .px_2()
                    .border_b_1()
                    .border_color(BORDER_DIVIDER)
                    .child(
                        div().flex().items_center().gap_1().children(
                            [
                                MessageFilter::All,
                                MessageFilter::Success,
                                MessageFilter::Warning,
                                MessageFilter::Error,
                                MessageFilter::Info,
                            ]
                            .into_iter()
                            .map(|filter| {
                                let active = self.bottom_message_filter == filter;
                                let count = if filter == MessageFilter::All {
                                    messages.len()
                                } else {
                                    messages
                                        .iter()
                                        .filter(|message| message.level == filter)
                                        .count()
                                };
                                div()
                                    .id(SharedString::from(format!(
                                        "message-filter:{}",
                                        filter.label()
                                    )))
                                    .tab_index(0)
                                    .cursor_pointer()
                                    .h(px(22.))
                                    .flex()
                                    .items_center()
                                    .gap_1()
                                    .px_2()
                                    .rounded(px(4.))
                                    .bg(if active { accent_soft() } else { INSET.rgba() })
                                    .font_family(cellar_desktop_gpui::theme::mono_font())
                                    .text_size(px(12.))
                                    .text_color(if active { ACCENT } else { FG_SECONDARY })
                                    .hover(|style| style.bg(PANEL_MUTED).text_color(FG))
                                    .child(filter.label())
                                    .child(
                                        div()
                                            .text_color(if active { ACCENT } else { FG_MUTED })
                                            .child(count.to_string()),
                                    )
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        this.bottom_message_filter = filter;
                                        cx.notify();
                                    }))
                            }),
                        ),
                    )
                    .child(
                        div()
                            .font_family(cellar_desktop_gpui::theme::mono_font())
                            .text_size(px(12.))
                            .text_color(FG_MUTED)
                            .child(format!("{} total", messages.len())),
                    ),
            )
            .child(if visible.is_empty() {
                bottom_empty(
                    "No matching messages",
                    "Adjust the severity filter to inspect the current execution feedback.",
                )
            } else {
                div()
                    .flex_1()
                    .min_h_0()
                    .overflow_scrollbar()
                    .child(
                        div()
                            .min_w(px(986.))
                            .flex()
                            .flex_col()
                            .font_family(cellar_desktop_gpui::theme::mono_font())
                            .text_size(px(12.))
                            .child(message_header())
                            .children(visible.into_iter().map(message_row)),
                    )
                    .into_any_element()
            })
            .into_any_element()
    }

    pub(super) fn bottom_history_panel(&self, cx: &mut Context<Self>) -> AnyElement {
        let active = self.model.active_tab();
        let scope = active.map_or_else(
            || "No active tab".to_owned(),
            |tab| {
                tab_database(tab).map_or_else(
                    || tab.title.clone(),
                    |database| format!("{database}.{}", tab.title),
                )
            },
        );
        div()
            .flex_1()
            .min_h_0()
            .flex()
            .flex_col()
            .bg(INSET)
            .child(
                div()
                    .h(px(36.))
                    .flex_shrink_0()
                    .flex()
                    .items_center()
                    .gap_2()
                    .px_2()
                    .border_b_1()
                    .border_color(BORDER_DIVIDER)
                    .child(
                        div()
                            .h(px(24.))
                            .min_w(px(180.))
                            .max_w(px(360.))
                            .flex_1()
                            .flex()
                            .items_center()
                            .gap_1()
                            .px(px(6.))
                            .rounded(px(4.))
                            .border_1()
                            .border_color(BORDER)
                            .bg(PANEL)
                            .child(
                                Icon::empty()
                                    .path("icons/search.svg")
                                    .size(px(11.))
                                    .text_color(FG_MUTED),
                            )
                            .child(
                                div().h_full().min_w_0().flex_1().child(
                                    Input::new(&self.bottom_history_search)
                                        .h_full()
                                        .appearance(false),
                                ),
                            ),
                    )
                    .child(
                        div()
                            .min_w_0()
                            .truncate()
                            .font_family(cellar_desktop_gpui::theme::mono_font())
                            .text_size(px(10.5))
                            .text_color(FG_MUTED)
                            .child(scope),
                    ),
            )
            .child(if active.is_none() {
                bottom_empty("Open a tab to view scoped history", "")
            } else if let Some(error) = self.history_error.as_deref() {
                bottom_empty("History unavailable", error.to_owned())
            } else if self.history_loading && self.history_records.is_empty() {
                bottom_empty("Loading history...", "")
            } else if self.history_records.is_empty() {
                bottom_empty(
                    if self
                        .bottom_history_search
                        .read(cx)
                        .value()
                        .trim()
                        .is_empty()
                    {
                        "No queries recorded for this tab"
                    } else {
                        "No matching queries"
                    },
                    "History starts filling as queries execute locally.",
                )
            } else {
                div()
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scrollbar()
                    .children(
                        self.history_records
                            .iter()
                            .cloned()
                            .map(|record| self.history_row(record, cx)),
                    )
                    .into_any_element()
            })
            .into_any_element()
    }

    fn history_row(&self, record: QueryHistoryRecord, cx: &mut Context<Self>) -> AnyElement {
        let copy_sql = record.sql.clone();
        let reuse = record.clone();
        let rows = record.row_count.map_or_else(
            || "no rows".to_owned(),
            |count| format!("{count} {}", if count == 1 { "row" } else { "rows" }),
        );
        div()
            .id(SharedString::from(format!("bottom-history:{}", record.id)))
            .flex()
            .items_start()
            .gap_3()
            .px(px(10.))
            .py_2()
            .border_b_1()
            .border_color(BORDER_DIVIDER)
            .hover(|style| style.bg(PANEL))
            .child(
                div()
                    .min_w_0()
                    .flex_1()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(6.))
                            .font_family(cellar_desktop_gpui::theme::mono_font())
                            .text_size(px(10.5))
                            .text_color(FG_SECONDARY)
                            .child(
                                div()
                                    .rounded(px(3.))
                                    .px_1()
                                    .bg(if record.success {
                                        INSERT_SOFT.rgba()
                                    } else {
                                        DELETE_SOFT.rgba()
                                    })
                                    .text_color(if record.success { INSERT } else { PROD })
                                    .child(if record.success { "ok" } else { "error" }),
                            )
                            .child(format_duration(record.duration_ms))
                            .child(div().text_color(FG_DISABLED).child("·"))
                            .child(rows)
                            .when(record.truncated, |line| {
                                line.child(div().text_color(WARN).child("truncated"))
                            })
                            .child(div().text_color(FG_DISABLED).child("·"))
                            .child(
                                div()
                                    .text_color(FG_MUTED)
                                    .child(format_history_time(record.executed_at_ms)),
                            ),
                    )
                    .child(
                        div()
                            .max_h(px(58.))
                            .overflow_hidden()
                            .font_family(cellar_desktop_gpui::theme::mono_font())
                            .text_size(px(11.))
                            .line_height(px(16.))
                            .text_color(FG_SECONDARY)
                            .child(record.sql),
                    )
                    .when_some(record.error_summary, |row, error| {
                        row.child(
                            div()
                                .truncate()
                                .font_family(cellar_desktop_gpui::theme::mono_font())
                                .text_size(px(10.5))
                                .text_color(PROD)
                                .child(error),
                        )
                    }),
            )
            .child(
                div()
                    .flex_shrink_0()
                    .flex()
                    .items_start()
                    .gap_1()
                    .child(
                        panel_icon_button("copy-history", "icons/copy.svg").on_click(
                            move |_, _, cx| {
                                cx.stop_propagation();
                                cx.write_to_clipboard(ClipboardItem::new_string(copy_sql.clone()));
                            },
                        ),
                    )
                    .child(
                        panel_icon_button("reuse-history", "icons/edit.svg").on_click(cx.listener(
                            move |this, _, window, cx| {
                                cx.stop_propagation();
                                this.reuse_history(reuse.clone(), window, cx);
                            },
                        )),
                    ),
            )
            .into_any_element()
    }

    pub(super) fn bottom_notices_panel(&self, cx: &mut Context<Self>) -> AnyElement {
        let active = self.model.active_tab();
        let tab_id = active.map(|tab| tab.id);
        let notices = tab_id
            .and_then(|id| self.query_summaries.get(&id))
            .map(|summary| summary.notices.as_slice())
            .unwrap_or_default();
        let capture = tab_id
            .and_then(|id| self.query_summaries.get(&id))
            .map(|summary| &summary.notice_capture);
        let severity_counts = notice_counts(notices);
        let retained = tab_id.is_some_and(|id| self.bottom_retain_notice_tabs.contains(&id));
        let connection = self.model.active_connection();
        let context = active.map_or_else(
            || "no active query tab".to_owned(),
            |tab| {
                format!(
                    "{} / {}",
                    connection.map_or("", |config| config.name.as_str()),
                    tab.title
                )
            },
        );
        let engine = connection.map(|config| super::shell_widgets::dialect_label(config.engine));

        div()
            .flex_1()
            .min_h_0()
            .flex()
            .flex_col()
            .bg(PANEL)
            .child(
                div()
                    .h(px(32.))
                    .flex_shrink_0()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_3()
                    .px_2()
                    .border_b_1()
                    .border_color(BORDER_DIVIDER)
                    .child(
                        div()
                            .min_w_0()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(div().text_color(FG_SECONDARY).child("Database notices"))
                            .child(
                                div()
                                    .min_w_0()
                                    .truncate()
                                    .font_family(cellar_desktop_gpui::theme::mono_font())
                                    .text_size(px(10.5))
                                    .text_color(FG_MUTED)
                                    .child(context),
                            )
                            .when_some(engine, |line, engine| {
                                line.child(
                                    div()
                                        .rounded(px(4.))
                                        .border_1()
                                        .border_color(BORDER)
                                        .px(px(6.))
                                        .font_family(cellar_desktop_gpui::theme::mono_font())
                                        .text_size(px(9.5))
                                        .text_color(FG_MUTED)
                                        .child(engine),
                                )
                            }),
                    )
                    .child(
                        div()
                            .flex_shrink_0()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_1()
                                    .children(severity_counts.into_iter().map(
                                        |(severity, count)| {
                                            div()
                                                .rounded(px(4.))
                                                .px(px(6.))
                                                .font_family(
                                                    cellar_desktop_gpui::theme::mono_font(),
                                                )
                                                .text_size(px(9.5))
                                                .bg(notice_soft_color(&severity))
                                                .text_color(message_level_color(notice_filter(
                                                    &severity,
                                                )))
                                                .child(format!(
                                                    "{}:{count}",
                                                    notice_severity_label(&severity)
                                                ))
                                        },
                                    ))
                                    .when(notices.is_empty(), |counts| {
                                        counts.child(
                                            div()
                                                .font_family(
                                                    cellar_desktop_gpui::theme::mono_font(),
                                                )
                                                .text_size(px(10.))
                                                .text_color(FG_MUTED)
                                                .child("0"),
                                        )
                                    }),
                            )
                            .child(
                                div()
                                    .id("retain-notices")
                                    .cursor_pointer()
                                    .flex()
                                    .items_center()
                                    .gap(px(6.))
                                    .text_size(px(10.5))
                                    .text_color(FG_SECONDARY)
                                    .child(
                                        div()
                                            .size(px(12.))
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .border_1()
                                            .border_color(BORDER)
                                            .rounded(px(2.))
                                            .when(retained, |box_| {
                                                box_.bg(ACCENT).child(
                                                    Icon::empty()
                                                        .path("icons/check.svg")
                                                        .size(px(9.))
                                                        .text_color(ACCENT_FG),
                                                )
                                            }),
                                    )
                                    .child("Retain")
                                    .when_some(tab_id, |button, tab_id| {
                                        button.tab_index(0).on_click(cx.listener(move |this, _, _, cx| {
                                            if !this.bottom_retain_notice_tabs.remove(&tab_id) {
                                                this.bottom_retain_notice_tabs.insert(tab_id);
                                            }
                                            cx.notify();
                                        }))
                                    }),
                            )
                            .child(
                                div()
                                    .id("clear-notices")
                                    .h(px(20.))
                                    .flex()
                                    .items_center()
                                    .px_2()
                                    .rounded(px(4.))
                                    .border_1()
                                    .border_color(BORDER)
                                    .text_size(px(10.5))
                                    .text_color(FG_SECONDARY)
                                    .when(notices.is_empty(), |button| button.opacity(0.45))
                                    .when(!notices.is_empty(), |button| {
                                        button
                                            .tab_index(0)
                                            .cursor_pointer()
                                            .hover(|style| style.bg(PANEL_MUTED).text_color(FG))
                                            .when_some(tab_id, |button, tab_id| {
                                                button.on_click(cx.listener(
                                                    move |this, _, _, cx| {
                                                        if let Some(summary) =
                                                            this.query_summaries.get_mut(&tab_id)
                                                        {
                                                            summary.notices.clear();
                                                        }
                                                        cx.notify();
                                                    },
                                                ))
                                            })
                                    })
                                    .child("Clear"),
                            ),
                    ),
            )
            .child(if active.is_none() {
                notice_state(
                    "No active query tab",
                    "Open a table or query tab to collect database-emitted notices for that scope.",
                )
            } else if capture.is_some_and(|capture| !capture.supported) {
                notice_state(
                    "Notice capture unavailable",
                    capture
                        .and_then(|capture| capture.reason.clone())
                        .unwrap_or_else(|| {
                            "The current driver path cannot observe database notice frames."
                                .into()
                        }),
                )
            } else if notices.is_empty() {
                notice_state(
                    "No database notices",
                    "This scope has not received Postgres NOTICE/RAISE output or engine-equivalent messages.",
                )
            } else {
                div()
                    .flex_1()
                    .min_h_0()
                    .overflow_scrollbar()
                    .child(
                        div()
                            .min_w(px(720.))
                            .children(notices.iter().map(notice_row)),
                    )
                    .into_any_element()
            })
            .into_any_element()
    }
}

fn tab_error_message(tab: &WorkspaceTab) -> Option<(&'static str, String)> {
    match &tab.kind {
        TabKind::Table {
            target,
            state: TableLoadState::Error(error),
            ..
        } => Some((
            "driver",
            format!("Failed to load {}.{}: {error}", target.schema, target.table),
        )),
        TabKind::Query {
            state: QueryState::Error(error),
            ..
        } => Some(("driver", format!("Failed to run statement: {error}"))),
        _ => None,
    }
}
