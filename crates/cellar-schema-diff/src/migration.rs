//! Turn a [`SchemaDiff`] into an ordered list of dialect-specific DDL
//! statements, and assemble selected statements into a runnable script.
//!
//! Statements are emitted in a dependency-safe order (drop constraints before
//! columns/tables, create tables before the columns/indexes/keys that depend
//! on them) so the assembled script applies cleanly in sequence. Each
//! statement is idempotent where the dialect allows (`IF EXISTS` /
//! `IF NOT EXISTS`, `CREATE OR REPLACE`, drop-then-add for constraints).

use cellar_core::schema::{Column, ForeignKey, Index, Table, View};
use cellar_sql::Dialect;
use serde::{Deserialize, Serialize};
use specta::Type;

use crate::diff::{ChangeStatus, SchemaDiff, TableDiff};

/// What a single migration statement does. Drives grouping/iconography in the
/// UI and the destructive-confirmation gate.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum MigrationKind {
    CreateTable,
    DropTable,
    AddColumn,
    DropColumn,
    AlterColumn,
    AlterPrimaryKey,
    CreateIndex,
    DropIndex,
    AddForeignKey,
    DropForeignKey,
    CreateView,
    ReplaceView,
    DropView,
}

/// One reviewable, individually selectable unit of the migration. `sql` may
/// span multiple physical statements (e.g. drop-then-add a constraint) but is
/// one logical change the user toggles together.
#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
pub struct MigrationStatement {
    /// Stable id (`kind:object`) the frontend uses for selection state.
    pub id: String,
    pub kind: MigrationKind,
    /// Qualified object the statement targets, for display.
    pub object: String,
    pub description: String,
    /// `true` for statements that drop objects or can lose data — gated behind
    /// explicit confirmation in the UI.
    pub destructive: bool,
    pub sql: String,
}

/// Build the ordered statement list that transforms `diff`'s source into its
/// target. `schema` is the namespace the DDL operates in (the schema the
/// script would be applied to). Only Postgres DDL is emitted today; `dialect`
/// is threaded so identifier quoting is already engine-correct.
pub fn build_migration(
    diff: &SchemaDiff,
    schema: &str,
    dialect: Dialect,
) -> Vec<MigrationStatement> {
    let mut b = Builder::new(schema, dialect);

    // Phase 1 — drops (constraints → views → indexes → columns → tables).
    for table in &diff.tables {
        if matches!(table.status, ChangeStatus::Modified) {
            b.drop_removed_foreign_keys(table);
        }
    }
    for view in &diff.views {
        if matches!(view.status, ChangeStatus::Removed) {
            if let Some(v) = &view.source {
                b.drop_view(v);
            }
        }
    }
    for table in &diff.tables {
        if matches!(table.status, ChangeStatus::Modified) {
            b.drop_removed_indexes(table);
            b.drop_removed_columns(table);
        }
    }
    for table in &diff.tables {
        if matches!(table.status, ChangeStatus::Removed) {
            if let Some(t) = &table.source {
                b.drop_table(t);
            }
        }
    }

    // Phase 2 — creates and alters.
    for table in &diff.tables {
        match table.status {
            ChangeStatus::Added => {
                if let Some(t) = &table.target {
                    b.create_table(t);
                }
            }
            ChangeStatus::Modified => {
                b.add_columns(table);
                b.alter_columns(table);
                b.alter_primary_key(table);
            }
            _ => {}
        }
    }

    // Phase 3 — indexes then foreign keys (need their tables/columns present).
    for table in &diff.tables {
        if let Some(t) = table.target.as_ref() {
            b.create_indexes(table);
            b.add_foreign_keys(table, t);
        }
    }

    // Phase 4 — views last (may depend on the new table shapes).
    for view in &diff.views {
        match view.status {
            ChangeStatus::Added => {
                if let Some(v) = &view.target {
                    b.create_view(v, false);
                }
            }
            ChangeStatus::Modified => {
                if let Some(v) = &view.target {
                    b.create_view(v, true);
                }
            }
            _ => {}
        }
    }

    b.statements
}

