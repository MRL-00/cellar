use std::path::PathBuf;

use cellar_core::{driver::Engine, schema::Table};
use cellar_runtime::csv_import::{
    build_import_request, default_config, import_counts, parse_csv, validate_import, ImportConfig,
    ParsedCsv,
};
use gpui::{div, prelude::*, px, AnyElement, AppContext, Context, PathPromptOptions, SharedString};
use gpui_component::Icon;

use super::CellarApp;
use cellar_desktop_gpui::{
    model::{TabKind, TableTarget},
    theme::{ACCENT, BORDER, FG_MUTED, PANEL, PANEL_MUTED, PANEL_RAISED, PROD, WARN},
};

const MAX_IMPORT_BYTES: u64 = 64 * 1024 * 1024;
const MAX_IMPORT_ROWS: usize = 100_000;

pub(super) struct DataImport {
    tab_id: u64,
    target: TableTarget,
    table: Table,
    path: Option<PathBuf>,
    csv: Option<ParsedCsv>,
    config: Option<ImportConfig>,
    loading: bool,
    error: Option<String>,
}

impl DataImport {
    pub(super) fn tab_id(&self) -> u64 {
        self.tab_id
    }
}

impl CellarApp {
    pub(super) fn open_csv_import(&mut self, tab_id: u64, cx: &mut Context<Self>) {
        let Some((target, table)) = self.model.tabs().iter().find_map(|tab| {
            if tab.id != tab_id {
                return None;
            }
            let TabKind::Table { target, .. } = &tab.kind else {
                return None;
            };
            self.model
                .table(target)
                .cloned()
                .map(|table| (target.clone(), table))
        }) else {
            return;
        };
        let supported = self
            .model
            .connections()
            .iter()
            .find(|connection| connection.id == target.connection_id)
            .is_some_and(|connection| {
                matches!(connection.engine.family(), Engine::Postgres | Engine::Mssql)
            });
        self.data_import = Some(DataImport {
            tab_id,
            target,
            table,
            path: None,
            csv: None,
            config: None,
            loading: supported,
            error: (!supported)
                .then_some("This engine does not support transactional CSV import".into()),
        });
        cx.notify();
        if !supported {
            return;
        }

        let receiver = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some("Import CSV".into()),
        });
        cx.spawn(async move |this, cx| {
            let path = match receiver.await {
                Ok(Ok(Some(paths))) => paths.into_iter().next(),
                Ok(Ok(None)) => None,
                Ok(Err(error)) => {
                    set_import_error(&this, cx, tab_id, error.to_string());
                    return;
                }
                Err(error) => {
                    set_import_error(&this, cx, tab_id, error.to_string());
                    return;
                }
            };
            let Some(path) = path else {
                this.update(cx, |this, cx| {
                    if this.data_import.as_ref().map(DataImport::tab_id) == Some(tab_id) {
                        this.data_import = None;
                        cx.notify();
                    }
                })
                .ok();
                return;
            };
            let read_path = path.clone();
            let task = cx.background_spawn(async move { read_csv(&read_path) });
            let parsed = task.await;
            this.update(cx, |this, cx| {
                let Some(import) = this
                    .data_import
                    .as_mut()
                    .filter(|import| import.tab_id == tab_id)
                else {
                    return;
                };
                import.loading = false;
                match parsed {
                    Ok(csv) => {
                        import.config = Some(default_config(&csv, &import.table));
                        import.csv = Some(csv);
                        import.path = Some(path);
                    }
                    Err(error) => import.error = Some(error),
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn cycle_import_mode(&mut self, cx: &mut Context<Self>) {
        if let Some(config) = self
            .data_import
            .as_mut()
            .and_then(|import| import.config.as_mut())
        {
            config.mode = config.mode.next();
            cx.notify();
        }
    }

    fn cycle_import_mapping(&mut self, column: &str, cx: &mut Context<Self>) {
        let Some(import) = &mut self.data_import else {
            return;
        };
        let (Some(csv), Some(config)) = (&import.csv, &mut import.config) else {
            return;
        };
        let next = config
            .mapping
            .get(column)
            .map(|index| index + 1)
            .unwrap_or(0);
        if next < csv.headers.len() {
            config.mapping.insert(column.to_owned(), next);
        } else {
            config.mapping.remove(column);
            config.match_keys.retain(|key| key != column);
            config.update_fields.retain(|field| field != column);
        }
        cx.notify();
    }

    fn toggle_import_key(&mut self, column: &str, cx: &mut Context<Self>) {
        let Some(config) = self
            .data_import
            .as_mut()
            .and_then(|import| import.config.as_mut())
        else {
            return;
        };
        if !config.mapping.contains_key(column) {
            return;
        }
        if config.match_keys.iter().any(|key| key == column) {
            config.match_keys.retain(|key| key != column);
        } else {
            config.match_keys.push(column.to_owned());
            config.update_fields.retain(|field| field != column);
        }
        cx.notify();
    }

    fn toggle_import_update(&mut self, column: &str, cx: &mut Context<Self>) {
        let Some(config) = self
            .data_import
            .as_mut()
            .and_then(|import| import.config.as_mut())
        else {
            return;
        };
        if !config.mapping.contains_key(column) || config.match_keys.iter().any(|key| key == column)
        {
            return;
        }
        if config.update_fields.iter().any(|field| field == column) {
            config.update_fields.retain(|field| field != column);
        } else {
            config.update_fields.push(column.to_owned());
        }
        cx.notify();
    }

    fn review_import(&mut self, cx: &mut Context<Self>) {
        let Some((tab_id, connection_id, database, table, csv, config)) =
            self.data_import.as_ref().and_then(|import| {
                Some((
                    import.tab_id,
                    import.target.connection_id.clone(),
                    import.target.database.clone(),
                    import.table.clone(),
                    import.csv.clone()?,
                    import.config.clone()?,
                ))
            })
        else {
            return;
        };
        if !validate_import(&csv, &table, &config).is_empty() {
            return;
        }
        if let Some(import) = &mut self.data_import {
            import.loading = true;
        }
        let task =
            cx.background_spawn(
                async move { build_import_request(&database, &table, &csv, &config) },
            );
        cx.spawn(async move |this, cx| {
            let request = task.await;
            this.update(cx, |this, cx| {
                if this.data_import.as_ref().map(DataImport::tab_id) != Some(tab_id) {
                    return;
                }
                if request.changes.is_empty() {
                    if let Some(import) = &mut this.data_import {
                        import.loading = false;
                        import.error = Some("No rows have usable match-key values".into());
                    }
                    cx.notify();
                    return;
                }
                let mut preview = request.clone();
                preview.changes.truncate(25);
                this.data_import = None;
                this.open_import_review(tab_id, connection_id, request, preview, cx);
            })
            .ok();
        })
        .detach();
        cx.notify();
    }

    fn dismiss_import(&mut self, cx: &mut Context<Self>) {
        self.data_import = None;
        cx.notify();
    }

    pub(super) fn import_overlay(&self, cx: &mut Context<Self>) -> AnyElement {
        let import = self
            .data_import
            .as_ref()
            .expect("import overlay requires import state");
        let blockers = import
            .csv
            .as_ref()
            .zip(import.config.as_ref())
            .map(|(csv, config)| validate_import(csv, &import.table, config))
            .unwrap_or_default();
        let counts = import
            .csv
            .as_ref()
            .zip(import.config.as_ref())
            .map(|(csv, config)| import_counts(csv, config));
        let ready = blockers.is_empty()
            && counts.is_some_and(|counts| counts.to_write > 0)
            && !import.loading
            && import.error.is_none();
        let close = cx.listener(|this, _, _, cx| this.dismiss_import(cx));

        div()
            .id("data-import-backdrop")
            .absolute()
            .inset_0()
            .flex()
            .items_start()
            .justify_center()
            .pt(gpui::relative(0.08))
            .bg(cellar_desktop_gpui::theme::overlay())
            .on_click(cx.listener(|this, _, _, cx| this.dismiss_import(cx)))
            .child(
                div()
                    .id("data-import-modal")
                    .w(px(760.))
                    .h(px(640.))
                    .max_h(gpui::relative(0.84))
                    .flex()
                    .flex_col()
                    .overflow_hidden()
                    .rounded(px(8.))
                    .bg(PANEL)
                    .border_1()
                    .border_color(BORDER)
                    .shadow_lg()
                    .on_click(|_, _, cx| cx.stop_propagation())
                    .child(
                        div()
                            .h(px(38.))
                            .flex_shrink_0()
                            .flex()
                            .items_center()
                            .gap_2()
                            .border_b_1()
                            .border_color(BORDER)
                            .pl(px(14.))
                            .pr_2()
                            .child(
                                Icon::empty()
                                    .path("icons/upload.svg")
                                    .size(px(14.))
                                    .text_color(ACCENT),
                            )
                            .child(
                                div()
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .child("Import data"),
                            )
                            .child(
                                div()
                                    .ml_1()
                                    .pl(px(6.))
                                    .border_l_1()
                                    .border_color(BORDER)
                                    .font_family(cellar_desktop_gpui::theme::mono_font())
                                    .text_color(FG_MUTED)
                                    .child(format!(
                                        "{}.{}",
                                        import.target.schema, import.target.table
                                    )),
                            )
                            .child(div().flex_1())
                            .child(
                                div()
                                    .id("close-import")
                                    .tab_index(0)
                                    .cursor_pointer()
                                    .text_color(FG_MUTED)
                                    .child(Icon::empty().path("icons/close.svg").size(px(13.)))
                                    .on_click(close),
                            ),
                    )
                    .child(self.import_body(import, &blockers, counts, cx))
                    .child(
                        div()
                            .h(px(44.))
                            .flex_shrink_0()
                            .flex()
                            .items_center()
                            .justify_between()
                            .px_3()
                            .border_t_1()
                            .border_color(BORDER)
                            .bg(PANEL_MUTED)
                            .child(
                                div().text_color(FG_MUTED).child(
                                    counts
                                        .map(|counts| {
                                            format!(
                                                "{} rows · {} to write · {} skipped",
                                                counts.total, counts.to_write, counts.skipped
                                            )
                                        })
                                        .unwrap_or_default(),
                                ),
                            )
                            .child(
                                div()
                                    .id("review-import")
                                    .px_3()
                                    .py_1()
                                    .bg(if ready { ACCENT } else { PANEL_RAISED })
                                    .text_color(if ready { PANEL } else { FG_MUTED })
                                    .child("Review Import")
                                    .when(ready, |element| {
                                        element.tab_index(0).cursor_pointer().on_click(
                                            cx.listener(|this, _, _, cx| this.review_import(cx)),
                                        )
                                    }),
                            ),
                    ),
            )
            .into_any_element()
    }

    fn import_body(
        &self,
        import: &DataImport,
        blockers: &[String],
        counts: Option<cellar_runtime::csv_import::ImportCounts>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        if import.loading {
            return centered("Reading and parsing CSV…", FG_MUTED.rgba());
        }
        if let Some(error) = &import.error {
            return centered(error.clone(), PROD.rgba());
        }
        let (Some(csv), Some(config)) = (&import.csv, &import.config) else {
            return centered("Choose a CSV file", FG_MUTED.rgba());
        };
        div()
            .id("import-columns")
            .flex_1()
            .min_h_0()
            .flex()
            .flex_col()
            .child(
                div()
                    .h(px(42.))
                    .flex_shrink_0()
                    .flex()
                    .items_center()
                    .gap_4()
                    .px_3()
                    .border_b_1()
                    .border_color(BORDER)
                    .child(
                        div()
                            .id("import-mode")
                            .tab_index(0)
                            .cursor_pointer()
                            .text_color(ACCENT)
                            .child(format!("Mode: {}", config.mode.label()))
                            .on_click(cx.listener(|this, _, _, cx| this.cycle_import_mode(cx))),
                    )
                    .child(
                        div().text_color(FG_MUTED).child(
                            import
                                .path
                                .as_ref()
                                .and_then(|path| path.file_name())
                                .map(|name| name.to_string_lossy().into_owned())
                                .unwrap_or_default(),
                        ),
                    )
                    .when_some(counts, |element, counts| {
                        element.child(format!("{} parsed rows", counts.total))
                    }),
            )
            .child(
                div()
                    .h(px(28.))
                    .flex_shrink_0()
                    .flex()
                    .items_center()
                    .px_3()
                    .bg(PANEL_RAISED)
                    .text_color(FG_MUTED)
                    .child("TABLE COLUMN · CSV SOURCE · ROLE"),
            )
            .child(
                div()
                    .id("import-column-list")
                    .flex_1()
                    .min_h_0()
                    .overflow_scroll()
                    .children(
                        import
                            .table
                            .columns
                            .iter()
                            .map(|column| self.import_column_row(column, csv, config, cx)),
                    ),
            )
            .when(!blockers.is_empty(), |element| {
                element.child(
                    div()
                        .flex_shrink_0()
                        .px_3()
                        .py_2()
                        .border_t_1()
                        .border_color(BORDER)
                        .text_color(WARN)
                        .children(blockers.iter().cloned().map(|blocker| div().child(blocker))),
                )
            })
            .into_any_element()
    }

    fn import_column_row(
        &self,
        column: &cellar_core::schema::Column,
        csv: &ParsedCsv,
        config: &ImportConfig,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let mapping = config.mapping.get(&column.name).copied();
        let mapped = mapping.is_some();
        let key = config.match_keys.contains(&column.name);
        let update = config.update_fields.contains(&column.name) && !key;
        let map_column = column.name.clone();
        let key_column = column.name.clone();
        let update_column = column.name.clone();
        div()
            .h(px(32.))
            .flex()
            .items_center()
            .gap_2()
            .px_3()
            .border_b_1()
            .border_color(BORDER)
            .child(
                div()
                    .w(px(280.))
                    .truncate()
                    .font_family(cellar_desktop_gpui::theme::mono_font())
                    .child(format!(
                        "{} · {}{}",
                        column.name,
                        column.data_type,
                        if column.is_primary_key { " · PK" } else { "" }
                    )),
            )
            .child(
                div()
                    .id(SharedString::from(format!("map-import:{}", column.name)))
                    .tab_index(0)
                    .cursor_pointer()
                    .w(px(220.))
                    .truncate()
                    .text_color(if mapped { ACCENT } else { FG_MUTED })
                    .child(
                        mapping
                            .and_then(|index| csv.headers.get(index))
                            .cloned()
                            .unwrap_or_else(|| "(skip)".into()),
                    )
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.cycle_import_mapping(&map_column, cx)
                    })),
            )
            .child(
                role_button(&format!("key-import:{}", column.name), "KEY", key, mapped).on_click(
                    cx.listener(move |this, _, _, cx| this.toggle_import_key(&key_column, cx)),
                ),
            )
            .child(
                role_button(
                    &format!("update-import:{}", column.name),
                    "UPDATE",
                    update,
                    mapped && !key,
                )
                .on_click(
                    cx.listener(move |this, _, _, cx| {
                        this.toggle_import_update(&update_column, cx)
                    }),
                ),
            )
            .into_any_element()
    }
}

fn read_csv(path: &PathBuf) -> Result<ParsedCsv, String> {
    let metadata = std::fs::metadata(path).map_err(|error| error.to_string())?;
    if metadata.len() > MAX_IMPORT_BYTES {
        return Err("CSV exceeds the 64 MiB import limit".into());
    }
    let text = std::fs::read_to_string(path).map_err(|error| error.to_string())?;
    let csv = parse_csv(&text)?;
    if csv.rows.len() > MAX_IMPORT_ROWS {
        return Err("CSV exceeds the 100,000 row import limit".into());
    }
    Ok(csv)
}

fn set_import_error(
    this: &gpui::WeakEntity<CellarApp>,
    cx: &mut gpui::AsyncApp,
    tab_id: u64,
    error: String,
) {
    this.update(cx, |this, cx| {
        if let Some(import) = this
            .data_import
            .as_mut()
            .filter(|import| import.tab_id == tab_id)
        {
            import.loading = false;
            import.error = Some(error);
            cx.notify();
        }
    })
    .ok();
}

fn centered(text: impl Into<SharedString>, color: gpui::Rgba) -> AnyElement {
    div()
        .flex_1()
        .flex()
        .items_center()
        .justify_center()
        .text_color(color)
        .child(text.into())
        .into_any_element()
}

fn role_button(
    id: &str,
    label: &'static str,
    active: bool,
    enabled: bool,
) -> gpui::Stateful<gpui::Div> {
    div()
        .id(SharedString::from(id.to_owned()))
        .px_2()
        .py_1()
        .text_color(if active { ACCENT } else { FG_MUTED })
        .bg(if active { PANEL_RAISED } else { PANEL })
        .when(enabled, |element| element.tab_index(0).cursor_pointer())
        .child(label)
}
