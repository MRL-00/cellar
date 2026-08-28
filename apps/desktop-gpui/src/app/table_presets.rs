use cellar_core::query::{TableFilterClause, TableSortClause};
use gpui::{
    div, prelude::*, px, AnyElement, Bounds, Context, Entity, MouseButton, Pixels, Point,
    SharedString, Window,
};
use gpui_component::{
    input::{InputEvent, InputState},
    Icon,
};
use serde::{Deserialize, Serialize};

use super::CellarApp;
use cellar_desktop_gpui::{
    model::{TabKind, TableTarget},
    theme::{ACCENT, BORDER, FG, FG_MUTED, PANEL, PANEL_RAISED},
};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub(super) struct FilterPreset {
    pub(super) name: String,
    pub(super) filters: Vec<TableFilterClause>,
    pub(super) sort: Option<TableSortClause>,
    #[serde(default)]
    pub(super) quick_filter: String,
    #[serde(default)]
    pub(super) quick_column: Option<usize>,
}

pub(super) struct PresetMenu {
    pub(super) tab_id: u64,
    pub(super) position: Point<Pixels>,
}

pub(super) struct PresetDraft {
    pub(super) tab_id: u64,
    pub(super) input: Entity<InputState>,
}

impl CellarApp {
    pub(super) fn active_filter_preset(&self, tab_id: u64) -> Option<String> {
        let target = table_target(self, tab_id)?;
        self.table_filter_presets
            .get(&table_key(target))?
            .iter()
            .find(|preset| {
                preset.filters == self.table_filters.get(&tab_id).cloned().unwrap_or_default()
                    && preset.sort == self.table_sorts.get(&tab_id).cloned()
                    && preset.quick_filter
                        == self
                            .table_quick_filters
                            .get(&tab_id)
                            .cloned()
                            .unwrap_or_default()
                    && preset.quick_column == self.table_quick_filter_columns.get(&tab_id).copied()
            })
            .map(|preset| preset.name.clone())
    }

    pub(super) fn open_filter_preset_menu(&mut self, tab_id: u64, cx: &mut Context<Self>) {
        let Some(position) = self
            .preset_trigger_bounds
            .get(&tab_id)
            .copied()
            .map(dropdown_below)
        else {
            return;
        };
        if self
            .table_preset_menu
            .as_ref()
            .is_some_and(|menu| menu.tab_id == tab_id)
        {
            self.table_preset_menu = None;
            cx.notify();
            return;
        }
        self.table_quick_column_menu = None;
        self.table_preset_menu = Some(PresetMenu { tab_id, position });
        cx.notify();
    }

    pub(super) fn start_filter_preset_draft(
        &mut self,
        tab_id: u64,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let input = cx.new(|cx| InputState::new(window, cx).placeholder("preset name"));
        input.update(cx, |input, cx| input.focus(window, cx));
        self.table_preset_subscription = Some(cx.subscribe(&input, |this, _, event, cx| {
            if matches!(event, InputEvent::PressEnter { .. }) {
                this.commit_filter_preset(cx);
            }
        }));
        self.table_preset_draft = Some(PresetDraft { tab_id, input });
        self.table_preset_menu = None;
        cx.notify();
    }

    pub(super) fn cancel_filter_preset_draft(&mut self, cx: &mut Context<Self>) {
        self.table_preset_draft = None;
        self.table_preset_subscription = None;
        cx.notify();
    }

    pub(super) fn commit_filter_preset(&mut self, cx: &mut Context<Self>) {
        let Some(draft) = self.table_preset_draft.take() else {
            return;
        };
        let name = draft.input.read(cx).value().trim().to_owned();
        if name.is_empty() {
            self.table_preset_draft = Some(draft);
            return;
        }
        let Some(target) = table_target(self, draft.tab_id).cloned() else {
            return;
        };
        let preset = FilterPreset {
            name: name.clone(),
            filters: self
                .table_filters
                .get(&draft.tab_id)
                .cloned()
                .unwrap_or_default(),
            sort: self.table_sorts.get(&draft.tab_id).cloned(),
            quick_filter: self
                .table_quick_filters
                .get(&draft.tab_id)
                .cloned()
                .unwrap_or_default(),
            quick_column: self.table_quick_filter_columns.get(&draft.tab_id).copied(),
        };
        let presets = self
            .table_filter_presets
            .entry(table_key(&target))
            .or_default();
        if let Some(existing) = presets.iter_mut().find(|preset| preset.name == name) {
            *existing = preset;
        } else {
            presets.push(preset);
        }
        self.table_preset_subscription = None;
        cx.notify();
    }

