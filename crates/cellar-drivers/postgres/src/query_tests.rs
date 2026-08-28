use super::*;

#[test]
fn import_statements_cannot_mutate_multiple_rows() {
    assert!(statement_row_count_valid(RowCountCheck::AtMost, 0));
    assert!(statement_row_count_valid(RowCountCheck::AtMost, 1));
    assert!(!statement_row_count_valid(RowCountCheck::AtMost, 2));
}
use serde_json::json;

#[test]
fn normalizes_a_single_statement_with_trailing_comments() {
    let sql = " SELECT ';' AS semi; -- ok\n /* done */ ";
    assert_eq!(
        normalize_single_statement(sql).expect("single statement"),
        "SELECT ';' AS semi"
    );
}

#[test]
fn rejects_multiple_statements() {
    let err = normalize_single_statement("SELECT 1; DROP TABLE users")
        .expect_err("multiple statements rejected");
    assert!(err.to_string().contains("one statement"));
}

#[test]
fn ignores_semicolons_in_dollar_quotes() {
    let sql = "SELECT $$semi;colon$$ AS body";
    assert_eq!(normalize_single_statement(sql).unwrap(), sql);
}

#[test]
fn parses_json_plan_nodes() {
    let plan = json!({
        "Node Type": "Seq Scan",
        "Relation Name": "orders",
        "Startup Cost": 0.0,
        "Total Cost": 12.5,
        "Plan Rows": 10,
        "Filter": "(total > 10)"
    });
    let parsed = parse_plan_node(&plan).expect("parse plan");
    assert_eq!(parsed.node_type, "Seq Scan");
    assert_eq!(parsed.relation_name.as_deref(), Some("orders"));
    assert_eq!(parsed.details[0].label, "Filter");
}
