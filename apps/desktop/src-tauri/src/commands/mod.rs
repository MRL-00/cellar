//! Tauri command surface, grouped by feature per SPEC §5.4.
//!
//! Adding a command:
//! 1. Define the `#[tauri::command]` `#[specta::specta]` function here.
//! 2. Add it to [`collect`] below so `tauri-specta` generates a typed wrapper
//!    and the invoke handler picks it up.

pub mod ai;
pub mod connection;
pub mod export;
pub mod history;
pub mod query;
pub mod schema;
pub mod schema_diff;
pub mod templates;
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
        connection::import_datagrip,
        schema::introspect,
        schema::find_usages,
        schema::er_graph,
        query::run_query,
        query::run_read_only_query,
        query::cancel_query,
        query::explain_query,
        query::detect_query_parameters,
        query::browse_table,
        transaction::preview_table_changes,
        transaction::commit_table_changes,
        transaction::commit_table_import,
        schema_diff::compare_schemas,
        schema_diff::build_migration_script,
        schema_diff::apply_migration,
        schema_diff::save_schema_snapshot,
        schema_diff::list_schema_snapshots,
        schema_diff::delete_schema_snapshot,
        history::list_query_history,
        templates::list_query_templates,
        templates::save_query_template,
        templates::delete_query_template,
        export::save_text_file,
        ai::ai_store_key,
        ai::ai_load_key,
        ai::ai_delete_key,
        ai::ai_has_key,
        ai::ai_backend_list_models,
        ai::ai_backend_generate,
        ai::ai_openai_oauth_status,
        ai::ai_openai_start_login,
        ai::ai_openai_cancel_login,
        ai::ai_openai_logout,
        ai::ai_openai_list_models,
        ai::ai_openai_generate,
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
