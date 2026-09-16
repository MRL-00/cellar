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
    #[serde(rename = "tLSMode", default)]
    #[allow(dead_code)]
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
        // ponytail: `tLSMode` is an undocumented TablePlus integer enum; default
        // to prefer like the new-connection form does and map it once someone
        // writes the values down.
        ssl_mode: SslMode::Prefer,
        env_tag: env_tag(&row.environment),
        application_name: Some("cellar".into()),
        color: color(&row.status_color),
    })
    // `isOverSSH` rows are imported as-is: Cellar has no tunnel support yet, and
    // the stored host is the one TablePlus would reach through the tunnel.
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
mod tests {
    use super::*;

    /// Minimal stand-in for a real `Connections.plist`: the top level is an
    /// array of dicts and each dict carries only the keys under test.
    const FIXTURE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<array>
  <dict>
    <key>ID</key><string>11111111-1111-1111-1111-111111111111</string>
    <key>ConnectionName</key><string>Prod PG</string>
    <key>Driver</key><string>PostgreSQL</string>
    <key>DatabaseHost</key><string>db.example.com</string>
    <key>DatabasePort</key><string>6543</string>
    <key>DatabaseName</key><string>shop</string>
    <key>DatabaseUser</key><string>app</string>
    <key>Enviroment</key><string>production</string>
    <key>statusColor</key><string>#6D0000</string>
    <key>GroupID</key><string>g-1</string>
    <key>isOverSSH</key><true/>
    <key>tLSMode</key><integer>2</integer>
  </dict>
  <dict>
    <key>ID</key><string>22222222-2222-2222-2222-222222222222</string>
    <key>ConnectionName</key><string>Local MySQL</string>
    <key>Driver</key><string>MySQL</string>
    <key>DatabaseHost</key><string></string>
    <key>DatabasePort</key><string></string>
    <key>DatabaseName</key><string></string>
    <key>DatabaseUser</key><string>root</string>
    <key>statusColor</key><string>blue</string>
    <key>GroupID</key><string>g-missing</string>
  </dict>
  <dict>
    <key>ID</key><string>33333333-3333-3333-3333-333333333333</string>
    <key>ConnectionName</key><string>Notes</string>
    <key>Driver</key><string>SQLite</string>
    <key>DatabasePath</key><string>/Users/me/notes.db</string>
  </dict>
  <dict>
    <key>ID</key><string>44444444-4444-4444-4444-444444444444</string>
    <key>ConnectionName</key><string>Azure Sales</string>
    <key>Driver</key><string>Microsoft SQL Server</string>
    <key>DatabaseHost</key><string>epicprod.database.windows.net</string>
    <key>DatabasePort</key><string>1433</string>
    <key>DatabaseName</key><string>Sales</string>
    <key>DatabaseUser</key><string>dbadmin</string>
    <key>GroupID</key><string>g-2</string>
  </dict>
  <dict>
    <key>ID</key><string>55555555-5555-5555-5555-555555555555</string>
    <key>ConnectionName</key><string>Cache</string>
    <key>Driver</key><string>Redis</string>
    <key>DatabaseHost</key><string>redis.example.com</string>
  </dict>
  <dict>
    <key>ID</key><string>66666666-6666-6666-6666-666666666666</string>
    <key>ConnectionName</key><string></string>
    <key>Driver</key><string>PostgreSQL</string>
    <key>DatabaseHost</key><string>anon.example.com</string>
    <key>DatabaseName</key><string>reports</string>
  </dict>
</array>
</plist>"#;

    /// Stand-in for `ConnectionGroups.plist`: a top-level group, a group nested
    /// under it, and a nameless row that must not become a folder.
    const GROUPS: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<plist version="1.0">
<array>
  <dict>
    <key>ID</key><string>g-1</string>
    <key>Name</key><string>OCO</string>
    <key>GroupID</key><string></string>
    <key>IsExpaned</key><true/>
  </dict>
  <dict>
    <key>ID</key><string>g-2</string>
    <key>Name</key><string>Reporting</string>
    <key>GroupID</key><string>g-1</string>
    <key>IsExpaned</key><false/>
  </dict>
  <dict>
    <key>ID</key><string>g-3</string>
    <key>Name</key><string></string>
    <key>GroupID</key><string></string>
  </dict>
</array>
</plist>"#;

