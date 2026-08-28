use std::rc::Rc;
use std::time::{Duration, Instant};

use gpui::{
    div, prelude::*, AnyElement, Context, MouseButton, MouseDownEvent, Pixels, Point, SharedString,
    WindowControlArea,
};
use gpui_component::Icon;

use super::sql_completion::SqlCompletionProvider;
use super::{
    shell_widgets::{
        bottom_empty, engine_color, keycap, title_crumb, title_database_crumb, title_separator,
    },
    CellarApp,
};
use cellar_desktop_gpui::{
    model::TabKind,
    theme::{
        accent_soft, ui_px, ACCENT, BG, BORDER, BORDER_SEPARATOR, BORDER_STRONG, FG, FG_MUTED,
        FG_SECONDARY, FG_TERTIARY, INSET, PANEL, PANEL_MUTED, PANEL_RAISED,
    },
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) enum BottomPanelTab {
    Results,
    Messages,
    Plan,
    History,
    Notices,
    FindUsages,
}

pub(super) struct QueryDatabaseMenu {
    tab_id: u64,
    position: Point<Pixels>,
}

impl BottomPanelTab {
    pub(super) fn id(self) -> &'static str {
        match self {
            Self::Results => "results",
            Self::Messages => "messages",
            Self::Plan => "plan",
            Self::History => "history",
            Self::Notices => "notices",
            Self::FindUsages => "find-usages",
        }
    }
}

impl CellarApp {
    pub(super) fn title_bar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let active = (!self.show_empty_state)
            .then(|| self.model.active_tab())
            .flatten()
            .and_then(|tab| {
                let (connection_id, database, context, context_icon, query_tab) = match &tab.kind {
                    TabKind::Table { target, .. } => (
                        target.connection_id.as_str(),
                        target.database.as_str(),
                        target.schema.as_str(),
                        "icons/schema.svg",
                        None,
                    ),
                    TabKind::Query { target, .. } => (
                        target.connection_id.as_str(),
                        target.database.as_str(),
                        tab.title.as_str(),
                        "icons/terminal.svg",
                        Some(tab.id),
                    ),
                    TabKind::ErDiagram { target, .. } => (
                        target.connection_id.as_str(),
                        target.database.as_str(),
                        tab.title.as_str(),
                        "icons/diagram.svg",
                        None,
                    ),
                    TabKind::SchemaCompare { config, .. } => (
                        config
                            .source
                            .live_connection_id()
                            .or_else(|| config.target.live_connection_id())?,
                        config
                            .source
                            .database()
                            .or_else(|| config.target.database())
                            .unwrap_or("snapshot"),
                        tab.title.as_str(),
                        "icons/diff.svg",
                        None,
                    ),
                };
                self.model
                    .connections()
                    .iter()
                    .find(|config| config.id == connection_id)
                    .map(|config| {
                        (
                            config.name.clone(),
                            database.to_owned(),
                            context.to_owned(),
                            context_icon,
                            query_tab,
                            config.engine,
                        )
                    })
            });
        let icon_button = |id: &'static str, icon: &'static str, active| {
            div()
                .id(id)
                .tab_index(0)
                .cursor_pointer()
                .size(ui_px(22.))
                .flex()
                .items_center()
                .justify_center()
                .rounded(ui_px(4.))
                .text_color(if active { ACCENT } else { FG_TERTIARY })
                .bg(if active {
                    cellar_desktop_gpui::theme::accent_soft()
                } else {
                    cellar_desktop_gpui::theme::accent(0.)
                })
                .hover(|style| style.bg(PANEL_RAISED).text_color(FG))
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .child(
                    Icon::empty()
                        .path(icon)
                        .size(ui_px(13.))
                        .text_color(if active { ACCENT } else { FG_TERTIARY }),
                )
        };

