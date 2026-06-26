//! Named (`:name`) and positional (`$N`) parameter detection and binding
//! preparation, built on `sqlparser-rs`'s tokenizer.
//!
//! Detection uses the tokenizer rather than the full parser so it works on the
//! statement-under-cursor even while the surrounding buffer is incomplete, and
//! so placeholders inside string literals, comments, or dollar-quoted bodies
//! are never mistaken for parameters.
//!
//! [`prepare`] additionally rewrites the statement to Postgres-native
//! positional placeholders (`$1..$N`) and reports the parameters in bind order,
//! so a driver can bind values through sqlx without ever interpolating them
//! into the SQL text.

use std::collections::HashMap;

use cellar_core::driver::Engine;
use cellar_core::query::{DetectedParameter, ParameterStyle};
use sqlparser::dialect::{
    Dialect, GenericDialect, MsSqlDialect, MySqlDialect, PostgreSqlDialect, SQLiteDialect,
};
use sqlparser::keywords::Keyword;
use sqlparser::tokenizer::{Token, Tokenizer};

/// Errors raised while analysing or preparing a parameterized statement.
#[derive(Debug, thiserror::Error)]
pub enum ParamError {
    #[error("could not tokenize SQL: {0}")]
    Tokenize(String),
    #[error("no value supplied for parameter `{0}`")]
    MissingValue(String),
}

/// A statement rewritten for native binding plus its parameters in bind order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedStatement {
    /// SQL with every placeholder rewritten to Postgres positional form
    /// (`$1..$N`) in first-appearance order. Repeated names collapse to one
    /// position.
    pub sql: String,
    /// Distinct parameters in bind order. `parameters[k]` binds to `$（k+1)`.
    pub parameters: Vec<DetectedParameter>,
}

/// Pick the sqlparser dialect that matches a Cellar engine. Detection is
/// largely dialect-independent; the dialect mainly affects identifier and
/// string-literal quoting rules.
fn dialect_for(engine: Engine) -> Box<dyn Dialect> {
    match engine {
        Engine::Postgres => Box::new(PostgreSqlDialect {}),
        Engine::MySql => Box::new(MySqlDialect {}),
        Engine::Sqlite => Box::new(SQLiteDialect {}),
        Engine::Mssql | Engine::Azure => Box::new(MsSqlDialect {}),
        Engine::Firestore => Box::new(GenericDialect {}),
    }
}

/// Tokenize `sql`, rewrite placeholders to `$1..$N` in bind order, and report
/// the distinct parameters. The rewrite is Postgres-native; other engines bind
/// with different placeholder syntax, but Postgres is the only driver wired to
/// this path today.
pub fn prepare(sql: &str, engine: Engine) -> Result<PreparedStatement, ParamError> {
    let dialect = dialect_for(engine);
    let raw_tokens = Tokenizer::new(dialect.as_ref(), sql)
        .tokenize()
        .map_err(|e| ParamError::Tokenize(e.to_string()))?;
    // sqlparser tokenizes `:name` as a `Colon` followed by a word rather than a
    // single placeholder, so fold that pair back into one synthetic
    // placeholder. `$N` already arrives as `Token::Placeholder`.
    let tokens = normalize_colon_placeholders(raw_tokens);

    // Significant (non-whitespace) tokens, for column-hint neighbour lookups.
    let sig_tokens: Vec<&Token> = tokens
        .iter()
        .filter(|t| !matches!(t, Token::Whitespace(_)))
        .collect();
    let mut sig_index_of: Vec<Option<usize>> = Vec::with_capacity(tokens.len());
    {
        let mut s = 0usize;
        for t in &tokens {
            if matches!(t, Token::Whitespace(_)) {
                sig_index_of.push(None);
            } else {
                sig_index_of.push(Some(s));
                s += 1;
            }
        }
    }

    let mut parameters: Vec<DetectedParameter> = Vec::new();
    let mut ordinal_of: HashMap<String, u32> = HashMap::new();
    let mut out = String::with_capacity(sql.len());

    for (i, token) in tokens.iter().enumerate() {
        let Token::Placeholder(raw) = token else {
            out.push_str(&token.to_string());
            continue;
        };
        let Some((name, style)) = classify(raw) else {
            // Not a shape we bind (e.g. a bare `?`); leave it untouched.
            out.push_str(&token.to_string());
            continue;
        };

        let ordinal = match ordinal_of.get(&name) {
            Some(o) => *o,
            None => {
                let ordinal = parameters.len() as u32 + 1;
                ordinal_of.insert(name.clone(), ordinal);
                let column_hint = sig_index_of[i].and_then(|pos| column_hint(&sig_tokens, pos));
                parameters.push(DetectedParameter {
                    name: name.clone(),
                    placeholder: raw.clone(),
                    style,
                    ordinal,
                    column_hint,
                });
                ordinal
            }
        };
        out.push('$');
        out.push_str(&ordinal.to_string());
    }

    Ok(PreparedStatement {
        sql: out,
        parameters,
    })
}

