use std::{collections::VecDeque, sync::Arc, time::Instant};

use cellar_core::{
    driver::{ConnectionConfig, ConnectionSecrets},
    error::{CellarError, SshHostKey},
};
use cellar_desktop_gpui::model::AppModel;
use gpui::{Context, EntityId, Window};

use super::{
    confirm::{ConfirmAction, Confirmation},
    connection_editor::EditorBusy,
    CellarApp,
};

// Credentials stay in memory and are dropped with the cancelled/stale operation.
pub(super) struct EditorTest {
    pub(super) editor_id: EntityId,
    pub(super) config: ConnectionConfig,
    pub(super) secrets: ConnectionSecrets,
}

pub(super) enum SshRetry {
    Saved {
        id: String,
        revision: u64,
        reconnect: bool,
        refresh: bool,
    },
    Test(EditorTest),
}

impl SshRetry {
    fn is_current(&self, model: &AppModel, editor_id: Option<EntityId>) -> bool {
        match self {
            Self::Saved { id, revision, .. } => model.connection_revision_is_current(id, *revision),
            Self::Test(test) => editor_id == Some(test.editor_id),
        }
    }
}

pub(super) struct SshTrustPrompt {
    key: SshHostKey,
    retry: SshRetry,
}

impl SshTrustPrompt {
    pub(super) fn from_error(error: CellarError, retry: SshRetry) -> Option<Self> {
        match error {
            CellarError::SshHostKeyUnknown(key) => Some(Self { key, retry }),
            // Authentication includes changed/revoked keys. Never offer an override.
            _ => None,
        }
    }

    fn confirmation(self) -> Confirmation {
        Confirmation {
            title: "Unknown SSH host key".into(),
            message: format!(
                "Host: {}:{}\nAlgorithm: {}\nSHA256 fingerprint:\n{}\n\nVerify this fingerprint with your server administrator through a separate, trusted channel before continuing. Trusting saves this public key to your SSH known_hosts file. Cancel if you cannot verify it.",
                self.key.host, self.key.port, self.key.algorithm, self.key.fingerprint,
            ),
            confirm_label: match &self.retry {
                SshRetry::Saved { .. } => "Trust & Connect",
                SshRetry::Test(_) => "Trust & Test",
            },
            cancel_label: "Cancel",
            danger: false,
            action: ConfirmAction::TrustSsh(self),
        }
    }
}

