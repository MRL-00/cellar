//! One-click import of DataGrip connections.
//!
//! DataGrip stores data sources *per project*, not in one global file. Each
//! version's `~/Library/Application Support/JetBrains/DataGrip<ver>/options/recentProjects.xml`
//! lists the open projects; the connections live in each project's
//! `.idea/dataSources.xml` (and the multi-file `.idea/dataSources/<uuid>.xml`).
//! Passwords are NOT in those files (they live in DataGrip's own keystore), so
//! we import the connection metadata only and let the user supply passwords at
//! import time or on first connect.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use cellar_core::driver::{ConnectionConfig, Engine, SslMode};
use quick_xml::events::Event;
use quick_xml::Reader;
use serde::{Deserialize, Serialize};
use specta::Type;

/// Result of scanning DataGrip for importable connections.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct DatagripImport {
    /// Connections we could map. Passwords are never included — the caller
    /// collects them separately (import-time form or on first connect).
    pub connections: Vec<ConnectionConfig>,
    /// Data sources we found but could not import, each with a human reason
    /// (unsupported engine, unparseable URL). Surfaced so the import is honest
    /// about what it dropped rather than silently skipping rows.
    pub skipped: Vec<String>,
}

/// DataGrip config directories (one per installed version), e.g.
/// `~/Library/Application Support/JetBrains/DataGrip2026.1`.
///
/// ponytail: macOS path only. Linux (`~/.config/JetBrains`) / Windows are
/// follow-ups.
fn config_dirs() -> Vec<PathBuf> {
    let Some(home) = dirs::home_dir() else {
        return Vec::new();
    };
    let jetbrains = home.join("Library/Application Support/JetBrains");
    let Ok(entries) = std::fs::read_dir(&jetbrains) else {
        return Vec::new();
    };
    entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .map(|n| {
                    n.to_string_lossy()
                        .to_ascii_lowercase()
                        .starts_with("datagrip")
                })
                .unwrap_or(false)
        })
        .collect()
}

/// Project directories DataGrip has open/recent, read from each version's
/// `options/recentProjects.xml`. DataGrip stores data sources per project under
/// `<project>/.idea/`, not in a global file.
fn recent_projects(config: &std::path::Path) -> Vec<PathBuf> {
    let Some(home) = dirs::home_dir() else {
        return Vec::new();
    };
    let Ok(xml) = std::fs::read_to_string(config.join("options/recentProjects.xml")) else {
        return Vec::new();
    };
    // Paths appear as attribute values like "$USER_HOME$/Developer/Bungy/Data".
    // A small substring scan is more robust here than full XML walking.
    let mut out = Vec::new();
    for chunk in xml.split("$USER_HOME$/").skip(1) {
        if let Some(rel) = chunk.split('"').next() {
            out.push(home.join(rel));
        }
    }
    out
}

/// Unique project directories DataGrip has open across all installed versions.
fn discover_projects() -> Vec<PathBuf> {
    let mut projects = Vec::new();
    let mut seen = HashSet::new();
    for config in config_dirs() {
        for project in recent_projects(&config) {
            if seen.insert(project.clone()) {
                projects.push(project);
            }
        }
    }
    projects
}

/// The `dataSources.xml` files in a project's `.idea`: the single-file store
/// plus the multi-file `.idea/dataSources/*.xml` entries (minus the
/// `data_sources_history.xml` recent-query log).
fn data_source_files(idea: &std::path::Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let single = idea.join("dataSources.xml");
    if single.is_file() {
        files.push(single);
    }
    if let Ok(entries) = std::fs::read_dir(idea.join("dataSources")) {
        for e in entries.flatten() {
            let p = e.path();
            let is_xml = p.extension().map(|x| x == "xml").unwrap_or(false);
            let is_history = p
                .file_name()
                .map(|n| n == "data_sources_history.xml")
                .unwrap_or(false);
            if is_xml && !is_history {
                files.push(p);
            }
        }
    }
    files
}