/// Assemble `statements` into one script, optionally wrapped in a transaction.
/// Statements are joined in the order given (the order [`build_migration`]
/// produced, filtered to the user's selection) with a header comment.
pub fn assemble_script(
    statements: &[MigrationStatement],
    dialect: Dialect,
    wrap_in_transaction: bool,
) -> String {
    let mut out = String::new();
    out.push_str("-- Cellar schema migration\n");
    if statements.is_empty() {
        out.push_str("-- No changes selected.\n");
        return out;
    }

    let wrap = wrap_in_transaction && supports_transactional_ddl(dialect);
    if wrap {
        out.push_str("BEGIN;\n");
    }
    for statement in statements {
        out.push('\n');
        out.push_str("-- ");
        out.push_str(&statement.description);
        out.push('\n');
        out.push_str(statement.sql.trim_end());
        out.push('\n');
    }
    if wrap {
        out.push_str("\nCOMMIT;\n");
    }
    out
}

/// Whether the engine wraps DDL in a transaction safely. Postgres does;
/// MySQL implicitly commits on most DDL, so wrapping there is a no-op the UI
/// should not promise.
fn supports_transactional_ddl(dialect: Dialect) -> bool {
    matches!(
        dialect,
        Dialect::Postgres | Dialect::Sqlite | Dialect::Mssql
    )
}

struct Builder<'a> {
    schema: &'a str,
    dialect: Dialect,
    statements: Vec<MigrationStatement>,
}

impl<'a> Builder<'a> {
    fn new(schema: &'a str, dialect: Dialect) -> Self {
        Self {
            schema,
            dialect,
            statements: Vec::new(),
        }
    }

    fn qualified(&self, object: &str) -> String {
        self.dialect.quote_qualified(self.schema, object)
    }

    fn ident(&self, ident: &str) -> String {
        self.dialect.quote_ident(ident)
    }

    fn push(
        &mut self,
        kind: MigrationKind,
        object: impl Into<String>,
        description: impl Into<String>,
        destructive: bool,
        sql: String,
    ) {
        let object = object.into();
        let id = format!("{}:{object}", kind_slug(kind));
        self.statements.push(MigrationStatement {
            id,
            kind,
            object,
            description: description.into(),
            destructive,
            sql,
        });
    }

    fn create_table(&mut self, table: &Table) {
        let qualified = self.qualified(&table.name);
        let mut lines: Vec<String> = table
            .columns
            .iter()
            .map(|c| format!("  {}", self.column_definition(c)))
            .collect();
        if !table.primary_key.is_empty() {
            let cols = table
                .primary_key
                .iter()
                .map(|c| self.ident(c))
                .collect::<Vec<_>>()
                .join(", ");
            lines.push(format!("  PRIMARY KEY ({cols})"));
        }
        let sql = format!(
            "CREATE TABLE IF NOT EXISTS {qualified} (\n{}\n);",
            lines.join(",\n")
        );
        self.push(
            MigrationKind::CreateTable,
            format!("{}.{}", self.schema, table.name),
            format!("create table {}", table.name),
            false,
            sql,
        );
    }

    fn drop_table(&mut self, table: &Table) {
        let qualified = self.qualified(&table.name);
        self.push(
            MigrationKind::DropTable,
            format!("{}.{}", self.schema, table.name),
            format!("drop table {}", table.name),
            true,
            format!("DROP TABLE IF EXISTS {qualified};"),
        );
    }

    fn add_columns(&mut self, table: &TableDiff) {
        let qualified = self.qualified(&table.name);
        for col in &table.columns {
            if !matches!(col.status, ChangeStatus::Added) {
                continue;
            }
            let Some(c) = &col.target else { continue };
            self.push(
                MigrationKind::AddColumn,
                format!("{}.{}.{}", self.schema, table.name, c.name),
                format!("add column {}.{}", table.name, c.name),
                false,
                format!(
                    "ALTER TABLE {qualified} ADD COLUMN IF NOT EXISTS {};",
                    self.column_definition(c)
                ),
            );
        }
    }

    fn drop_removed_columns(&mut self, table: &TableDiff) {
        let qualified = self.qualified(&table.name);
        for col in &table.columns {
            if !matches!(col.status, ChangeStatus::Removed) {
                continue;
            }
            self.push(
                MigrationKind::DropColumn,
                format!("{}.{}.{}", self.schema, table.name, col.name),
                format!("drop column {}.{}", table.name, col.name),
                true,
                format!(
                    "ALTER TABLE {qualified} DROP COLUMN IF EXISTS {};",
                    self.ident(&col.name)
                ),
            );
        }
    }

