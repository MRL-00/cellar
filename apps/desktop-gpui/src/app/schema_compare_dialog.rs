use std::{fs, sync::Arc};

use cellar_schema_diff::{SchemaSnapshot, SchemaSnapshotMeta};
use gpui::{div, prelude::*, px, AnyElement, Context, SharedString};
use gpui_component::Icon;

use super::CellarApp;
use cellar_desktop_gpui::{
    model::{SchemaCompareConfig, SchemaCompareSource},
    theme::{
        ACCENT, ACCENT_FG, BORDER, FG, FG_MUTED, INSERT, PANEL, PANEL_MUTED, PANEL_RAISED, PROD,
        WARN,
    },
};

#[derive(Clone)]
struct SourceOption {
    source: SchemaCompareSource,
    label: String,
    snapshot: bool,
}

pub(super) struct SchemaCompareDialog {
    options: Vec<SourceOption>,
    snapshots: Vec<SchemaSnapshotMeta>,
    source: Option<SchemaCompareSource>,
    target: Option<SchemaCompareSource>,
    busy: bool,
    error: Option<String>,
}

#[derive(Clone, Copy)]
enum Side {
    Source,
    Target,
}

impl CellarApp {
    pub(super) fn open_schema_compare_dialog(
        &mut self,
        preset: Option<SchemaCompareSource>,
        cx: &mut Context<Self>,
    ) {
        let snapshots = load_snapshots();
        let options = self.schema_source_options(&snapshots);
        let source = preset
            .as_ref()
            .and_then(|preset| {
                options
                    .iter()
                    .find(|option| same_source(&option.source, preset))
                    .map(|option| option.source.clone())
            })
            .or(preset)
            .or_else(|| options.first().map(|option| option.source.clone()));
        let target = options
            .iter()
            .find(|option| {
                source
                    .as_ref()
                    .is_none_or(|source| !same_source(&option.source, source))
            })
            .or_else(|| options.first())
            .map(|option| option.source.clone());
        self.schema_compare_dialog = Some(SchemaCompareDialog {
            options,
            snapshots,
            source,
            target,
            busy: false,
            error: None,
        });
        self.command_palette = None;
        self.command_palette_subscription = None;
        self.schema_menu = None;
        cx.notify();
    }

    fn schema_source_options(&self, snapshots: &[SchemaSnapshotMeta]) -> Vec<SourceOption> {
        let mut options = Vec::new();
        for connection in self.model.connections() {
            for database in self.model.databases(&connection.id) {
                for schema in &database.schemas {
                    options.push(SourceOption {
                        source: SchemaCompareSource::Live {
                            connection_id: connection.id.clone(),
                            database: database.name.clone(),
                            schema: schema.name.clone(),
                            label: Some(format!(
                                "{} · {} / {}",
                                connection.name, database.name, schema.name
                            )),
                        },
                        label: format!("{} · {} / {}", connection.name, database.name, schema.name),
                        snapshot: false,
                    });
                }
            }
        }
        for snapshot in snapshots {
            for schema in &snapshot.schemas {
                options.push(SourceOption {
                    source: SchemaCompareSource::Snapshot {
                        id: snapshot.id.clone(),
                        schema: schema.clone(),
                        label: Some(format!("{} / {schema}", snapshot.label)),
                    },
                    label: format!("{} / {schema}", snapshot.label),
                    snapshot: true,
                });
            }
        }
        options
    }

    fn refresh_schema_compare_dialog(&mut self) {
        let snapshots = load_snapshots();
        let options = self.schema_source_options(&snapshots);
        if let Some(dialog) = self.schema_compare_dialog.as_mut() {
            dialog.snapshots = snapshots;
            dialog.options = options;
            dialog.busy = false;
        }
    }

    fn select_schema_source(
        &mut self,
        side: Side,
        source: SchemaCompareSource,
        cx: &mut Context<Self>,
    ) {
        if let Some(dialog) = self.schema_compare_dialog.as_mut() {
            match side {
                Side::Source => dialog.source = Some(source),
                Side::Target => dialog.target = Some(source),
            }
            dialog.error = None;
        }
        cx.notify();
    }

