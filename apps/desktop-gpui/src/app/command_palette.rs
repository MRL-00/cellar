use gpui::{div, prelude::*, px, AnyElement, Context, KeyDownEvent, SharedString, Window};
use gpui_component::{input::InputState, Icon};

use super::{shell_widgets::keycap, CellarApp};
use cellar_desktop_gpui::{
    model::TableTarget,
    theme::{ACCENT, BORDER, FG, FG_MUTED, FG_SECONDARY, FG_TERTIARY, PANEL, PANEL_MUTED, WARN},
    widgets::compact_input,
};

use super::shell::BottomPanelTab;

#[derive(Clone)]
enum PaletteAction {
    NewConnection,
    NewQuery,
    ReviewPending,
    RevertPending,
    ExportSetup,
    ImportSetup,
    Settings,
    ToggleLeft,
    ToggleBottom,
    ToggleRight,
    ActivateTab(u64),
    ToggleConnection(String),
    RefreshConnection(String),
    OpenTemplate(String),
    OpenTable(TableTarget),
    ShowBottom(BottomPanelTab),
    CompareSchemas,
}

struct PaletteEntry {
    group: String,
    label: String,
    hint: String,
    search: String,
    kbd: &'static [&'static str],
    action: PaletteAction,
}

impl PaletteEntry {
    fn new(
        group: &str,
        label: impl Into<String>,
        hint: impl Into<String>,
        action: PaletteAction,
    ) -> Self {
        let label = label.into();
        let hint = hint.into();
        Self {
            group: group.into(),
            search: format!("{label} {hint}").to_ascii_lowercase(),
            label,
            hint,
            kbd: &[],
            action,
        }
    }

    fn with_kbd(mut self, kbd: &'static [&'static str]) -> Self {
        self.kbd = kbd;
        self
    }

    fn with_search(mut self, search: impl AsRef<str>) -> Self {
        self.search.push(' ');
        self.search.push_str(&search.as_ref().to_ascii_lowercase());
        self
    }
}

impl CellarApp {
    pub(super) fn toggle_command_palette(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.command_palette.take().is_some() {
            self.command_palette_subscription = None;
            cx.notify();
            return;
        }
        let input = cx
            .new(|cx| InputState::new(window, cx).placeholder("Search tables, columns, commands…"));
        input.update(cx, |state, cx| state.focus(window, cx));
        self.command_palette_active = 0;
        self.command_palette_subscription = Some(cx.observe(&input, |this, _, cx| {
            this.command_palette_active = 0;
            cx.notify();
        }));
        self.command_palette = Some(input);
        self.refresh_query_templates(cx);
        cx.notify();
    }

    fn run_palette_action(
        &mut self,
        action: PaletteAction,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.command_palette = None;
        self.command_palette_subscription = None;
        self.command_palette_active = 0;
        match action {
            PaletteAction::NewConnection => self.open_connection_editor(None, window, cx),
            PaletteAction::NewQuery => self.new_query(window, cx),
            PaletteAction::ReviewPending => {
                if let Some(grid) = self
                    .model
                    .active_tab()
                    .and_then(|tab| self.grids.get(&tab.id))
                    .cloned()
                {
                    grid.update(cx, |grid, cx| grid.request_review(cx));
                }
            }
            PaletteAction::RevertPending => {
                if let Some(grid) = self
                    .model
                    .active_tab()
                    .and_then(|tab| self.grids.get(&tab.id))
                    .cloned()
                {
                    grid.update(cx, |grid, cx| grid.clear_pending(cx));
                }
            }
            PaletteAction::ExportSetup => self.open_export_setup(cx),
            PaletteAction::ImportSetup => self.open_import_setup(window, cx),
            PaletteAction::Settings => {
                self.open_settings(super::settings::SettingsCategory::Appearance, cx)
            }
            PaletteAction::ToggleLeft => {
                self.sidebar_open = !self.sidebar_open;
                cx.notify();
            }
            PaletteAction::ToggleBottom => {
                self.bottom_panel_open = !self.bottom_panel_open;
                cx.notify();
            }
            PaletteAction::ToggleRight => {
                self.right_panel_open = !self.right_panel_open;
                cx.notify();
            }
            PaletteAction::ActivateTab(id) => {
                self.model.select_tab(id);
                cx.notify();
            }
            PaletteAction::ToggleConnection(id) => {
                if matches!(
                    self.model.connection_state(&id),
                    cellar_desktop_gpui::model::ConnectionState::Connected
                ) {
                    self.disconnect(id, cx);
                } else {
                    self.model.select_connection(&id);
                    self.start_connect(id, cx);
                }
            }
            PaletteAction::RefreshConnection(id) => self.refresh_schema(id, cx),
            PaletteAction::OpenTemplate(sql) => {
                if let Some(config) = self.model.active_connection() {
                    self.open_query(
                        cellar_desktop_gpui::model::QueryTarget {
                            connection_id: config.id.clone(),
                            database: config.database.clone(),
                        },
                        sql,
                        window,
                        cx,
                    );
                }
            }
            PaletteAction::OpenTable(target) => self.open_table(target, window, cx),
            PaletteAction::ShowBottom(tab) => {
                self.bottom_panel_tab = tab;
                self.bottom_panel_open = true;
                if tab == BottomPanelTab::History {
                    self.refresh_history(cx);
                }
                cx.notify();
            }
            PaletteAction::CompareSchemas => self.open_schema_compare_dialog(None, cx),
        }
    }