    fn apply_filter_preset(
        &mut self,
        tab_id: u64,
        name: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(target) = table_target(self, tab_id) else {
            return;
        };
        let Some(preset) = self
            .table_filter_presets
            .get(&table_key(target))
            .and_then(|presets| presets.iter().find(|preset| preset.name == name))
            .cloned()
        else {
            return;
        };
        if preset.filters.is_empty() {
            self.table_filters.remove(&tab_id);
        } else {
            self.table_filters.insert(tab_id, preset.filters);
        }
        if let Some(sort) = preset.sort {
            self.table_sorts.insert(tab_id, sort);
        } else {
            self.table_sorts.remove(&tab_id);
        }
        if preset.quick_filter.is_empty() {
            self.table_quick_filters.remove(&tab_id);
        } else {
            self.table_quick_filters
                .insert(tab_id, preset.quick_filter.clone());
        }
        if let Some(column) = preset.quick_column {
            self.table_quick_filter_columns.insert(tab_id, column);
        }
        if let Some(input) = self.table_quick_filter_inputs.get(&tab_id) {
            input.update(cx, |input, cx| {
                input.set_value(preset.quick_filter, window, cx)
            });
        }
        self.table_preset_menu = None;
        self.restart_table(tab_id, cx);
    }

    fn clear_filter_toolbar(&mut self, tab_id: u64, window: &mut Window, cx: &mut Context<Self>) {
        self.table_filters.remove(&tab_id);
        self.table_sorts.remove(&tab_id);
        self.table_quick_filters.remove(&tab_id);
        if let Some(input) = self.table_quick_filter_inputs.get(&tab_id) {
            input.update(cx, |input, cx| input.set_value("", window, cx));
        }
        self.table_preset_menu = None;
        self.restart_table(tab_id, cx);
    }

    fn delete_filter_preset(&mut self, tab_id: u64, name: &str, cx: &mut Context<Self>) {
        let Some(target) = table_target(self, tab_id) else {
            return;
        };
        let key = table_key(target);
        if let Some(presets) = self.table_filter_presets.get_mut(&key) {
            presets.retain(|preset| preset.name != name);
            if presets.is_empty() {
                self.table_filter_presets.remove(&key);
            }
        }
        cx.notify();
    }

    pub(super) fn table_preset_menu_overlay(&self, cx: &mut Context<Self>) -> AnyElement {
        let state = self
            .table_preset_menu
            .as_ref()
            .expect("preset menu requires state");
        let tab_id = state.tab_id;
        let position = state.position;
        let active = self.active_filter_preset(tab_id);
        let presets = table_target(self, tab_id)
            .and_then(|target| self.table_filter_presets.get(&table_key(target)))
            .cloned()
            .unwrap_or_default();
        let mut menu = overlay_at("filter-preset-menu", position);
        for preset in presets {
            let name = preset.name.clone();
            let apply_name = name.clone();
            let delete_name = name.clone();
            let selected = active.as_deref() == Some(name.as_str());
            menu = menu.child(
                div()
                    .h(px(28.))
                    .flex()
                    .items_center()
                    .rounded(px(4.))
                    .hover(|style| style.bg(PANEL_RAISED))
                    .child(
                        div()
                            .id(SharedString::from(format!("preset-apply:{name}")))
                            .tab_index(0)
                            .flex_1()
                            .min_w_0()
                            .h_full()
                            .flex()
                            .items_center()
                            .gap(px(7.))
                            .px(px(6.))
                            .text_color(if selected { ACCENT } else { FG })
                            .child(div().w(px(12.)).flex().justify_center().when(
                                selected,
                                |element| {
                                    element.child(
                                        Icon::empty().path("icons/grid-check.svg").size(px(10.)),
                                    )
                                },
                            ))
                            .child(div().flex_1().truncate().child(name))
                            .on_click(cx.listener(move |this, _, window, cx| {
                                if selected {
                                    this.clear_filter_toolbar(tab_id, window, cx);
                                } else {
                                    this.apply_filter_preset(tab_id, &apply_name, window, cx);
                                }
                            })),
                    )
                    .when(!selected, |element| {
                        element.child(
                            div()
                                .id(SharedString::from(format!("preset-delete:{delete_name}")))
                                .tab_index(0)
                                .size(px(14.))
                                .mr(px(4.))
                                .flex()
                                .items_center()
                                .justify_center()
                                .child(Icon::empty().path("icons/close.svg").size(px(9.)))
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.delete_filter_preset(tab_id, &delete_name, cx);
                                })),
                        )
                    }),
            );
        }
        let save = tab_id;
        let clear = tab_id;
        div()
            .id("filter-preset-backdrop")
            .absolute()
            .inset_0()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _, _, cx| {
                    this.table_preset_menu = None;
                    cx.notify();
                }),
            )
            .child(
                menu.child(separator())
                    .child(
                        menu_action(
                            "preset-save",
                            "icons/bookmark.svg",
                            "Save current as preset…",
                        )
                        .on_click(cx.listener(
                            move |this, _, window, cx| {
                                this.start_filter_preset_draft(save, window, cx)
                            },
                        )),
                    )
                    .child(
                        menu_action("preset-clear", "icons/close.svg", "Clear current filters")
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.clear_filter_toolbar(clear, window, cx)
                            })),
                    ),
            )
            .into_any_element()
    }
}

