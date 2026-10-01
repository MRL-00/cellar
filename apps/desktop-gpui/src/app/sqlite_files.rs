use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    sync::Arc,
};

use cellar_core::driver::{ConnectionConfig, Engine, EnvTag, SslMode};
use gpui::{Context, Window};
use uuid::Uuid;

use super::CellarApp;

const SQLITE_EXTENSIONS: [&str; 5] = ["db", "db3", "sdb", "sqlite", "sqlite3"];

pub(super) fn is_sqlite_file(path: &Path) -> bool {
    path.is_file()
        && path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| {
                SQLITE_EXTENSIONS
                    .iter()
                    .any(|candidate| extension.eq_ignore_ascii_case(candidate))
            })
}

fn canonical_sqlite_path(path: &Path) -> Option<PathBuf> {
    is_sqlite_file(path)
        .then(|| path.canonicalize().ok())
        .flatten()
}

fn config_for_path(path: &Path) -> ConnectionConfig {
    let name = path
        .file_stem()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .unwrap_or("SQLite database")
        .to_owned();
    ConnectionConfig {
        id: format!("sqlite-{}", Uuid::new_v4()),
        name,
        engine: Engine::Sqlite,
        host: String::new(),
        port: 0,
        database: path.to_string_lossy().into_owned(),
        user: String::new(),
        ssl_mode: SslMode::Disable,
        env_tag: Some(EnvTag::Local),
        application_name: None,
        color: Some("#a78bfa".into()),
    }
}

impl CellarApp {
    pub(crate) fn open_sqlite_files(
        &mut self,
        paths: Vec<PathBuf>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let mut seen = HashSet::new();
        for path in paths {
            let Some(path) = canonical_sqlite_path(&path) else {
                continue;
            };
            if !seen.insert(path.clone()) || self.opening_sqlite_files.contains(&path) {
                continue;
            }

            if let Some(id) = self.model.connections().iter().find_map(|config| {
                (config.engine == Engine::Sqlite
                    && Path::new(&config.database)
                        .canonicalize()
                        .is_ok_and(|existing| existing == path))
                .then(|| config.id.clone())
            }) {
                self.model.select_connection(&id);
                if !self.model.connection_expanded(&id) {
                    self.model.toggle_connection_expanded(&id);
                }
                self.start_connect(id, window, cx);
                continue;
            }

            self.opening_sqlite_files.insert(path.clone());
            let config = config_for_path(&path);
            let registry = Arc::clone(&self.registry);
            let runtime = Arc::clone(&self.runtime);
            cx.spawn_in(window, async move |this, cx| {
                let task_config = config.clone();
                let result = runtime
                    .spawn(async move { registry.save(task_config).await })
                    .await
                    .map_err(|error| format!("save task failed: {error}"))
                    .and_then(|result| result.map_err(|error| error.to_string()));
                this.update_in(cx, |this, window, cx| {
                    this.opening_sqlite_files.remove(&path);
                    match result {
                        Ok(config) => {
                            let id = config.id.clone();
                            this.model.upsert_connection(config);
                            this.model.toggle_connection_expanded(&id);
                            this.reconcile_sidebar_layout();
                            this.start_connect(id, window, cx);
                        }
                        Err(error) => {
                            this.open_connection_editor(Some(config), window, cx);
                            if let Some(editor) = &mut this.connection_editor {
                                editor.message = Some(Err(format!(
                                    "Could not save this SQLite connection: {error}"
                                )));
                            }
                        }
                    }
                    cx.notify();
                })
                .ok();
            })
            .detach();
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use super::{canonical_sqlite_path, config_for_path, is_sqlite_file};
    use cellar_core::driver::{Engine, EnvTag, SslMode};

    #[test]
    fn accepts_common_sqlite_extensions_case_insensitively() {
        let dir = tempfile::tempdir().unwrap();
        let sqlite = dir.path().join("analytics.SQLite3");
        let text = dir.path().join("notes.txt");
        fs::write(&sqlite, b"SQLite format 3\0").unwrap();
        fs::write(&text, b"not a database").unwrap();

        assert!(is_sqlite_file(&sqlite));
        assert!(!is_sqlite_file(&text));
        assert!(!is_sqlite_file(&dir.path().join("missing.db")));
        assert_eq!(canonical_sqlite_path(&sqlite), sqlite.canonicalize().ok());
    }

    #[test]
    fn builds_a_local_sqlite_connection_from_the_file_name() {
        let path = PathBuf::from("/tmp/customer-data.sqlite");
        let config = config_for_path(&path);

        assert!(config.id.starts_with("sqlite-"));
        assert_eq!(config.name, "customer-data");
        assert_eq!(config.engine, Engine::Sqlite);
        assert_eq!(config.database, "/tmp/customer-data.sqlite");
        assert_eq!(config.port, 0);
        assert_eq!(config.ssl_mode, SslMode::Disable);
        assert_eq!(config.env_tag, Some(EnvTag::Local));
    }
}
