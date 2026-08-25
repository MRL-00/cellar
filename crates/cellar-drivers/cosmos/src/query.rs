//! Translate Cellar grid filters into Cosmos SQL and apply page-local sorts.

use cellar_core::error::{CellarError, CellarResult};
use cellar_core::query::{SortDirection, TableFilterClause, TableFilterOperator, TableSortClause};
use cellar_core::schema::Table;
use serde_json::{json, Map, Value};

#[derive(Debug, Clone)]
pub(crate) struct CosmosQuery {
    pub sql: String,
    pub parameters: Vec<Value>,
}

/// Build a parameterized Cosmos SQL query from grid filter clauses.
///
/// Nested JSON columns use `ToString(c["field"])` for text operators so
/// `contains` finds values inside the document blob.
///
/// Sorts are intentionally omitted: the Cosmos REST gateway cannot serve
/// cross-partition `ORDER BY` (it returns a 400 that only full SDKs handle by
/// fanning out per partition). Callers should sort the returned page locally.
pub(crate) fn build_browse_query(
    table: &Table,
    filters: &[TableFilterClause],
    _sorts: &[TableSortClause],
) -> CellarResult<CosmosQuery> {
    let mut sql = String::from("SELECT * FROM c");
    let mut parameters = Vec::new();

    if !filters.is_empty() {
        let mut parts = Vec::with_capacity(filters.len());
        for filter in filters {
            parts.push(render_filter(table, filter, &mut parameters)?);
        }
        sql.push_str(" WHERE ");
        sql.push_str(&parts.join(" AND "));
    }

    Ok(CosmosQuery { sql, parameters })
}

/// Same filters as browse, but `SELECT VALUE COUNT(1)` for filtered totals.
pub(crate) fn build_count_query(
    table: &Table,
    filters: &[TableFilterClause],
) -> CellarResult<CosmosQuery> {
    let browse = build_browse_query(table, filters, &[])?;
    let sql = browse
        .sql
        .replacen("SELECT * FROM c", "SELECT VALUE COUNT(1) FROM c", 1);
    Ok(CosmosQuery {
        sql,
        parameters: browse.parameters,
    })
}

fn render_filter(
    table: &Table,
    filter: &TableFilterClause,
    parameters: &mut Vec<Value>,
) -> CellarResult<String> {
    let path = column_path(table, &filter.column)?;
    let json_col = is_json_column(table, &filter.column);
    let text_expr = if json_col {
        format!("ToString({path})")
    } else {
        path.clone()
    };

    match filter.operator {
        TableFilterOperator::IsNull => Ok(format!("(NOT IS_DEFINED({path}) OR IS_NULL({path}))")),
        TableFilterOperator::IsNotNull => {
            Ok(format!("(IS_DEFINED({path}) AND NOT IS_NULL({path}))"))
        }
        TableFilterOperator::Equals | TableFilterOperator::NotEquals => {
            let value = required_value(filter)?;
            let param = push_param(parameters, bind_value(table, &filter.column, value));
            let op = if filter.operator == TableFilterOperator::Equals {
                "="
            } else {
                "!="
            };
            if json_col {
                Ok(format!("{text_expr} {op} {param}"))
            } else {
                Ok(format!("{path} {op} {param}"))
            }
        }
        TableFilterOperator::Contains => {
            let value = required_value(filter)?;
            let param = push_param(parameters, Value::String(value.to_ascii_uppercase()));
            Ok(format!("CONTAINS(UPPER({text_expr}), {param})"))
        }
        TableFilterOperator::NotContains => {
            let value = required_value(filter)?;
            let param = push_param(parameters, Value::String(value.to_ascii_uppercase()));
            Ok(format!("NOT CONTAINS(UPPER({text_expr}), {param})"))
        }
        TableFilterOperator::StartsWith => {
            let value = required_value(filter)?;
            let param = push_param(parameters, Value::String(value.to_ascii_uppercase()));
            Ok(format!("STARTSWITH(UPPER({text_expr}), {param})"))
        }
        TableFilterOperator::EndsWith => {
            let value = required_value(filter)?;
            let param = push_param(parameters, Value::String(value.to_ascii_uppercase()));
            Ok(format!("ENDSWITH(UPPER({text_expr}), {param})"))
        }
        TableFilterOperator::Like => {
            let value = required_value(filter)?;
            if let Some(expr) = like_to_cosmos(&text_expr, value, parameters) {
                Ok(expr)
            } else {
                Err(CellarError::invalid_config(
                    "Cosmos LIKE supports only literal, prefix, suffix, and contains patterns",
                ))
            }
        }
        TableFilterOperator::GreaterThan
        | TableFilterOperator::GreaterThanOrEqual
        | TableFilterOperator::LessThan
        | TableFilterOperator::LessThanOrEqual => {
            let value = required_value(filter)?;
            let param = push_param(parameters, bind_value(table, &filter.column, value));
            let op = match filter.operator {
                TableFilterOperator::GreaterThan => ">",
                TableFilterOperator::GreaterThanOrEqual => ">=",
                TableFilterOperator::LessThan => "<",
                TableFilterOperator::LessThanOrEqual => "<=",
                _ => unreachable!(),
            };
            Ok(format!("{path} {op} {param}"))
        }
    }
}

