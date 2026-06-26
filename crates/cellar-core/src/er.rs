//! Entity-relationship graph derivation.
//!
//! ER diagrams are built from the engine-neutral [`crate::schema`] types that
//! every driver already populates during `introspect`. Foreign-key metadata is
//! part of the shared `Schema` contract (see [`crate::schema::ForeignKey`]), so
//! any engine whose driver fills in [`crate::schema::Table::foreign_keys`] gets
//! ER diagrams for free — there is no per-engine ER code to write. Postgres is
//! the only driver wired today, but the contract is deliberately engine-neutral.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use specta::Type;

use crate::schema::Database;

/// A column rendered inside an ER node. Carries just enough for the diagram:
/// name, type, and key-role badges.
#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
pub struct ErColumn {
    pub name: String,
    pub data_type: String,
    pub nullable: bool,
    pub is_primary_key: bool,
    /// `true` when the column participates in at least one outgoing foreign key.
    pub is_foreign_key: bool,
}

/// One table in the diagram. `id` is `"schema.table"`; edges reference nodes by
/// this id.
#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
pub struct ErNode {
    pub id: String,
    pub schema: String,
    pub name: String,
    pub columns: Vec<ErColumn>,
    pub primary_key: Vec<String>,
    pub row_count: Option<u64>,
}

/// A foreign-key relationship, drawn as an edge from the referencing table to
/// the referenced table.
#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
pub struct ErEdge {
    /// Stable id derived from the endpoints and constraint name.
    pub id: String,
    pub constraint_name: String,
    /// `"schema.table"` of the table that holds the FK columns.
    pub source: String,
    /// `"schema.table"` of the referenced table.
    pub target: String,
    pub source_columns: Vec<String>,
    pub target_columns: Vec<String>,
}

/// The full graph for one database, scoped to a selection of schemas.
#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
pub struct ErGraph {
    pub database: String,
    /// Schema names present in this graph, sorted — drives the show/hide UI.
    pub schemas: Vec<String>,
    pub nodes: Vec<ErNode>,
    pub edges: Vec<ErEdge>,
}

fn node_id(schema: &str, table: &str) -> String {
    format!("{schema}.{table}")
}

