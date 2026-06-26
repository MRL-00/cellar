//! Developer-convenience dump & restore for Postgres (W2, first slice).
//!
//! This is deliberately NOT backup orchestration (SPEC §2 non-goal). It shells
//! out to the PostgreSQL client tools — `pg_dump` for a table/schema dump and
//! `psql` for restoring a plain-SQL file — streaming bytes to/from disk so even
//! a multi-GB dump never lands in memory. Progress is pushed over a typed
//! `Channel`; an in-flight operation can be cancelled by id.
//!
//! Safety properties:
//! - The DB password is passed to the child via the `PGPASSWORD` env var, never
//!   on the command line and never written to disk.
//! - The child is spawned with an explicit binary path and an argument vector —
//!   there is no shell, so identifiers can't be interpolated into a command
//!   string. Scope identifiers are still validated and double-quoted.
//! - Restore is plain-SQL only and runs `psql --single-transaction
//!   -v ON_ERROR_STOP=1`, so a failure rolls back rather than half-applying.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;

use cellar_core::driver::{ConnectionConfig, Engine, SslMode};
use cellar_core::error::CellarError;
use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::ipc::Channel;
use tauri::State;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::process::Command;
use tokio::sync::{Mutex, Notify};

use crate::state::ConnectionRegistry;

/// Emit a progress event roughly every megabyte rather than per 64 KiB read,
/// so a large transfer doesn't flood the IPC channel.
const PROGRESS_INTERVAL_BYTES: u64 = 1 << 20;
const CHUNK_BYTES: usize = 64 * 1024;

/// What to dump. Whole-database dumps are intentionally out of the v1 slice —
/// scope is a single table or a single schema.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum DumpScope {
    Table {
        database: String,
        schema: String,
        table: String,
    },
    Schema {
        database: String,
        schema: String,
    },
}

impl DumpScope {
    fn database(&self) -> &str {
        match self {
            DumpScope::Table { database, .. } | DumpScope::Schema { database, .. } => database,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Type)]
#[serde(rename_all = "kebab-case")]
pub enum DumpContents {
    SchemaOnly,
    DataOnly,
    Both,
}

/// Pushed over the IPC `Channel` as the transfer makes progress. `bytes` is the
/// running count written to (dump) or read from (restore) disk.
#[derive(Debug, Clone, Serialize, Type)]
pub struct TransferProgress {
    pub bytes: u64,
}

#[derive(Debug, Clone, Serialize, Type)]
pub struct DumpSummary {
    pub path: String,
    pub bytes: u64,
}

#[derive(Debug, Clone, Serialize, Type)]
pub struct RestoreSummary {
    pub bytes: u64,
}

/// Connection parameters pg_dump/psql need, minus the per-operation database
/// (which comes from the dump scope, so the same connection can target any of
/// its databases). The password is handled separately via the environment.
#[derive(Debug, Clone)]
struct PgTarget {
    host: String,
    port: u16,
    user: String,
    ssl_mode: SslMode,
}

impl From<&ConnectionConfig> for PgTarget {
    fn from(c: &ConnectionConfig) -> Self {
        Self {
            host: c.host.clone(),
            port: c.port,
            user: c.user.clone(),
            ssl_mode: c.ssl_mode,
        }
    }
}

/// Tracks cancellation signals for in-flight dump/restore operations, keyed by
/// the frontend-supplied `operation_id`. A `Notify` is enough: the running task
/// awaits it and kills its child process when notified.
#[derive(Default)]
pub struct DumpRegistry {
    inner: Mutex<HashMap<String, Arc<Notify>>>,
}

impl DumpRegistry {
    async fn register(&self, id: &str) -> Arc<Notify> {
        let notify = Arc::new(Notify::new());
        self.inner.lock().await.insert(id.to_string(), notify.clone());
        notify
    }

    async fn unregister(&self, id: &str) {
        self.inner.lock().await.remove(id);
    }

