use gpui::{div, prelude::*, px, AnyElement, Context, SharedString, Window};
use gpui_component::Icon;

use super::{settings::SettingsCategory, CellarApp};
use cellar_desktop_gpui::theme::{BORDER, FG, FG_MUTED, FG_SECONDARY, INSET, PANEL, PANEL_RAISED};

#[derive(Clone, Copy)]
pub(super) struct Entry {
    pub(super) category: SettingsCategory,
    section: &'static str,
    label: &'static str,
    terms: &'static str,
}

const ENTRIES: &[Entry] = &[
    entry(
        SettingsCategory::General,
        "General",
        "Startup",
        "restore last session empty workspace welcome",
    ),
    entry(
        SettingsCategory::General,
        "General",
        "Default schema search path",
        "public audit analytics",
    ),
    entry(
        SettingsCategory::General,
        "General",
        "Confirm before quitting",
        "quit exit",
    ),
    entry(
        SettingsCategory::General,
        "General",
        "Allow background queries",
        "execution",
    ),
    entry(
        SettingsCategory::General,
        "Updates",
        "Channel",
        "stable beta nightly",
    ),
    entry(
        SettingsCategory::General,
        "Updates",
        "Auto-install on quit",
        "",
    ),
    entry(
        SettingsCategory::Appearance,
        "Theme",
        "Theme",
        "system dark light",
    ),
    entry(
        SettingsCategory::Appearance,
        "Theme",
        "Accent",
        "color colour swatch palette",
    ),
    entry(
        SettingsCategory::Appearance,
        "Theme",
        "Density",
        "compact comfortable",
    ),
    entry(
        SettingsCategory::Appearance,
        "Type",
        "Interface font",
        "sans typeface geist inter",
    ),
    entry(
        SettingsCategory::Appearance,
        "Type",
        "Editor / mono font",
        "monospace sql jetbrains",
    ),
    entry(
        SettingsCategory::Appearance,
        "Type",
        "Font size",
        "scale interface px",
    ),
    entry(
        SettingsCategory::Appearance,
        "Window",
        "Show traffic lights",
        "macos window",
    ),
    entry(
        SettingsCategory::Appearance,
        "Window",
        "Native window controls",
        "title bar",
    ),
    entry(SettingsCategory::Editor, "SQL editor", "Tab size", "2 4 8"),
    entry(
        SettingsCategory::Editor,
        "SQL editor",
        "Indent with",
        "spaces tabs",
    ),
    entry(
        SettingsCategory::Editor,
        "SQL editor",
        "Auto-format on save",
        "format",
    ),
    entry(
        SettingsCategory::Editor,
        "SQL editor",
        "Keyword case",
        "UPPER lower Preserve",
    ),
    entry(
        SettingsCategory::Editor,
        "SQL editor",
        "Show line numbers",
        "gutter",
    ),
    entry(
        SettingsCategory::Editor,
        "SQL editor",
        "Soft wrap",
        "long lines",
    ),
    entry(
        SettingsCategory::Editor,
        "SQL editor",
        "Bracket matching",
        "parentheses pairs",
    ),
    entry(
        SettingsCategory::Editor,
        "Execution",
        "Statement at cursor runs",
        "current statement selection whole file run",
    ),
    entry(
        SettingsCategory::Editor,
        "Execution",
        "LIMIT applied to SELECT *",
        "row limit select",
    ),
    entry(
        SettingsCategory::Grid,
        "Data grid",
        "Row height",
        "20px 22px 28px 36px",
    ),
    entry(
        SettingsCategory::Grid,
        "Data grid",
        "NULL display",
        "dim italic strong empty",
    ),
    entry(
        SettingsCategory::Grid,
        "Data grid",
        "Number alignment",
        "left right",
    ),
    entry(
        SettingsCategory::Grid,
        "Data grid",
        "Stripe alternating rows",
        "zebra",
    ),
    entry(
        SettingsCategory::Grid,
        "Data grid",
        "Remember table sort",
        "column order by persist",
    ),
    entry(
        SettingsCategory::Grid,
        "Data grid",
        "Sticky pkey column",
        "primary key frozen",
    ),
    entry(
        SettingsCategory::Grid,
        "Data grid",
        "Truncate cells over",
        "max cell preview length characters",
    ),
    entry(
        SettingsCategory::Keymap,
        "Keymap",
        "Preset",
        "Cellar DataGrip VS Code Linear",
    ),
    entry(
        SettingsCategory::Keymap,
        "Workspace",
        "Command palette",
        "new connection sql tab settings",
    ),
    entry(
        SettingsCategory::Keymap,
        "Editor",
        "Run statement",
        "run selection format ghost text",
    ),
    entry(
        SettingsCategory::Keymap,
        "Grid",
        "Edit cell",
        "revert commit set null",
    ),
    entry(
        SettingsCategory::Connections,
        "Defaults for new connections",
        "Read-only by default",
        "safety",
    ),
    entry(
        SettingsCategory::Connections,
        "Defaults for new connections",
        "Connection timeout",
        "seconds",
    ),
    entry(
        SettingsCategory::Connections,
        "Defaults for new connections",
        "Keep-alive interval",
        "seconds",
    ),
    entry(
        SettingsCategory::Connections,
        "Defaults for new connections",
        "Application name",
        "cellar client",
    ),
    entry(
        SettingsCategory::Connections,
        "Production safety",
        "Confirm DML on prod",
        "destructive",
    ),
    entry(
        SettingsCategory::Connections,
        "Production safety",
        "Confirm DROP / TRUNCATE on prod",
        "production destructive",
    ),
    entry(
        SettingsCategory::Connections,
        "Production safety",
        "Block UPDATE without WHERE",
        "",
    ),
    entry(
        SettingsCategory::Connections,
        "Production safety",
        "Block DELETE without WHERE",
        "",
    ),
    entry(
        SettingsCategory::Connections,
        "Production safety",
        "Max rows affected before warn",
        "",
    ),
    entry(
        SettingsCategory::History,
        "Query history",
        "Retain history for",
        "7 days 30 days 90 days forever",
    ),
    entry(
        SettingsCategory::History,
        "Query history",
        "Store query results",
        "",
    ),
    entry(
        SettingsCategory::History,
        "Query history",
        "Storage summary",
        "queries last cleared MB",
    ),
    entry(
        SettingsCategory::Backups,
        "Backups",
        "Auto-snapshot before commits",
        "pg_dump schema-only affected rows",
    ),
    entry(
        SettingsCategory::Backups,
        "Backups",
        "Snapshot location",
        "~/.cellar/snapshots",
    ),
    entry(
        SettingsCategory::Backups,
        "Backups",
        "Retain snapshots for",
        "days",
    ),
    entry(
        SettingsCategory::Backups,
        "Export defaults",
        "Format",
        "CSV JSON Parquet SQL INSERT",
    ),
    entry(
        SettingsCategory::Backups,
        "Export defaults",
        "NULL as",
        "\\N",
    ),
    entry(
        SettingsCategory::Backups,
        "Export defaults",
        "Include headers",
        "",
    ),
    entry(
        SettingsCategory::Ai,
        "AI Assistant",
        "Bring-your-own-key",
        "schema queries results privacy",
    ),
    entry(
        SettingsCategory::Ai,
        "Provider",
        "Provider",
        "Anthropic OpenAI Google local custom Ollama",
    ),
    entry(
        SettingsCategory::Ai,
        "Provider",
        "Model",
        "claude gpt gemini fast balanced max",
    ),
    entry(
        SettingsCategory::Ai,
        "Provider",
        "API key",
        "keychain stored secret",
    ),
    entry(
        SettingsCategory::Ai,
        "Provider",
        "Endpoint",
        "proxy custom router OpenAI-compatible",
    ),
    entry(
        SettingsCategory::Ai,
        "Danger zone",
        "Clear AI conversation history",
        "local delete",
    ),
    entry(
        SettingsCategory::Ai,
        "Danger zone",
        "Revoke API key",
        "remove from keychain provider",
    ),
    entry(
        SettingsCategory::Privacy,
        "Telemetry",
        "Send anonymous usage stats",
        "off local private",
    ),
    entry(
        SettingsCategory::Privacy,
        "Telemetry",
        "Send crash reports",
        "stack traces off",
    ),
    entry(
        SettingsCategory::Privacy,
        "Stored locally only",
        "Local data",
        "history snapshots schemas cellar",
    ),
    entry(
        SettingsCategory::Updates,
        "Updates",
        "Version",
        "updater last checked check now",
    ),
    entry(
        SettingsCategory::Updates,
        "Updates",
        "Channel",
        "stable beta nightly",
    ),
    entry(
        SettingsCategory::Updates,
        "Updates",
        "Auto-install on quit",
        "release",
    ),
    entry(
        SettingsCategory::About,
        "About",
        "Cellar",
        "database client development MIT licence",
    ),
    entry(
        SettingsCategory::About,
        "About",
        "Links",
        "docs github changelog acknowledgements",
    ),
];

