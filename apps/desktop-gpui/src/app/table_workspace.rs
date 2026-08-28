use std::{sync::Arc, time::Duration};

use cellar_core::{
    query::{
        QueryResultSummary, SortDirection, TableBrowseRequest, TableFilterClause,
        TableFilterOperator, TableSortClause,
    },
    schema::Table,
};
use gpui::{
    div, percentage, prelude::*, px, svg, Animation, AnimationExt, AnyElement, Context, Div,
    Stateful, Transformation, Window,
};
use gpui_component::input::{InputEvent, InputState};

use super::{
    table_quick_filter::{is_text_type, quick_filter_clause, quick_filter_operator},
    CellarApp,
};
use cellar_desktop_gpui::{
    grid::{DataGrid, DataGridEvent},
    model::{TabKind, TableLoadState, TablePage, TableTarget, WorkspaceTab},
    theme::{ACCENT, BORDER, FG_MUTED, INSET, PANEL_RAISED, PROD, WARN},
};

impl CellarApp {
    pub(super) fn open_table(
        &mut self,
        target: TableTarget,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let (tab_id, should_load) = self.model.open_table(target.clone());
        if let Some(layout) = self.table_layouts.get(&table_layout_key(&target)).cloned() {
            self.grid_layouts.entry(tab_id).or_insert(layout);
        }
        self.table_filter_inputs.entry(tab_id).or_insert_with(|| {
            cx.new(|cx| InputState::new(window, cx).placeholder("Filter value"))
        });
        if !self.table_quick_filter_inputs.contains_key(&tab_id) {
            let input =
                cx.new(|cx| InputState::new(window, cx).placeholder("Quick filter (id or text)…"));
            let subscription = cx.subscribe(&input, move |this, _, event, cx| {
                if matches!(event, InputEvent::Change) {
                    this.queue_quick_filter(tab_id, cx);
                }
            });
            self.table_quick_filter_inputs.insert(tab_id, input);
            self.table_quick_filter_subscriptions
                .insert(tab_id, subscription);
        }
        self.table_quick_filter_columns
            .entry(tab_id)
            .or_insert_with(|| {
                self.model
                    .table(&target)
                    .and_then(|table| {
                        table
                            .columns
                            .iter()
                            .position(|column| is_text_type(&column.data_type))
                    })
                    .unwrap_or(0)
            });
        self.table_filter_columns.entry(tab_id).or_insert(0);
        self.table_filter_operators
            .entry(tab_id)
            .or_insert_with(|| {
                self.model
                    .table(&target)
                    .and_then(|table| table.columns.first())
                    .map(|column| quick_filter_operator(&column.data_type))
                    .unwrap_or(TableFilterOperator::Equals)
            });
        cx.notify();
        if should_load && self.model.table(&target).is_some() {
            self.start_table_load(tab_id, target, TablePage::default(), cx);
        }
    }