/// Build an ER graph for `database` out of an introspected database tree.
///
/// When `schemas` is `Some`, only tables in those schemas are included; `None`
/// includes every schema. Edges are kept only when both endpoints are present
/// in the resulting node set, so hiding a schema also drops the relationships
/// that would dangle out of the view. Returns `None` when `database` is absent
/// from `databases`.
pub fn build_er_graph(
    databases: &[Database],
    database: &str,
    schemas: Option<&[String]>,
) -> Option<ErGraph> {
    let db = databases.iter().find(|d| d.name == database)?;
    let allow: Option<BTreeSet<&str>> = schemas.map(|s| s.iter().map(String::as_str).collect());
    let included = |schema: &str| allow.as_ref().is_none_or(|a| a.contains(schema));

    let mut nodes = Vec::new();
    let mut node_ids = BTreeSet::new();
    let mut schema_names = BTreeSet::new();

    for schema in &db.schemas {
        if !included(&schema.name) {
            continue;
        }
        schema_names.insert(schema.name.clone());
        for table in &schema.tables {
            let fk_cols: BTreeSet<&str> = table
                .foreign_keys
                .iter()
                .flat_map(|fk| fk.columns.iter().map(String::as_str))
                .collect();
            let columns = table
                .columns
                .iter()
                .map(|c| ErColumn {
                    name: c.name.clone(),
                    data_type: c.data_type.clone(),
                    nullable: c.nullable,
                    is_primary_key: c.is_primary_key,
                    is_foreign_key: fk_cols.contains(c.name.as_str()),
                })
                .collect();
            let id = node_id(&schema.name, &table.name);
            node_ids.insert(id.clone());
            nodes.push(ErNode {
                id,
                schema: schema.name.clone(),
                name: table.name.clone(),
                columns,
                primary_key: table.primary_key.clone(),
                row_count: table.row_count,
            });
        }
    }

    let mut edges = Vec::new();
    for schema in &db.schemas {
        if !included(&schema.name) {
            continue;
        }
        for table in &schema.tables {
            let source = node_id(&schema.name, &table.name);
            for fk in &table.foreign_keys {
                let target = node_id(&fk.referenced_schema, &fk.referenced_table);
                // Skip relationships whose referenced table was filtered out so
                // every rendered edge connects two real nodes.
                if !node_ids.contains(&target) {
                    continue;
                }
                edges.push(ErEdge {
                    id: format!("{source}->{target}:{}", fk.name),
                    constraint_name: fk.name.clone(),
                    source: source.clone(),
                    target,
                    source_columns: fk.columns.clone(),
                    target_columns: fk.referenced_columns.clone(),
                });
            }
        }
    }

    Some(ErGraph {
        database: db.name.clone(),
        schemas: schema_names.into_iter().collect(),
        nodes,
        edges,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::{Column, ForeignKey, Schema, Table};

    fn col(name: &str, pk: bool) -> Column {
        Column {
            name: name.into(),
            data_type: "int8".into(),
            nullable: !pk,
            default: None,
            is_primary_key: pk,
            ordinal: 1,
            comment: None,
        }
    }

    fn table_with_fk(name: &str, fks: Vec<ForeignKey>) -> Table {
        Table {
            name: name.into(),
            schema: "public".into(),
            row_count: Some(10),
            columns: vec![col("id", true), col("customer_id", false)],
            primary_key: vec!["id".into()],
            foreign_keys: fks,
            indexes: vec![],
        }
    }

    fn sample() -> Vec<Database> {
        let orders = table_with_fk(
            "orders",
            vec![ForeignKey {
                name: "orders_customer_fk".into(),
                columns: vec!["customer_id".into()],
                referenced_schema: "public".into(),
                referenced_table: "customers".into(),
                referenced_columns: vec!["id".into()],
            }],
        );
        let customers = table_with_fk("customers", vec![]);
        let audit = Table {
            name: "audit".into(),
            schema: "internal".into(),
            row_count: None,
            columns: vec![col("id", true)],
            primary_key: vec!["id".into()],
            foreign_keys: vec![],
            indexes: vec![],
        };
        vec![Database {
            name: "shop".into(),
            is_default: true,
            schemas: vec![
                Schema {
                    name: "public".into(),
                    tables: vec![orders, customers],
                    views: vec![],
                },
                Schema {
                    name: "internal".into(),
                    tables: vec![audit],
                    views: vec![],
                },
            ],
        }]
    }

    #[test]
    fn builds_nodes_and_edges_for_all_schemas() {
        let graph = build_er_graph(&sample(), "shop", None).expect("graph");
        assert_eq!(graph.schemas, vec!["internal", "public"]);
        assert_eq!(graph.nodes.len(), 3);
        assert_eq!(graph.edges.len(), 1);
        let edge = &graph.edges[0];
        assert_eq!(edge.source, "public.orders");
        assert_eq!(edge.target, "public.customers");
        assert_eq!(edge.source_columns, vec!["customer_id"]);
    }

    #[test]
    fn marks_foreign_key_columns() {
        let graph = build_er_graph(&sample(), "shop", None).expect("graph");
        let orders = graph
            .nodes
            .iter()
            .find(|n| n.id == "public.orders")
            .unwrap();
        let fk_col = orders
            .columns
            .iter()
            .find(|c| c.name == "customer_id")
            .unwrap();
        assert!(fk_col.is_foreign_key);
        let pk_col = orders.columns.iter().find(|c| c.name == "id").unwrap();
        assert!(pk_col.is_primary_key);
        assert!(!pk_col.is_foreign_key);
    }

    #[test]
    fn schema_filter_drops_nodes_and_dangling_edges() {
        let only_public = vec!["public".to_string()];
        let graph = build_er_graph(&sample(), "shop", Some(&only_public)).expect("graph");
        assert_eq!(graph.schemas, vec!["public"]);
        assert_eq!(graph.nodes.len(), 2);
        assert_eq!(graph.edges.len(), 1);

        // Filtering to only `internal` keeps the audit node but drops the
        // public→public edge entirely.
        let only_internal = vec!["internal".to_string()];
        let graph = build_er_graph(&sample(), "shop", Some(&only_internal)).expect("graph");
        assert_eq!(graph.nodes.len(), 1);
        assert!(graph.edges.is_empty());
    }

    #[test]
    fn unknown_database_is_none() {
        assert!(build_er_graph(&sample(), "nope", None).is_none());
    }
}