    pub(super) fn handle_command_palette_key(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        match event.keystroke.key.as_str() {
            "down" => {
                self.command_palette_active = step_palette_index(
                    self.command_palette_active,
                    self.palette_entries(cx).len(),
                    1,
                );
                cx.notify();
                true
            }
            "up" => {
                self.command_palette_active = step_palette_index(
                    self.command_palette_active,
                    self.palette_entries(cx).len(),
                    -1,
                );
                cx.notify();
                true
            }
            "enter" => {
                if let Some(entry) = self.palette_entries(cx).get(self.command_palette_active) {
                    self.run_palette_action(entry.action.clone(), window, cx);
                }
                true
            }
            "escape" => {
                self.command_palette = None;
                self.command_palette_subscription = None;
                self.command_palette_active = 0;
                cx.notify();
                true
            }
            _ => false,
        }
    }

    fn palette_entries(&self, cx: &Context<Self>) -> Vec<PaletteEntry> {
        let Some(input) = &self.command_palette else {
            return Vec::new();
        };
        let query = input.read(cx).value().trim().to_lowercase();
        let pending = self
            .grids
            .values()
            .map(|grid| grid.read(cx).pending_count())
            .sum::<usize>();
        let mut entries = vec![PaletteEntry::new(
            "Actions",
            "New connection",
            "create a saved database connection",
            PaletteAction::NewConnection,
        )
        .with_kbd(&["⌘", "N"])];
        if !self.model.connections().is_empty() {
            entries.push(PaletteEntry::new(
                "Actions",
                "New SQL query",
                "open an editor for the active connection",
                PaletteAction::NewQuery,
            ));
        }
        entries.extend([
            PaletteEntry::new(
                "Actions",
                "Review pending changes",
                if pending == 0 {
                    "no pending changes".into()
                } else {
                    format!("{pending} pending")
                },
                PaletteAction::ReviewPending,
            )
            .with_kbd(&["⌘", "S"]),
            PaletteEntry::new(
                "Actions",
                "Compare schemas…",
                "diff two schemas and generate migration DDL",
                PaletteAction::CompareSchemas,
            )
            .with_search("schema diff migration ddl snapshot compare"),
            PaletteEntry::new(
                "Actions",
                "Export setup…",
                "share connections, settings, layouts",
                PaletteAction::ExportSetup,
            )
            .with_search("backup transfer share download"),
            PaletteEntry::new(
                "Actions",
                "Import setup…",
                "load a shared setup file",
                PaletteAction::ImportSetup,
            )
            .with_search("restore transfer upload merge"),
        ]);
        if pending > 0 {
            entries.push(PaletteEntry::new(
                "Actions",
                "Revert active table changes",
                "discard pending edits on the active table",
                PaletteAction::RevertPending,
            ));
        }
        if !self.model.connections().is_empty() {
            entries.extend(self.query_templates.iter().map(|template| {
                PaletteEntry::new(
                    "Templates",
                    template.name.clone(),
                    if template.description.is_empty() {
                        "open in a new query tab".into()
                    } else {
                        template.description.clone()
                    },
                    PaletteAction::OpenTemplate(template.sql.clone()),
                )
                .with_search(format!("{} saved query", template.sql))
            }));
        }
        entries.extend(self.model.tabs().iter().map(|tab| {
            let hint = match &tab.kind {
                cellar_desktop_gpui::model::TabKind::Query { .. } => "query tab".into(),
                cellar_desktop_gpui::model::TabKind::Table { target, .. } => {
                    target.database.clone()
                }
                _ => "workspace tab".into(),
            };
            PaletteEntry::new(
                "Tabs",
                tab.title.clone(),
                hint,
                PaletteAction::ActivateTab(tab.id),
            )
        }));
        for config in self.model.connections() {
            let state = self.model.connection_state(&config.id);
            let connected = matches!(
                state,
                cellar_desktop_gpui::model::ConnectionState::Connected
            );
            let state_label = match state {
                cellar_desktop_gpui::model::ConnectionState::Connected => "connected",
                cellar_desktop_gpui::model::ConnectionState::Connecting => "connecting",
                cellar_desktop_gpui::model::ConnectionState::Disconnecting => "disconnecting",
                cellar_desktop_gpui::model::ConnectionState::Error(_) => "error",
                cellar_desktop_gpui::model::ConnectionState::Disconnected => "disconnected",
            };
            entries.push(
                PaletteEntry::new(
                    "Connections",
                    config.name.clone(),
                    state_label,
                    PaletteAction::ToggleConnection(config.id.clone()),
                )
                .with_search(format!(
                    "{:?} {} {}",
                    config.engine, config.host, config.database
                )),
            );
            if connected {
                entries.push(PaletteEntry::new(
                    "Connections",
                    format!("Refresh {} schema", config.name),
                    "introspect catalog",
                    PaletteAction::RefreshConnection(config.id.clone()),
                ));
            }
        }
        for config in self.model.connections() {
            for database in self.model.databases(&config.id) {
                for schema in &database.schemas {
                    for table in &schema.tables {
                        let relation = format!("{}.{}", schema.name, table.name);
                        let target = TableTarget {
                            connection_id: config.id.clone(),
                            database: database.name.clone(),
                            schema: schema.name.clone(),
                            table: table.name.clone(),
                        };
                        entries.push(
                            PaletteEntry::new(
                                "Catalog",
                                relation.clone(),
                                format!("{} · {}", config.name, database.name),
                                PaletteAction::OpenTable(target.clone()),
                            )
                            .with_search("table"),
                        );
                        if !query.is_empty() {
                            entries.extend(table.columns.iter().map(|column| {
                                PaletteEntry::new(
                                    "Columns",
                                    format!("{relation}.{}", column.name),
                                    column.data_type.clone(),
                                    PaletteAction::OpenTable(target.clone()),
                                )
                                .with_search(format!("{} {} column", config.name, database.name))
                            }));
                        }
                    }
                    entries.extend(schema.views.iter().map(|view| {
                        PaletteEntry::new(
                            "Catalog",
                            format!("{}.{}", schema.name, view.name),
                            format!("{} · view", config.name),
                            PaletteAction::OpenTable(TableTarget {
                                connection_id: config.id.clone(),
                                database: database.name.clone(),
                                schema: schema.name.clone(),
                                table: view.name.clone(),
                            }),
                        )
                        .with_search(format!("{} {} view", config.name, database.name))
                    }));
                }
            }
        }
        entries.extend([
            PaletteEntry::new(
                "View",
                if self.sidebar_open {
                    "Hide Connections panel"
                } else {
                    "Show Connections panel"
                },
                if self.sidebar_open {
                    "visible"
                } else {
                    "hidden"
                },
                PaletteAction::ToggleLeft,
            ),
            PaletteEntry::new(
                "View",
                if self.bottom_panel_open {
                    "Hide Output panel"
                } else {
                    "Show Output panel"
                },
                if self.bottom_panel_open {
                    "visible"
                } else {
                    "hidden"
                },
                PaletteAction::ToggleBottom,
            ),
            PaletteEntry::new(
                "View",
                if self.right_panel_open {
                    "Hide AI panel"
                } else {
                    "Show AI panel"
                },
                if self.right_panel_open {
                    "visible"
                } else {
                    "hidden"
                },
                PaletteAction::ToggleRight,
            ),
        ]);
        for (tab, label) in [
            (BottomPanelTab::Results, "Results"),
            (BottomPanelTab::Messages, "Messages"),
            (BottomPanelTab::Plan, "Plan"),
            (BottomPanelTab::History, "History"),
            (BottomPanelTab::Notices, "Notices"),
        ] {
            entries.push(PaletteEntry::new(
                "View",
                format!("Show {label}"),
                if self.bottom_panel_tab == tab {
                    "active"
                } else {
                    "output panel"
                },
                PaletteAction::ShowBottom(tab),
            ));
        }
        entries.push(
            PaletteEntry::new("View", "Open settings", "", PaletteAction::Settings)
                .with_kbd(&["⌘", ","]),
        );
        entries.sort_by_key(|entry| group_rank(&entry.group));
        entries
            .into_iter()
            .filter(|entry| query.is_empty() || entry.search.contains(&query))
            .take(if query.is_empty() { 80 } else { 120 })
            .collect()
    }

