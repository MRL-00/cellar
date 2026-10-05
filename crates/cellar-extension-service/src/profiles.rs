use cellar_core::driver::{ConnectionConfig, Engine, SslMode};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{env, path::PathBuf};

#[derive(Clone, Deserialize, Serialize)]
pub struct Profile {
    #[serde(flatten)]
    pub config: ConnectionConfig,
    #[serde(default)]
    pub allow_edits: bool,
    #[serde(default)]
    pub managed: bool,
    #[serde(default)]
    pub has_password: bool,
    #[serde(default)]
    pub ssl_ca_pem: Option<String>,
}

pub fn directory() -> Result<PathBuf, String> {
    env::var("CELLAR_EXTENSION_STATE")
        .map(PathBuf::from)
        .map_err(|_| "Local profile storage is not configured".into())
}

pub fn list() -> Result<Vec<Profile>, String> {
    let mut profiles = Vec::new();
    if let Ok(path) = env::var("CELLAR_EXTENSION_CONFIG") {
        profiles = read(PathBuf::from(path))?;
    }
    if let Ok(directory) = directory() {
        let path = directory.join("profiles.json");
        if path.exists() {
            profiles.extend(read(path)?);
        }
    }
    Ok(profiles)
}

fn read(path: PathBuf) -> Result<Vec<Profile>, String> {
    let bytes = std::fs::read(path).map_err(|_| "Cannot read connection profiles")?;
    if bytes.len() > 262144 {
        return Err("Connection profiles exceed size limit".into());
    }
    serde_json::from_slice(&bytes).map_err(|_| "Invalid connection profiles".into())
}

pub fn public(profiles: &[Profile]) -> Value {
    json!({"connections": profiles.iter().map(|p| json!({"id":p.config.id,"name":p.config.name,"engine":p.config.engine,"environment":p.config.env_tag,"readOnly":!p.allow_edits,"managed":p.managed})).collect::<Vec<_>>()})
}

pub fn manage(
    action: &str,
    value: Option<Value>,
    id: Option<&str>,
    password: Option<&str>,
) -> Result<Value, String> {
    let directory = directory()?;
    std::fs::create_dir_all(&directory).map_err(|_| "Cannot create private profile storage")?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&directory, std::fs::Permissions::from_mode(0o700))
            .map_err(|_| "Cannot protect profile storage")?;
    }
    let lock = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(directory.join("profiles.lock"))
        .map_err(|_| "Cannot lock profile storage")?;
    fs2::FileExt::lock_exclusive(&lock).map_err(|_| "Cannot lock profile storage")?;
    let path = directory.join("profiles.json");
    let mut profiles = if path.exists() {
        read(path.clone())?
    } else {
        Vec::new()
    };
    if action == "details" {
        let profile = profiles
            .iter()
            .find(|p| Some(p.config.id.as_str()) == id)
            .ok_or("Select a user-created connection")?;
        return Ok(json!({"profile":profile}));
    }
    if action == "remove" {
        let id = id.ok_or("Select a saved connection")?;
        if !profiles.iter().any(|p| p.config.id == id) {
            return Err("Only user-created profiles can be removed".into());
        }
        if profiles.iter().any(|p| p.config.id == id && p.has_password) {
            cellar_secrets::delete(&format!("openai-extension:{id}"))
                .map_err(|_| "Cannot remove keychain credential")?;
        }
        profiles.retain(|p| p.config.id != id);
    } else {
        let mut profile: Profile =
            serde_json::from_value(value.ok_or("Connection details required")?)
                .map_err(|_| "Invalid connection details")?;
        let previous = profiles
            .iter()
            .find(|p| p.config.id == profile.config.id)
            .cloned();
        if !profile.config.id.is_empty() && previous.is_none() {
            return Err("Only user-created profiles can be updated".into());
        }
        if profiles.len() >= 32 && previous.is_none() {
            return Err("At most 32 saved connections are supported".into());
        }
        if previous.is_none() {
            profile.config.id = uuid::Uuid::new_v4().to_string();
        }
        profile.managed = true;
        profile.has_password = previous.as_ref().is_some_and(|p| p.has_password);
        if profile
            .ssl_ca_pem
            .as_ref()
            .is_some_and(|pem| pem.len() > 16384 || !pem.contains("-----BEGIN CERTIFICATE-----"))
        {
            return Err("Provide a PEM root certificate under 16 KiB".into());
        }
        let c = &mut profile.config;
        if c.name.trim().is_empty()
            || c.name.len() > 80
            || c.database.is_empty()
            || c.database.len() > 4096
        {
            return Err("Connection name and database are required".into());
        }
        if c.engine.family() == Engine::Sqlite {
            let path =
                std::fs::canonicalize(&c.database).map_err(|_| "Select an existing SQLite file")?;
            let metadata = std::fs::metadata(&path).map_err(|_| "Cannot read SQLite file")?;
            if !metadata.is_file() {
                return Err("SQLite path must be a regular file".into());
            }
            use std::io::Read;
            let mut header = [0u8; 16];
            std::fs::File::open(&path)
                .and_then(|mut f| f.read_exact(&mut header))
                .map_err(|_| "Cannot read SQLite header")?;
            if &header != b"SQLite format 3\0" {
                return Err("Selected file is not SQLite".into());
            }
            c.database = path.to_string_lossy().into_owned();
        } else if c.engine.family() == Engine::Postgres {
            if c.host.is_empty()
                || c.host.len() > 255
                || c.port == 0
                || c.user.is_empty()
                || c.host.contains(['\n', '\r', '@'])
            {
                return Err("Host, port and database user are required".into());
            }
            if !["127.0.0.1", "localhost", "::1"].contains(&c.host.as_str())
                && !c.host.starts_with('/')
            {
                c.ssl_mode = SslMode::VerifyFull;
            }
            if let Some(password) = password.filter(|p| !p.is_empty()) {
                cellar_secrets::store(&format!("openai-extension:{}", c.id), password)
                    .map_err(|_| "Cannot save password in OS keychain; connection was not saved")?;
                profile.has_password = true;
            }
        } else {
            return Err("Choose SQLite, PostgreSQL, Supabase or Neon".into());
        }
        profiles.retain(|p| p.config.id != profile.config.id);
        profiles.push(profile);
    }
    let temporary = directory.join(format!("profiles-{}.tmp", uuid::Uuid::new_v4()));
    let bytes = serde_json::to_vec_pretty(&profiles).map_err(|_| "Cannot encode profiles")?;
    std::fs::write(&temporary, bytes).map_err(|_| "Cannot save profiles")?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&temporary, std::fs::Permissions::from_mode(0o600))
            .map_err(|_| "Cannot protect profiles")?;
    }
    std::fs::rename(temporary, path).map_err(|_| "Cannot finish saving profiles")?;
    public(&list()?)
        .as_object()
        .cloned()
        .map(Value::Object)
        .ok_or("Cannot read saved profiles".into())
}
