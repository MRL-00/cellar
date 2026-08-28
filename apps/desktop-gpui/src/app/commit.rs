use std::{sync::Arc, time::Instant};

use cellar_core::driver::{Engine, EnvTag};
use cellar_diff::{
    build_mssql_plan, build_postgres_plan, RowChange, TableChangeRequest, TableCommitPreview,
};
use cellar_runtime::history::NewQueryHistoryRecord;
use gpui::{div, prelude::*, px, AnyElement, ClipboardItem, Context};
use gpui_component::{scroll::ScrollableElement, Icon};

use super::CellarApp;
use cellar_desktop_gpui::theme::{
    accent_soft, ACCENT, ACCENT_FG, BORDER, FG, FG_MUTED, FG_SECONDARY, FG_TERTIARY, INSERT, INSET,
    PANEL, PANEL_MUTED, PANEL_RAISED, PROD, WARN,
};

pub(super) struct CommitReview {
    tab_id: u64,
    connection_id: String,
    request: TableChangeRequest,
    pub(super) preview: Option<TableCommitPreview>,
    import: bool,
    pub(super) committing: bool,
    error: Option<String>,
}

impl CommitReview {
    pub(super) fn tab_id(&self) -> u64 {
        self.tab_id
    }
}

impl CellarApp {
    pub(super) fn open_commit_review(
        &mut self,
        tab_id: u64,
        connection_id: String,
        request: TableChangeRequest,
        cx: &mut Context<Self>,
    ) {
        self.open_change_review(tab_id, connection_id, request, None, false, cx);
    }

    pub(super) fn open_import_review(
        &mut self,
        tab_id: u64,
        connection_id: String,
        request: TableChangeRequest,
        preview_request: TableChangeRequest,
        cx: &mut Context<Self>,
    ) {
        self.open_change_review(
            tab_id,
            connection_id,
            request,
            Some(preview_request),
            true,
            cx,
        );
    }

    fn open_change_review(
        &mut self,
        tab_id: u64,
        connection_id: String,
        request: TableChangeRequest,
        preview_request: Option<TableChangeRequest>,
        import: bool,
        cx: &mut Context<Self>,
    ) {
        let engine = self
            .model
            .connections()
            .iter()
            .find(|config| config.id == connection_id)
            .map(|config| config.engine.family());
        let plan_request = preview_request.as_ref().unwrap_or(&request);
        let plan = match engine {
            Some(Engine::Postgres) => build_postgres_plan(plan_request),
            Some(Engine::Mssql) => build_mssql_plan(plan_request),
            Some(engine) => {
                self.commit_review = Some(CommitReview {
                    tab_id,
                    connection_id,
                    request,
                    preview: None,
                    import,
                    committing: false,
                    error: Some(format!(
                        "{} does not support safe grid commits yet",
                        engine.as_str()
                    )),
                });
                cx.notify();
                return;
            }
            None => {
                self.commit_review = Some(CommitReview {
                    tab_id,
                    connection_id,
                    request,
                    preview: None,
                    import,
                    committing: false,
                    error: Some("Connection metadata is unavailable".into()),
                });
                cx.notify();
                return;
            }
        };
        let (preview, error) = match plan {
            Ok(mut plan) => {
                if import {
                    plan.preview.expected_rows = request.changes.len() as u64;
                    plan.preview.statement_count =
                        request.changes.len().min(u32::MAX as usize) as u32;
                }
                (Some(plan.preview), None)
            }
            Err(error) => (None, Some(error.to_string())),
        };
        self.commit_review = Some(CommitReview {
            tab_id,
            connection_id,
            request,
            preview,
            import,
            committing: false,
            error,
        });
        cx.notify();
    }

    fn dismiss_commit_review(&mut self, cx: &mut Context<Self>) {
        if self
            .commit_review
            .as_ref()
            .is_some_and(|review| review.committing)
        {
            return;
        }
        self.commit_review = None;
        cx.notify();
    }

