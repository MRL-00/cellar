use cellar_core::{query::SortDirection, schema::Table};
use gpui::{
    div, prelude::*, AnyElement, Context, MouseButton, Pixels, Point, SharedString,
};
use gpui_component::Icon;

use super::{
    table_presets::{dropdown_below, overlay_at, table_target},
    table_quick_filter::quick_filter_operator,
    CellarApp,
};
use cellar_desktop_gpui::theme::{ui_px, ACCENT, FG, FG_MUTED, PANEL_RAISED};

pub(super) struct ColumnMenu {
    pub(super) tab_id: u64,
    pub(super) position: Point<Pixels>,
}

impl CellarApp {
    pub(super) fn open_filter_column_menu(&mut self, tab_id: u64, cx: &mut Context<Self>) {
        let Some(position) = self
            .filter_column_trigger_bounds
            .get(&tab_id)
            .copied()
            .map(dropdown_below)
        else {
            return;
        };
        if self
            .table_filter_column_menu
            .as_ref()
            .is_some_and(|menu| menu.tab_id == tab_id)
        {
            self.table_filter_column_menu = None;
            cx.notify();
            return;
        }
        self.table_preset_menu = None;
        self.table_quick_column_menu = None;
        self.table_sort_column_menu = None;
        self.table_filter_column_menu = Some(ColumnMenu { tab_id, position });
        cx.notify();
    }

    pub(super) fn open_sort_column_menu(&mut self, tab_id: u64, cx: &mut Context<Self>) {
        let Some(position) = self
            .sort_column_trigger_bounds
            .get(&tab_id)
            .copied()
            .map(dropdown_below)
        else {
            return;
        };
        if self
            .table_sort_column_menu
            .as_ref()
            .is_some_and(|menu| menu.tab_id == tab_id)
        {
            self.table_sort_column_menu = None;
            cx.notify();
            return;
        }
        self.table_preset_menu = None;
        self.table_quick_column_menu = None;
        self.table_filter_column_menu = None;
        self.table_sort_column_menu = Some(ColumnMenu { tab_id, position });
        cx.notify();
    }

    fn set_filter_column(&mut self, tab_id: u64, index: usize, cx: &mut Context<Self>) {
        self.table_filter_column_menu = None;
        let Some(target) = table_target(self, tab_id).cloned() else {
            return;
        };
        self.table_filter_columns.insert(tab_id, index);
        if let Some(operator) = self
            .model
            .table(&target)
            .and_then(|table| table.columns.get(index))
            .map(|column| quick_filter_operator(&column.data_type))
        {
            self.table_filter_operators.insert(tab_id, operator);
        }
        cx.notify();
    }

    fn set_sort_column(&mut self, tab_id: u64, column: Option<String>, cx: &mut Context<Self>) {
        self.table_sort_column_menu = None;
        match column {
            Some(column) => {
                let direction = self
                    .table_sorts
                    .get(&tab_id)
                    .map(|sort| sort.direction)
                    .unwrap_or(SortDirection::Asc);
                self.change_table_sort(tab_id, column, Some(direction), cx);
            }
            None => self.change_table_sort(tab_id, String::new(), None, cx),
        }
    }

    pub(super) fn filter_column_menu_overlay(&self, cx: &mut Context<Self>) -> AnyElement {
        let state = self
            .table_filter_column_menu
            .as_ref()
            .expect("filter column menu requires state");
        let tab_id = state.tab_id;
        let position = state.position;
        let current = *self.table_filter_columns.get(&tab_id).unwrap_or(&0);
        let columns = table_target(self, tab_id)
            .and_then(|target| self.model.table(target))
            .map(sorted_columns)
            .unwrap_or_default();
        let mut menu = overlay_at("filter-column-menu", position);
        for (index, name) in columns {
            menu = menu.child(column_row(
                format!("filter-column-pick:{tab_id}:{index}"),
                name,
                index == current,
                cx.listener(move |this, _, _, cx| {
                    this.set_filter_column(tab_id, index, cx);
                }),
            ));
        }
        div()
            .id("filter-column-backdrop")
            .absolute()
            .inset_0()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _, _, cx| {
                    this.table_filter_column_menu = None;
                    cx.notify();
                }),
            )
            .child(menu)
            .into_any_element()
    }

    pub(super) fn sort_column_menu_overlay(&self, cx: &mut Context<Self>) -> AnyElement {
        let state = self
            .table_sort_column_menu
            .as_ref()
            .expect("sort column menu requires state");
        let tab_id = state.tab_id;
        let position = state.position;
        let current = self.table_sorts.get(&tab_id).map(|sort| sort.column.clone());
        let columns = table_target(self, tab_id)
            .and_then(|target| self.model.table(target))
            .map(sorted_columns)
            .unwrap_or_default();
        let mut menu = overlay_at("sort-column-menu", position).child(column_row(
            format!("sort-column-pick:{tab_id}:none"),
            "None".to_string(),
            current.is_none(),
            cx.listener(move |this, _, _, cx| {
                this.set_sort_column(tab_id, None, cx);
            }),
        ));
        for (_, name) in columns {
            let selected = current.as_deref() == Some(name.as_str());
            menu = menu.child(column_row(
                format!("sort-column-pick:{tab_id}:{name}"),
                name.clone(),
                selected,
                cx.listener(move |this, _, _, cx| {
                    this.set_sort_column(tab_id, Some(name.clone()), cx);
                }),
            ));
        }
        div()
            .id("sort-column-backdrop")
            .absolute()
            .inset_0()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _, _, cx| {
                    this.table_sort_column_menu = None;
                    cx.notify();
                }),
            )
            .child(menu)
            .into_any_element()
    }
}

fn sorted_columns(table: &Table) -> Vec<(usize, String)> {
    let mut columns: Vec<(usize, String)> = table
        .columns
        .iter()
        .enumerate()
        .map(|(index, column)| (index, column.name.clone()))
        .collect();
    columns.sort_by(|a, b| a.1.to_lowercase().cmp(&b.1.to_lowercase()));
    columns
}

fn column_row(
    id: String,
    name: String,
    selected: bool,
    on_click: impl Fn(&gpui::ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static,
) -> gpui::Stateful<gpui::Div> {
    div()
        .id(SharedString::from(id))
        .tab_index(0)
        .cursor_pointer()
        .h(ui_px(28.))
        .flex()
        .items_center()
        .gap(ui_px(7.))
        .rounded(ui_px(4.))
        .px(ui_px(6.))
        .text_color(if selected { ACCENT } else { FG })
        .hover(|style| style.bg(PANEL_RAISED))
        .child(div().w(ui_px(12.)).flex().justify_center().when(
            selected,
            |element| {
                element.child(
                    Icon::empty()
                        .path("icons/grid-check.svg")
                        .size(ui_px(10.))
                        .text_color(FG_MUTED),
                )
            },
        ))
        .child(div().flex_1().truncate().child(name))
        .on_click(on_click)
}