    async fn cancel(&self, id: &str) -> bool {
        match self.inner.lock().await.get(id) {
            Some(notify) => {
                notify.notify_one();
                true
            }
            None => false,
        }
    }
}

#[tauri::command]
#[specta::specta]
pub async fn dump_postgres(
    registry: State<'_, ConnectionRegistry>,
    dumps: State<'_, DumpRegistry>,
    connection_id: String,
    scope: DumpScope,
    contents: DumpContents,
    dest_path: String,
    operation_id: String,
    on_progress: Channel<TransferProgress>,
) -> Result<DumpSummary, CellarError> {
    let target = postgres_target(&registry, &connection_id).await?;
    let password = cellar_secrets::load(&connection_id).ok().flatten();
    validate_scope(&scope)?;

    let cancel = dumps.register(&operation_id).await;
    let result = run_dump(
        &target,
        password.as_deref(),
        &scope,
        contents,
        Path::new(&dest_path),
        &on_progress,
        cancel,
    )
    .await;
    dumps.unregister(&operation_id).await;

    let bytes = result?;
    Ok(DumpSummary {
        path: dest_path,
        bytes,
    })
}

#[tauri::command]
#[specta::specta]
pub async fn restore_postgres(
    registry: State<'_, ConnectionRegistry>,
    dumps: State<'_, DumpRegistry>,
    connection_id: String,
    database: String,
    source_path: String,
    operation_id: String,
    on_progress: Channel<TransferProgress>,
) -> Result<RestoreSummary, CellarError> {
    let target = postgres_target(&registry, &connection_id).await?;
    let password = cellar_secrets::load(&connection_id).ok().flatten();
    if database.trim().is_empty() {
        return Err(CellarError::invalid_config("target database is empty"));
    }

    let cancel = dumps.register(&operation_id).await;
    let result = run_restore(
        &target,
        password.as_deref(),
        &database,
        Path::new(&source_path),
        &on_progress,
        cancel,
    )
    .await;
    dumps.unregister(&operation_id).await;

    let bytes = result?;
    Ok(RestoreSummary { bytes })
}

/// Cancel an in-flight dump or restore. Returns `true` when a running operation
/// with that id was signalled, `false` when it had already finished.
#[tauri::command]
#[specta::specta]
pub async fn cancel_dump(
    dumps: State<'_, DumpRegistry>,
    operation_id: String,
) -> Result<bool, CellarError> {
    Ok(dumps.cancel(&operation_id).await)
}

async fn postgres_target(
    registry: &ConnectionRegistry,
    connection_id: &str,
) -> Result<PgTarget, CellarError> {
    let config = registry.connection_config(connection_id).await?;
    if config.engine != Engine::Postgres {
        return Err(CellarError::invalid_config(format!(
            "dump & restore is only available for Postgres in this version, not {}",
            config.engine.as_str()
        )));
    }
    Ok(PgTarget::from(&config))
}

/// Spawn `pg_dump`, streaming its stdout to `dest`. Returns the byte count
/// written. On failure or cancellation the partial file is removed so a failed
/// dump never looks like a usable one.
async fn run_dump(
    target: &PgTarget,
    password: Option<&str>,
    scope: &DumpScope,
    contents: DumpContents,
    dest: &Path,
    on_progress: &Channel<TransferProgress>,
    cancel: Arc<Notify>,
) -> Result<u64, CellarError> {
    let bin = resolve_binary("pg_dump")?;
    let args = pg_dump_args(target, scope, contents);

    let mut cmd = Command::new(&bin);
    cmd.args(&args);
    apply_pg_env(&mut cmd, target, password);
    cmd.stdin(Stdio::null());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    let mut child = cmd.spawn().map_err(|e| spawn_error("pg_dump", &bin, e))?;
    let mut stdout = child.stdout.take().expect("piped stdout");
    let stderr = child.stderr.take().expect("piped stderr");
    let stderr_task = tokio::spawn(drain_to_string(stderr));

    let mut file = tokio::fs::File::create(dest)
        .await
        .map_err(|e| CellarError::Io(format!("could not create {}: {e}", dest.display())))?;

    let mut buf = vec![0u8; CHUNK_BYTES];
    let mut total: u64 = 0;
    let mut last_emit: u64 = 0;
    let cancelled;
    loop {
        tokio::select! {
            biased;
            _ = cancel.notified() => { cancelled = true; break; }
            read = stdout.read(&mut buf) => {
                let n = read.map_err(|e| CellarError::Io(e.to_string()))?;
                if n == 0 { cancelled = false; break; }
                file.write_all(&buf[..n])
                    .await
                    .map_err(|e| CellarError::Io(format!("write to {} failed: {e}", dest.display())))?;
                total += n as u64;
                if total - last_emit >= PROGRESS_INTERVAL_BYTES {
                    last_emit = total;
                    let _ = on_progress.send(TransferProgress { bytes: total });
                }
            }
        }
    }

    if cancelled {
        let _ = child.start_kill();
        let _ = child.wait().await;
        let _ = tokio::fs::remove_file(dest).await;
        return Err(CellarError::Io("dump cancelled".into()));
    }

    file.flush()
        .await
        .map_err(|e| CellarError::Io(e.to_string()))?;
    let status = child
        .wait()
        .await
        .map_err(|e| CellarError::Io(e.to_string()))?;
    let stderr_text = stderr_task.await.unwrap_or_default();

    if !status.success() {
        let _ = tokio::fs::remove_file(dest).await;
        return Err(classify_pg_error("pg_dump", &stderr_text));
    }
    let _ = on_progress.send(TransferProgress { bytes: total });
    Ok(total)
}

/// Spawn `psql`, streaming the dump file into its stdin. Returns the byte count
/// read. Errors from psql surface its stderr verbatim (sans secrets — the
/// password is never echoed there).
async fn run_restore(
    target: &PgTarget,
    password: Option<&str>,
    database: &str,
    source: &Path,
    on_progress: &Channel<TransferProgress>,
    cancel: Arc<Notify>,
) -> Result<u64, CellarError> {
    let bin = resolve_binary("psql")?;
    let args = psql_args(target, database);

    let mut file = tokio::fs::File::open(source)
        .await
        .map_err(|e| CellarError::Io(format!("could not open {}: {e}", source.display())))?;

    let mut cmd = Command::new(&bin);
    cmd.args(&args);
    apply_pg_env(&mut cmd, target, password);
    cmd.stdin(Stdio::piped());
    cmd.stdout(Stdio::null());
    cmd.stderr(Stdio::piped());

    let mut child = cmd.spawn().map_err(|e| spawn_error("psql", &bin, e))?;
    let mut stdin = child.stdin.take().expect("piped stdin");
    let stderr = child.stderr.take().expect("piped stderr");
    let stderr_task = tokio::spawn(drain_to_string(stderr));

    let mut buf = vec![0u8; CHUNK_BYTES];
    let mut total: u64 = 0;
    let mut last_emit: u64 = 0;
    let mut cancelled = false;
    loop {
        tokio::select! {
            biased;
            _ = cancel.notified() => { cancelled = true; break; }
            read = file.read(&mut buf) => {
                let n = read.map_err(|e| CellarError::Io(e.to_string()))?;
                if n == 0 { break; }
                // A broken pipe means psql exited early (e.g. ON_ERROR_STOP
                // tripped). Stop feeding it and let wait()/stderr report the
                // real SQL error rather than a generic write failure.
                if stdin.write_all(&buf[..n]).await.is_err() {
                    break;
                }
                total += n as u64;
                if total - last_emit >= PROGRESS_INTERVAL_BYTES {
                    last_emit = total;
                    let _ = on_progress.send(TransferProgress { bytes: total });
                }
            }
        }
    }
    // Close psql's stdin so it processes the input and exits.
    drop(stdin);

    if cancelled {
        let _ = child.start_kill();
        let _ = child.wait().await;
        return Err(CellarError::Io("restore cancelled".into()));
    }

    let status = child
        .wait()
        .await
        .map_err(|e| CellarError::Io(e.to_string()))?;
    let stderr_text = stderr_task.await.unwrap_or_default();

    if !status.success() {
        return Err(classify_pg_error("psql", &stderr_text));
    }
    let _ = on_progress.send(TransferProgress { bytes: total });
    Ok(total)
}

async fn drain_to_string<R: tokio::io::AsyncRead + Unpin>(mut reader: R) -> String {
    let mut out = Vec::new();
    let _ = reader.read_to_end(&mut out).await;
    String::from_utf8_lossy(&out).into_owned()
}

fn apply_pg_env(cmd: &mut Command, target: &PgTarget, password: Option<&str>) {
    cmd.env("PGSSLMODE", ssl_mode_str(target.ssl_mode));
    cmd.env("PGCONNECT_TIMEOUT", "10");
    // Belt-and-braces: --no-password already disables the prompt, but clearing
    // any inherited PGPASSWORD avoids leaking an unrelated secret to the child.
    cmd.env_remove("PGPASSWORD");
    if let Some(p) = password {
        cmd.env("PGPASSWORD", p);
    }
}

/// pg_dump argument vector (binary excluded). Pure so it can be unit-tested.
fn pg_dump_args(target: &PgTarget, scope: &DumpScope, contents: DumpContents) -> Vec<String> {
    let mut args = vec![
        format!("--host={}", target.host),
        format!("--port={}", target.port),
        format!("--username={}", target.user),
        "--no-password".to_string(),
        "--format=plain".to_string(),
        // Dev-convenience dumps move between databases/roles, so strip
        // ownership and grants — they're the usual cause of restore failures
        // into a differently-owned database.
        "--no-owner".to_string(),
        "--no-privileges".to_string(),
    ];
    match contents {
        DumpContents::SchemaOnly => args.push("--schema-only".to_string()),
        DumpContents::DataOnly => args.push("--data-only".to_string()),
        DumpContents::Both => {}
    }
    match scope {
        DumpScope::Table { schema, table, .. } => args.push(format!(
            "--table={}.{}",
            quote_ident(schema),
            quote_ident(table)
        )),
        DumpScope::Schema { schema, .. } => args.push(format!("--schema={}", quote_ident(schema))),
    }
    // Positional connection database. A pg_dump pattern (--table/--schema) is
    // matched within this database.
    args.push(scope.database().to_string());
    args
}

/// psql argument vector for a restore (binary excluded). SQL is fed on stdin.
fn psql_args(target: &PgTarget, database: &str) -> Vec<String> {
    vec![
        format!("--host={}", target.host),
        format!("--port={}", target.port),
        format!("--username={}", target.user),
        format!("--dbname={}", database),
        "--no-password".to_string(),
        "--quiet".to_string(),
        "--no-psqlrc".to_string(),
        // Abort and roll back the whole restore on the first error rather than
        // limping on and half-applying the dump.
        "-v".to_string(),
        "ON_ERROR_STOP=1".to_string(),
        "--single-transaction".to_string(),
    ]
}

fn ssl_mode_str(mode: SslMode) -> &'static str {
    match mode {
        SslMode::Disable => "disable",
        SslMode::Prefer => "prefer",
        SslMode::Require => "require",
        SslMode::VerifyCa => "verify-ca",
        SslMode::VerifyFull => "verify-full",
    }
}

