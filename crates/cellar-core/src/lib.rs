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
pub use query::{Query, QueryResult};
pub use schema::{Column, Database, ForeignKey, Index, Schema, Table, View};
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
            rows_affected: None,
            duration_ms: 12,
            truncated: false,
        };
        let s = serde_json::to_string(&result).expect("serialize");
        let back: QueryResult = serde_json::from_str(&s).expect("deserialize");
        assert_eq!(back.rows.len(), 2);
        assert_eq!(back.columns.len(), 2);
        assert!(matches!(back.rows[1][1], CellValue::Null));
    }

    #[test]
    fn cellar_error_round_trips() {
        let err = CellarError::Connection("dns lookup failed".into());
        let s = serde_json::to_string(&err).expect("serialize");
        let back: CellarError = serde_json::from_str(&s).expect("deserialize");
        assert!(matches!(back, CellarError::Connection(_)));
    }
}
