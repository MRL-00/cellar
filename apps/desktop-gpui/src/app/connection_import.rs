use std::{collections::HashSet, sync::Arc};

use cellar_core::driver::ConnectionConfig;
use cellar_runtime::connection_import::ConnectionImport as ImportResult;
use gpui::{div, prelude::*, px, AnyElement, Context, Entity, SharedString, Window};
use gpui_component::{checkbox::Checkbox, input::InputState, Disableable, Icon};

use cellar_desktop_gpui::theme::{
    accent, ACCENT, ACCENT_FG, BORDER, BORDER_STRONG, FG, FG_MUTED, FG_SECONDARY, INSET, PANEL,
    PANEL_MUTED, PANEL_RAISED, WARN,
};
use cellar_desktop_gpui::widgets::compact_input;

use super::{
    sidebar_layout::SidebarItem,
    sidebar_menu::{find_or_create_folder, remove_connection_from_layout},
    CellarApp,
};

/// External client whose saved connections can be imported into Cellar.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum ImportSource {
    Datagrip,
    Tableplus,
}

impl ImportSource {
    fn label(self) -> &'static str {
        match self {
            Self::Datagrip => "DataGrip",
            Self::Tableplus => "TablePlus",
        }
    }

    /// One-line caveat shown above the candidate list.
    fn hint(self) -> &'static str {
        match self {
            Self::Datagrip => "Passwords aren't stored by DataGrip — add them now or on first connect.",
            Self::Tableplus => "Passwords aren't stored by TablePlus — add them now or on first connect. Tunnelled connections will import, but Cellar can't open SSH tunnels yet.",
        }
    }

    fn scan(self) -> ImportResult {
        match self {
            Self::Datagrip => cellar_runtime::datagrip::scan(),
            Self::Tableplus => cellar_runtime::tableplus::scan(),
        }
    }
}

#[derive(Clone)]
struct ImportCandidate {
    config: ConnectionConfig,
    /// Sidebar folder this connection belongs in, from the source app's own
    /// grouping. `None` leaves it at the top level.
    folder: Option<String>,
    selected: bool,
    conflict: bool,
    database: Entity<InputState>,
    password: Entity<InputState>,
}

pub(super) struct ConnectionImport {
    source: ImportSource,
    candidates: Vec<ImportCandidate>,
    skipped: Vec<String>,
    scanning: bool,
    importing: bool,
    error: Option<String>,
}

impl ConnectionImport {
    fn scanning(source: ImportSource) -> Self {
        Self {
            source,
            candidates: Vec::new(),
            skipped: Vec::new(),
            scanning: true,
            importing: false,
            error: None,
        }
    }

    fn from_scan(
        source: ImportSource,
        result: ImportResult,
        existing: &HashSet<String>,
        window: &mut Window,
        cx: &mut Context<CellarApp>,
    ) -> Self {
        let ImportResult {
            connections,
            skipped,
            groups,
        } = result;
        Self {
            source,
            candidates: connections
                .into_iter()
                .map(|config| {
                    let (selected, conflict) = candidate_state(existing, &config.id);
                    let folder = groups.get(&config.id).cloned();
                    let database = cx.new(|cx| {
                        InputState::new(window, cx)
                            .default_value(config.database.clone())
                            .placeholder("database")
                    });
                    let password = cx.new(|cx| {
                        InputState::new(window, cx)
                            .masked(true)
                            .placeholder("password (optional)")
                    });
                    ImportCandidate {
                        config,
                        folder,
                        selected,
                        conflict,
                        database,
                        password,
                    }
                })
                .collect(),
            skipped,
            scanning: false,
            importing: false,
            error: None,
        }
    }
}

fn candidate_state(existing: &HashSet<String>, id: &str) -> (bool, bool) {
    (true, existing.contains(id))
}