fn required_value<'a>(filter: &'a TableFilterClause) -> CellarResult<&'a str> {
    filter
        .value
        .as_deref()
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| {
            CellarError::invalid_config(format!("filter on '{}' needs a value", filter.column))
        })
}

fn push_param(parameters: &mut Vec<Value>, value: Value) -> String {
    let name = format!("@p{}", parameters.len());
    parameters.push(json!({ "name": name, "value": value }));
    name
}

fn bind_value(table: &Table, column: &str, raw: &str) -> Value {
    let data_type = table
        .columns
        .iter()
        .find(|c| c.name == column)
        .map(|c| c.data_type.as_str())
        .unwrap_or("string");
    match data_type {
        "integer" | "int" | "bigint" | "long" => raw
            .parse::<i64>()
            .map(Value::from)
            .unwrap_or_else(|_| Value::String(raw.to_string())),
        "double" | "float" | "number" => raw
            .parse::<f64>()
            .map(Value::from)
            .unwrap_or_else(|_| Value::String(raw.to_string())),
        "boolean" | "bool" => match raw.to_ascii_lowercase().as_str() {
            "true" | "1" => Value::Bool(true),
            "false" | "0" => Value::Bool(false),
            _ => Value::String(raw.to_string()),
        },
        _ => Value::String(raw.to_string()),
    }
}

fn is_json_column(table: &Table, column: &str) -> bool {
    table
        .columns
        .iter()
        .find(|c| c.name == column)
        .map(|c| {
            matches!(
                c.data_type.as_str(),
                "json" | "jsonb" | "object" | "array" | "map"
            )
        })
        .unwrap_or(false)
}

/// Resolve a column name into a Cosmos path expression.
///
/// Exact table column names win first (so keys with spaces, Unicode, or `.`
/// are addressed as a single property). Otherwise dotted paths walk nested
/// JSON under a known root column.
fn column_path(table: &Table, column: &str) -> CellarResult<String> {
    if column.is_empty() {
        return Err(CellarError::invalid_config("filter column is empty"));
    }

    if table.columns.iter().any(|c| c.name == column) {
        return path_from_segments(&[column]);
    }

    let segments: Vec<&str> = column.split('.').filter(|s| !s.is_empty()).collect();
    if segments.is_empty() {
        return Err(CellarError::invalid_config("filter column is empty"));
    }
    let root = segments[0];
    if !table.columns.iter().any(|c| c.name == root) {
        return Err(CellarError::invalid_config(format!(
            "unknown column '{column}'"
        )));
    }
    path_from_segments(&segments)
}

fn path_from_segments(segments: &[&str]) -> CellarResult<String> {
    for segment in segments {
        if !is_safe_property_name(segment) {
            return Err(CellarError::invalid_config(format!(
                "invalid filter column segment '{segment}'"
            )));
        }
    }
    Ok(format!(
        "c{}",
        segments
            .iter()
            .map(|s| format!("[\"{s}\"]"))
            .collect::<String>()
    ))
}

