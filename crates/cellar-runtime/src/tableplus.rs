//! One-click import of TablePlus connections.
//!
//! TablePlus keeps every connection in one plist,
//! `<root>/<bundle dir>/Data/Connections.plist`, where `<root>` is
//! `~/Library/Application Support` on macOS or `%LOCALAPPDATA%` on Windows and
//! `<bundle dir>` is `com.tinyapp.TablePlus` (normal build) or
//! `com.tinyapp.TablePlus-setapp` (Setapp build). A machine can have both, so
//! we scan them all and de-duplicate. The Linux build uses a different layout
//! (`~/.tableplus/settings/connections.json`) and is not handled here yet.
//! Passwords are NOT in the file (they live in the OS keychain), so we import
//! the connection metadata only and let the user supply passwords at import
//! time or on first connect. The sibling `Data/ConnectionGroups.plist` names
//! the groups a connection can sit in; those map onto Cellar's sidebar folders.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use cellar_core::driver::{ConnectionConfig, Engine, EnvTag, SslMode};
use serde::Deserialize;

use crate::connection_import::{slugify, ConnectionImport};

/// One `<dict>` from `Connections.plist`. Every field is `#[serde(default)]` so
/// a missing or unexpected key degrades that field instead of failing the whole
/// import — TablePlus adds keys between releases and per driver.
#[derive(Debug, Default, Deserialize)]
struct RawConnection {
    #[serde(rename = "ID", default)]
    id: String,
    #[serde(rename = "ConnectionName", default)]
    name: String,
    #[serde(rename = "Driver", default)]
    driver: String,
    #[serde(rename = "DatabaseHost", default)]
    host: String,
    /// TablePlus stores the port as a *string*, and leaves it empty when the
    /// connection uses the driver default.
    #[serde(rename = "DatabasePort", default)]
    port: String,
    #[serde(rename = "DatabaseName", default)]
    database: String,
    #[serde(rename = "DatabaseUser", default)]
    user: String,
    /// Absolute file path, used by the SQLite driver instead of host/port.
    #[serde(rename = "DatabasePath", default)]
    path: String,
    /// `Enviroment` (sic) — TablePlus misspells the key on disk, so the rename
    /// has to keep the typo or the value never binds.
    #[serde(rename = "Enviroment", default)]
    environment: String,
    #[serde(rename = "statusColor", default)]
    status_color: String,
    /// ID of the `ConnectionGroups.plist` group holding this connection; empty
    /// when the connection sits at the top level.
    #[serde(rename = "GroupID", default)]
    group_id: String,
    #[serde(rename = "isOverSSH", default)]
    #[allow(dead_code)]
    is_over_ssh: bool,
    /// Index of the SSL-mode dropdown in TablePlus's per-driver connection
    /// form; the option list differs per driver, see [`ssl_mode`].
    #[serde(rename = "tLSMode", default)]
    tls_mode: i64,
}

/// One `<dict>` from `ConnectionGroups.plist`. TablePlus nests groups by
/// pointing a group at its parent through `GroupID`; Cellar folders are one
/// level deep, so only the leaf name is used.
#[derive(Debug, Default, Deserialize)]
struct RawGroup {
    #[serde(rename = "ID", default)]
    id: String,
    #[serde(rename = "Name", default)]
    name: String,
    /// Parent group id, empty for a top-level group. Kept so the nesting is
    /// visible in the data even though the mapping flattens it.
    #[serde(rename = "GroupID", default)]
    #[allow(dead_code)]
    group_id: String,
}

/// Parse a `ConnectionGroups.plist` document into `group id -> group name`. A
/// missing or malformed file simply means "no groups": grouping is a nicety and
/// must never cost the user their connections.
fn parse_groups(plist_bytes: &[u8]) -> HashMap<String, String> {
    let mut groups = HashMap::new();
    let Ok(rows) = plist::from_bytes::<Vec<plist::Value>>(plist_bytes) else {
        return groups;
    };
    for value in &rows {
        let Ok(row) = plist::from_value::<RawGroup>(value) else {
            continue;
        };
        let name = row.name.trim();
        if row.id.is_empty() || name.is_empty() {
            continue;
        }
        groups.insert(row.id, name.to_string());
    }
    groups
}

