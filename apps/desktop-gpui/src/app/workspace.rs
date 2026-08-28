use gpui::{
    div, prelude::*, px, AnyElement, Axis, Context, MouseButton, MouseDownEvent, Pixels, Point,
    Render, SharedString, Window,
};
use gpui_component::resizable::{resizable_panel, ResizablePanelGroup};

use super::{shell_widgets::keycap, CellarApp};
use cellar_desktop_gpui::{
    model::{
        QueryTarget, SchemaCompareConfig, SplitOrientation, TabKind, TableTarget, WorkspaceTab,
    },
    theme::{
        accent_soft, ui_px, ACCENT, BG, BORDER, FG, FG_MUTED, FG_SECONDARY, FG_TERTIARY, INSET,
        PANEL, PANEL_RAISED,
    },
};

pub(super) enum ClosedTab {
    Table(TableTarget),
    Query(QueryTarget, String, String),
    ErDiagram(cellar_desktop_gpui::model::ErDiagramTarget),
    SchemaCompare(SchemaCompareConfig),
}

#[derive(Clone)]
struct DraggedTab {
    id: u64,
    title: SharedString,
    position: Point<Pixels>,
}

impl Render for DraggedTab {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .pl(self.position.x - ui_px(70.))
            .pt(self.position.y - ui_px(14.))
            .child(
                div()
                    .max_w(ui_px(220.))
                    .px_3()
                    .py_1()
                    .rounded(ui_px(4.))
                    .border_1()
                    .border_color(ACCENT)
                    .bg(PANEL_RAISED)
                    .text_color(FG)
                    .truncate()
                    .child(self.title.clone()),
            )
    }
}

impl CellarApp {
    pub(super) fn workspace(&self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        if self.model.tabs().is_empty() {
            return div()
                .relative()
                .flex_1()
                .h_full()
                .flex()
                .flex_col()
                .child(self.tab_bar(None, cx))
                .child(self.empty_workspace())
                .into_any_element();
        }
        div()
            .relative()
            .flex_1()
            .h_full()
            .flex()
            .flex_col()
            .child(match self.model.split() {
                Some(orientation) => self.split_workspace(orientation, window, cx),
                None => div()
                    .flex_1()
                    .min_h_0()
                    .flex()
                    .flex_col()
                    .child(self.tab_bar(None, cx))
                    .child(match self.model.active_tab() {
                        Some(tab) => self.workspace_content(tab, window, cx),
                        None => self.empty_workspace(),
                    })
                    .into_any_element(),
            })
            .when(
                cx.has_active_drag() && self.model.tabs().len() > 1,
                |element| element.child(self.split_drop_zones(cx)),
            )
            .into_any_element()
    }

    fn split_workspace(
        &self,
        orientation: SplitOrientation,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let primary = self.split_pane(0, window, cx);
        let secondary = self.split_pane(1, window, cx);
        ResizablePanelGroup::new("workspace-split")
            .axis(if orientation == SplitOrientation::Vertical {
                Axis::Horizontal
            } else {
                Axis::Vertical
            })
            .child(resizable_panel().child(primary))
            .child(resizable_panel().child(secondary))
            .into_any_element()
    }