    fn alter_columns(&mut self, table: &TableDiff) {
        let qualified = self.qualified(&table.name);
        for col in &table.columns {
            if !matches!(col.status, ChangeStatus::Modified) {
                continue;
            }
            let (Some(source), Some(target)) = (&col.source, &col.target) else {
                continue;
            };
            let mut parts: Vec<String> = Vec::new();
            let ident = self.ident(&col.name);
            let mut destructive = false;
            if source.data_type != target.data_type {
                destructive = true;
                parts.push(format!(
                    "ALTER TABLE {qualified} ALTER COLUMN {ident} TYPE {};",
                    target.data_type
                ));
            }
            if source.nullable != target.nullable {
                let clause = if target.nullable {
                    "DROP NOT NULL"
                } else {
                    "SET NOT NULL"
                };
                parts.push(format!(
                    "ALTER TABLE {qualified} ALTER COLUMN {ident} {clause};"
                ));
            }
            if source.default != target.default {
                let clause = match &target.default {
                    Some(def) => format!("SET DEFAULT {def}"),
                    None => "DROP DEFAULT".to_string(),
                };
                parts.push(format!(
                    "ALTER TABLE {qualified} ALTER COLUMN {ident} {clause};"
                ));
            }
            if parts.is_empty() {
                continue;
            }
            self.push(
                MigrationKind::AlterColumn,
                format!("{}.{}.{}", self.schema, table.name, col.name),
                format!(
                    "alter column {}.{} ({})",
                    table.name,
                    col.name,
                    col.changes.join(", ")
                ),
                destructive,
                parts.join("\n"),
            );
        }
    }

    fn alter_primary_key(&mut self, table: &TableDiff) {
        if !table.primary_key.status.is_change() {
            return;
        }
        let qualified = self.qualified(&table.name);
        // Postgres auto-names a table's primary key `<table>_pkey`. We do not
        // model constraint names, so target that convention and guard the drop
        // with IF EXISTS so a differently-named key is left for manual review.
        let pkey = self.ident(&format!("{}_pkey", table.name));
        let mut parts = vec![format!(
            "ALTER TABLE {qualified} DROP CONSTRAINT IF EXISTS {pkey};"
        )];
        if !table.primary_key.target.is_empty() {
            let cols = table
                .primary_key
                .target
                .iter()
                .map(|c| self.ident(c))
                .collect::<Vec<_>>()
                .join(", ");
            parts.push(format!("ALTER TABLE {qualified} ADD PRIMARY KEY ({cols});"));
        }
        self.push(
            MigrationKind::AlterPrimaryKey,
            format!("{}.{}", self.schema, table.name),
            format!("alter primary key on {}", table.name),
            true,
            parts.join("\n"),
        );
    }

    fn create_indexes(&mut self, table: &TableDiff) {
        for idx in &table.indexes {
            // A primary-key index is created/dropped via PRIMARY KEY, not here.
            let is_primary = idx
                .target
                .as_ref()
                .or(idx.source.as_ref())
                .map(|i| i.primary)
                .unwrap_or(false);
            if is_primary {
                continue;
            }
            match idx.status {
                ChangeStatus::Added => {
                    if let Some(i) = &idx.target {
                        self.create_index(&table.name, i);
                    }
                }
                ChangeStatus::Modified => {
                    // Recreate so definition changes (columns, uniqueness) land.
                    self.drop_index(&idx.name);
                    if let Some(i) = &idx.target {
                        self.create_index(&table.name, i);
                    }
                }
                _ => {}
            }
        }
    }

    fn create_index(&mut self, table_name: &str, index: &Index) {
        let unique = if index.unique { "UNIQUE " } else { "" };
        let cols = index
            .columns
            .iter()
            .map(|c| self.ident(c))
            .collect::<Vec<_>>()
            .join(", ");
        let sql = format!(
            "CREATE {unique}INDEX IF NOT EXISTS {} ON {} ({cols});",
            self.ident(&index.name),
            self.qualified(table_name)
        );
        self.push(
            MigrationKind::CreateIndex,
            format!("{}.{}", self.schema, index.name),
            format!("create index {}", index.name),
            false,
            sql,
        );
    }

    fn drop_removed_indexes(&mut self, table: &TableDiff) {
        for idx in &table.indexes {
            if !matches!(idx.status, ChangeStatus::Removed) {
                continue;
            }
            let is_primary = idx.source.as_ref().map(|i| i.primary).unwrap_or(false);
            if is_primary {
                continue;
            }
            self.drop_index(&idx.name);
        }
    }

    fn drop_index(&mut self, name: &str) {
        // Indexes are schema-scoped in Postgres; qualify the drop.
        self.push(
            MigrationKind::DropIndex,
            format!("{}.{}", self.schema, name),
            format!("drop index {name}"),
            true,
            format!("DROP INDEX IF EXISTS {};", self.qualified(name)),
        );
    }