/// Double-quote a Postgres identifier for use in a pg_dump `--table`/`--schema`
/// pattern, doubling embedded quotes. Quoting forces an exact, case-sensitive
/// match (pg_dump otherwise treats the pattern as a regex).
fn quote_ident(ident: &str) -> String {
    format!("\"{}\"", ident.replace('"', "\"\""))
}

fn validate_scope(scope: &DumpScope) -> Result<(), CellarError> {
    match scope {
        DumpScope::Table {
            database,
            schema,
            table,
        } => {
            validate_ident("database", database)?;
            validate_ident("schema", schema)?;
            validate_ident("table", table)?;
        }
        DumpScope::Schema { database, schema } => {
            validate_ident("database", database)?;
            validate_ident("schema", schema)?;
        }
    }
    Ok(())
}

fn validate_ident(label: &str, value: &str) -> Result<(), CellarError> {
    if value.trim().is_empty() {
        return Err(CellarError::invalid_config(format!("{label} is empty")));
    }
    // No shell is involved, but a NUL or newline in an identifier is never
    // legitimate and would corrupt the argument vector.
    if value.contains('\0') || value.contains('\n') {
        return Err(CellarError::invalid_config(format!(
            "{label} contains an illegal character"
        )));
    }
    Ok(())
}

/// PostgreSQL client tools are frequently installed outside the default PATH
/// (Homebrew keeps `libpq` keg-only; Postgres.app lives under /Applications),
/// so fall back to a few well-known locations before giving up.
const COMMON_PG_BIN_DIRS: &[&str] = &[
    "/opt/homebrew/opt/libpq/bin",
    "/usr/local/opt/libpq/bin",
    "/opt/homebrew/bin",
    "/usr/local/bin",
    "/usr/bin",
    "/usr/lib/postgresql/16/bin",
    "/usr/lib/postgresql/15/bin",
    "/Applications/Postgres.app/Contents/Versions/latest/bin",
];