/// Scan the local machine for DataGrip connections, de-duplicating by id (the
/// same project can be listed under several DataGrip versions).
pub fn scan() -> DatagripImport {
    let mut connections = Vec::new();
    let mut skipped = Vec::new();
    let mut seen = HashSet::new();
    for project in discover_projects() {
        let idea = project.join(".idea");
        // Usernames live in dataSources.local.xml (keyed by uuid), not in
        // dataSources.xml — read them first so we can fill them in below.
        let users = std::fs::read_to_string(idea.join("dataSources.local.xml"))
            .map(|x| parse_user_names(&x))
            .unwrap_or_default();
        for path in data_source_files(&idea) {
            let Ok(xml) = std::fs::read_to_string(&path) else {
                continue;
            };
            let parsed = parse_data_sources(&xml, &users);
            for c in parsed.connections {
                if seen.insert(c.id.clone()) {
                    connections.push(c);
                }
            }
            skipped.extend(parsed.skipped);
        }
    }
    connections.sort_by(|a, b| a.name.cmp(&b.name));
    DatagripImport {
        connections,
        skipped,
    }
}

/// Map `uuid -> user-name` from a `dataSources.local.xml` document. DataGrip
/// stores the username here, separate from the connection config.
pub fn parse_user_names(xml: &str) -> HashMap<String, String> {
    let mut reader = Reader::from_str(xml);
    let mut map = HashMap::new();
    let mut uuid: Option<String> = None;
    let mut text: Option<String> = None;
    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) | Ok(Event::Empty(e)) => match e.local_name().as_ref() {
                b"data-source" => uuid = attr(&e, b"uuid"),
                b"user-name" => text = Some(String::new()),
                _ => {}
            },
            Ok(Event::Text(t)) => {
                if let Some(buf) = text.as_mut() {
                    buf.push_str(&t.xml_content().unwrap_or_default());
                }
            }
            Ok(Event::End(e)) if e.local_name().as_ref() == b"user-name" => {
                if let (Some(id), Some(u)) = (uuid.as_ref(), text.take()) {
                    if !u.is_empty() {
                        map.insert(id.clone(), u);
                    }
                }
            }
            Ok(Event::Eof) => break,
            Ok(_) => {}
            Err(_) => break,
        }
    }
    map
}

/// Parse a `dataSources.xml` document into importable connections. `users` maps
/// uuid -> user-name from the sibling `dataSources.local.xml`, used when a
/// data-source has no inline `<user-name>`.
pub fn parse_data_sources(xml: &str, users: &HashMap<String, String>) -> DatagripImport {
    let mut reader = Reader::from_str(xml);
    let mut connections = Vec::new();
    let mut skipped = Vec::new();

    // Accumulated state for the <data-source> we are currently inside.
    let mut name: Option<String> = None;
    let mut uuid: Option<String> = None;
    let mut jdbc_url: Option<String> = None;
    let mut user: Option<String> = None;
    let mut current_text: Option<String> = None; // which child element we're reading
    let mut in_source = false;

    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) | Ok(Event::Empty(e)) => {
                let tag = e.local_name();
                match tag.as_ref() {
                    b"data-source" => {
                        in_source = true;
                        name = attr(&e, b"name");
                        uuid = attr(&e, b"uuid");
                        jdbc_url = None;
                        user = None;
                    }
                    b"jdbc-url" => current_text = Some(String::new()),
                    b"user-name" => current_text = Some(String::new()),
                    _ => {}
                }
            }
            Ok(Event::Text(t)) => {
                if let Some(buf) = current_text.as_mut() {
                    buf.push_str(&t.xml_content().unwrap_or_default());
                }
            }
            Ok(Event::End(e)) => match e.local_name().as_ref() {
                b"jdbc-url" => jdbc_url = current_text.take().filter(|s| !s.is_empty()),
                b"user-name" => user = current_text.take().filter(|s| !s.is_empty()),
                b"data-source" if in_source => {
                    in_source = false;
                    let label = name.clone().unwrap_or_else(|| "unnamed".into());
                    // Fall back to the username from dataSources.local.xml.
                    let resolved_user = user
                        .take()
                        .or_else(|| uuid.as_ref().and_then(|id| users.get(id).cloned()));
                    match build_config(name.take(), jdbc_url.take(), resolved_user) {
                        Ok(cfg) => connections.push(cfg),
                        Err(reason) => skipped.push(format!("{label} — {reason}")),
                    }
                }
                _ => {}
            },
            Ok(Event::Eof) => break,
            Ok(_) => {}      // comments, CDATA, declarations: ignore
            Err(_) => break, // malformed XML: keep whatever we parsed so far
        }
    }

    DatagripImport {
        connections,
        skipped,
    }
}