/// TablePlus data directories: the normal build (`com.tinyapp.TablePlus`)
/// and the Setapp build (`…-setapp`), under whichever root the platform uses.
fn config_dirs() -> Vec<PathBuf> {
    // macOS keeps app data under the config dir; Windows keeps it under the
    // local (non-roaming) data dir. Both resolve to the same place on macOS.
    let roots: Vec<PathBuf> = [dirs::config_dir(), dirs::data_local_dir()]
        .into_iter()
        .flatten()
        .collect();
    let mut seen = HashSet::new();
    roots
        .into_iter()
        .filter(|root| seen.insert(root.clone()))
        .filter_map(|root| std::fs::read_dir(root).ok())
        .flat_map(|entries| entries.flatten().map(|e| e.path()))
        .filter(|p| {
            p.file_name()
                .map(|n| {
                    n.to_string_lossy()
                        .to_ascii_lowercase()
                        .starts_with("com.tinyapp.tableplus")
                })
                .unwrap_or(false)
        })
        .collect()
}
/// `<dir>/Data/<name>`, matched case-insensitively inside `Data/` so a casing
/// change in a future build does not silently break the scan.
fn data_file(dir: &std::path::Path, name: &str) -> Option<PathBuf> {
    let data = dir.join("Data");
    let entries = std::fs::read_dir(&data).ok()?;
    entries
        .flatten()
        .map(|e| e.path())
        .find(|p| {
            p.file_name()
                .map(|n| n.to_string_lossy().eq_ignore_ascii_case(name))
                .unwrap_or(false)
        })
        .filter(|p| p.is_file())
}

/// Scan the local machine for TablePlus connections, de-duplicating across the
/// normal and Setapp installs (both can hold the same connections).
pub fn scan() -> ConnectionImport {
    let mut files = Vec::new();
    for dir in config_dirs() {
        let Some(path) = data_file(&dir, "connections.plist") else {
            continue;
        };
        // Groups are per install, so they are read alongside that install's
        // connections and never shared between them.
        let groups = data_file(&dir, "connectiongroups.plist")
            .and_then(|path| std::fs::read(path).ok())
            .map(|bytes| parse_groups(&bytes))
            .unwrap_or_default();
        match std::fs::read(&path) {
            Ok(bytes) => files.push(parse_file(&bytes, &groups)),
            Err(_) => files.push(ParsedFile {
                result: ConnectionImport {
                    connections: Vec::new(),
                    skipped: vec![format!("{} — could not be read", path.display())],
                    groups: HashMap::new(),
                },
                rows: Vec::new(),
            }),
        }
    }
    merge(files)
}

/// Combine the parsed files into one result. TablePlus `ID`s are stable uuids:
/// the same connection present in two installs collapses silently, and only
/// then are genuinely different connections that share a name (and therefore a
/// slug id) reported as duplicates.
fn merge(files: Vec<ParsedFile>) -> ConnectionImport {
    let mut connections = Vec::new();
    let mut skipped = Vec::new();
    let mut groups = HashMap::new();
    let mut seen_uuid = HashSet::new();
    let mut seen_id = HashSet::new();
    for ParsedFile { result, rows } in files {
        skipped.extend(result.skipped);
        for ((uuid, folder), cfg) in rows.into_iter().zip(result.connections) {
            if !uuid.is_empty() && !seen_uuid.insert(uuid) {
                continue; // same TablePlus connection seen in another install
            }
            if seen_id.insert(cfg.id.clone()) {
                if let Some(folder) = folder {
                    groups.insert(cfg.id.clone(), folder);
                }
                connections.push(cfg);
            } else {
                skipped.push(format!(
                    "{} — duplicate id '{}' (a connection with the same name was already imported)",
                    cfg.name, cfg.id
                ));
            }
        }
    }
    connections.sort_by(|a, b| a.name.cmp(&b.name));
    ConnectionImport {
        connections,
        skipped,
        groups,
    }
}
/// [`parse_connections`] plus the TablePlus `ID` of each imported row, so
/// [`scan`] can de-duplicate across installs before slug collisions are judged.
struct ParsedFile {
    result: ConnectionImport,
    /// Parallel to `result.connections`: the TablePlus `ID` and folder name of
    /// each row. Kept per row rather than keyed by slug so a dropped duplicate
    /// can never hand its folder to the row that survives.
    rows: Vec<(String, Option<String>)>,
}

