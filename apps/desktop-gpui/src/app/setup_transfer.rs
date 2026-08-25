use std::{
    collections::{HashMap, HashSet},
    fs::{self, OpenOptions},
    io::Write,
    path::Path,
};

use cellar_core::driver::ConnectionConfig;
use cellar_desktop_gpui::grid::{GridLayout, PortableGridLayout};
use gpui::Entity;
use gpui_component::input::InputState;
use serde::{Deserialize, Serialize};

use super::{preferences::Preferences, CellarApp};

const SETUP_FORMAT: &str = "cellar.setup";
const SETUP_VERSION: u8 = 1;
const MAX_SETUP_BYTES: usize = 8 * 1024 * 1024;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct SetupBundle {
    format: String,
    version: u8,
    exported_at: String,
    app: String,
    sections: SetupSections,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct SetupSections {
    settings: Option<Preferences>,
    connections: Option<Vec<ConnectionConfig>>,
    table_layouts: Option<HashMap<String, PortableGridLayout>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) enum SetupSection {
    Settings,
    Connections,
    TableLayouts,
}

#[derive(Clone, Debug)]
pub(super) struct ExportSetup {
    pub selected: HashSet<SetupSection>,
    pub connection_ids: HashSet<String>,
    pub message: Option<Result<String, String>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ImportDecision {
    Skip,
    Add,
    Replace,
    Copy,
}

#[derive(Clone, Debug)]
pub(super) struct ConnectionImportItem {
    pub incoming: ConnectionConfig,
    pub duplicate_id: Option<String>,
    pub duplicate_name: Option<String>,
    pub decision: ImportDecision,
}

#[derive(Clone, Debug)]
pub(super) struct LayoutImportItem {
    pub key: String,
    pub layout: GridLayout,
    pub exists: bool,
    pub apply: bool,
}

#[derive(Clone, Debug)]
pub(super) struct ImportPlan {
    pub connections: Vec<ConnectionImportItem>,
    pub layouts: Vec<LayoutImportItem>,
    pub settings: Option<(Preferences, bool)>,
}

#[derive(Clone, Debug)]
pub(super) enum ImportSetupState {
    Source { loading: bool },
    Review(ImportPlan),
    Complete(ImportSummary),
}

#[derive(Clone, Copy, Debug)]
pub(super) struct ImportSummary {
    pub connections_added: usize,
    pub connections_replaced: usize,
    pub connections_skipped: usize,
    pub layouts_added: usize,
    pub layouts_replaced: usize,
    pub layouts_skipped: usize,
    pub settings_applied: bool,
}

impl ImportSummary {
    pub(super) fn from_plan(plan: &ImportPlan) -> Self {
        Self {
            connections_added: plan
                .connections
                .iter()
                .filter(|item| matches!(item.decision, ImportDecision::Add | ImportDecision::Copy))
                .count(),
            connections_replaced: plan
                .connections
                .iter()
                .filter(|item| item.decision == ImportDecision::Replace)
                .count(),
            connections_skipped: plan
                .connections
                .iter()
                .filter(|item| item.decision == ImportDecision::Skip)
                .count(),
            layouts_added: plan
                .layouts
                .iter()
                .filter(|item| item.apply && !item.exists)
                .count(),
            layouts_replaced: plan
                .layouts
                .iter()
                .filter(|item| item.apply && item.exists)
                .count(),
            layouts_skipped: plan.layouts.iter().filter(|item| !item.apply).count(),
            settings_applied: plan.settings.as_ref().is_some_and(|(_, apply)| *apply),
        }
    }
}

#[derive(Clone, Debug)]
pub(super) struct ImportSetup {
    pub state: ImportSetupState,
    pub file_name: Option<String>,
    pub raw: Entity<InputState>,
    pub error: Option<String>,
    pub applying: bool,
}

#[derive(Clone, Debug)]
pub(super) enum SetupTransfer {
    Export(ExportSetup),
    Import(ImportSetup),
}

impl CellarApp {
    pub(super) fn setup_bundle(&self, export: &ExportSetup) -> SetupBundle {
        let selected = &export.selected;
        let connections = selected.contains(&SetupSection::Connections).then(|| {
            self.model
                .connections()
                .iter()
                .filter(|connection| export.connection_ids.contains(&connection.id))
                .cloned()
                .collect()
        });
        let table_layouts = selected.contains(&SetupSection::TableLayouts).then(|| {
            self.table_layouts
                .iter()
                .filter(|(key, _)| {
                    !selected.contains(&SetupSection::Connections)
                        || export
                            .connection_ids
                            .contains(key.split_once("::").map_or("", |(id, _)| id))
                })
                .map(|(key, layout)| (key.clone(), layout.portable()))
                .collect()
        });
        SetupBundle {
            format: SETUP_FORMAT.into(),
            version: SETUP_VERSION,
            exported_at: chrono::Utc::now().to_rfc3339(),
            app: env!("CARGO_PKG_VERSION").into(),
            sections: SetupSections {
                settings: selected
                    .contains(&SetupSection::Settings)
                    .then(|| self.preferences.clone()),
                connections,
                table_layouts,
            },
        }
    }

    pub(super) fn import_plan(&self, bundle: SetupBundle) -> ImportPlan {
        let connections = bundle
            .sections
            .connections
            .unwrap_or_default()
            .into_iter()
            .map(|incoming| {
                let duplicate = self.model.connections().iter().find(|existing| {
                    connection_identity(existing) == connection_identity(&incoming)
                });
                ConnectionImportItem {
                    duplicate_id: duplicate.map(|connection| connection.id.clone()),
                    duplicate_name: duplicate.map(|connection| connection.name.clone()),
                    decision: if duplicate.is_some() {
                        ImportDecision::Skip
                    } else {
                        ImportDecision::Add
                    },
                    incoming,
                }
            })
            .collect();
        let layouts = bundle
            .sections
            .table_layouts
            .unwrap_or_default()
            .into_iter()
            .map(|(key, layout)| {
                let exists = self.table_layouts.contains_key(&key);
                LayoutImportItem {
                    exists,
                    key,
                    layout: GridLayout::from_portable(layout),
                    apply: !exists,
                }
            })
            .collect();
        ImportPlan {
            connections,
            layouts,
            settings: bundle
                .sections
                .settings
                .map(|settings| (settings.sanitized(), true)),
        }
    }
}

pub(super) fn parse_setup(bytes: &[u8]) -> Result<SetupBundle, String> {
    if bytes.len() > MAX_SETUP_BYTES {
        return Err("That setup file is larger than 8 MB.".into());
    }
    let value: serde_json::Value =
        serde_json::from_slice(bytes).map_err(|_| "That doesn't look like valid JSON.")?;
    if value.get("format").and_then(serde_json::Value::as_str) != Some(SETUP_FORMAT) {
        return Err("Not a Cellar setup file (missing the cellar.setup marker).".into());
    }
    let version = value
        .get("version")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0);
    if version > u64::from(SETUP_VERSION) {
        return Err(format!(
            "This file was exported by a newer Cellar (v{version}). Update Cellar to import it."
        ));
    }
    let bundle: SetupBundle = serde_json::from_value(value)
        .map_err(|error| format!("That setup file is malformed: {error}"))?;
    let has_content = bundle.sections.settings.is_some()
        || bundle
            .sections
            .connections
            .as_ref()
            .is_some_and(|items| !items.is_empty())
        || bundle
            .sections
            .table_layouts
            .as_ref()
            .is_some_and(|items| !items.is_empty());
    has_content
        .then_some(bundle)
        .ok_or_else(|| "This file has no importable sections.".into())
}

pub(super) fn serialize_setup(bundle: &SetupBundle) -> Result<Vec<u8>, String> {
    serde_json::to_vec_pretty(bundle).map_err(|error| error.to_string())
}

pub(super) fn write_setup(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let parent = path.parent().ok_or("Choose a file inside a folder.")?;
    fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    let temporary = path.with_extension(format!("json.{}.tmp", std::process::id()));
    let mut options = OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options
        .open(&temporary)
        .map_err(|error| error.to_string())?;
    file.write_all(bytes).map_err(|error| error.to_string())?;
    file.sync_all().map_err(|error| error.to_string())?;
    drop(file);
    #[cfg(windows)]
    if path.exists() {
        fs::remove_file(path).map_err(|error| error.to_string())?;
    }
    fs::rename(temporary, path).map_err(|error| error.to_string())
}

fn connection_identity(connection: &ConnectionConfig) -> String {
    format!(
        "{}|{}|{}|{}|{}",
        connection.engine.as_str(),
        connection.host.trim().to_lowercase(),
        connection.port,
        connection.database.trim().to_lowercase(),
        connection.user.trim().to_lowercase()
    )
}

pub(super) fn prepared_connections(
    plan: &ImportPlan,
    existing_ids: impl IntoIterator<Item = String>,
) -> Vec<ConnectionConfig> {
    let mut taken = existing_ids.into_iter().collect::<HashSet<_>>();
    let mut prepared = Vec::new();
    for item in &plan.connections {
        let mut connection = item.incoming.clone();
        match item.decision {
            ImportDecision::Skip => continue,
            ImportDecision::Replace => {
                let Some(id) = &item.duplicate_id else {
                    continue;
                };
                connection.id = id.clone();
            }
            ImportDecision::Copy => {
                connection.name = format!("{} (imported)", connection.name);
                connection.id = unique_id(&connection.name, &taken);
            }
            ImportDecision::Add => {
                if connection.id.is_empty() || taken.contains(&connection.id) {
                    connection.id = unique_id(&connection.name, &taken);
                }
            }
        }
        taken.insert(connection.id.clone());
        prepared.push(connection);
    }
    prepared
}

pub(super) fn set_connection_bulk(plan: &mut ImportPlan, decision: ImportDecision) {
    for item in &mut plan.connections {
        match decision {
            ImportDecision::Add if item.duplicate_id.is_none() => item.decision = decision,
            ImportDecision::Replace if item.duplicate_id.is_some() => item.decision = decision,
            ImportDecision::Skip => item.decision = decision,
            ImportDecision::Add | ImportDecision::Replace | ImportDecision::Copy => {}
        }
    }
}

pub(super) fn set_layout_bulk(plan: &mut ImportPlan, apply: bool, existing: bool) {
    for item in &mut plan.layouts {
        if !apply || item.exists == existing {
            item.apply = apply;
        }
    }
}

fn unique_id(base: &str, taken: &HashSet<String>) -> String {
    let slug = base
        .to_lowercase()
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .chars()
        .take(64)
        .collect::<String>();
    let slug = if slug.is_empty() { "connection" } else { &slug };
    if !taken.contains(slug) {
        return slug.into();
    }
    (2..)
        .map(|number| format!("{slug}-{number}"))
        .find(|candidate| !taken.contains(candidate))
        .expect("connection id space exhausted")
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn setup_parser_rejects_wrong_markers_and_strips_unknown_secrets() {
        assert!(parse_setup(br#"{"format":"wrong","version":1,"sections":{}}"#).is_err());
        let raw = serde_json::to_vec(&json!({
            "format": "cellar.setup",
            "version": 1,
            "exportedAt": "",
            "app": "0.2.0",
            "sections": { "connections": [{
                "id": "safe", "name": "Safe", "engine": "postgres",
                "host": "localhost", "port": 5432, "database": "app",
                "user": "me", "ssl_mode": "prefer", "env_tag": null,
                "application_name": null, "color": null, "password": "never"
            }] }
        }))
        .unwrap();
        let serialized = serialize_setup(&parse_setup(&raw).unwrap()).unwrap();
        assert!(!String::from_utf8(serialized).unwrap().contains("never"));
    }

    #[test]
    fn bulk_import_only_changes_the_requested_class() {
        let connection = serde_json::from_value::<ConnectionConfig>(json!({
            "id": "new", "name": "New", "engine": "postgres", "host": "localhost",
            "port": 5432, "database": "app", "user": "me", "ssl_mode": "prefer",
            "env_tag": null, "application_name": null, "color": null
        }))
        .unwrap();
        let mut plan = ImportPlan {
            connections: vec![
                ConnectionImportItem {
                    incoming: connection.clone(),
                    duplicate_id: None,
                    duplicate_name: None,
                    decision: ImportDecision::Skip,
                },
                ConnectionImportItem {
                    incoming: connection,
                    duplicate_id: Some("old".into()),
                    duplicate_name: Some("Old".into()),
                    decision: ImportDecision::Skip,
                },
            ],
            layouts: Vec::new(),
            settings: None,
        };
        set_connection_bulk(&mut plan, ImportDecision::Add);
        assert_eq!(plan.connections[0].decision, ImportDecision::Add);
        assert_eq!(plan.connections[1].decision, ImportDecision::Skip);
        set_connection_bulk(&mut plan, ImportDecision::Replace);
        assert_eq!(plan.connections[1].decision, ImportDecision::Replace);
        plan.layouts = vec![
            LayoutImportItem {
                key: "new".into(),
                layout: GridLayout::from_portable(PortableGridLayout::default()),
                exists: false,
                apply: true,
            },
            LayoutImportItem {
                key: "old".into(),
                layout: GridLayout::from_portable(PortableGridLayout::default()),
                exists: true,
                apply: false,
            },
        ];
        let summary = ImportSummary::from_plan(&plan);
        assert_eq!(
            (summary.connections_added, summary.connections_replaced),
            (1, 1)
        );
        assert_eq!((summary.layouts_added, summary.layouts_skipped), (1, 1));
        set_connection_bulk(&mut plan, ImportDecision::Skip);
        assert!(plan
            .connections
            .iter()
            .all(|item| item.decision == ImportDecision::Skip));
    }
}
