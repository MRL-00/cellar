use std::sync::Arc;

use cellar_runtime::history::{HistoryStore, QueryHistoryFilter, QueryHistoryRecord};
use gpui::{Context, Window};

use super::CellarApp;

impl CellarApp {
    pub(crate) fn load_history(&mut self, cx: &mut Context<Self>) {
        let runtime = Arc::clone(&self.runtime);
        cx.spawn(async move |this, cx| {
            let loaded = runtime
                .spawn(HistoryStore::load())
                .await
                .map_err(|error| error.to_string())
                .and_then(|result| result.map(Arc::new).map_err(|error| error.to_string()));
            this.update(cx, |this, cx| match loaded {
                Ok(history) => {
                    this.history = Some(history);
                    this.refresh_history(cx);
                }
                Err(error) => {
                    this.history_loading = false;
                    this.history_error = Some(error);
                    cx.notify();
                }
            })
            .ok();
        })
        .detach();
    }

    pub(super) fn refresh_history(&mut self, cx: &mut Context<Self>) {
        let Some(history) = self.history.clone() else {
            return;
        };
        if self.model.active_tab().is_none() {
            self.history_records.clear();
            self.history_loading = false;
            self.history_error = None;
            cx.notify();
            return;
        }
        let (connection_id, database, tab_id) = self.model.active_tab().map_or_else(
            || (None, None, None),
            |tab| {
                let (connection_id, database) = match &tab.kind {
                    cellar_desktop_gpui::model::TabKind::Table { target, .. } => (
                        Some(target.connection_id.clone()),
                        Some(target.database.clone()),
                    ),
                    cellar_desktop_gpui::model::TabKind::Query { target, .. } => (
                        Some(target.connection_id.clone()),
                        Some(target.database.clone()),
                    ),
                    cellar_desktop_gpui::model::TabKind::ErDiagram { target, .. } => (
                        Some(target.connection_id.clone()),
                        Some(target.database.clone()),
                    ),
                    cellar_desktop_gpui::model::TabKind::SchemaCompare { .. } => (None, None),
                };
                (connection_id, database, Some(tab.id.to_string()))
            },
        );
        let search = self
            .bottom_history_search
            .read(cx)
            .value()
            .trim()
            .to_owned();
        let runtime = Arc::clone(&self.runtime);
        self.history_generation += 1;
        let generation = self.history_generation;
        self.history_loading = true;
        self.history_error = None;
        cx.notify();
        cx.spawn(async move |this, cx| {
            let records = runtime
                .spawn(async move {
                    history
                        .list(QueryHistoryFilter {
                            connection_id,
                            database,
                            tab_id,
                            search: (!search.is_empty()).then_some(search),
                            limit: Some(100),
                            ..Default::default()
                        })
                        .await
                })
                .await
                .map_err(|error| error.to_string())
                .and_then(|result| result.map_err(|error| error.to_string()));
            this.update(cx, |this, cx| {
                if this.history_generation != generation {
                    return;
                }
                this.history_loading = false;
                match records {
                    Ok(records) => this.history_records = records,
                    Err(error) => {
                        this.history_records.clear();
                        this.history_error = Some(error);
                    }
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    pub(super) fn reuse_history(
        &mut self,
        record: QueryHistoryRecord,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.model.select_connection(&record.connection_id) {
            return;
        }
        let Some(config) = self.model.active_connection() else {
            return;
        };
        self.open_query(
            cellar_desktop_gpui::model::QueryTarget {
                connection_id: config.id.clone(),
                database: record.database.unwrap_or_else(|| config.database.clone()),
            },
            record.sql,
            window,
            cx,
        );
    }
}
