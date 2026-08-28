use cellar_core::schema::{Schema, Table, View};
use gpui::{
    div, prelude::*, AnyElement, Context, KeyDownEvent, MouseButton, MouseDownEvent, SharedString,
};
use gpui_component::Icon;

use super::CellarApp;
use cellar_desktop_gpui::{
    model::{ConnectionState, SchemaNode, TabKind, TableTarget},
    theme::{accent_soft, ui_px, ACCENT, FG_MUTED, FG_SECONDARY, PANEL_MUTED, WARN},
};

impl CellarApp {
    pub(super) fn schema_tree(
        &self,
        connection_id: Option<&str>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let Some(connection_id) = connection_id else {
            return div()
                .p_3()
                .text_color(FG_MUTED)
                .child("Select a connection")
                .into_any_element();
        };
        let databases = self.model.databases(connection_id);
        let connection_state = self.model.connection_state(connection_id).clone();
        div()
            .id("schema-tree")
            .tab_group()
            .on_key_down(
                |event: &KeyDownEvent, window, cx| match event.keystroke.key.as_str() {
                    "down" => {
                        window.focus_next();
                        cx.stop_propagation();
                    }
                    "up" => {
                        window.focus_prev();
                        cx.stop_propagation();
                    }
                    _ => {}
                },
            )
            .when(databases.is_empty(), |element| {
                element.child(match connection_state {
                    ConnectionState::Connecting | ConnectionState::Disconnecting => div()
                        .pl(ui_px(32.))
                        .py_1()
                        .text_size(ui_px(14.))
                        .text_color(FG_MUTED)
                        .child("loading schemas…")
                        .into_any_element(),
                    ConnectionState::Error(_) => {
                        let failed_click = connection_id.to_owned();
                        let failed_key = connection_id.to_owned();
                        tree_row(32.)
                            .id("schema-connection-failed")
                            .text_color(WARN)
                            .child("Connection failed")
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.show_connection_error(&failed_click, Some(window), cx)
                            }))
                            .on_key_down(cx.listener(
                                move |this, event: &KeyDownEvent, window, cx| {
                                    if activate_key(event) {
                                        this.show_connection_error(&failed_key, Some(window), cx);
                                        cx.stop_propagation();
                                    }
                                },
                            ))
                            .into_any_element()
                    }
                    _ => div()
                        .pl(ui_px(32.))
                        .py_1()
                        .text_size(ui_px(14.))
                        .text_color(FG_MUTED)
                        .child("Connect to browse schemas")
                        .into_any_element(),
                })
            })
            .children(databases.iter().map(|database| {
                let visible =
                    self.visible_schemas(connection_id, &database.name, &database.schemas);
                let visible_count = visible.len();
                let node = SchemaNode::Database {
                    connection_id: connection_id.to_owned(),
                    database: database.name.clone(),
                };
                let expanded = self.model.node_expanded(&node);
                let toggle = node.clone();
                let keyboard_toggle = node.clone();
                let visibility_connection = connection_id.to_owned();
                let visibility_database = database.name.clone();
                let menu_connection = connection_id.to_owned();
                let menu_database = database.name.clone();
                div()
                    .child(
                        tree_row(18.)
                            .id(SharedString::from(format!(
                                "database:{connection_id}:{}",
                                database.name
                            )))
                            .group("database-row")
                            .child(if visible_count == 0 {
                                empty_twisty().into_any_element()
                            } else {
                                twisty(expanded).into_any_element()
                            })
                            .child(icon("icons/database.svg", 12.))
                            .child(label(database.name.clone()))
                            .child(meta(if database.schemas.is_empty() {
                                "—".into()
                            } else if visible_count == database.schemas.len() {
                                format!("{} schemas", database.schemas.len())
                            } else {
                                format!("{visible_count}/{} schemas", database.schemas.len())
                            }))
                            .when(!database.schemas.is_empty(), |element| {
                                element.child(
                                    div()
                                        .id(SharedString::from(format!(
                                            "schema-visibility:{connection_id}:{}",
                                            database.name
                                        )))
                                        .cursor_pointer()
                                        .size(ui_px(18.))
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .invisible()
                                        .group_hover("database-row", |style| style.visible())
                                        .text_color(FG_MUTED)
                                        .child(Icon::empty().path("icons/eye.svg").size(ui_px(11.)))
                                        .on_click(cx.listener(move |this, _, window, cx| {
                                            cx.stop_propagation();
                                            this.open_schema_visibility(
                                                visibility_connection.clone(),
                                                visibility_database.clone(),
                                                window,
                                                cx,
                                            );
                                        })),
                                )
                            })
                            .on_click(cx.listener(move |this, _, _, cx| {
                                if visible_count > 0 {
                                    this.model.toggle_node(toggle.clone());
                                    cx.notify();
                                }
                            }))
                            .on_key_down(cx.listener(move |this, event: &KeyDownEvent, _, cx| {
                                if visible_count > 0 && toggle_key(event, expanded) {
                                    this.model.toggle_node(keyboard_toggle.clone());
                                    cx.notify();
                                    cx.stop_propagation();
                                }
                            }))
                            .on_mouse_down(
                                MouseButton::Right,
                                cx.listener(move |this, event: &MouseDownEvent, _, cx| {
                                    cx.stop_propagation();
                                    this.schema_menu = Some(super::context_menu::SchemaMenu {
                                        connection_id: menu_connection.clone(),
                                        database: menu_database.clone(),
                                        schema: None,
                                        position: event.position,
                                    });
                                    cx.notify();
                                }),
                            ),
                    )
                    .when(expanded, |element| {
                        element.children(visible.into_iter().map(|schema| {
                            self.schema_node(connection_id, &database.name, schema, cx)
                        }))
                    })
            }))
            .into_any_element()
    }

    fn schema_node(
        &self,
        connection_id: &str,
        database: &str,
        schema: &Schema,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let node = SchemaNode::Schema {
            connection_id: connection_id.to_owned(),
            database: database.to_owned(),
            schema: schema.name.clone(),
        };
        let expanded = self.model.node_expanded(&node);
        let toggle = node.clone();
        let keyboard_toggle = node.clone();
        let menu_connection = connection_id.to_owned();
        let menu_database = database.to_owned();
        let menu_schema = schema.name.clone();
        div()
            .child(
                tree_row(30.)
                    .id(SharedString::from(format!(
                        "schema:{connection_id}:{database}:{}",
                        schema.name
                    )))
                    .child(twisty(expanded))
                    .child(icon("icons/schema.svg", 12.))
                    .child(label(schema.name.clone()))
                    .child(meta(schema.tables.len().to_string()))
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
                    }))
                    .on_mouse_down(
                        MouseButton::Right,
                        cx.listener(move |this, event: &MouseDownEvent, _, cx| {
                            cx.stop_propagation();
                            this.schema_menu = Some(super::context_menu::SchemaMenu {
                                connection_id: menu_connection.clone(),
                                database: menu_database.clone(),
                                schema: Some(menu_schema.clone()),
                                position: event.position,
                            });
                            cx.notify();
                        }),
                    ),
            )
            .when(expanded && !schema.tables.is_empty(), |element| {
                element.child(self.table_group(connection_id, database, schema, cx))
            })
            .when(expanded && !schema.views.is_empty(), |element| {
                element.child(self.view_group(connection_id, database, schema, cx))
            })
            .into_any_element()
    }

    fn table_group(
        &self,
        connection_id: &str,
        database: &str,
        schema: &Schema,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let node = group_node(connection_id, database, &schema.name, "tables");
        let expanded = self.model.node_expanded(&node);
        let toggle = node.clone();
        let keyboard_toggle = node.clone();
        div()
            .child(
                tree_row(42.)
                    .id(SharedString::from(format!(
                        "tables:{connection_id}:{database}:{}",
                        schema.name
                    )))
                    .child(twisty(expanded))
                    .child(folder_icon(expanded))
                    .child(label("tables"))
                    .child(meta(schema.tables.len().to_string()))
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
            .when(expanded, |element| {
                element.children(
                    schema.tables.iter().map(|table| {
                        self.table_node(connection_id, database, &schema.name, table, cx)
                    }),
                )
            })
            .into_any_element()
    }

    fn table_node(
        &self,
        connection_id: &str,
        database: &str,
        schema: &str,
        table: &Table,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let target = TableTarget {
            connection_id: connection_id.to_owned(),
            database: database.to_owned(),
            schema: schema.to_owned(),
            table: table.name.clone(),
        };
        let active = self.model.active_tab().is_some_and(
            |tab| matches!(&tab.kind, TabKind::Table { target: open, .. } if open == &target),
        );
        let open = target.clone();
        let keyboard_open = target.clone();
        let menu_target = target.clone();
        tree_row(66.)
            .id(SharedString::from(format!(
                "table:{connection_id}:{database}:{schema}:{}",
                table.name
            )))
            .when(active, |element| {
                element
                    .bg(accent_soft())
                    .text_color(ACCENT)
                    .font_weight(gpui::FontWeight::MEDIUM)
            })
            .child(empty_twisty())
            .child(icon("icons/table.svg", 11.))
            .child(label(table.name.clone()))
            .when_some(table.row_count, |element, count| {
                element.child(meta(format_row_count(count)))
            })
            .when(!table.foreign_keys.is_empty(), |element| {
                element.child(
                    div()
                        .ml_1()
                        .rounded(ui_px(3.))
                        .bg(PANEL_MUTED)
                        .px_1()
                        .text_size(ui_px(10.))
                        .text_color(FG_MUTED)
                        .child(format!("fk·{}", table.foreign_keys.len())),
                )
            })
            .on_mouse_down(
                MouseButton::Right,
                cx.listener(move |this, event: &MouseDownEvent, _, cx| {
                    cx.stop_propagation();
                    this.table_menu = Some(super::context_menu::TableMenu {
                        target: menu_target.clone(),
                        position: event.position,
                    });
                    cx.notify();
                }),
            )
            .on_click(cx.listener(move |this, _, window, cx| {
                this.open_table(open.clone(), window, cx);
            }))
            .on_key_down(cx.listener(move |this, event: &KeyDownEvent, window, cx| {
                if activate_key(event) {
                    this.open_table(keyboard_open.clone(), window, cx);
                    cx.stop_propagation();
                }
            }))
            .into_any_element()
    }

    fn view_group(
        &self,
        connection_id: &str,
        database: &str,
        schema: &Schema,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let node = group_node(connection_id, database, &schema.name, "views");
        let expanded = self.model.node_expanded(&node);
        let toggle = node.clone();
        let keyboard_toggle = node.clone();
        div()
            .child(
                tree_row(42.)
                    .id(SharedString::from(format!(
                        "views:{connection_id}:{database}:{}",
                        schema.name
                    )))
                    .child(twisty(expanded))
                    .child(folder_icon(expanded))
                    .child(label("views"))
                    .child(meta(schema.views.len().to_string()))
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
            .when(expanded, |element| {
                element.children(
                    schema.views.iter().map(|view| {
                        self.view_node(connection_id, database, &schema.name, view, cx)
                    }),
                )
            })
            .into_any_element()
    }

    fn view_node(
        &self,
        connection_id: &str,
        database: &str,
        schema: &str,
        view: &View,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let target = TableTarget {
            connection_id: connection_id.to_owned(),
            database: database.to_owned(),
            schema: schema.to_owned(),
            table: view.name.clone(),
        };
        let keyboard_target = target.clone();
        let menu_target = target.clone();
        tree_row(66.)
            .id(SharedString::from(format!(
                "view:{connection_id}:{database}:{schema}:{}",
                view.name
            )))
            .child(empty_twisty())
            .child(icon("icons/tree.svg", 11.))
            .child(label(view.name.clone()))
            .on_mouse_down(
                MouseButton::Right,
                cx.listener(move |this, event: &MouseDownEvent, _, cx| {
                    cx.stop_propagation();
                    this.table_menu = Some(super::context_menu::TableMenu {
                        target: menu_target.clone(),
                        position: event.position,
                    });
                    cx.notify();
                }),
            )
            .on_click(cx.listener(move |this, _, window, cx| {
                this.open_table(target.clone(), window, cx);
            }))
            .on_key_down(cx.listener(move |this, event: &KeyDownEvent, window, cx| {
                if activate_key(event) {
                    this.open_table(keyboard_target.clone(), window, cx);
                    cx.stop_propagation();
                }
            }))
            .into_any_element()
    }
}

fn group_node(connection_id: &str, database: &str, schema: &str, kind: &'static str) -> SchemaNode {
    SchemaNode::Group {
        connection_id: connection_id.to_owned(),
        database: database.to_owned(),
        schema: schema.to_owned(),
        kind,
    }
}

fn tree_row(padding: f32) -> gpui::Div {
    div()
        .cursor_pointer()
        .h(ui_px(22.))
        .rounded(ui_px(3.))
        .flex()
        .items_center()
        .gap_1()
        .pl(ui_px(padding))
        .pr(ui_px(6.))
        .text_size(ui_px(14.))
        .text_color(FG_SECONDARY)
        .tab_index(0)
        .focus(|style| style.bg(PANEL_MUTED))
        .hover(|style| style.bg(PANEL_MUTED))
}

fn activate_key(event: &KeyDownEvent) -> bool {
    matches!(event.keystroke.key.as_str(), "enter" | "space")
}

fn toggle_key(event: &KeyDownEvent, expanded: bool) -> bool {
    activate_key(event)
        || event.keystroke.key == "right" && !expanded
        || event.keystroke.key == "left" && expanded
}

fn twisty(expanded: bool) -> impl IntoElement {
    div()
        .text_color(FG_MUTED)
        .hover(|style| style.text_color(FG_SECONDARY))
        .child(icon(
            if expanded {
                "icons/chevron-down.svg"
            } else {
                "icons/chevron-right.svg"
            },
            10.,
        ))
}

fn empty_twisty() -> impl IntoElement {
    div().size(ui_px(14.)).flex_shrink_0()
}

fn folder_icon(expanded: bool) -> impl IntoElement {
    icon(
        if expanded {
            "icons/folder-open.svg"
        } else {
            "icons/folder.svg"
        },
        12.,
    )
}

fn icon(path: &'static str, size: f32) -> impl IntoElement {
    div()
        .size(ui_px(14.))
        .flex_shrink_0()
        .flex()
        .items_center()
        .justify_center()
        .child(Icon::empty().path(path).size(ui_px(size)))
}

fn label(text: impl Into<SharedString>) -> impl IntoElement {
    div()
        .min_w_0()
        .flex_1()
        .truncate()
        .line_height(ui_px(18.))
        .child(text.into())
}

fn meta(text: impl Into<SharedString>) -> impl IntoElement {
    div()
        .ml_auto()
        .flex_shrink_0()
        .pr_1()
        .text_size(ui_px(11.))
        .font_weight(gpui::FontWeight::MEDIUM)
        .text_color(FG_MUTED)
        .child(text.into())
}

fn format_row_count(count: u64) -> String {
    if count >= 1_000_000 {
        format!("{:.1}M", count as f64 / 1_000_000.)
    } else if count >= 1_000 {
        format!("{:.1}k", count as f64 / 1_000.)
    } else {
        count.to_string()
    }
}