    pub(super) fn resume_table_loads(&mut self, connection_id: &str, cx: &mut Context<Self>) {
        let pending = self
            .model
            .tabs()
            .iter()
            .filter_map(|tab| match &tab.kind {
                TabKind::Table {
                    target,
                    state: TableLoadState::Loading,
                    page,
                } if target.connection_id == connection_id && !self.grids.contains_key(&tab.id) => {
                    Some((tab.id, target.clone(), *page))
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        for (tab_id, target, page) in pending {
            self.start_table_load(tab_id, target, page, cx);
        }
    }

    fn table_reload_allowed(&mut self, tab_id: u64, cx: &mut Context<Self>) -> bool {
        self.grids
            .get(&tab_id)
            .is_none_or(|grid| grid.update(cx, |grid, cx| grid.prepare_for_reload(cx)))
    }

    pub(super) fn reload_table(&mut self, tab_id: u64, cx: &mut Context<Self>) {
        if !self.table_reload_allowed(tab_id, cx) {
            return;
        }
        let Some((target, page)) = self.model.begin_table_load(tab_id, None) else {
            return;
        };
        self.start_table_load(tab_id, target, page, cx);
    }

    fn change_table_page(&mut self, tab_id: u64, next: bool, cx: &mut Context<Self>) {
        if !self.table_reload_allowed(tab_id, cx) {
            return;
        }
        let Some(page) = self.model.table_page(tab_id) else {
            return;
        };
        if (next && !page.has_next()) || (!next && !page.has_previous()) {
            return;
        }
        let offset = if next {
            page.offset.saturating_add(page.limit)
        } else {
            page.offset.saturating_sub(page.limit)
        };
        let Some((target, page)) = self.model.begin_table_load(tab_id, Some(offset)) else {
            return;
        };
        cx.notify();
        self.start_table_load(tab_id, target, page, cx);
    }

    fn change_table_sort(
        &mut self,
        tab_id: u64,
        column: String,
        direction: Option<SortDirection>,
        cx: &mut Context<Self>,
    ) {
        if !self.table_reload_allowed(tab_id, cx) {
            return;
        }
        if let Some(direction) = direction {
            self.table_sorts
                .insert(tab_id, TableSortClause { column, direction });
        } else {
            self.table_sorts.remove(&tab_id);
        }
        let Some((target, page)) = self.model.begin_table_load(tab_id, Some(0)) else {
            return;
        };
        cx.notify();
        self.start_table_load(tab_id, target, page, cx);
    }

    pub(super) fn cycle_toolbar_sort_column(&mut self, tab_id: u64, cx: &mut Context<Self>) {
        let Some(target) = self.model.tabs().iter().find_map(|tab| match &tab.kind {
            TabKind::Table { target, .. } if tab.id == tab_id => Some(target),
            _ => None,
        }) else {
            return;
        };
        let Some(table) = self.model.table(target) else {
            return;
        };
        let current = self.table_sorts.get(&tab_id).and_then(|sort| {
            table
                .columns
                .iter()
                .position(|column| column.name == sort.column)
        });
        let next = current.map_or(0, |index| index + 1);
        if next >= table.columns.len() {
            self.change_table_sort(tab_id, String::new(), None, cx);
        } else {
            let direction = self
                .table_sorts
                .get(&tab_id)
                .map(|sort| sort.direction)
                .unwrap_or(SortDirection::Asc);
            self.change_table_sort(
                tab_id,
                table.columns[next].name.clone(),
                Some(direction),
                cx,
            );
        }
    }

    pub(super) fn toggle_toolbar_sort_direction(&mut self, tab_id: u64, cx: &mut Context<Self>) {
        let Some(sort) = self.table_sorts.get(&tab_id).cloned() else {
            return;
        };
        self.change_table_sort(
            tab_id,
            sort.column,
            Some(match sort.direction {
                SortDirection::Asc => SortDirection::Desc,
                SortDirection::Desc => SortDirection::Asc,
            }),
            cx,
        );
    }

    pub(super) fn cycle_filter_column(&mut self, tab_id: u64, cx: &mut Context<Self>) {
        let Some(tab) = self.model.tabs().iter().find(|tab| tab.id == tab_id) else {
            return;
        };
        let TabKind::Table { target, .. } = &tab.kind else {
            return;
        };
        let count = self
            .model
            .table(target)
            .map(|table| table.columns.len())
            .unwrap_or(0);
        if count == 0 {
            return;
        }
        let column = self.table_filter_columns.entry(tab_id).or_insert(0);
        *column = (*column + 1) % count;
        if let Some(table) = self.model.table(target) {
            self.table_filter_operators.insert(
                tab_id,
                quick_filter_operator(&table.columns[*column].data_type),
            );
        }
        cx.notify();
    }

    pub(super) fn cycle_filter_operator(&mut self, tab_id: u64, cx: &mut Context<Self>) {
        let Some(data_type) = self.model.tabs().iter().find_map(|tab| {
            if tab.id != tab_id {
                return None;
            }
            let TabKind::Table { target, .. } = &tab.kind else {
                return None;
            };
            self.model.table(target).and_then(|table| {
                table
                    .columns
                    .get(*self.table_filter_columns.get(&tab_id).unwrap_or(&0))
                    .map(|column| column.data_type.clone())
            })
        }) else {
            return;
        };
        let operator = self
            .table_filter_operators
            .entry(tab_id)
            .or_insert(TableFilterOperator::Equals);
        *operator = next_filter_operator(*operator, &data_type);
        cx.notify();
    }

    pub(super) fn apply_table_filter(
        &mut self,
        tab_id: u64,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.table_reload_allowed(tab_id, cx) {
            return;
        }
        let Some(value) = self
            .table_filter_inputs
            .get(&tab_id)
            .map(|input| input.read(cx).value().trim().to_string())
        else {
            return;
        };
        let Some(tab) = self.model.tabs().iter().find(|tab| tab.id == tab_id) else {
            return;
        };
        let TabKind::Table { target, .. } = &tab.kind else {
            return;
        };
        let Some(column) = self.model.table(target).and_then(|table| {
            table
                .columns
                .get(*self.table_filter_columns.get(&tab_id).unwrap_or(&0))
        }) else {
            return;
        };
        let operator = self
            .table_filter_operators
            .get(&tab_id)
            .copied()
            .unwrap_or_else(|| quick_filter_operator(&column.data_type));
        let null_check = matches!(
            operator,
            TableFilterOperator::IsNull | TableFilterOperator::IsNotNull
        );
        if value.is_empty() && !null_check {
            return;
        }
        let filters = self.table_filters.entry(tab_id).or_default();
        filters.retain(|filter| filter.column != column.name || filter.operator != operator);
        filters.push(TableFilterClause {
            column: column.name.clone(),
            operator,
            value: (!null_check).then_some(value),
        });
        self.table_filter_composers.remove(&tab_id);
        if let Some(input) = self.table_filter_inputs.get(&tab_id) {
            input.update(cx, |input, cx| input.set_value("", window, cx));
        }
        self.restart_table(tab_id, cx);
    }

    pub(super) fn open_table_filter(
        &mut self,
        tab_id: u64,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.table_filter_composers.insert(tab_id);
        if let Some(input) = self.table_filter_inputs.get(&tab_id) {
            input.update(cx, |input, cx| input.focus(window, cx));
        }
        cx.notify();
    }

    pub(super) fn edit_table_filter(
        &mut self,
        tab_id: u64,
        index: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(filter) = self
            .table_filters
            .get(&tab_id)
            .and_then(|filters| filters.get(index))
            .cloned()
        else {
            return;
        };
        let Some(target) = self.model.tabs().iter().find_map(|tab| match &tab.kind {
            TabKind::Table { target, .. } if tab.id == tab_id => Some(target),
            _ => None,
        }) else {
            return;
        };
        if let Some(column) = self.model.table(target).and_then(|table| {
            table
                .columns
                .iter()
                .position(|column| column.name == filter.column)
        }) {
            self.table_filter_columns.insert(tab_id, column);
        }
        self.table_filter_operators.insert(tab_id, filter.operator);
        self.table_filters
            .get_mut(&tab_id)
            .map(|filters| filters.remove(index));
        if let Some(input) = self.table_filter_inputs.get(&tab_id) {
            input.update(cx, |input, cx| {
                input.set_value(filter.value.unwrap_or_default(), window, cx);
                input.focus(window, cx);
            });
        }
        self.table_filter_composers.insert(tab_id);
        cx.notify();
    }

    pub(super) fn remove_table_filter(
        &mut self,
        tab_id: u64,
        index: usize,
        cx: &mut Context<Self>,
    ) {
        if !self.table_reload_allowed(tab_id, cx) {
            return;
        }
        let Some(filters) = self.table_filters.get_mut(&tab_id) else {
            return;
        };
        if index >= filters.len() {
            return;
        }
        filters.remove(index);
        if filters.is_empty() {
            self.table_filters.remove(&tab_id);
        }
        self.restart_table(tab_id, cx);
    }

    pub(super) fn clear_table_filter(
        &mut self,
        tab_id: u64,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.table_reload_allowed(tab_id, cx) {
            return;
        }
        self.table_filters.remove(&tab_id);
        self.table_filter_composers.remove(&tab_id);
        if let Some(input) = self.table_filter_inputs.get(&tab_id) {
            input.update(cx, |input, cx| input.set_value("", window, cx));
        }
        self.restart_table(tab_id, cx);
    }

    pub(super) fn restart_table(&mut self, tab_id: u64, cx: &mut Context<Self>) {
        let Some((target, page)) = self.model.reset_table_page(tab_id) else {
            return;
        };
        cx.notify();
        self.start_table_load(tab_id, target, page, cx);
    }

    pub(super) fn start_table_load(
        &mut self,
        tab_id: u64,
        target: TableTarget,
        page: TablePage,
        cx: &mut Context<Self>,
    ) {
        self.query_summaries.remove(&tab_id);
        let generation = self.model.next_table_load(tab_id);
        let Some(table) = self.model.table(&target).cloned() else {
            self.model.finish_table_load(
                tab_id,
                generation,
                Err("Table metadata is unavailable".into()),
            );
            cx.notify();
            return;
        };
        self.load_table(tab_id, generation, target, table, page, cx);
    }

    fn load_table(
        &mut self,
        tab_id: u64,
        generation: u64,
        target: TableTarget,
        table: Table,
        page: TablePage,
        cx: &mut Context<Self>,
    ) {
        let registry = Arc::clone(&self.registry);
        let runtime = Arc::clone(&self.runtime);
        let sort = self.table_sorts.get(&tab_id).cloned();
        let mut filters = self.table_filters.get(&tab_id).cloned().unwrap_or_default();
        if let Some(value) = self.table_quick_filters.get(&tab_id) {
            if let Some(filter) = quick_filter_clause(
                &table,
                value,
                *self.table_quick_filter_columns.get(&tab_id).unwrap_or(&0),
            ) {
                filters.push(filter);
            }
        }
        cx.spawn(async move |this, cx| {
            let request = TableBrowseRequest {
                connection_id: target.connection_id.clone(),
                database: Some(target.database.clone()),
                schema: target.schema.clone(),
                table: target.table.clone(),
                limit: Some(page.limit),
                offset: Some(page.offset),
                sorts: sort.clone().into_iter().collect(),
                filters,
                primary_key_fallback_ordering: true,
                include_total: page.total_rows.is_none(),
            };
            let result = runtime
                .spawn(async move { registry.browse_table(request).await })
                .await
                .map_err(|error| format!("table task failed: {error}"))
                .and_then(|result| result.map_err(|error| error.to_string()));
            this.update(cx, |this, cx| {
                match result {
                    Ok(result) => {
                        let rows = result.rows.len().min(u32::MAX as usize) as u32;
                        let total_rows = result.total_rows;
                        if !this
                            .model
                            .finish_table_load(tab_id, generation, Ok((rows, total_rows)))
                        {
                            return;
                        }
                        this.query_summaries.insert(
                            tab_id,
                            QueryResultSummary {
                                notices: result.notices.clone(),
                                notice_capture: result.notice_capture.clone(),
                                rows_affected: result.rows_affected,
                                duration_ms: result.duration_ms,
                                truncated: result.truncated,
                                total_rows: result.total_rows,
                                row_count: result.rows.len() as u64,
                            },
                        );
                        this.last_query_metrics =
                            Some((u64::from(rows), result.truncated, result.duration_ms));
                        let grid_sort = sort.as_ref().and_then(|sort| {
                            result
                                .columns
                                .iter()
                                .position(|column| column.name == sort.column)
                                .map(|index| (index, sort.direction))
                        });
                        let null_display = this.preferences.grid.null_display.clone();
                        let stripe_rows = this.preferences.grid.stripe_rows;
                        let grid = cx.new(|cx| {
                            let mut grid =
                                DataGrid::new_table(result, target, table, grid_sort, cx);
                            grid.set_display_preferences(null_display, stripe_rows, cx);
                            grid
                        });
                        if let Some(layout) = this.grid_layouts.get(&tab_id) {
                            grid.update(cx, |grid, cx| grid.apply_layout(layout, cx));
                        }
                        cx.subscribe(&grid, move |this, _, event: &DataGridEvent, cx| match event
                            .clone()
                        {
                            DataGridEvent::ImportCsv => this.open_csv_import(tab_id, cx),
                            DataGridEvent::ReviewChanges {
                                connection_id,
                                request,
                            } => this.open_commit_review(tab_id, connection_id, request, cx),
                            DataGridEvent::SortColumn { column, direction } => {
                                this.change_table_sort(tab_id, column, direction, cx)
                            }
                            DataGridEvent::FindUsages { target, column } => {
                                this.start_find_usages_for(target, column, false, cx)
                            }
                        })
                        .detach();
                        this.grids.insert(tab_id, grid);
                    }
                    Err(error) => {
                        this.model.finish_table_load(tab_id, generation, Err(error));
                    }
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    pub(super) fn table_content(
        &self,
        tab: &WorkspaceTab,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let TabKind::Table {
            target,
            state,
            page,
        } = &tab.kind
        else {
            unreachable!("table_content called for a non-table tab");
        };
        match state {
            TableLoadState::Loading => div()
                .flex_1()
                .flex()
                .items_center()
                .justify_center()
                .gap_2()
                .bg(INSET)
                .text_size(px(12.5))
                .text_color(FG_MUTED)
                .child(
                    svg()
                        .path("icons/spinner.svg")
                        .size(px(14.))
                        .text_color(ACCENT)
                        .with_animation(
                            "table-loading-spinner",
                            Animation::new(Duration::from_millis(900)).repeat(),
                            |icon, delta| {
                                icon.with_transformation(Transformation::rotate(percentage(delta)))
                            },
                        ),
                )
                .child(format!("Loading {}.{}…", target.schema, target.table))
                .into_any_element(),
            TableLoadState::Error(_) => div()
                .flex_1()
                .flex()
                .items_center()
                .justify_center()
                .bg(INSET)
                .text_size(px(12.5))
                .text_color(WARN)
                .child("Table load failed. See Messages for details.")
                .into_any_element(),
            TableLoadState::Loaded => {
                let Some(grid) = self.grids.get(&tab.id).cloned() else {
                    return div()
                        .flex_1()
                        .p_4()
                        .text_color(PROD)
                        .child("Grid state is unavailable")
                        .into_any_element();
                };
                let previous = tab.id;
                let next = tab.id;
                div()
                    .flex_1()
                    .min_h_0()
                    .flex()
                    .flex_col()
                    .when_some(
                        self.table_filter_bar(tab.id, target, *page, cx),
                        |element, bar| element.child(bar),
                    )
                    .child(grid)
                    .child(
                        div()
                            .h(px(32.))
                            .flex_shrink_0()
                            .flex()
                            .items_center()
                            .justify_between()
                            .px_3()
                            .border_t_1()
                            .border_color(BORDER)
                            .bg(PANEL_RAISED)
                            .child(page_label(*page))
                            .child(
                                div()
                                    .flex()
                                    .gap_3()
                                    .child(pager_button("Previous", page.has_previous()).on_click(
                                        cx.listener(move |this, _, _, cx| {
                                            this.change_table_page(previous, false, cx);
                                        }),
                                    ))
                                    .child(pager_button("Next", page.has_next()).on_click(
                                        cx.listener(move |this, _, _, cx| {
                                            this.change_table_page(next, true, cx);
                                        }),
                                    )),
                            ),
                    )
                    .into_any_element()
            }
        }
    }
}

pub(super) fn table_layout_key(target: &TableTarget) -> String {
    format!(
        "{}::{}.{}.{}",
        target.connection_id, target.database, target.schema, target.table
    )
}

fn next_filter_operator(operator: TableFilterOperator, data_type: &str) -> TableFilterOperator {
    let kind = data_type.to_ascii_lowercase();
    let operators: &[TableFilterOperator] = if ["char", "text", "string", "uuid"]
        .iter()
        .any(|needle| kind.contains(needle))
    {
        &[
            TableFilterOperator::Equals,
            TableFilterOperator::NotEquals,
            TableFilterOperator::Contains,
            TableFilterOperator::NotContains,
            TableFilterOperator::StartsWith,
            TableFilterOperator::EndsWith,
            TableFilterOperator::Like,
            TableFilterOperator::IsNull,
            TableFilterOperator::IsNotNull,
        ]
    } else if [
        "int", "serial", "oid", "float", "double", "real", "numeric", "decimal", "date", "time",
    ]
    .iter()
    .any(|needle| kind.contains(needle))
    {
        &[
            TableFilterOperator::Equals,
            TableFilterOperator::NotEquals,
            TableFilterOperator::GreaterThan,
            TableFilterOperator::GreaterThanOrEqual,
            TableFilterOperator::LessThan,
            TableFilterOperator::LessThanOrEqual,
            TableFilterOperator::IsNull,
            TableFilterOperator::IsNotNull,
        ]
    } else {
        &[
            TableFilterOperator::Equals,
            TableFilterOperator::NotEquals,
            TableFilterOperator::IsNull,
            TableFilterOperator::IsNotNull,
        ]
    };
    let index = operators
        .iter()
        .position(|candidate| *candidate == operator)
        .unwrap_or(0);
    operators[(index + 1) % operators.len()]
}

fn page_label(page: TablePage) -> String {
    if page.rows == 0 {
        return "0 rows".into();
    }
    let first = u64::from(page.offset) + 1;
    let last = u64::from(page.offset) + u64::from(page.rows);
    match page.total_rows {
        Some(total) => format!("{first}–{last} of {total}"),
        None => format!("{first}–{last}"),
    }
}

fn pager_button(label: &'static str, enabled: bool) -> Stateful<Div> {
    div()
        .id(label)
        .px_2()
        .py_1()
        .text_color(if enabled { ACCENT } else { FG_MUTED })
        .when(enabled, |element| element.tab_index(0).cursor_pointer())
}

#[cfg(test)]
mod tests {
    use super::{next_filter_operator, page_label};
    use cellar_core::query::TableFilterOperator;
    use cellar_desktop_gpui::model::TablePage;

    #[test]
    fn page_bounds_and_labels_are_stable() {
        let page = TablePage {
            offset: 500,
            limit: 500,
            rows: 120,
            total_rows: Some(620),
        };
        assert!(page.has_previous());
        assert!(!page.has_next());
        assert_eq!(page_label(page), "501–620 of 620");
    }

    #[test]
    fn advanced_filter_operators_cycle_back_to_equals() {
        let mut operator = TableFilterOperator::Equals;
        for _ in 0..9 {
            operator = next_filter_operator(operator, "text");
        }
        assert_eq!(operator, TableFilterOperator::Equals);
        assert_eq!(
            next_filter_operator(TableFilterOperator::NotEquals, "int8"),
            TableFilterOperator::GreaterThan
        );
    }
}
