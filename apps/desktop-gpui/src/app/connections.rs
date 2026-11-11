use std::sync::Arc;

use super::ssh_trust::{SshRetry, SshTrustPrompt};
use cellar_core::error::CellarError;
use gpui::{Context, Window};

use super::CellarApp;
use cellar_desktop_gpui::model::ConnectionState;

/// Whether a superseded connect may close the runtime connection for its ID.
///
/// A stale attempt may have opened a connection (or tunnel) after save/delete
/// already disconnected, even when introspection later failed, and the runtime
/// would reuse it with the old config. Revisions reset the state to
/// `Disconnected`, so `Connecting`/`Connected`/`Disconnecting` here belong to a
/// newer attempt that owns the runtime connection and must not be torn down.
fn close_stale_connection(current: &ConnectionState) -> bool {
    matches!(
        current,
        ConnectionState::Disconnected | ConnectionState::Error(_)
    )
}

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
        let revision = self
            .model
            .connection_revision(&id)
            .expect("known connection");
        self.connect_and_introspect(id, revision, false, false, window, cx);
    }

    pub(super) fn reconnect(&mut self, id: String, window: &mut Window, cx: &mut Context<Self>) {
        if !self.model.begin_reconnect(&id) {
            return;
        }
        let revision = self
            .model
            .connection_revision(&id)
            .expect("known connection");
        self.connect_and_introspect(id, revision, true, true, window, cx);
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
        let revision = self
            .model
            .connection_revision(&id)
            .expect("known connection");
        self.connect_and_introspect(id, revision, false, true, window, cx);
    }

    pub(super) fn connect_and_introspect(
        &mut self,
        id: String,
        revision: u64,
        reconnect: bool,
        refresh: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.model.connection_revision_is_current(&id, revision) {
            return;
        }
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
                .map_err(|error| CellarError::Internal(format!("connection task failed: {error}")))
                .and_then(|result| result);
            this.update_in(cx, |this, window, cx| {
                if !this.model.connection_revision_is_current(&id, revision) {
                    if close_stale_connection(this.model.connection_state(&id)) {
                        let registry = Arc::clone(&this.registry);
                        let id = id.clone();
                        // Fire-and-forget: the runtime task outlives this UI
                        // update and `disconnect` has nothing to report here.
                        drop(this.runtime.spawn(async move {
                            let _ = registry.disconnect(&id).await;
                        }));
                    }
                    // A stale result must not stall prompts queued behind it.
                    this.show_next_pending_connection_error(window, cx);
                    return;
                }
                let connected = result.is_ok();
                match result {
                    Ok((info, databases)) => {
                        this.driver_infos.insert(id.clone(), info);
                        this.model.finish_connect(&id, Ok(databases));
                    }
                    Err(error) => {
                        this.driver_infos.remove(&id);
                        this.model.finish_connect(&id, Err(error.to_string()));
                        if let Some(prompt) = SshTrustPrompt::from_error(
                            error,
                            SshRetry::Saved {
                                id: id.clone(),
                                revision,
                                reconnect,
                                refresh,
                            },
                        ) {
                            this.pending_ssh_trust.push_back(prompt);
                            this.show_next_pending_connection_error(window, cx);
                        } else {
                            this.show_connection_error(&id, Some(window), cx);
                        }
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
        let revision = self
            .model
            .connection_revision(&id)
            .expect("known connection");
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
                if !this.model.connection_revision_is_current(&id, revision) {
                    return;
                }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stale_connect_closes_only_when_no_newer_attempt_owns_the_id() {
        assert!(close_stale_connection(&ConnectionState::Disconnected));
        assert!(close_stale_connection(&ConnectionState::Error(
            "failed".into()
        )));
        assert!(!close_stale_connection(&ConnectionState::Connecting));
        assert!(!close_stale_connection(&ConnectionState::Connected));
        assert!(!close_stale_connection(&ConnectionState::Disconnecting));
    }
}
