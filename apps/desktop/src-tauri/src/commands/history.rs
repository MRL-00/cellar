use cellar_core::error::CellarError;
use tauri::State;

use crate::history::{HistoryStore, QueryHistoryFilter, QueryHistoryRecord};

#[tauri::command]
#[specta::specta]
pub async fn list_query_history(
    history: State<'_, HistoryStore>,
    connection_id: Option<String>,
    database: Option<String>,
    tab_id: Option<String>,
    search: Option<String>,
    limit: Option<u32>,
) -> Result<Vec<QueryHistoryRecord>, CellarError> {
    history
        .list(QueryHistoryFilter {
            connection_id,
            database,
            tab_id,
            search,
            limit,
        })
        .await
}