/// Parse a `Connections.plist` document (XML or binary) into importable
/// connections. A malformed plist yields a single skipped entry rather than
/// failing the whole scan; a row with an unexpected value type is skipped on
/// its own so one odd entry cannot hide the rest.
/// `groups` maps TablePlus group ids to folder names, as read from the sibling
/// `ConnectionGroups.plist`; pass an empty map when there is none.
pub fn parse_connections(plist_bytes: &[u8], groups: &HashMap<String, String>) -> ConnectionImport {
    merge(vec![parse_file(plist_bytes, groups)])
}

fn parse_file(plist_bytes: &[u8], groups: &HashMap<String, String>) -> ParsedFile {
    let mut connections = Vec::new();
    let mut skipped = Vec::new();
    let mut meta = Vec::new();

    let rows = match plist::from_bytes::<Vec<plist::Value>>(plist_bytes) {
        Ok(rows) => rows,
        Err(e) => {
            skipped.push(format!("Connections.plist — could not be parsed: {e}"));
            return ParsedFile {
                result: ConnectionImport {
                    connections,
                    skipped,
                    groups: HashMap::new(),
                },
                rows: Vec::new(),
            };
        }
    };

    for (index, value) in rows.into_iter().enumerate() {
        let row: RawConnection = match plist::from_value(&value) {
            Ok(row) => row,
            Err(e) => {
                skipped.push(format!("entry #{} — unreadable: {e}", index + 1));
                continue;
            }
        };
        let label = if row.name.is_empty() {
            "unnamed".to_string()
        } else {
            row.name.clone()
        };
        let uuid = row.id.clone();
        // An unknown group id means the groups file is missing, unreadable, or
        // out of step with the connections; the connection still imports, just
        // without a folder.
        let folder = groups.get(row.group_id.trim()).cloned();
        match build_config(row) {
            Ok(cfg) => {
                connections.push(cfg);
                meta.push((uuid, folder));
            }
            Err(reason) => skipped.push(format!("{label} — {reason}")),
        }
    }

    ParsedFile {
        result: ConnectionImport {
            connections,
            skipped,
            groups: HashMap::new(),
        },
        rows: meta,
    }
}

/// Engine defaults for a TablePlus `Driver` value, matched case-insensitively.
fn engine_for(driver: &str) -> Option<(Engine, u16, &'static str)> {
    match driver.trim().to_ascii_lowercase().as_str() {
        "postgresql" => Some((Engine::Postgres, 5432, "postgres")),
        "mysql" | "mariadb" => Some((Engine::MySql, 3306, "mysql")),
        "microsoft sql server" | "sql server" => Some((Engine::Mssql, 1433, "master")),
        "sqlite" => Some((Engine::Sqlite, 0, "")),
        _ => None,
    }
}

