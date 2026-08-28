use std::collections::HashMap;

use cellar_core::{
    driver::ConnectionConfig,
    error::{CellarError, CellarResult},
};

use super::{support::persist, ConnectionRegistry};

pub(super) fn merge_new_configs(
    existing: &HashMap<String, ConnectionConfig>,
    configs: &[ConnectionConfig],
) -> CellarResult<HashMap<String, ConnectionConfig>> {
    let mut merged = existing.clone();
    for config in configs {
        if config.id.is_empty() {
            return Err(CellarError::invalid_config("connection id is empty"));
        }
        if merged.insert(config.id.clone(), config.clone()).is_some() {
            return Err(CellarError::invalid_config(format!(
                "connection '{}' already exists",
                config.id
            )));
        }
    }
    Ok(merged)
}

impl ConnectionRegistry {
    /// Atomically persist new connection metadata without overwriting an
    /// existing connection. Passwords are intentionally handled separately.
    pub async fn save_new(
        &self,
        configs: Vec<ConnectionConfig>,
    ) -> CellarResult<Vec<ConnectionConfig>> {
        if configs.is_empty() {
            return Ok(configs);
        }
        let mut inner = self.inner.write().await;
        let merged = merge_new_configs(&inner.configs, &configs)?;
        persist(&merged).await?;
        inner.configs = merged;
        Ok(configs)
    }
}