    pub(super) fn start_commit(&mut self, cx: &mut Context<Self>) {
        let Some(review) = &mut self.commit_review else {
            return;
        };
        if review.committing || review.preview.is_none() {
            return;
        }
        review.committing = true;
        review.error = None;
        let tab_id = review.tab_id;
        let connection_id = review.connection_id.clone();
        let request = review.request.clone();
        let import = review.import;
        let history_sql = review
            .preview
            .as_ref()
            .map(|preview| preview.sql.clone())
            .unwrap_or_default();
        let history_database = request.database.clone();
        let history = self.history.clone();
        let registry = Arc::clone(&self.registry);
        let runtime = Arc::clone(&self.runtime);
        cx.notify();
        cx.spawn(async move |this, cx| {
            let result = runtime
                .spawn(async move {
                    let context = registry.history_context(&connection_id).await;
                    let started = Instant::now();
                    let result = if import {
                        registry.commit_table_import(&connection_id, request).await
                    } else {
                        registry.commit_table_changes(&connection_id, request).await
                    };
                    if let Some(history) = history {
                        let (success, sql, rows, error) = match &result {
                            Ok(result) => (
                                true,
                                result.sql.clone(),
                                Some(result.rows_affected.min(i64::MAX as u64) as i64),
                                None,
                            ),
                            Err(error) => (false, history_sql, None, Some(error.to_string())),
                        };
                        let duration_ms = result
                            .as_ref()
                            .map(|result| result.duration_ms)
                            .unwrap_or_else(|_| started.elapsed().as_millis() as u64)
                            .min(i64::MAX as u64) as i64;
                        let _ = history
                            .insert(NewQueryHistoryRecord {
                                connection_id,
                                connection_name: context.name,
                                tab_id: Some(tab_id.to_string()),
                                database: history_database.or(context.database),
                                sql,
                                duration_ms,
                                success,
                                row_count: rows,
                                truncated: false,
                                error_summary: error,
                            })
                            .await;
                    }
                    result
                })
                .await
                .map_err(|error| format!("commit task failed: {error}"))
                .and_then(|result| result.map_err(|error| error.to_string()));
            this.update(cx, |this, cx| {
                match result {
                    Ok(_) => {
                        if let Some(grid) = this.grids.get(&tab_id) {
                            grid.update(cx, |grid, cx| grid.clear_pending(cx));
                        }
                        this.commit_review = None;
                        this.reload_table(tab_id, cx);
                    }
                    Err(error) => {
                        if let Some(review) = &mut this.commit_review {
                            review.committing = false;
                            review.error = Some(error);
                        }
                    }
                }
                this.refresh_history(cx);
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    pub(super) fn commit_overlay(&self, cx: &mut Context<Self>) -> AnyElement {
        let review = self
            .commit_review
            .as_ref()
            .expect("commit overlay requires review state");
        let can_commit = review.preview.is_some() && !review.committing;
        let inserts = review
            .request
            .changes
            .iter()
            .filter(|change| matches!(change, RowChange::Insert { .. } | RowChange::Upsert { .. }))
            .count();
        let updates = review
            .request
            .changes
            .iter()
            .filter(|change| matches!(change, RowChange::Update { .. }))
            .count();
        let deletes = review
            .request
            .changes
            .iter()
            .filter(|change| matches!(change, RowChange::Delete { .. }))
            .count();
        let connection = self
            .model
            .connections()
            .iter()
            .find(|connection| connection.id == review.connection_id)
            .map(|connection| {
                format!(
                    "{}{}",
                    connection.name,
                    connection
                        .env_tag
                        .map(|tag| format!(" ({})", env_label(tag)))
                        .unwrap_or_default()
                )
            })
            .unwrap_or_else(|| "no connection".into());
        let target = format!(
            "{}.{} · {connection}",
            review.request.schema, review.request.table
        );
        let production = self
            .model
            .connections()
            .iter()
            .find(|connection| connection.id == review.connection_id)
            .is_some_and(|connection| connection.env_tag == Some(EnvTag::Prod));
        let sql_lines = review
            .preview
            .as_ref()
            .map(|preview| preview.sql.lines().map(str::to_owned).collect::<Vec<_>>())
            .unwrap_or_default();
        let sql = review
            .preview
            .as_ref()
            .map(|preview| preview.sql.clone())
            .unwrap_or_default();

        div()
            .id("commit-review-backdrop")
            .absolute()
            .inset_0()
            .flex()
            .items_start()
            .justify_center()
            .pt(gpui::relative(0.08))
            .bg(cellar_desktop_gpui::theme::overlay())
            .on_click(cx.listener(|this, _, _, cx| {
                if !this
                    .commit_review
                    .as_ref()
                    .is_some_and(|review| review.committing)
                {
                    this.commit_review = None;
                    cx.notify();
                }
            }))
            .child(
                div()
                    .id("commit-review-modal")
                    .w(px(880.))
                    .h(px(560.))
                    .max_h(gpui::relative(0.84))
                    .flex()
                    .flex_col()
                    .overflow_hidden()
                    .rounded(px(8.))
                    .bg(PANEL)
                    .border_1()
                    .border_color(BORDER)
                    .shadow_lg()
                    .on_click(|_, _, cx| cx.stop_propagation())
                    .child(
                        div()
                            .h(px(38.))
                            .flex_shrink_0()
                            .flex()
                            .items_center()
                            .gap_2()
                            .pl(px(14.))
                            .pr_2()
                            .border_b_1()
                            .border_color(BORDER)
                            .child(
                                Icon::empty()
                                    .path("icons/commit.svg")
                                    .size(px(14.))
                                    .text_color(ACCENT),
                            )
                            .child(
                                div()
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .child("Review & commit"),
                            )
                            .child(
                                div()
                                    .ml_1()
                                    .min_w_0()
                                    .truncate()
                                    .border_l_1()
                                    .border_color(BORDER)
                                    .pl(px(6.))
                                    .font_family(cellar_desktop_gpui::theme::mono_font())
                                    .text_size(px(12.))
                                    .text_color(FG_TERTIARY)
                                    .child(target),
                            )
                            .child(div().flex_1())
                            .child(
                                div()
                                    .id("close-commit-review")
                                    .tab_index(0)
                                    .cursor_pointer()
                                    .size(px(22.))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .rounded(px(4.))
                                    .child(
                                        Icon::empty()
                                            .path("icons/close.svg")
                                            .size(px(13.))
                                            .text_color(FG_MUTED),
                                    )
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.dismiss_commit_review(cx)
                                    })),
                            ),
                    )
                    .child(
                        div()
                            .flex_shrink_0()
                            .flex()
                            .items_center()
                            .gap_4()
                            .border_b_1()
                            .border_color(BORDER)
                            .bg(PANEL_MUTED)
                            .px_4()
                            .py(px(10.))
                            .child(summary_item(
                                "icons/plus.svg",
                                INSERT.rgba(),
                                inserts,
                                "insert",
                            ))
                            .child(summary_item(
                                "icons/diff.svg",
                                ACCENT.rgba(),
                                updates,
                                "update",
                            ))
                            .child(summary_item(
                                "icons/close.svg",
                                PROD.rgba(),
                                deletes,
                                "delete",
                            ))
                            .child(div().flex_1())
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(6.))
                                    .rounded(px(4.))
                                    .bg(INSET)
                                    .px_2()
                                    .py_1()
                                    .font_family(cellar_desktop_gpui::theme::mono_font())
                                    .text_size(px(11.5))
                                    .text_color(FG_TERTIARY)
                                    .child(Icon::empty().path("icons/bracket.svg").size(px(10.)))
                                    .child("BEGIN ... COMMIT - atomic"),
                            )
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_h_0()
                            .flex()
                            .child(
                                div()
                                    .w(px(320.))
                                    .flex_shrink_0()
                                    .flex()
                                    .flex_col()
                                    .border_r_1()
                                    .border_color(BORDER)
                                    .child(section_header(
                                        "Changes",
                                        Some(review.request.changes.len().to_string()),
                                    ))
                                    .child(
                                        div()
                                            .flex_1()
                                            .min_h_0()
                                            .overflow_y_scrollbar()
                                            .py(px(6.))
                                            .children(review.request.changes.iter().map(change_row)),
                                    ),
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .flex()
                                    .flex_col()
                                    .bg(INSET)
                                    .child(
                                        section_header("Generated SQL", None).child(
                                        div()
                                            .id("copy-commit-sql")
                                            .tab_index(0)
                                            .cursor_pointer()
                                                .ml_auto()
                                                .h(px(22.))
                                                .flex()
                                                .items_center()
                                                .gap_1()
                                                .rounded(px(4.))
                                                .border_1()
                                                .border_color(BORDER)
                                                .bg(PANEL_RAISED)
                                                .px_2()
                                                .child(
                                                    Icon::empty()
                                                        .path("icons/copy.svg")
                                                        .size(px(11.)),
                                                )
                                                .child("Copy")
                                                .on_click(cx.listener(move |_, _, _, cx| {
                                                    cx.write_to_clipboard(
                                                        ClipboardItem::new_string(sql.clone()),
                                                    )
                                                })),
                                        ),
                                    )
                                    .child(
                                        div()
                                            .id("commit-sql-preview")
                                            .flex_1()
                                            .min_h_0()
                                            .overflow_scroll()
                                            .py_2()
                                            .font_family(
                                                cellar_desktop_gpui::theme::mono_font(),
                                            )
                                            .children(sql_lines.into_iter().enumerate().map(
                                                |(index, line)| {
                                                    div()
                                                        .min_h(px(20.))
                                                        .flex()
                                                        .px_3()
                                                        .child(
                                                            div()
                                                                .w(px(28.))
                                                                .flex_shrink_0()
                                                                .pr(px(10.))
                                                                .flex()
                                                                .justify_end()
                                                                .text_size(px(11.))
                                                                .text_color(FG_MUTED)
                                                                .child((index + 1).to_string()),
                                                        )
                                                        .child(
                                                            div()
                                                                .whitespace_nowrap()
                                                                .child(line),
                                                        )
                                                },
                                            ))
                                            .when_some(review.error.clone(), |element, error| {
                                                element.child(
                                                    div().px_3().text_color(PROD).child(error),
                                                )
                                            }),
                                    ),
                            ),
                    )
                    .child(
                        div()
                            .min_h(px(44.))
                            .flex_shrink_0()
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap_3()
                            .border_t_1()
                            .border_color(BORDER)
                            .bg(PANEL_MUTED)
                            .px_3()
                            .py_2()
                            .child(
                                div()
                                    .min_w_0()
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .text_size(px(11.5))
                                    .text_color(if production { WARN } else { FG_TERTIARY })
                                    .child(
                                        Icon::empty()
                                            .path("icons/triangle-alert.svg")
                                            .size(px(10.)),
                                    )
                                    .child(if production { "prod" } else { "transaction" })
                                    .child(
                                        div()
                                            .text_color(FG_TERTIARY)
                                            .child(review.error.clone().unwrap_or_else(|| {
                                                "commits rollback on error and require the expected row count".into()
                                            })),
                                    ),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .child(
                                        review_button("cancel-commit", "", "Cancel", false, true)
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                this.dismiss_commit_review(cx)
                                            })),
                                    )
                                    .child(review_button(
                                        "save-commit-migration",
                                        "icons/undo.svg",
                                        "Save as migration",
                                        false,
                                        false,
                                    ))
                                    .child(
                                        review_button(
                                            "confirm-commit",
                                            "icons/commit.svg",
                                            if review.committing {
                                                "Committing..."
                                            } else {
                                                "Commit transaction   Return"
                                            },
                                            true,
                                            can_commit,
                                        )
                                        .when(can_commit, |button| {
                                            button.on_click(cx.listener(|this, _, _, cx| {
                                                this.start_commit(cx)
                                            }))
                                        }),
                                    ),
                            ),
                    ),
            )
            .into_any_element()
    }
}

