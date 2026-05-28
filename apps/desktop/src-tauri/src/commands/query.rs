use cellar_core::error::CellarError;
use cellar_core::query::{Query, QueryResult};
use tauri::State;

use crate::state::ConnectionRegistry;

#[tauri::command]
#[specta::specta]
pub async fn run_query(
    registry: State<'_, ConnectionRegistry>,
    connection_id: String,
    sql: String,
    max_rows: Option<u32>,
    database: Option<String>,
) -> Result<QueryResult, CellarError> {
    let mut query = Query::new(sql);
    if let Some(n) = max_rows {
        query = query.with_max_rows(n);
    }
    if let Some(db) = database {
        query = query.with_database(db);
    }
    registry.run_query(&connection_id, query).await
}
