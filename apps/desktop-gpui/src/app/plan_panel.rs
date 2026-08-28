use cellar_core::query::{PlanMode, PlanNode, QueryPlan};
use gpui::{div, prelude::*, px, AnyElement, ClipboardItem, Context, SharedString};
use gpui_component::Icon;

use super::CellarApp;
use cellar_desktop_gpui::{
    model::{ConnectionState, TabKind},
    theme::{
        ACCENT, BORDER, FG, FG_MUTED, FG_SECONDARY, INSET, PANEL, PANEL_RAISED, PROD, WARN,
        WARN_SOFT,
    },
};

impl CellarApp {
    pub(super) fn query_plan_panel(&self, cx: &mut Context<Self>) -> AnyElement {
        let Some(tab) = self.model.active_tab() else {
            return plan_empty(
                "No query selected",
                "Open a SQL query tab to inspect its plan.",
            );
        };
        let TabKind::Query { target, .. } = &tab.kind else {
            return plan_empty(
                "Plans are available for SQL query tabs",
                "Table tabs do not expose freeform SQL to explain.",
            );
        };
        let tab_id = tab.id;
        let title = tab.title.clone();
        let mode = self
            .plan_modes
            .get(&tab_id)
            .copied()
            .unwrap_or(PlanMode::Estimate);
        let loading = self.plan_loading.contains(&tab_id);
        let plan = self.query_plans.get(&tab_id).cloned();
        let statement = self
            .editors
            .get(&tab_id)
            .map_or_else(String::new, |editor| {
                let editor = editor.read(cx);
                let sql = editor.value();
                cellar_sql::statement_at_offset(&sql, editor.cursor())
                    .map(|statement| statement.text.to_owned())
                    .unwrap_or_default()
            });
        let unavailable = self
            .model
            .connections()
            .iter()
            .find(|config| config.id == target.connection_id)
            .map_or(Some("No connection selected"), |config| {
                if config.engine != cellar_core::driver::Engine::Postgres {
                    Some("Execution plans are Postgres-only for now")
                } else if !matches!(
                    self.model.connection_state(&target.connection_id),
                    ConnectionState::Connected
                ) {
                    Some("Connection is not open")
                } else if statement.trim().is_empty() {
                    Some("No SQL statement selected")
                } else {
                    None
                }
            });
        let stale = plan
            .as_ref()
            .and_then(|plan| plan.as_ref().ok())
            .is_some_and(|plan| normalize_sql(&plan.sql) != normalize_sql(&statement));

        div()
            .id("query-plan")
            .flex_1()
            .min_h_0()
            .flex()
            .flex_col()
            .bg(INSET)
            .child(
                div()
                    .h(px(36.))
                    .flex_shrink_0()
                    .flex()
                    .items_center()
                    .justify_between()
                    .px(px(10.))
                    .border_b_1()
                    .border_color(BORDER)
                    .child(
                        div()
                            .min_w_0()
                            .flex()
                            .items_center()
                            .gap_2()
                            .text_color(FG_SECONDARY)
                            .child(Icon::empty().path("icons/tree.svg").size(px(13.)).text_color(ACCENT))
                            .child(div().font_family(cellar_desktop_gpui::theme::mono_font()).text_size(px(12.)).child(title))
                            .when(stale, |element| element.child(badge("stale", WARN.rgba()))),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(6.))
                            .child(mode_picker(tab_id, mode, cx))
                            .child(plan_button("plan-json", "icons/copy.svg", "JSON", plan.as_ref().and_then(|plan| plan.as_ref().ok()).is_some()).when(
                                plan.as_ref().and_then(|plan| plan.as_ref().ok()).is_some(),
                                |button| {
                                    let json = serde_json::to_string_pretty(
                                        &plan.as_ref().and_then(|plan| plan.as_ref().ok()).expect("checked").raw_json,
                                    )
                                    .unwrap_or_default();
                                    button.on_click(cx.listener(move |_, _, _, cx| {
                                        cx.write_to_clipboard(ClipboardItem::new_string(json.clone()));
                                    }))
                                },
                            ))
                            .child(
                                plan_button("plan-run", "icons/bolt.svg", if loading { "Explaining…" } else { "Run" }, unavailable.is_none() && !loading)
                                    .when(unavailable.is_none() && !loading, |button| {
                                        button.on_click(cx.listener(move |this, _, window, cx| {
                                            this.explain_query(tab_id, mode, window, cx);
                                        }))
                                    }),
                            ),
                    ),
            )
            .child(
                div()
                    .id("plan-scroll")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .p(px(10.))
                    .when(mode == PlanMode::Analyze, |element| {
                        element.child(analyze_warning())
                    })
                    .child(if let Some(reason) = unavailable {
                        plan_empty(reason, unavailable_detail(reason))
                    } else if loading {
                        plan_empty(
                            "Loading execution plan",
                            if mode == PlanMode::Analyze {
                                "Postgres is executing the statement and collecting timings."
                            } else {
                                "Postgres is estimating the selected statement."
                            },
                        )
                    } else {
                        match plan {
                            Some(Ok(plan)) => plan_content(&plan, stale),
                            Some(Err(error)) => plan_empty_owned("Plan failed", error, true),
                            None => plan_empty(
                                "No plan yet",
                                if mode == PlanMode::Analyze {
                                    "Run Analyze only when executing the selected SQL is acceptable."
                                } else {
                                    "Run Explain to inspect the estimated Postgres plan before executing SQL."
                                },
                            ),
                        }
                    }),
            )
            .into_any_element()
    }
}