/// Resolve the supplied `params` map against a prepared statement, producing
/// the values in bind order. Errors if a detected parameter has no value.
pub fn order_values<'a, T>(
    parameters: &[DetectedParameter],
    by_name: &'a HashMap<&str, T>,
) -> Result<Vec<&'a T>, ParamError> {
    parameters
        .iter()
        .map(|p| {
            by_name
                .get(p.name.as_str())
                .ok_or_else(|| ParamError::MissingValue(p.name.clone()))
        })
        .collect()
}

/// Fold each `Colon` + unquoted-`Word` pair into a single synthetic
/// `:name` placeholder token. A bare `:` not followed by a word is left as-is.
fn normalize_colon_placeholders(tokens: Vec<Token>) -> Vec<Token> {
    let mut out: Vec<Token> = Vec::with_capacity(tokens.len());
    let mut i = 0;
    while i < tokens.len() {
        if matches!(tokens[i], Token::Colon) {
            if let Some(Token::Word(w)) = tokens.get(i + 1) {
                if w.quote_style.is_none() {
                    out.push(Token::Placeholder(format!(":{}", w.value)));
                    i += 2;
                    continue;
                }
            }
        }
        out.push(tokens[i].clone());
        i += 1;
    }
    out
}

/// Split a placeholder token into its bare name and style. Returns `None` for
/// shapes we do not bind by name (e.g. a bare `?`).
fn classify(raw: &str) -> Option<(String, ParameterStyle)> {
    if let Some(rest) = raw.strip_prefix(':') {
        if rest.is_empty() {
            return None;
        }
        return Some((rest.to_string(), ParameterStyle::Named));
    }
    if let Some(rest) = raw.strip_prefix('$') {
        if rest.is_empty() {
            return None;
        }
        if rest.chars().all(|c| c.is_ascii_digit()) {
            return Some((rest.to_string(), ParameterStyle::Positional));
        }
        // `$name` is unusual but treat it as named rather than dropping it.
        return Some((rest.to_string(), ParameterStyle::Named));
    }
    if let Some(rest) = raw.strip_prefix('@') {
        if rest.is_empty() {
            return None;
        }
        return Some((rest.to_string(), ParameterStyle::Named));
    }
    None
}

/// Best-effort: if the placeholder at significant position `pos` sits on one
/// side of a comparison whose other side is a column reference, return that
/// column's name so the UI can infer an input type from schema.
fn column_hint(sig: &[&Token], pos: usize) -> Option<String> {
    // `<column> <op> <placeholder>`
    if pos >= 2 && is_comparison(sig[pos - 1]) {
        if let Some(col) = ident_value(sig[pos - 2]) {
            return Some(col);
        }
    }
    // `<placeholder> <op> <column>` — walk a qualified `table.column` and keep
    // the trailing component.
    if pos + 2 < sig.len() && is_comparison(sig[pos + 1]) {
        if let Some(col) = qualified_column_tail(sig, pos + 2) {
            return Some(col);
        }
    }
    None
}

/// Starting at significant index `start` (expected to be a word), follow any
/// `.word` chain and return the final component (`id` for `u.id`).
fn qualified_column_tail(sig: &[&Token], start: usize) -> Option<String> {
    let mut tail = ident_value(sig[start])?;
    let mut i = start;
    while i + 2 < sig.len() && matches!(sig[i + 1], Token::Period) {
        if let Some(next) = ident_value(sig[i + 2]) {
            tail = next;
            i += 2;
        } else {
            break;
        }
    }
    Some(tail)
}

fn is_comparison(token: &Token) -> bool {
    matches!(
        token,
        Token::Eq | Token::Neq | Token::Lt | Token::Gt | Token::LtEq | Token::GtEq
    ) || matches!(token, Token::Word(w) if w.keyword == Keyword::LIKE || w.keyword == Keyword::ILIKE)
}

/// The identifier text of a word token, or `None` if it is a logical/operator
/// keyword. sqlparser classifies many ordinary column names as keywords (e.g.
/// `id` → `Keyword::ID`), so we accept any word that is not on a small denylist
/// of words that cannot be a column in this position.
fn ident_value(token: &Token) -> Option<String> {
    match token {
        Token::Word(w) if !is_non_column_keyword(w.keyword) => Some(w.value.clone()),
        _ => None,
    }
}

