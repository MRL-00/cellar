use cellar_sql::Dialect;
use gpui::{div, prelude::*, AnyElement, ClipboardItem, Context, Pixels, Point, SharedString};
use gpui_component::Icon;

use super::CellarApp;
use cellar_desktop_gpui::{
    model::{ConnectionState, ErDiagramTarget, QueryTarget, SchemaCompareSource, TableTarget},
    theme::{accent_soft, ui_px, ACCENT, BORDER_STRONG, FG_MUTED, FG_SECONDARY, PANEL_MUTED},
};

pub(super) struct ConnectionMenu {
    pub(super) connection_id: String,
    pub(super) position: Point<Pixels>,
    pub(super) show_folders: bool,
}

pub(super) struct TableMenu {
    pub(super) target: TableTarget,
    pub(super) position: Point<Pixels>,
}

pub(super) struct TabMenu {
    pub(super) tab_id: u64,
    pub(super) position: Point<Pixels>,
}

pub(super) struct SchemaMenu {
    pub(super) connection_id: String,
    pub(super) database: String,
    pub(super) schema: Option<String>,
    pub(super) position: Point<Pixels>,
}

impl CellarApp {
    pub(super) fn dismiss_context_menus(&mut self) -> bool {
        let dismissed = self.connection_menu.is_some()
            || self.table_menu.is_some()
            || self.schema_menu.is_some()
            || self.tab_menu.is_some()
            || self.sidebar_menu.is_some()
            || self.folder_menu.is_some()
            || self.query_database_menu.is_some()
            || self.table_preset_menu.is_some()
            || self.bottom_export_menu;
        self.connection_menu = None;
        self.table_menu = None;
        self.schema_menu = None;
        self.tab_menu = None;
        self.sidebar_menu = None;
        self.folder_menu = None;
        self.query_database_menu = None;
        self.table_preset_menu = None;
        self.bottom_export_menu = false;
        dismissed
    }

