use cellar_core::driver::{Engine, SslMode};
use gpui::{Context, Entity};
use gpui_component::input::InputState;

use super::CellarApp;

pub(super) fn text(state: &Entity<InputState>, cx: &Context<CellarApp>) -> String {
    state.read(cx).value().trim().to_string()
}

pub(super) fn optional_text(state: &Entity<InputState>, cx: &Context<CellarApp>) -> Option<String> {
    let value = text(state, cx);
    (!value.is_empty()).then_some(value)
}

pub(super) fn slug(value: &str) -> String {
    let slug = value
        .to_lowercase()
        .chars()
        .fold(String::new(), |mut out, c| {
            if c.is_ascii_alphanumeric() {
                out.push(c);
            } else if !out.ends_with('-') {
                out.push('-');
            }
            out
        });
    slug.trim_matches('-').chars().take(64).collect::<String>()
}

pub(super) fn default_port(engine: Engine) -> u16 {
    match engine {
        Engine::Postgres | Engine::Supabase | Engine::Neon => 5432,
        Engine::MySql | Engine::PlanetScale => 3306,
        Engine::Mssql | Engine::Azure => 1433,
        Engine::Sqlite => 0,
        Engine::Firestore | Engine::Convex | Engine::Cosmos => 443,
    }
}

pub(super) fn default_database(engine: Engine) -> &'static str {
    match engine {
        Engine::Postgres | Engine::Supabase => "postgres",
        Engine::MySql => "mysql",
        Engine::Mssql | Engine::Azure => "master",
        Engine::Neon => "neondb",
        _ => "",
    }
}

pub(super) fn engine_color(engine: Engine) -> &'static str {
    match engine {
        Engine::Postgres => "#4f8ff7",
        Engine::MySql => "#f6a44a",
        Engine::Mssql => "#d97a5a",
        Engine::Azure => "#5bb8e0",
        Engine::Sqlite => "#a78bfa",
        Engine::Firestore => "#f4c542",
        Engine::Convex => "#f25c4d",
        Engine::Cosmos => "#6b5ce7",
        Engine::Supabase => "#3ecf8e",
        Engine::Neon => "#00e599",
        Engine::PlanetScale => "#c8ccd4",
    }
}

pub(super) fn default_host_user(engine: Engine) -> (&'static str, &'static str) {
    match engine {
        Engine::Firestore => ("firestore.googleapis.com", "(default)"),
        Engine::Supabase => ("", "postgres"),
        Engine::Convex | Engine::Cosmos | Engine::Neon | Engine::PlanetScale | Engine::Sqlite => {
            ("", "")
        }
        _ => ("localhost", ""),
    }
}

pub(super) fn default_ssl(engine: Engine) -> SslMode {
    match engine {
        Engine::Sqlite => SslMode::Disable,
        Engine::Firestore
        | Engine::Convex
        | Engine::Cosmos
        | Engine::Supabase
        | Engine::Neon
        | Engine::PlanetScale => SslMode::Require,
        _ => SslMode::Prefer,
    }
}

pub(super) fn name_placeholder(engine: Engine) -> String {
    match engine {
        Engine::Firestore => "prod-firestore".into(),
        Engine::Convex => "prod-convex".into(),
        Engine::Cosmos => "prod-cosmos".into(),
        _ => format!("local-{}", engine.as_str()),
    }
}

pub(super) fn host_placeholder(engine: Engine) -> &'static str {
    match engine {
        Engine::Convex => "acoustic-panther-123.convex.cloud",
        Engine::Cosmos => "myaccount.documents.azure.com",
        Engine::Supabase => "db.abcdefghijkl.supabase.co",
        Engine::Neon => "ep-cool-name-123456.us-east-1.aws.neon.tech",
        Engine::PlanetScale => "aws.connect.psdb.cloud",
        _ => "",
    }
}

pub(super) fn database_placeholder(engine: Engine) -> &'static str {
    match engine {
        Engine::Firestore => "my-gcp-project",
        Engine::Cosmos => "mydb (optional)",
        _ => "",
    }
}

pub(super) fn user_placeholder(engine: Engine) -> &'static str {
    if engine == Engine::Firestore {
        "(default)"
    } else {
        ""
    }
}

pub(super) fn password_placeholder(engine: Engine, existing: bool) -> &'static str {
    if existing {
        "•••••••• (unchanged)"
    } else {
        match engine {
            Engine::Firestore => "{ service_account_json }",
            Engine::Convex => "prod:acoustic-panther-123|…",
            Engine::Cosmos => "Account primary key",
            _ => "",
        }
    }
}

pub(super) fn valid_color(value: &str) -> bool {
    value.len() == 7
        && value.starts_with('#')
        && value.as_bytes()[1..].iter().all(u8::is_ascii_hexdigit)
}

#[cfg(test)]
mod tests {
    use super::{
        database_placeholder, default_database, default_port, host_placeholder, name_placeholder,
        password_placeholder, slug, user_placeholder, valid_color,
    };
    use cellar_core::driver::Engine;

    #[test]
    fn connection_defaults_match_classic() {
        assert_eq!(slug(" Local DB / Dev "), "local-db-dev");
        assert!(valid_color("#4f8ff7"));
        assert!(!valid_color("red"));
        assert_eq!(default_database(Engine::Postgres), "postgres");
        assert_eq!(default_port(Engine::Sqlite), 0);
        assert_eq!(default_port(Engine::Cosmos), 443);
        assert_eq!(name_placeholder(Engine::Postgres), "local-postgres");
        assert_eq!(
            host_placeholder(Engine::Neon),
            "ep-cool-name-123456.us-east-1.aws.neon.tech"
        );
        assert_eq!(database_placeholder(Engine::Cosmos), "mydb (optional)");
        assert_eq!(user_placeholder(Engine::Firestore), "(default)");
        assert_eq!(
            password_placeholder(Engine::Convex, false),
            "prod:acoustic-panther-123|…"
        );
        assert_eq!(
            password_placeholder(Engine::Postgres, true),
            "•••••••• (unchanged)"
        );
    }
}
