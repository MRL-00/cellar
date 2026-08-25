use gpui::{div, prelude::*, px, AnyElement, ClipboardItem, Context, SharedString, Window};
use gpui_component::{scroll::ScrollableElement, Icon};

use super::CellarApp;
use cellar_desktop_gpui::{
    model::{QueryTarget, TabKind},
    theme::{ACCENT, BORDER, FG, FG_MUTED, FG_SECONDARY, INSET, PANEL_RAISED},
};

#[derive(Debug, PartialEq)]
enum Segment {
    Text(String),
    Code { language: String, code: String },
}

impl CellarApp {
    pub(super) fn render_ai_message(
        &self,
        content: &str,
        message_index: usize,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        div()
            .flex()
            .flex_col()
            .gap_2()
            .children(parse_segments(content).into_iter().enumerate().map(
                |(segment_index, segment)| match segment {
                    Segment::Text(text) => render_text(text),
                    Segment::Code { language, code } => {
                        self.render_ai_code(language, code, message_index, segment_index, cx)
                    }
                },
            ))
            .into_any_element()
    }

    fn render_ai_code(
        &self,
        language: String,
        code: String,
        message_index: usize,
        segment_index: usize,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let sql = is_sql_language(&language);
        let runnable = sql && can_run_from_ai(&code);
        let id = move |action| {
            SharedString::from(format!("ai-{action}-{message_index}-{segment_index}"))
        };
        div()
            .overflow_hidden()
            .rounded(px(5.))
            .border_1()
            .border_color(BORDER)
            .bg(INSET)
            .child(
                div()
                    .h(px(25.))
                    .flex()
                    .items_center()
                    .justify_between()
                    .border_b_1()
                    .border_color(BORDER)
                    .px_2()
                    .child(
                        div()
                            .font_family(cellar_desktop_gpui::theme::mono_font())
                            .text_size(px(10.5))
                            .text_color(FG_MUTED)
                            .child(if language.is_empty() {
                                "code".to_owned()
                            } else {
                                language.to_uppercase()
                            }),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            .child(
                                code_action(id("copy"), "icons/copy.svg", "copy", true).on_click({
                                    let code = code.clone();
                                    move |_, _, cx| {
                                        cx.write_to_clipboard(ClipboardItem::new_string(
                                            code.clone(),
                                        ))
                                    }
                                }),
                            )
                            .when(sql, |element| {
                                element
                                    .child(
                                        code_action(id("insert"), "icons/edit.svg", "insert", true)
                                            .on_click(cx.listener({
                                                let code = code.clone();
                                                move |this, _, window, cx| {
                                                    this.insert_ai_sql(&code, window, cx);
                                                }
                                            })),
                                    )
                                    .child(
                                        code_action(
                                            id("run"),
                                            "icons/play-small.svg",
                                            "run",
                                            runnable,
                                        )
                                        .when(
                                            runnable,
                                            |button| {
                                                button.on_click(cx.listener({
                                                    let code = code.clone();
                                                    move |this, _, window, cx| {
                                                        this.run_ai_sql(&code, window, cx)
                                                    }
                                                }))
                                            },
                                        ),
                                    )
                            }),
                    ),
            )
            .child(
                div()
                    .overflow_x_scrollbar()
                    .px(px(10.))
                    .py_2()
                    .font_family(cellar_desktop_gpui::theme::mono_font())
                    .text_size(px(13.))
                    .text_color(FG)
                    .whitespace_nowrap()
                    .child(code),
            )
            .into_any_element()
    }

