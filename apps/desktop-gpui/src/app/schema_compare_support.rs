use cellar_core::{driver::Engine, schema::Schema};
use cellar_schema_diff::{compare, ChangeStatus, Dialect, SchemaComparison, SchemaSnapshot};
use gpui::{div, prelude::*, px, AnyElement, Context, SharedString};
use gpui_component::Icon;

use super::{schema_compare::SchemaCompareWorkspace, CellarApp};
use cellar_desktop_gpui::{
    model::{SchemaCompareConfig, SchemaCompareSource},
    theme::{ACCENT, BORDER, FG, FG_MUTED, FG_SECONDARY, INSERT, INSET, PANEL_RAISED, PROD, WARN},
};

pub(super) async fn compare_config(
    registry: &cellar_runtime::ConnectionRegistry,
    config: &SchemaCompareConfig,
) -> Result<SchemaComparison, String> {
    let (source, source_label, dialect) = resolve_source(registry, &config.source).await?;
    let (target, target_label, _) = resolve_source(registry, &config.target).await?;
    let namespace = source.name.clone();
    Ok(compare(
        &source,
        &target,
        source_label,
        target_label,
        &namespace,
        dialect,
    ))
}

async fn resolve_source(
    registry: &cellar_runtime::ConnectionRegistry,
    source: &SchemaCompareSource,
) -> Result<(Schema, String, Dialect), String> {
    match source {
        SchemaCompareSource::Live {
            connection_id,
            database,
            schema,
            label,
        } => {
            let resolved = registry
                .schema_for(connection_id, database, schema)
                .await
                .map_err(|error| error.to_string())?;
            let dialect = dialect_for(registry.engine_for(connection_id).await);
            Ok((
                resolved,
                label
                    .clone()
                    .unwrap_or_else(|| format!("{database} / {schema}")),
                dialect,
            ))
        }
        SchemaCompareSource::Snapshot { id, schema, label } => {
            validate_snapshot_id(id)?;
            let path = cellar_runtime::cellar_dir()
                .ok_or_else(|| "could not resolve Cellar data directory".to_owned())?
                .join("snapshots")
                .join(format!("{id}.json"));
            let raw = tokio::fs::read_to_string(path)
                .await
                .map_err(|error| error.to_string())?;
            let snapshot: SchemaSnapshot =
                serde_json::from_str(&raw).map_err(|error| error.to_string())?;
            let resolved = snapshot
                .database
                .schemas
                .into_iter()
                .find(|candidate| candidate.name == *schema)
                .ok_or_else(|| format!("snapshot {id} does not contain schema {schema}"))?;
            Ok((
                resolved,
                label
                    .clone()
                    .unwrap_or_else(|| format!("{} / {schema}", snapshot.meta.label)),
                dialect_for_str(&snapshot.meta.engine),
            ))
        }
    }
}

fn validate_snapshot_id(id: &str) -> Result<(), String> {
    if !id.is_empty()
        && id
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_'))
    {
        Ok(())
    } else {
        Err(format!("invalid snapshot id: {id}"))
    }
}

fn dialect_for(engine: Option<Engine>) -> Dialect {
    match engine.map(|engine| engine.family()) {
        Some(Engine::MySql) => Dialect::MySql,
        Some(Engine::Sqlite) => Dialect::Sqlite,
        Some(Engine::Mssql) => Dialect::Mssql,
        _ => Dialect::Postgres,
    }
}

fn dialect_for_str(engine: &str) -> Dialect {
    match engine {
        "mysql" => Dialect::MySql,
        "sqlite" => Dialect::Sqlite,
        "mssql" | "azure" => Dialect::Mssql,
        _ => Dialect::Postgres,
    }
}

