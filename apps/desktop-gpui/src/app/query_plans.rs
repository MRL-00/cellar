use std::sync::Arc;

use cellar_core::{
    driver::EnvTag,
    query::{PlanMode, Query},
};
use gpui::{Context, Window};

use super::{
    confirm::{ConfirmAction, Confirmation},
    query_control::required_analyze_confirmations,
    CellarApp,
};
use cellar_desktop_gpui::model::TabKind;

impl CellarApp {
    pub(super) fn explain_query(
        &mut self,
        tab_id: u64,
        mode: PlanMode,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some((target, sql)) = self.model.tabs().iter().find_map(|tab| {
            if tab.id != tab_id {
                return None;
            }
            let TabKind::Query { target, .. } = &tab.kind else {
                return None;
            };
            self.editors.get(&tab_id).map(|editor| {
                let editor = editor.read(cx);
                let buffer = editor.value().to_string();
                let sql = cellar_sql::statement_at_offset(&buffer, editor.cursor())
                    .map(|statement| statement.text.to_owned())
                    .unwrap_or_default();
                (target.clone(), sql)
            })
        }) else {
            return;
        };
        if sql.trim().is_empty() || !self.plan_loading.insert(tab_id) {
            return;
        }
        let Some((engine, production)) = self
            .model
            .connections()
            .iter()
            .find(|config| config.id == target.connection_id)
            .map(|config| (config.engine, config.env_tag == Some(EnvTag::Prod)))
        else {
            self.plan_loading.remove(&tab_id);
            return;
        };
        let params = match self.query_params_for_run(tab_id, &target, engine, &sql, window, cx) {
            Ok(Some(params)) => params,
            Ok(None) => {
                self.plan_loading.remove(&tab_id);
                return;
            }
            Err(error) => {
                self.plan_loading.remove(&tab_id);
                self.query_plans.insert(tab_id, Err(error));
                cx.notify();
                return;
            }
        };
        if mode == PlanMode::Analyze {
            let required = required_analyze_confirmations(
                production,
                cellar_sql::destructive_reason(&sql, engine).is_some(),
            );
            let completed = self
                .analyze_confirmations
                .get(&tab_id)
                .filter(|(armed_sql, _)| armed_sql == &sql)
                .map(|(_, completed)| *completed)
                .unwrap_or(0);
            if completed < required {
                self.analyze_confirmations
                    .insert(tab_id, (sql, completed + 1));
                self.plan_loading.remove(&tab_id);
                self.ask_confirmation(
                    Confirmation {
                        title: "Run EXPLAIN ANALYZE?".into(),
                        message: "Postgres will execute the SQL. Writes can change data, and long statements can take locks.".into(),
                        confirm_label: "Run",
                        cancel_label: "Cancel",
                        danger: false,
                        action: ConfirmAction::Analyze(tab_id),
                    },
                    window,
                    cx,
                );
                return;
            }
            self.analyze_confirmations.remove(&tab_id);
        }
        self.query_plans.remove(&tab_id);
        let registry = Arc::clone(&self.registry);
        let runtime = Arc::clone(&self.runtime);
        cx.spawn(async move |this, cx| {
            let result = runtime
                .spawn(async move {
                    registry
                        .explain_query(
                            &target.connection_id,
                            Query::new(sql)
                                .with_database(target.database)
                                .with_params(params),
                            mode,
                        )
                        .await
                        .map_err(|error| error.to_string())
                })
                .await
                .map_err(|error| format!("plan task failed: {error}"))
                .and_then(|result| result);
            this.update(cx, |this, cx| {
                this.plan_loading.remove(&tab_id);
                this.query_plans.insert(tab_id, result);
                cx.notify();
            })
            .ok();
        })
        .detach();
        cx.notify();
    }
}