fn summary_item(
    icon: &'static str,
    color: gpui::Rgba,
    count: usize,
    label: &'static str,
) -> AnyElement {
    div()
        .flex()
        .items_center()
        .gap(px(6.))
        .child(
            div()
                .size(px(22.))
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(4.))
                .bg(gpui::Rgba { a: 0.12, ..color })
                .text_color(color)
                .child(Icon::empty().path(icon).size(px(11.))),
        )
        .child(
            div()
                .font_family(cellar_desktop_gpui::theme::mono_font())
                .font_weight(gpui::FontWeight::SEMIBOLD)
                .text_color(color)
                .child(count.to_string()),
        )
        .child(format!("{label}{}", if count == 1 { "" } else { "s" }))
        .into_any_element()
}

fn section_header(label: &'static str, count: Option<String>) -> gpui::Div {
    div()
        .h(px(26.))
        .flex_shrink_0()
        .flex()
        .items_center()
        .gap(px(6.))
        .border_b_1()
        .border_color(BORDER)
        .bg(PANEL)
        .px_3()
        .text_size(px(11.))
        .font_weight(gpui::FontWeight::SEMIBOLD)
        .text_color(FG_MUTED)
        .child(label.to_uppercase())
        .when_some(count, |element, count| {
            element.child(
                div()
                    .rounded(px(8.))
                    .bg(PANEL_RAISED)
                    .px(px(6.))
                    .font_family(cellar_desktop_gpui::theme::mono_font())
                    .font_weight(gpui::FontWeight::NORMAL)
                    .text_color(FG_TERTIARY)
                    .child(count),
            )
        })
}