fn mode_picker(tab_id: u64, mode: PlanMode, cx: &mut Context<CellarApp>) -> impl IntoElement {
    div()
        .h(px(23.))
        .flex()
        .rounded(px(4.))
        .border_1()
        .border_color(BORDER)
        .bg(PANEL)
        .p(px(1.))
        .children([
            mode_button(tab_id, "Estimate", PlanMode::Estimate, mode, cx),
            mode_button(tab_id, "Analyze", PlanMode::Analyze, mode, cx),
        ])
}

fn mode_button(
    tab_id: u64,
    label: &'static str,
    value: PlanMode,
    selected: PlanMode,
    cx: &mut Context<CellarApp>,
) -> AnyElement {
    div()
        .id(SharedString::from(format!("plan-mode:{label}")))
        .tab_index(0)
        .cursor_pointer()
        .h(px(19.))
        .flex()
        .items_center()
        .rounded(px(3.))
        .px_2()
        .text_size(px(11.5))
        .text_color(if value == selected { FG } else { FG_MUTED })
        .bg(if value == selected {
            PANEL_RAISED
        } else {
            PANEL
        })
        .on_click(cx.listener(move |this, _, _, cx| {
            this.plan_modes.insert(tab_id, value);
            cx.notify();
        }))
        .into_any_element()
}

fn plan_button(
    id: &'static str,
    icon: &'static str,
    label: &'static str,
    enabled: bool,
) -> gpui::Stateful<gpui::Div> {
    div()
        .id(id)
        .h(px(23.))
        .flex()
        .items_center()
        .gap(px(5.))
        .rounded(px(4.))
        .border_1()
        .border_color(BORDER)
        .bg(PANEL_RAISED)
        .px_2()
        .text_size(px(12.))
        .text_color(FG_SECONDARY)
        .opacity(if enabled { 1. } else { 0.5 })
        .when(enabled, |element| {
            element
                .tab_index(0)
                .cursor_pointer()
                .hover(|style| style.text_color(FG))
        })
        .child(Icon::empty().path(icon).size(px(10.)))
        .child(label)
}

fn analyze_warning() -> AnyElement {
    div()
        .flex()
        .items_start()
        .gap_2()
        .mb_2()
        .rounded(px(4.))
        .border_1()
        .border_color(WARN)
        .bg(WARN_SOFT)
        .px(px(10.))
        .py_2()
        .text_size(px(11.5))
        .text_color(FG_SECONDARY)
        .child(Icon::empty().path("icons/triangle-alert.svg").size(px(13.)).text_color(WARN))
        .child("EXPLAIN ANALYZE executes SQL. Writes can change data, take locks, or trigger side effects.")
        .into_any_element()
}

fn plan_content(plan: &QueryPlan, stale: bool) -> AnyElement {
    div()
        .flex()
        .flex_col()
        .gap_2()
        .child(div().flex().gap_2().children([
            metric(
                "mode",
                if plan.mode == PlanMode::Analyze {
                    "analyze".into()
                } else {
                    "estimate".into()
                },
                stale || plan.mode == PlanMode::Analyze,
            ),
            metric("planning", format_ms(plan.planning_time_ms), false),
            metric("execution", format_ms(plan.execution_time_ms), false),
            metric("round trip", format!("{} ms", plan.duration_ms), false),
        ]))
        .child(plan_node(&plan.root, plan.root.total_cost.unwrap_or(0.)))
        .into_any_element()
}

