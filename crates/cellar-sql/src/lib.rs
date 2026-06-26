//! SQL formatting and dialect awareness for Cellar.
//!
//! This crate is the single place that knows how each engine quotes
//! identifiers and string literals. DDL/SQL builders (the grid commit path in
//! `cellar-diff`, the schema migration path in `cellar-schema-diff`) format
//! through here so escaping rules live in one audited spot rather than being
//! re-implemented per builder.
//!
//! A full `sqlparser-rs`-backed parser still lands here later (autocomplete
//! context, format-on-save). The formatting primitives below are the first
//! real slice.

use serde::{Deserialize, Serialize};
use specta::Type;

/// SQL dialect a statement is being generated for. Identifier quoting and a
/// handful of DDL spellings differ per engine; everything that varies routes
/// through [`Dialect`] so callers stay engine-agnostic.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Type, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum Dialect {
    Postgres,
    MySql,
    Sqlite,
    Mssql,
}

impl Dialect {
    /// Quote a single identifier (table, column, schema, constraint name),
    /// escaping any embedded quote characters per the dialect's doubling rule.
    ///
    /// - Postgres / SQLite use double quotes: `"name"`.
    /// - MySQL uses backticks: `` `name` `` (an embedded backtick doubles).
    /// - SQL Server uses brackets: `[name]` (an embedded `]` doubles).
    pub fn quote_ident(self, ident: &str) -> String {
        match self {
            Dialect::Postgres | Dialect::Sqlite => {
                format!("\"{}\"", ident.replace('"', "\"\""))
            }
            Dialect::MySql => format!("`{}`", ident.replace('`', "``")),
            Dialect::Mssql => format!("[{}]", ident.replace(']', "]]")),
        }
    }

    /// Quote a `schema.object` pair into a fully qualified, dialect-safe name.
    pub fn quote_qualified(self, schema: &str, object: &str) -> String {
        format!("{}.{}", self.quote_ident(schema), self.quote_ident(object))
    }
}

/// Quote a SQL string literal, escaping embedded single quotes by doubling.
/// This is dialect-independent for the engines Cellar targets (all use
/// `'...'` with `''` escaping). NULLs are the caller's concern — this always
/// returns a non-NULL quoted literal.
pub fn quote_string_literal(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quotes_identifiers_per_dialect() {
        assert_eq!(Dialect::Postgres.quote_ident("users"), "\"users\"");
        assert_eq!(Dialect::Sqlite.quote_ident("users"), "\"users\"");
        assert_eq!(Dialect::MySql.quote_ident("users"), "`users`");
        assert_eq!(Dialect::Mssql.quote_ident("users"), "[users]");
    }

    #[test]
    fn escapes_embedded_quote_characters() {
        assert_eq!(Dialect::Postgres.quote_ident("a\"b"), "\"a\"\"b\"");
        assert_eq!(Dialect::MySql.quote_ident("a`b"), "`a``b`");
        assert_eq!(Dialect::Mssql.quote_ident("a]b"), "[a]]b]");
    }

    #[test]
    fn qualifies_schema_and_object() {
        assert_eq!(
            Dialect::Postgres.quote_qualified("public", "orders"),
            "\"public\".\"orders\""
        );
    }

    #[test]
    fn escapes_string_literals() {
        assert_eq!(quote_string_literal("paid's"), "'paid''s'");
        assert_eq!(quote_string_literal("plain"), "'plain'");
    }
}