fn table_target(app: &CellarApp, tab_id: u64) -> Option<&TableTarget> {
    app.model.tabs().iter().find_map(|tab| match &tab.kind {
        TabKind::Table { target, .. } if tab.id == tab_id => Some(target),
        _ => None,
    })
}

pub(super) fn table_key(target: &TableTarget) -> String {
    format!(
        "{}::{}.{}.{}",
        target.connection_id, target.database, target.schema, target.table
    )
}

pub(super) fn dropdown_below(trigger: Bounds<Pixels>) -> Point<Pixels> {
    Point::new(trigger.origin.x, trigger.origin.y + px(22.))
}

pub(super) fn overlay_at(id: &'static str, position: Point<Pixels>) -> gpui::Stateful<gpui::Div> {
    div()
        .id(id)
        .tab_group()
        .absolute()
        .left(position.x)
        .top(position.y)
        .min_w(px(180.))
        .max_h(px(300.))
        .overflow_y_scroll()
        .p_1()
        .rounded(px(6.))
        .border_1()
        .border_color(BORDER)
        .bg(PANEL)
        .shadow_lg()
        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
}

fn separator() -> gpui::Div {
    div().h(px(1.)).mx(px(2.)).my_1().bg(BORDER)
}

fn menu_action(
    id: &'static str,
    icon: &'static str,
    label: &'static str,
) -> gpui::Stateful<gpui::Div> {
    div()
        .id(id)
        .tab_index(0)
        .h(px(28.))
        .flex()
        .items_center()
        .gap_2()
        .rounded(px(4.))
        .px(px(6.))
        .text_color(FG)
        .hover(|style| style.bg(PANEL_RAISED))
        .child(Icon::empty().path(icon).size(px(11.)).text_color(FG_MUTED))
        .child(label)
}

#[cfg(test)]
mod tests {
    use super::{dropdown_below, table_key};
    use cellar_desktop_gpui::model::TableTarget;
    use gpui::{point, px, size, Bounds};

    #[test]
    fn preset_key_matches_the_classic_table_identity() {
        assert_eq!(
            table_key(&TableTarget {
                connection_id: "c".into(),
                database: "db".into(),
                schema: "dbo".into(),
                table: "users".into()
            }),
            "c::db.dbo.users"
        );
    }

    #[test]
    fn dropdown_sits_under_the_field_even_if_bounds_are_taller() {
        let trigger = Bounds {
            origin: point(px(800.), px(40.)),
            size: size(px(90.), px(22.)),
        };
        let bloated = Bounds {
            origin: trigger.origin,
            size: size(px(90.), px(54.)),
        };
        let click = point(px(885.), px(51.));
        assert_eq!(dropdown_below(trigger), point(px(800.), px(62.)));
        assert_eq!(dropdown_below(bloated), point(px(800.), px(62.)));
        assert!(dropdown_below(trigger).x < click.x);
    }

    #[test]
    fn split_panes_keep_their_own_dropdown_anchor() {
        let mut bounds = std::collections::HashMap::new();
        bounds.insert(
            1,
            Bounds {
                origin: point(px(10.), px(40.)),
                size: size(px(90.), px(54.)),
            },
        );
        bounds.insert(
            2,
            Bounds {
                origin: point(px(800.), px(40.)),
                size: size(px(90.), px(54.)),
            },
        );
        assert_eq!(
            bounds.get(&1).copied().map(dropdown_below),
            Some(point(px(10.), px(62.)))
        );
        assert_eq!(
            bounds.get(&2).copied().map(dropdown_below),
            Some(point(px(800.), px(62.)))
        );
    }
}
