use std::time::Duration;

use cellar_core::{
    query::{TableFilterClause, TableFilterOperator},
    schema::Table,
};
use gpui::{Context, Window};

use super::CellarApp;
use cellar_desktop_gpui::model::TabKind;

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

    pub(super) fn cycle_quick_filter_column(&mut self, tab_id: u64, cx: &mut Context<Self>) {
        let Some(target) = self.model.tabs().iter().find_map(|tab| match &tab.kind {
            TabKind::Table { target, .. } if tab.id == tab_id => Some(target),
            _ => None,
        }) else {
            return;
        };
        let Some(table) = self.model.table(target) else {
            return;
        };
        let text_columns = table
            .columns
            .iter()
            .enumerate()
            .filter_map(|(index, column)| is_text_type(&column.data_type).then_some(index))
            .collect::<Vec<_>>();
        if text_columns.is_empty() {
            return;
        }
        let current = *self
            .table_quick_filter_columns
            .get(&tab_id)
            .unwrap_or(&text_columns[0]);
        let next = text_columns
            .iter()
            .position(|index| *index == current)
            .map_or(0, |index| (index + 1) % text_columns.len());
        self.table_quick_filter_columns
            .insert(tab_id, text_columns[next]);
        if self.table_quick_filters.contains_key(&tab_id) {
            self.restart_table(tab_id, cx);
        } else {
            cx.notify();
        }
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