const fn entry(
    category: SettingsCategory,
    section: &'static str,
    label: &'static str,
    terms: &'static str,
) -> Entry {
    Entry {
        category,
        section,
        label,
        terms,
    }
}

impl CellarApp {
    pub(super) fn settings_search_results(
        &self,
        query: &str,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let results = search(query);
        let count = results.len();
        let query = query.to_owned();
        div()
            .id("settings-search-results")
            .flex_1()
            .min_w_0()
            .h_full()
            .overflow_y_scroll()
            .pb_6()
            .pt_1()
            .bg(PANEL)
            .child(
                div()
                    .px_6()
                    .pt(px(18.))
                    .pb_1()
                    .child(
                        div()
                            .mb_3()
                            .flex()
                            .items_end()
                            .justify_between()
                            .gap_3()
                            .child(
                                div()
                                    .child(
                                        div()
                                            .font_weight(gpui::FontWeight::SEMIBOLD)
                                            .text_color(FG)
                                            .child("Search results"),
                                    )
                                    .child(div().mt(px(1.)).text_color(FG_SECONDARY).child(
                                        if count == 0 {
                                            format!("No settings match \"{query}\"")
                                        } else {
                                            format!(
                                                "{count} match{} for \"{query}\"",
                                                if count == 1 { "" } else { "es" }
                                            )
                                        },
                                    )),
                            )
                            .child(
                                div()
                                    .font_family(cellar_desktop_gpui::theme::mono_font())
                                    .text_size(px(11.5))
                                    .text_color(FG_MUTED)
                                    .child("labels, sections, values"),
                            ),
                    )
                    .when(count == 0, |element| {
                        element.child(
                            div()
                                .rounded(px(5.))
                                .border_1()
                                .border_dashed()
                                .border_color(BORDER)
                                .bg(INSET)
                                .px_3()
                                .py_6()
                                .text_center()
                                .child(
                                    div()
                                        .font_weight(gpui::FontWeight::MEDIUM)
                                        .text_color(FG_SECONDARY)
                                        .child("No matching settings"),
                                )
                                .child(div().mt_1().text_size(px(12.)).text_color(FG_MUTED).child(
                                    "Try a label, category, provider, shortcut, or stored value.",
                                )),
                        )
                    })
                    .when(count > 0, |element| {
                        element.child(
                            div()
                                .rounded(px(5.))
                                .border_1()
                                .border_color(BORDER)
                                .overflow_hidden()
                                .children(results.into_iter().enumerate().map(|(index, entry)| {
                                    let category = entry.category;
                                    div()
                                            .id(SharedString::from(format!(
                                                "settings-result:{}:{}",
                                                category.label(),
                                                entry.label
                                            )))
                                            .tab_index(0)
                                            .cursor_pointer()
                                            .min_h(px(42.))
                                            .flex()
                                            .items_center()
                                            .gap_3()
                                            .bg(PANEL_RAISED)
                                            .px_3()
                                            .py_2()
                                            .when(index > 0, |row| {
                                                row.border_t_1().border_color(BORDER)
                                            })
                                            .hover(|style| style.bg(INSET))
                                            .child(
                                                div()
                                                    .min_w_0()
                                                    .w(px(150.))
                                                    .flex_shrink_0()
                                                    .child(
                                                        div()
                                                            .truncate()
                                                            .font_weight(gpui::FontWeight::MEDIUM)
                                                            .child(entry.label),
                                                    )
                                                    .child(
                                                        div()
                                                            .truncate()
                                                            .text_size(px(11.5))
                                                            .text_color(FG_MUTED)
                                                            .child(entry.section),
                                                    ),
                                            )
                                            .child(
                                                div()
                                                    .min_w_0()
                                                    .flex_1()
                                                    .truncate()
                                                    .font_family(
                                                        cellar_desktop_gpui::theme::mono_font(),
                                                    )
                                                    .text_color(FG_SECONDARY)
                                                    .child(category.label()),
                                            )
                                            .child(
                                                div()
                                                    .ml_auto()
                                                    .flex_shrink_0()
                                                    .flex()
                                                    .justify_end()
                                                    .items_center()
                                                    .gap_1()
                                                    .text_size(px(11.5))
                                                    .text_color(FG_MUTED)
                                                    .child("open")
                                                    .child(
                                                        Icon::empty()
                                                            .path("icons/chevron-right.svg")
                                                            .size(px(10.)),
                                                    ),
                                            )
                                            .on_click(cx.listener(move |this, _, window, cx| {
                                                this.open_settings_search_result(
                                                    category, window, cx,
                                                );
                                            }))
                                })),
                        )
                    }),
            )
            .into_any_element()
    }