    fn split_pane(&self, pane: u8, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let focused = self.model.focused_pane() == pane;
        div()
            .id(SharedString::from(format!("workspace-pane:{pane}")))
            .flex_1()
            .min_h_0()
            .min_w_0()
            .flex()
            .flex_col()
            .bg(if focused { BG } else { INSET })
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, _, _, cx| {
                    if this.model.focus_pane(pane) {
                        cx.notify();
                    }
                }),
            )
            .child(self.tab_bar(Some(pane), cx))
            .when_some(self.model.active_tab_in_pane(pane), |element, tab| {
                element.child(self.workspace_content(tab, window, cx))
            })
            .into_any_element()
    }

    fn tab_bar(&self, pane: Option<u8>, cx: &mut Context<Self>) -> AnyElement {
        let drop_app = cx.entity().downgrade();
        let active_id = pane
            .and_then(|pane| self.model.active_tab_in_pane(pane))
            .or_else(|| self.model.active_tab())
            .map(|tab| tab.id);
        let tabs = self
            .model
            .tabs()
            .iter()
            .filter(|tab| pane.is_none_or(|pane| self.model.tab_pane(tab.id) == pane));
        let can_query = self.model.active_connection().is_some();
        let can_split = self.model.split().is_some() || self.model.tabs().len() > 1;
        let horizontal = self.model.split() == Some(SplitOrientation::Horizontal);
        let vertical = self.model.split() == Some(SplitOrientation::Vertical);
        div()
            .h(px(cellar_desktop_gpui::theme::tab_height()))
            .flex_shrink_0()
            .flex()
            .items_end()
            .border_b_1()
            .border_color(BORDER)
            .bg(PANEL)
            .when(self.model.tabs().is_empty(), |element| {
                element.child(
                    div()
                        .h(px(cellar_desktop_gpui::theme::tab_height()))
                        .flex()
                        .items_center()
                        .px(ui_px(12.))
                        .text_size(ui_px(12.))
                        .text_color(FG_MUTED)
                        .child("no tabs — double-click a table in the sidebar"),
                )
            })
            .children(tabs.map(|tab| {
                self.tab_element(
                    tab,
                    active_id == Some(tab.id),
                    pane.is_none_or(|pane| self.model.focused_pane() == pane),
                    cx,
                )
            }))
            .child(
                div()
                    .id(SharedString::from(format!(
                        "new-query:{}",
                        pane.unwrap_or(0)
                    )))
                    .h(px(cellar_desktop_gpui::theme::tab_height()))
                    .w(ui_px(28.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_color(FG_TERTIARY)
                    .when(!can_query, |element| element.opacity(0.40))
                    .when(can_query, |element| {
                        element
                            .cursor_pointer()
                            .hover(|style| style.bg(PANEL_RAISED).text_color(FG))
                            .on_click(cx.listener(move |this, _, window, cx| {
                                if let Some(pane) = pane {
                                    this.model.focus_pane(pane);
                                }
                                this.new_query(window, cx);
                            }))
                    })
                    .child(
                        gpui_component::Icon::empty()
                            .path("icons/plus.svg")
                            .size(ui_px(12.)),
                    ),
            )
            .child(
                div()
                    .id(SharedString::from(format!(
                        "tab-strip-drop:{}",
                        pane.unwrap_or(0)
                    )))
                    .h_full()
                    .flex_1()
                    .drag_over::<DraggedTab>(|style, _, _, _| style.bg(PANEL_RAISED))
                    .on_drop(move |drag: &DraggedTab, _, cx| {
                        if let Some(pane) = pane {
                            drop_app
                                .update(cx, |this, cx| {
                                    if this.model.move_tab_to_pane(drag.id, pane) {
                                        cx.notify();
                                    }
                                })
                                .ok();
                        }
                    }),
            )
            .child(
                div()
                    .h_full()
                    .flex()
                    .items_center()
                    .gap(ui_px(1.))
                    .border_l_1()
                    .border_color(BORDER)
                    .px(ui_px(6.))
                    .child(split_button(
                        SharedString::from(format!("split-horizontal:{}", pane.unwrap_or(0))),
                        "icons/split-horizontal.svg",
                        horizontal,
                        can_split,
                        SplitOrientation::Horizontal,
                        cx,
                    ))
                    .child(split_button(
                        SharedString::from(format!("split-vertical:{}", pane.unwrap_or(0))),
                        "icons/split-vertical.svg",
                        vertical,
                        can_split,
                        SplitOrientation::Vertical,
                        cx,
                    ))
                    .child(
                        div()
                            .id(SharedString::from(format!(
                                "reopen-closed-tab:{}",
                                pane.unwrap_or(0)
                            )))
                            .size(ui_px(22.))
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded(ui_px(4.))
                            .text_color(FG_MUTED)
                            .when(self.closed_tabs.is_empty(), |element| element.opacity(0.45))
                            .when(!self.closed_tabs.is_empty(), |element| {
                                element
                                    .cursor_pointer()
                                    .hover(|style| style.bg(PANEL_RAISED).text_color(FG))
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.reopen_closed_tab(window, cx);
                                    }))
                            })
                            .child(
                                gpui_component::Icon::empty()
                                    .path("icons/history.svg")
                                    .size(ui_px(12.)),
                            ),
                    ),
            )
            .into_any_element()
    }

    fn empty_workspace(&self) -> AnyElement {
        div()
            .flex_1()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap(ui_px(14.))
            .bg(INSET)
            .p(ui_px(40.))
            .text_center()
            .child(
                gpui_component::Icon::empty()
                    .path("icons/cellar-mark.svg")
                    .size(ui_px(44.))
                    .text_color(ACCENT),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .items_center()
                    .child(
                        div()
                            .text_size(ui_px(15.))
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_color(FG)
                            .child("Open a table to begin"),
                    )
                    .child(
                        div()
                            .w(ui_px(360.))
                            .text_center()
                            .text_size(ui_px(12.5))
                            .line_height(ui_px(18.75))
                            .text_color(FG_MUTED)
                            .child("Add a Postgres connection in the sidebar, expand it, and click a table to load real rows — or hit + in the tab bar to open a SQL editor."),
                    ),
            )
            .child(
                div()
                    .flex()
                    .gap_3()
                    .text_size(ui_px(11.5))
                    .text_color(FG_MUTED)
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(ui_px(4.))
                            .child(keycap("⌘"))
                            .child(keycap("N"))
                            .child("new connection"),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(ui_px(4.))
                            .child(keycap("⌘"))
                            .child(keycap("K"))
                            .child("command palette"),
                    ),
            )
            .into_any_element()
    }

    fn tab_element(
        &self,
        tab: &WorkspaceTab,
        active: bool,
        strip_focused: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let select_id = tab.id;
        let close_id = tab.id;
        let menu_id = tab.id;
        let drag_title = tab.title.clone();
        let query_dirty = matches!(tab.kind, TabKind::Query { .. })
            && self.editors.get(&tab.id).is_some_and(|editor| {
                query_is_dirty(
                    editor.read(cx).value().as_ref(),
                    self.query_saved_sql.get(&tab.id).map(String::as_str),
                )
            });
        let drop_app = cx.entity().downgrade();
        let icon = match tab.kind {
            TabKind::Table { .. } => "icons/table.svg",
            TabKind::Query { .. } => "icons/terminal.svg",
            TabKind::ErDiagram { .. } => "icons/diagram.svg",
            TabKind::SchemaCompare { .. } => "icons/diff.svg",
        };
        div()
            .id(SharedString::from(format!("tab:{}", tab.id)))
            .tab_index(0)
            .group("workspace-tab")
            .relative()
            .cursor_pointer()
            .h(px(cellar_desktop_gpui::theme::tab_height()))
            .max_w(ui_px(260.))
            .flex_shrink_0()
            .flex()
            .items_center()
            .gap(ui_px(6.))
            .pl(ui_px(10.))
            .pr(ui_px(8.))
            .border_r_1()
            .border_color(BORDER)
            .bg(if active { BG } else { PANEL })
            .text_color(if active { FG } else { FG_SECONDARY })
            .when(active, |tab| tab.border_b_1().border_color(BG))
            .when(!active, |tab| {
                tab.hover(|style| style.bg(PANEL_RAISED).text_color(FG_SECONDARY))
            })
            .on_drag(
                DraggedTab {
                    id: tab.id,
                    title: drag_title.into(),
                    position: Point::default(),
                },
                |drag, position, _, cx| {
                    cx.refresh_windows();
                    cx.new(|_| DraggedTab {
                        position,
                        ..drag.clone()
                    })
                },
            )
            .drag_over::<DraggedTab>(|style, _, _, _| style.border_l_2().border_color(ACCENT))
            .on_drop(move |drag: &DraggedTab, _, cx| {
                drop_app
                    .update(cx, |this, cx| {
                        if this.model.reorder_tab(drag.id, select_id) {
                            cx.notify();
                        }
                    })
                    .ok();
            })
            .when(active && strip_focused, |element| {
                element.child(
                    div()
                        .absolute()
                        .left_0()
                        .top_0()
                        .bottom_0()
                        .w(ui_px(2.))
                        .bg(ACCENT),
                )
            })
            .child(
                gpui_component::Icon::empty()
                    .path(icon)
                    .size(ui_px(11.))
                    .flex_shrink_0()
                    .text_color(FG_SECONDARY),
            )
            .child(
                div()
                    .min_w_0()
                    .flex_1()
                    .truncate()
                    .font_family(cellar_desktop_gpui::theme::mono_font())
                    .child(tab.title.clone()),
            )
            .when(query_dirty, |element| {
                element.child(
                    div()
                        .size(ui_px(6.))
                        .flex_shrink_0()
                        .rounded(ui_px(3.))
                        .bg(FG_MUTED)
                        .group_hover("workspace-tab", |style| style.invisible()),
                )
            })
            .child(
                div()
                    .id(SharedString::from(format!("close-tab:{}", tab.id)))
                    .tab_index(0)
                    .ml(ui_px(2.))
                    .size(ui_px(16.))
                    .flex_shrink_0()
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(ui_px(3.))
                    .text_color(FG_MUTED)
                    .when(!active, |element| {
                        element
                            .invisible()
                            .group_hover("workspace-tab", |style| style.visible())
                    })
                    .hover(|style| style.bg(PANEL_RAISED).text_color(FG))
                    .child(
                        gpui_component::Icon::empty()
                            .path("icons/close.svg")
                            .size(ui_px(10.)),
                    )
                    .on_click(cx.listener(move |this, _, _, cx| {
                        cx.stop_propagation();
                        this.close_tab(close_id, cx);
                    })),
            )
            .on_mouse_down(
                MouseButton::Right,
                cx.listener(move |this, event: &MouseDownEvent, _, cx| {
                    cx.stop_propagation();
                    this.model.select_tab(menu_id);
                    this.tab_menu = Some(super::context_menu::TabMenu {
                        tab_id: menu_id,
                        position: event.position,
                    });
                    cx.notify();
                }),
            )
            .on_click(cx.listener(move |this, _, _, cx| {
                if this.model.select_tab(select_id) {
                    cx.notify();
                }
            }))
    }

    fn workspace_content(
        &self,
        tab: &WorkspaceTab,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        match &tab.kind {
            TabKind::Table { .. } => self.table_content(tab, window, cx),
            TabKind::Query { .. } => self.query_content(tab, window, cx),
            TabKind::ErDiagram { .. } => self.er_diagram_content(tab, cx),
            TabKind::SchemaCompare { .. } => self.schema_compare_content(tab, window, cx),
        }
    }

    fn split_drop_zones(&self, cx: &mut Context<Self>) -> AnyElement {
        let zone =
            |id: &'static str,
             orientation: SplitOrientation,
             pane: u8,
             position: fn(gpui::Stateful<gpui::Div>) -> gpui::Stateful<gpui::Div>| {
                let app = cx.entity().downgrade();
                position(
                    div()
                        .id(id)
                        .absolute()
                        .drag_over::<DraggedTab>(|style, _, _, _| {
                            style.bg(accent_soft()).border_2().border_color(ACCENT)
                        })
                        .on_drop(move |drag: &DraggedTab, _, cx| {
                            app.update(cx, |this, cx| {
                                if this.model.drop_tab_to_split(drag.id, orientation, pane) {
                                    cx.notify();
                                }
                            })
                            .ok();
                        }),
                )
            };
        div()
            .absolute()
            .inset_0()
            .child(zone(
                "split-drop-left",
                SplitOrientation::Vertical,
                0,
                |e| e.left_0().top_0().bottom_0().w_1_6(),
            ))
            .child(zone(
                "split-drop-right",
                SplitOrientation::Vertical,
                1,
                |e| e.right_0().top_0().bottom_0().w_1_6(),
            ))
            .child(zone(
                "split-drop-top",
                SplitOrientation::Horizontal,
                0,
                |e| e.left_0().right_0().top_0().h_1_6(),
            ))
            .child(zone(
                "split-drop-bottom",
                SplitOrientation::Horizontal,
                1,
                |e| e.left_0().right_0().bottom_0().h_1_6(),
            ))
            .into_any_element()
    }

    pub(crate) fn close_active_tab(&mut self, cx: &mut Context<Self>) {
        if let Some(tab_id) = self.model.active_tab().map(|tab| tab.id) {
            self.close_tab(tab_id, cx);
        }
    }

    pub(super) fn close_tab(&mut self, id: u64, cx: &mut Context<Self>) {
        if let Some(tab) = self.model.tabs().iter().find(|tab| tab.id == id) {
            if let TabKind::Table { target, .. } = &tab.kind {
                if let Some(layout) = self.grid_layout(id, cx) {
                    self.table_layouts
                        .insert(super::table_workspace::table_layout_key(target), layout);
                }
            }
            let closed = match &tab.kind {
                TabKind::Table { target, .. } => ClosedTab::Table(target.clone()),
                TabKind::Query { target, .. } => ClosedTab::Query(
                    target.clone(),
                    self.editors
                        .get(&id)
                        .map(|editor| editor.read(cx).value().to_string())
                        .unwrap_or_default(),
                    self.query_saved_sql.get(&id).cloned().unwrap_or_default(),
                ),
                TabKind::ErDiagram { target, .. } => ClosedTab::ErDiagram(target.clone()),
                TabKind::SchemaCompare { config, .. } => ClosedTab::SchemaCompare(config.clone()),
            };
            self.closed_tabs.push(closed);
            if self.closed_tabs.len() > 20 {
                self.closed_tabs.remove(0);
            }
        }
        self.cancel_query(id, cx);
        self.model.close_tab(id);
        self.grids.remove(&id);
        self.grid_layouts.remove(&id);
        self.editors.remove(&id);
        self.query_editor_subscriptions.remove(&id);
        self.query_saved_sql.remove(&id);
        self.query_params.remove(&id);
        self.query_wrap.remove(&id);
        self.table_sorts.remove(&id);
        self.table_filters.remove(&id);
        self.table_filter_operators.remove(&id);
        self.table_filter_inputs.remove(&id);
        self.table_filter_columns.remove(&id);
        self.query_summaries.remove(&id);
        self.query_generations.remove(&id);
        self.query_confirmations.remove(&id);
        self.query_plans.remove(&id);
        self.plan_loading.remove(&id);
        self.er_views.remove(&id);
        self.schema_compares.remove(&id);
        self.analyze_confirmations.remove(&id);
        if self
            .commit_review
            .as_ref()
            .is_some_and(|review| review.tab_id() == id)
        {
            self.commit_review = None;
        }
        if self
            .data_import
            .as_ref()
            .is_some_and(|import| import.tab_id() == id)
        {
            self.data_import = None;
        }
        cx.notify();
    }

    fn reopen_closed_tab(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        match self.closed_tabs.pop() {
            Some(ClosedTab::Table(target)) => self.open_table(target, window, cx),
            Some(ClosedTab::Query(target, sql, saved_sql)) => {
                let id = self.open_query(target, sql, window, cx);
                self.query_saved_sql.insert(id, saved_sql);
            }
            Some(ClosedTab::ErDiagram(target)) => {
                self.open_er_diagram(target, cx);
            }
            Some(ClosedTab::SchemaCompare(config)) => {
                self.open_schema_compare(config, window, cx);
            }
            None => {}
        }
    }
}