fn change_row(change: &RowChange) -> AnyElement {
    let (tag, row_id, assignments, color) = match change {
        RowChange::Update { row_id, edits, .. } => ("UPDATE", row_id, edits, ACCENT.rgba()),
        RowChange::Insert { row_id, values } => ("INSERT", row_id, values, INSERT.rgba()),
        RowChange::Delete { row_id, keys } => ("DELETE", row_id, keys, PROD.rgba()),
        RowChange::Upsert { row_id, values, .. } => ("UPSERT", row_id, values, INSERT.rgba()),
    };
    div()
        .px_3()
        .py_2()
        .border_b_1()
        .border_color(BORDER)
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(
                    div()
                        .rounded(px(3.))
                        .bg(gpui::Rgba { a: 0.12, ..color })
                        .px(px(5.))
                        .py(px(1.))
                        .font_family(cellar_desktop_gpui::theme::mono_font())
                        .text_size(px(10.))
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_color(color)
                        .child(tag),
                )
                .child(
                    div()
                        .min_w_0()
                        .truncate()
                        .font_family(cellar_desktop_gpui::theme::mono_font())
                        .font_weight(gpui::FontWeight::MEDIUM)
                        .text_color(FG)
                        .child(row_id.clone()),
                ),
        )
        .children(assignments.iter().take(4).map(|assignment| {
            div()
                .mt_1()
                .flex()
                .gap(px(5.))
                .pl_1()
                .font_family(cellar_desktop_gpui::theme::mono_font())
                .child(
                    div()
                        .w(px(90.))
                        .truncate()
                        .text_color(FG_TERTIARY)
                        .child(assignment.column.clone()),
                )
                .child(
                    div()
                        .min_w_0()
                        .flex_1()
                        .truncate()
                        .rounded(px(3.))
                        .bg(accent_soft())
                        .px(px(6.))
                        .text_color(ACCENT)
                        .child(
                            assignment
                                .value
                                .value
                                .clone()
                                .unwrap_or_else(|| "NULL".into()),
                        ),
                )
        }))
        .into_any_element()
}

fn review_button(
    id: &'static str,
    icon: &'static str,
    label: &'static str,
    danger: bool,
    enabled: bool,
) -> gpui::Stateful<gpui::Div> {
    div()
        .id(id)
        .h(px(26.))
        .flex()
        .items_center()
        .gap(px(5.))
        .rounded(px(4.))
        .border_1()
        .border_color(if danger { PROD } else { BORDER })
        .bg(if danger { PROD } else { PANEL_RAISED })
        .px(px(10.))
        .font_weight(gpui::FontWeight::MEDIUM)
        .text_color(if danger { ACCENT_FG } else { FG_SECONDARY })
        .opacity(if enabled { 1. } else { 0.6 })
        .when(enabled, |element| element.tab_index(0).cursor_pointer())
        .when(!icon.is_empty(), |element| {
            element.child(Icon::empty().path(icon).size(px(11.)))
        })
        .child(label)
}

fn env_label(tag: EnvTag) -> &'static str {
    match tag {
        EnvTag::Prod => "prod",
        EnvTag::Staging => "staging",
        EnvTag::Dev => "dev",
        EnvTag::Local => "local",
    }
}
