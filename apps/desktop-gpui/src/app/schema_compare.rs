use std::{collections::HashSet, sync::Arc, time::Instant};

use cellar_core::driver::{Engine, EnvTag};
use cellar_runtime::history::NewQueryHistoryRecord;
use cellar_schema_diff::{assemble_script, MigrationStatement, SchemaComparison};
use gpui::{div, prelude::*, px, AnyElement, ClipboardItem, Context, Entity, SharedString, Window};
use gpui_component::{input::Input, input::InputState, Icon};

use super::{schema_compare_support::*, CellarApp};
use cellar_desktop_gpui::{
    model::{SchemaCompareConfig, SchemaCompareSource, SchemaCompareState, TabKind, WorkspaceTab},
    theme::{ACCENT, BG, BORDER, FG_MUTED, INSERT, INSET, PANEL, PANEL_MUTED, PROD, WARN},
};

pub(super) struct SchemaCompareWorkspace {
    pub(super) comparison: Option<SchemaComparison>,
    pub(super) selected: HashSet<String>,
    pub(super) wrap: bool,
    pub(super) generated_sql: String,
    pub(super) editor: Entity<InputState>,
    pub(super) expanded: HashSet<String>,
    pub(super) show_unchanged: bool,
    pub(super) applying: bool,
    pub(super) confirming: bool,
    pub(super) message: Option<String>,
}

impl SchemaCompareWorkspace {
    fn new(editor: Entity<InputState>) -> Self {
        Self {
            comparison: None,
            selected: HashSet::new(),
            wrap: true,
            generated_sql: String::new(),
            editor,
            expanded: HashSet::new(),
            show_unchanged: false,
            applying: false,
            confirming: false,
            message: None,
        }
    }

