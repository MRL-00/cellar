use std::time::Duration;

use cellar_core::{
    query::{TableFilterClause, TableFilterOperator},
    schema::Table,
};
use gpui::{
    div, prelude::*, px, AnyElement, Context, MouseButton, Pixels, Point, SharedString, Window,
};

use super::{table_presets::overlay_at, CellarApp};
use cellar_desktop_gpui::{
    model::TabKind,
    theme::{ACCENT, FG, PANEL_RAISED},
};

pub(super) struct QuickColumnMenu {
    pub(super) tab_id: u64,
    pub(super) position: Point<Pixels>,
}

impl CellarApp {
    pub(super) fn queue_quick_filter(&mut self, tab_id: u64, cx: &mut Context<Self>) {
        let Some(input) = self.table_quick_filter_inputs.get(&tab_id).cloned() else {
            return;
        };
        let expected = input.read(cx).value().trim().to_owned();
        cx.spawn(async move |this, cx| {
            cx.background_executor()
                .timer(Duration::from_millis(250))
                .await;
            this.update(cx, |this, cx| {
                let current = input.read(cx).value().trim().to_owned();
                if current != expected
                    || this.table_quick_filters.get(&tab_id).map(String::as_str)
                        == Some(current.as_str())
                    || current.is_empty() && !this.table_quick_filters.contains_key(&tab_id)
                {
                    return;
                }
                if current.is_empty() {
                    this.table_quick_filters.remove(&tab_id);
                } else {
                    this.table_quick_filters.insert(tab_id, current);
                }
                this.restart_table(tab_id, cx);
            })
            .ok();
        })
        .detach();
    }

    pub(super) fn open_quick_column_menu(&mut self, tab_id: u64, cx: &mut Context<Self>) {
        let Some(position) = self
            .quick_column_trigger_bounds
            .get(&tab_id)
            .copied()
            .map(super::table_presets::dropdown_below)
        else {
            return;
        };
        if self
            .table_quick_column_menu
            .as_ref()
            .is_some_and(|menu| menu.tab_id == tab_id)
        {
            self.table_quick_column_menu = None;
            cx.notify();
            return;
        }
        self.table_preset_menu = None;
        self.table_quick_column_menu = Some(QuickColumnMenu { tab_id, position });
        cx.notify();
    }

    pub(super) fn set_quick_filter_column(
        &mut self,
        tab_id: u64,
        column: usize,
        cx: &mut Context<Self>,
    ) {
        self.table_quick_filter_columns.insert(tab_id, column);
        self.table_quick_column_menu = None;
        if self.table_quick_filters.contains_key(&tab_id) {
            self.restart_table(tab_id, cx);
        } else {
            cx.notify();
        }
    }

    pub(super) fn quick_column_menu_overlay(&self, cx: &mut Context<Self>) -> AnyElement {
        let state = self
            .table_quick_column_menu
            .as_ref()
            .expect("quick column menu requires state");
        let tab_id = state.tab_id;
        let position = state.position;
        let current = *self.table_quick_filter_columns.get(&tab_id).unwrap_or(&0);
        let columns = self
            .model
            .tabs()
            .iter()
            .find_map(|tab| match &tab.kind {
                TabKind::Table { target, .. } if tab.id == tab_id => self.model.table(target),
                _ => None,
            })
            .map(|table| {
                table
                    .columns
                    .iter()
                    .enumerate()
                    .filter(|(_, column)| is_text_type(&column.data_type))
                    .map(|(index, column)| (index, column.name.clone()))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let mut menu = overlay_at("quick-column-menu", position);
        for (index, name) in columns {
            let selected = index == current;
            menu = menu.child(
                div()
                    .id(SharedString::from(format!(
                        "quick-column-pick:{tab_id}:{index}"
                    )))
                    .tab_index(0)
                    .cursor_pointer()
                    .h(px(28.))
                    .flex()
                    .items_center()
                    .gap(px(7.))
                    .rounded(px(4.))
                    .px(px(6.))
                    .text_color(if selected { ACCENT } else { FG })
                    .hover(|style| style.bg(PANEL_RAISED))
                    .child(div().w(px(12.)).flex().justify_center().when(
                        selected,
                        |element| {
                            element.child(
                                gpui_component::Icon::empty()
                                    .path("icons/grid-check.svg")
                                    .size(px(10.)),
                            )
                        },
                    ))
                    .child(div().flex_1().truncate().child(name))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.set_quick_filter_column(tab_id, index, cx);
                    })),
            );
        }
        div()
            .id("quick-column-backdrop")
            .absolute()
            .inset_0()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _, _, cx| {
                    this.table_quick_column_menu = None;
                    cx.notify();
                }),
            )
            .child(menu)
            .into_any_element()
    }

    pub(super) fn clear_quick_filter(
        &mut self,
        tab_id: u64,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let had_filter = self.table_quick_filters.remove(&tab_id).is_some();
        if let Some(input) = self.table_quick_filter_inputs.get(&tab_id) {
            input.update(cx, |input, cx| input.set_value("", window, cx));
        }
        if had_filter {
            self.restart_table(tab_id, cx);
        } else {
            cx.notify();
        }
    }
}

