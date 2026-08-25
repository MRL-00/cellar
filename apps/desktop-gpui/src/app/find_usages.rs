use std::sync::Arc;

use cellar_core::schema::{UsageKind, UsageReference};
use gpui::{div, prelude::*, px, AnyElement, Context, SharedString};
use gpui_component::Icon;

use super::{shell::BottomPanelTab, CellarApp};
use cellar_desktop_gpui::{
    model::{QueryTarget, TableTarget},
    theme::{ACCENT, BORDER, FG_MUTED, FG_SECONDARY, INSET, PANEL, PANEL_RAISED, WARN},
};

pub(super) struct FindUsagesState {
    target: TableTarget,
    column: Option<String>,
    all_schemas: bool,
    status: FindUsagesStatus,
}

enum FindUsagesStatus {
    Loading,
    Ready(Vec<UsageReference>),
    Error(String),
}

impl CellarApp {
    pub(super) fn find_usages_count(&self) -> Option<usize> {
        self.find_usages
            .as_ref()
            .and_then(|search| match &search.status {
                FindUsagesStatus::Ready(results) => Some(results.len()),
                _ => None,
            })
    }

    pub(super) fn start_find_usages(
        &mut self,
        target: TableTarget,
        all_schemas: bool,
        cx: &mut Context<Self>,
    ) {
        self.start_find_usages_for(target, None, all_schemas, cx);
    }

