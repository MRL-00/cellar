pub mod backend;
pub mod google;
pub mod openai;

pub fn store_key(provider: &str, key: &str) -> cellar_core::error::CellarResult<()> {
    cellar_secrets::store(&format!("ai:{provider}"), key)?;
    Ok(())
}

pub fn delete_key(provider: &str) -> cellar_core::error::CellarResult<()> {
    cellar_secrets::delete(&format!("ai:{provider}"))?;
    Ok(())
}

pub fn has_key(provider: &str) -> cellar_core::error::CellarResult<bool> {
    Ok(cellar_secrets::load(&format!("ai:{provider}"))?.is_some())
}