fn build_config(row: RawConnection) -> Result<ConnectionConfig, String> {
    let (mut engine, default_port, default_db) =
        engine_for(&row.driver).ok_or_else(|| format!("unsupported engine: {}", row.driver))?;

    let (host, port, database) = if engine == Engine::Sqlite {
        // The SQLite driver reads the file path from `config.database`
        // (crates/cellar-drivers/sqlite/src/connect.rs), so host/port are unused.
        // Some rows store the path in `DatabaseHost` instead of `DatabasePath`.
        let path = if row.path.is_empty() {
            row.host.trim()
        } else {
            row.path.trim()
        };
        if path.is_empty() {
            return Err("SQLite connection has no database file path".into());
        }
        (String::new(), 0, path.to_string())
    } else {
        let host = if row.host.trim().is_empty() {
            "localhost".to_string()
        } else {
            row.host.trim().to_string()
        };
        // Azure SQL is the same wire protocol but a distinct Cellar engine; route
        // *.database.windows.net hosts there so they connect with the right driver.
        if engine == Engine::Mssql && host.to_ascii_lowercase().ends_with(".database.windows.net") {
            engine = Engine::Azure;
        }
        let port = row.port.trim().parse().unwrap_or(default_port);
        let database = if row.database.trim().is_empty() {
            default_db.to_string()
        } else {
            row.database.trim().to_string()
        };
        (host, port, database)
    };

    let name = if row.name.trim().is_empty() {
        format!("{host}/{database}")
    } else {
        row.name.trim().to_string()
    };

    Ok(ConnectionConfig {
        id: slugify(&name),
        name,
        engine,
        host,
        port,
        database,
        user: row.user.trim().to_string(),
        ssl_mode: ssl_mode(&row.driver, row.tls_mode),
        ssh: None,
        env_tag: env_tag(&row.environment),
        application_name: Some("cellar".into()),
        color: color(&row.status_color),
    })
    // SSH settings are not imported. The UI asks users to configure the tunnel;
    // the stored host is the one TablePlus would reach through that tunnel.
}

/// Map TablePlus's `tLSMode` to Cellar's [`SslMode`]. TablePlus stores the
/// zero-based index of the SSL-mode dropdown, and the dropdown differs per
/// driver (read from the connection forms of the current macOS build; the
/// vendor docs don't list the values). SQL Server has no such dropdown and
/// SQLite has no TLS, so both keep the default. An index we don't recognise
/// also falls back to the default rather than guessing.
fn ssl_mode(driver: &str, tls_mode: i64) -> SslMode {
    match (driver.trim().to_ascii_lowercase().as_str(), tls_mode) {
        // PostgreSQL: PREFERRED, DISABLED, REQUIRED, ALLOW, VERIFY-CA, VERIFY-FULL.
        // Cellar has no "allow", and "prefer" is the closest opportunistic mode.
        ("postgresql", 0) | ("postgresql", 3) => SslMode::Prefer,
        ("postgresql", 1) => SslMode::Disable,
        ("postgresql", 2) => SslMode::Require,
        ("postgresql", 4) => SslMode::VerifyCa,
        ("postgresql", 5) => SslMode::VerifyFull,
        // MySQL: PREFERRED, DISABLED, REQUIRED, VERIFY-CA, VERIFY-IDENTITY.
        ("mysql", 1) => SslMode::Disable,
        ("mysql", 2) => SslMode::Require,
        ("mysql", 3) => SslMode::VerifyCa,
        ("mysql", 4) => SslMode::VerifyFull,
        // MariaDB: PREFERRED, ENFORCE, VERIFY-SERVER-CERT.
        ("mariadb", 1) => SslMode::Require,
        ("mariadb", 2) => SslMode::VerifyFull,
        _ => SslMode::Prefer,
    }
}

fn env_tag(value: &str) -> Option<EnvTag> {
    match value.trim().to_ascii_lowercase().as_str() {
        "local" => Some(EnvTag::Local),
        "development" | "dev" => Some(EnvTag::Dev),
        "staging" | "testing" => Some(EnvTag::Staging),
        "production" | "prod" => Some(EnvTag::Prod),
        _ => None,
    }
}

/// TablePlus writes `statusColor` as `#RRGGBB`. Anything else (empty, named,
/// alpha) is dropped rather than handed to the sidebar as an invalid swatch, and
/// so are TablePlus's own "no colour" defaults (grey and near-white), which
/// would otherwise paint every imported connection with a meaningless accent.
fn color(value: &str) -> Option<String> {
    const DEFAULTS: [&str; 2] = ["#686B6F", "#F8F8F8"];
    let v = value.trim();
    let hex = v.strip_prefix('#')?;
    let valid = hex.len() == 6 && hex.chars().all(|c| c.is_ascii_hexdigit());
    (valid && !DEFAULTS.iter().any(|d| d.eq_ignore_ascii_case(v))).then(|| v.to_string())
}

#[cfg(test)]
#[path = "tableplus_tests.rs"]
mod tests;
