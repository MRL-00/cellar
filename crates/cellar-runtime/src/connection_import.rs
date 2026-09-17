//! Shared result type for the one-click connection importers (DataGrip,
//! TablePlus, …). Every importer reads another app's on-disk config, maps what
//! it can to [`ConnectionConfig`], and reports the rest as human-readable
//! reasons rather than dropping rows silently.

use std::collections::HashMap;

use cellar_core::driver::ConnectionConfig;
use serde::{Deserialize, Serialize};
use specta::Type;

/// Result of scanning another database client for importable connections.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct ConnectionImport {
    /// Connections we could map. Passwords are never included — the caller
    /// collects them separately (import-time form or on first connect).
    pub connections: Vec<ConnectionConfig>,
    /// Connections we found but could not import, each with a human reason
    /// (unsupported engine, unparseable URL). Surfaced so the import is honest
    /// about what it dropped rather than silently skipping rows.
    pub skipped: Vec<String>,
    /// Folder name for each grouped connection, keyed by connection id. The
    /// caller maps these onto its own sidebar folders. Importers that read a
    /// source without grouping leave this empty.
    pub groups: HashMap<String, String>,
}

/// Canonical connection-id slug used by importers and the native editor.
pub(crate) fn slugify(s: &str) -> String {
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
