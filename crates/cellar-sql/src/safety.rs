use cellar_core::driver::Engine;
use sqlparser::{
    dialect::{
        Dialect, GenericDialect, MsSqlDialect, MySqlDialect, PostgreSqlDialect, SQLiteDialect,
    },
    tokenizer::{Token, Tokenizer},
};

/// Return true when persisting this SQL could put a credential in plaintext.
/// The caller should omit the statement instead of trying to rewrite it.
pub fn sql_contains_credentials(sql: &str) -> bool {
    let lower = sql.to_ascii_lowercase();
    if lower.contains("://")
        || [
            "password=",
            "pwd=",
            "token=",
            "secret=",
            "api_key=",
            "apikey=",
        ]
        .iter()
        .any(|marker| lower.contains(marker))
    {
        return true;
    }
    let generic = GenericDialect;
    let Ok(tokens) = Tokenizer::new(&generic, sql).tokenize() else {
        return ["password", "identified by", "credential", "secret"]
            .iter()
            .any(|marker| lower.contains(marker));
    };
    let significant = tokens
        .iter()
        .filter(|token| !matches!(token, Token::Whitespace(_)))
        .collect::<Vec<_>>();
    let mut statement_words = Vec::<String>::new();
    for (index, token) in significant.iter().enumerate() {
        if matches!(token, Token::SemiColon) {
            statement_words.clear();
            continue;
        }
        let Token::Word(word) = token else {
            continue;
        };
        if word.quote_style.is_some() {
            continue;
        }
        let value = word.value.to_ascii_lowercase();
        let next_word = significant.get(index + 1).and_then(|token| match token {
            Token::Word(word) if word.quote_style.is_none() => {
                Some(word.value.to_ascii_lowercase())
            }
            _ => None,
        });
        let account_ddl = statement_words
            .iter()
            .any(|word| word == "create" || word == "alter")
            && statement_words
                .iter()
                .any(|word| word == "user" || word == "role");
        if (value == "password" && (account_ddl || next_word.as_deref() == Some("for")))
            || (value == "identified" && next_word.as_deref() == Some("by"))
            || (value == "secret"
                && statement_words
                    .iter()
                    .any(|word| word == "create" || word == "alter"))
        {
            return true;
        }
        statement_words.push(value);
    }
    false
}

/// Explain why SQL needs an explicit destructive-operation confirmation.
/// Quoted text, identifiers, and comments are ignored by sqlparser's tokenizer.
pub fn destructive_reason(sql: &str, engine: Engine) -> Option<&'static str> {
    let generic = GenericDialect;
    let postgres = PostgreSqlDialect {};
    let mysql = MySqlDialect {};
    let sqlite = SQLiteDialect {};
    let mssql = MsSqlDialect {};
    let dialect: &dyn Dialect = match engine.family() {
        Engine::Postgres => &postgres,
        Engine::MySql => &mysql,
        Engine::Sqlite => &sqlite,
        Engine::Mssql => &mssql,
        _ => &generic,
    };
    let Ok(tokens) = Tokenizer::new(dialect, sql).tokenize() else {
        return Some("SQL that could not be safety-checked");
    };

    let mut depth = 0usize;
    let mut mutations = Vec::<(usize, &'static str, bool)>::new();
    for token in tokens.into_iter().chain([Token::SemiColon]) {
        match token {
            Token::LParen => depth += 1,
            Token::RParen => {
                if let Some((_, reason, false)) = mutations.iter().find(|(d, _, _)| *d == depth) {
                    return Some(reason);
                }
                mutations.retain(|(d, _, _)| *d != depth);
                depth = depth.saturating_sub(1);
            }
            Token::SemiColon if depth == 0 => {
                if let Some((_, reason, false)) = mutations.iter().find(|(_, _, safe)| !*safe) {
                    return Some(reason);
                }
                mutations.clear();
            }
            Token::Word(word) if word.quote_style.is_none() => {
                if word.value.eq_ignore_ascii_case("drop") {
                    return Some("DROP statement");
                }
                if word.value.eq_ignore_ascii_case("truncate") {
                    return Some("TRUNCATE statement");
                }
                if word.value.eq_ignore_ascii_case("delete") {
                    mutations.push((depth, "DELETE without WHERE", false));
                } else if word.value.eq_ignore_ascii_case("update") {
                    mutations.push((depth, "UPDATE without WHERE", false));
                } else if word.value.eq_ignore_ascii_case("where") {
                    if let Some((_, _, safe)) =
                        mutations.iter_mut().rev().find(|(d, _, _)| *d == depth)
                    {
                        *safe = true;
                    }
                }
            }
            _ => {}
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use cellar_core::driver::Engine;

    use super::{destructive_reason, sql_contains_credentials};

    #[test]
    fn classifies_destructive_sql_without_being_fooled_by_nested_or_quoted_where() {
        assert_eq!(
            destructive_reason("UPDATE users SET active = false", Engine::Postgres),
            Some("UPDATE without WHERE")
        );
        assert_eq!(
            destructive_reason(
                "UPDATE users SET note = (SELECT note FROM defaults WHERE id = 1)",
                Engine::Postgres
            ),
            Some("UPDATE without WHERE")
        );
        assert_eq!(
            destructive_reason("DELETE FROM users WHERE id = 1", Engine::Postgres),
            None
        );
        assert_eq!(
            destructive_reason(
                "SELECT 'DROP TABLE users' -- TRUNCATE users",
                Engine::Postgres
            ),
            None
        );
        assert_eq!(
            destructive_reason("ALTER TABLE users DROP COLUMN legacy", Engine::Postgres),
            Some("DROP statement")
        );
    }

    #[test]
    fn detects_credentials_without_rewriting_ordinary_password_columns() {
        assert!(sql_contains_credentials(
            "CREATE ROLE app PASSWORD $$secret$$"
        ));
        assert!(sql_contains_credentials(
            "CREATE USER app IDENTIFIED BY 'secret'"
        ));
        assert!(sql_contains_credentials("SET PASSWORD FOR app = 'secret'"));
        assert!(!sql_contains_credentials(
            "SELECT password FROM users WHERE name = 'alice'"
        ));
        assert!(!sql_contains_credentials(
            "SELECT password, email FROM users"
        ));
        assert!(!sql_contains_credentials(
            "SELECT password::text FROM users"
        ));
    }
}