pub(super) fn statement_list(
    tab_id: u64,
    comparison: &SchemaComparison,
    workspace: &SchemaCompareWorkspace,
    cx: &mut Context<CellarApp>,
) -> AnyElement {
    if comparison.statements.is_empty() {
        return div()
            .h(px(54.))
            .flex_shrink_0()
            .flex()
            .items_center()
            .px_3()
            .text_color(FG_MUTED)
            .child("No DDL to generate — the schemas already match.")
            .into_any_element();
    }
    div()
        .id(SharedString::from(format!("schema-statements:{tab_id}")))
        .max_h(px(120.))
        .min_h(px(54.))
        .flex_shrink_0()
        .overflow_y_scroll()
        .py_1()
        .children(comparison.statements.iter().map(|statement| {
            let id = statement.id.clone();
            let checked = workspace.selected.contains(&statement.id);
            div()
                .id(SharedString::from(format!(
                    "schema-statement:{}",
                    statement.id
                )))
                .cursor_pointer()
                .h(px(23.))
                .flex()
                .items_center()
                .gap_2()
                .px_3()
                .hover(|style| style.bg(PANEL_RAISED))
                .child(
                    div()
                        .w(px(12.))
                        .text_color(ACCENT)
                        .child(if checked { "✓" } else { "" }),
                )
                .child(
                    div()
                        .rounded(px(3.))
                        .bg(PANEL_RAISED)
                        .px_1()
                        .font_family(cellar_desktop_gpui::theme::mono_font())
                        .text_size(px(10.))
                        .text_color(if statement.destructive { PROD } else { INSERT })
                        .child(format!("{:?}", statement.kind).to_lowercase()),
                )
                .child(
                    div()
                        .min_w_0()
                        .flex_1()
                        .truncate()
                        .font_family(cellar_desktop_gpui::theme::mono_font())
                        .child(statement.object.clone()),
                )
                .child(
                    div()
                        .flex_shrink_0()
                        .text_size(px(11.))
                        .text_color(FG_MUTED)
                        .child(statement.description.clone()),
                )
                .on_click(cx.listener(move |this, _, window, cx| {
                    this.toggle_schema_statement(tab_id, id.clone(), window, cx)
                }))
        }))
        .into_any_element()
}

pub(super) fn table_row(
    tab_id: u64,
    table: &cellar_schema_diff::TableDiff,
    workspace: &SchemaCompareWorkspace,
    cx: &mut Context<CellarApp>,
) -> AnyElement {
    let open = workspace.expanded.contains(&table.name);
    let name = table.name.clone();
    let columns = if open {
        table
            .columns
            .iter()
            .filter(|column| table.status != ChangeStatus::Modified || column.status.is_change())
            .cloned()
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };
    div()
        .border_b_1()
        .border_color(BORDER)
        .child(
            div()
                .id(SharedString::from(format!("schema-table:{}", table.name)))
                .cursor_pointer()
                .h(px(24.))
                .flex()
                .items_center()
                .gap_2()
                .px_2()
                .hover(|style| style.bg(PANEL_RAISED))
                .child(
                    Icon::empty()
                        .path(if open {
                            "icons/chevron-down.svg"
                        } else {
                            "icons/chevron-right.svg"
                        })
                        .size(px(10.))
                        .text_color(FG_MUTED),
                )
                .child(status_badge(table.status))
                .child(
                    Icon::empty()
                        .path("icons/table.svg")
                        .size(px(11.))
                        .text_color(FG_MUTED),
                )
                .child(
                    div()
                        .font_family(cellar_desktop_gpui::theme::mono_font())
                        .child(table.name.clone()),
                )
                .when(table.status == ChangeStatus::Modified, |element| {
                    let count = table
                        .columns
                        .iter()
                        .filter(|column| column.status.is_change())
                        .count();
                    element.child(
                        div()
                            .text_size(px(11.))
                            .text_color(FG_MUTED)
                            .child(format!("{count} columns")),
                    )
                })
                .on_click(cx.listener(move |this, _, _, cx| {
                    if let Some(workspace) = this.schema_compares.get_mut(&tab_id) {
                        if !workspace.expanded.remove(&name) {
                            workspace.expanded.insert(name.clone());
                        }
                    }
                    cx.notify();
                })),
        )
        .children(columns.into_iter().map(|column| {
            let source = column
                .source
                .as_ref()
                .map(column_text)
                .unwrap_or_else(|| "—".into());
            let target = column
                .target
                .as_ref()
                .map(column_text)
                .unwrap_or_else(|| "—".into());
            div()
                .h(px(21.))
                .grid()
                .grid_cols(2)
                .gap_2()
                .pl(px(28.))
                .pr_2()
                .font_family(cellar_desktop_gpui::theme::mono_font())
                .text_size(px(11.5))
                .child(
                    div()
                        .truncate()
                        .text_color(
                            if column.status == ChangeStatus::Removed
                                || column.status == ChangeStatus::Modified
                            {
                                status_color(column.status)
                            } else {
                                FG_MUTED.rgba()
                            },
                        )
                        .child(source),
                )
                .child(
                    div()
                        .truncate()
                        .text_color(
                            if column.status == ChangeStatus::Added
                                || column.status == ChangeStatus::Modified
                            {
                                status_color(column.status)
                            } else {
                                FG_MUTED.rgba()
                            },
                        )
                        .child(if column.changes.is_empty() {
                            target
                        } else {
                            format!("{target}  {}", column.changes.join(", "))
                        }),
                )
        }))
        .into_any_element()
}