    fn groups() -> HashMap<String, String> {
        parse_groups(GROUPS.as_bytes())
    }

    fn by_name<'a>(r: &'a ConnectionImport, name: &str) -> &'a ConnectionConfig {
        r.connections
            .iter()
            .find(|c| c.name == name)
            .unwrap_or_else(|| panic!("no connection named {name}"))
    }

    #[test]
    fn maps_supported_drivers_and_skips_the_rest() {
        let r = parse_connections(FIXTURE.as_bytes(), &groups());
        assert_eq!(r.connections.len(), 5, "5 supported rows imported");
        assert_eq!(r.skipped.len(), 1, "redis skipped");
        assert!(r.skipped[0].contains("Cache"));
        assert!(r.skipped[0].contains("unsupported engine: Redis"));
    }

    #[test]
    fn postgres_row_maps_every_field() {
        let r = parse_connections(FIXTURE.as_bytes(), &groups());
        let pg = by_name(&r, "Prod PG");
        assert_eq!(pg.engine, Engine::Postgres);
        assert_eq!(pg.host, "db.example.com");
        assert_eq!(pg.port, 6543);
        assert_eq!(pg.database, "shop");
        assert_eq!(pg.user, "app");
        assert_eq!(pg.id, "prod-pg");
        assert_eq!(pg.env_tag, Some(EnvTag::Prod));
        assert_eq!(pg.color.as_deref(), Some("#6D0000"));
        assert_eq!(pg.ssl_mode, SslMode::Prefer);
        assert_eq!(pg.application_name.as_deref(), Some("cellar"));
    }

    #[test]
    fn empty_host_port_and_database_fall_back_to_defaults() {
        let r = parse_connections(FIXTURE.as_bytes(), &groups());
        let my = by_name(&r, "Local MySQL");
        assert_eq!(my.engine, Engine::MySql);
        assert_eq!(my.host, "localhost");
        assert_eq!(my.port, 3306);
        assert_eq!(my.database, "mysql");
        assert_eq!(my.color, None, "non-#RRGGBB status color dropped");
        assert_eq!(my.env_tag, None);
    }

    #[test]
    fn sqlite_uses_the_file_path_as_database() {
        let r = parse_connections(FIXTURE.as_bytes(), &groups());
        let db = by_name(&r, "Notes");
        assert_eq!(db.engine, Engine::Sqlite);
        assert_eq!(db.host, "");
        assert_eq!(db.port, 0);
        assert_eq!(db.database, "/Users/me/notes.db");
    }

    #[test]
    fn azure_host_routes_to_the_azure_engine() {
        let r = parse_connections(FIXTURE.as_bytes(), &groups());
        let az = by_name(&r, "Azure Sales");
        assert_eq!(az.engine, Engine::Azure);
        assert_eq!(az.database, "Sales");
        assert_eq!(az.port, 1433);
    }

    #[test]
    fn empty_name_falls_back_to_host_slash_database() {
        let r = parse_connections(FIXTURE.as_bytes(), &groups());
        let c = by_name(&r, "anon.example.com/reports");
        assert_eq!(c.id, "anon-example-com-reports");
    }

    #[test]
    fn sqlite_without_any_path_is_skipped() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<plist version="1.0"><array><dict>
  <key>ConnectionName</key><string>Broken</string>
  <key>Driver</key><string>SQLite</string>