    pub(super) fn command_palette_overlay(&self, cx: &mut Context<Self>) -> AnyElement {
        let input = self
            .command_palette
            .as_ref()
            .expect("command palette overlay requires input");
        let visible = self.palette_entries(cx);
        let count = visible.len();
        let query = input.read(cx).value().to_string();
        let mut previous_group = String::new();
        let mut results = Vec::new();
        for (index, entry) in visible.into_iter().enumerate() {
            if entry.group != previous_group {
                previous_group = entry.group.clone();
                results.push(
                    div()
                        .pt(px(6.))
                        .pb_1()
                        .px(px(14.))
                        .text_size(px(11.))
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_color(FG_MUTED)
                        .child(entry.group.to_ascii_uppercase())
                        .into_any_element(),
                );
            }
            let selected = index == self.command_palette_active;
            let action = entry.action.clone();
            results.push(
                div()
                    .id(SharedString::from(format!(
                        "palette:{index}:{}",
                        entry.label
                    )))
                    .cursor_pointer()
                    .min_h(px(32.))
                    .flex()
                    .items_center()
                    .gap(px(10.))
                    .px(px(14.))
                    .py(px(6.))
                    .bg(if selected {
                        cellar_desktop_gpui::theme::accent_soft()
                    } else {
                        PANEL.rgba()
                    })
                    .text_color(if selected { ACCENT } else { FG_SECONDARY })
                    .hover(|style| style.bg(PANEL_MUTED).text_color(FG))
                    .child(
                        div()
                            .w(px(18.))
                            .flex_shrink_0()
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(group_icon(&entry.group)),
                    )
                    .child(
                        div()
                            .min_w_0()
                            .flex_shrink()
                            .truncate()
                            .font_weight(gpui::FontWeight::MEDIUM)
                            .child(entry.label),
                    )
                    .when(!entry.hint.is_empty(), |row| {
                        row.child(
                            div()
                                .ml_auto()
                                .min_w_0()
                                .truncate()
                                .pr(px(6.))
                                .text_size(px(12.))
                                .text_color(FG_MUTED)
                                .child(entry.hint),
                        )
                    })
                    .when(!entry.kbd.is_empty(), |row| {
                        row.child(
                            div()
                                .flex_shrink_0()
                                .flex()
                                .gap(px(2.))
                                .children(entry.kbd.iter().map(|key| keycap(key))),
                        )
                    })
                    .on_mouse_move(cx.listener(move |this, _, _, cx| {
                        if this.command_palette_active != index {
                            this.command_palette_active = index;
                            cx.notify();
                        }
                    }))
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.run_palette_action(action.clone(), window, cx);
                    }))
                    .into_any_element(),
            );
        }

        div()
            .id("command-palette-backdrop")
            .absolute()
            .inset_0()
            .bg(cellar_desktop_gpui::theme::overlay())
            .flex()
            .items_start()
            .justify_center()
            .pt(gpui::relative(0.14))
            .on_click(cx.listener(|this, _, _, cx| {
                this.command_palette = None;
                this.command_palette_subscription = None;
                cx.notify();
            }))
            .child(
                div()
                    .id("command-palette")
                    .w(px(580.))
                    .flex()
                    .flex_col()
                    .rounded(px(8.))
                    .border_1()
                    .border_color(BORDER)
                    .bg(PANEL)
                    .shadow_lg()
                    .on_click(|_, _, cx| cx.stop_propagation())
                    .child(
                        div()
                            .h(px(38.))
                            .flex_shrink_0()
                            .flex()
                            .items_center()
                            .gap(px(9.))
                            .px_3()
                            .border_b_1()
                            .border_color(BORDER)
                            .child(
                                Icon::empty()
                                    .path("icons/search.svg")
                                    .size(px(13.))
                                    .text_color(FG_MUTED),
                            )
                            .child(compact_input(input).flex_1())
                            .child(keycap("esc")),
                    )
                    .child(
                        div()
                            .id("command-palette-results")
                            .max_h(px(420.))
                            .overflow_y_scroll()
                            .pt_1()
                            .pb_2()
                            .when(count == 0, |element| {
                                element.child(
                                    div()
                                        .px(px(14.))
                                        .py_5()
                                        .text_center()
                                        .text_color(FG_MUTED)
                                        .child(format!("No matches for “{query}”")),
                                )
                            })
                            .children(results),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_3()
                            .px_3()
                            .py(px(6.))
                            .border_t_1()
                            .border_color(BORDER)
                            .bg(PANEL_MUTED)
                            .text_size(px(11.5))
                            .text_color(FG_MUTED)
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_1()
                                    .child(keycap("↑↓"))
                                    .child("navigate"),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_1()
                                    .child(keycap("⏎"))
                                    .child("select"),
                            )
                            .child(div().flex_1())
                            .child(format!("{count} matches")),
                    ),
            )
            .into_any_element()
    }
}