    pub(super) fn start_find_usages_for(
        &mut self,
        target: TableTarget,
        column: Option<String>,
        all_schemas: bool,
        cx: &mut Context<Self>,
    ) {
        self.find_usages_generation = self.find_usages_generation.wrapping_add(1);
        let generation = self.find_usages_generation;
        self.find_usages = Some(FindUsagesState {
            target: target.clone(),
            column: column.clone(),
            all_schemas,
            status: FindUsagesStatus::Loading,
        });
        self.bottom_panel_open = true;
        self.bottom_panel_tab = BottomPanelTab::FindUsages;
        let registry = Arc::clone(&self.registry);
        let runtime = Arc::clone(&self.runtime);
        cx.notify();
        cx.spawn(async move |this, cx| {
            let task_target = target.clone();
            let result = runtime
                .spawn(async move {
                    registry
                        .find_usages(
                            &task_target.connection_id,
                            Some(task_target.database),
                            task_target.schema,
                            task_target.table,
                            column,
                            all_schemas,
                        )
                        .await
                })
                .await
                .map_err(|error| format!("find usages task failed: {error}"))
                .and_then(|result| result.map_err(|error| error.to_string()));
            this.update(cx, |this, cx| {
                if this.find_usages_generation != generation {
                    return;
                }
                if let Some(search) = &mut this.find_usages {
                    search.status = match result {
                        Ok(results) => FindUsagesStatus::Ready(results),
                        Err(error) => FindUsagesStatus::Error(error),
                    };
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    pub(super) fn find_usages_panel(&self, cx: &mut Context<Self>) -> AnyElement {
        let Some(search) = &self.find_usages else {
            return usage_empty(
                "No usages searched yet",
                "Right-click a table in the sidebar and choose Find Usages.",
                false,
            );
        };
        let mut target_label = format!("{}.{}", search.target.schema, search.target.table);
        if let Some(column) = search.column.as_deref() {
            target_label.push('.');
            target_label.push_str(column);
        }
        let this_schema = search.target.clone();
        let this_column = search.column.clone();
        let all_schemas = search.target.clone();
        let all_column = search.column.clone();
        let refresh = search.target.clone();
        let refresh_column = search.column.clone();
        let refresh_all = search.all_schemas;
        div()
            .flex_1()
            .min_h_0()
            .flex()
            .flex_col()
            .bg(INSET)
            .child(
                div()
                    .h(px(32.))
                    .flex_shrink_0()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_3()
                    .px_2()
                    .border_b_1()
                    .border_color(BORDER)
                    .child(
                        div()
                            .min_w_0()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(div().text_color(FG_SECONDARY).child("Find Usages"))
                            .child(
                                div()
                                    .truncate()
                                    .font_family(cellar_desktop_gpui::theme::mono_font())
                                    .text_size(px(11.5))
                                    .text_color(FG_MUTED)
                                    .child(target_label.clone()),
                            ),
                    )
                    .child(
                        div()
                            .flex_shrink_0()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(
                                div()
                                    .flex()
                                    .rounded(px(4.))
                                    .border_1()
                                    .border_color(BORDER)
                                    .overflow_hidden()
                                    .child(
                                        scope_button(
                                            "usage-this-schema",
                                            "This schema",
                                            !search.all_schemas,
                                        )
                                        .on_click(
                                            cx.listener(move |this, _, _, cx| {
                                                this.start_find_usages_for(
                                                    this_schema.clone(),
                                                    this_column.clone(),
                                                    false,
                                                    cx,
                                                );
                                            }),
                                        ),
                                    )
                                    .child(
                                        scope_button(
                                            "usage-all-schemas",
                                            "All schemas",
                                            search.all_schemas,
                                        )
                                        .on_click(
                                            cx.listener(move |this, _, _, cx| {
                                                this.start_find_usages_for(
                                                    all_schemas.clone(),
                                                    all_column.clone(),
                                                    true,
                                                    cx,
                                                );
                                            }),
                                        ),
                                    ),
                            )
                            .child(
                                div()
                                    .id("usage-refresh")
                                    .tab_index(0)
                                    .cursor_pointer()
                                    .size(px(22.))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .rounded(px(4.))
                                    .hover(|style| style.bg(PANEL_RAISED))
                                    .child(Icon::empty().path("icons/book-open.svg").size(px(11.)))
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        this.start_find_usages_for(
                                            refresh.clone(),
                                            refresh_column.clone(),
                                            refresh_all,
                                            cx,
                                        );
                                    })),
                            ),
                    ),
            )
            .child(match &search.status {
                FindUsagesStatus::Loading => usage_empty(
                    "Searching…",
                    "Scanning cached database definitions for structural references.",
                    false,
                ),
                FindUsagesStatus::Error(error) => usage_empty("Search failed", error.clone(), true),
                FindUsagesStatus::Ready(results) if results.is_empty() => usage_empty(
                    "No usages found",
                    if search.all_schemas {
                        format!("No database object references {target_label}.")
                    } else {
                        format!(
                            "No references in schema {}. Try All schemas.",
                            search.target.schema
                        )
                    },
                    false,
                ),
                FindUsagesStatus::Ready(results) => self.usage_results(results, search, cx),
            })
            .into_any_element()
    }

    fn usage_results(
        &self,
        results: &[UsageReference],
        search: &FindUsagesState,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let target = search.target.clone();
        div()
            .id("usage-results")
            .flex_1()
            .min_h_0()
            .overflow_y_scroll()
            .children(results.iter().cloned().enumerate().map(|(index, usage)| {
                let open_usage = usage.clone();
                let open_target = target.clone();
                div()
                    .id(SharedString::from(format!("usage:{index}:{}", usage.name)))
                    .tab_index(0)
                    .cursor_pointer()
                    .flex()
                    .items_start()
                    .gap_2()
                    .px(px(10.))
                    .py_2()
                    .border_b_1()
                    .border_color(BORDER)
                    .hover(|style| style.bg(PANEL))
                    .child(
                        Icon::empty()
                            .path(usage_icon(usage.kind))
                            .size(px(12.))
                            .text_color(FG_MUTED),
                    )
                    .child(
                        div()
                            .min_w_0()
                            .flex_1()
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .child(
                                        div()
                                            .rounded(px(3.))
                                            .bg(PANEL_RAISED)
                                            .px_1()
                                            .font_family(cellar_desktop_gpui::theme::mono_font())
                                            .text_size(px(10.))
                                            .text_color(FG_MUTED)
                                            .child(usage_kind(usage.kind)),
                                    )
                                    .child(
                                        div()
                                            .truncate()
                                            .font_family(cellar_desktop_gpui::theme::mono_font())
                                            .text_color(FG_SECONDARY)
                                            .child(format!("{}.{}", usage.schema, usage.name)),
                                    )
                                    .child(
                                        div()
                                            .ml_auto()
                                            .font_family(cellar_desktop_gpui::theme::mono_font())
                                            .text_size(px(10.5))
                                            .text_color(FG_MUTED)
                                            .child(format!("L{}", usage.line)),
                                    ),
                            )
                            .child(
                                div()
                                    .mt_1()
                                    .truncate()
                                    .font_family(cellar_desktop_gpui::theme::mono_font())
                                    .text_size(px(11.5))
                                    .text_color(FG_MUTED)
                                    .child(usage.snippet.clone()),
                            ),
                    )
                    .on_click(cx.listener(move |this, _, window, cx| {
                        if matches!(
                            open_usage.kind,
                            UsageKind::View | UsageKind::MaterializedView
                        ) {
                            this.open_table(
                                TableTarget {
                                    connection_id: open_target.connection_id.clone(),
                                    database: open_target.database.clone(),
                                    schema: open_usage.schema.clone(),
                                    table: open_usage.name.clone(),
                                },
                                window,
                                cx,
                            );
                        } else {
                            this.open_query(
                                QueryTarget {
                                    connection_id: open_target.connection_id.clone(),
                                    database: open_target.database.clone(),
                                },
                                open_usage.definition.clone(),
                                window,
                                cx,
                            );
                        }
                    }))
            }))
            .into_any_element()
    }
}

fn scope_button(id: &'static str, label: &'static str, active: bool) -> gpui::Stateful<gpui::Div> {
    div()
        .id(id)
        .tab_index(0)
        .cursor_pointer()
        .h(px(20.))
        .flex()
        .items_center()
        .px_2()
        .bg(if active { PANEL_RAISED } else { PANEL })
        .text_size(px(11.))
        .text_color(if active { ACCENT } else { FG_MUTED })
        .child(label)
}

fn usage_empty(title: &'static str, detail: impl Into<SharedString>, warn: bool) -> AnyElement {
    div()
        .flex_1()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap_1()
        .p_6()
        .text_center()
        .text_color(FG_MUTED)
        .child(
            div()
                .text_color(if warn { WARN } else { FG_SECONDARY })
                .child(title),
        )
        .child(div().max_w(px(520.)).child(detail.into()))
        .into_any_element()
}

fn usage_kind(kind: UsageKind) -> &'static str {
    match kind {
        UsageKind::View => "view",
        UsageKind::MaterializedView => "matview",
        UsageKind::Function => "fn",
        UsageKind::Procedure => "proc",
        UsageKind::Trigger => "trigger",
        UsageKind::Constraint => "constraint",
    }
}

fn usage_icon(kind: UsageKind) -> &'static str {
    match kind {
        UsageKind::View | UsageKind::MaterializedView => "icons/tree.svg",
        UsageKind::Function | UsageKind::Procedure => "icons/file-text.svg",
        UsageKind::Trigger | UsageKind::Constraint => "icons/context.svg",
    }
}

#[cfg(test)]
mod tests {
    use cellar_core::schema::UsageKind;

    use super::usage_kind;

    #[test]
    fn usage_kind_labels_match_classic_panel_pills() {
        assert_eq!(usage_kind(UsageKind::MaterializedView), "matview");
        assert_eq!(usage_kind(UsageKind::Function), "fn");
        assert_eq!(usage_kind(UsageKind::Constraint), "constraint");
    }
}