fn is_non_column_keyword(keyword: Keyword) -> bool {
    matches!(
        keyword,
        Keyword::AND
            | Keyword::OR
            | Keyword::NOT
            | Keyword::NULL
            | Keyword::TRUE
            | Keyword::FALSE
            | Keyword::IS
            | Keyword::IN
            | Keyword::LIKE
            | Keyword::ILIKE
            | Keyword::BETWEEN
            | Keyword::EXISTS
            | Keyword::ANY
            | Keyword::ALL
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(params: &[DetectedParameter]) -> Vec<&str> {
        params.iter().map(|p| p.name.as_str()).collect()
    }

    #[test]
    fn detects_named_parameters_in_order() {
        let prepared = prepare(
            "SELECT * FROM users WHERE id = :user_id AND created_at > :since",
            Engine::Postgres,
        )
        .unwrap();
        assert_eq!(names(&prepared.parameters), vec!["user_id", "since"]);
        assert_eq!(prepared.parameters[0].style, ParameterStyle::Named);
        assert_eq!(prepared.parameters[0].ordinal, 1);
        assert_eq!(prepared.parameters[1].ordinal, 2);
        assert_eq!(
            prepared.sql,
            "SELECT * FROM users WHERE id = $1 AND created_at > $2"
        );
    }

    #[test]
    fn detects_positional_parameters() {
        let prepared =
            prepare("SELECT * FROM t WHERE a = $1 AND b = $2", Engine::Postgres).unwrap();
        assert_eq!(names(&prepared.parameters), vec!["1", "2"]);
        assert!(prepared
            .parameters
            .iter()
            .all(|p| p.style == ParameterStyle::Positional));
        assert_eq!(prepared.sql, "SELECT * FROM t WHERE a = $1 AND b = $2");
    }

    #[test]
    fn dedupes_repeated_named_parameter() {
        let prepared = prepare(
            "SELECT * FROM t WHERE a = :x OR b = :x OR c = :y",
            Engine::Postgres,
        )
        .unwrap();
        assert_eq!(names(&prepared.parameters), vec!["x", "y"]);
        assert_eq!(
            prepared.sql,
            "SELECT * FROM t WHERE a = $1 OR b = $1 OR c = $2"
        );
    }

    #[test]
    fn ignores_placeholders_inside_strings_and_comments() {
        let prepared = prepare(
            "SELECT ':not_a_param' AS s -- :also_not\nFROM t WHERE id = :real",
            Engine::Postgres,
        )
        .unwrap();
        assert_eq!(names(&prepared.parameters), vec!["real"]);
    }

    #[test]
    fn ignores_dollar_quoted_bodies() {
        let prepared = prepare(
            "SELECT $$ body with $1 inside $$ AS b WHERE id = :id",
            Engine::Postgres,
        )
        .unwrap();
        assert_eq!(names(&prepared.parameters), vec!["id"]);
    }

    #[test]
    fn infers_column_hint_from_comparison() {
        let prepared = prepare("SELECT * FROM users WHERE id = :uid", Engine::Postgres).unwrap();
        assert_eq!(prepared.parameters[0].column_hint.as_deref(), Some("id"));
    }

    #[test]
    fn infers_column_hint_with_qualified_column_and_reversed_order() {
        let prepared =
            prepare("SELECT * FROM users u WHERE :uid = u.id", Engine::Postgres).unwrap();
        assert_eq!(prepared.parameters[0].column_hint.as_deref(), Some("id"));
    }

    #[test]
    fn no_parameters_round_trips_sql() {
        let sql = "SELECT a, b FROM t WHERE c = 'x' AND d = 3 -- note\n ORDER BY a";
        let prepared = prepare(sql, Engine::Postgres).unwrap();
        assert!(prepared.parameters.is_empty());
        assert_eq!(prepared.sql, sql);
    }

    #[test]
    fn order_values_resolves_by_name() {
        let prepared =
            prepare("SELECT * FROM t WHERE a = :x AND b = :y", Engine::Postgres).unwrap();
        let mut by_name: HashMap<&str, i32> = HashMap::new();
        by_name.insert("x", 10);
        by_name.insert("y", 20);
        let ordered = order_values(&prepared.parameters, &by_name).unwrap();
        assert_eq!(ordered, vec![&10, &20]);
    }

    #[test]
    fn order_values_reports_missing() {
        let prepared = prepare("SELECT * FROM t WHERE a = :x", Engine::Postgres).unwrap();
        let by_name: HashMap<&str, i32> = HashMap::new();
        let err = order_values(&prepared.parameters, &by_name).unwrap_err();
        assert!(matches!(err, ParamError::MissingValue(n) if n == "x"));
    }
}