fn metric(label: &'static str, value: String, warn: bool) -> AnyElement {
    div()
        .flex_1()
        .rounded(px(4.))
        .border_1()
        .border_color(BORDER)
        .bg(PANEL)
        .px_2()
        .py_1()
        .child(
            div()
                .text_size(px(10.5))
                .text_color(FG_MUTED)
                .child(label.to_uppercase()),
        )
        .child(
            div()
                .font_family(cellar_desktop_gpui::theme::mono_font())
                .text_size(px(12.))
                .text_color(if warn { WARN } else { FG_SECONDARY })
                .child(value),
        )
        .into_any_element()
}

fn plan_node(node: &PlanNode, max_cost: f64) -> AnyElement {
    let relation = node.relation_name.as_ref().map(|name| {
        node.schema_name
            .as_ref()
            .map_or_else(|| name.clone(), |schema| format!("{schema}.{name}"))
    });
    let heat = if max_cost > 0. {
        node.total_cost.unwrap_or(0.) / max_cost
    } else {
        0.
    };
    div()
        .rounded(px(4.))
        .border_1()
        .border_color(BORDER)
        .bg(PANEL)
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .px_2()
                .py_1()
                .font_family(cellar_desktop_gpui::theme::mono_font())
                .child(div().text_color(FG).child(node.node_type.clone()))
                .when_some(relation, |element, relation| {
                    element.child(div().text_color(FG_SECONDARY).child(relation))
                })
                .when_some(node.index_name.clone(), |element, index| {
                    element.child(div().text_color(ACCENT).child(index))
                })
                .child(div().flex_1())
                .child(div().text_size(px(11.)).text_color(FG_MUTED).child(format!(
                    "cost {} · rows {} · {}%",
                    node.total_cost.map_or_else(|| "?".into(), |cost| format!("{cost:.2}")),
                    node.plan_rows.map_or_else(|| "?".into(), |rows| rows.to_string()),
                    (heat.clamp(0., 1.) * 100.).round() as u8,
                ))),
        )
        .when(
            node.actual_total_time_ms.is_some() || !node.details.is_empty(),
            |element| {
                element.child(
                    div()
                        .border_t_1()
                        .border_color(BORDER)
                        .px_2()
                        .py_1()
                        .font_family(cellar_desktop_gpui::theme::mono_font())
                        .text_size(px(11.5))
                        .text_color(FG_SECONDARY)
                        .when_some(node.actual_total_time_ms, |element, time| {
                            element.child(format!(
                                "actual {time:.2} ms · rows {} · loops {}",
                                node.actual_rows
                                    .map_or_else(|| "?".into(), |rows| rows.to_string()),
                                node.actual_loops
                                    .map_or_else(|| "?".into(), |loops| loops.to_string())
                            ))
                        })
                        .children(node.details.iter().map(|detail| {
                            div().child(format!("{}: {}", detail.label, detail.value))
                        })),
                )
            },
        )
        .when(!node.children.is_empty(), |element| {
            element.child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(6.))
                    .border_t_1()
                    .border_color(BORDER)
                    .p(px(6.))
                    .pl(px(16.))
                    .children(node.children.iter().map(|child| plan_node(child, max_cost))),
            )
        })
        .into_any_element()
}

fn plan_empty(title: &'static str, detail: &'static str) -> AnyElement {
    plan_empty_owned(title, detail.to_owned(), false)
}

fn plan_empty_owned(title: &'static str, detail: String, warn: bool) -> AnyElement {
    div()
        .flex_1()
        .min_h(px(120.))
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap(px(6.))
        .text_center()
        .child(
            div()
                .text_color(if warn { PROD } else { FG_SECONDARY })
                .child(title),
        )
        .child(
            div()
                .max_w(px(360.))
                .text_size(px(11.5))
                .text_color(FG_MUTED)
                .child(detail),
        )
        .into_any_element()
}

fn badge(label: &'static str, color: gpui::Rgba) -> AnyElement {
    div()
        .rounded(px(3.))
        .border_1()
        .border_color(color)
        .px(px(6.))
        .text_size(px(11.))
        .text_color(color)
        .child(label)
        .into_any_element()
}

fn format_ms(value: Option<f64>) -> String {
    value.map_or_else(|| "n/a".into(), |value| format!("{value:.2} ms"))
}

fn unavailable_detail(reason: &str) -> &'static str {
    match reason {
        "Execution plans are Postgres-only for now" => {
            "Other engines need their own typed plan renderer."
        }
        "Connection is not open" => "Connect the tab's database before requesting a plan.",
        _ => "Select a SQL statement to inspect its plan.",
    }
}

fn normalize_sql(sql: &str) -> &str {
    sql.trim().trim_end_matches(';').trim_end()
}