    fn insert_ai_sql(
        &mut self,
        sql: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<u64> {
        let active = self.model.active_tab()?.clone();
        let tab_id = match active.kind {
            TabKind::Query { .. } => active.id,
            TabKind::Table { target, .. } => self.open_query(
                QueryTarget {
                    connection_id: target.connection_id,
                    database: target.database,
                },
                String::new(),
                window,
                cx,
            ),
            TabKind::ErDiagram { target, .. } => self.open_query(
                QueryTarget {
                    connection_id: target.connection_id,
                    database: target.database,
                },
                String::new(),
                window,
                cx,
            ),
            TabKind::SchemaCompare { config, .. } => self.open_query(
                QueryTarget {
                    connection_id: config
                        .source
                        .live_connection_id()
                        .or_else(|| config.target.live_connection_id())?
                        .to_owned(),
                    database: config
                        .source
                        .database()
                        .or_else(|| config.target.database())?
                        .to_owned(),
                },
                String::new(),
                window,
                cx,
            ),
        };
        let sql = format!("{}\n", sql.trim());
        self.editors
            .get(&tab_id)?
            .update(cx, |editor, cx| editor.set_value(sql, window, cx));
        self.model.select_tab(tab_id);
        cx.notify();
        Some(tab_id)
    }

    fn run_ai_sql(&mut self, sql: &str, window: &mut Window, cx: &mut Context<Self>) {
        if !can_run_from_ai(sql) {
            return;
        }
        if let Some(tab_id) = self.insert_ai_sql(sql, window, cx) {
            self.start_query_all(tab_id, window, cx);
        }
    }
}

fn render_text(text: String) -> AnyElement {
    div()
        .text_size(px(13.))
        .text_color(FG_SECONDARY)
        .children(
            text.lines()
                .filter(|line| !line.trim().is_empty())
                .map(|line| {
                    let trimmed = line.trim();
                    let (heading, body) = if let Some(body) = trimmed
                        .strip_prefix("#### ")
                        .or_else(|| trimmed.strip_prefix("### "))
                        .or_else(|| trimmed.strip_prefix("## "))
                        .or_else(|| trimmed.strip_prefix("# "))
                    {
                        (true, body)
                    } else {
                        (
                            false,
                            trimmed
                                .strip_prefix("- ")
                                .or_else(|| trimmed.strip_prefix("* "))
                                .unwrap_or(trimmed),
                        )
                    };
                    div()
                        .when(heading, |line| {
                            line.font_weight(gpui::FontWeight::SEMIBOLD)
                                .text_color(FG)
                                .pt_1()
                        })
                        .when(
                            trimmed.starts_with("- ") || trimmed.starts_with("* "),
                            |line| line.pl_3().child("- "),
                        )
                        .child(strip_inline_markdown(body))
                }),
        )
        .into_any_element()
}

fn code_action(
    id: SharedString,
    icon: &'static str,
    label: &'static str,
    enabled: bool,
) -> gpui::Stateful<gpui::Div> {
    div()
        .id(id)
        .h(px(20.))
        .flex()
        .items_center()
        .gap_1()
        .rounded(px(3.))
        .px_1()
        .text_size(px(11.))
        .text_color(if label == "run" { ACCENT } else { FG_MUTED })
        .opacity(if enabled { 1. } else { 0.4 })
        .when(enabled, |button| {
            button
                .tab_index(0)
                .cursor_pointer()
                .hover(|style| style.bg(PANEL_RAISED))
        })
        .child(Icon::empty().path(icon).size(px(10.)))
        .child(label)
}

fn parse_segments(content: &str) -> Vec<Segment> {
    let mut segments = Vec::new();
    let mut rest = content;
    while let Some(start) = rest.find("```") {
        let text = rest[..start].trim();
        if !text.is_empty() {
            segments.push(Segment::Text(text.to_owned()));
        }
        let fenced = &rest[start + 3..];
        let Some(end) = fenced.find("```") else {
            segments.push(Segment::Text(rest[start..].to_owned()));
            rest = "";
            break;
        };
        let block = &fenced[..end];
        let (language, code) = block.split_once('\n').unwrap_or(("", block));
        segments.push(Segment::Code {
            language: language.trim().to_owned(),
            code: code.trim().to_owned(),
        });
        rest = &fenced[end + 3..];
    }
    if !rest.trim().is_empty() {
        segments.push(Segment::Text(rest.trim().to_owned()));
    }
    if segments.is_empty() {
        segments.push(Segment::Text(content.to_owned()));
    }
    segments
}

fn is_sql_language(language: &str) -> bool {
    matches!(
        language.trim().to_ascii_lowercase().as_str(),
        "sql" | "postgres" | "postgresql" | "pgsql" | "mysql" | "sqlite" | "tsql" | "mssql"
    )
}

fn can_run_from_ai(sql: &str) -> bool {
    let normalized = sql
        .lines()
        .filter(|line| !line.trim_start().starts_with("--"))
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase();
    let trimmed = normalized.trim_start();
    let read_only = ["select", "with", "show", "describe", "desc", "explain"]
        .iter()
        .any(|keyword| trimmed.starts_with(keyword));
    let write = [
        "insert", "update", "delete", "drop", "truncate", "alter", "create", "merge", "grant",
        "revoke", "vacuum", "analyze",
    ]
    .iter()
    .any(|keyword| {
        normalized
            .split(|c: char| !c.is_ascii_alphanumeric() && c != '_')
            .any(|word| word == *keyword)
    });
    read_only && !write
}

fn strip_inline_markdown(text: &str) -> String {
    text.replace("**", "").replace('`', "")
}

#[cfg(test)]
mod tests {
    use super::{can_run_from_ai, parse_segments, Segment};

    #[test]
    fn parses_fences_and_only_runs_read_only_sql() {
        assert_eq!(
            parse_segments("Try:\n```sql\nselect * from users\n```"),
            vec![
                Segment::Text("Try:".into()),
                Segment::Code {
                    language: "sql".into(),
                    code: "select * from users".into()
                }
            ]
        );
        assert!(can_run_from_ai(
            "-- safe\nWITH x AS (SELECT 1) SELECT * FROM x"
        ));
        assert!(!can_run_from_ai("SELECT 1; DELETE FROM users"));
        assert!(!can_run_from_ai("EXPLAIN ANALYZE SELECT 1"));
    }
}