fn resolve_binary(name: &str) -> Result<PathBuf, CellarError> {
    if let Ok(path) = std::env::var("PATH") {
        if let Some(found) = find_on_path(name, &path) {
            return Ok(found);
        }
    }
    for dir in COMMON_PG_BIN_DIRS {
        let candidate = Path::new(dir).join(exe_name(name));
        if candidate.is_file() {
            return Ok(candidate);
        }
    }
    Err(missing_binary_error(name))
}

fn find_on_path(name: &str, path_var: &str) -> Option<PathBuf> {
    let exe = exe_name(name);
    std::env::split_paths(path_var)
        .map(|dir| dir.join(&exe))
        .find(|candidate| candidate.is_file())
}

fn exe_name(name: &str) -> String {
    if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.to_string()
    }
}

fn missing_binary_error(name: &str) -> CellarError {
    CellarError::Io(format!(
        "{name} was not found. Install the PostgreSQL client tools (`brew install libpq` on macOS, \
         `apt install postgresql-client` on Linux, or the EDB installer on Windows) and make sure \
         {name} is on your PATH."
    ))
}

fn spawn_error(name: &str, bin: &Path, err: std::io::Error) -> CellarError {
    if err.kind() == std::io::ErrorKind::NotFound {
        return missing_binary_error(name);
    }
    CellarError::Io(format!("could not run {} ({}): {err}", name, bin.display()))
}

