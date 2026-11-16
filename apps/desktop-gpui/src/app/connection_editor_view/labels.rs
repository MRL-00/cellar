use super::*;

pub(super) fn engine_label(engine: Engine) -> &'static str {
    match engine {
        Engine::Postgres => "PostgreSQL",
        Engine::MySql => "MySQL",
        Engine::Mssql => "SQL Server",
        Engine::Azure => "Azure SQL",
        Engine::Sqlite => "SQLite",
        Engine::Firestore => "Firestore",
        Engine::Convex => "Convex",
        Engine::Cosmos => "Cosmos DB",
        Engine::Supabase => "Supabase",
        Engine::Neon => "Neon",
        Engine::PlanetScale => "PlanetScale",
    }
}

pub(super) fn engine_rgb(engine: Engine) -> u32 {
    u32::from_str_radix(&engine_color(engine)[1..], 16).unwrap_or(0xa78bfa)
}

pub(super) fn host_label(engine: Engine) -> &'static str {
    match engine {
        Engine::Firestore => "API host",
        Engine::Convex => "Deployment host",
        Engine::Cosmos => "Account endpoint",
        _ => "Host",
    }
}

pub(super) fn database_label(engine: Engine) -> &'static str {
    match engine {
        Engine::Sqlite => "Database file",
        Engine::Firestore => "Project ID",
        Engine::Cosmos => "Database",
        _ => "Database",
    }
}

pub(super) fn password_label(engine: Engine) -> &'static str {
    match engine {
        Engine::Firestore => "Credentials",
        Engine::Convex => "Deploy key",
        Engine::Cosmos => "Primary key",
        _ => "Password",
    }
}

pub(super) fn password_hint(editor: &ConnectionEditor) -> &'static str {
    match (editor.original.is_some(), editor.engine) {
        (true, Engine::Firestore) => "Leave blank to keep saved JSON/token",
        (false, Engine::Firestore) => {
            "Leave blank for emulator; JSON/token is stored in OS keychain"
        }
        (true, Engine::Convex) => "Leave blank to keep the saved deploy key",
        (false, Engine::Convex) => "Leave blank for a local backend; stored in OS keychain",
        (true, Engine::Cosmos) => "Leave blank to keep the saved account key",
        (false, Engine::Cosmos) => {
            "Account primary key from the Azure portal; stored in OS keychain"
        }
        (true, _) => "Leave blank to keep the saved password",
        (false, _) => "Stored in OS keychain",
    }
}

pub(super) fn ssl_mode_label(mode: SslMode) -> &'static str {
    match mode {
        SslMode::Disable => "disable",
        SslMode::Prefer => "prefer",
        SslMode::Require => "require",
        SslMode::VerifyCa => "verify-ca",
        SslMode::VerifyFull => "verify-full",
    }
}

pub(super) fn env_label(env: EnvTag) -> &'static str {
    match env {
        EnvTag::Prod => "prod",
        EnvTag::Staging => "staging",
        EnvTag::Dev => "dev",
        EnvTag::Local => "local",
    }
}
