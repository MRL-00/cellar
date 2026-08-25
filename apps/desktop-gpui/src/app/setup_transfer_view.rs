use std::{collections::HashSet, path::PathBuf, sync::Arc};

use gpui::{
    div, prelude::*, px, AnyElement, AppContext, ClipboardItem, Context, PathPromptOptions,
    SharedString, Window,
};
use gpui_component::{input::InputState, scroll::ScrollableElement, Icon};

use super::setup_transfer_widgets::*;
use super::{
    setup_transfer::{
        parse_setup, prepared_connections, serialize_setup, set_connection_bulk, set_layout_bulk,
        write_setup, ExportSetup, ImportDecision, ImportPlan, ImportSetup, ImportSetupState,
        ImportSummary, SetupSection, SetupTransfer,
    },
    CellarApp,
};
use cellar_desktop_gpui::theme::{
    ACCENT, BORDER, FG_MUTED, FG_SECONDARY, INSET, PANEL, PANEL_RAISED, PROD,
};

impl CellarApp {
    pub(super) fn open_export_setup(&mut self, cx: &mut Context<Self>) {
        let layouts = self
            .model
            .tabs()
            .iter()
            .filter_map(|tab| match &tab.kind {
                cellar_desktop_gpui::model::TabKind::Table { target, .. } => self
                    .grid_layout(tab.id, cx)
                    .map(|layout| (super::table_workspace::table_layout_key(target), layout)),
                _ => None,
            })
            .collect::<Vec<_>>();
        self.table_layouts.extend(layouts);
        self.settings_open = false;
        self.setup_transfer = Some(SetupTransfer::Export(ExportSetup {
            selected: HashSet::from([
                SetupSection::Settings,
                SetupSection::Connections,
                SetupSection::TableLayouts,
            ]),
            connection_ids: self
                .model
                .connections()
                .iter()
                .map(|connection| connection.id.clone())
                .collect(),
            message: None,
        }));
        cx.notify();
    }

    pub(super) fn open_import_setup(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.settings_open = false;
        self.setup_transfer = Some(SetupTransfer::Import(ImportSetup {
            state: ImportSetupState::Source { loading: false },
            file_name: None,
            raw: cx.new(|cx| {
                InputState::new(window, cx)
                    .multi_line(true)
                    .placeholder("{\n  \"format\": \"cellar.setup\",\n  ...\n}")
            }),
            error: None,
            applying: false,
        }));
        cx.notify();
    }

    pub(super) fn setup_transfer_overlay(&self, cx: &mut Context<Self>) -> AnyElement {
        let Some(transfer) = self.setup_transfer.clone() else {
            return div().into_any_element();
        };
        let width = if matches!(transfer, SetupTransfer::Import(_)) {
            620.
        } else {
            560.
        };
        div()
            .id("setup-transfer-backdrop")
            .absolute()
            .inset_0()
            .bg(cellar_desktop_gpui::theme::overlay())
            .flex()
            .items_start()
            .justify_center()
            .pt(gpui::relative(0.08))
            .on_click(cx.listener(|this, _, _, cx| {
                if !this.setup_import_busy() {
                    this.setup_transfer = None;
                    cx.notify();
                }
            }))
            .child(
                div()
                    .id("setup-transfer-card")
                    .w(px(width))
                    .max_h(px(660.))
                    .flex()
                    .flex_col()
                    .rounded(px(8.))
                    .border_1()
                    .border_color(BORDER)
                    .bg(PANEL)
                    .overflow_hidden()
                    .on_click(|_, _, cx| cx.stop_propagation())
                    .child(match transfer {
                        SetupTransfer::Export(export) => self.export_setup_content(export, cx),
                        SetupTransfer::Import(import) => self.import_setup_content(import, cx),
                    }),
            )
            .into_any_element()
    }