fn split_button(
    id: SharedString,
    path: &'static str,
    active: bool,
    enabled: bool,
    orientation: SplitOrientation,
    cx: &mut Context<CellarApp>,
) -> impl IntoElement {
    div()
        .id(id)
        .size(ui_px(22.))
        .flex()
        .items_center()
        .justify_center()
        .rounded(ui_px(4.))
        .text_color(if active { ACCENT } else { FG_MUTED })
        .when(!enabled, |element| element.opacity(0.45))
        .when(enabled, |element| {
            element
                .tab_index(0)
                .cursor_pointer()
                .hover(|style| style.bg(PANEL_RAISED).text_color(FG))
                .on_click(cx.listener(move |this, _, _, cx| {
                    if this.model.toggle_split(orientation) {
                        cx.notify();
                    }
                }))
        })
        .child(gpui_component::Icon::empty().path(path).size(ui_px(12.)))
}

fn query_is_dirty(current: &str, saved: Option<&str>) -> bool {
    current != saved.unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::query_is_dirty;

    #[test]
    fn query_dirty_state_tracks_the_successful_run_baseline() {
        assert!(!query_is_dirty("", None));
        assert!(query_is_dirty("select 1", None));
        assert!(!query_is_dirty("select 1", Some("select 1")));
    }
}
