use std::ops::ControlFlow;

use sqlparser::{
    ast::{BinaryOperator, DataType, Expr, Query, SetExpr, Statement, TableFactor, Visit, Visitor},
    dialect::{PostgreSqlDialect, SQLiteDialect},
    parser::Parser,
};

pub fn validate(sql: &str, postgres: bool) -> Result<String, String> {
    if cellar_sql::sql_contains_credentials(sql) {
        return Err("SQL resembling credentials or connection strings is disabled".into());
    }
    if sql.len() > 32_768 {
        return Err("SQL exceeds the 32 KiB limit".into());
    }
    let dialect: &dyn sqlparser::dialect::Dialect = if postgres {
        &PostgreSqlDialect {}
    } else {
        &SQLiteDialect {}
    };
    let statements = Parser::parse_sql(dialect, sql)
        .map_err(|_| "SQL could not be parsed; nothing was executed")?;
    if statements.len() != 1 || !matches!(statements[0], Statement::Query(_)) {
        return Err("Only one read-only SELECT or WITH query is supported".into());
    }
    if let ControlFlow::Break(reason) = statements[0].visit(&mut Guard) {
        return Err(reason.into());
    }
    Ok(statements[0].to_string())
}

fn read_body(body: &SetExpr) -> bool {
    match body {
        SetExpr::Select(select) => select.into.is_none(),
        SetExpr::Query(query) => read_body(&query.body),
        SetExpr::SetOperation { left, right, .. } => read_body(left) && read_body(right),
        SetExpr::Values(_) => true,
        _ => false,
    }
}

struct Guard;

impl Visitor for Guard {
    type Break = &'static str;

    fn pre_visit_query(&mut self, query: &Query) -> ControlFlow<Self::Break> {
        if !read_body(&query.body) || !query.locks.is_empty() {
            return ControlFlow::Break("Mutating CTEs, SELECT INTO and row locks are disabled");
        }
        ControlFlow::Continue(())
    }

    fn pre_visit_statement(&mut self, statement: &Statement) -> ControlFlow<Self::Break> {
        if !matches!(statement, Statement::Query(_)) {
            return ControlFlow::Break("Nested write statements are disabled");
        }
        ControlFlow::Continue(())
    }

    fn pre_visit_table_factor(&mut self, table: &TableFactor) -> ControlFlow<Self::Break> {
        match table {
            TableFactor::Table { args: None, .. }
            | TableFactor::Derived { .. }
            | TableFactor::NestedJoin { .. } => ControlFlow::Continue(()),
            _ => ControlFlow::Break("Table functions and external sources are disabled"),
        }
    }

    fn pre_visit_expr(&mut self, expression: &Expr) -> ControlFlow<Self::Break> {
        if matches!(
            expression,
            Expr::Cast {
                data_type: DataType::Custom(..),
                ..
            } | Expr::BinaryOp {
                op: BinaryOperator::Custom(_) | BinaryOperator::PGCustomBinaryOperator(_),
                ..
            }
        ) {
            return ControlFlow::Break("Custom casts and operators are disabled");
        }
        if let Expr::Function(function) = expression {
            let parts = &function.name.0;
            let name = parts
                .last()
                .map(|name| name.value.to_lowercase())
                .unwrap_or_default();
            let safe = [
                "count",
                "sum",
                "avg",
                "min",
                "max",
                "lower",
                "upper",
                "length",
                "abs",
                "round",
                "coalesce",
                "nullif",
                "trim",
                "substr",
                "substring",
                "replace",
                "row_number",
                "rank",
                "dense_rank",
            ];
            if !safe.contains(&name.as_str())
                || parts.len() > 2
                || (parts.len() == 2 && parts[0].value != "pg_catalog")
            {
                return ControlFlow::Break("This function is outside the read-only allowlist");
            }
        }
        ControlFlow::Continue(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_write_and_side_effect_paths() {
        for sql in [
            "DELETE FROM orders",
            "SELECT 1; DELETE FROM orders",
            "WITH x AS (DELETE FROM orders RETURNING *) SELECT * FROM x",
            "SELECT * INTO TEMP copied FROM orders",
            "SELECT * FROM orders FOR UPDATE",
            "SELECT nextval('seq')",
            "SELECT set_config('transaction_read_only','off',false)",
            "SELECT load_extension('/tmp/a')",
            "SELECT public.lower('x')",
            "SELECT * FROM generate_series(1, 1000000)",
            "COMMIT",
            "ATTACH '/tmp/a' AS x",
            "SELECT 'postgres://reader:secret@localhost/db'",
            "SELECT 1::public.custom_type",
            "SELECT 1 OPERATOR(public.+) 2",
        ] {
            assert!(validate(sql, true).is_err(), "{sql}");
        }
    }

    #[test]
    fn allows_joins_aggregates_ctes_and_quoted_semicolons() {
        for sql in [
            "SELECT ';' AS value",
            "WITH x AS (SELECT 1 AS n) SELECT n FROM x",
            "SELECT count(*), max(id) FROM orders",
            "SELECT o.id FROM orders o JOIN customers c ON c.id=o.customer_id",
            "SELECT 1 UNION SELECT 2",
            "SELECT row_number() OVER (ORDER BY id) FROM orders",
        ] {
            assert!(validate(sql, true).is_ok(), "{sql}");
        }
    }
}
