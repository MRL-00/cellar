use std::rc::Rc;

use gpui::{AppContext, Context, Entity, Window};
use gpui_component::input::{InputEvent, InputState, Position, TabSize};

use super::{sql_completion::SqlCompletionProvider, CellarApp};
use cellar_core::schema::Database;
use cellar_desktop_gpui::model::{QueryTarget, SchemaCompareSource, TabKind, WorkspaceTab};

pub(super) fn preferred_database(databases: &[Database], fallback: &str) -> String {
    databases
        .iter()
        .find(|database| database.is_default)
        .or_else(|| databases.first())
        .map(|database| database.name.clone())
        .unwrap_or_else(|| fallback.to_owned())
}

pub(super) fn query_target_for_tab(tab: &WorkspaceTab) -> Option<QueryTarget> {
    match &tab.kind {
        TabKind::Table { target, .. } => Some(QueryTarget {
            connection_id: target.connection_id.clone(),
            database: target.database.clone(),
        }),
        TabKind::Query { target, .. } => Some(target.clone()),
        TabKind::ErDiagram { target, .. } => Some(QueryTarget {
            connection_id: target.connection_id.clone(),
            database: target.database.clone(),
        }),
        TabKind::SchemaCompare { config, .. } => [&config.source, &config.target]
            .into_iter()
            .find_map(|source| match source {
                SchemaCompareSource::Live {
                    connection_id,
                    database,
                    ..
                } => Some(QueryTarget {
                    connection_id: connection_id.clone(),
                    database: database.clone(),
                }),
                SchemaCompareSource::Snapshot { .. } => None,
            }),
    }
}

impl CellarApp {
    pub(super) fn install_query_editor(
        &mut self,
        tab_id: u64,
        editor: Entity<InputState>,
        cx: &mut Context<Self>,
    ) {
        let subscription = cx.subscribe(&editor, move |this, _, event, cx| {
            if is_query_content_change(event) {
                this.clear_query_error(tab_id, cx);
                this.query_params.remove(&tab_id);
                cx.notify();
            }
        });
        self.editors.insert(tab_id, editor);
        self.query_editor_subscriptions.insert(tab_id, subscription);
    }

    pub(super) fn build_query_editor(
        &self,
        target: &QueryTarget,
        sql: String,
        cursor: Option<Position>,
        soft_wrap: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Entity<InputState> {
        let completion = Rc::new(SqlCompletionProvider::new(
            self.model.databases(&target.connection_id),
            &target.database,
        ));
        let preferences = self.preferences.editor.clone();
        let editor = cx.new(|cx| {
            let mut editor = InputState::new(window, cx)
                .code_editor("sql")
                .line_number(preferences.line_numbers)
                .tab_size(TabSize {
                    tab_size: preferences.tab_size,
                    hard_tabs: false,
                })
                .placeholder("SELECT * FROM …")
                .default_value(sql);
            editor.lsp.completion_provider = Some(completion.clone());
            editor
        });
        editor.update(cx, |editor, cx| {
            editor.set_soft_wrap(soft_wrap, window, cx);
            if let Some(cursor) = cursor {
                editor.set_cursor_position(cursor, window, cx);
            }
        });
        editor
    }

    pub(super) fn apply_editor_tab_size(
        &mut self,
        tab_size: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.preferences.editor.tab_size = tab_size;
        let snapshots = self
            .model
            .tabs()
            .iter()
            .filter_map(|tab| {
                let TabKind::Query { target, .. } = &tab.kind else {
                    return None;
                };
                let editor = self.editors.get(&tab.id)?.read(cx);
                Some((
                    tab.id,
                    target.clone(),
                    editor.value().to_string(),
                    editor.cursor_position(),
                    self.query_wrap.get(&tab.id).copied().unwrap_or(true),
                ))
            })
            .collect::<Vec<_>>();
        // ponytail: the component has no runtime tab-size setter; rebuild only
        // when this rare preference changes, retaining text, caret and context.
        for (tab_id, target, sql, cursor, soft_wrap) in snapshots {
            let editor = self.build_query_editor(&target, sql, Some(cursor), soft_wrap, window, cx);
            self.install_query_editor(tab_id, editor, cx);
        }
        cx.notify();
    }
}

fn is_query_content_change(event: &InputEvent) -> bool {
    matches!(event, InputEvent::Change)
}

#[cfg(test)]
mod tests {
    use super::{
        is_query_content_change, preferred_database, query_target_for_tab, InputEvent, QueryTarget,
        TabKind, WorkspaceTab,
    };
    use cellar_core::schema::Database;
    use cellar_desktop_gpui::model::QueryState;

    #[test]
    fn only_content_changes_invalidate_query_ui() {
        assert!(is_query_content_change(&InputEvent::Change));
        assert!(!is_query_content_change(&InputEvent::Blur));
    }

    #[test]
    fn new_queries_inherit_the_active_tab_target() {
        let target = QueryTarget {
            connection_id: "primary".into(),
            database: "analytics".into(),
        };
        let tab = WorkspaceTab {
            id: 1,
            title: "untitled-1.sql".into(),
            pinned: false,
            kind: TabKind::Query {
                target: target.clone(),
                state: QueryState::Editing,
            },
        };
        assert_eq!(query_target_for_tab(&tab), Some(target));
    }

    #[test]
    fn connection_queries_prefer_the_loaded_default_database() {
        let databases = [
            Database {
                name: "first".into(),
                is_default: false,
                schemas: vec![],
            },
            Database {
                name: "default".into(),
                is_default: true,
                schemas: vec![],
            },
        ];
        assert_eq!(preferred_database(&databases, "configured"), "default");
        assert_eq!(preferred_database(&databases[..1], "configured"), "first");
        assert_eq!(preferred_database(&[], "configured"), "configured");
    }
}