</dict></array></plist>"#;
        let r = parse_connections(xml.as_bytes(), &HashMap::new());
        assert!(r.connections.is_empty());
        assert!(r.skipped[0].contains("Broken"));
    }

    #[test]
    fn rows_sharing_a_tableplus_id_collapse_to_one() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<plist version="1.0"><array>
  <dict>
    <key>ID</key><string>same-uuid</string>
    <key>ConnectionName</key><string>Dupe</string>
    <key>Driver</key><string>PostgreSQL</string>
    <key>DatabaseHost</key><string>h1</string>
  </dict>
  <dict>
    <key>ID</key><string>same-uuid</string>
    <key>ConnectionName</key><string>Dupe</string>
    <key>Driver</key><string>PostgreSQL</string>
    <key>DatabaseHost</key><string>h1</string>
  </dict>
</array></plist>"#;
        let r = merge(vec![parse_file(xml.as_bytes(), &HashMap::new())]);
        assert_eq!(
            r.connections.len(),
            1,
            "identical TablePlus IDs collapse silently"
        );
        assert!(r.skipped.is_empty());
    }

    #[test]
    fn different_ids_with_the_same_name_report_a_duplicate_slug() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<plist version="1.0"><array>
  <dict>
    <key>ID</key><string>a</string>
    <key>ConnectionName</key><string>Shared Name</string>
    <key>Driver</key><string>PostgreSQL</string>
    <key>DatabaseHost</key><string>h1</string>
  </dict>
  <dict>
    <key>ID</key><string>b</string>
    <key>ConnectionName</key><string>Shared Name</string>
    <key>Driver</key><string>PostgreSQL</string>
    <key>DatabaseHost</key><string>h2</string>
  </dict>
</array></plist>"#;
        let r = merge(vec![parse_file(xml.as_bytes(), &HashMap::new())]);
        assert_eq!(r.connections.len(), 1);
        assert_eq!(r.skipped.len(), 1);
        assert!(r.skipped[0].contains("duplicate id 'shared-name'"));
    }

    #[test]
    fn same_connection_in_two_installs_is_imported_once() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<plist version="1.0"><array><dict>
  <key>ID</key><string>uuid-1</string>
  <key>ConnectionName</key><string>Shared</string>
  <key>Driver</key><string>PostgreSQL</string>
</dict></array></plist>"#;
        let r = merge(vec![
            parse_file(xml.as_bytes(), &HashMap::new()),
            parse_file(xml.as_bytes(), &HashMap::new()),
        ]);
        assert_eq!(r.connections.len(), 1);
        assert!(r.skipped.is_empty());
    }

    #[test]
    fn a_row_with_an_unexpected_type_is_skipped_alone() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<plist version="1.0"><array>
  <dict>
    <key>ConnectionName</key><string>Odd</string>
    <key>Driver</key><string>PostgreSQL</string>
    <key>DatabasePort</key><integer>5432</integer>
  </dict>
  <dict>
    <key>ConnectionName</key><string>Fine</string>
    <key>Driver</key><string>PostgreSQL</string>
  </dict>
