//! Structure folders under an expanded table in the schema tree: columns,
//! keys, indexes, and defaults. They render from the introspection cache in
//! `AppModel`, so opening a table costs no extra query.

use std::collections::HashSet;

use cellar_core::schema::{Column, ForeignKey, Index, Table};
use gpui::{div, prelude::*, AnyElement, Context, KeyDownEvent, SharedString};

use super::schema_tree::{
    detail, empty_twisty, folder_icon, icon_slot, info_row, label, meta, pill, toggle_key,
    tree_row, twisty, type_icon,
};
use super::CellarApp;
use cellar_desktop_gpui::{
    grid::column_type_icon,
    model::{SchemaNode, TableTarget},
    theme::{ACCENT, FG_MUTED, WARN},
};

/// Indent of a structure folder under a table row.
const FOLDER_PADDING: f32 = 78.;
/// Indent of a column, key, index, or default row.
const ITEM_PADDING: f32 = 102.;

impl CellarApp {
    /// Structure folders for an expanded table. Folders with nothing to show
    /// are left out so a wide schema stays scannable.
    pub(super) fn table_structure(
        &self,
        target: &TableTarget,
        table: &Table,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let foreign_columns = foreign_key_columns(table);
        let keys = key_rows(table);
        let defaults = default_rows(table);
        let mut structure = div();
        if !table.columns.is_empty() {
            structure = structure.child(self.structure_folder(
                target,
                "columns",
                table.columns.len(),
                cx,
                || {
                    table
                        .columns
                        .iter()
                        .map(|column| column_row(column, &foreign_columns))
                        .collect()
                },
            ));
        }
        if !keys.is_empty() {
            let count = keys.len();
            structure =
                structure.child(self.structure_folder(target, "keys", count, cx, move || keys));
        }
        if !table.indexes.is_empty() {
            structure = structure.child(self.structure_folder(
                target,
                "indexes",
                table.indexes.len(),
                cx,
                || table.indexes.iter().map(index_row).collect(),
            ));
        }
        if !defaults.is_empty() {
            let count = defaults.len();
            structure =
                structure
                    .child(self.structure_folder(target, "defaults", count, cx, move || defaults));
        }
        structure.into_any_element()
    }

    /// One collapsible structure folder. Children are built only while the
    /// folder is open.
    fn structure_folder(
        &self,
        target: &TableTarget,
        kind: &'static str,
        count: usize,
        cx: &mut Context<Self>,
        children: impl FnOnce() -> Vec<AnyElement>,
    ) -> AnyElement {
        let node = SchemaNode::relation_group(target, kind);
        let expanded = self.model.node_expanded(&node);
        let toggle = node.clone();
        let keyboard_toggle = node;
        div()
            .child(
                tree_row(FOLDER_PADDING)
                    .id(SharedString::from(format!(
                        "{kind}:{}:{}:{}:{}",
                        target.connection_id, target.database, target.schema, target.table
                    )))
                    .child(twisty(expanded))
                    .child(folder_icon(expanded))
                    .child(label(kind))
                    .child(meta(count.to_string()))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.model.toggle_node(toggle.clone());
                        cx.notify();
                    }))
                    .on_key_down(cx.listener(move |this, event: &KeyDownEvent, _, cx| {
                        if toggle_key(event, expanded) {
                            this.model.toggle_node(keyboard_toggle.clone());
                            cx.notify();
                            cx.stop_propagation();
                        }
                    })),
            )
            .when(expanded, |element| element.children(children()))
            .into_any_element()
    }
}

/// A column of a table: key or type glyph, name, engine-native type.
fn column_row(column: &Column, foreign_columns: &HashSet<&str>) -> AnyElement {
    let (path, color) = column_type_icon(
        &column.data_type,
        column.is_primary_key,
        foreign_columns.contains(column.name.as_str()),
    );
    info_row(ITEM_PADDING)
        .child(empty_twisty())
        .child(type_icon(path, color, 11.))
        .child(label(column.name.clone()))
        .child(detail(column.data_type.to_lowercase()))
        .into_any_element()
}

/// Primary key first, then foreign keys — the order DDL reads them in. An
/// empty table has no keys row at all.
fn key_rows(table: &Table) -> Vec<AnyElement> {
    primary_key_row(table)
        .into_iter()
        .chain(
            table
                .foreign_keys
                .iter()
                .map(|foreign_key| foreign_key_row(&table.schema, foreign_key)),
        )
        .collect()
}

fn primary_key_row(table: &Table) -> Option<AnyElement> {
    let index = table.indexes.iter().find(|index| index.primary);
    let columns = index
        .map(|index| index.columns.clone())
        .filter(|columns| !columns.is_empty())
        .unwrap_or_else(|| table.primary_key.clone());
    if columns.is_empty() {
        return None;
    }
    let name = index.map_or_else(|| "primary key".into(), |index| index.name.clone());
    Some(
        info_row(ITEM_PADDING)
            .child(empty_twisty())
            .child(type_icon("icons/type-key.svg", WARN, 11.))
            .child(label(name))
            .child(detail(column_list(&columns)))
            .into_any_element(),
    )
}

fn foreign_key_row(schema: &str, foreign_key: &ForeignKey) -> AnyElement {
    let referenced =
        if foreign_key.referenced_schema.is_empty() || foreign_key.referenced_schema == schema {
            foreign_key.referenced_table.clone()
        } else {
            format!(
                "{}.{}",
                foreign_key.referenced_schema, foreign_key.referenced_table
            )
        };
    info_row(ITEM_PADDING)
        .child(empty_twisty())
        .child(type_icon("icons/type-link.svg", ACCENT, 11.))
        .child(label(foreign_key.name.clone()))
        .child(detail(format!(
            "{} → {}{}",
            column_list(&foreign_key.columns),
            referenced,
            column_list(&foreign_key.referenced_columns)
        )))
        .into_any_element()
}

fn index_row(index: &Index) -> AnyElement {
    let trait_badge = if index.primary {
        Some("primary")
    } else if index.unique {
        Some("unique")
    } else {
        None
    };
    info_row(ITEM_PADDING)
        .child(empty_twisty())
        .child(type_icon("icons/index.svg", FG_MUTED, 11.))
        .child(label(index.name.clone()))
        .when_some(trait_badge, |element, badge| element.child(pill(badge)))
        .child(detail(column_list(&index.columns)))
        .into_any_element()
}

/// Columns carrying a server-side default, shown as `name = expression`.
fn default_rows(table: &Table) -> Vec<AnyElement> {
    table
        .columns
        .iter()
        .filter_map(|column| {
            let default = column.default.as_deref()?;
            Some(
                info_row(ITEM_PADDING)
                    .child(empty_twisty())
                    .child(icon_slot())
                    .child(label(column.name.clone()))
                    .child(detail(format!("= {default}")))
                    .into_any_element(),
            )
        })
        .collect()
}

fn foreign_key_columns(table: &Table) -> HashSet<&str> {
    table
        .foreign_keys
        .iter()
        .flat_map(|foreign_key| foreign_key.columns.iter().map(String::as_str))
        .collect()
}

fn column_list(columns: &[String]) -> String {
    format!("({})", columns.join(", "))
}