fn attr(e: &quick_xml::events::BytesStart, key: &[u8]) -> Option<String> {
    e.attributes()
        .flatten()
        .find(|a| a.key.as_ref() == key)
        .map(|a| String::from_utf8_lossy(&a.value).into_owned())
}

fn build_config(
    name: Option<String>,
    jdbc_url: Option<String>,
    user: Option<String>,
) -> Result<ConnectionConfig, String> {
    let url = jdbc_url.ok_or("no JDBC URL")?;
    let (engine, host, port, database) = parse_jdbc_url(&url)?;
    let name = name.unwrap_or_else(|| format!("{host}/{database}"));
    Ok(ConnectionConfig {
        id: slugify(&name),
        name,
        engine,
        host,
        port,
        database,
        user: user.unwrap_or_default(),
        // DataGrip's SSL settings live elsewhere; default to prefer like the
        // new-connection form does.
        ssl_mode: SslMode::Prefer,
        env_tag: None,
        application_name: Some("cellar".into()),
        color: None,
    })
}

/// Map a JDBC URL to `(engine, host, port, database)`, or `Err(reason)` if the
/// engine isn't one Cellar can connect to.
fn parse_jdbc_url(url: &str) -> Result<(Engine, String, u16, String), String> {
    let rest = url.strip_prefix("jdbc:").unwrap_or(url);
    let (scheme, after) = rest
        .split_once("://")
        .ok_or_else(|| format!("unsupported URL: {url}"))?;

    let (mut engine, default_port, default_db) = match scheme {
        "postgresql" => (Engine::Postgres, 5432, "postgres"),
        "mysql" | "mariadb" => (Engine::MySql, 3306, "mysql"),
        "sqlserver" => (Engine::Mssql, 1433, "master"),
        other => return Err(format!("unsupported engine: {other}")),
    };

    // SQL Server uses `host[:port];key=value;…`; everyone else uses
    // `host[:port]/database?params`.
    let (authority, database) = if engine == Engine::Mssql {
        let (auth, props) = after.split_once(';').unwrap_or((after, ""));
        let db = props
            .split(';')
            .filter_map(|kv| kv.split_once('='))
            .find(|(k, _)| {
                let k = k.trim().to_ascii_lowercase();
                k == "databasename" || k == "database"
            })
            .map(|(_, v)| v.trim().to_string());
        (auth, db.filter(|d| !d.is_empty()))
    } else {
        let (auth, tail) = after.split_once('/').unwrap_or((after, ""));
        let db = tail.split(['?', ';']).next().unwrap_or("").to_string();
        (auth, Some(db).filter(|d| !d.is_empty()))
    };

    let (host, port) = match authority.rsplit_once(':') {
        Some((h, p)) => (h.to_string(), p.parse().unwrap_or(default_port)),
        None => (authority.to_string(), default_port),
    };
    let host = if host.is_empty() {
        "localhost".to_string()
    } else {
        host
    };

    // Azure SQL is the same wire protocol but a distinct Cellar engine; route
    // *.database.windows.net hosts there so they connect with the right driver.
    if engine == Engine::Mssql && host.to_ascii_lowercase().ends_with(".database.windows.net") {
        engine = Engine::Azure;
    }

    Ok((
        engine,
        host,
        port,
        database.unwrap_or_else(|| default_db.to_string()),
    ))
}

