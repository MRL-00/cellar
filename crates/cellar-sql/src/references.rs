//! Structural reference detection.
//!
//! We tokenize SQL with `sqlparser`'s Postgres tokenizer and look for the
//! target name as a whole *identifier* token. That gets us four things naive
//! `LIKE '%name%'` matching can't:
//!
//! - **No substring hits.** `user_identities` is one word token, so a search
//!   for `user` never matches it.
//! - **No matches inside string literals or comments.** The tokenizer isolates
//!   `'users'` as a single-quoted string and strips `-- comments`, so neither
//!   can masquerade as a reference.
//! - **Quoting-aware case folding.** Unquoted Postgres identifiers fold to
//!   lowercase; quoted ones are case-sensitive. We match the same way.
//! - **Schema-aware qualification.** A reference written `other.users` is only
//!   a usage of `target_schema.users` when `other` equals the target schema.
//!   Unqualified references (`users`) still match, since their schema resolves
//!   through `search_path` and we conservatively include them.

use sqlparser::dialect::PostgreSqlDialect;
use sqlparser::tokenizer::{Token, TokenWithLocation, Tokenizer, Word};

/// One confirmed occurrence of the searched name inside a definition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reference {
    /// 1-based line within the source where the reference was found.
    pub line: u32,
    /// 1-based column where the matching token starts.
    pub column: u32,
    /// The matching source line, trimmed (and truncated if very long).
    pub snippet: String,
    /// The column name matched, when the search was column-scoped.
    pub matched_column: Option<String>,
}

const MAX_SNIPPET_CHARS: usize = 200;

/// Find structurally-confirmed references to `schema`.`object` (a table/view) in
/// `sql`. A schema-qualified reference (`other.object`) only counts when its
/// qualifier equals `schema`; an unqualified reference (`object`) always counts,
/// since its schema is resolved via `search_path` at runtime and we can't know
/// it statically.
///
/// When `column` is given, the result is narrowed to column references: a hit is
/// reported only when the table identifier *and* the column identifier both
/// appear in the definition (so a same-named column on an unrelated table that
/// the definition never mentions is not a false positive), and the returned
/// references point at the column occurrences.
///
/// Returns an empty vector when the SQL cannot be tokenized — a malformed
/// definition simply yields no confirmed references rather than an error.
pub fn find_references(
    sql: &str,
    schema: &str,
    object: &str,
    column: Option<&str>,
) -> Vec<Reference> {
    let dialect = PostgreSqlDialect {};
    let tokens = match Tokenizer::new(&dialect, sql).tokenize_with_location() {
        Ok(tokens) => tokens,
        Err(_) => return Vec::new(),
    };

    let mut table_hits: Vec<(u32, u32)> = Vec::new();
    let mut column_hits: Vec<(u32, u32)> = Vec::new();
    for (i, tok) in tokens.iter().enumerate() {
        if let Token::Word(word) = &tok.token {
            let at = (tok.location.line as u32, tok.location.column as u32);
            if ident_eq(word, object) && qualifier_matches(&tokens, i, schema) {
                table_hits.push(at);
            }
            if let Some(col) = column {
                if ident_eq(word, col) {
                    column_hits.push(at);
                }
            }
        }
    }

    let lines: Vec<&str> = sql.lines().collect();
    match column {
        None => table_hits
            .into_iter()
            .map(|(line, col)| to_ref(&lines, line, col, None))
            .collect(),
        Some(col) => {
            // A real column reference needs the owning table mentioned too;
            // otherwise a column of the same name on a different relation would
            // match definitions that never touch our table.
            if table_hits.is_empty() || column_hits.is_empty() {
                return Vec::new();
            }
            column_hits
                .into_iter()
                .map(|(line, c)| to_ref(&lines, line, c, Some(col.to_string())))
                .collect()
        }
    }
}

/// Decide whether the identifier at `index` belongs to `schema`. Walks back over
/// whitespace looking for a `.`; if one is found, the identifier before it is the
/// schema qualifier and must equal `schema`. No qualifier means an unqualified
/// reference, which we accept (its schema resolves via `search_path`).
fn qualifier_matches(tokens: &[TokenWithLocation], index: usize, schema: &str) -> bool {
    let Some(dot) = prev_significant(tokens, index) else {
        return true;
    };
    if !matches!(tokens[dot].token, Token::Period) {
        return true;
    }
    match prev_significant(tokens, dot) {
        Some(q) => match &tokens[q].token {
            Token::Word(word) => ident_eq(word, schema),
            // A `.` not preceded by an identifier (e.g. a quoted-string edge
            // case) isn't a schema qualifier we can reason about — don't reject.
            _ => true,
        },
        None => true,
    }
}