impl CellarApp {
    pub(super) fn scan_connection_import(
        &mut self,
        source: ImportSource,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.connection_import = Some(ConnectionImport::scanning(source));
        let runtime = Arc::clone(&self.runtime);
        let label = source.label();
        cx.spawn_in(window, async move |this, cx| {
            let result = runtime
                .spawn_blocking(move || source.scan())
                .await
                .map_err(|error| format!("{label} scan failed: {error}"));
            // `this.update_in` reaches the app entity directly; the window's
            // root view is a `Root` wrapper, so downcasting it would fail.
            this.update_in(cx, |this, window, cx| {
                if this.connection_import.is_none() {
                    return;
                }
                match result {
                    Ok(result) => {
                        let existing = this
                            .model
                            .connections()
                            .iter()
                            .map(|config| config.id.clone())
                            .collect();
                        this.connection_import = Some(ConnectionImport::from_scan(
                            source, result, &existing, window, cx,
                        ));
                    }
                    Err(error) => {
                        let import = this.connection_import.as_mut().unwrap();
                        import.scanning = false;
                        import.error = Some(error);
                    }
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
        cx.notify();
    }

    fn toggle_connection_import(&mut self, id: &str, cx: &mut Context<Self>) {
        let Some(import) = &mut self.connection_import else {
            return;
        };
        if import.importing {
            return;
        }
        if let Some(candidate) = import
            .candidates
            .iter_mut()
            .find(|candidate| candidate.config.id == id)
        {
            candidate.selected = !candidate.selected;
            cx.notify();
        }
    }

    fn toggle_all_connection_imports(&mut self, cx: &mut Context<Self>) {
        let Some(import) = &mut self.connection_import else {
            return;
        };
        if import.importing {
            return;
        }
        let selected = !import.candidates.is_empty()
            && import.candidates.iter().all(|candidate| candidate.selected);
        for candidate in &mut import.candidates {
            candidate.selected = !selected;
        }
        cx.notify();
    }

    fn import_selected_connections(&mut self, cx: &mut Context<Self>) {
        let Some(import) = &mut self.connection_import else {
            return;
        };
        if import.importing {
            return;
        }
        let selected: Vec<_> = import
            .candidates
            .iter()
            .filter(|candidate| candidate.selected)
            .map(|candidate| {
                let mut config = candidate.config.clone();
                let database = candidate.database.read(cx).value().trim().to_owned();
                if !database.is_empty() {
                    config.database = database;
                }
                let password = candidate.password.read(cx).value().to_string();
                (
                    config,
                    (!password.is_empty()).then_some(password),
                    candidate.folder.clone(),
                )
            })
            .collect();
        if selected.is_empty() {
            return;
        }
        // Folders are applied after the save, so the id -> folder pairing has to
        // survive the round trip through the runtime task.
        let folders: Vec<(String, String)> = selected
            .iter()
            .filter_map(|(config, _, folder)| {
                folder.clone().map(|folder| (config.id.clone(), folder))
            })
            .collect();
        let configs: Vec<_> = selected
            .into_iter()
            .map(|(config, password, _)| (config, password))
            .collect();
        import.importing = true;
        import.error = None;
        let registry = Arc::clone(&self.registry);
        let runtime = Arc::clone(&self.runtime);
        cx.spawn(async move |this, cx| {
            let result = runtime
                .spawn(async move {
                    let mut saved = Vec::with_capacity(configs.len());
                    for (config, password) in configs {
                        saved.push(
                            registry
                                .save_with_secret(config, password.as_deref())
                                .await?,
                        );
                    }
                    Ok::<_, cellar_core::error::CellarError>(saved)
                })
                .await
                .map_err(|error| format!("connection import task failed: {error}"))
                .and_then(|result| result.map_err(|error| error.to_string()));
            this.update(cx, |this, cx| {
                match result {
                    Ok(configs) => {
                        for config in configs {
                            this.model.upsert_connection(config);
                            this.reconcile_sidebar_layout();
                        }
                        // `reconcile_sidebar_layout` has just parked every new
                        // connection at the top level; move the grouped ones into
                        // their folder now that they all exist.
                        let stamp = chrono::Utc::now().timestamp_millis();
                        for (index, (connection_id, folder)) in folders.into_iter().enumerate() {
                            remove_connection_from_layout(&mut this.sidebar_layout, &connection_id);
                            let at = find_or_create_folder(
                                &mut this.sidebar_layout,
                                &folder,
                                format!("folder-{stamp}-{index}"),
                            );
                            if let SidebarItem::Folder { children, .. } =
                                &mut this.sidebar_layout[at]
                            {
                                children.push(connection_id);
                            }
                        }
                        this.connection_import = None;
                    }
                    Err(error) => {
                        if let Some(import) = &mut this.connection_import {
                            import.importing = false;
                            import.error = Some(error);
                        }
                    }
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
        cx.notify();
    }

    pub(super) fn connection_import_overlay(&self, cx: &mut Context<Self>) -> AnyElement {
        let import = self
            .connection_import
            .as_ref()
            .expect("connection import overlay requires state");
        let selected = import
            .candidates
            .iter()
            .filter(|candidate| candidate.selected)
            .count();
        let all_selected = !import.candidates.is_empty() && selected == import.candidates.len();
        let can_import = selected > 0 && !import.importing;
        let label = import.source.label();
        div()
            .id("connection-import-backdrop")
            .occlude()
            .absolute()
            .inset_0()
            .flex()
            .items_start()
            .justify_center()
            .pt(gpui::relative(0.08))
            .bg(cellar_desktop_gpui::theme::overlay())
            .on_click(cx.listener(|this, _, _, cx| {
                this.connection_import = None;
                cx.notify();
            }))
            .on_scroll_wheel(|_, _, cx| cx.stop_propagation())
            .child(
                div()
                    .id("connection-import-modal")
                    .w(px(760.))
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
                            .pl(px(14.))
                            .pr_2()
                            .border_b_1()
                            .border_color(BORDER)
                            .child(
                                Icon::empty()
                                    .path("icons/database.svg")
                                    .size(px(14.))
                                    .text_color(ACCENT),
                            )
                            .child(
                                div()
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .child(format!("Import from {label}")),
                            )
                            .child(div().flex_1())
                            .child(
                                div()
                                    .id("close-connection-import")
                                    .tab_index(0)
                                    .cursor_pointer()
                                    .size(px(22.))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .rounded(px(4.))
                                    .text_color(FG_MUTED)
                                    .hover(|style| style.bg(PANEL_RAISED).text_color(FG))
                                    .child(Icon::empty().path("icons/close.svg").size(px(13.)))
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.connection_import = None;
                                        cx.notify();
                                    })),
                            ),
                    )
                    .child(
                        div()
                            .id("connection-import-list")
                            .flex_1()
                            .min_h_0()
                            .overflow_y_scroll()
                            .px_4()
                            .pt_3()
                            .pb_4()
                            .when(import.scanning, |element| {
                                element.child(
                                    div()
                                        .px_2()
                                        .py_8()
                                        .text_center()
                                        .text_color(FG_MUTED)
                                        .child(format!("Scanning {label}…")),
                                )
                            })
                            .when(
                                !import.scanning
                                    && import.candidates.is_empty()
                                    && import.error.is_none(),
                                |element| {
                                    element.child(
                                        div()
                                            .px_2()
                                            .py_8()
                                            .text_center()
                                            .text_color(FG_MUTED)
                                            .child(format!(
                                                "No importable {label} connections found."
                                            )),
                                    )
                                },
                            )
                            .when(
                                !import.scanning
                                    && import.candidates.is_empty()
                                    && import.error.is_some(),
                                |element| {
                                    element.child(
                                        div()
                                            .px_2()
                                            .py_8()
                                            .text_center()
                                            .text_color(WARN)
                                            .child(import.error.clone().unwrap_or_default()),
                                    )
                                },
                            )
                            .when(!import.candidates.is_empty(), |element| {
                                element.child(
                                    div()
                                        .flex()
                                        .flex_col()
                                        .gap(px(6.))
                                        .child(
                                            div()
                                                .mb_1()
                                                .flex()
                                                .items_center()
                                                .justify_between()
                                                .gap_3()
                                                .child(
                                                    div()
                                                        .flex_1()
                                                        .min_w_0()
                                                        .text_size(px(12.))
                                                        .text_color(FG_MUTED)
                                                        .child(import.source.hint()),
                                                )
                                                .child(
                                                    div()
                                                        .id("toggle-all-import")
                                                        .tab_index(0)
                                                        .cursor_pointer()
                                                        .flex_shrink_0()
                                                        .font_weight(gpui::FontWeight::MEDIUM)
                                                        .text_color(ACCENT)
                                                        .child(if all_selected { "Unselect all" } else { "Select all" })
                                                        .on_click(cx.listener(|this, _, _, cx| this.toggle_all_connection_imports(cx))),
                                                ),
                                        )
                                        .children(import.candidates.iter().map(|candidate| {
                                let id = candidate.config.id.clone();
                                let selected = candidate.selected;
                                let conflict = candidate.conflict;
                                let folder = candidate.folder.clone();
                                let app = cx.entity().downgrade();
                                div()
                                    .id(SharedString::from(format!("import-candidate:{id}")))
                                    .flex()
                                    .items_center()
                                    .gap(px(10.))
                                    .rounded(px(5.))
                                    .border_1()
                                    .border_color(BORDER)
                                    .bg(PANEL_MUTED)
                                    .px(px(10.))
                                    .py(px(6.))
                                    .child(
                                        Checkbox::new(SharedString::from(format!("import-check:{id}")))
                                            .checked(selected)
                                            .disabled(import.importing)
                                            .on_click(move |_, _, cx| {
                                                app.update(cx, |this, cx| this.toggle_connection_import(&id, cx)).ok();
                                            }),
                                    )
                                    .child(engine_badge(candidate.config.engine))
                                    .child(
                                        div()
                                            .min_w_0()
                                            .flex_1()
                                            .child(
                                                div()
                                                    .truncate()
                                                    .font_weight(gpui::FontWeight::MEDIUM)
                                                    .child(candidate.config.name.clone())
                                                    .when_some(folder, |name, folder| {
                                                        name.child(
                                                            div()
                                                                .ml_2()
                                                                .flex_shrink_0()
                                                                .text_size(px(11.))
                                                                .font_weight(gpui::FontWeight::NORMAL)
                                                                .text_color(FG_MUTED)
                                                                .child(format!("in {folder}")),
                                                        )
                                                    })
                                                    .when(conflict, |name| {
                                                        name.child(
                                                            div()
                                                                .ml_2()
                                                                .text_size(px(11.))
                                                                .font_weight(gpui::FontWeight::NORMAL)
                                                                .text_color(WARN)
                                                                .child("overwrites existing"),
                                                        )
                                                    }),
                                            )
                                            .child(
                                                div()
                                                    .truncate()
                                                    .font_family(cellar_desktop_gpui::theme::mono_font())
                                                    .text_size(px(11.))
                                                    .text_color(FG_MUTED)
                                                    .child(format!(
                                                        "{}{}:{}",
                                                        if candidate.config.user.is_empty() { String::new() } else { format!("{}@", candidate.config.user) },
                                                        candidate.config.host,
                                                        candidate.config.port,
                                                    )),
                                            ),
                                    )
                                    .child(import_row_input(&candidate.database, 130., selected && !import.importing))
                                    .child(import_row_input(&candidate.password, 150., selected && !import.importing))
                            }))
                            .when(!import.skipped.is_empty(), |list| {
                                list.child(
                                    div()
                                        .mt_2()
                                        .rounded(px(5.))
                                        .border_1()
                                        .border_color(BORDER)
                                        .bg(INSET)
                                        .px(px(10.))
                                        .py_2()
                                        .text_size(px(11.5))
                                        .text_color(FG_MUTED)
                                        .child(
                                            div()
                                                .mb_1()
                                                .font_weight(gpui::FontWeight::MEDIUM)
                                                .text_color(FG_SECONDARY)
                                                .child(format!("Skipped {}", import.skipped.len())),
                                        )
                                        .children(import.skipped.iter().map(|reason| {
                                            div()
                                                .truncate()
                                                .font_family(cellar_desktop_gpui::theme::mono_font())
                                                .child(reason.clone())
                                        })),
                                )
                            }),
                                )
                            }),
                    )
                    .child(
                        div()
                            .h(px(44.))
                            .flex_shrink_0()
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap_3()
                            .border_t_1()
                            .border_color(BORDER)
                            .bg(PANEL_MUTED)
                            .px_3()
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .text_color(FG_MUTED)
                                    .child(if import.candidates.is_empty() {
                                        String::new()
                                    } else {
                                        format!("{selected} of {} selected", import.candidates.len())
                                    }),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .when_some(import.error.as_ref().filter(|_| !import.candidates.is_empty()), |actions, error| {
                                        actions.child(div().text_size(px(12.)).text_color(WARN).child(error.clone()))
                                    })
                                    .child(
                                        import_button("cancel-connection-import", "", "Cancel", false, true)
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                this.connection_import = None;
                                                cx.notify();
                                            })),
                                    )
                                    .child(
                                        import_button(
                                            "confirm-connection-import",
                                            "icons/plus.svg",
                                            if import.importing { "Importing…" } else { "Import" },
                                            true,
                                            can_import,
                                        )
                                        .when(!import.importing && selected > 0, |button| {
                                            button.child(selected.to_string())
                                        })
                                        .when(can_import, |element| {
                                            element.on_click(cx.listener(|this, _, _, cx| {
                                                this.import_selected_connections(cx)
                                            }))
                                        }),
                                    ),
                            ),
                    ),
            )
            .into_any_element()
    }
}

fn engine_badge(engine: cellar_core::driver::Engine) -> AnyElement {
    let color = super::shell_widgets::engine_color(engine);
    div()
        .size(px(20.))
        .flex_shrink_0()
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(3.))
        .border_1()
        .border_color(gpui::Rgba { a: 0.36, ..color })
        .bg(gpui::Rgba { a: 0.14, ..color })
        .text_color(color)
        .child(
            Icon::empty()
                .path(SharedString::from(format!(
                    "engines/{}.svg",
                    engine.as_str()
                )))
                .size(px(16.)),
        )
        .into_any_element()
}

fn import_row_input(state: &Entity<InputState>, width: f32, enabled: bool) -> AnyElement {
    div()
        .w(px(width))
        .h(px(24.))
        .flex_shrink_0()
        .rounded(px(4.))
        .border_1()
        .border_color(BORDER)
        .bg(INSET)
        .px_2()
        .font_family(cellar_desktop_gpui::theme::mono_font())
        .opacity(if enabled { 1. } else { 0.4 })
        .child(compact_input(state).disabled(!enabled))
        .into_any_element()
}

fn import_button(
    id: &'static str,
    icon: &'static str,
    label: &'static str,
    primary: bool,
    enabled: bool,
) -> gpui::Stateful<gpui::Div> {
    div()
        .id(id)
        .h(px(26.))
        .flex()
        .items_center()
        .gap(px(5.))
        .rounded(px(4.))
        .border_1()
        .border_color(if primary { accent(0.) } else { BORDER.rgba() })
        .bg(if primary { ACCENT.rgba() } else { accent(0.) })
        .px(px(10.))
        .font_weight(gpui::FontWeight::MEDIUM)
        .text_color(if primary { ACCENT_FG } else { FG_SECONDARY })
        .opacity(if enabled { 1. } else { 0.4 })
        .when(enabled, |button| button.tab_index(0).cursor_pointer())
        .when(enabled, |button| {
            button.hover(|style| {
                if primary {
                    style.bg(cellar_desktop_gpui::theme::hover_bright(ACCENT.rgba()))
                } else {
                    style
                        .bg(PANEL_RAISED)
                        .border_color(BORDER_STRONG)
                        .text_color(FG)
                }
            })
        })
        .when(!icon.is_empty(), |button| {
            button.child(Icon::empty().path(icon).size(px(11.)))
        })
        .child(label)
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::candidate_state;

    #[test]
    fn existing_connections_are_selected_and_marked_for_overwrite() {
        let existing = HashSet::from(["existing".to_string()]);
        assert_eq!(candidate_state(&existing, "existing"), (true, true));
        assert_eq!(candidate_state(&existing, "new"), (true, false));
    }
}