fn column_text(column: &cellar_core::schema::Column) -> String {
    format!(
        "{}  {}{}",
        column.name,
        column.data_type,
        if column.nullable { "" } else { "  NOT NULL" }
    )
}

pub(super) fn status_badge(status: ChangeStatus) -> impl IntoElement {
    div()
        .w(px(34.))
        .h(px(14.))
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(3.))
        .bg(PANEL_RAISED)
        .font_family(cellar_desktop_gpui::theme::mono_font())
        .text_size(px(9.5))
        .text_color(status_color(status))
        .child(match status {
            ChangeStatus::Added => "ADD",
            ChangeStatus::Removed => "DROP",
            ChangeStatus::Modified => "CHG",
            ChangeStatus::Unchanged => "—",
        })
}

fn status_color(status: ChangeStatus) -> gpui::Rgba {
    match status {
        ChangeStatus::Added => INSERT.rgba(),
        ChangeStatus::Removed => PROD.rgba(),
        ChangeStatus::Modified => WARN.rgba(),
        ChangeStatus::Unchanged => FG_MUTED.rgba(),
    }
}

pub(super) fn summary_text(n: u32, label: &'static str, color: gpui::Rgba) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .gap_1()
        .text_size(px(12.))
        .child(
            div()
                .font_family(cellar_desktop_gpui::theme::mono_font())
                .text_color(color)
                .child(n.to_string()),
        )
        .child(div().text_color(FG_MUTED).child(label))
}

pub(super) fn button(id: &'static str, label: &'static str) -> gpui::Stateful<gpui::Div> {
    div()
        .id(id)
        .tab_index(0)
        .cursor_pointer()
        .h(px(24.))
        .flex()
        .items_center()
        .rounded(px(4.))
        .border_1()
        .border_color(BORDER)
        .bg(PANEL_RAISED)
        .px_2()
        .text_size(px(11.5))
        .text_color(FG_SECONDARY)
        .hover(|style| style.text_color(FG))
        .child(label)
}

pub(super) fn text_action(id: &'static str, label: &'static str) -> gpui::Stateful<gpui::Div> {
    div()
        .id(id)
        .tab_index(0)
        .cursor_pointer()
        .rounded(px(3.))
        .px_1()
        .text_color(FG_MUTED)
        .hover(|style| style.bg(PANEL_RAISED).text_color(FG))
        .child(label)
}

pub(super) fn centered(message: &'static str, warn: bool) -> AnyElement {
    div()
        .flex_1()
        .flex()
        .items_center()
        .justify_center()
        .bg(INSET)
        .text_color(if warn { PROD } else { FG_MUTED })
        .child(message)
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::{dialect_for_str, validate_snapshot_id};
    use cellar_schema_diff::Dialect;

    #[test]
    fn snapshot_paths_cannot_escape_the_cellar_directory() {
        assert!(validate_snapshot_id("prod-123").is_ok());
        assert!(validate_snapshot_id("../prod").is_err());
        assert_eq!(dialect_for_str("mysql"), Dialect::MySql);
    }
}