fn group_rank(group: &str) -> usize {
    match group {
        "Actions" => 0,
        "Templates" => 1,
        "Tabs" => 2,
        "Connections" => 3,
        "Catalog" => 4,
        "Columns" => 5,
        "View" => 6,
        _ => 7,
    }
}

fn group_icon(group: &str) -> AnyElement {
    let (path, color) = match group {
        "Actions" => ("icons/bolt.svg", WARN.rgba()),
        "Templates" => ("icons/star.svg", FG_TERTIARY.rgba()),
        "Catalog" => ("icons/table.svg", FG_TERTIARY.rgba()),
        "Columns" => ("icons/bracket.svg", FG_TERTIARY.rgba()),
        "Connections" => ("icons/database.svg", FG_TERTIARY.rgba()),
        "Tabs" => ("icons/terminal.svg", FG_TERTIARY.rgba()),
        _ => ("icons/layout.svg", FG_TERTIARY.rgba()),
    };
    Icon::empty()
        .path(path)
        .size(px(11.))
        .text_color(color)
        .into_any_element()
}

fn step_palette_index(current: usize, len: usize, delta: isize) -> usize {
    current
        .saturating_add_signed(delta)
        .min(len.saturating_sub(1))
}

#[cfg(test)]
mod tests {
    use super::{group_rank, step_palette_index};

    #[test]
    fn palette_selection_stays_inside_visible_results() {
        assert_eq!(step_palette_index(0, 3, -1), 0);
        assert_eq!(step_palette_index(1, 3, 1), 2);
        assert_eq!(step_palette_index(2, 3, 1), 2);
        assert_eq!(step_palette_index(0, 0, 1), 0);
        assert!(group_rank("Actions") < group_rank("Tabs"));
        assert!(group_rank("Catalog") < group_rank("View"));
    }
}
