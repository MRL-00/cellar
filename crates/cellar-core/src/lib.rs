//! Shared traits, types, and errors for Cellar. Every driver implements
//! [`Driver`]; every IPC type that crosses into the frontend is defined here
//! and exported via specta in `apps/desktop/src-tauri`.

pub mod driver;
pub mod error;
pub mod query;
pub mod schema;
pub mod value;

pub use driver::{Connection, ConnectionConfig, Driver, DriverInfo, Engine, EnvTag, SslMode};
pub use error::{CellarError, CellarResult};
pub use query::{
    DatabaseNotice, NoticeCapture, NoticeSeverity, PlanDetail, PlanMode, PlanNode, Query,
    QueryPlan, QueryResult,
};
pub use schema::{
    Column, Database, ForeignKey, Index, Schema, Table, UsageDefinition, UsageKind, UsageReference,
    View,
};
pub use value::{CellValue, ColumnMeta, Row};

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use serde_json::json;

    #[test]
    fn round_trips_a_database_tree() {
        let original = Database {
            name: "shop_eu".into(),
            is_default: true,
            schemas: vec![Schema {
                name: "public".into(),
                tables: vec![Table {
                    name: "orders".into(),
                    schema: "public".into(),
                    row_count: Some(1_800_000),
                    columns: vec![Column {
                        name: "id".into(),
                        data_type: "int8".into(),
                        nullable: false,
                        default: Some("nextval(...)".into()),
                        is_primary_key: true,
                        ordinal: 1,
                        comment: None,
                    }],
                    primary_key: vec!["id".into()],
                    foreign_keys: vec![ForeignKey {
                        name: "orders_customer_fk".into(),
                        columns: vec!["customer_id".into()],
                        referenced_schema: "public".into(),
                        referenced_table: "customers".into(),
                        referenced_columns: vec!["id".into()],
                    }],
                    indexes: vec![Index {
                        name: "orders_pkey".into(),
                        columns: vec!["id".into()],
                        unique: true,
                        primary: true,
                    }],
                }],
                views: vec![],
            }],
        };
        let json = serde_json::to_string(&original).expect("serialize");
        let round: Database = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(round, original);
    }

    #[test]
    fn round_trips_every_cell_value_variant() {
        let values = vec![
            CellValue::Null,
            CellValue::Bool(true),
            CellValue::Int(-42),
            CellValue::Float(std::f64::consts::PI),
            CellValue::Numeric("999999999999999999.99999".into()),
            CellValue::Text("hello".into()),
            CellValue::Bytes(vec![0, 1, 2, 0xff]),
            CellValue::Json(json!({"k": [1, 2]})),
            CellValue::Date(NaiveDate::from_ymd_opt(2026, 5, 28).unwrap()),
        ];
        for v in values {
            let s = serde_json::to_string(&v).expect("serialize cell value");
            let back: CellValue = serde_json::from_str(&s).expect("deserialize cell value");
            assert_eq!(back, v);
        }
    }

    #[test]
    fn round_trips_a_query_result() {
        let result = QueryResult {
            columns: vec![
                ColumnMeta {
                    name: "id".into(),
                    data_type: "int8".into(),
                    nullable: false,
                },
                ColumnMeta {
                    name: "email".into(),
                    data_type: "text".into(),
                    nullable: true,
                },
            ],
            rows: vec![
                vec![CellValue::Int(1), CellValue::Text("a@b.co".into())],
                vec![CellValue::Int(2), CellValue::Null],
            ],
            notices: vec![DatabaseNotice {
                severity: NoticeSeverity::Notice,
                code: Some("00000".into()),
                message: "loaded fixtures".into(),
                detail: Some("two rows materialized".into()),
                hint: None,
                timestamp: "2026-05-30T00:00:00Z".into(),
                connection_id: Some("local".into()),
                database: Some("postgres".into()),
                query_id: Some("query-1".into()),
            }],
            notice_capture: NoticeCapture::supported(),
            rows_affected: None,
            duration_ms: 12,
            truncated: false,
            total_rows: None,
        };
        let s = serde_json::to_string(&result).expect("serialize");
        let back: QueryResult = serde_json::from_str(&s).expect("deserialize");
        assert_eq!(back.rows.len(), 2);
        assert_eq!(back.columns.len(), 2);
        assert_eq!(back.notices.len(), 1);
        assert!(back.notice_capture.supported);
        assert!(matches!(back.rows[1][1], CellValue::Null));
    }

    #[test]
    fn round_trips_a_query_plan() {
        let plan = QueryPlan {
            mode: PlanMode::Estimate,
            engine: "postgres".into(),
            database: Some("postgres".into()),
            sql: "SELECT 1".into(),
            root: PlanNode {
                node_type: "Result".into(),
                relation_name: None,
                schema_name: None,
                alias: None,
                index_name: None,
                join_type: None,
                startup_cost: Some(0.0),
                total_cost: Some(0.01),
                plan_rows: Some(1),
                plan_width: Some(4),
                actual_startup_time_ms: None,
                actual_total_time_ms: None,
                actual_rows: None,
                actual_loops: None,
                details: vec![PlanDetail {
                    label: "Output".into(),
                    value: "1".into(),
                }],
                children: vec![],
            },
            planning_time_ms: Some(0.1),
            execution_time_ms: None,
            duration_ms: 3,
            raw_json: json!([{"Plan": {"Node Type": "Result"}}]),
        };
        let s = serde_json::to_string(&plan).expect("serialize");
        let back: QueryPlan = serde_json::from_str(&s).expect("deserialize");
        assert_eq!(back.mode, PlanMode::Estimate);
        assert_eq!(back.root.node_type, "Result");
        assert_eq!(back.root.details[0].label, "Output");
    }

    #[test]
    fn cellar_error_round_trips() {
        let err = CellarError::Connection("dns lookup failed".into());
        let s = serde_json::to_string(&err).expect("serialize");
        let back: CellarError = serde_json::from_str(&s).expect("deserialize");
        assert!(matches!(back, CellarError::Connection(_)));
    }
}
