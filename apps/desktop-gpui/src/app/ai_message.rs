use std::ops::Range;

use gpui::{
    div, prelude::*, rems, AnyElement, App, ClipboardItem, Context, HighlightStyle, SharedString,
    StyledText, Window,
};
use gpui_component::{
    highlighter::SyntaxHighlighter,
    scroll::ScrollableElement,
    text::{TextView, TextViewStyle},
    ActiveTheme, Icon,
};

use super::CellarApp;
use cellar_desktop_gpui::{
    model::{QueryTarget, TabKind},
    theme::{ui_px, ACCENT, BORDER, FG, FG_MUTED, FG_SECONDARY, INSET, PANEL_RAISED},
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
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        div()
            .flex()
            .flex_col()
            .gap(ui_px(10.))
            .children(parse_segments(content).into_iter().enumerate().map(
                |(segment_index, segment)| match segment {
                    Segment::Text(text) => render_text(
                        SharedString::from(format!("ai-text-{message_index}-{segment_index}")),
                        text,
                        window,
                        cx,
                    ),
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
        let runnable = sql && (self.ai.allow_writes || can_run_from_ai(&code));
        let id = move |action| {
            SharedString::from(format!("ai-{action}-{message_index}-{segment_index}"))
        };
        let highlighted = highlight_code(&code, if sql { "sql" } else { &language }, cx);
        div()
            .overflow_hidden()
            .rounded(ui_px(8.))
            .border_1()
            .border_color(BORDER)
            .bg(INSET)
            .child(
                div()
                    .h(ui_px(30.))
                    .flex()
                    .items_center()
                    .justify_between()
                    .bg(PANEL_RAISED)
                    .pl(ui_px(12.))
                    .pr(ui_px(6.))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(ui_px(6.))
                            .text_size(ui_px(11.5))
                            .text_color(FG_SECONDARY)
                            .child(Icon::empty().path("icons/bracket.svg").size(ui_px(11.)))
                            .child(if language.is_empty() {
                                "Code".to_owned()
                            } else if sql {
                                "SQL".to_owned()
                            } else {
                                language.clone()
                            }),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(ui_px(2.))
                            .child(
                                code_action(id("copy"), "icons/copy.svg", "Copy", true).on_click({
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
                                        code_action(id("insert"), "icons/edit.svg", "Insert", true)
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
                                            "Run",
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
                    .id(id("code"))
                    .overflow_x_scrollbar()
                    .px(ui_px(12.))
                    .py(ui_px(10.))
                    .font_family(cellar_desktop_gpui::theme::mono_font())
                    .text_size(ui_px(12.5))
                    .line_height(ui_px(19.))
                    .text_color(FG)
                    .whitespace_nowrap()
                    .child(highlighted),
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
        if !self.ai.allow_writes && !can_run_from_ai(sql) {
            return;
        }
        if let Some(tab_id) = self.insert_ai_sql(sql, window, cx) {
            self.start_query_all(tab_id, window, cx);
        }
    }
}

fn render_text(
    id: SharedString,
    text: String,
    window: &mut Window,
    cx: &mut Context<CellarApp>,
) -> AnyElement {
    let style = TextViewStyle {
        paragraph_gap: rems(0.6),
        heading_base_font_size: ui_px(13.),
        highlight_theme: cx.theme().highlight_theme.clone(),
        is_dark: cx.theme().is_dark(),
        ..TextViewStyle::default()
    };
    div()
        .text_size(ui_px(13.))
        .line_height(ui_px(20.))
        .text_color(FG)
        .child(
            TextView::markdown(id, text, window, cx)
                .style(style)
                .selectable(true),
        )
        .into_any_element()
}

/// Colors code with the same tree-sitter grammar and theme as the query editor.
fn highlight_code(code: &str, language: &str, cx: &App) -> StyledText {
    let mut highlighter = SyntaxHighlighter::new(&language.to_ascii_lowercase());
    highlighter.update(None, &ropey::Rope::from_str(code));
    let theme = &cx.theme().highlight_theme;
    let mut styles = highlighter.styles(&(0..code.len()), theme);
    if language.eq_ignore_ascii_case("sql") {
        styles = fill_sql_keywords(code, styles, theme.style("keyword").unwrap_or_default());
    }
    StyledText::new(code.to_owned()).with_highlights(styles)
}

/// The SQL grammar stops recognising keywords after dialect syntax it lacks,
/// such as T-SQL `TOP`, leaving the rest of the statement uncoloured. Colour
/// known keywords that the grammar left unstyled.
fn fill_sql_keywords(
    code: &str,
    styles: Vec<(Range<usize>, HighlightStyle)>,
    keyword: HighlightStyle,
) -> Vec<(Range<usize>, HighlightStyle)> {
    const KEYWORDS: &[&str] = &[
        "ALL",
        "AND",
        "APPLY",
        "AS",
        "ASC",
        "BEGIN",
        "BETWEEN",
        "BY",
        "CASE",
        "COMMIT",
        "CROSS",
        "DECLARE",
        "DELETE",
        "DESC",
        "DISTINCT",
        "ELSE",
        "END",
        "EXISTS",
        "FETCH",
        "FROM",
        "FULL",
        "GROUP",
        "HAVING",
        "IN",
        "INNER",
        "INSERT",
        "INTO",
        "IS",
        "JOIN",
        "LEFT",
        "LIKE",
        "LIMIT",
        "MERGE",
        "NEXT",
        "NOT",
        "NULL",
        "OFFSET",
        "ON",
        "ONLY",
        "OR",
        "ORDER",
        "OUTER",
        "OUTPUT",
        "OVER",
        "PARTITION",
        "PERCENT",
        "RIGHT",
        "ROLLBACK",
        "ROWS",
        "SELECT",
        "SET",
        "THEN",
        "TIES",
        "TOP",
        "TRAN",
        "TRANSACTION",
        "UNION",
        "UPDATE",
        "VALUES",
        "WHEN",
        "WHERE",
        "WITH",
    ];
    let mut per_byte = vec![HighlightStyle::default(); code.len()];
    for (range, style) in styles {
        per_byte[range].fill(style);
    }
    let is_word = |byte: u8| byte.is_ascii_alphanumeric() || byte == b'_';
    let bytes = code.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        let start = index;
        while index < bytes.len() && is_word(bytes[index]) {
            index += 1;
        }
        if index == start {
            index += 1;
            continue;
        }
        let word = code[start..index].to_ascii_uppercase();
        let unstyled = per_byte[start..index]
            .iter()
            .all(|style| *style == HighlightStyle::default());
        if unstyled && KEYWORDS.contains(&word.as_str()) {
            per_byte[start..index].fill(keyword);
        }
    }
    // Runs only break at grammar ranges or ASCII word edges: char boundaries.
    let mut runs: Vec<(Range<usize>, HighlightStyle)> = Vec::new();
    for (index, style) in per_byte.into_iter().enumerate() {
        match runs.last_mut() {
            Some((range, last)) if *last == style => range.end = index + 1,
            _ => runs.push((index..index + 1, style)),
        }
    }
    runs
}

fn code_action(
    id: SharedString,
    icon: &'static str,
    label: &'static str,
    enabled: bool,
) -> gpui::Stateful<gpui::Div> {
    div()
        .id(id)
        .h(ui_px(22.))
        .flex()
        .items_center()
        .gap(ui_px(4.))
        .rounded(ui_px(4.))
        .px(ui_px(6.))
        .text_size(ui_px(11.5))
        .text_color(if label == "Run" { ACCENT } else { FG_MUTED })
        .opacity(if enabled { 1. } else { 0.4 })
        .when(enabled, |button| {
            button
                .tab_index(0)
                .cursor_pointer()
                .hover(|style| style.bg(INSET).text_color(FG))
        })
        .child(Icon::empty().path(icon).size(ui_px(11.)))
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

#[cfg(test)]
mod tests {
    use super::{can_run_from_ai, fill_sql_keywords, parse_segments, Segment};
    use gpui::{FontWeight, HighlightStyle};

    #[test]
    fn keywords_the_grammar_missed_are_coloured_outside_strings() {
        let code = "SELECT TOP 5 * FROM t WHERE n = 'by' ORDER BY n DESC;";
        let keyword = HighlightStyle {
            font_weight: Some(FontWeight::BOLD),
            ..Default::default()
        };
        let string = HighlightStyle {
            font_weight: Some(FontWeight::LIGHT),
            ..Default::default()
        };
        let quote = code.find("'by'").unwrap();
        let runs = fill_sql_keywords(
            code,
            vec![(0..6, keyword), (quote..quote + 4, string)],
            keyword,
        );
        let coloured = runs
            .iter()
            .filter(|(_, style)| *style == keyword)
            .map(|(range, _)| &code[range.clone()])
            .collect::<Vec<_>>();
        assert_eq!(
            coloured,
            ["SELECT", "TOP", "FROM", "WHERE", "ORDER", "BY", "DESC"]
        );
        assert_eq!(runs.last().unwrap().0.end, code.len());
    }

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
