//! Schema comparison and DDL migration generation for Cellar.
//!
//! Two responsibilities, split across modules:
//!
//! - [`diff`] — structurally compare two [`cellar_core::schema::Schema`] trees
//!   into a typed, render-ready [`SchemaDiff`].
//! - [`migration`] — turn that diff into an ordered, idempotent, dialect-aware
//!   list of [`MigrationStatement`]s and assemble selected statements into a
//!   runnable script.
//!
//! [`snapshot`] defines the on-disk snapshot shape for offline comparison.
//!
//! This is schema diffing only — row/data migration is out of scope and lives
//! nowhere in this crate. Postgres DDL is implemented first; the [`Dialect`]
//! seam keeps identifier quoting engine-correct for the rest.

pub mod diff;
pub mod migration;
pub mod snapshot;

use serde::{Deserialize, Serialize};
use specta::Type;

pub use cellar_sql::Dialect;
pub use diff::{
    diff_schemas, ChangeStatus, ColumnDiff, DiffSummary, ForeignKeyDiff, IndexDiff, PrimaryKeyDiff,
    SchemaDiff, TableDiff, ViewDiff,
};
pub use migration::{assemble_script, build_migration, MigrationKind, MigrationStatement};
pub use snapshot::{SchemaSnapshot, SchemaSnapshotMeta};

/// Bundled output of a comparison: the render-ready diff tree, the migration
/// statements that transform source into target, and the dialect the DDL was
/// generated for. Returned by the `compare_schemas` IPC command so the UI gets
/// everything (including the dialect to round-trip back into script assembly)
/// in one call.
#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
pub struct SchemaComparison {
    pub diff: SchemaDiff,
    pub statements: Vec<MigrationStatement>,
    pub dialect: Dialect,
}