/// Mirror of the frontend `slugify` so imported ids match what the dialog would
/// produce for the same name.
fn slugify(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut prev_dash = false;
    for c in s.to_ascii_lowercase().chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c);
            prev_dash = false;
        } else if !prev_dash {
            out.push('-');
            prev_dash = true;
        }
    }
    let slug: String = out.trim_matches('-').chars().take(64).collect();
    if slug.is_empty() {
        "connection".into()
    } else {
        slug
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_postgres_mysql_and_sqlserver() {
        let xml = r#"
        <component name="DataSourceManagerImpl">
          <data-source name="Prod PG">
            <jdbc-url>jdbc:postgresql://db.example.com:6543/shop?sslmode=require</jdbc-url>
            <user-name>app</user-name>
          </data-source>
          <data-source name="Local MySQL">
            <jdbc-url>jdbc:mysql://localhost/store</jdbc-url>
            <user-name>root</user-name>
          </data-source>
          <data-source name="MSSQL">
            <jdbc-url>jdbc:sqlserver://winhost:1433;databaseName=Sales;encrypt=true</jdbc-url>
            <user-name>sa</user-name>
          </data-source>
          <data-source name="Old Oracle">
            <jdbc-url>jdbc:oracle:thin:@host:1521:orcl</jdbc-url>
          </data-source>
        </component>"#;

        let r = parse_data_sources(xml, &HashMap::new());
        assert_eq!(r.connections.len(), 3, "3 supported engines imported");
        assert_eq!(r.skipped.len(), 1, "oracle skipped");
        assert!(r.skipped[0].contains("Old Oracle"));

        let pg = &r.connections[0];
        assert_eq!(pg.engine, Engine::Postgres);
        assert_eq!(pg.host, "db.example.com");
        assert_eq!(pg.port, 6543);
        assert_eq!(pg.database, "shop"); // query params stripped
        assert_eq!(pg.user, "app");
        assert_eq!(pg.id, "prod-pg");

        let my = &r.connections[1];
        assert_eq!(my.engine, Engine::MySql);
        assert_eq!(my.port, 3306); // default applied
        assert_eq!(my.database, "store");

        let ms = &r.connections[2];
        assert_eq!(ms.engine, Engine::Mssql);
        assert_eq!(ms.host, "winhost");
        assert_eq!(ms.database, "Sales"); // pulled from ;databaseName=
    }

    #[test]
    fn empty_database_falls_back_to_engine_default() {
        let xml = r#"<root><data-source name="bare">
            <jdbc-url>jdbc:postgresql://localhost/</jdbc-url>
        </data-source></root>"#;
        let r = parse_data_sources(xml, &HashMap::new());
        assert_eq!(r.connections[0].database, "postgres");
    }

    #[test]
    fn user_name_joined_from_local_file_by_uuid() {
        // DataGrip's real layout: dataSources.xml has no <user-name>, it lives
        // in dataSources.local.xml keyed by the same uuid.
        let sources = r#"<root>
          <data-source name="Epic Prod V2" uuid="d7fb-2dcd">
            <jdbc-url>jdbc:sqlserver://epicprod.database.windows.net:1433</jdbc-url>
          </data-source>
        </root>"#;
        let local = r#"<root>
          <data-source name="Epic Prod V2" uuid="d7fb-2dcd">
            <secret-storage>master_key</secret-storage>
            <user-name>dbadmin</user-name>
          </data-source>
        </root>"#;

        let users = parse_user_names(local);
        assert_eq!(users.get("d7fb-2dcd").map(String::as_str), Some("dbadmin"));

        let r = parse_data_sources(sources, &users);
        let c = &r.connections[0];
        assert_eq!(c.user, "dbadmin");
        assert_eq!(c.engine, Engine::Azure); // *.database.windows.net
    }
}