    pub(super) fn schema_context_menu(&self, cx: &mut Context<Self>) -> AnyElement {
        let menu = self
            .schema_menu
            .as_ref()
            .expect("schema menu requires state");
        let connection_id = menu.connection_id.clone();
        let database_name = menu.database.clone();
        let schema_name = menu.schema.clone();
        let visibility = self
            .schema_visibility
            .get(&Self::schema_visibility_key(&connection_id, &database_name))
            .cloned()
            .unwrap_or_default();
        let show_empty = visibility.show_hidden;
        let schema_hidden = schema_name
            .as_ref()
            .is_some_and(|schema| visibility.hidden.contains(schema));
        let is_database = schema_name.is_none();
        let query = QueryTarget {
            connection_id: connection_id.clone(),
            database: database_name.clone(),
        };
        let diagram = ErDiagramTarget {
            connection_id: connection_id.clone(),
            database: database_name.clone(),
            schemas: schema_name.clone().map(|schema| vec![schema]),
        };
        let database_diagram = diagram.clone();
        let compare = menu
            .schema
            .clone()
            .or_else(|| {
                self.model
                    .databases(&menu.connection_id)
                    .iter()
                    .find(|database| database.name == menu.database)
                    .and_then(|database| database.schemas.first())
                    .map(|schema| schema.name.clone())
            })
            .map(|schema| SchemaCompareSource::Live {
                connection_id: menu.connection_id.clone(),
                database: menu.database.clone(),
                schema,
                label: None,
            });
        let database_compare = compare.clone();
        div()
            .id("schema-menu-backdrop")
            .absolute()
            .inset_0()
            .on_mouse_down(
                gpui::MouseButton::Left,
                cx.listener(|this, _, _, cx| {
                    this.schema_menu = None;
                    cx.notify();
                }),
            )
            .child(
                div()
                    .id("schema-context-menu")
                    .absolute()
                    .left(menu.position.x)
                    .top(menu.position.y)
                    .min_w(ui_px(176.))
                    .py_1()
                    .rounded(ui_px(6.))
                    .border_1()
                    .border_color(BORDER_STRONG)
                    .bg(PANEL_MUTED)
                    .shadow_lg()
                    .on_mouse_down(gpui::MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .child(
                        menu_item("schema-menu-query", "icons/terminal.svg", "New SQL query")
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.schema_menu = None;
                                this.open_query(query.clone(), String::new(), window, cx);
                            })),
                    )
                    .when(is_database, |element| {
                        let refresh = connection_id.clone();
                        let visibility_connection = connection_id.clone();
                        let visibility_database = database_name.clone();
                        let empty_connection = connection_id.clone();
                        let empty_database = database_name.clone();
                        let copy_name = database_name.clone();
                        element
                            .child(
                                menu_item("schema-menu-er", "icons/diagram.svg", "Open ER diagram")
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        this.schema_menu = None;
                                        this.open_er_diagram(database_diagram.clone(), cx);
                                    })),
                            )
                            .child(
                                menu_item(
                                    "schema-menu-refresh",
                                    "icons/history.svg",
                                    "Refresh schemas",
                                )
                                .on_click(cx.listener(
                                    move |this, _, window, cx| {
                                        this.schema_menu = None;
                                        this.refresh_schema(refresh.clone(), window, cx);
                                    },
                                )),
                            )
                            .child(
                                menu_item(
                                    "schema-menu-compare",
                                    "icons/diff.svg",
                                    "Compare schema…",
                                )
                                .on_click(cx.listener(
                                    move |this, _, _, cx| {
                                        this.schema_menu = None;
                                        this.open_schema_compare_dialog(
                                            database_compare.clone(),
                                            cx,
                                        );
                                    },
                                )),
                            )
                            .child(
                                menu_item(
                                    "schema-menu-visibility",
                                    "icons/eye.svg",
                                    "Choose visible schemas…",
                                )
                                .on_click(cx.listener(
                                    move |this, _, window, cx| {
                                        this.schema_menu = None;
                                        this.open_schema_visibility(
                                            visibility_connection.clone(),
                                            visibility_database.clone(),
                                            window,
                                            cx,
                                        );
                                    },
                                )),
                            )
                            .child(
                                menu_item(
                                    "schema-menu-empty",
                                    if show_empty {
                                        "icons/eye-off.svg"
                                    } else {
                                        "icons/eye.svg"
                                    },
                                    if show_empty {
                                        "Hide empty schemas"
                                    } else {
                                        "Show empty schemas"
                                    },
                                )
                                .on_click(cx.listener(
                                    move |this, _, _, cx| {
                                        this.schema_menu = None;
                                        this.set_empty_schemas_visible(
                                            &empty_connection,
                                            &empty_database,
                                            !show_empty,
                                            cx,
                                        );
                                    },
                                )),
                            )
                            .child(
                                menu_item("schema-menu-copy-name", "icons/copy.svg", "Copy name")
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        this.schema_menu = None;
                                        cx.write_to_clipboard(ClipboardItem::new_string(
                                            copy_name.clone(),
                                        ));
                                        cx.notify();
                                    })),
                            )
                    })
                    .when_some(schema_name, |element, schema| {
                        let hide_connection = connection_id.clone();
                        let hide_database = database_name.clone();
                        let hide_schema = schema.clone();
                        let qualified = Dialect::Postgres.quote_qualified(&database_name, &schema);
                        element
                            .when_some(compare, |element, preset| {
                                element.child(
                                    menu_item(
                                        "schema-menu-compare",
                                        "icons/diff.svg",
                                        "Compare schema…",
                                    )
                                    .on_click(cx.listener(
                                        move |this, _, _, cx| {
                                            this.schema_menu = None;
                                            this.open_schema_compare_dialog(
                                                Some(preset.clone()),
                                                cx,
                                            );
                                        },
                                    )),
                                )
                            })
                            .child(
                                menu_item("schema-menu-er", "icons/diagram.svg", "Open ER diagram")
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        this.schema_menu = None;
                                        this.open_er_diagram(diagram.clone(), cx);
                                    })),
                            )
                            .child(
                                menu_item(
                                    "schema-menu-hide",
                                    if schema_hidden {
                                        "icons/eye.svg"
                                    } else {
                                        "icons/eye-off.svg"
                                    },
                                    if schema_hidden {
                                        "Show in sidebar"
                                    } else {
                                        "Hide from sidebar"
                                    },
                                )
                                .on_click(cx.listener(
                                    move |this, _, _, cx| {
                                        this.schema_menu = None;
                                        this.set_schema_hidden(
                                            &hide_connection,
                                            &hide_database,
                                            &hide_schema,
                                            !schema_hidden,
                                            cx,
                                        );
                                    },
                                )),
                            )
                            .child(
                                menu_item(
                                    "schema-menu-copy-qualified",
                                    "icons/copy.svg",
                                    "Copy qualified name",
                                )
                                .on_click(cx.listener(
                                    move |this, _, _, cx| {
                                        this.schema_menu = None;
                                        cx.write_to_clipboard(ClipboardItem::new_string(
                                            qualified.clone(),
                                        ));
                                        cx.notify();
                                    },
                                )),
                            )
                            .child(
                                menu_item("schema-menu-copy-name", "icons/copy.svg", "Copy name")
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        this.schema_menu = None;
                                        cx.write_to_clipboard(ClipboardItem::new_string(
                                            schema.clone(),
                                        ));
                                        cx.notify();
                                    })),
                            )
                    }),
            )
            .into_any_element()
    }

    pub(super) fn connection_context_menu(&self, cx: &mut Context<Self>) -> AnyElement {
        let menu = self
            .connection_menu
            .as_ref()
            .expect("connection menu overlay requires state");
        let Some(config) = self
            .model
            .connections()
            .iter()
            .find(|config| config.id == menu.connection_id)
            .cloned()
        else {
            return div().into_any_element();
        };
        let position = menu.position;
        let folders = self
            .sidebar_layout
            .iter()
            .filter_map(|item| match item {
                super::sidebar_layout::SidebarItem::Folder { id, name, .. } => {
                    Some((id.clone(), name.clone()))
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        let current_folder = self.sidebar_layout.iter().find_map(|item| match item {
            super::sidebar_layout::SidebarItem::Folder { id, children, .. }
                if children.contains(&config.id) =>
            {
                Some(id.clone())
            }
            _ => None,
        });

        if menu.show_folders {
            let connection_id = config.id.clone();
            return menu_backdrop("connection-menu-backdrop", cx)
                .child(
                    context_menu_at("connection-context-menu", position).children(
                        folders
                            .into_iter()
                            .filter(|(id, _)| Some(id) != current_folder.as_ref())
                            .map(|(id, name)| {
                                let move_connection = connection_id.clone();
                                menu_item(
                                    format!("connection-move:{id}"),
                                    "icons/folder.svg",
                                    format!("Move to \"{name}\""),
                                )
                                .on_click(cx.listener(
                                    move |this, _, _, cx| {
                                        this.move_connection_to_folder(
                                            move_connection.clone(),
                                            Some(id.clone()),
                                            cx,
                                        );
                                    },
                                ))
                            }),
                    ),
                )
                .into_any_element();
        }

        let connected = matches!(
            self.model.connection_state(&config.id),
            ConnectionState::Connected
        );
        let connecting = matches!(
            self.model.connection_state(&config.id),
            ConnectionState::Connecting | ConnectionState::Disconnecting
        );
        let retry = matches!(
            self.model.connection_state(&config.id),
            ConnectionState::Connected | ConnectionState::Error(_)
        );
        let query_target = QueryTarget {
            connection_id: config.id.clone(),
            database: super::query_editor::preferred_database(
                self.model.databases(&config.id),
                &config.database,
            ),
        };
        let edit = config.clone();
        let duplicate = config.clone();
        let remove = config.clone();
        let toggle_id = config.id.clone();
        let reconnect_id = config.id.clone();
        let move_new = config.id.clone();
        let remove_from = config.id.clone();
        let has_other_folders = folders
            .iter()
            .any(|(id, _)| Some(id) != current_folder.as_ref());

        menu_backdrop("connection-menu-backdrop", cx)
            .child(
                context_menu_at("connection-context-menu", position)
                    .child(
                        menu_item(
                            "connection-menu-query",
                            "icons/terminal.svg",
                            "New SQL query",
                        )
                        .on_click(cx.listener(
                            move |this, _, window, cx| {
                                this.connection_menu = None;
                                this.open_query(query_target.clone(), String::new(), window, cx);
                            },
                        )),
                    )
                    .child(
                        menu_item("connection-menu-edit", "icons/edit.svg", "Edit…").on_click(
                            cx.listener(move |this, _, window, cx| {
                                this.connection_menu = None;
                                this.open_connection_editor(Some(edit.clone()), window, cx);
                            }),
                        ),
                    )
                    .child(
                        menu_item("connection-menu-duplicate", "icons/copy.svg", "Duplicate")
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.connection_menu = None;
                                this.duplicate_connection(duplicate.clone(), window, cx);
                            })),
                    )
                    .when(has_other_folders, |element| {
                        element.child(
                            menu_item(
                                "connection-menu-move",
                                "icons/folder.svg",
                                "Move to folder…",
                            )
                            .on_click(cx.listener(|this, _, _, cx| {
                                if let Some(menu) = this.connection_menu.as_mut() {
                                    menu.show_folders = true;
                                }
                                cx.notify();
                            })),
                        )
                    })
                    .child(
                        menu_item(
                            "connection-menu-new-folder",
                            "icons/folder-plus.svg",
                            "Move to new folder",
                        )
                        .on_click(cx.listener(
                            move |this, _, window, cx| {
                                this.start_new_sidebar_folder(Some(move_new.clone()), window, cx);
                            },
                        )),
                    )
                    .when(current_folder.is_some(), |element| {
                        element.child(
                            menu_item(
                                "connection-menu-remove-folder",
                                "icons/folder-open.svg",
                                "Remove from folder",
                            )
                            .on_click(cx.listener(
                                move |this, _, _, cx| {
                                    this.move_connection_to_folder(remove_from.clone(), None, cx);
                                },
                            )),
                        )
                    })
                    .when(retry, |element| {
                        element.child(
                            menu_item(
                                "connection-menu-reconnect",
                                "icons/history.svg",
                                if matches!(
                                    self.model.connection_state(&config.id),
                                    ConnectionState::Error(_)
                                ) {
                                    "Retry connection"
                                } else {
                                    "Reconnect"
                                },
                            )
                            .on_click(cx.listener(
                                move |this, _, window, cx| {
                                    this.connection_menu = None;
                                    this.reconnect(reconnect_id.clone(), window, cx);
                                },
                            )),
                        )
                    })
                    .child(
                        menu_item(
                            "connection-menu-toggle",
                            "icons/power.svg",
                            if connecting {
                                "Connecting..."
                            } else if connected {
                                "Disconnect"
                            } else {
                                "Connect"
                            },
                        )
                        .opacity(if connecting { 0.45 } else { 1. })
                        .when(!connecting, |element| {
                            element.on_click(cx.listener(move |this, _, window, cx| {
                                this.connection_menu = None;
                                if connected {
                                    this.disconnect(toggle_id.clone(), window, cx);
                                } else {
                                    this.model.select_connection(&toggle_id);
                                    this.start_connect(toggle_id.clone(), window, cx);
                                }
                            }))
                        }),
                    )
                    .child(
                        menu_item("connection-menu-remove", "icons/trash.svg", "Remove")
                            .text_color(cellar_desktop_gpui::theme::PROD)
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.connection_menu = None;
                                this.confirm_delete_connection(remove.clone(), window, cx);
                            })),
                    ),
            )
            .into_any_element()
    }

    pub(super) fn table_context_menu(&self, cx: &mut Context<Self>) -> AnyElement {
        let menu = self
            .table_menu
            .as_ref()
            .expect("table menu overlay requires state");
        let open = menu.target.clone();
        let query = QueryTarget {
            connection_id: menu.target.connection_id.clone(),
            database: menu.target.database.clone(),
        };
        let sql = format!(
            "SELECT *\nFROM {}\nLIMIT 100;",
            Dialect::Postgres.quote_qualified(&menu.target.schema, &menu.target.table)
        );
        let is_view = self
            .model
            .databases(&menu.target.connection_id)
            .iter()
            .find(|database| database.name == menu.target.database)
            .and_then(|database| {
                database
                    .schemas
                    .iter()
                    .find(|schema| schema.name == menu.target.schema)
            })
            .is_some_and(|schema| {
                schema
                    .views
                    .iter()
                    .any(|view| view.name == menu.target.table)
            });
        let import = menu.target.clone();
        let usage = menu.target.clone();
        let qualified = Dialect::Postgres.quote_qualified(&menu.target.schema, &menu.target.table);
        let name = menu.target.table.clone();
        div()
            .id("table-menu-backdrop")
            .absolute()
            .inset_0()
            .on_mouse_down(
                gpui::MouseButton::Left,
                cx.listener(|this, _, _, cx| {
                    this.table_menu = None;
                    cx.notify();
                }),
            )
            .child(
                div()
                    .id("table-context-menu")
                    .absolute()
                    .left(menu.position.x)
                    .top(menu.position.y)
                    .min_w(ui_px(176.))
                    .py_1()
                    .rounded(ui_px(6.))
                    .border_1()
                    .border_color(BORDER_STRONG)
                    .bg(PANEL_MUTED)
                    .shadow_lg()
                    .on_mouse_down(gpui::MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .child(
                        menu_item(
                            "table-menu-open",
                            if is_view {
                                "icons/tree.svg"
                            } else {
                                "icons/table.svg"
                            },
                            "Open",
                        )
                        .on_click(cx.listener(
                            move |this, _, window, cx| {
                                this.table_menu = None;
                                this.open_table(open.clone(), window, cx);
                            },
                        )),
                    )
                    .child(
                        menu_item("table-menu-query", "icons/terminal.svg", "Query SELECT *")
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.table_menu = None;
                                this.open_query(query.clone(), sql.clone(), window, cx);
                            })),
                    )
                    .child(
                        menu_item("table-menu-import", "icons/upload.svg", "Import data…")
                            .opacity(if is_view { 0.45 } else { 1. })
                            .when(is_view, |element| {
                                element
                                    .cursor_default()
                                    .hover(|style| style.bg(PANEL_MUTED))
                            })
                            .when(!is_view, |element| {
                                element.on_click(cx.listener(move |this, _, window, cx| {
                                    this.table_menu = None;
                                    this.open_table(import.clone(), window, cx);
                                    if let Some(tab_id) = this.model.active_tab().map(|tab| tab.id)
                                    {
                                        this.open_csv_import(tab_id, cx);
                                    }
                                }))
                            }),
                    )
                    .child(
                        menu_item("table-menu-usages", "icons/search.svg", "Find Usages").on_click(
                            cx.listener(move |this, _, _, cx| {
                                this.table_menu = None;
                                this.start_find_usages(usage.clone(), false, cx);
                            }),
                        ),
                    )
                    .child(
                        menu_item(
                            "table-menu-copy-qualified",
                            "icons/copy.svg",
                            "Copy qualified name",
                        )
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.table_menu = None;
                            cx.write_to_clipboard(ClipboardItem::new_string(qualified.clone()));
                            cx.notify();
                        })),
                    )
                    .child(
                        menu_item("table-menu-copy-name", "icons/copy.svg", "Copy name").on_click(
                            cx.listener(move |this, _, _, cx| {
                                this.table_menu = None;
                                cx.write_to_clipboard(ClipboardItem::new_string(name.clone()));
                                cx.notify();
                            }),
                        ),
                    ),
            )
            .into_any_element()
    }
}