    fn add_foreign_keys(&mut self, table: &TableDiff, target: &Table) {
        for fk in &table.foreign_keys {
            match fk.status {
                ChangeStatus::Added | ChangeStatus::Modified => {
                    if let Some(f) = &fk.target {
                        self.add_foreign_key(&target.name, f);
                    }
                }
                _ => {}
            }
        }
    }

    fn add_foreign_key(&mut self, table_name: &str, fk: &ForeignKey) {
        let qualified = self.qualified(table_name);
        let name = self.ident(&fk.name);
        let cols = fk
            .columns
            .iter()
            .map(|c| self.ident(c))
            .collect::<Vec<_>>()
            .join(", ");
        let ref_table = self
            .dialect
            .quote_qualified(&fk.referenced_schema, &fk.referenced_table);
        let ref_cols = fk
            .referenced_columns
            .iter()
            .map(|c| self.ident(c))
            .collect::<Vec<_>>()
            .join(", ");
        // Drop-then-add makes re-running the migration idempotent (Postgres has
        // no ADD CONSTRAINT IF NOT EXISTS).
        let sql = format!(
            "ALTER TABLE {qualified} DROP CONSTRAINT IF EXISTS {name};\nALTER TABLE {qualified} ADD CONSTRAINT {name} FOREIGN KEY ({cols}) REFERENCES {ref_table} ({ref_cols});"
        );
        self.push(
            MigrationKind::AddForeignKey,
            format!("{}.{}", self.schema, fk.name),
            format!("add foreign key {}", fk.name),
            false,
            sql,
        );
    }

    fn drop_removed_foreign_keys(&mut self, table: &TableDiff) {
        let qualified = self.qualified(&table.name);
        for fk in &table.foreign_keys {
            if !matches!(fk.status, ChangeStatus::Removed) {
                continue;
            }
            self.push(
                MigrationKind::DropForeignKey,
                format!("{}.{}", self.schema, fk.name),
                format!("drop foreign key {}", fk.name),
                true,
                format!(
                    "ALTER TABLE {qualified} DROP CONSTRAINT IF EXISTS {};",
                    self.ident(&fk.name)
                ),
            );
        }
    }

    fn create_view(&mut self, view: &View, replace: bool) {
        let qualified = self.qualified(&view.name);
        let kind = if replace {
            MigrationKind::ReplaceView
        } else {
            MigrationKind::CreateView
        };
        let verb = if replace { "replace" } else { "create" };
        let sql = match &view.definition {
            Some(def) => format!(
                "CREATE OR REPLACE VIEW {qualified} AS\n{};",
                def.trim().trim_end_matches(';')
            ),
            None => format!("-- view definition unavailable for {qualified}; cannot generate DDL"),
        };
        self.push(
            kind,
            format!("{}.{}", self.schema, view.name),
            format!("{verb} view {}", view.name),
            false,
            sql,
        );
    }

    fn drop_view(&mut self, view: &View) {
        let qualified = self.qualified(&view.name);
        self.push(
            MigrationKind::DropView,
            format!("{}.{}", self.schema, view.name),
            format!("drop view {}", view.name),
            true,
            format!("DROP VIEW IF EXISTS {qualified};"),
        );
    }

    fn column_definition(&self, column: &Column) -> String {
        let mut def = format!("{} {}", self.ident(&column.name), column.data_type);
        if !column.nullable {
            def.push_str(" NOT NULL");
        }
        if let Some(default) = &column.default {
            def.push_str(" DEFAULT ");
            def.push_str(default);
        }
        def
    }
}

fn kind_slug(kind: MigrationKind) -> &'static str {
    match kind {
        MigrationKind::CreateTable => "create-table",
        MigrationKind::DropTable => "drop-table",
        MigrationKind::AddColumn => "add-column",
        MigrationKind::DropColumn => "drop-column",
        MigrationKind::AlterColumn => "alter-column",
        MigrationKind::AlterPrimaryKey => "alter-primary-key",
        MigrationKind::CreateIndex => "create-index",
        MigrationKind::DropIndex => "drop-index",
        MigrationKind::AddForeignKey => "add-foreign-key",
        MigrationKind::DropForeignKey => "drop-foreign-key",
        MigrationKind::CreateView => "create-view",
        MigrationKind::ReplaceView => "replace-view",
        MigrationKind::DropView => "drop-view",
    }
}
