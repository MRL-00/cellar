use std::sync::Arc;

use gpui::{Context, Window};

use super::CellarApp;

impl CellarApp {
    pub(super) fn start_connect(
        &mut self,
        id: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.model.begin_connect(&id) {
            cx.notify();
            return;
        }
        self.connect_and_introspect(id, false, false, window, cx);
    }

    pub(super) fn reconnect(&mut self, id: String, window: &mut Window, cx: &mut Context<Self>) {
        if !self.model.begin_reconnect(&id) {
            return;
        }
        self.connect_and_introspect(id, true, true, window, cx);
    }

    pub(super) fn refresh_schema(
        &mut self,
        id: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.model.begin_reconnect(&id) {
            return;
        }
        self.connect_and_introspect(id, false, true, window, cx);
    }

    fn connect_and_introspect(
        &mut self,
        id: String,
        reconnect: bool,
        refresh: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let registry = Arc::clone(&self.registry);
        let runtime = Arc::clone(&self.runtime);
        cx.notify();
        cx.spawn_in(window, async move |this, cx| {
            let task_id = id.clone();
            let result = runtime
                .spawn(async move {
                    let info = if reconnect {
                        registry.reconnect_saved(&task_id).await?
                    } else {
                        registry.connect_saved(&task_id).await?
                    };
                    registry
                        .introspect(&task_id, refresh)
                        .await
                        .map(|databases| (info, databases))
                })
                .await
                .map_err(|error| format!("connection task failed: {error}"))
                .and_then(|result| result.map_err(|error| error.to_string()));
            this.update_in(cx, |this, window, cx| {
                let connected = result.is_ok();
                match result {
                    Ok((info, databases)) => {
                        this.driver_infos.insert(id.clone(), info);
                        this.model.finish_connect(&id, Ok(databases));
                    }
                    Err(error) => {
                        this.driver_infos.remove(&id);
                        this.model.finish_connect(&id, Err(error));
                        this.show_connection_error(&id, Some(window), cx);
                    }
                }
                if connected {
                    this.resume_table_loads(&id, cx);
                    this.show_next_pending_connection_error(window, cx);
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    pub(super) fn disconnect(&mut self, id: String, window: &mut Window, cx: &mut Context<Self>) {
        if !self.model.begin_disconnect(&id) {
            return;
        }
        let registry = Arc::clone(&self.registry);
        let runtime = Arc::clone(&self.runtime);
        cx.spawn_in(window, async move |this, cx| {
            let task_id = id.clone();
            let result = runtime
                .spawn(async move { registry.disconnect(&task_id).await })
                .await
                .map_err(|error| format!("disconnect task failed: {error}"))
                .and_then(|result| result.map_err(|error| error.to_string()));
            this.update_in(cx, |this, window, cx| {
                match result {
                    Ok(()) => {
                        this.driver_infos.remove(&id);
                        this.model.finish_disconnect(&id)
                    }
                    Err(error) => {
                        this.driver_infos.remove(&id);
                        this.model.finish_connect(&id, Err(error));
                        this.show_connection_error(&id, Some(window), cx);
                    }
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
        cx.notify();
    }
}
