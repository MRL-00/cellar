use cellar_core::{
    driver::Engine,
    error::{CellarError, CellarResult},
    query::{ForeignKeyLookupRequest, Query, QueryParam, QueryResult},
    value::CellValue,
};

use super::ConnectionRegistry;

impl ConnectionRegistry {
    /// Execute the bounded, typed lookup behind a foreign-key cell action.
    /// Metadata is checked here as well as in the grid so every caller gets
    /// the same qualified identity and no UI-provided identifier can reach a
    /// driver without validation.
    pub async fn lookup_foreign_key(
        &self,
        request: ForeignKeyLookupRequest,
    ) -> CellarResult<QueryResult> {
        if request.database.trim().is_empty()
            || request.schema.trim().is_empty()
            || request.table.trim().is_empty()
        {
            return Err(CellarError::invalid_config(
                "foreign-key lookup has an incomplete table identity",
            ));
        }
        if request.columns.is_empty() || request.columns.len() != request.values.len() {
            return Err(CellarError::invalid_config(
                "foreign-key lookup columns and values do not match",
            ));
        }
        if request.values.iter().any(CellValue::is_null) {
            return Err(CellarError::query(
                "foreign-key lookup cannot use NULL values",
            ));
        }

        let dbs = self.introspect(&request.connection_id, false).await?;
        let table = dbs
            .iter()
            .find(|database| database.name == request.database)
            .and_then(|database| {
                database
                    .schemas
                    .iter()
                    .find(|schema| schema.name == request.schema)
            })
            .and_then(|schema| {
                schema
                    .tables
                    .iter()
                    .find(|table| table.name == request.table)
            })
            .ok_or_else(|| {
                CellarError::invalid_config(format!(
                    "referenced table {}.{}.{} is not available in schema metadata",
                    request.database, request.schema, request.table
                ))
            })?;
        for column in &request.columns {
            if !table.columns.iter().any(|known| known.name == *column) {
                return Err(CellarError::invalid_config(format!(
                    "referenced column {}.{} is not available in schema metadata",
                    request.table, column
                )));
            }
        }

        let engine = self
            .engine_for(&request.connection_id)
            .await
            .ok_or_else(|| {
                CellarError::NotConnected(format!(
                    "no engine is known for connection {}",
                    request.connection_id
                ))
            })?
            .family();
        let dialect = match engine {
            Engine::Postgres => cellar_sql::Dialect::Postgres,
            Engine::MySql => cellar_sql::Dialect::MySql,
            Engine::Sqlite => cellar_sql::Dialect::Sqlite,
            Engine::Mssql => cellar_sql::Dialect::Mssql,
            _ => {
                return Err(CellarError::query(format!(
                    "foreign-key row navigation is unavailable for {}",
                    engine.as_str()
                )));
            }
        };
        let qualified_table = format!(
            "{}.{}",
            dialect.quote_ident(&request.schema),
            dialect.quote_ident(&request.table)
        );
        let predicates = request
            .columns
            .iter()
            .enumerate()
            .map(|(index, column)| format!("{} = :fk_{index}", dialect.quote_ident(column)))
            .collect::<Vec<_>>()
            .join(" AND ");
        let sql = if engine == Engine::Mssql {
            format!("SELECT TOP (2) * FROM {qualified_table} WHERE {predicates}")
        } else {
            format!("SELECT * FROM {qualified_table} WHERE {predicates} LIMIT 2")
        };
        let params = request
            .values
            .into_iter()
            .enumerate()
            .map(|(index, value)| QueryParam {
                name: format!("fk_{index}"),
                value,
            })
            .collect();
        self.run_query(
            &request.connection_id,
            Query::new(sql)
                .with_database(request.database)
                .with_max_rows(2)
                .with_params(params),
        )
        .await
    }
}