/// Index of the nearest non-whitespace token before `index`, if any.
fn prev_significant(tokens: &[TokenWithLocation], index: usize) -> Option<usize> {
    (0..index)
        .rev()
        .find(|&j| !matches!(tokens[j].token, Token::Whitespace(_)))
}

/// Compare a tokenized identifier to a target name using Postgres' own folding
/// rules: unquoted identifiers are case-insensitive, quoted ones are exact.
fn ident_eq(word: &Word, target: &str) -> bool {
    match word.quote_style {
        Some(_) => word.value == target,
        None => word.value.eq_ignore_ascii_case(target),
    }
}

fn to_ref(lines: &[&str], line: u32, column: u32, matched_column: Option<String>) -> Reference {
    let snippet = lines
        .get(line.saturating_sub(1) as usize)
        .map(|raw| {
            let trimmed = raw.trim();
            if trimmed.chars().count() > MAX_SNIPPET_CHARS {
                let head: String = trimmed.chars().take(MAX_SNIPPET_CHARS - 3).collect();
                format!("{head}...")
            } else {
                trimmed.to_string()
            }
        })
        .unwrap_or_default();
    Reference {
        line,
        column,
        snippet,
        matched_column,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_a_real_table_reference() {
        let refs = find_references("SELECT * FROM users WHERE active", "public", "users", None);
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].line, 1);
        assert!(refs[0].snippet.contains("users"));
    }

    #[test]
    fn does_not_match_a_substring() {
        // `user_identities` must not match a search for `user`.
        let refs = find_references("SELECT id FROM user_identities", "public", "user", None);
        assert!(refs.is_empty());
    }

    #[test]
    fn does_not_match_inside_a_string_literal() {
        let refs = find_references(
            "SELECT 'users' AS label FROM accounts",
            "public",
            "users",
            None,
        );
        assert!(refs.is_empty());
    }

    #[test]
    fn does_not_match_inside_a_comment() {
        let refs = find_references(
            "-- references users here\nSELECT 1 FROM accounts",
            "public",
            "users",
            None,
        );
        assert!(refs.is_empty());
    }

    #[test]
    fn matches_unquoted_case_insensitively() {
        let refs = find_references("SELECT * FROM USERS", "public", "users", None);
        assert_eq!(refs.len(), 1);
    }

    #[test]
    fn quoted_identifier_is_case_sensitive() {
        let hit = find_references("SELECT * FROM \"Users\"", "public", "Users", None);
        assert_eq!(hit.len(), 1);
        let miss = find_references("SELECT * FROM \"Users\"", "public", "users", None);
        assert!(miss.is_empty());
    }

    #[test]
    fn reports_line_for_multiline_definition() {
        let sql = "SELECT u.id\nFROM accounts a\nJOIN users u ON u.id = a.user_id";
        let refs = find_references(sql, "public", "users", None);
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].line, 3);
    }

    #[test]
    fn column_scope_requires_both_table_and_column() {
        let sql = "SELECT email FROM users";
        let refs = find_references(sql, "public", "users", Some("email"));
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].matched_column.as_deref(), Some("email"));
    }

    #[test]
    fn column_scope_misses_when_table_absent() {
        // `email` exists, but the definition never mentions `users`.
        let sql = "SELECT email FROM contacts";
        let refs = find_references(sql, "public", "users", Some("email"));
        assert!(refs.is_empty());
    }

    #[test]
    fn matches_constraint_definition() {
        let def = "FOREIGN KEY (user_id) REFERENCES users(id)";
        assert_eq!(find_references(def, "public", "users", None).len(), 1);
        assert_eq!(find_references(def, "public", "users", Some("id")).len(), 1);
    }

    #[test]
    fn matches_schema_qualified_reference_in_same_schema() {
        let refs = find_references("SELECT * FROM public.users", "public", "users", None);
        assert_eq!(refs.len(), 1);
    }

    #[test]
    fn rejects_schema_qualified_reference_in_another_schema() {
        // `analytics.users` is a different table than `public.users`.
        let refs = find_references("SELECT * FROM analytics.users", "public", "users", None);
        assert!(refs.is_empty());
    }

    #[test]
    fn still_matches_unqualified_reference_for_any_schema() {
        // Bare `users` resolves via search_path, so we conservatively match it.
        let refs = find_references("SELECT * FROM users", "analytics", "users", None);
        assert_eq!(refs.len(), 1);
    }

    #[test]
    fn qualified_column_access_does_not_confuse_table_match() {
        // `users.id`: `users` is unqualified (the table), `id` is qualified by it.
        let refs = find_references("SELECT users.id FROM users", "public", "users", None);
        assert_eq!(refs.len(), 2);
    }
}
