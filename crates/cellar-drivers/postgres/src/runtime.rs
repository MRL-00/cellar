use cellar_core::error::{CellarError, CellarResult};

use crate::connect::PgConnection;

/// Removes a query from the active-query registry on every exit path.
pub(crate) struct RegisteredQuery<'a> {
    pub(crate) conn: &'a PgConnection,
    pub(crate) query_id: &'a str,
}

impl Drop for RegisteredQuery<'_> {
    fn drop(&mut self) {
        self.conn.unregister_query(self.query_id);
    }
}

/// Signal a running backend on a second pool connection.
pub(crate) async fn cancel_query(conn: &PgConnection, query_id: &str) -> CellarResult<bool> {
    let Some(active) = conn.lookup_query(query_id) else {
        return Ok(false);
    };
    let pool = conn.pool_for_database(&active.database).await?;
    sqlx::query_scalar("SELECT pg_cancel_backend($1)")
        .bind(active.pid)
        .fetch_one(&pool)
        .await
        .map_err(|e| {
            crate::connect::map_sqlx_err_for_runtime(e, "query cancellation", CellarError::query)
        })
}

pub(crate) fn query_sqlx_err(err: sqlx::Error) -> CellarError {
    if let sqlx::Error::Database(db) = &err {
        if db.code().as_deref() == Some("57014") {
            return CellarError::Query(db.message().to_string());
        }
    }
    crate::connect::map_sqlx_err_for_runtime(err, "query execution", CellarError::query)
}
