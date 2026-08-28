use std::{
    sync::Arc,
    time::{Instant, SystemTime, UNIX_EPOCH},
};

use cellar_core::driver::{ConnectionConfig, Engine, EnvTag, SslMode};
use gpui::{AppContext, Context, Entity, Window};
use gpui_component::input::InputState;

use super::{
    confirm::{ConfirmAction, Confirmation},
    connection_editor_support::*,
    CellarApp,
};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum ConnectionTab {
    General,
    Ssh,
    Ssl,
    Options,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum EditorBusy {
    Testing,
    Saving,
}

pub(super) const ENGINES: [Engine; 11] = [
    Engine::Postgres,
    Engine::MySql,
    Engine::Sqlite,
    Engine::Supabase,
    Engine::Neon,
    Engine::PlanetScale,
    Engine::Firestore,
    Engine::Convex,
    Engine::Cosmos,
    Engine::Mssql,
    Engine::Azure,
];

pub(super) struct ConnectionEditor {
    pub(super) original: Option<ConnectionConfig>,
    pub(super) engine: Engine,
    pub(super) tab: ConnectionTab,
    pub(super) ssh: bool,
    pub(super) ssl_mode: SslMode,
    pub(super) env_tag: EnvTag,
    pub(super) name: Entity<InputState>,
    pub(super) host: Entity<InputState>,
    pub(super) port: Entity<InputState>,
    pub(super) database: Entity<InputState>,
    pub(super) user: Entity<InputState>,
    pub(super) password: Entity<InputState>,
    pub(super) application_name: Entity<InputState>,
    pub(super) color: Entity<InputState>,
    pub(super) busy: Option<EditorBusy>,
    pub(super) message: Option<Result<String, String>>,
}

impl ConnectionEditor {
    fn new(
        config: Option<ConnectionConfig>,
        window: &mut Window,
        cx: &mut Context<CellarApp>,
    ) -> Self {
        let engine = config
            .as_ref()
            .map(|config| config.engine)
            .unwrap_or(Engine::Postgres);
        let existing = config.is_some();
        let mut value = |value: String, placeholder: String, cx: &mut Context<CellarApp>| {
            cx.new(|cx| {
                InputState::new(window, cx)
                    .default_value(value)
                    .placeholder(placeholder)
            })
        };
        Self {
            engine,
            tab: ConnectionTab::General,
            ssh: false,
            ssl_mode: config
                .as_ref()
                .map(|config| config.ssl_mode)
                .unwrap_or_else(|| default_ssl(engine)),
            env_tag: config
                .as_ref()
                .and_then(|config| config.env_tag)
                .unwrap_or(EnvTag::Local),
            name: value(
                config.as_ref().map(|c| c.name.clone()).unwrap_or_default(),
                name_placeholder(engine),
                cx,
            ),
            host: value(
                config
                    .as_ref()
                    .map(|c| c.host.clone())
                    .unwrap_or_else(|| "localhost".into()),
                host_placeholder(engine).into(),
                cx,
            ),
            port: value(
                config
                    .as_ref()
                    .map(|c| c.port.to_string())
                    .unwrap_or_else(|| default_port(engine).to_string()),
                String::new(),
                cx,
            ),
            database: value(
                config
                    .as_ref()
                    .map(|c| c.database.clone())
                    .unwrap_or_else(|| default_database(engine).into()),
                database_placeholder(engine).into(),
                cx,
            ),
            user: value(
                config.as_ref().map(|c| c.user.clone()).unwrap_or_default(),
                user_placeholder(engine).into(),
                cx,
            ),
            application_name: value(
                config
                    .as_ref()
                    .and_then(|c| c.application_name.clone())
                    .unwrap_or_else(|| "cellar".into()),
                String::new(),
                cx,
            ),
            color: value(
                config
                    .as_ref()
                    .and_then(|c| c.color.clone())
                    .unwrap_or_else(|| engine_color(engine).into()),
                String::new(),
                cx,
            ),
            password: cx.new(|cx| {
                InputState::new(window, cx)
                    .masked(true)
                    .placeholder(password_placeholder(engine, existing))
            }),
            original: config,
            busy: None,
            message: None,
        }
    }

    fn config(&self, cx: &Context<CellarApp>) -> Result<ConnectionConfig, String> {
        let entered_name = text(&self.name, cx);
        let host = text(&self.host, cx);
        let database = text(&self.database, cx);
        let user = text(&self.user, cx);
        let name = if entered_name.is_empty() {
            if self.engine == Engine::Sqlite {
                database.clone()
            } else if self.engine == Engine::Cosmos {
                host.clone()
            } else {
                format!(
                    "{}@{host}/{database}",
                    if user.is_empty() { "user" } else { &user }
                )
            }
        } else {
            entered_name
        };
        let port = text(&self.port, cx)
            .parse::<u16>()
            .map_err(|_| "Port must be between 0 and 65535".to_string())?;
        let color = optional_text(&self.color, cx);
        if color.as_ref().is_some_and(|color| !valid_color(color)) {
            return Err("Color must be an opaque hex value such as #4f8ff7".into());
        }
        let id = self
            .original
            .as_ref()
            .map(|config| config.id.clone())
            .unwrap_or_else(|| {
                let suffix = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_nanos();
                format!("{}-{suffix}", slug(&name))
            });
        Ok(ConnectionConfig {
            id,
            name,
            engine: self.engine,
            host,
            port,
            database,
            user,
            ssl_mode: self.ssl_mode,
            env_tag: Some(self.env_tag),
            application_name: optional_text(&self.application_name, cx),
            color,
        })
    }
}

impl CellarApp {
    pub(super) fn open_connection_editor(
        &mut self,
        config: Option<ConnectionConfig>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.connection_editor = Some(ConnectionEditor::new(config, window, cx));
        cx.notify();
    }

    pub(super) fn select_connection_engine(
        &mut self,
        engine: Engine,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(editor) = &mut self.connection_editor else {
            return;
        };
        editor.engine = engine;
        editor.ssl_mode = default_ssl(editor.engine);
        editor.port.update(cx, |port, cx| {
            port.set_value(default_port(editor.engine).to_string(), window, cx)
        });
        editor.database.update(cx, |database, cx| {
            database.set_value(default_database(editor.engine), window, cx)
        });
        editor.color.update(cx, |color, cx| {
            color.set_value(engine_color(editor.engine), window, cx)
        });
        let (host, user) = default_host_user(editor.engine);
        editor
            .host
            .update(cx, |state, cx| state.set_value(host, window, cx));
        editor
            .user
            .update(cx, |state, cx| state.set_value(user, window, cx));
        let existing = editor.original.is_some();
        editor.name.update(cx, |state, cx| {
            state.set_placeholder(name_placeholder(engine), window, cx)
        });
        editor.host.update(cx, |state, cx| {
            state.set_placeholder(host_placeholder(engine), window, cx)
        });
        editor.database.update(cx, |state, cx| {
            state.set_placeholder(database_placeholder(engine), window, cx)
        });
        editor.user.update(cx, |state, cx| {
            state.set_placeholder(user_placeholder(engine), window, cx)
        });
        editor.password.update(cx, |state, cx| {
            state.set_placeholder(password_placeholder(engine, existing), window, cx)
        });
        editor.message = None;
        cx.notify();
    }

    pub(super) fn select_connection_tab(&mut self, tab: ConnectionTab, cx: &mut Context<Self>) {
        let Some(editor) = &mut self.connection_editor else {
            return;
        };
        editor.tab = tab;
        cx.notify();
    }

    pub(super) fn toggle_connection_ssh(&mut self, cx: &mut Context<Self>) {
        let Some(editor) = &mut self.connection_editor else {
            return;
        };
        editor.ssh = !editor.ssh;
        cx.notify();
    }

    pub(super) fn toggle_connection_ssl(&mut self, cx: &mut Context<Self>) {
        let Some(editor) = &mut self.connection_editor else {
            return;
        };
        editor.ssl_mode = if editor.ssl_mode == SslMode::Disable {
            SslMode::Prefer
        } else {
            SslMode::Disable
        };
        cx.notify();
    }

    pub(super) fn select_connection_ssl(&mut self, mode: SslMode, cx: &mut Context<Self>) {
        if let Some(editor) = &mut self.connection_editor {
            editor.ssl_mode = mode;
            cx.notify();
        }
    }

    pub(super) fn select_connection_env(&mut self, env: EnvTag, cx: &mut Context<Self>) {
        if let Some(editor) = &mut self.connection_editor {
            editor.env_tag = env;
            cx.notify();
        }
    }

    pub(super) fn select_connection_color(
        &mut self,
        color: &'static str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(editor) = &mut self.connection_editor {
            editor
                .color
                .update(cx, |state, cx| state.set_value(color, window, cx));
        }
        cx.notify();
    }

    pub(super) fn duplicate_edited_connection(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(config) = self
            .connection_editor
            .as_ref()
            .and_then(|editor| editor.original.clone())
        else {
            return;
        };
        let mut editor = ConnectionEditor::new(Some(config.clone()), window, cx);
        editor.original = None;
        editor.name.update(cx, |name, cx| {
            name.set_value(format!("{} Copy", config.name), window, cx)
        });
        self.connection_editor = Some(editor);
        cx.notify();
    }

    pub(super) fn duplicate_connection(
        &mut self,
        config: ConnectionConfig,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.open_connection_editor(Some(config), window, cx);
        self.duplicate_edited_connection(window, cx);
    }

    pub(super) fn confirm_delete_connection(
        &mut self,
        config: ConnectionConfig,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.ask_confirmation(
            Confirmation {
                title: "Remove connection".into(),
                message: format!(
                    "Remove connection \"{}\"?\n\nThis deletes its saved password from the keychain.",
                    config.name
                ),
                confirm_label: "Remove",
                danger: true,
                action: ConfirmAction::RemoveConnection(config.id),
            },
            window,
            cx,
        );
    }

    pub(super) fn test_edited_connection(&mut self, cx: &mut Context<Self>) {
        let Some(editor) = &self.connection_editor else {
            return;
        };
        if editor.busy.is_some() {
            return;
        }
        let config = match editor.config(cx) {
            Ok(config) => config,
            Err(error) => {
                self.connection_editor.as_mut().unwrap().message = Some(Err(error));
                cx.notify();
                return;
            }
        };
        let password = optional_text(&editor.password, cx);
        self.connection_editor.as_mut().unwrap().busy = Some(EditorBusy::Testing);
        let registry = Arc::clone(&self.registry);
        let runtime = Arc::clone(&self.runtime);
        let started = Instant::now();
        cx.spawn(async move |this, cx| {
            let result = runtime
                .spawn(async move { registry.test_with_secret(&config, password).await })
                .await
                .map_err(|error| format!("connection test task failed: {error}"))
                .and_then(|result| {
                    result
                        .map(|info| {
                            format!(
                                "{} ms · {}",
                                started.elapsed().as_millis(),
                                super::shell_widgets::short_driver_version(&info.version)
                            )
                        })
                        .map_err(|error| error.to_string())
                });
            this.update(cx, |this, cx| {
                if let Some(editor) = &mut this.connection_editor {
                    editor.busy = None;
                    editor.message = Some(result);
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
        cx.notify();
    }

    pub(super) fn save_edited_connection(&mut self, cx: &mut Context<Self>) {
        let Some(editor) = &self.connection_editor else {
            return;
        };
        if editor.busy.is_some() {
            return;
        }
        let config = match editor.config(cx) {
            Ok(config) => config,
            Err(error) => {
                self.connection_editor.as_mut().unwrap().message = Some(Err(error));
                cx.notify();
                return;
            }
        };
        let password = optional_text(&editor.password, cx);
        self.connection_editor.as_mut().unwrap().busy = Some(EditorBusy::Saving);
        let registry = Arc::clone(&self.registry);
        let runtime = Arc::clone(&self.runtime);
        cx.spawn(async move |this, cx| {
            let result = runtime
                .spawn(async move {
                    let saved = registry
                        .save_with_secret(config, password.as_deref())
                        .await?;
                    registry.disconnect(&saved.id).await?;
                    Ok::<_, cellar_core::error::CellarError>(saved)
                })
                .await
                .map_err(|error| format!("save task failed: {error}"))
                .and_then(|result| result.map_err(|error| error.to_string()));
            this.update(cx, |this, cx| {
                match result {
                    Ok(config) => {
                        this.model.upsert_connection(config);
                        this.reconcile_sidebar_layout();
                        this.connection_editor = None;
                    }
                    Err(error) => {
                        if let Some(editor) = &mut this.connection_editor {
                            editor.busy = None;
                            editor.message = Some(Err(error));
                        }
                    }
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
        cx.notify();
    }

    pub(super) fn delete_connection_confirmed(&mut self, id: String, cx: &mut Context<Self>) {
        if let Some(editor) = &mut self.connection_editor {
            editor.busy = Some(EditorBusy::Saving);
        }
        let registry = Arc::clone(&self.registry);
        let runtime = Arc::clone(&self.runtime);
        cx.spawn(async move |this, cx| {
            let result = runtime
                .spawn({
                    let id = id.clone();
                    async move { registry.delete_with_secret(&id).await }
                })
                .await
                .map_err(|error| format!("delete task failed: {error}"))
                .and_then(|result| result.map_err(|error| error.to_string()));
            this.update(cx, |this, cx| {
                match result {
                    Ok(()) => {
                        this.driver_infos.remove(&id);
                        this.model.remove_connection(&id);
                        this.reconcile_sidebar_layout();
                        this.grids
                            .retain(|tab, _| this.model.tabs().iter().any(|open| open.id == *tab));
                        this.grid_layouts
                            .retain(|tab, _| this.model.tabs().iter().any(|open| open.id == *tab));
                        this.editors
                            .retain(|tab, _| this.model.tabs().iter().any(|open| open.id == *tab));
                        this.table_sorts
                            .retain(|tab, _| this.model.tabs().iter().any(|open| open.id == *tab));
                        this.table_filters
                            .retain(|tab, _| this.model.tabs().iter().any(|open| open.id == *tab));
                        this.table_filter_operators
                            .retain(|tab, _| this.model.tabs().iter().any(|open| open.id == *tab));
                        this.table_filter_inputs
                            .retain(|tab, _| this.model.tabs().iter().any(|open| open.id == *tab));
                        this.table_filter_columns
                            .retain(|tab, _| this.model.tabs().iter().any(|open| open.id == *tab));
                        this.query_summaries
                            .retain(|tab, _| this.model.tabs().iter().any(|open| open.id == *tab));
                        this.commit_review = None;
                        this.data_import = None;
                        this.connection_editor = None;
                    }
                    Err(error) => {
                        if let Some(editor) = &mut this.connection_editor {
                            editor.busy = None;
                        }
                        this.confirmation = Some(Confirmation {
                            title: "Remove connection failed".into(),
                            message: error,
                            confirm_label: "Close",
                            danger: false,
                            action: ConfirmAction::Dismiss,
                        });
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
    use super::{Engine, ENGINES};

    #[test]
    fn azure_is_preserved_as_an_editable_engine() {
        assert!(ENGINES.contains(&Engine::Azure));
    }
}