</array></plist>"#;
        let r = parse_connections(xml.as_bytes(), &HashMap::new());
        assert_eq!(r.connections.len(), 1);
        assert_eq!(r.connections[0].name, "Fine");
        assert_eq!(r.skipped.len(), 1);
        assert!(r.skipped[0].contains("entry #1"));
    }

    #[test]
    fn tableplus_default_swatches_are_not_imported_as_accents() {
        assert_eq!(color("#686B6F"), None);
        assert_eq!(color("#f8f8f8"), None);
        assert_eq!(color("#007F3D").as_deref(), Some("#007F3D"));
        assert_eq!(color("blue"), None);
    }

    #[test]
    fn a_grouped_connection_records_its_folder_name() {
        let r = parse_connections(FIXTURE.as_bytes(), &groups());
        assert_eq!(r.groups.get("prod-pg").map(String::as_str), Some("OCO"));
    }

    #[test]
    fn a_nested_group_maps_to_its_own_leaf_name() {
        let r = parse_connections(FIXTURE.as_bytes(), &groups());
        assert_eq!(
            r.groups.get("azure-sales").map(String::as_str),
            Some("Reporting"),
            "Cellar folders are one level deep, so the leaf name wins"
        );
    }

    #[test]
    fn an_ungrouped_connection_has_no_folder() {
        let r = parse_connections(FIXTURE.as_bytes(), &groups());
        assert!(by_name(&r, "Notes").id == "notes");
        assert!(!r.groups.contains_key("notes"));
    }

    #[test]
    fn an_unknown_group_id_is_ignored() {
        let r = parse_connections(FIXTURE.as_bytes(), &groups());
        assert!(
            !r.groups.contains_key("local-mysql"),
            "a GroupID with no matching group is dropped, not invented"
        );
        assert_eq!(r.connections.len(), 5, "the connection still imports");
    }

    #[test]
    fn a_nameless_group_is_not_a_folder() {
        assert!(!groups().contains_key("g-3"));
        assert_eq!(groups().len(), 2);
    }

    #[test]
    fn a_garbage_groups_file_yields_no_groups_but_keeps_connections() {
        let bad = parse_groups(b"not a plist at all \x00\xff");
        assert!(bad.is_empty());
        let r = parse_connections(FIXTURE.as_bytes(), &bad);
        assert_eq!(r.connections.len(), 5);
        assert!(r.groups.is_empty());
    }

    #[test]
    fn a_duplicate_connection_leaves_no_stray_group_entry() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<plist version="1.0"><array><dict>
  <key>ID</key><string>uuid-1</string>
  <key>ConnectionName</key><string>Shared</string>
  <key>Driver</key><string>PostgreSQL</string>
  <key>GroupID</key><string>g-1</string>
</dict></array></plist>"#;
        let r = merge(vec![
            parse_file(xml.as_bytes(), &groups()),
            parse_file(xml.as_bytes(), &groups()),
        ]);
        assert_eq!(r.connections.len(), 1);
        assert_eq!(r.groups.len(), 1, "one connection, one folder entry");
        assert_eq!(r.groups.get("shared").map(String::as_str), Some("OCO"));
    }

    #[test]
    fn a_duplicate_name_keeps_the_first_rows_folder_not_the_dropped_ones() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<plist version="1.0"><array>
  <dict>
    <key>ID</key><string>a</string>
    <key>ConnectionName</key><string>Twin</string>
    <key>Driver</key><string>PostgreSQL</string>
    <key>GroupID</key><string>g-1</string>
  </dict>
  <dict>
    <key>ID</key><string>b</string>
    <key>ConnectionName</key><string>Twin</string>
    <key>Driver</key><string>PostgreSQL</string>
    <key>GroupID</key><string>g-2</string>
  </dict>
</array></plist>"#;
        let r = parse_connections(xml.as_bytes(), &groups());
        assert_eq!(r.connections.len(), 1);
        assert_eq!(r.groups.get("twin").map(String::as_str), Some("OCO"));
    }

    #[test]
    fn a_dropped_duplicates_folder_never_leaks_onto_the_kept_row() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<plist version="1.0"><array>
  <dict>
    <key>ID</key><string>a</string>
    <key>ConnectionName</key><string>Twin</string>
    <key>Driver</key><string>PostgreSQL</string>
  </dict>
  <dict>
    <key>ID</key><string>b</string>
    <key>ConnectionName</key><string>Twin</string>
    <key>Driver</key><string>PostgreSQL</string>
    <key>GroupID</key><string>g-1</string>
  </dict>
</array></plist>"#;
        let r = parse_connections(xml.as_bytes(), &groups());
        assert_eq!(r.connections.len(), 1);
        assert!(r.groups.is_empty(), "the kept row was ungrouped");
    }

    #[test]
    fn garbage_bytes_yield_an_empty_result() {
        let r = parse_connections(b"not a plist at all \x00\xff", &HashMap::new());
        assert!(r.connections.is_empty());
        assert_eq!(
            r.skipped.len(),
            1,
            "unparseable file is reported, not hidden"
        );
    }
}