fn next_current_prompt(
    pending: &mut VecDeque<SshTrustPrompt>,
    is_current: impl Fn(&SshRetry) -> bool,
) -> Option<SshTrustPrompt> {
    while let Some(prompt) = pending.pop_front() {
        if is_current(&prompt.retry) {
            return Some(prompt);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn retry(id: &str) -> SshRetry {
        SshRetry::Saved {
            id: id.into(),
            revision: 1,
            reconnect: false,
            refresh: true,
        }
    }

    fn unknown() -> CellarError {
        CellarError::SshHostKeyUnknown(SshHostKey {
            host: "jump.example".into(),
            port: 2222,
            algorithm: "ssh-ed25519".into(),
            fingerprint: "SHA256:public-fingerprint".into(),
            public_key: "public-key".into(),
        })
    }

    #[test]
    fn only_unknown_keys_offer_explicit_trust_with_original_retry_flags() {
        for error in [
            CellarError::Authentication("changed host key".into()),
            CellarError::Authentication("revoked host key".into()),
            CellarError::Connection("timeout".into()),
        ] {
            assert!(SshTrustPrompt::from_error(error, retry("saved")).is_none());
        }
        let confirmation = SshTrustPrompt::from_error(unknown(), retry("saved"))
            .unwrap()
            .confirmation();
        assert_eq!(confirmation.confirm_label, "Trust & Connect");
        assert_eq!(confirmation.cancel_label, "Cancel");
        for text in [
            "jump.example:2222",
            "ssh-ed25519",
            "SHA256:public-fingerprint",
            "server administrator",
            "separate, trusted channel",
        ] {
            assert!(confirmation.message.contains(text));
        }
        assert!(
            matches!(confirmation.action, ConfirmAction::TrustSsh(SshTrustPrompt {
            retry: SshRetry::Saved { ref id, revision: 1, reconnect: false, refresh: true }, ..
        }) if id == "saved")
        );
    }

    #[test]
    fn saved_challenges_expire_on_edit_credential_save_and_same_id_recreation() {
        use cellar_core::driver::{Engine, SslMode};
        let mut config = ConnectionConfig {
            id: "saved".into(),
            name: "saved".into(),
            engine: Engine::Postgres,
            host: "old.example".into(),
            port: 5432,
            database: "db".into(),
            user: "user".into(),
            ssl_mode: SslMode::Prefer,
            ssh: None,
            env_tag: None,
            application_name: None,
            color: None,
        };
        let mut model = AppModel::new(vec![config.clone()]);
        for change in ["host", "credentials", "delete/readd"] {
            let operation = SshRetry::Saved {
                id: config.id.clone(),
                revision: model.connection_revision(&config.id).unwrap(),
                reconnect: true,
                refresh: false,
            };
            assert!(operation.is_current(&model, None));
            let mut queue =
                VecDeque::from([SshTrustPrompt::from_error(unknown(), operation).unwrap()]);
            match change {
                "host" => config.host = "new.example".into(),
                "credentials" => {} // Credential-only save leaves metadata unchanged.
                _ => model.remove_connection(&config.id),
            }
            model.upsert_connection(config.clone());
            assert!(!queue.front().unwrap().retry.is_current(&model, None));
            assert!(
                next_current_prompt(&mut queue, |retry| retry.is_current(&model, None)).is_none()
            );
        }
    }

    #[test]
    fn pending_challenges_are_fifo_and_stale_operations_are_dropped() {
        let mut queue = ["closed", "jump", "server"]
            .into_iter()
            .map(|id| SshTrustPrompt::from_error(unknown(), retry(id)).unwrap())
            .collect();
        let current =
            |retry: &SshRetry| !matches!(retry, SshRetry::Saved { id, .. } if id == "closed");
        for expected in ["jump", "server"] {
            let prompt = next_current_prompt(&mut queue, current).unwrap();
            assert!(matches!(prompt.retry, SshRetry::Saved { id, .. } if id == expected));
        }
        assert!(next_current_prompt(&mut queue, current).is_none());
    }
}

impl CellarApp {
    pub(super) fn dismiss_connection_editor(&mut self) -> bool {
        let existed = self.connection_editor.take().is_some();
        self.pending_ssh_trust
            .retain(|prompt| !matches!(prompt.retry, SshRetry::Test(_)));
        if matches!(
            self.confirmation.as_ref().map(|c| &c.action),
            Some(ConfirmAction::TrustSsh(SshTrustPrompt {
                retry: SshRetry::Test(_),
                ..
            }))
        ) {
            self.confirmation = None;
        }
        existed
    }

    fn ssh_retry_is_current(&self, retry: &SshRetry) -> bool {
        retry.is_current(
            &self.model,
            self.connection_editor
                .as_ref()
                .map(|editor| editor.name.entity_id()),
        )
    }

    // FIFO trust challenges take priority over generic connection error modals,
    // but never replace a confirmation already on screen.
    pub(super) fn show_next_ssh_trust(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let mut pending = std::mem::take(&mut self.pending_ssh_trust);
        let prompt = next_current_prompt(&mut pending, |retry| self.ssh_retry_is_current(retry));
        self.pending_ssh_trust = pending;
        if let Some(prompt) = prompt {
            self.ask_confirmation(prompt.confirmation(), window, cx);
            true
        } else {
            false
        }
    }

    pub(super) fn resolve_ssh_trust(
        &mut self,
        prompt: SshTrustPrompt,
        confirmed: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.ssh_retry_is_current(&prompt.retry) {
            self.show_next_pending_connection_error(window, cx);
            return;
        }
        if !confirmed {
            self.finish_ssh_retry_error(
                &prompt.retry,
                "SSH host trust cancelled; no key was saved.".into(),
            );
            return;
        }
        let runtime = Arc::clone(&self.runtime);
        cx.spawn_in(window, async move |this, cx| {
            // The spawned task may start after another UI event saved or removed
            // this connection. Recheck before dispatching the blocking write.
            let current = this
                .update_in(cx, |this, window, cx| {
                    let current = this.ssh_retry_is_current(&prompt.retry);
                    if !current {
                        this.show_next_pending_connection_error(window, cx);
                    }
                    current
                })
                .unwrap_or(false);
            if !current {
                return;
            }
            let result = runtime
                .spawn_blocking(move || cellar_runtime::trust_ssh_host_key(&prompt.key))
                .await
                .map_err(|error| format!("SSH host trust task failed: {error}"))
                .and_then(|result| {
                    result.map_err(|error| format!("Could not trust SSH host: {error}"))
                });
            this.update_in(cx, |this, window, cx| {
                if this.ssh_retry_is_current(&prompt.retry) {
                    match result {
                        Ok(()) => match prompt.retry {
                            SshRetry::Saved {
                                id,
                                revision,
                                reconnect,
                                refresh,
                            } => {
                                if this.model.begin_reconnect(&id) {
                                    this.connect_and_introspect(
                                        id, revision, reconnect, refresh, window, cx,
                                    );
                                }
                            }
                            SshRetry::Test(test) => this.run_editor_test(test, window, cx),
                        },
                        Err(error) => {
                            this.finish_ssh_retry_error(&prompt.retry, error);
                            if let SshRetry::Saved { id, .. } = &prompt.retry {
                                this.show_connection_error(id, Some(window), cx);
                            }
                        }
                    }
                }
                this.show_next_pending_connection_error(window, cx);
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn finish_ssh_retry_error(&mut self, retry: &SshRetry, error: String) {
        match retry {
            SshRetry::Saved { id, .. } => self.model.finish_connect(id, Err(error)),
            SshRetry::Test(_) => {
                if let Some(editor) = &mut self.connection_editor {
                    editor.busy = None;
                    editor.message = Some(Err(error));
                }
            }
        }
    }

    pub(super) fn run_editor_test(
        &mut self,
        test: EditorTest,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self
            .connection_editor
            .as_ref()
            .is_some_and(|editor| editor.name.entity_id() == test.editor_id)
        {
            return;
        }
        self.connection_editor.as_mut().unwrap().busy = Some(EditorBusy::Testing);
        let registry = Arc::clone(&self.registry);
        let runtime = Arc::clone(&self.runtime);
        let started = Instant::now();
        cx.spawn_in(window, async move |this, cx| {
            let config = test.config.clone();
            let secrets = test.secrets.clone();
            let result = runtime
                .spawn(async move { registry.test_with_secrets(&config, secrets).await })
                .await
                .map_err(|error| {
                    CellarError::Internal(format!("connection test task failed: {error}"))
                })
                .and_then(|result| result);
            this.update_in(cx, |this, window, cx| {
                let Some(editor) = &mut this.connection_editor else {
                    return;
                };
                if editor.name.entity_id() != test.editor_id {
                    return;
                }
                match result {
                    Ok(info) => {
                        editor.busy = None;
                        editor.message = Some(Ok(format!(
                            "{} ms · {}",
                            started.elapsed().as_millis(),
                            super::shell_widgets::short_driver_version(&info.version)
                        )));
                    }
                    Err(error) => {
                        let message = error.to_string();
                        if let Some(prompt) =
                            SshTrustPrompt::from_error(error, SshRetry::Test(test))
                        {
                            editor.message =
                                Some(Err("Verify the SSH host key to continue this test.".into()));
                            this.pending_ssh_trust.push_back(prompt);
                        } else {
                            editor.busy = None;
                            editor.message = Some(Err(message));
                        }
                    }
                }
                this.show_next_pending_connection_error(window, cx);
                cx.notify();
            })
            .ok();
        })
        .detach();
        cx.notify();
    }
}
