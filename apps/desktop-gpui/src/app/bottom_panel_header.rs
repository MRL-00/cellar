use gpui::{div, prelude::*, AnyElement, Context, Entity, SharedString};
use gpui_component::Icon;

use super::{shell::BottomPanelTab, shell_widgets::disabled_icon, CellarApp};
use cellar_desktop_gpui::{
    grid::DataGrid,
    model::TabKind,
    theme::{
        ui_px, ACCENT, BORDER, BORDER_DIVIDER, FG, FG_MUTED, FG_TERTIARY, PANEL, PANEL_MUTED,
        PANEL_RAISED,
    },
};

const TABS: &[(BottomPanelTab, &str, &str)] = &[
    (BottomPanelTab::Results, "Results", "icons/table.svg"),
    (BottomPanelTab::Messages, "Messages", "icons/info.svg"),
    (BottomPanelTab::Plan, "Plan", "icons/tree.svg"),
    (BottomPanelTab::History, "History", "icons/history.svg"),
    (
        BottomPanelTab::Notices,
        "Notices",
        "icons/triangle-alert.svg",
    ),
    (
        BottomPanelTab::FindUsages,
        "Find Usages",
        "icons/search.svg",
    ),
];

impl CellarApp {
    pub(super) fn bottom_panel_header(
        &self,
        active_grid: Option<Entity<DataGrid>>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        div()
            .h(ui_px(28.))
            .flex_shrink_0()
            .flex()
            .justify_between()
            .pl(ui_px(6.))
            .pr(ui_px(4.))
            .border_b_1()
            .border_color(BORDER)
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .overflow_hidden()
                    .flex()
                    .items_center()
                    .gap(ui_px(2.))
                    .children(
                        TABS.iter()
                            .map(|(tab, label, icon)| self.bottom_tab(*tab, label, icon, cx)),
                    )
                    .child(
                        div()
                            .mx(ui_px(6.))
                            .h(ui_px(18.))
                            .w(ui_px(1.))
                            .flex_shrink_0()
                            .bg(BORDER_DIVIDER),
                    )
                    .child(self.bottom_header_meta()),
            )
            .child(
                div()
                    .flex_shrink_0()
                    .flex()
                    .items_center()
                    .gap(ui_px(2.))
                    .bg(PANEL)
                    .child(
                        div()
                            .id("export-bottom-results")
                            .size(ui_px(24.))
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded(ui_px(4.))
                            .text_color(FG_TERTIARY)
                            .when(active_grid.is_none(), |button| button.opacity(0.45))
                            .when(active_grid.is_some(), |button| {
                                button
                                    .tab_index(0)
                                    .cursor_pointer()
                                    .hover(|style| style.bg(PANEL_RAISED).text_color(FG))
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.bottom_export_menu = !this.bottom_export_menu;
                                        cx.notify();
                                    }))
                            })
                            .child(Icon::empty().path("icons/file-text.svg").size(ui_px(15.))),
                    )
                    .child(disabled_icon("icons/expand.svg", 15.))
                    .child(
                        div()
                            .id("hide-bottom-panel")
                            .tab_index(0)
                            .cursor_pointer()
                            .size(ui_px(24.))
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded(ui_px(4.))
                            .text_color(FG_TERTIARY)
                            .hover(|style| style.bg(PANEL_RAISED).text_color(FG))
                            .child(
                                Icon::empty()
                                    .path("icons/chevrons-down.svg")
                                    .size(ui_px(15.)),
                            )
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.bottom_panel_open = false;
                                cx.notify();
                            })),
                    ),
            )
            .into_any_element()
    }

    fn bottom_tab(
        &self,
        tab: BottomPanelTab,
        label: &'static str,
        icon: &'static str,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let active = self.bottom_panel_tab == tab;
        div()
            .id(SharedString::from(format!("bottom-tab:{}", tab.id())))
            .tab_index(0)
            .cursor_pointer()
            .mt(ui_px(3.))
            .h(ui_px(22.))
            .flex_shrink()
            .flex()
            .items_center()
            .gap(ui_px(6.))
            .px(ui_px(8.))
            .rounded(ui_px(4.))
            .text_color(if active { FG } else { FG_TERTIARY })
            .bg(if active { PANEL_RAISED } else { PANEL })
            .hover(|style| style.bg(PANEL_MUTED).text_color(FG))
            .child(
                Icon::empty()
                    .path(icon)
                    .size(ui_px(11.))
                    .text_color(if active { ACCENT } else { FG_MUTED }),
            )
            .child(label)
            .when_some(self.bottom_tab_count(tab), |button, count| {
                button.child(
                    div()
                        .rounded(ui_px(8.))
                        .bg(if active { PANEL } else { PANEL_MUTED })
                        .px_1()
                        .font_family(cellar_desktop_gpui::theme::mono_font())
                        .text_size(ui_px(10.5))
                        .text_color(if active { FG } else { FG_TERTIARY })
                        .child(count.to_string()),
                )
            })
            .on_click(cx.listener(move |this, _, _, cx| {
                this.bottom_panel_tab = tab;
                cx.notify();
            }))
            .into_any_element()
    }

    fn bottom_tab_count(&self, tab: BottomPanelTab) -> Option<u64> {
        let active = self.model.active_tab();
        match tab {
            BottomPanelTab::Results => active
                .and_then(|tab| self.query_summaries.get(&tab.id))
                .map(|summary| summary.row_count),
            BottomPanelTab::Messages => active
                .and_then(|tab| self.query_summaries.get(&tab.id))
                .map(|summary| 1 + summary.notices.len() as u64),
            BottomPanelTab::Plan => None,
            BottomPanelTab::History => Some(self.history_records.len() as u64),
            BottomPanelTab::Notices => Some(
                active
                    .and_then(|tab| self.query_summaries.get(&tab.id))
                    .map_or(0, |summary| summary.notices.len() as u64),
            ),
            BottomPanelTab::FindUsages => self.find_usages_count().map(|count| count as u64),
        }
    }

    fn bottom_header_meta(&self) -> AnyElement {
        let items = match self.model.active_tab() {
            None => vec!["no active tab".to_owned()],
            Some(tab) => match &tab.kind {
                TabKind::Query { .. } => vec![tab.title.clone(), "query tab".into()],
                TabKind::SchemaCompare { .. } => {
                    vec![tab.title.clone(), "schema compare tab".into()]
                }
                TabKind::ErDiagram { .. } => vec![tab.title.clone(), "ER diagram".into()],
                TabKind::Table { target, .. } => vec![
                    format!("{}.{}.{}", target.database, target.schema, target.table),
                    "table rows shown above".into(),
                ],
            },
        };
        div()
            .min_w_0()
            .flex()
            .items_center()
            .gap(ui_px(6.))
            .overflow_hidden()
            .font_family(cellar_desktop_gpui::theme::mono_font())
            .text_size(ui_px(10.5))
            .children(items.into_iter().enumerate().map(|(index, item)| {
                div()
                    .min_w_0()
                    .flex()
                    .gap_1()
                    .text_color(if index == 0 { FG_TERTIARY } else { FG_MUTED })
                    .when(index > 0, |item| {
                        item.child(div().text_color(FG_MUTED).child("·"))
                    })
                    .child(div().truncate().child(item))
            }))
            .into_any_element()
    }
}