pub(super) fn quick_filter_operator(data_type: &str) -> TableFilterOperator {
    let data_type = data_type.to_ascii_lowercase();
    if ["char", "text", "string", "uuid"]
        .iter()
        .any(|kind| data_type.contains(kind))
    {
        TableFilterOperator::Contains
    } else {
        TableFilterOperator::Equals
    }
}

pub(super) fn is_text_type(data_type: &str) -> bool {
    quick_filter_operator(data_type) == TableFilterOperator::Contains
}

pub(super) fn quick_filter_clause(
    table: &Table,
    value: &str,
    text_column: usize,
) -> Option<TableFilterClause> {
    let value = value.trim();
    if value.is_empty() {
        return None;
    }
    let digits = value.strip_prefix('-').unwrap_or(value);
    if !digits.is_empty() && digits.bytes().all(|byte| byte.is_ascii_digit()) {
        let id = table.primary_key.first().or_else(|| {
            table
                .columns
                .iter()
                .find(|column| column.name.eq_ignore_ascii_case("id"))
                .map(|column| &column.name)
        });
        if let Some(column) = id {
            return Some(TableFilterClause {
                column: column.clone(),
                operator: TableFilterOperator::Equals,
                value: Some(value.into()),
            });
        }
    }
    let column = table
        .columns
        .get(text_column)
        .filter(|column| is_text_type(&column.data_type))
        .or_else(|| {
            table
                .columns
                .iter()
                .find(|column| is_text_type(&column.data_type))
        })?;
    Some(TableFilterClause {
        column: column.name.clone(),
        operator: TableFilterOperator::Contains,
        value: Some(value.into()),
    })
}

#[cfg(test)]
mod tests {
    use cellar_core::{
        query::TableFilterOperator,
        schema::{Column, Table},
    };

    use super::{quick_filter_clause, quick_filter_operator};

    #[test]
    fn quick_filter_matches_classic_id_and_text_routing() {
        let column = |name: &str, data_type: &str, primary: bool| Column {
            name: name.into(),
            data_type: data_type.into(),
            nullable: false,
            default: None,
            is_primary_key: primary,
            ordinal: 0,
            comment: None,
        };
        let table = Table {
            name: "people".into(),
            schema: "public".into(),
            row_count: None,
            columns: vec![column("id", "int8", true), column("name", "text", false)],
            primary_key: vec!["id".into()],
            foreign_keys: vec![],
            indexes: vec![],
        };
        let numeric = quick_filter_clause(&table, "42", 1).unwrap();
        let text = quick_filter_clause(&table, "Ada", 1).unwrap();
        assert_eq!(
            (numeric.column.as_str(), numeric.operator),
            ("id", TableFilterOperator::Equals)
        );
        assert_eq!(
            (text.column.as_str(), text.operator),
            ("name", TableFilterOperator::Contains)
        );
        assert_eq!(quick_filter_operator("uuid"), TableFilterOperator::Contains);
    }
}