/// Compare two schemas and build the migration in one step. `schema` is the
/// namespace the generated DDL operates in (typically the source schema name,
/// since the script is applied to the source to make it match the target).
pub fn compare(
    source: &cellar_core::schema::Schema,
    target: &cellar_core::schema::Schema,
    source_label: impl Into<String>,
    target_label: impl Into<String>,
    schema: &str,
    dialect: Dialect,
) -> SchemaComparison {
    let diff = diff_schemas(source, target, source_label, target_label);
    let statements = build_migration(&diff, schema, dialect);
    SchemaComparison {
        diff,
        statements,
        dialect,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cellar_core::schema::{Column, ForeignKey, Index, Schema, Table, View};

    fn col(name: &str, ty: &str, nullable: bool) -> Column {
        Column {
            name: name.into(),
            data_type: ty.into(),
            nullable,
            default: None,
            is_primary_key: false,
            ordinal: 0,
            comment: None,
        }
    }

    fn table(name: &str, columns: Vec<Column>) -> Table {
        Table {
            name: name.into(),
            schema: "public".into(),
            row_count: None,
            columns,
            primary_key: vec!["id".into()],
            foreign_keys: vec![],
            indexes: vec![],
        }
    }

    fn schema(tables: Vec<Table>) -> Schema {
        Schema {
            name: "public".into(),
            tables,
            views: vec![],
        }
    }

    #[test]
    fn classifies_added_removed_and_modified_tables() {
        let source = schema(vec![
            table("orders", vec![col("id", "int4", false)]),
            table("legacy", vec![col("id", "int4", false)]),
        ]);
        let target = schema(vec![
            table(
                "orders",
                vec![col("id", "int4", false), col("note", "text", true)],
            ),
            table("customers", vec![col("id", "int4", false)]),
        ]);

        let diff = diff_schemas(&source, &target, "src", "tgt");
        let by = |name: &str| diff.tables.iter().find(|t| t.name == name).unwrap().status;
        assert_eq!(by("orders"), ChangeStatus::Modified);
        assert_eq!(by("legacy"), ChangeStatus::Removed);
        assert_eq!(by("customers"), ChangeStatus::Added);
        assert_eq!(diff.summary.tables_added, 1);
        assert_eq!(diff.summary.tables_removed, 1);
        assert_eq!(diff.summary.tables_modified, 1);
    }

    #[test]
    fn detects_column_field_changes() {
        let source = schema(vec![table(
            "orders",
            vec![col("id", "int4", false), col("total", "int4", true)],
        )]);
        let mut changed = col("total", "numeric", false);
        changed.default = Some("0".into());
        let target = schema(vec![table(
            "orders",
            vec![col("id", "int4", false), changed],
        )]);

        let diff = diff_schemas(&source, &target, "src", "tgt");
        let orders = diff.tables.iter().find(|t| t.name == "orders").unwrap();
        let total = orders.columns.iter().find(|c| c.name == "total").unwrap();
        assert_eq!(total.status, ChangeStatus::Modified);
        assert!(total.changes.iter().any(|c| c.contains("numeric")));
        assert!(total.changes.iter().any(|c| c.contains("NOT NULL")));
    }

    #[test]
    fn builds_create_and_drop_table_ddl() {
        let source = schema(vec![table("legacy", vec![col("id", "int4", false)])]);
        let target = schema(vec![table(
            "customers",
            vec![col("id", "int4", false), col("email", "text", false)],
        )]);

        let diff = diff_schemas(&source, &target, "src", "tgt");
        let statements = build_migration(&diff, "public", Dialect::Postgres);

        let create = statements
            .iter()
            .find(|s| s.kind == MigrationKind::CreateTable)
            .expect("create table");
        assert!(create
            .sql
            .contains("CREATE TABLE IF NOT EXISTS \"public\".\"customers\""));
        assert!(create.sql.contains("\"email\" text NOT NULL"));
        assert!(create.sql.contains("PRIMARY KEY (\"id\")"));

        let drop = statements
            .iter()
            .find(|s| s.kind == MigrationKind::DropTable)
            .expect("drop table");
        assert!(drop.destructive);
        assert!(drop
            .sql
            .contains("DROP TABLE IF EXISTS \"public\".\"legacy\""));
    }

    #[test]
    fn builds_alter_column_ddl_for_type_change() {
        let source = schema(vec![table(
            "orders",
            vec![col("id", "int4", false), col("total", "int4", true)],
        )]);
        let target = schema(vec![table(
            "orders",
            vec![col("id", "int4", false), col("total", "bigint", false)],
        )]);

        let diff = diff_schemas(&source, &target, "src", "tgt");
        let statements = build_migration(&diff, "public", Dialect::Postgres);
        let alter = statements
            .iter()
            .find(|s| s.kind == MigrationKind::AlterColumn)
            .expect("alter column");
        assert!(alter.destructive, "type change is destructive");
        assert!(alter.sql.contains("ALTER COLUMN \"total\" TYPE bigint"));
        assert!(alter.sql.contains("SET NOT NULL"));
    }

    #[test]
    fn add_column_and_index_and_foreign_key() {
        let mut tgt_orders = table(
            "orders",
            vec![col("id", "int4", false), col("customer_id", "int4", false)],
        );
        tgt_orders.indexes = vec![Index {
            name: "orders_customer_idx".into(),
            columns: vec!["customer_id".into()],
            unique: false,
            primary: false,
        }];
        tgt_orders.foreign_keys = vec![ForeignKey {
            name: "orders_customer_fk".into(),
            columns: vec!["customer_id".into()],
            referenced_schema: "public".into(),
            referenced_table: "customers".into(),
            referenced_columns: vec!["id".into()],
        }];

        let source = schema(vec![table("orders", vec![col("id", "int4", false)])]);
        let target = schema(vec![tgt_orders]);

        let diff = diff_schemas(&source, &target, "src", "tgt");
        let statements = build_migration(&diff, "public", Dialect::Postgres);

        assert!(statements.iter().any(|s| s.kind == MigrationKind::AddColumn
            && s.sql.contains("ADD COLUMN IF NOT EXISTS \"customer_id\"")));
        assert!(statements
            .iter()
            .any(|s| s.kind == MigrationKind::CreateIndex
                && s.sql
                    .contains("CREATE INDEX IF NOT EXISTS \"orders_customer_idx\"")));
        let fk = statements
            .iter()
            .find(|s| s.kind == MigrationKind::AddForeignKey)
            .expect("foreign key");
        assert!(fk.sql.contains("DROP CONSTRAINT IF EXISTS"));
        assert!(fk
            .sql
            .contains("REFERENCES \"public\".\"customers\" (\"id\")"));
    }

    #[test]
    fn drops_modified_index_before_altering_its_column() {
        let mut src = table(
            "orders",
            vec![col("id", "int4", false), col("total", "int4", false)],
        );
        src.indexes = vec![Index {
            name: "orders_total_idx".into(),
            columns: vec!["total".into()],
            unique: false,
            primary: false,
        }];
        let mut tgt = table(
            "orders",
            vec![col("id", "int4", false), col("total", "bigint", false)],
        );
        // Same index, now unique → Modified, so it is dropped then recreated.
        tgt.indexes = vec![Index {
            name: "orders_total_idx".into(),
            columns: vec!["total".into()],
            unique: true,
            primary: false,
        }];

        let diff = diff_schemas(&schema(vec![src]), &schema(vec![tgt]), "s", "t");
        let statements = build_migration(&diff, "public", Dialect::Postgres);
        let pos = |kind: MigrationKind| statements.iter().position(|s| s.kind == kind);
        let drop = pos(MigrationKind::DropIndex).expect("drop index");
        let alter = pos(MigrationKind::AlterColumn).expect("alter column");
        let create = pos(MigrationKind::CreateIndex).expect("create index");
        assert!(
            drop < alter,
            "index must be dropped before the column alter"
        );
        assert!(alter < create, "index must be recreated after the alter");
    }

    #[test]
    fn assembles_transactional_script_in_order() {
        let source = schema(vec![table("legacy", vec![col("id", "int4", false)])]);
        let target = schema(vec![table("customers", vec![col("id", "int4", false)])]);
        let diff = diff_schemas(&source, &target, "src", "tgt");
        let statements = build_migration(&diff, "public", Dialect::Postgres);

        let script = assemble_script(&statements, Dialect::Postgres, true);
        assert!(script.starts_with("-- Cellar schema migration"));
        assert!(script.contains("BEGIN;"));
        assert!(script.contains("COMMIT;"));
        // Drop must precede create so the script applies cleanly.
        let drop_at = script.find("DROP TABLE").expect("drop present");
        let create_at = script.find("CREATE TABLE").expect("create present");
        assert!(drop_at < create_at);
    }

    #[test]
    fn detects_view_changes_and_generates_replace() {
        let source = Schema {
            name: "public".into(),
            tables: vec![],
            views: vec![View {
                name: "active".into(),
                schema: "public".into(),
                columns: vec![],
                definition: Some("SELECT 1".into()),
            }],
        };
        let target = Schema {
            name: "public".into(),
            tables: vec![],
            views: vec![View {
                name: "active".into(),
                schema: "public".into(),
                columns: vec![],
                definition: Some("SELECT 2".into()),
            }],
        };
        let diff = diff_schemas(&source, &target, "src", "tgt");
        assert_eq!(diff.views[0].status, ChangeStatus::Modified);
        let statements = build_migration(&diff, "public", Dialect::Postgres);
        let view = statements
            .iter()
            .find(|s| s.kind == MigrationKind::ReplaceView)
            .expect("replace view");
        assert!(view
            .sql
            .contains("CREATE OR REPLACE VIEW \"public\".\"active\""));
    }

    #[test]
    fn empty_selection_assembles_to_no_op() {
        let script = assemble_script(&[], Dialect::Postgres, true);
        assert!(script.contains("No changes selected"));
        assert!(!script.contains("BEGIN;"));
    }
}
