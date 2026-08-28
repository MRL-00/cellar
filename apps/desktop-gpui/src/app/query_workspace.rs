use std::{sync::Arc, time::Instant};

use cellar_core::{
    driver::{Engine, EnvTag},
    error::CellarError,
    query::{PlanMode, Query, QueryParam, QueryResultPage, QueryResultSummary},
};
use cellar_runtime::history::NewQueryHistoryRecord;
use gpui::{div, prelude::*, AnyElement, Context, SharedString, Window};
use gpui_component::{
    highlighter::{Diagnostic, DiagnosticSeverity},
    input::{Input, InputState, Position},
    Icon,
};

use super::{
    query_control::required_confirmations,
    query_params::{infer_param_kind, parameter_value, ParamKind, QueryParameterInput},
    query_widgets::{first_line, query_ai_strip, query_keycap},
    CellarApp,
};
use cellar_desktop_gpui::{
    grid::DataGrid,
    model::{ConnectionState, QueryState, QueryTarget, TabKind, WorkspaceTab},
    theme::{ui_px, ACCENT, ACCENT_FG, BORDER, FG_MUTED, PANEL, PANEL_RAISED, PROD},
};

const QUERY_PAGE_SIZE: usize = 250;
const QUERY_ROW_LIMIT: u32 = 10_000;

enum QueryUiEvent {
    Page(QueryResultPage),
    Complete(Result<QueryResultSummary, String>),
}

impl CellarApp {
    pub(crate) fn new_query(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let target = self
            .model
            .active_tab()
            .and_then(super::query_editor::query_target_for_tab)
            .or_else(|| {
                self.model
                    .connections()
                    .iter()
                    .find(|config| {
                        self.model.connection_state(&config.id) == &ConnectionState::Connected
                    })
                    .or_else(|| self.model.connections().first())
                    .map(|config| QueryTarget {
                        connection_id: config.id.clone(),
                        database: config.database.clone(),
                    })
            });
        let Some(target) = target else {
            return;
        };
        self.open_query(target, String::new(), window, cx);
    }