        div()
            .id("title-bar")
            .relative()
            .h(ui_px(34.))
            .flex_shrink_0()
            .flex()
            .items_center()
            .px(ui_px(10.))
            .bg(if self.show_empty_state { BG } else { PANEL })
            .border_b_1()
            .border_color(if self.show_empty_state {
                cellar_desktop_gpui::theme::accent(0.)
            } else {
                BORDER.rgba()
            })
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, event: &MouseDownEvent, window, _| {
                    let now = Instant::now();
                    if is_titlebar_double_press(
                        this.last_titlebar_press,
                        now,
                        event.click_count,
                    ) {
                        this.last_titlebar_press = None;
                        window.zoom_window();
                    } else {
                        this.last_titlebar_press = Some(now);
                    }
                }),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .h_full()
                    .window_control_area(WindowControlArea::Drag)
                    .flex()
                    .items_center()
                    .child(div().w(ui_px(68.)).flex_shrink_0())
                    .when_some(
                        active,
                        |element, (connection, database, context, context_icon, query_tab, engine)| {
                            let database_crumb = if let Some(tab_id) = query_tab {
                                title_database_crumb(database, engine)
                                    .id("switch-query-database")
                                    .cursor_pointer()
                                    .hover(|style| style.bg(PANEL_RAISED))
                                    .child(
                                        Icon::empty()
                                            .path("icons/chevron-down.svg")
                                            .size(ui_px(10.))
                                            .text_color(FG_MUTED),
                                    )
                                    .on_mouse_down(
                                        MouseButton::Left,
                                        cx.listener(move |this, event: &MouseDownEvent, _, cx| {
                                            cx.stop_propagation();
                                            this.query_database_menu = Some(QueryDatabaseMenu {
                                                tab_id,
                                                position: Point {
                                                    x: event.position.x,
                                                    y: ui_px(38.),
                                                },
                                            });
                                            cx.notify();
                                        }),
                                    )
                                    .into_any_element()
                            } else {
                                title_database_crumb(database, engine).into_any_element()
                            };
                            element
                                .child(div().mx(ui_px(4.)).h(ui_px(16.)).w(ui_px(1.)).bg(BORDER))
                                .child(title_crumb("icons/database.svg", connection, 12.))
                                .child(title_separator())
                                .child(database_crumb)
                                .child(title_separator())
                                .child(title_crumb(context_icon, context, 11.))
                        },
                    ),
            )
            .child(
                div()
                    .flex_1()
                    .h_full()
                    .flex()
                    .items_center()
                    .gap(ui_px(6.))
                    .child(
                        div()
                            .flex_1()
                            .h_full()
                            .window_control_area(WindowControlArea::Drag),
                    )
                    .when(!self.show_empty_state, |element| {
                        element.child(
                            icon_button(
                                "toggle-left-panel",
                                "icons/panel-left.svg",
                                self.sidebar_open,
                            )
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.sidebar_open = !this.sidebar_open;
                                cx.notify();
                            })),
                        )
                    })
                    .when(!self.show_empty_state, |element| {
                        element.child(
                            icon_button(
                                "toggle-bottom-panel",
                                "icons/panel-bottom.svg",
                                self.bottom_panel_open,
                            )
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.bottom_panel_open = !this.bottom_panel_open;
                                cx.notify();
                            })),
                        )
                    })
                    .when(!self.show_empty_state, |element| {
                        element.child(
                            icon_button(
                                "toggle-right-panel",
                                "icons/panel-right.svg",
                                self.right_panel_open,
                            )
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.right_panel_open = !this.right_panel_open;
                                cx.notify();
                            })),
                        )
                    })
                    .when(!self.show_empty_state, |element| {
                        element.child(div().mx(ui_px(2.)).h(ui_px(16.)).w(ui_px(1.)).bg(BORDER))
                    })
                    .child(
                        icon_button(
                            "toggle-empty-state",
                            "icons/layout.svg",
                            self.show_empty_state,
                        )
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.show_empty_state = !this.show_empty_state;
                            cx.notify();
                        })),
                    ),
            )
            .child(
                div()
                    .id("open-command-palette")
                    .tab_index(0)
                    .absolute()
                    .top(ui_px(5.))
                    .left(gpui::relative(0.5))
                    .ml(ui_px(-160.))
                    .w(ui_px(320.))
                    .h(ui_px(24.))
                    .flex()
                    .items_center()
                    .gap_2()
                    .px_2()
                    .rounded(ui_px(5.))
                    .border_1()
                    .border_color(BORDER)
                    .bg(INSET)
                    .text_size(ui_px(14.))
                    .text_color(FG_MUTED)
                    .hover(|style| style.border_color(BORDER_STRONG))
                    .child(Icon::empty().path("icons/search.svg").size(ui_px(12.)))
                    .child(div().flex_1().child("Search tables, columns, queries…"))
                    .child(
                        div()
                            .flex()
                            .gap(ui_px(2.))
                            .child(keycap("⌘"))
                            .child(keycap("K")),
                    )
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.toggle_command_palette(window, cx);
                    })),
            )
    }

    pub(super) fn query_database_menu(&self, cx: &mut Context<Self>) -> AnyElement {
        let Some(menu) = &self.query_database_menu else {
            return div().into_any_element();
        };
        let Some((connection_id, selected)) = self.model.tabs().iter().find_map(|tab| {
            (tab.id == menu.tab_id).then(|| match &tab.kind {
                TabKind::Query { target, .. } => {
                    Some((target.connection_id.clone(), target.database.clone()))
                }
                _ => None,
            })?
        }) else {
            return div().into_any_element();
        };
        let tab_id = menu.tab_id;
        let position = menu.position;
        let engine = self
            .model
            .connections()
            .iter()
            .find(|config| config.id == connection_id)
            .map(|config| config.engine)
            .unwrap_or(cellar_core::driver::Engine::Postgres);
        let databases = self
            .model
            .databases(&connection_id)
            .iter()
            .map(|database| database.name.clone())
            .collect::<Vec<_>>();
        div()
            .id("query-database-menu-backdrop")
            .absolute()
            .inset_0()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _, _, cx| {
                    this.query_database_menu = None;
                    cx.notify();
                }),
            )
            .child(
                div()
                    .id("query-database-menu")
                    .absolute()
                    .left(position.x)
                    .top(position.y)
                    .min_w(ui_px(176.))
                    .py(ui_px(4.))
                    .rounded(ui_px(6.))
                    .border_1()
                    .border_color(BORDER_STRONG)
                    .bg(PANEL_MUTED)
                    .shadow_lg()
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .children(databases.into_iter().map(|database| {
                        let choose = database.clone();
                        let active = database == selected;
                        div()
                            .id(SharedString::from(format!("query-database:{database}")))
                            .cursor_pointer()
                            .h(ui_px(28.))
                            .flex()
                            .items_center()
                            .gap(ui_px(8.))
                            .px(ui_px(10.))
                            .text_size(ui_px(14.))
                            .text_color(FG_SECONDARY)
                            .hover(|style| style.bg(accent_soft()).text_color(ACCENT))
                            .child(
                                div()
                                    .size(ui_px(14.))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .text_size(ui_px(10.))
                                    .text_color(if active {
                                        engine_color(engine)
                                    } else {
                                        FG_MUTED.rgba()
                                    })
                                    .child(if active { "●" } else { "○" }),
                            )
                            .child(database)
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.select_query_database(tab_id, choose.clone(), cx);
                            }))
                    })),
            )
            .into_any_element()
    }

    fn select_query_database(&mut self, tab_id: u64, database: String, cx: &mut Context<Self>) {
        let connection_id = self.model.tabs().iter().find_map(|tab| match &tab.kind {
            TabKind::Query { target, .. } if tab.id == tab_id => Some(target.connection_id.clone()),
            _ => None,
        });
        self.query_database_menu = None;
        let Some(connection_id) = connection_id else {
            cx.notify();
            return;
        };
        self.cancel_query(tab_id, cx);
        if !self.model.set_query_database(tab_id, database.clone()) {
            cx.notify();
            return;
        }
        *self.query_generations.entry(tab_id).or_default() += 1;
        self.grids.remove(&tab_id);
        self.grid_layouts.remove(&tab_id);
        self.query_params.remove(&tab_id);
        self.query_summaries.remove(&tab_id);
        self.query_confirmations.remove(&tab_id);
        self.query_plans.remove(&tab_id);
        self.plan_loading.remove(&tab_id);
        if let Some(editor) = self.editors.get(&tab_id) {
            let completion = Rc::new(SqlCompletionProvider::new(
                self.model.databases(&connection_id),
                &database,
            ));
            editor.update(cx, |editor, _| {
                editor.lsp.completion_provider = Some(completion)
            });
        }
        cx.notify();
    }

    pub(super) fn bottom_panel(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let active_grid = self
            .model
            .active_tab()
            .and_then(|tab| self.grids.get(&tab.id))
            .cloned();
        div()
            .relative()
            .h(ui_px(self.bottom_panel_height))
            .flex_shrink_0()
            .flex()
            .flex_col()
            .bg(PANEL)
            .border_t_1()
            .border_color(BORDER)
            .child(
                div()
                    .absolute()
                    .left_0()
                    .right_0()
                    .top(ui_px(-3.))
                    .h(ui_px(7.))
                    .group("bottom-panel-resizer")
                    .cursor_row_resize()
                    .child(
                        div()
                            .absolute()
                            .left_0()
                            .right_0()
                            .top(ui_px(3.))
                            .h(ui_px(1.))
                            .bg(if self.bottom_panel_resize.is_some() {
                                ACCENT.rgba()
                            } else {
                                BORDER_SEPARATOR.rgba()
                            })
                            .group_hover("bottom-panel-resizer", |style| {
                                style.bg(cellar_desktop_gpui::theme::accent(0.32))
                            }),
                    )
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|this, event: &MouseDownEvent, _, cx| {
                            this.bottom_panel_resize =
                                Some((f32::from(event.position.y), this.bottom_panel_height));
                            cx.notify();
                        }),
                    ),
            )
            .child(self.bottom_panel_header(active_grid.clone(), cx))
            .child(self.bottom_panel_body(cx))
            .when(
                self.bottom_export_menu && active_grid.is_some(),
                |element| element.child(self.bottom_export_menu(active_grid.unwrap(), cx)),
            )
    }

    fn bottom_panel_body(&self, cx: &mut Context<Self>) -> AnyElement {
        let active = self.model.active_tab();
        match self.bottom_panel_tab {
            BottomPanelTab::Results => match active {
                Some(tab) if matches!(tab.kind, TabKind::Table { .. }) => {
                    let TabKind::Table { target, .. } = &tab.kind else { unreachable!() };
                    bottom_empty(
                        "Table rows are already shown",
                        format!("{}.{}.{} is a table-browsing tab. The Results grid is reserved for SQL query output — open a query tab and run a statement to use it.", target.database, target.schema, target.table),
                    )
                }
                Some(tab) => self.grids.get(&tab.id).cloned().map_or_else(
                    || match &tab.kind {
                        TabKind::Query { .. } => self.query_summaries.get(&tab.id).and_then(|summary| summary.rows_affected).map_or_else(
                            || bottom_empty("Run a query to see results", format!("{} has not produced a result set yet.", tab.title)),
                            |rows| bottom_empty(format!("Query OK — {rows} {} affected", if rows == 1 { "row" } else { "rows" }), ""),
                        ),
                        TabKind::SchemaCompare { .. } => bottom_empty("Schema comparison", format!("{} is a schema-compare tab. Its diff and generated migration are shown in the main pane above.", tab.title)),
                        TabKind::ErDiagram { .. } => bottom_empty("ER diagram", format!("{} renders the foreign-key graph above. The Results grid is reserved for SQL query output.", tab.title)),
                        TabKind::Table { .. } => unreachable!(),
                    },
                    |grid| div().flex_1().min_h_0().child(grid).into_any_element(),
                ),
                None => bottom_empty(
                    "No active tab",
                    "Open a table from the sidebar to load rows, or open a SQL editor with + in the tab bar and run a query to populate this panel.",
                ),
            },
            BottomPanelTab::Messages => self.bottom_messages_panel(cx),
            BottomPanelTab::Plan => self.query_plan_panel(cx),
            BottomPanelTab::History => self.bottom_history_panel(cx),
            BottomPanelTab::Notices => self.bottom_notices_panel(cx),
            BottomPanelTab::FindUsages => self.find_usages_panel(cx),
        }
    }

    pub(super) fn ai_panel(&self, cx: &mut Context<Self>) -> impl IntoElement {
        self.render_ai_panel(cx)
    }
}

fn is_titlebar_double_press(last: Option<Instant>, now: Instant, click_count: usize) -> bool {
    click_count >= 2
        || last.is_some_and(|last| now.duration_since(last) < Duration::from_millis(400))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_double_press_when_dragging_resets_native_click_count() {
        let now = Instant::now();
        assert!(is_titlebar_double_press(
            Some(now - Duration::from_millis(399)),
            now,
            1
        ));
        assert!(!is_titlebar_double_press(
            Some(now - Duration::from_millis(400)),
            now,
            1
        ));
        assert!(is_titlebar_double_press(None, now, 2));
    }
}