    fn open_settings_search_result(
        &mut self,
        category: SettingsCategory,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.settings_category = category;
        self.settings_search
            .update(cx, |search, cx| search.set_value("", window, cx));
        cx.notify();
    }
}

pub(super) fn search(query: &str) -> Vec<Entry> {
    let tokens = query
        .to_ascii_lowercase()
        .split_whitespace()
        .map(str::to_owned)
        .collect::<Vec<_>>();
    let mut matches = ENTRIES
        .iter()
        .filter_map(|entry| {
            let category = entry.category.label().to_ascii_lowercase();
            let section = entry.section.to_ascii_lowercase();
            let label = entry.label.to_ascii_lowercase();
            let terms = entry.terms.to_ascii_lowercase();
            let haystack = format!("{category} {section} {label} {terms}");
            tokens
                .iter()
                .all(|token| haystack.contains(token))
                .then(|| {
                    let score = tokens
                        .iter()
                        .map(|token| {
                            if category.contains(token) {
                                8
                            } else if label.contains(token) {
                                6
                            } else if section.contains(token) {
                                4
                            } else {
                                2
                            }
                        })
                        .sum::<u16>();
                    (*entry, score)
                })
        })
        .collect::<Vec<_>>();
    matches.sort_by_key(|(entry, score)| {
        (
            std::cmp::Reverse(*score),
            entry.category.label(),
            entry.label,
        )
    });
    matches.into_iter().map(|(entry, _)| entry).collect()
}

#[cfg(test)]
mod tests {
    use super::search;

    #[test]
    fn settings_search_matches_all_tokens_and_ranks_labels() {
        let results = search("provider key");
        assert_eq!(results.first().map(|entry| entry.label), Some("API key"));
        assert!(search("soft wrap")
            .iter()
            .any(|entry| entry.label == "Soft wrap"));
        assert!(search("no-such-setting").is_empty());
    }
}