    fn export_setup_content(&self, export: ExportSetup, cx: &mut Context<Self>) -> AnyElement {
        let json = serialize_setup(&self.setup_bundle(&export)).unwrap_or_default();
        let connections = self.model.connections();
        let selected_count = export.connection_ids.len();
        let layouts = self.table_layouts.len();
        div()
            .flex()
            .flex_col()
            .min_h_0()
            .child(modal_header("icons/download.svg", "Export setup", cx))
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scrollbar()
                    .px_4()
                    .py_3()
                    .child(div().mb_3().max_w(px(470.)).text_color(FG_SECONDARY).child("Pick what to include, then download a .json file you can share or move to another machine."))
                    .child(section_card(
                        "setup-section-settings",
                        "Appearance & settings",
                        "Theme, accent, density, fonts, font size",
                        "×1".into(),
                        export.selected.contains(&SetupSection::Settings),
                        cx.listener(|this, _, _, cx| this.toggle_setup_section(SetupSection::Settings, cx)),
                    ))
                    .child(section_card(
                        "setup-section-connections",
                        "Connections",
                        &format!("{selected_count} of {} selected — passwords excluded", connections.len()),
                        format!("{selected_count}/{}", connections.len()),
                        export.selected.contains(&SetupSection::Connections),
                        cx.listener(|this, _, _, cx| this.toggle_setup_section(SetupSection::Connections, cx)),
                    ))
                    .when(export.selected.contains(&SetupSection::Connections), |element| {
                        element.child(
                            div()
                                .ml_2()
                                .pl_2()
                                .border_l_1()
                                .border_color(BORDER)
                                .children(connections.iter().map(|connection| {
                                    let id = connection.id.clone();
                                    let picked = export.connection_ids.contains(&id);
                                    div()
                                        .id(SharedString::from(format!("setup-connection:{id}")))
                                        .tab_index(0)
                                        .cursor_pointer()
                                        .mt_1()
                                        .flex()
                                        .items_center()
                                        .gap_2()
                                        .rounded(px(5.))
                                        .border_1()
                                        .border_color(BORDER)
                                        .bg(if picked { PANEL_RAISED } else { INSET })
                                        .px_2()
                                        .py_1()
                                        .opacity(if picked { 1. } else { 0.7 })
                                        .child(check_box(picked))
                                        .child(Icon::empty().path("icons/database.svg").size(px(12.)).text_color(FG_MUTED))
                                        .child(div().min_w_0().flex_1().child(div().truncate().font_weight(gpui::FontWeight::MEDIUM).child(connection.name.clone())).child(div().truncate().font_family(cellar_desktop_gpui::theme::mono_font()).text_size(px(11.)).text_color(FG_MUTED).child(connection_hint(connection))))
                                        .on_click(cx.listener(move |this, _, _, cx| this.toggle_setup_connection(&id, cx)))
                                })),
                        )
                    })
                    .child(section_card(
                        "setup-section-layouts",
                        "Table grid layouts",
                        &format!("{layouts} saved layouts (column order & widths)"),
                        format!("×{layouts}"),
                        export.selected.contains(&SetupSection::TableLayouts),
                        cx.listener(|this, _, _, cx| this.toggle_setup_section(SetupSection::TableLayouts, cx)),
                    ))
                    .child(
                        div()
                            .mt_3()
                            .flex()
                            .items_center()
                            .gap_2()
                            .rounded(px(4.))
                            .border_1()
                            .border_color(BORDER)
                            .bg(INSET)
                            .px_3()
                            .py_2()
                            .text_color(FG_SECONDARY)
                            .child(Icon::empty().path("icons/lock.svg").size(px(12.)).text_color(FG_MUTED))
                            .child("Passwords and API keys are never exported — recipients re-enter their own."),
                    )
                    .when_some(export.message.clone(), |element, message| {
                        element.child(div().mt_2().text_color(if message.is_ok() { ACCENT } else { PROD }).child(message.unwrap_or_else(|error| error)))
                    }),
            )
            .child(
                modal_footer()
                    .child(div().font_family(cellar_desktop_gpui::theme::mono_font()).text_size(px(11.5)).text_color(FG_MUTED).child(format!("{} bytes", json.len())))
                    .child(div().flex_1())
                    .child(footer_button("cancel-setup-export", "Cancel", false).on_click(cx.listener(|this, _, _, cx| { this.setup_transfer = None; cx.notify(); })))
                    .child(footer_button("copy-setup-json", "Copy JSON", false).on_click(cx.listener(|this, _, _, cx| this.copy_setup_json(cx))))
                    .child(footer_button("download-setup-json", "Download .json", true).on_click(cx.listener(|this, _, _, cx| this.download_setup_json(cx)))),
            )
            .into_any_element()
    }

    fn import_setup_content(&self, import: ImportSetup, cx: &mut Context<Self>) -> AnyElement {
        let body = match import.state.clone() {
            ImportSetupState::Source { loading } => {
                import_source(import.raw.clone(), import.error.clone(), loading, cx)
            }
            ImportSetupState::Review(plan) => self.import_review(plan, import.error.clone(), cx),
            ImportSetupState::Complete(summary) => import_result(summary),
        };
        div()
            .flex()
            .flex_col()
            .min_h_0()
            .child(modal_header("icons/upload.svg", "Import setup", cx))
            .child(body)
            .child(self.import_footer(&import, cx))
            .into_any_element()
    }

    fn import_review(
        &self,
        plan: ImportPlan,
        error: Option<String>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        div()
            .flex_1()
            .min_h_0()
            .overflow_y_scrollbar()
            .px_4()
            .py_3()
            .child(div().mb_2().text_color(FG_SECONDARY).child(
                "Review each item. Existing matches are skipped unless you choose replace or copy.",
            ))
            .when_some(plan.settings.clone(), |element, (_, apply)| {
                element.child(review_row(
                    "import-settings",
                    "Appearance & settings",
                    "theme, accent, density and fonts",
                    if apply { "apply" } else { "skip" },
                    apply,
                    cx.listener(|this, _, _, cx| this.toggle_import_settings(cx)),
                ))
            })
            .when(!plan.connections.is_empty(), |element| {
                element
                    .child(review_heading(format!(
                        "Connections ({})",
                        plan.connections.len()
                    )))
                    .child(
                        div()
                            .mb_1()
                            .flex()
                            .items_center()
                            .gap_1()
                            .child(div().text_size(px(11.)).text_color(FG_MUTED).child("BULK"))
                            .child(bulk_button(
                                "bulk-connections-add",
                                "Add all new",
                                cx.listener(|this, _, _, cx| {
                                    this.bulk_import_connections(ImportDecision::Add, cx)
                                }),
                            ))
                            .child(bulk_button(
                                "bulk-connections-replace",
                                "Replace all duplicates",
                                cx.listener(|this, _, _, cx| {
                                    this.bulk_import_connections(ImportDecision::Replace, cx)
                                }),
                            ))
                            .child(bulk_button(
                                "bulk-connections-skip",
                                "Skip all",
                                cx.listener(|this, _, _, cx| {
                                    this.bulk_import_connections(ImportDecision::Skip, cx)
                                }),
                            )),
                    )
            })
            .children(plan.connections.iter().enumerate().map(|(index, item)| {
                let detail = item.duplicate_name.as_ref().map_or_else(
                    || connection_hint(&item.incoming),
                    |name| format!("matches {name}"),
                );
                review_row(
                    SharedString::from(format!("import-connection:{index}")),
                    &item.incoming.name,
                    &detail,
                    decision_label(item.decision),
                    item.decision != ImportDecision::Skip,
                    cx.listener(move |this, _, _, cx| this.cycle_import_connection(index, cx)),
                )
            }))
            .when(!plan.layouts.is_empty(), |element| {
                element
                    .child(review_heading(format!(
                        "Table grid layouts ({})",
                        plan.layouts.len()
                    )))
                    .child(
                        div()
                            .mb_1()
                            .flex()
                            .items_center()
                            .gap_1()
                            .child(div().text_size(px(11.)).text_color(FG_MUTED).child("BULK"))
                            .child(bulk_button(
                                "bulk-layouts-add",
                                "Add all new",
                                cx.listener(|this, _, _, cx| {
                                    this.bulk_import_layouts(true, false, cx)
                                }),
                            ))
                            .child(bulk_button(
                                "bulk-layouts-replace",
                                "Replace all existing",
                                cx.listener(|this, _, _, cx| {
                                    this.bulk_import_layouts(true, true, cx)
                                }),
                            ))
                            .child(bulk_button(
                                "bulk-layouts-skip",
                                "Skip all",
                                cx.listener(|this, _, _, cx| {
                                    this.bulk_import_layouts(false, true, cx)
                                }),
                            )),
                    )
            })
            .children(plan.layouts.iter().enumerate().map(|(index, item)| {
                review_row(
                    SharedString::from(format!("import-layout:{index}")),
                    item.key
                        .split_once("::")
                        .map_or(item.key.as_str(), |(_, path)| path),
                    if item.exists {
                        "saved layout already exists"
                    } else {
                        "new table layout"
                    },
                    if item.apply {
                        if item.exists {
                            "replace"
                        } else {
                            "add"
                        }
                    } else {
                        "skip"
                    },
                    item.apply,
                    cx.listener(move |this, _, _, cx| this.toggle_import_layout(index, cx)),
                )
            }))
            .when_some(error, |element, error| {
                element.child(div().mt_2().text_color(PROD).child(error))
            })
            .into_any_element()
    }

    fn import_footer(&self, import: &ImportSetup, cx: &mut Context<Self>) -> AnyElement {
        let review = matches!(&import.state, ImportSetupState::Review(_));
        let complete = matches!(&import.state, ImportSetupState::Complete(_));
        let source = matches!(&import.state, ImportSetupState::Source { .. });
        let can_review = source && !import.raw.read(cx).value().trim().is_empty();
        let can_apply = matches!(&import.state, ImportSetupState::Review(plan) if plan.settings.as_ref().is_some_and(|(_, apply)| *apply) || plan.connections.iter().any(|item| item.decision != ImportDecision::Skip) || plan.layouts.iter().any(|item| item.apply));
        modal_footer()
            .child(if review {
                footer_button("import-setup-back", "Back", false)
                    .on_click(cx.listener(|this, _, _, cx| {
                        if let Some(SetupTransfer::Import(import)) = this.setup_transfer.as_mut() {
                            import.state = ImportSetupState::Source { loading: false };
                            import.error = None;
                        }
                        cx.notify();
                    }))
                    .into_any_element()
            } else {
                div().into_any_element()
            })
            .when(source, |footer| {
                footer.child(
                    div()
                        .font_family(cellar_desktop_gpui::theme::mono_font())
                        .text_size(px(11.5))
                        .text_color(FG_MUTED)
                        .child(
                            import
                                .file_name
                                .clone()
                                .unwrap_or_else(|| "no file selected".into()),
                        ),
                )
            })
            .child(div().flex_1())
            .child(if review {
                footer_button(
                    "apply-setup-import",
                    if import.applying {
                        "Applying…"
                    } else {
                        "Apply import"
                    },
                    true,
                )
                .opacity(if can_apply && !import.applying {
                    1.
                } else {
                    0.4
                })
                .when(can_apply && !import.applying, |button| {
                    button.on_click(
                        cx.listener(|this, _, window, cx| this.apply_setup_import(window, cx)),
                    )
                })
                .into_any_element()
            } else if complete {
                footer_button("done-setup-import", "Done", true)
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.setup_transfer = None;
                        cx.notify();
                    }))
                    .into_any_element()
            } else {
                footer_button("review-setup-json", "Review", true)
                    .opacity(if can_review { 1. } else { 0.4 })
                    .when(can_review, |button| {
                        button.on_click(cx.listener(|this, _, _, cx| this.review_setup_text(cx)))
                    })
                    .into_any_element()
            })
            .into_any_element()
    }

    pub(super) fn setup_import_busy(&self) -> bool {
        matches!(&self.setup_transfer, Some(SetupTransfer::Import(import)) if import.applying || matches!(import.state, ImportSetupState::Source { loading: true }))
    }

    fn toggle_setup_section(&mut self, section: SetupSection, cx: &mut Context<Self>) {
        if let Some(SetupTransfer::Export(export)) = self.setup_transfer.as_mut() {
            if !export.selected.remove(&section) {
                export.selected.insert(section);
            }
            export.message = None;
            cx.notify();
        }
    }

    fn toggle_setup_connection(&mut self, id: &str, cx: &mut Context<Self>) {
        if let Some(SetupTransfer::Export(export)) = self.setup_transfer.as_mut() {
            if !export.connection_ids.remove(id) {
                export.connection_ids.insert(id.into());
            }
            export.message = None;
            cx.notify();
        }
    }

    fn copy_setup_json(&mut self, cx: &mut Context<Self>) {
        let Some(SetupTransfer::Export(export)) = self.setup_transfer.as_ref() else {
            return;
        };
        let result = serialize_setup(&self.setup_bundle(export)).map(|json| {
            cx.write_to_clipboard(ClipboardItem::new_string(
                String::from_utf8_lossy(&json).into_owned(),
            ));
            "Copied JSON".into()
        });
        if let Some(SetupTransfer::Export(export)) = self.setup_transfer.as_mut() {
            export.message = Some(result);
        }
        cx.notify();
    }

    fn download_setup_json(&mut self, cx: &mut Context<Self>) {
        let Some(SetupTransfer::Export(export)) = self.setup_transfer.as_ref() else {
            return;
        };
        let bytes = match serialize_setup(&self.setup_bundle(export)) {
            Ok(bytes) => bytes,
            Err(error) => {
                if let Some(SetupTransfer::Export(export)) = self.setup_transfer.as_mut() {
                    export.message = Some(Err(error));
                }
                return;
            }
        };
        let filename = format!(
            "cellar-setup-{}.json",
            chrono::Local::now().format("%Y-%m-%d")
        );
        let receiver = cx.prompt_for_new_path(
            &std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            Some(&filename),
        );
        cx.spawn(async move |this, cx| {
            let path = match receiver.await {
                Ok(Ok(Some(path))) => path,
                Ok(Ok(None)) => return,
                Ok(Err(error)) => {
                    set_export_message(&this, cx, Err(error.to_string()));
                    return;
                }
                Err(error) => {
                    set_export_message(&this, cx, Err(error.to_string()));
                    return;
                }
            };
            let label = path.to_string_lossy().into_owned();
            let result = cx
                .background_spawn(async move { write_setup(&path, &bytes) })
                .await
                .map(|_| format!("Saved {label}"));
            set_export_message(&this, cx, result);
        })
        .detach();
    }

    pub(super) fn choose_setup_file(&mut self, cx: &mut Context<Self>) {
        let receiver = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some("Import setup".into()),
        });
        if let Some(SetupTransfer::Import(import)) = self.setup_transfer.as_mut() {
            import.state = ImportSetupState::Source { loading: true };
            import.error = None;
        }
        cx.notify();
        cx.spawn(async move |this, cx| {
            let path = match receiver.await {
                Ok(Ok(Some(paths))) => paths.into_iter().next(),
                Ok(Ok(None)) => None,
                Ok(Err(error)) => {
                    set_import_error(&this, cx, error.to_string());
                    return;
                }
                Err(error) => {
                    set_import_error(&this, cx, error.to_string());
                    return;
                }
            };
            let Some(path) = path else {
                set_import_source(&this, cx);
                return;
            };
            let file_name = path
                .file_name()
                .map(|name| name.to_string_lossy().into_owned());
            let parsed = cx
                .background_spawn(async move {
                    std::fs::read(path)
                        .map_err(|error| error.to_string())
                        .and_then(|bytes| parse_setup(&bytes))
                })
                .await;
            this.update(cx, |this, cx| {
                let plan = parsed.map(|bundle| this.import_plan(bundle));
                if let Some(SetupTransfer::Import(import)) = this.setup_transfer.as_mut() {
                    import.file_name = file_name;
                    match plan {
                        Ok(plan) => import.state = ImportSetupState::Review(plan),
                        Err(error) => {
                            import.state = ImportSetupState::Source { loading: false };
                            import.error = Some(error);
                        }
                    }
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn review_setup_text(&mut self, cx: &mut Context<Self>) {
        let Some(SetupTransfer::Import(import)) = self.setup_transfer.as_ref() else {
            return;
        };
        let raw = import.raw.read(cx).value().to_string();
        let plan = parse_setup(raw.as_bytes()).map(|bundle| self.import_plan(bundle));
        if let Some(SetupTransfer::Import(import)) = self.setup_transfer.as_mut() {
            match plan {
                Ok(plan) => {
                    import.error = None;
                    import.state = ImportSetupState::Review(plan);
                }
                Err(error) => import.error = Some(error),
            }
        }
        cx.notify();
    }

    fn cycle_import_connection(&mut self, index: usize, cx: &mut Context<Self>) {
        if let Some(SetupTransfer::Import(ImportSetup {
            state: ImportSetupState::Review(plan),
            ..
        })) = self.setup_transfer.as_mut()
        {
            if let Some(item) = plan.connections.get_mut(index) {
                item.decision = if item.duplicate_id.is_some() {
                    match item.decision {
                        ImportDecision::Skip => ImportDecision::Replace,
                        ImportDecision::Replace => ImportDecision::Copy,
                        _ => ImportDecision::Skip,
                    }
                } else if item.decision == ImportDecision::Add {
                    ImportDecision::Skip
                } else {
                    ImportDecision::Add
                };
            }
            cx.notify();
        }
    }

    fn toggle_import_layout(&mut self, index: usize, cx: &mut Context<Self>) {
        if let Some(SetupTransfer::Import(ImportSetup {
            state: ImportSetupState::Review(plan),
            ..
        })) = self.setup_transfer.as_mut()
        {
            if let Some(item) = plan.layouts.get_mut(index) {
                item.apply = !item.apply;
            }
            cx.notify();
        }
    }

    fn bulk_import_connections(&mut self, decision: ImportDecision, cx: &mut Context<Self>) {
        if let Some(SetupTransfer::Import(ImportSetup {
            state: ImportSetupState::Review(plan),
            ..
        })) = self.setup_transfer.as_mut()
        {
            set_connection_bulk(plan, decision);
            cx.notify();
        }
    }

    fn bulk_import_layouts(&mut self, apply: bool, existing: bool, cx: &mut Context<Self>) {
        if let Some(SetupTransfer::Import(ImportSetup {
            state: ImportSetupState::Review(plan),
            ..
        })) = self.setup_transfer.as_mut()
        {
            set_layout_bulk(plan, apply, existing);
            cx.notify();
        }
    }

    fn toggle_import_settings(&mut self, cx: &mut Context<Self>) {
        if let Some(SetupTransfer::Import(ImportSetup {
            state: ImportSetupState::Review(plan),
            ..
        })) = self.setup_transfer.as_mut()
        {
            if let Some((_, apply)) = plan.settings.as_mut() {
                *apply = !*apply;
            }
            cx.notify();
        }
    }

    fn apply_setup_import(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(SetupTransfer::Import(import)) = self.setup_transfer.as_mut() else {
            return;
        };
        let ImportSetupState::Review(plan) = import.state.clone() else {
            return;
        };
        import.applying = true;
        import.error = None;
        let summary = ImportSummary::from_plan(&plan);
        if let Some((settings, true)) = plan.settings.clone() {
            self.preferences = settings;
            self.apply_appearance(window, cx);
        }
        for item in plan.layouts.iter().filter(|item| item.apply) {
            self.table_layouts
                .insert(item.key.clone(), item.layout.clone());
        }
        let connections = prepared_connections(
            &plan,
            self.model
                .connections()
                .iter()
                .map(|connection| connection.id.clone()),
        );
        let registry = Arc::clone(&self.registry);
        let runtime = Arc::clone(&self.runtime);
        cx.spawn(async move |this, cx| {
            let saved = runtime
                .spawn(async move {
                    let mut saved = Vec::new();
                    for connection in connections {
                        saved.push(
                            registry
                                .save(connection)
                                .await
                                .map_err(|error| error.to_string())?,
                        );
                    }
                    Ok::<_, String>(saved)
                })
                .await
                .map_err(|error| error.to_string())
                .and_then(|result| result);
            this.update(cx, |this, cx| {
                let result = saved.map(|saved| {
                    for connection in saved {
                        this.model.upsert_connection(connection);
                    }
                    this.reconcile_sidebar_layout();
                });
                if let Some(SetupTransfer::Import(import)) = this.setup_transfer.as_mut() {
                    import.applying = false;
                    match result {
                        Ok(()) => import.state = ImportSetupState::Complete(summary),
                        Err(error) => import.error = Some(error),
                    }
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
        cx.notify();
    }
}
