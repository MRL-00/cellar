//! Tauri command surface, grouped by feature per SPEC §5.4.
//!
//! Adding a command:
//! 1. Define the `#[tauri::command]` `#[specta::specta]` function here.
//! 2. Add it to [`collect`] below so `tauri-specta` generates a typed wrapper
//!    and the invoke handler picks it up.

pub mod connection;
pub mod history;
pub mod query;
pub mod schema;
pub mod transaction;

use specta_typescript::{BigIntExportBehavior, Typescript};
use tauri_specta::{collect_commands, Builder};

/// Build the `tauri-specta` Builder with every command Cellar exposes. The
/// same builder is consumed by `lib.rs::run` (for `invoke_handler`) and by
/// `bin/codegen.rs` (for the TypeScript export).
pub fn builder() -> Builder<tauri::Wry> {
    Builder::<tauri::Wry>::new().commands(collect_commands![
        connection::list_connections,
        connection::save_connection,
        connection::delete_connection,
        connection::test_connection,
        connection::connect,
        connection::reconnect,
        connection::disconnect,
        schema::introspect,
        query::run_query,
        query::explain_query,
        query::browse_table,
        transaction::preview_table_changes,
        transaction::commit_table_changes,
        history::list_query_history,
    ])
}

pub fn typescript_exporter() -> Typescript {
    // CellValue::Int carries i64 because most database engines use 64-bit
    // integers natively. JS `number` covers anything inside the safe-integer
    // range (±2^53 - 1); the grid renders the rest as monospace text so the
    // raw value is still visible. The frontend can lift these to BigInt
    // later if a real precision problem shows up.
    Typescript::default().bigint(BigIntExportBehavior::Number)
}