fn menu_backdrop(id: &'static str, cx: &mut Context<CellarApp>) -> gpui::Stateful<gpui::Div> {
    div().id(id).absolute().inset_0().on_mouse_down(
        gpui::MouseButton::Left,
        cx.listener(|this, _, _, cx| {
            this.connection_menu = None;
            cx.notify();
        }),
    )
}

fn context_menu_at(id: &'static str, position: Point<Pixels>) -> gpui::Stateful<gpui::Div> {
    div()
        .id(id)
        .tab_group()
        .absolute()
        .left(position.x)
        .top(position.y)
        .min_w(ui_px(176.))
        .py(ui_px(4.))
        .rounded(ui_px(6.))
        .border_1()
        .border_color(BORDER_STRONG)
        .bg(PANEL_MUTED)
        .shadow_lg()
        .on_mouse_down(gpui::MouseButton::Left, |_, _, cx| cx.stop_propagation())
}

pub(super) fn menu_item(
    id: impl Into<SharedString>,
    icon: &'static str,
    label: impl Into<SharedString>,
) -> gpui::Stateful<gpui::Div> {
    let id = id.into();
    let label = label.into();
    div()
        .id(id)
        .tab_index(0)
        .cursor_pointer()
        .h(ui_px(28.))
        .flex()
        .items_center()
        .gap(ui_px(8.))
        .px(ui_px(10.))
        .text_color(FG_SECONDARY)
        .hover(|style| style.bg(accent_soft()).text_color(ACCENT))
        .child(
            div()
                .size(ui_px(14.))
                .flex_shrink_0()
                .flex()
                .items_center()
                .justify_center()
                .text_color(FG_MUTED)
                .child(Icon::empty().path(icon).size(ui_px(12.))),
        )
        .child(label)
}