/// Property names usable inside Cosmos bracket notation `c["…"]`.
/// Reject quotes and control characters that would break or inject into SQL.
fn is_safe_property_name(value: &str) -> bool {
    !value.is_empty()
        && !value.contains('"')
        && !value.contains('\\')
        && !value.chars().any(char::is_control)
}

/// Map `foo%` / `%foo` / `%foo%` / `foo` to Cosmos string functions. Returns
/// `None` when the pattern needs `_` or an unsupported `%` layout.
fn like_to_cosmos(expr: &str, pattern: &str, parameters: &mut Vec<Value>) -> Option<String> {
    if pattern.contains('_') {
        return None;
    }
    let percent_count = pattern.matches('%').count();
    if percent_count > 2 {
        return None;
    }
    if pattern == "%" {
        return Some("true".into());
    }
    let upper_expr = format!("UPPER({expr})");
    if !pattern.contains('%') {
        let param = push_param(parameters, Value::String(pattern.to_ascii_uppercase()));
        return Some(format!("STRINGEQUALS({upper_expr}, {param})"));
    }
    if let Some(inner) = pattern.strip_prefix('%').and_then(|p| p.strip_suffix('%')) {
        if inner.contains('%') {
            return None;
        }
        let param = push_param(parameters, Value::String(inner.to_ascii_uppercase()));
        return Some(format!("CONTAINS({upper_expr}, {param})"));
    }
    if let Some(prefix) = pattern.strip_suffix('%') {
        if prefix.contains('%') {
            return None;
        }
        let param = push_param(parameters, Value::String(prefix.to_ascii_uppercase()));
        return Some(format!("STARTSWITH({upper_expr}, {param})"));
    }
    if let Some(suffix) = pattern.strip_prefix('%') {
        if suffix.contains('%') {
            return None;
        }
        let param = push_param(parameters, Value::String(suffix.to_ascii_uppercase()));
        return Some(format!("ENDSWITH({upper_expr}, {param})"));
    }
    None
}

pub(crate) fn sort_documents(documents: &mut [Map<String, Value>], sorts: &[TableSortClause]) {
    if sorts.is_empty() {
        return;
    }
    documents.sort_by(|left, right| {
        for sort in sorts {
            let lv = left.get(&sort.column).unwrap_or(&Value::Null);
            let rv = right.get(&sort.column).unwrap_or(&Value::Null);
            let cmp = compare_json_values(lv, rv);
            let cmp = match sort.direction {
                SortDirection::Asc => cmp,
                SortDirection::Desc => cmp.reverse(),
            };
            if cmp != std::cmp::Ordering::Equal {
                return cmp;
            }
        }
        std::cmp::Ordering::Equal
    });
}