/// Map a failed child's stderr onto a typed error, keeping the original text so
/// version mismatch, permission, and SQL errors all reach the user verbatim.
fn classify_pg_error(tool: &str, stderr: &str) -> CellarError {
    let trimmed = stderr.trim();
    let detail = if trimmed.is_empty() {
        format!("{tool} failed with no diagnostic output")
    } else {
        format!("{tool} failed:\n{trimmed}")
    };
    let lower = trimmed.to_lowercase();
    if lower.contains("authentication failed")
        || lower.contains("no password supplied")
        || lower.contains("password authentication")
    {
        CellarError::Authentication(detail)
    } else {
        // Version mismatch, permission denied, and SQL errors are all clearest
        // as the tool's own message under the Query surface.
        CellarError::Query(detail)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn target() -> PgTarget {
        PgTarget {
            host: "db.example.com".into(),
            port: 5433,
            user: "svc".into(),
            ssl_mode: SslMode::Require,
        }
    }

    #[test]
    fn dump_args_for_table_both() {
        let scope = DumpScope::Table {
            database: "shop".into(),
            schema: "public".into(),
            table: "orders".into(),
        };
        let args = pg_dump_args(&target(), &scope, DumpContents::Both);
        assert!(args.contains(&"--host=db.example.com".to_string()));
        assert!(args.contains(&"--port=5433".to_string()));
        assert!(args.contains(&"--username=svc".to_string()));
        assert!(args.contains(&"--no-password".to_string()));
        assert!(args.contains(&"--format=plain".to_string()));
        assert!(args.contains(&"--no-owner".to_string()));
        assert!(args.contains(&"--table=\"public\".\"orders\"".to_string()));
        // Both contents => neither restricting flag.
        assert!(!args.contains(&"--schema-only".to_string()));
        assert!(!args.contains(&"--data-only".to_string()));
        // Positional dbname is last.
        assert_eq!(args.last().unwrap(), "shop");
    }

    #[test]
    fn dump_args_for_schema_only() {
        let scope = DumpScope::Schema {
            database: "shop".into(),
            schema: "analytics".into(),
        };
        let args = pg_dump_args(&target(), &scope, DumpContents::SchemaOnly);
        assert!(args.contains(&"--schema-only".to_string()));
        assert!(args.contains(&"--schema=\"analytics\"".to_string()));
        assert!(!args.iter().any(|a| a.starts_with("--table=")));
    }

    #[test]
    fn dump_args_data_only_flag() {
        let scope = DumpScope::Schema {
            database: "shop".into(),
            schema: "public".into(),
        };
        let args = pg_dump_args(&target(), &scope, DumpContents::DataOnly);
        assert!(args.contains(&"--data-only".to_string()));
        assert!(!args.contains(&"--schema-only".to_string()));
    }

    #[test]
    fn quote_ident_doubles_embedded_quotes() {
        assert_eq!(quote_ident("orders"), "\"orders\"");
        assert_eq!(quote_ident("we\"ird"), "\"we\"\"ird\"");
        // A quoting attempt can't break out of the quoted pattern.
        assert_eq!(quote_ident("a\".\"b"), "\"a\"\".\"\"b\"");
    }

    #[test]
    fn psql_args_are_transactional_and_stop_on_error() {
        let args = psql_args(&target(), "shop_copy");
        assert!(args.contains(&"--dbname=shop_copy".to_string()));
        assert!(args.contains(&"--single-transaction".to_string()));
        assert!(args.contains(&"ON_ERROR_STOP=1".to_string()));
        assert!(args.contains(&"--no-password".to_string()));
    }

    #[test]
    fn ssl_mode_maps_to_libpq_values() {
        assert_eq!(ssl_mode_str(SslMode::Disable), "disable");
        assert_eq!(ssl_mode_str(SslMode::Prefer), "prefer");
        assert_eq!(ssl_mode_str(SslMode::Require), "require");
        assert_eq!(ssl_mode_str(SslMode::VerifyCa), "verify-ca");
        assert_eq!(ssl_mode_str(SslMode::VerifyFull), "verify-full");
    }

    #[test]
    fn validate_scope_rejects_empty_and_control_chars() {
        assert!(validate_scope(&DumpScope::Schema {
            database: "shop".into(),
            schema: "  ".into(),
        })
        .is_err());
        assert!(validate_scope(&DumpScope::Table {
            database: "shop".into(),
            schema: "public".into(),
            table: "bad\nname".into(),
        })
        .is_err());
        assert!(validate_scope(&DumpScope::Table {
            database: "shop".into(),
            schema: "public".into(),
            table: "orders".into(),
        })
        .is_ok());
    }

    #[test]
    fn missing_binary_error_is_actionable() {
        let err = missing_binary_error("pg_dump");
        let msg = err.to_string();
        assert!(msg.contains("pg_dump"));
        assert!(msg.contains("PATH"));
    }

    #[test]
    fn find_on_path_locates_an_executable() {
        let dir = tempfile::tempdir().unwrap();
        let name = "pg_dump";
        let file = dir.path().join(exe_name(name));
        std::fs::write(&file, b"#!/bin/sh\n").unwrap();
        let path_var = dir.path().to_str().unwrap();
        assert_eq!(find_on_path(name, path_var), Some(file));
        // A directory that doesn't contain it yields nothing.
        let empty = tempfile::tempdir().unwrap();
        assert_eq!(find_on_path(name, empty.path().to_str().unwrap()), None);
    }

    #[test]
    fn classify_auth_vs_other() {
        assert!(matches!(
            classify_pg_error("pg_dump", "FATAL: password authentication failed for user"),
            CellarError::Authentication(_)
        ));
        assert!(matches!(
            classify_pg_error("pg_dump", "pg_dump: error: server version mismatch"),
            CellarError::Query(_)
        ));
    }
}