    pub(super) fn open_query(
        &mut self,
        target: QueryTarget,
        sql: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> u64 {
        let tab_id = self.model.new_query(target.clone());
        let editor = self.build_query_editor(
            &target,
            sql,
            None,
            self.preferences.editor.soft_wrap,
            window,
            cx,
        );
        self.install_query_editor(tab_id, editor, cx);
        self.query_saved_sql.insert(tab_id, String::new());
        self.query_wrap
            .insert(tab_id, self.preferences.editor.soft_wrap);
        cx.notify();
        tab_id
    }

    pub(super) fn query_params_for_run(
        &mut self,
        tab_id: u64,
        target: &QueryTarget,
        engine: Engine,
        sql: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Result<Option<Vec<QueryParam>>, String> {
        let detected = cellar_sql::prepare(sql, engine)
            .map_err(|error| error.to_string())?
            .parameters;
        if detected.is_empty() {
            self.query_params.remove(&tab_id);
            return Ok(Some(Vec::new()));
        }
        if engine.family() != Engine::Postgres {
            return Err(format!(
                "bound query parameters are not supported for {} yet",
                engine.as_str()
            ));
        }
        let matches = self.query_params.get(&tab_id).is_some_and(|inputs| {
            inputs.len() == detected.len()
                && inputs
                    .iter()
                    .zip(&detected)
                    .all(|(input, parameter)| input.parameter == *parameter)
        });
        if !matches {
            let databases = self.model.databases(&target.connection_id);
            self.query_params.insert(
                tab_id,
                detected
                    .into_iter()
                    .map(|parameter| {
                        let kind =
                            infer_param_kind(&parameter, databases, Some(target.database.as_str()));
                        let initial = matches!(kind, ParamKind::Boolean)
                            .then_some("false")
                            .unwrap_or_default();
                        QueryParameterInput {
                            parameter,
                            kind,
                            state: cx.new(|cx| InputState::new(window, cx).default_value(initial)),
                        }
                    })
                    .collect(),
            );
            cx.notify();
            return Ok(None);
        }
        self.query_params
            .get(&tab_id)
            .expect("parameter inputs were checked above")
            .iter()
            .map(|input| {
                Ok(QueryParam {
                    name: input.parameter.name.clone(),
                    value: parameter_value(input, cx)?,
                })
            })
            .collect::<Result<Vec<_>, String>>()
            .map(Some)
    }

    pub(super) fn cycle_param_kind(
        &mut self,
        tab_id: u64,
        index: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(input) = self
            .query_params
            .get_mut(&tab_id)
            .and_then(|inputs| inputs.get_mut(index))
        else {
            return;
        };
        input.kind = input.kind.next();
        if input.kind == ParamKind::Boolean {
            input
                .state
                .update(cx, |state, cx| state.set_value("false", window, cx));
        }
        cx.notify();
    }

    pub(crate) fn run_active_query(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.save_template_editor.is_some() {
            self.save_query_template(cx);
            return;
        }
        if let Some(tab_id) = self
            .model
            .active_tab()
            .and_then(|tab| matches!(&tab.kind, TabKind::Query { .. }).then_some(tab.id))
        {
            self.start_query(tab_id, window, cx);
        }
    }

    pub(crate) fn run_active_query_all(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(tab_id) = self
            .model
            .active_tab()
            .and_then(|tab| matches!(&tab.kind, TabKind::Query { .. }).then_some(tab.id))
        {
            self.start_query_all(tab_id, window, cx);
        }
    }

    pub(super) fn start_query(&mut self, tab_id: u64, window: &mut Window, cx: &mut Context<Self>) {
        let Some(editor) = self.editors.get(&tab_id) else {
            return;
        };
        let editor = editor.read(cx);
        let buffer = editor.value().to_string();
        let statement = cellar_sql::statement_at_offset(&buffer, editor.cursor());
        let sql = statement
            .map(|statement| statement.text.to_owned())
            .unwrap_or_default();
        self.start_query_sql(
            tab_id,
            sql,
            statement.map(|statement| statement.start_line),
            window,
            cx,
        );
    }

    pub(super) fn start_query_all(
        &mut self,
        tab_id: u64,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let sql = self
            .editors
            .get(&tab_id)
            .map(|editor| editor.read(cx).value().to_string())
            .unwrap_or_default();
        let error_line = cellar_sql::split_statements(&sql)
            .first()
            .map(|statement| statement.start_line);
        self.start_query_sql(tab_id, sql, error_line, window, cx);
    }

    fn toggle_query_wrap(&mut self, tab_id: u64, window: &mut Window, cx: &mut Context<Self>) {
        let wrap = !self.query_wrap.get(&tab_id).copied().unwrap_or(true);
        self.query_wrap.insert(tab_id, wrap);
        if let Some(editor) = self.editors.get(&tab_id) {
            editor.update(cx, |editor, cx| editor.set_soft_wrap(wrap, window, cx));
        }
        cx.notify();
    }

    fn start_query_sql(
        &mut self,
        tab_id: u64,
        sql: String,
        error_line: Option<usize>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(target) = self.model.tabs().iter().find_map(|tab| {
            if tab.id != tab_id {
                return None;
            }
            match &tab.kind {
                TabKind::Query { target, .. } => Some(target.clone()),
                TabKind::Table { .. }
                | TabKind::ErDiagram { .. }
                | TabKind::SchemaCompare { .. } => None,
            }
        }) else {
            return;
        };
        if sql.trim().is_empty() {
            self.model
                .finish_query(tab_id, Err("Enter a SQL statement first".into()));
            cx.notify();
            return;
        }
        let Some((engine, production)) = self
            .model
            .connections()
            .iter()
            .find(|config| config.id == target.connection_id)
            .map(|config| (config.engine, config.env_tag == Some(EnvTag::Prod)))
        else {
            return;
        };
        let params = match self.query_params_for_run(tab_id, &target, engine, &sql, window, cx) {
            Ok(Some(params)) => params,
            Ok(None) => return,
            Err(error) => {
                self.model.finish_query(tab_id, Err(error));
                cx.notify();
                return;
            }
        };
        let destructive = cellar_sql::destructive_reason(&sql, engine);
        let required = required_confirmations(production, destructive.is_some());
        let completed = self
            .query_confirmations
            .get(&tab_id)
            .filter(|(armed_sql, _)| armed_sql == &sql)
            .map(|(_, completed)| *completed)
            .unwrap_or(0);
        if completed < required {
            self.query_confirmations
                .insert(tab_id, (sql.clone(), completed + 1));
            let warning = if completed == 0 {
                destructive.unwrap_or("execution on the PROD connection")
            } else {
                "the extra PROD confirmation"
            };
            self.model
                .finish_query(tab_id, Err(format!("Confirm {warning} before execution")));
            cx.notify();
            return;
        }
        self.query_confirmations.remove(&tab_id);
        if !self.model.begin_query(tab_id) {
            return;
        }
        self.bottom_panel_tab = super::shell::BottomPanelTab::Results;
        self.reveal_bottom_panel(true, cx);
        self.clear_query_error(tab_id, cx);
        let generation = self.query_generations.entry(tab_id).or_default();
        *generation += 1;
        let generation = *generation;

        self.grids.remove(&tab_id);
        self.query_summaries.remove(&tab_id);
        let registry = Arc::clone(&self.registry);
        let runtime = Arc::clone(&self.runtime);
        let (sender, receiver) = async_channel::bounded(2);
        let history = self.history.clone();
        let history_sql = sql.clone();
        let history_database = target.database.clone();
        let connection_id = target.connection_id.clone();
        let query = Query::new(sql)
            .with_max_rows(QUERY_ROW_LIMIT)
            .with_database(target.database)
            .with_query_id(format!("gpui-{tab_id}"))
            .with_params(params);
        runtime.spawn(async move {
            let history_context = registry.history_context(&connection_id).await;
            let started = Instant::now();
            let page_sender = sender.clone();
            let mut on_page = move |page| {
                page_sender
                    .send_blocking(QueryUiEvent::Page(page))
                    .map_err(|_| CellarError::query("query view closed"))
            };
            let result = registry
                .run_query_stream(&target.connection_id, query, QUERY_PAGE_SIZE, &mut on_page)
                .await
                .map_err(|error| error.to_string());
            if let Some(history) = history {
                let (success, duration_ms, row_count, truncated, error_summary) = match &result {
                    Ok(summary) => (
                        true,
                        summary.duration_ms as i64,
                        Some(summary.row_count.min(i64::MAX as u64) as i64),
                        summary.truncated,
                        None,
                    ),
                    Err(error) => (
                        false,
                        started.elapsed().as_millis().min(i64::MAX as u128) as i64,
                        None,
                        false,
                        Some(error.clone()),
                    ),
                };
                let _ = history
                    .insert(NewQueryHistoryRecord {
                        connection_id,
                        connection_name: history_context.name,
                        tab_id: Some(tab_id.to_string()),
                        database: Some(history_database),
                        sql: history_sql,
                        duration_ms,
                        success,
                        row_count,
                        truncated,
                        error_summary,
                    })
                    .await;
            }
            let _ = sender.send(QueryUiEvent::Complete(result)).await;
        });

        cx.spawn(async move |this, cx| {
            while let Ok(event) = receiver.recv().await {
                this.update(cx, |this, cx| {
                    if this.query_generations.get(&tab_id) != Some(&generation)
                        || !this.model.tabs().iter().any(|tab| tab.id == tab_id)
                    {
                        return;
                    }
                    let complete = matches!(&event, QueryUiEvent::Complete(_));
                    match event {
                        QueryUiEvent::Page(page) => {
                            let row_count = page.rows.len() as u64;
                            let result = if let Some(grid) = this.grids.get(&tab_id) {
                                grid.update(cx, |grid, cx| grid.append_page(page, cx))
                            } else {
                                let null_display = this.preferences.grid.null_display.clone();
                                let stripe_rows = this.preferences.grid.stripe_rows;
                                let grid = cx.new(|cx| {
                                    let mut grid = DataGrid::from_page(page, cx);
                                    grid.set_display_preferences(null_display, stripe_rows, cx);
                                    grid
                                });
                                if let Some(layout) = this.grid_layouts.get(&tab_id) {
                                    grid.update(cx, |grid, cx| grid.apply_layout(layout, cx));
                                }
                                this.grids.insert(tab_id, grid);
                                Ok(())
                            };
                            match result {
                                Ok(()) => this.model.receive_query_page(tab_id, row_count),
                                Err(error) => this.model.finish_query(tab_id, Err(error)),
                            }
                        }
                        QueryUiEvent::Complete(result) => match result {
                            Ok(mut summary) => {
                                if let Some(editor) = this.editors.get(&tab_id) {
                                    this.query_saved_sql
                                        .insert(tab_id, editor.read(cx).value().to_string());
                                }
                                this.last_query_metrics = Some((
                                    summary.row_count,
                                    summary.truncated,
                                    summary.duration_ms,
                                ));
                                if let Some(grid) = this.grids.get(&tab_id) {
                                    grid.update(cx, |grid, cx| grid.complete(&summary, cx));
                                }
                                this.model.finish_query(
                                    tab_id,
                                    Ok((summary.row_count, summary.duration_ms)),
                                );
                                if this.bottom_retain_notice_tabs.contains(&tab_id) {
                                    if let Some(previous) = this.query_summaries.get(&tab_id) {
                                        let mut retained = previous.notices.clone();
                                        retained.append(&mut summary.notices);
                                        summary.notices = retained;
                                    }
                                }
                                this.query_summaries.insert(tab_id, summary);
                            }
                            Err(error) => {
                                this.mark_query_error(tab_id, error_line, &error, cx);
                                this.model.finish_query(tab_id, Err(error));
                            }
                        },
                    }
                    if complete {
                        this.refresh_history(cx);
                    }
                    cx.notify();
                })
                .ok();
            }
        })
        .detach();
        cx.notify();
    }

    pub(super) fn clear_query_error(&self, tab_id: u64, cx: &mut Context<Self>) {
        if let Some(editor) = self.editors.get(&tab_id) {
            editor.update(cx, |editor, cx| {
                if let Some(diagnostics) = editor.diagnostics_mut() {
                    diagnostics.clear();
                    cx.notify();
                }
            });
        }
    }

    fn mark_query_error(
        &self,
        tab_id: u64,
        line: Option<usize>,
        message: &str,
        cx: &mut Context<Self>,
    ) {
        let (Some(editor), Some(line)) = (self.editors.get(&tab_id), line) else {
            return;
        };
        let message = message.to_owned();
        editor.update(cx, |editor, cx| {
            let row = line.saturating_sub(1);
            let width = editor
                .value()
                .lines()
                .nth(row)
                .map(str::chars)
                .map(Iterator::count)
                .unwrap_or(0);
            if let Some(diagnostics) = editor.diagnostics_mut() {
                diagnostics.clear();
                diagnostics.push(
                    Diagnostic::new(
                        Position::new(row as u32, 0)..Position::new(row as u32, width as u32),
                        message,
                    )
                    .with_source("Cellar")
                    .with_severity(DiagnosticSeverity::Error),
                );
                cx.notify();
            }
        });
    }

    pub(super) fn query_content(
        &self,
        tab: &WorkspaceTab,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let TabKind::Query { state, .. } = &tab.kind else {
            unreachable!("query_content called for a table tab");
        };
        let Some(editor) = self.editors.get(&tab.id) else {
            return div()
                .flex_1()
                .p_4()
                .text_color(PROD)
                .child("SQL editor state is unavailable")
                .into_any_element();
        };
        let tab_id = tab.id;
        let running = matches!(state, QueryState::Running { .. });
        let plan_loading = self.plan_loading.contains(&tab.id);
        let wrap = self.query_wrap.get(&tab.id).copied().unwrap_or(true);
        let editor_state = editor.read(cx);
        let buffer = editor_state.value().to_string();
        let can_save_template = !buffer.trim().is_empty();
        let sql = cellar_sql::statement_at_offset(&buffer, editor_state.cursor())
            .map(|statement| statement.text.to_owned())
            .unwrap_or_default();
        let parameter_inputs = self.query_params.get(&tab.id);
        let (can_cancel, production, engine) = match &tab.kind {
            TabKind::Query { target, .. } => self
                .model
                .connections()
                .iter()
                .find(|config| config.id == target.connection_id)
                .map(|config| {
                    (
                        config.engine.family() == Engine::Postgres,
                        config.env_tag == Some(EnvTag::Prod),
                        config.engine,
                    )
                })
                .unwrap_or((false, false, Engine::Postgres)),
            _ => unreachable!(),
        };
        let confirmation_steps = self
            .query_confirmations
            .get(&tab_id)
            .filter(|(armed_sql, _)| armed_sql == &sql)
            .map(|(_, steps)| *steps)
            .unwrap_or(0);
        let destructive = cellar_sql::destructive_reason(&sql, engine);
        let run_label = if running {
            "Running…".into()
        } else if confirmation_steps > 0 {
            if production && destructive.is_some() && confirmation_steps > 1 {
                "Final confirm on PROD".into()
            } else if let Some(reason) = destructive {
                format!("Confirm {reason}")
            } else {
                "Confirm run on PROD".into()
            }
        } else {
            "Run".into()
        };

        div()
            .flex_1()
            .min_h_0()
            .flex()
            .flex_col()
            .child(
                div()
                    .h(ui_px(30.))
                    .flex_shrink_0()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_2()
                    .px_2()
                    .bg(PANEL)
                    .border_b_1()
                    .border_color(BORDER)
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            .child(
                                div()
                                    .id(SharedString::from(format!("run-query:{tab_id}")))
                                    .h(ui_px(22.))
                                    .flex()
                                    .items_center()
                                    .gap(ui_px(5.))
                                    .px_2()
                                    .rounded(ui_px(4.))
                                    .border_1()
                                    .border_color(if confirmation_steps > 0 {
                                        PROD
                                    } else {
                                        ACCENT
                                    })
                                    .bg(if confirmation_steps > 0 { PROD } else { ACCENT })
                                    .text_color(ACCENT_FG)
                                    .opacity(if running { 0.4 } else { 1. })
                                    .child(
                                        Icon::empty().path("icons/play-small.svg").size(ui_px(11.)),
                                    )
                                    .child(run_label)
                                    .child(query_keycap("⌘⏎", true))
                                    .when(!running, |element| {
                                        element
                                            .tab_index(0)
                                            .cursor_pointer()
                                            .hover(move |style| {
                                                style.bg(cellar_desktop_gpui::theme::hover_bright(
                                                    if confirmation_steps > 0 {
                                                        PROD.rgba()
                                                    } else {
                                                        ACCENT.rgba()
                                                    },
                                                ))
                                            })
                                            .on_click(cx.listener(move |this, _, window, cx| {
                                                this.start_query(tab_id, window, cx);
                                            }))
                                    }),
                            )
                            .when(running && can_cancel, |element| {
                                element.child(
                                    div()
                                        .id(SharedString::from(format!("cancel-query:{tab_id}")))
                                        .tab_index(0)
                                        .cursor_pointer()
                                        .h(ui_px(22.))
                                        .flex()
                                        .items_center()
                                        .gap(ui_px(5.))
                                        .px_2()
                                        .rounded(ui_px(4.))
                                        .border_1()
                                        .border_color(BORDER)
                                        .child(
                                            Icon::empty().path("icons/stop.svg").size(ui_px(11.)),
                                        )
                                        .child("Cancel")
                                        .child(query_keycap("⌘.", false))
                                        .on_click(cx.listener(move |this, _, _, cx| {
                                            this.cancel_query(tab_id, cx)
                                        })),
                                )
                            })
                            .child(
                                div()
                                    .id(SharedString::from(format!("run-all-query:{tab_id}")))
                                    .h(ui_px(22.))
                                    .flex()
                                    .items_center()
                                    .gap(ui_px(5.))
                                    .px_2()
                                    .rounded(ui_px(4.))
                                    .border_1()
                                    .border_color(BORDER)
                                    .opacity(if running { 0.4 } else { 1. })
                                    .child(Icon::empty().path("icons/play.svg").size(ui_px(11.)))
                                    .child("Run all")
                                    .child(query_keycap("⌘⇧⏎", false))
                                    .when(!running, |element| {
                                        element.tab_index(0).cursor_pointer().on_click(cx.listener(
                                            move |this, _, window, cx| {
                                                this.start_query_all(tab_id, window, cx);
                                            },
                                        ))
                                    }),
                            )
                            .child(div().mx_1().h(ui_px(16.)).w(ui_px(1.)).bg(BORDER))
                            .child(query_icon("format", "icons/format.svg", false))
                            .child(
                                query_icon("wrap", "icons/wrap.svg", true)
                                    .when(wrap, |element| {
                                        element.bg(PANEL_RAISED).text_color(ACCENT)
                                    })
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        this.toggle_query_wrap(tab_id, window, cx)
                                    })),
                            )
                            .child(query_icon("explain", "icons/tree.svg", !plan_loading).when(
                                !plan_loading,
                                |element| {
                                    element.on_click(cx.listener(move |this, _, window, cx| {
                                        this.bottom_panel_open = true;
                                        this.bottom_panel_tab = super::shell::BottomPanelTab::Plan;
                                        this.explain_query(tab_id, PlanMode::Estimate, window, cx);
                                    }))
                                },
                            ))
                            .child(
                                query_icon("save-template", "icons/star.svg", can_save_template)
                                    .when(can_save_template, |element| {
                                        element.on_click(cx.listener(move |this, _, window, cx| {
                                            this.open_save_template(tab_id, window, cx)
                                        }))
                                    }),
                            ),
                    )
                    .child(
                        div()
                            .min_w_0()
                            .flex()
                            .items_center()
                            .gap_2()
                            .text_size(ui_px(11.5))
                            .text_color(FG_MUTED)
                            .child(
                                div()
                                    .flex()
                                    .gap(ui_px(6.))
                                    .truncate()
                                    .child("statement under cursor:")
                                    .child(
                                        div()
                                            .truncate()
                                            .font_family(cellar_desktop_gpui::theme::mono_font())
                                            .text_color(cellar_desktop_gpui::theme::FG_SECONDARY)
                                            .child(first_line(&sql)),
                                    ),
                            )
                            .child(div().h(ui_px(16.)).w(ui_px(1.)).bg(BORDER))
                            .child(
                                div()
                                    .size(ui_px(6.))
                                    .rounded(ui_px(3.))
                                    .bg(super::shell_widgets::engine_color(engine)),
                            )
                            .child(super::shell_widgets::dialect_label(engine)),
                    ),
            )
            .when_some(parameter_inputs, |element, inputs| {
                element.child(self.query_parameter_panel(tab_id, inputs, cx))
            })
            .child(
                div()
                    .relative()
                    .flex_1()
                    .min_h_0()
                    .border_b_1()
                    .border_color(BORDER)
                    .child(
                        Input::new(editor)
                            .h_full()
                            .appearance(false)
                            .line_height(ui_px(21.7)),
                    )
                    .child(query_ai_strip()),
            )
            .into_any_element()
    }
}

fn query_icon(id: &'static str, path: &'static str, enabled: bool) -> gpui::Stateful<gpui::Div> {
    div()
        .id(SharedString::from(format!("query-toolbar:{id}")))
        .size(ui_px(22.))
        .flex()
        .items_center()
        .justify_center()
        .rounded(ui_px(4.))
        .text_color(FG_MUTED)
        .opacity(if enabled { 1. } else { 0.4 })
        .when(enabled, |element| {
            element
                .tab_index(0)
                .cursor_pointer()
                .hover(|style| style.bg(PANEL_RAISED))
        })
        .child(Icon::empty().path(path).size(ui_px(12.)))
}