fn compare_json_values(left: &Value, right: &Value) -> std::cmp::Ordering {
    use std::cmp::Ordering;
    match (left, right) {
        (Value::Null, Value::Null) => Ordering::Equal,
        (Value::Null, _) => Ordering::Less,
        (_, Value::Null) => Ordering::Greater,
        (Value::Number(a), Value::Number(b)) => a
            .as_f64()
            .partial_cmp(&b.as_f64())
            .unwrap_or(Ordering::Equal),
        (Value::String(a), Value::String(b)) => a.cmp(b),
        (Value::Bool(a), Value::Bool(b)) => a.cmp(b),
        _ => left.to_string().cmp(&right.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mapping::column;
    use crate::SCHEMA_NAME;
    use cellar_core::query::SortDirection;
    use serde_json::json;

    fn sample_table() -> Table {
        Table {
            name: "ibis".into(),
            schema: SCHEMA_NAME.into(),
            row_count: None,
            columns: vec![
                column("id", "string", false, true, 1),
                column("data", "json", true, false, 2),
                column("IbisServerCode", "string", true, false, 3),
                column("Order Date", "string", true, false, 4),
                column("a.b", "string", true, false, 5),
            ],
            primary_key: vec!["id".into()],
            foreign_keys: vec![],
            indexes: vec![],
        }
    }

    #[test]
    fn builds_contains_filter_on_json_column_via_tostring() {
        let query = build_browse_query(
            &sample_table(),
            &[TableFilterClause {
                column: "data".into(),
                operator: TableFilterOperator::Contains,
                value: Some("ajhackett".into()),
            }],
            &[],
        )
        .expect("query");
        assert_eq!(
            query.sql,
            "SELECT * FROM c WHERE CONTAINS(UPPER(ToString(c[\"data\"])), @p0)"
        );
        assert_eq!(query.parameters[0]["value"], json!("AJHACKETT"));
    }

    #[test]
    fn builds_equals_and_nested_path_filters() {
        let query = build_browse_query(
            &sample_table(),
            &[
                TableFilterClause {
                    column: "IbisServerCode".into(),
                    operator: TableFilterOperator::Equals,
                    value: Some("skypark".into()),
                },
                TableFilterClause {
                    column: "data.Customer".into(),
                    operator: TableFilterOperator::IsNotNull,
                    value: None,
                },
            ],
            &[TableSortClause {
                column: "id".into(),
                direction: SortDirection::Desc,
            }],
        )
        .expect("query");
        assert_eq!(
            query.sql,
            "SELECT * FROM c WHERE c[\"IbisServerCode\"] = @p0 AND (IS_DEFINED(c[\"data\"][\"Customer\"]) AND NOT IS_NULL(c[\"data\"][\"Customer\"]))"
        );
        assert!(!query.sql.contains("ORDER BY"));
    }

    #[test]
    fn filters_columns_with_spaces_and_dotted_literal_names() {
        let table = sample_table();
        let spaced = build_browse_query(
            &table,
            &[TableFilterClause {
                column: "Order Date".into(),
                operator: TableFilterOperator::Equals,
                value: Some("2024-01-01".into()),
            }],
            &[],
        )
        .expect("spaced");
        assert_eq!(spaced.sql, "SELECT * FROM c WHERE c[\"Order Date\"] = @p0");

        let dotted = build_browse_query(
            &table,
            &[TableFilterClause {
                column: "a.b".into(),
                operator: TableFilterOperator::Equals,
                value: Some("x".into()),
            }],
            &[],
        )
        .expect("dotted literal");
        assert_eq!(dotted.sql, "SELECT * FROM c WHERE c[\"a.b\"] = @p0");
    }

    #[test]
    fn rejects_unknown_filter_columns() {
        let table = Table {
            name: "ibis".into(),
            schema: SCHEMA_NAME.into(),
            row_count: None,
            columns: vec![column("id", "string", false, true, 1)],
            primary_key: vec!["id".into()],
            foreign_keys: vec![],
            indexes: vec![],
        };
        let err = build_browse_query(
            &table,
            &[TableFilterClause {
                column: "nope".into(),
                operator: TableFilterOperator::Equals,
                value: Some("x".into()),
            }],
            &[],
        )
        .expect_err("unknown column");
        assert!(err.to_string().contains("unknown column"));
    }

    #[test]
    fn rejects_unsupported_like_patterns() {
        let err = build_browse_query(
            &sample_table(),
            &[TableFilterClause {
                column: "IbisServerCode".into(),
                operator: TableFilterOperator::Like,
                value: Some("foo_bar".into()),
            }],
            &[],
        )
        .expect_err("underscore like");
        assert!(err.to_string().contains("LIKE supports only"));
    }

    #[test]
    fn preserves_whitespace_in_filter_values() {
        let query = build_browse_query(
            &sample_table(),
            &[TableFilterClause {
                column: "IbisServerCode".into(),
                operator: TableFilterOperator::Equals,
                value: Some("  padded  ".into()),
            }],
            &[],
        )
        .expect("query");
        assert_eq!(query.parameters[0]["value"], json!("  padded  "));
    }

    #[test]
    fn builds_count_query_from_same_filters() {
        let query = build_count_query(
            &sample_table(),
            &[TableFilterClause {
                column: "IbisServerCode".into(),
                operator: TableFilterOperator::Equals,
                value: Some("skypark".into()),
            }],
        )
        .expect("count");
        assert_eq!(
            query.sql,
            "SELECT VALUE COUNT(1) FROM c WHERE c[\"IbisServerCode\"] = @p0"
        );
    }
}