    fn selected_statements(&self) -> Vec<MigrationStatement> {
        self.comparison
            .as_ref()
            .map(|comparison| {
                comparison
                    .statements
                    .iter()
                    .filter(|statement| self.selected.contains(&statement.id))
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }
}

impl CellarApp {
    pub(super) fn open_schema_compare(
        &mut self,
        config: SchemaCompareConfig,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> u64 {
        let (tab_id, opened) = self.model.open_schema_compare(config.clone());
        if opened || !self.schema_compares.contains_key(&tab_id) {
            let editor = cx.new(|cx| {
                InputState::new(window, cx)
                    .code_editor("sql")
                    .line_number(true)
                    .placeholder("Select migration statements above.")
            });
            self.schema_compares
                .insert(tab_id, SchemaCompareWorkspace::new(editor));
            self.load_schema_compare(tab_id, config, window, cx);
        }
        cx.notify();
        tab_id
    }

    fn load_schema_compare(
        &mut self,
        tab_id: u64,
        config: SchemaCompareConfig,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.model.start_schema_compare(tab_id);
        if let Some(workspace) = self.schema_compares.get_mut(&tab_id) {
            workspace.message = None;
            workspace.confirming = false;
        }
        let registry = Arc::clone(&self.registry);
        let runtime = Arc::clone(&self.runtime);
        let window_handle = window.window_handle();
        cx.spawn(async move |this, cx| {
            let task_config = config.clone();
            let result = runtime
                .spawn(async move { compare_config(&registry, &task_config).await })
                .await
                .map_err(|error| format!("schema comparison task failed: {error}"))
                .and_then(|result| result);
            let _ = cx.update_window(window_handle, |view, window, cx| {
                let Ok(app) = view.downcast::<CellarApp>() else {
                    return;
                };
                app.update(cx, |app, cx| {
                    app.install_schema_comparison(tab_id, result, window, cx);
                });
            });
            drop(this);
        })
        .detach();
    }

    fn install_schema_comparison(
        &mut self,
        tab_id: u64,
        result: Result<SchemaComparison, String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match result {
            Ok(comparison) => {
                let selected = comparison
                    .statements
                    .iter()
                    .map(|statement| statement.id.clone())
                    .collect::<HashSet<_>>();
                let sql = assemble_script(&comparison.statements, comparison.dialect, true);
                if let Some(workspace) = self.schema_compares.get_mut(&tab_id) {
                    workspace.expanded = comparison
                        .diff
                        .tables
                        .iter()
                        .filter(|table| table.status.is_change())
                        .map(|table| table.name.clone())
                        .collect();
                    workspace.selected = selected;
                    workspace.generated_sql = sql.clone();
                    workspace.comparison = Some(comparison);
                    workspace.applying = false;
                    workspace.confirming = false;
                    workspace.message = None;
                    workspace
                        .editor
                        .update(cx, |editor, cx| editor.set_value(sql, window, cx));
                }
                self.model.finish_schema_compare(tab_id, Ok(()));
            }
            Err(error) => {
                self.model.finish_schema_compare(tab_id, Err(error.clone()));
                if let Some(workspace) = self.schema_compares.get_mut(&tab_id) {
                    workspace.applying = false;
                    workspace.message = Some(error);
                }
            }
        }
        cx.notify();
    }

    fn regenerate_schema_sql(&mut self, tab_id: u64, window: &mut Window, cx: &mut Context<Self>) {
        let Some(workspace) = self.schema_compares.get_mut(&tab_id) else {
            return;
        };
        let Some(comparison) = workspace.comparison.as_ref() else {
            return;
        };
        let statements = comparison
            .statements
            .iter()
            .filter(|statement| workspace.selected.contains(&statement.id))
            .cloned()
            .collect::<Vec<_>>();
        let sql = assemble_script(&statements, comparison.dialect, workspace.wrap);
        workspace.generated_sql = sql.clone();
        workspace
            .editor
            .update(cx, |editor, cx| editor.set_value(sql, window, cx));
        workspace.confirming = false;
        workspace.message = None;
        cx.notify();
    }

    pub(super) fn toggle_schema_statement(
        &mut self,
        tab_id: u64,
        statement_id: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let clean = self.schema_compares.get(&tab_id).is_some_and(|workspace| {
            workspace.editor.read(cx).value().as_ref() == workspace.generated_sql
        });
        if let Some(workspace) = self.schema_compares.get_mut(&tab_id) {
            if !workspace.selected.remove(&statement_id) {
                workspace.selected.insert(statement_id);
            }
        }
        if clean {
            self.regenerate_schema_sql(tab_id, window, cx);
        } else {
            cx.notify();
        }
    }

    fn select_schema_statements(
        &mut self,
        tab_id: u64,
        all: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let clean = self.schema_compares.get(&tab_id).is_some_and(|workspace| {
            workspace.editor.read(cx).value().as_ref() == workspace.generated_sql
        });
        if let Some(workspace) = self.schema_compares.get_mut(&tab_id) {
            workspace.selected = if all {
                workspace
                    .comparison
                    .as_ref()
                    .map(|comparison| {
                        comparison
                            .statements
                            .iter()
                            .map(|statement| statement.id.clone())
                            .collect()
                    })
                    .unwrap_or_default()
            } else {
                HashSet::new()
            };
        }
        if clean {
            self.regenerate_schema_sql(tab_id, window, cx);
        } else {
            cx.notify();
        }
    }

    fn apply_schema_migration(
        &mut self,
        tab_id: u64,
        config: SchemaCompareConfig,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let SchemaCompareSource::Live {
            connection_id,
            database,
            ..
        } = &config.source
        else {
            if let Some(workspace) = self.schema_compares.get_mut(&tab_id) {
                workspace.message = Some("Apply needs a live source connection.".into());
            }
            cx.notify();
            return;
        };
        let Some(workspace) = self.schema_compares.get_mut(&tab_id) else {
            return;
        };
        let sql = workspace.editor.read(cx).value().to_string();
        let statements = workspace.selected_statements();
        if sql.trim().is_empty() || statements.is_empty() || workspace.applying {
            return;
        }
        let production = self
            .model
            .connections()
            .iter()
            .find(|connection| connection.id == *connection_id)
            .is_some_and(|connection| connection.env_tag == Some(EnvTag::Prod));
        let destructive = statements.iter().any(|statement| statement.destructive)
            || cellar_sql::destructive_reason(&sql, Engine::Postgres).is_some();
        if (production || destructive) && !workspace.confirming {
            workspace.confirming = true;
            workspace.message = Some(format!(
                "Apply {}{}?",
                if destructive {
                    "destructive migration"
                } else {
                    "migration"
                },
                if production { " to PROD" } else { "" }
            ));
            cx.notify();
            return;
        }
        workspace.applying = true;
        workspace.confirming = false;
        workspace.message = None;

        let registry = Arc::clone(&self.registry);
        let runtime = Arc::clone(&self.runtime);
        let history = self.history.clone();
        let apply_connection = connection_id.clone();
        let apply_database = database.clone();
        let window_handle = window.window_handle();
        cx.spawn(async move |this, cx| {
            let task_config = config.clone();
            let result = runtime
                .spawn(async move {
                    let context = registry.history_context(&apply_connection).await;
                    let started = Instant::now();
                    let apply = registry
                        .apply_migration(&apply_connection, &apply_database, &sql)
                        .await;
                    let duration = apply
                        .as_ref()
                        .copied()
                        .unwrap_or_else(|_| started.elapsed().as_millis() as u64);
                    if let Some(history) = history {
                        let _ = history
                            .insert(NewQueryHistoryRecord {
                                connection_id: apply_connection,
                                connection_name: context.name,
                                tab_id: Some(format!("schema-compare-{tab_id}")),
                                database: Some(apply_database),
                                sql,
                                duration_ms: duration.min(i64::MAX as u64) as i64,
                                success: apply.is_ok(),
                                row_count: None,
                                truncated: false,
                                error_summary: apply.as_ref().err().map(ToString::to_string),
                            })
                            .await;
                    }
                    apply.map_err(|error| error.to_string())?;
                    compare_config(&registry, &task_config).await
                })
                .await
                .map_err(|error| format!("migration task failed: {error}"))
                .and_then(|result| result);
            let _ = cx.update_window(window_handle, |view, window, cx| {
                let Ok(app) = view.downcast::<CellarApp>() else {
                    return;
                };
                app.update(cx, |app, cx| {
                    app.install_schema_comparison(tab_id, result, window, cx);
                    app.refresh_history(cx);
                });
            });
            drop(this);
        })
        .detach();
        cx.notify();
    }

    pub(super) fn schema_compare_content(
        &self,
        tab: &WorkspaceTab,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let TabKind::SchemaCompare { config, state } = &tab.kind else {
            unreachable!("schema compare renderer requires compare tab");
        };
        let tab_id = tab.id;
        let Some(workspace) = self.schema_compares.get(&tab_id) else {
            return centered("Schema comparison state is unavailable.", true);
        };
        match state {
            SchemaCompareState::Loading => return centered("comparing schemas…", false),
            SchemaCompareState::Error(error) if workspace.comparison.is_none() => {
                let retry = config.clone();
                return div()
                    .flex_1()
                    .flex()
                    .flex_col()
                    .items_center()
                    .justify_center()
                    .gap_2()
                    .bg(INSET)
                    .text_color(PROD)
                    .child(error.clone())
                    .child(button("compare-retry", "Retry").on_click(cx.listener(
                        move |this, _, window, cx| {
                            this.load_schema_compare(tab_id, retry.clone(), window, cx)
                        },
                    )))
                    .into_any_element();
            }
            _ => {}
        }
        let Some(comparison) = workspace.comparison.as_ref() else {
            return centered("Comparison failed.", true);
        };
        let summary = &comparison.diff.summary;
        let added = summary.tables_added + summary.views_added;
        let removed = summary.tables_removed + summary.views_removed;
        let modified = summary.tables_modified + summary.views_modified;
        let refresh = config.clone();
        let source_live = matches!(config.source, SchemaCompareSource::Live { .. });
        let apply_supported = match &config.source {
            SchemaCompareSource::Live { connection_id, .. } => self
                .model
                .connections()
                .iter()
                .find(|connection| connection.id == *connection_id)
                .is_some_and(|connection| connection.engine.family() == Engine::Postgres),
            SchemaCompareSource::Snapshot { .. } => false,
        };
        let selected = workspace.selected.len();
        let total = comparison.statements.len();
        let sql = workspace.editor.read(cx).value().to_string();
        let dirty = sql != workspace.generated_sql;
        let can_apply =
            apply_supported && selected > 0 && !sql.trim().is_empty() && !workspace.applying;
        let apply = config.clone();

        div()
            .flex_1()
            .min_h_0()
            .flex()
            .flex_col()
            .bg(BG)
            .child(
                div()
                    .h(px(30.))
                    .flex_shrink_0()
                    .flex()
                    .items_center()
                    .gap_2()
                    .px_3()
                    .border_b_1()
                    .border_color(BORDER)
                    .bg(PANEL)
                    .child(Icon::empty().path("icons/diff.svg").size(px(13.)).text_color(ACCENT))
                    .child(div().font_weight(gpui::FontWeight::SEMIBOLD).child("Schema compare"))
                    .child(summary_text(added, "added", INSERT.rgba()))
                    .child(summary_text(removed, "dropped", PROD.rgba()))
                    .child(summary_text(modified, "changed", WARN.rgba()))
                    .when(added + removed + modified == 0, |element| {
                        element.child(div().text_color(FG_MUTED).child("schemas match"))
                    })
                    .child(div().flex_1())
                    .child(button("schema-recompare", "Recompare").on_click(cx.listener(
                        move |this, _, window, cx| {
                            this.load_schema_compare(tab_id, refresh.clone(), window, cx)
                        },
                    ))),
            )
            .child(self.schema_diff_tree(tab_id, comparison, workspace, cx))
            .child(
                div()
                    .id(SharedString::from(format!("schema-diff-scroll:{tab_id}")))
                    .flex_1()
                    .min_h_0()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .h(px(26.))
                            .flex_shrink_0()
                            .flex()
                            .items_center()
                            .justify_between()
                            .px_3()
                            .border_b_1()
                            .border_color(BORDER)
                            .bg(PANEL)
                            .text_size(px(11.))
                            .text_color(FG_MUTED)
                            .child(format!("INCLUDE IN MIGRATION · {selected}/{total}"))
                            .child(
                                div()
                                    .flex()
                                    .gap_2()
                                    .child(text_action("schema-select-all", "all").on_click(
                                        cx.listener(move |this, _, window, cx| {
                                            this.select_schema_statements(tab_id, true, window, cx)
                                        }),
                                    ))
                                    .child(text_action("schema-select-none", "none").on_click(
                                        cx.listener(move |this, _, window, cx| {
                                            this.select_schema_statements(tab_id, false, window, cx)
                                        }),
                                    )),
                            ),
                    )
                    .child(statement_list(tab_id, comparison, workspace, cx))
                    .child(
                        div()
                            .h(px(26.))
                            .flex_shrink_0()
                            .flex()
                            .items_center()
                            .justify_between()
                            .px_3()
                            .border_y_1()
                            .border_color(BORDER)
                            .bg(PANEL)
                            .text_size(px(11.))
                            .text_color(FG_MUTED)
                            .child(if dirty { "MIGRATION SCRIPT · EDITED" } else { "MIGRATION SCRIPT" })
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .child(
                                        text_action(
                                            "schema-wrap",
                                            if workspace.wrap { "☑ transaction" } else { "☐ transaction" },
                                        )
                                        .on_click(cx.listener(move |this, _, window, cx| {
                                            if let Some(workspace) = this.schema_compares.get_mut(&tab_id) {
                                                workspace.wrap = !workspace.wrap;
                                            }
                                            this.regenerate_schema_sql(tab_id, window, cx);
                                        })),
                                    )
                                    .child(button("schema-regenerate", "Regenerate").on_click(
                                        cx.listener(move |this, _, window, cx| {
                                            this.regenerate_schema_sql(tab_id, window, cx)
                                        }),
                                    ))
                                    .child(button("schema-copy", "Copy").on_click(cx.listener(
                                        move |this, _, _, cx| {
                                            if let Some(workspace) = this.schema_compares.get(&tab_id) {
                                                cx.write_to_clipboard(ClipboardItem::new_string(
                                                    workspace.editor.read(cx).value().to_string(),
                                                ));
                                            }
                                        },
                                    ))),
                            ),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_h_0()
                            .child(Input::new(&workspace.editor).h_full().appearance(false)),
                    )
                    .child(
                        div()
                            .min_h(px(44.))
                            .flex_shrink_0()
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap_3()
                            .px_3()
                            .py_2()
                            .border_t_1()
                            .border_color(BORDER)
                            .bg(PANEL_MUTED)
                            .child(
                                div()
                                    .min_w_0()
                                    .flex_1()
                                    .truncate()
                                    .text_size(px(11.5))
                                    .text_color(if workspace.confirming { WARN } else { FG_MUTED })
                                    .child(workspace.message.clone().unwrap_or_else(|| {
                                        if apply_supported {
                                            "review the script above; it runs in a transaction where supported".into()
                                        } else if source_live {
                                            "apply is only supported for Postgres sources right now".into()
                                        } else {
                                            "apply needs a live source connection — snapshot sources are read-only".into()
                                        }
                                    })),
                            )
                            .child(
                                button(
                                    "schema-apply",
                                    if workspace.applying {
                                        "Applying…"
                                    } else if workspace.confirming {
                                        "Confirm apply"
                                    } else {
                                        "Apply migration"
                                    },
                                )
                                .opacity(if can_apply { 1. } else { 0.45 })
                                .when(can_apply, |element| {
                                    element.on_click(cx.listener(move |this, _, window, cx| {
                                        this.apply_schema_migration(tab_id, apply.clone(), window, cx)
                                    }))
                                }),
                            ),
                    ),
            )
            .into_any_element()
    }

    fn schema_diff_tree(
        &self,
        tab_id: u64,
        comparison: &SchemaComparison,
        workspace: &SchemaCompareWorkspace,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let diff = &comparison.diff;
        let hidden = diff.summary.tables_unchanged + diff.summary.views_unchanged;
        let show = workspace.show_unchanged;
        div()
            .flex_1()
            .min_h(px(100.))
            .flex()
            .flex_col()
            .border_b_1()
            .border_color(BORDER)
            .child(
                div()
                    .h(px(26.))
                    .flex_shrink_0()
                    .flex()
                    .items_center()
                    .gap_2()
                    .px_3()
                    .bg(PANEL)
                    .border_b_1()
                    .border_color(BORDER)
                    .text_size(px(11.))
                    .text_color(FG_MUTED)
                    .child(
                        div()
                            .flex_1()
                            .truncate()
                            .child(format!("SOURCE · {}", diff.source_label)),
                    )
                    .child(
                        div()
                            .flex_1()
                            .truncate()
                            .child(format!("TARGET · {}", diff.target_label)),
                    )
                    .when(hidden > 0, |element| {
                        element.child(
                            text_action(
                                "schema-show-unchanged",
                                if show {
                                    "☑ show unchanged"
                                } else {
                                    "☐ show unchanged"
                                },
                            )
                            .on_click(cx.listener(
                                move |this, _, _, cx| {
                                    if let Some(workspace) = this.schema_compares.get_mut(&tab_id) {
                                        workspace.show_unchanged = !workspace.show_unchanged;
                                    }
                                    cx.notify();
                                },
                            )),
                        )
                    }),
            )
            .child(
                div()
                    .id(SharedString::from(format!("schema-diff-objects:{tab_id}")))
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .py_1()
                    .children(
                        diff.tables
                            .iter()
                            .filter(|table| show || table.status.is_change())
                            .map(|table| table_row(tab_id, table, workspace, cx)),
                    )
                    .children(
                        diff.views
                            .iter()
                            .filter(|view| show || view.status.is_change())
                            .map(|view| {
                                div()
                                    .h(px(24.))
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .px_2()
                                    .pl(px(20.))
                                    .border_b_1()
                                    .border_color(BORDER)
                                    .child(status_badge(view.status))
                                    .child(
                                        Icon::empty()
                                            .path("icons/tree.svg")
                                            .size(px(11.))
                                            .text_color(FG_MUTED),
                                    )
                                    .child(
                                        div()
                                            .font_family(cellar_desktop_gpui::theme::mono_font())
                                            .child(view.name.clone()),
                                    )
                                    .child(
                                        div().text_size(px(11.)).text_color(FG_MUTED).child("view"),
                                    )
                            }),
                    ),
            )
            .into_any_element()
    }
}