    fn save_schema_snapshot(
        &mut self,
        connection_id: String,
        database: String,
        cx: &mut Context<Self>,
    ) {
        let Some(connection) = self
            .model
            .connections()
            .iter()
            .find(|connection| connection.id == connection_id)
            .cloned()
        else {
            return;
        };
        if let Some(dialog) = self.schema_compare_dialog.as_mut() {
            dialog.busy = true;
            dialog.error = None;
        }
        let registry = Arc::clone(&self.registry);
        let runtime = Arc::clone(&self.runtime);
        cx.spawn(async move |this, cx| {
            let result = runtime
                .spawn(async move {
                    let tree = registry
                        .database_for(&connection_id, &database)
                        .await
                        .map_err(|error| error.to_string())?;
                    let created_at_ms = chrono::Utc::now().timestamp_millis();
                    let id = format!(
                        "{}-{created_at_ms}",
                        snapshot_slug(&format!("{}-{database}", connection.name))
                    );
                    let meta = SchemaSnapshotMeta {
                        id: id.clone(),
                        label: format!("{} · {database}", connection.name),
                        engine: connection.engine.as_str().to_owned(),
                        connection_id,
                        connection_name: connection.name,
                        database,
                        schemas: tree
                            .schemas
                            .iter()
                            .map(|schema| schema.name.clone())
                            .collect(),
                        table_count: tree
                            .schemas
                            .iter()
                            .map(|schema| schema.tables.len())
                            .sum::<usize>() as u32,
                        created_at_ms,
                    };
                    let snapshot = SchemaSnapshot {
                        meta: meta.clone(),
                        database: tree,
                    };
                    let directory = snapshot_directory()
                        .ok_or_else(|| "could not resolve Cellar data directory".to_owned())?;
                    tokio::fs::create_dir_all(&directory)
                        .await
                        .map_err(|error| error.to_string())?;
                    tokio::fs::write(
                        directory.join(format!("{id}.json")),
                        serde_json::to_vec_pretty(&snapshot).map_err(|error| error.to_string())?,
                    )
                    .await
                    .map_err(|error| error.to_string())?;
                    Ok::<_, String>(())
                })
                .await
                .map_err(|error| format!("snapshot task failed: {error}"))
                .and_then(|result| result);
            this.update(cx, |this, cx| {
                this.refresh_schema_compare_dialog();
                if let (Some(dialog), Err(error)) = (this.schema_compare_dialog.as_mut(), result) {
                    dialog.error = Some(error);
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
        cx.notify();
    }

    fn delete_schema_snapshot(&mut self, id: String, cx: &mut Context<Self>) {
        if !valid_snapshot_id(&id) {
            return;
        }
        if let Some(dialog) = self.schema_compare_dialog.as_mut() {
            dialog.busy = true;
        }
        let runtime = Arc::clone(&self.runtime);
        cx.spawn(async move |this, cx| {
            let result = runtime
                .spawn(async move {
                    let path = snapshot_directory()
                        .ok_or_else(|| "could not resolve Cellar data directory".to_owned())?
                        .join(format!("{id}.json"));
                    match tokio::fs::remove_file(path).await {
                        Ok(()) => Ok(()),
                        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
                        Err(error) => Err(error.to_string()),
                    }
                })
                .await
                .map_err(|error| format!("snapshot task failed: {error}"))
                .and_then(|result| result);
            this.update(cx, |this, cx| {
                this.refresh_schema_compare_dialog();
                if let (Some(dialog), Err(error)) = (this.schema_compare_dialog.as_mut(), result) {
                    dialog.error = Some(error);
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    pub(super) fn schema_compare_dialog_overlay(&self, cx: &mut Context<Self>) -> AnyElement {
        let dialog = self
            .schema_compare_dialog
            .as_ref()
            .expect("schema compare dialog requires state");
        let source = dialog.source.clone();
        let target = dialog.target.clone();
        let can_compare = source.is_some() && target.is_some() && !dialog.busy;
        div()
            .id("schema-compare-dialog-backdrop")
            .absolute()
            .inset_0()
            .bg(cellar_desktop_gpui::theme::overlay())
            .flex()
            .items_start()
            .justify_center()
            .pt(gpui::relative(0.08))
            .on_click(cx.listener(|this, _, _, cx| {
                this.schema_compare_dialog = None;
                cx.notify();
            }))
            .child(
                div()
                    .id("schema-compare-dialog")
                    .w(px(760.))
                    .max_h(px(680.))
                    .flex()
                    .flex_col()
                    .rounded(px(7.))
                    .border_1()
                    .border_color(BORDER)
                    .bg(PANEL)
                    .shadow_lg()
                    .on_click(|_, _, cx| cx.stop_propagation())
                    .child(
                        div()
                            .h(px(38.))
                            .flex_shrink_0()
                            .flex()
                            .items_center()
                            .gap_2()
                            .px_3()
                            .border_b_1()
                            .border_color(BORDER)
                            .child(Icon::empty().path("icons/diff.svg").size(px(14.)).text_color(ACCENT))
                            .child(div().font_weight(gpui::FontWeight::SEMIBOLD).child("Compare schemas"))
                            .child(div().flex_1())
                            .child(
                                div()
                                    .id("close-schema-compare-dialog")
                                    .tab_index(0)
                                    .cursor_pointer()
                                    .size(px(22.))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .rounded(px(4.))
                                    .hover(|style| style.bg(PANEL_RAISED))
                                    .child(Icon::empty().path("icons/close.svg").size(px(11.)))
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.schema_compare_dialog = None;
                                        cx.notify();
                                    })),
                            ),
                    )
                    .child(
                        div()
                            .id("schema-compare-dialog-scroll")
                            .flex_1()
                            .min_h_0()
                            .overflow_y_scroll()
                            .p(px(14.))
                            .flex()
                            .flex_col()
                            .gap_3()
                            .child(
                                div()
                                    .flex()
                                    .gap_3()
                                    .child(source_panel(Side::Source, "Source", "changed to match target", &dialog.options, dialog.source.as_ref(), cx))
                                    .child(source_panel(Side::Target, "Target", "the desired end state", &dialog.options, dialog.target.as_ref(), cx)),
                            )
                            .child(self.snapshot_manager(dialog, cx))
                            .when_some(dialog.error.clone(), |element, error| {
                                element.child(div().text_color(WARN).child(error))
                            }),
                    )
                    .child(
                        div()
                            .min_h(px(44.))
                            .flex_shrink_0()
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap_3()
                            .px_3()
                            .py_2()
                            .border_t_1()
                            .border_color(BORDER)
                            .bg(PANEL_MUTED)
                            .child(div().text_size(px(11.5)).text_color(FG_MUTED).child("The migration transforms the source into the target. Generated SQL is reviewed before it runs."))
                            .child(
                                div()
                                    .id("confirm-schema-compare")
                                    .h(px(26.))
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .rounded(px(4.))
                                    .bg(ACCENT)
                                    .px_3()
                                    .text_color(ACCENT_FG)
                                    .opacity(if can_compare { 1. } else { 0.45 })
                                    .child(Icon::empty().path("icons/diff.svg").size(px(11.)))
                                    .child(if dialog.busy { "Working…" } else { "Compare" })
                                    .when(can_compare, |element| {
                                        element
                                            .cursor_pointer()
                                            .hover(|style| {
                                                style.bg(cellar_desktop_gpui::theme::hover_bright(
                                                    ACCENT.rgba(),
                                                ))
                                            })
                                            .on_click(cx.listener(move |this, _, window, cx| {
                                                let (Some(source), Some(target)) = (source.clone(), target.clone()) else { return };
                                                this.schema_compare_dialog = None;
                                                this.open_schema_compare(SchemaCompareConfig { source, target }, window, cx);
                                            }))
                                    }),
                            ),
                    ),
            )
            .into_any_element()
    }

    fn snapshot_manager(&self, dialog: &SchemaCompareDialog, cx: &mut Context<Self>) -> AnyElement {
        let mut databases = Vec::new();
        for connection in self.model.connections() {
            for database in self.model.databases(&connection.id) {
                databases.push((
                    connection.id.clone(),
                    database.name.clone(),
                    connection.name.clone(),
                ));
            }
        }
        div()
            .flex()
            .flex_col()
            .gap_2()
            .rounded(px(6.))
            .border_1()
            .border_color(BORDER)
            .bg(PANEL_RAISED)
            .p(px(10.))
            .child(
                div()
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .child("Snapshots"),
            )
            .child(
                div()
                    .text_size(px(11.))
                    .text_color(FG_MUTED)
                    .child("Save a current database tree for offline comparison."),
            )
            .children(
                databases
                    .into_iter()
                    .map(|(connection_id, database, name)| {
                        let save_id = connection_id.clone();
                        let save_database = database.clone();
                        div()
                            .h(px(26.))
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(
                                div()
                                    .flex_1()
                                    .truncate()
                                    .child(format!("{name} · {database}")),
                            )
                            .child(
                                mini_button("Save snapshot")
                                    .opacity(if dialog.busy { 0.45 } else { 1. })
                                    .when(!dialog.busy, |element| {
                                        element.on_click(cx.listener(move |this, _, _, cx| {
                                            this.save_schema_snapshot(
                                                save_id.clone(),
                                                save_database.clone(),
                                                cx,
                                            )
                                        }))
                                    }),
                            )
                    }),
            )
            .children(dialog.snapshots.iter().map(|snapshot| {
                let id = snapshot.id.clone();
                div()
                    .h(px(25.))
                    .flex()
                    .items_center()
                    .gap_2()
                    .border_t_1()
                    .border_color(BORDER)
                    .child(div().flex_1().truncate().child(snapshot.label.clone()))
                    .child(
                        div()
                            .text_size(px(11.))
                            .text_color(FG_MUTED)
                            .child(format!("{} tables", snapshot.table_count)),
                    )
                    .child(
                        mini_button("Delete")
                            .text_color(PROD)
                            .opacity(if dialog.busy { 0.45 } else { 1. })
                            .when(!dialog.busy, |element| {
                                element.on_click(cx.listener(move |this, _, _, cx| {
                                    this.delete_schema_snapshot(id.clone(), cx)
                                }))
                            }),
                    )
            }))
            .into_any_element()
    }
}

fn source_panel(
    side: Side,
    heading: &'static str,
    subtitle: &'static str,
    options: &[SourceOption],
    selected: Option<&SchemaCompareSource>,
    cx: &mut Context<CellarApp>,
) -> AnyElement {
    div()
        .flex_1()
        .min_w_0()
        .flex()
        .flex_col()
        .rounded(px(6.))
        .border_1()
        .border_color(BORDER)
        .bg(PANEL_RAISED)
        .child(
            div()
                .h(px(32.))
                .flex()
                .items_center()
                .justify_between()
                .px_2()
                .child(div().font_weight(gpui::FontWeight::SEMIBOLD).child(heading))
                .child(
                    div()
                        .text_size(px(11.))
                        .text_color(FG_MUTED)
                        .child(subtitle),
                ),
        )
        .child(
            div()
                .id(SharedString::from(format!("schema-source-list:{heading}")))
                .h(px(190.))
                .overflow_y_scroll()
                .border_t_1()
                .border_color(BORDER)
                .when(options.is_empty(), |element| {
                    element.child(
                        div()
                            .p_3()
                            .text_color(FG_MUTED)
                            .child("Connect a database or save a snapshot first."),
                    )
                })
                .children(options.iter().cloned().map(|option| {
                    let active =
                        selected.is_some_and(|selected| same_source(selected, &option.source));
                    let pick = option.source.clone();
                    div()
                        .id(SharedString::from(format!(
                            "schema-source:{}:{}",
                            heading, option.label
                        )))
                        .cursor_pointer()
                        .h(px(27.))
                        .flex()
                        .items_center()
                        .gap_2()
                        .px_2()
                        .bg(if active { PANEL } else { PANEL_RAISED })
                        .hover(|style| style.bg(PANEL))
                        .child(div().w(px(12.)).text_color(ACCENT).child(if active {
                            "✓"
                        } else {
                            ""
                        }))
                        .child(
                            div()
                                .rounded(px(3.))
                                .bg(PANEL)
                                .px_1()
                                .text_size(px(9.5))
                                .text_color(if option.snapshot { WARN } else { INSERT })
                                .child(if option.snapshot { "SNAP" } else { "LIVE" }),
                        )
                        .child(div().min_w_0().flex_1().truncate().child(option.label))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.select_schema_source(side, pick.clone(), cx)
                        }))
                })),
        )
        .into_any_element()
}

fn load_snapshots() -> Vec<SchemaSnapshotMeta> {
    let Some(directory) = snapshot_directory() else {
        return Vec::new();
    };
    let mut snapshots = fs::read_dir(directory)
        .ok()
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .filter_map(|entry| fs::read(entry.path()).ok())
        .filter_map(|raw| serde_json::from_slice::<SchemaSnapshot>(&raw).ok())
        .map(|snapshot| snapshot.meta)
        .collect::<Vec<_>>();
    snapshots.sort_by_key(|snapshot| std::cmp::Reverse(snapshot.created_at_ms));
    snapshots
}

fn snapshot_directory() -> Option<std::path::PathBuf> {
    Some(cellar_runtime::cellar_dir()?.join("snapshots"))
}

fn valid_snapshot_id(id: &str) -> bool {
    !id.is_empty()
        && id
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_'))
}

fn same_source(left: &SchemaCompareSource, right: &SchemaCompareSource) -> bool {
    match (left, right) {
        (
            SchemaCompareSource::Live {
                connection_id: left_connection,
                database: left_database,
                schema: left_schema,
                ..
            },
            SchemaCompareSource::Live {
                connection_id: right_connection,
                database: right_database,
                schema: right_schema,
                ..
            },
        ) => {
            left_connection == right_connection
                && left_database == right_database
                && left_schema == right_schema
        }
        (
            SchemaCompareSource::Snapshot {
                id: left_id,
                schema: left_schema,
                ..
            },
            SchemaCompareSource::Snapshot {
                id: right_id,
                schema: right_schema,
                ..
            },
        ) => left_id == right_id && left_schema == right_schema,
        _ => false,
    }
}

fn snapshot_slug(value: &str) -> String {
    let slug = value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character
            } else {
                '-'
            }
        })
        .collect::<String>();
    let slug = slug.trim_matches('-').to_lowercase();
    if slug.is_empty() {
        "snapshot".into()
    } else {
        slug
    }
}

fn mini_button(label: &'static str) -> gpui::Stateful<gpui::Div> {
    div()
        .id(SharedString::from(label))
        .tab_index(0)
        .cursor_pointer()
        .h(px(22.))
        .flex()
        .items_center()
        .rounded(px(4.))
        .border_1()
        .border_color(BORDER)
        .bg(PANEL)
        .px_2()
        .text_size(px(11.))
        .text_color(FG)
        .hover(|style| style.bg(PANEL_RAISED))
        .child(label)
}

#[cfg(test)]
mod tests {
    use super::{snapshot_slug, valid_snapshot_id};

    #[test]
    fn snapshot_names_are_safe_local_paths() {
        assert_eq!(snapshot_slug("Prod / Main"), "prod---main");
        assert!(valid_snapshot_id("prod-main-123"));
        assert!(!valid_snapshot_id("../prod"));
    }
}
