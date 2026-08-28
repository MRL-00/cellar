use std::{collections::HashSet, fs, path::PathBuf};

use cellar_core::driver::ConnectionConfig;
use gpui::{
    div, prelude::*, AnyElement, Context, MouseButton, MouseDownEvent, Point, Rgba, SharedString,
};
use gpui_component::{input::Input, Icon};
use serde::{Deserialize, Serialize};
use sqlx::{sqlite::SqliteConnectOptions, ConnectOptions, Connection, Row};

use super::{
    sidebar_drag::{apply_sidebar_drop, DraggedSidebarItem, SidebarDragKind, SidebarDropTarget},
    CellarApp,
};
use cellar_desktop_gpui::theme::{
    ui_px, ACCENT, BORDER, FG, FG_MUTED, FG_TERTIARY, PANEL_MUTED, PANEL_RAISED,
};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub(super) enum SidebarItem {
    Connection {
        id: String,
    },
    Folder {
        id: String,
        name: String,
        collapsed: bool,
        #[serde(default)]
        children: Vec<String>,
        color: Option<String>,
    },
}

#[derive(Default)]
pub(crate) struct SidebarLayout {
    pub(super) items: Vec<SidebarItem>,
    pub(super) shell: Option<ShellLayout>,
}

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ShellLayout {
    pub(super) panels: ShellPanels,
    pub(super) left_width: f32,
    pub(super) right_width: f32,
    pub(super) bottom_height: f32,
}

#[derive(Clone, Copy, Deserialize)]
pub(super) struct ShellPanels {
    pub(super) left: bool,
    pub(super) right: bool,
    pub(super) bottom: bool,
}

#[derive(Deserialize)]
struct PersistedLayout {
    state: PersistedState,
}

#[derive(Deserialize)]
struct PersistedState {
    items: Vec<SidebarItem>,
}

#[derive(Deserialize)]
struct PersistedShell {
    state: ShellLayout,
}

impl SidebarLayout {
    pub(crate) async fn load(connections: &[ConnectionConfig]) -> Self {
        let valid: HashSet<&str> = connections.iter().map(|c| c.id.as_str()).collect();
        let mut best = Vec::new();
        let mut best_score = 0;
        let mut best_path = None;
        for path in tauri_local_storage_paths() {
            let Some(items) = read_layout(&path).await else {
                continue;
            };
            let score = items
                .iter()
                .map(|item| match item {
                    SidebarItem::Connection { id } => usize::from(valid.contains(id.as_str())),
                    SidebarItem::Folder { children, .. } => children
                        .iter()
                        .filter(|id| valid.contains(id.as_str()))
                        .count(),
                })
                .sum();
            if score > best_score || score == best_score && prefers_dev_layout(&path) {
                best = items;
                best_score = score;
                best_path = Some(path);
            }
        }

        reconcile(&mut best, connections);
        let shell = match best_path {
            Some(path) => read_shell_layout(&path).await,
            None => None,
        };
        Self { items: best, shell }
    }
}

fn reconcile(items: &mut Vec<SidebarItem>, connections: &[ConnectionConfig]) {
    let valid = connections
        .iter()
        .map(|connection| connection.id.as_str())
        .collect::<HashSet<_>>();
    let mut seen = HashSet::new();
    items.retain_mut(|item| match item {
        SidebarItem::Connection { id } => valid.contains(id.as_str()) && seen.insert(id.clone()),
        SidebarItem::Folder { children, .. } => {
            children.retain(|id| valid.contains(id.as_str()) && seen.insert(id.clone()));
            true
        }
    });
    items.extend(
        connections
            .iter()
            .filter(|connection| !seen.contains(&connection.id))
            .map(|connection| SidebarItem::Connection {
                id: connection.id.clone(),
            }),
    );
}

pub(super) fn tauri_local_storage_paths() -> Vec<PathBuf> {
    let Some(library) = dirs::home_dir().map(|path| path.join("Library/WebKit")) else {
        return Vec::new();
    };
    ["cellar-desktop", "com.cellar.desktop"]
        .into_iter()
        .flat_map(|bundle| {
            let root = library.join(bundle).join("WebsiteData/Default");
            fs::read_dir(root)
                .into_iter()
                .flatten()
                .flatten()
                .map(|entry| {
                    let name = entry.file_name();
                    entry
                        .path()
                        .join(name)
                        .join("LocalStorage/localstorage.sqlite3")
                })
                .filter(|path| path.is_file())
                .collect::<Vec<_>>()
        })
        .collect()
}

async fn read_layout(path: &std::path::Path) -> Option<Vec<SidebarItem>> {
    let json = read_local_storage_value(path, "cellar.sidebarLayout.v1").await?;
    serde_json::from_str::<PersistedLayout>(&json)
        .ok()
        .map(|layout| layout.state.items)
}

async fn read_shell_layout(path: &std::path::Path) -> Option<ShellLayout> {
    let json = read_local_storage_value(path, "cellar.layout.v1").await?;
    serde_json::from_str::<PersistedShell>(&json)
        .ok()
        .map(|layout| layout.state)
}

pub(super) async fn read_local_storage_value(path: &std::path::Path, key: &str) -> Option<String> {
    let options = SqliteConnectOptions::new()
        .filename(&path)
        .read_only(true)
        .disable_statement_logging();
    let mut connection = sqlx::SqliteConnection::connect_with(&options).await.ok()?;
    let bytes: Vec<u8> = sqlx::query("SELECT value FROM ItemTable WHERE key = ?")
        .bind(key)
        .fetch_one(&mut connection)
        .await
        .ok()?
        .try_get(0)
        .ok()?;
    decode_webkit_value(&bytes)
}

fn decode_webkit_value(bytes: &[u8]) -> Option<String> {
    let utf16: Vec<u16> = bytes
        .chunks_exact(2)
        .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
        .collect();
    String::from_utf16(&utf16).ok()
}

pub(super) fn prefers_dev_layout(path: &std::path::Path) -> bool {
    let Some(origin) = path.ancestors().nth(2).map(|dir| dir.join("origin")) else {
        return false;
    };
    fs::read(origin).is_ok_and(|bytes| bytes.windows(3).any(|window| window == [1, 0x96, 0x05]))
}

impl CellarApp {
    pub(super) fn reconcile_sidebar_layout(&mut self) {
        reconcile(&mut self.sidebar_layout, self.model.connections());
    }

    pub(super) fn sidebar_rows(
        &self,
        active_id: Option<&str>,
        cx: &mut Context<Self>,
    ) -> Vec<AnyElement> {
        let filter = self.sidebar_filter.read(cx).value().trim().to_lowercase();
        let filtering = !filter.is_empty();
        let mut rows = Vec::new();
        for (root_index, item) in self.sidebar_layout.clone().into_iter().enumerate() {
            match item {
                SidebarItem::Connection { id } => {
                    if let Some(config) = self.model.connections().iter().find(|c| c.id == id) {
                        if filtering && !connection_matches(config, &filter) {
                            continue;
                        }
                        let drag_id = id.clone();
                        let drag_label = config.name.clone();
                        let drop_app = cx.entity().downgrade();
                        rows.push(
                            div()
                                .id(SharedString::from(format!("drag-connection:{id}")))
                                .relative()
                                .when(!filtering, |element| {
                                    element.on_drag(
                                        DraggedSidebarItem {
                                            kind: SidebarDragKind::Connection,
                                            id: drag_id,
                                            label: drag_label.into(),
                                            position: Point::default(),
                                        },
                                        |drag, position, _, cx| {
                                            cx.refresh_windows();
                                            cx.new(|_| DraggedSidebarItem {
                                                position,
                                                ..drag.clone()
                                            })
                                        },
                                    )
                                })
                                .drag_over::<DraggedSidebarItem>(|style, _, _, _| {
                                    style.border_t_2().border_color(ACCENT)
                                })
                                .on_drop(move |drag: &DraggedSidebarItem, _, cx| {
                                    drop_app
                                        .update(cx, |this, cx| {
                                            if apply_sidebar_drop(
                                                &mut this.sidebar_layout,
                                                drag,
                                                SidebarDropTarget::Root(root_index),
                                            ) {
                                                cx.notify();
                                            }
                                        })
                                        .ok();
                                })
                                .child(self.connection_row(
                                    config,
                                    active_id == Some(id.as_str()),
                                    cx,
                                ))
                                .when(active_id == Some(id.as_str()), |element| {
                                    element.child(self.schema_tree(Some(&id), cx))
                                })
                                .into_any_element(),
                        );
                    }
                }
                SidebarItem::Folder {
                    id,
                    name,
                    collapsed,
                    children,
                    color,
                } => {
                    let rename = self
                        .folder_rename
                        .as_ref()
                        .filter(|rename| rename.id == id)
                        .map(|rename| rename.input.clone());
                    let toggle_id = id.clone();
                    let menu_id = id.clone();
                    let right_menu_id = id.clone();
                    let child_count = children.len();
                    let visible_children = children
                        .iter()
                        .enumerate()
                        .filter(|(_, child_id)| {
                            self.model
                                .connections()
                                .iter()
                                .find(|config| config.id == **child_id)
                                .is_some_and(|config| {
                                    !filtering || connection_matches(config, &filter)
                                })
                        })
                        .map(|(index, id)| (index, id.clone()))
                        .collect::<Vec<_>>();
                    if filtering && visible_children.is_empty() {
                        continue;
                    }
                    let expanded = filtering || !collapsed;
                    let count = visible_children.len();
                    let drag_id = id.clone();
                    let drag_label = name.clone();
                    let drop_folder = id.clone();
                    let drop_child_index = if collapsed { child_count } else { 0 };
                    let empty_folder = visible_children.is_empty();
                    let drop_app = cx.entity().downgrade();
                    rows.push(
                        div()
                            .child(
                                div()
                                    .id(SharedString::from(id.clone()))
                                    .tab_index(0)
                                    .group("sidebar-folder")
                                    .relative()
                                    .cursor_pointer()
                                    .h(ui_px(26.))
                                    .rounded(ui_px(3.))
                                    .flex()
                                    .items_center()
                                    .gap_1()
                                    .pl_1()
                                    .pr(ui_px(6.))
                                    .text_size(ui_px(14.))
                                    .font_weight(gpui::FontWeight::MEDIUM)
                                    .hover(|style| style.bg(PANEL_MUTED))
                                    .when(!filtering, |element| {
                                        element.on_drag(
                                            DraggedSidebarItem {
                                                kind: SidebarDragKind::Folder,
                                                id: drag_id,
                                                label: drag_label.into(),
                                                position: Point::default(),
                                            },
                                            |drag, position, _, cx| {
                                                cx.refresh_windows();
                                                cx.new(|_| DraggedSidebarItem {
                                                    position,
                                                    ..drag.clone()
                                                })
                                            },
                                        )
                                    })
                                    .drag_over::<DraggedSidebarItem>(|style, drag, _, _| {
                                        if drag.kind == SidebarDragKind::Folder {
                                            style.border_t_2().border_color(ACCENT)
                                        } else {
                                            style.bg(cellar_desktop_gpui::theme::accent_soft())
                                        }
                                    })
                                    .on_drop(move |drag: &DraggedSidebarItem, _, cx| {
                                        let target = if drag.kind == SidebarDragKind::Folder {
                                            SidebarDropTarget::Root(root_index)
                                        } else {
                                            SidebarDropTarget::Folder(
                                                drop_folder.clone(),
                                                drop_child_index,
                                            )
                                        };
                                        drop_app
                                            .update(cx, |this, cx| {
                                                if apply_sidebar_drop(
                                                    &mut this.sidebar_layout,
                                                    drag,
                                                    target,
                                                ) {
                                                    cx.notify();
                                                }
                                            })
                                            .ok();
                                    })
                                    .when_some(color.as_deref().and_then(parse_color), |element, color| {
                                        element.child(
                                            div()
                                                .absolute()
                                                .left_0()
                                                .top_0()
                                                .h_full()
                                                .w(ui_px(2.))
                                                .bg(color),
                                        )
                                    })
                                    .child(
                                        div()
                                            .size(ui_px(14.))
                                            .flex_shrink_0()
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .child(
                                                Icon::empty()
                                                    .path(if expanded {
                                                        "icons/chevron-down.svg"
                                                    } else {
                                                        "icons/chevron-right.svg"
                                                    })
                                                    .size(ui_px(10.))
                                                    .text_color(FG_MUTED),
                                            ),
                                    )
                                    .child(
                                        div()
                                            .size(ui_px(14.))
                                            .flex_shrink_0()
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .child(
                                                Icon::empty()
                                                    .path(if expanded {
                                                        "icons/folder-open.svg"
                                                    } else {
                                                        "icons/folder.svg"
                                                    })
                                                    .size(ui_px(12.))
                                                    .text_color(
                                                        color
                                                            .as_deref()
                                                            .and_then(parse_color)
                                                        .unwrap_or(FG_TERTIARY.rgba()),
                                                    ),
                                            ),
                                    )
                                    .when(rename.is_none(), |element| {
                                        element.child(div().flex_1().truncate().text_color(FG).child(name))
                                    })
                                    .when_some(rename, |element, input| {
                                        element.child(
                                            div()
                                                .id(SharedString::from(format!("folder-rename:{}", input.entity_id())))
                                                .h(ui_px(20.))
                                                .min_w_0()
                                                .flex_1()
                                                .rounded(ui_px(3.))
                                                .border_1()
                                                .border_color(FG_MUTED)
                                                .bg(cellar_desktop_gpui::theme::INSET)
                                                .px_1()
                                                .on_click(|_, _, cx| cx.stop_propagation())
                                                .child(Input::new(&input).h_full().appearance(false)),
                                        )
                                    })
                                    .child(
                                        div()
                                            .pr_1()
                                            .text_size(ui_px(11.))
                                            .font_weight(gpui::FontWeight::MEDIUM)
                                            .text_color(FG_MUTED)
                                            .child(count.to_string()),
                                    )
                                    .child(
                                        div()
                                            .id(SharedString::from(format!("folder-actions:{menu_id}")))
                                            .size(ui_px(22.))
                                            .flex_shrink_0()
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .rounded(ui_px(4.))
                                            .text_color(FG_MUTED)
                                            .invisible()
                                            .group_hover("sidebar-folder", |style| style.visible())
                                            .hover(|style| style.bg(PANEL_RAISED).text_color(FG))
                                            .child(Icon::empty().path("icons/ellipsis.svg").size(ui_px(11.)))
                                            .on_mouse_down(
                                                MouseButton::Left,
                                                cx.listener(move |this, event: &MouseDownEvent, _, cx| {
                                                    cx.stop_propagation();
                                                    this.folder_menu = Some(super::sidebar_menu::FolderMenu {
                                                        id: menu_id.clone(),
                                                        position: event.position,
                                                        color_page: false,
                                                    });
                                                    cx.notify();
                                                }),
                                            ),
                                    )
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        if let Some(SidebarItem::Folder { collapsed, .. }) = this
                                            .sidebar_layout
                                            .iter_mut()
                                            .find(|item| matches!(item, SidebarItem::Folder { id, .. } if id == &toggle_id))
                                        {
                                            *collapsed = !*collapsed;
                                            cx.notify();
                                        }
                                    }))
                                    .on_mouse_down(
                                        MouseButton::Right,
                                        cx.listener(move |this, event: &MouseDownEvent, _, cx| {
                                            cx.stop_propagation();
                                            this.folder_menu = Some(super::sidebar_menu::FolderMenu {
                                                id: right_menu_id.clone(),
                                                position: event.position,
                                                color_page: false,
                                            });
                                            cx.notify();
                                        }),
                                    ),
                            )
                            .when(expanded && empty_folder, |element| {
                                let empty_folder = id.clone();
                                let drop_app = cx.entity().downgrade();
                                element.child(
                                    div()
                                        .id(SharedString::from(format!("folder-empty:{id}")))
                                        .ml(ui_px(21.))
                                        .mr_2()
                                        .my(ui_px(2.))
                                        .rounded(ui_px(4.))
                                        .border_1()
                                        .border_dashed()
                                        .border_color(BORDER)
                                        .px_2()
                                        .py_1()
                                        .text_size(ui_px(11.5))
                                        .text_color(FG_MUTED)
                                        .child("empty folder")
                                        .drag_over::<DraggedSidebarItem>(|style, drag, _, _| {
                                            if drag.kind == SidebarDragKind::Connection {
                                                style
                                                    .border_color(ACCENT)
                                                    .bg(cellar_desktop_gpui::theme::accent_soft())
                                                    .text_color(ACCENT)
                                            } else {
                                                style
                                            }
                                        })
                                        .on_drop(move |drag: &DraggedSidebarItem, _, cx| {
                                            drop_app
                                                .update(cx, |this, cx| {
                                                    if apply_sidebar_drop(
                                                        &mut this.sidebar_layout,
                                                        drag,
                                                        SidebarDropTarget::Folder(
                                                            empty_folder.clone(),
                                                            0,
                                                        ),
                                                    ) {
                                                        cx.notify();
                                                    }
                                                })
                                                .ok();
                                        }),
                                )
                            })
                            .when(expanded, |element| {
                                element.children(visible_children.into_iter().filter_map(|(child_index, child_id)| {
                                    let config = self
                                        .model
                                        .connections()
                                        .iter()
                                        .find(|config| config.id == child_id)?;
                                    let drag_id = child_id.clone();
                                    let drag_label = config.name.clone();
                                    let drop_folder = id.clone();
                                    let drop_app = cx.entity().downgrade();
                                    Some(
                                        div()
                                            .id(SharedString::from(format!(
                                                "drag-folder-connection:{id}:{child_id}"
                                            )))
                                            .ml(ui_px(13.))
                                            .border_l_1()
                                            .border_color(color.as_deref().and_then(parse_color).map_or_else(
                                                || BORDER.rgba(),
                                                |color| Rgba { a: 0.45, ..color },
                                            ))
                                            .when(!filtering, |element| {
                                                element.on_drag(
                                                    DraggedSidebarItem {
                                                        kind: SidebarDragKind::Connection,
                                                        id: drag_id,
                                                        label: drag_label.into(),
                                                        position: Point::default(),
                                                    },
                                                    |drag, position, _, cx| {
                                                        cx.refresh_windows();
                                                        cx.new(|_| DraggedSidebarItem {
                                                            position,
                                                            ..drag.clone()
                                                        })
                                                    },
                                                )
                                            })
                                            .drag_over::<DraggedSidebarItem>(|style, drag, _, _| {
                                                if drag.kind == SidebarDragKind::Connection {
                                                    style.border_t_2().border_color(ACCENT)
                                                } else {
                                                    style
                                                }
                                            })
                                            .on_drop(move |drag: &DraggedSidebarItem, _, cx| {
                                                drop_app
                                                    .update(cx, |this, cx| {
                                                        if apply_sidebar_drop(
                                                            &mut this.sidebar_layout,
                                                            drag,
                                                            SidebarDropTarget::Folder(
                                                                drop_folder.clone(),
                                                                child_index,
                                                            ),
                                                        ) {
                                                            cx.notify();
                                                        }
                                                    })
                                                    .ok();
                                            })
                                            .child(self.connection_row(
                                                config,
                                                active_id == Some(child_id.as_str()),
                                                cx,
                                            ))
                                            .when(active_id == Some(child_id.as_str()), |element| {
                                                element.child(self.schema_tree(Some(&child_id), cx))
                                            }),
                                    )
                                }))
                            })
                            .into_any_element(),
                    );
                }
            }
        }
        if !filtering {
            let index = self.sidebar_layout.len();
            let drop_app = cx.entity().downgrade();
            rows.push(
                div()
                    .id("sidebar-drop-end")
                    .mx_2()
                    .h(ui_px(2.))
                    .drag_over::<DraggedSidebarItem>(|style, _, _, _| style.bg(ACCENT))
                    .on_drop(move |drag: &DraggedSidebarItem, _, cx| {
                        drop_app
                            .update(cx, |this, cx| {
                                if apply_sidebar_drop(
                                    &mut this.sidebar_layout,
                                    drag,
                                    SidebarDropTarget::Root(index),
                                ) {
                                    cx.notify();
                                }
                            })
                            .ok();
                    })
                    .into_any_element(),
            );
        }
        rows
    }
}

pub(super) fn connection_matches(config: &ConnectionConfig, query: &str) -> bool {
    [
        config.name.as_str(),
        config.host.as_str(),
        config.database.as_str(),
        config.user.as_str(),
    ]
    .iter()
    .any(|value| value.to_lowercase().contains(query))
        || format!("{:?}", config.engine)
            .to_lowercase()
            .contains(query)
}

fn parse_color(value: &str) -> Option<Rgba> {
    let hex = value.strip_prefix('#')?;
    let rgb = u32::from_str_radix(hex, 16).ok()?;
    Some(Rgba {
        r: ((rgb >> 16) & 0xff) as f32 / 255.,
        g: ((rgb >> 8) & 0xff) as f32 / 255.,
        b: (rgb & 0xff) as f32 / 255.,
        a: 1.,
    })
}

#[cfg(test)]
mod tests {
    use cellar_core::driver::{ConnectionConfig, Engine, SslMode};

    use super::{connection_matches, decode_webkit_value, reconcile, SidebarItem};

    #[test]
    fn decodes_webkit_utf16_local_storage() {
        let bytes: Vec<u8> = "{\"state\":{}}"
            .encode_utf16()
            .flat_map(u16::to_le_bytes)
            .collect();
        assert_eq!(
            decode_webkit_value(&bytes).as_deref(),
            Some("{\"state\":{}}")
        );
    }

    #[test]
    fn connection_filter_matches_visible_metadata() {
        let config = ConnectionConfig {
            id: "prod".into(),
            name: "Epic Prod V2".into(),
            engine: Engine::Postgres,
            host: "db.internal".into(),
            port: 5432,
            user: "cellar".into(),
            database: "epicdb".into(),
            ssl_mode: SslMode::Prefer,
            env_tag: None,
            application_name: None,
            color: None,
        };
        assert!(connection_matches(&config, "epic prod"));
        assert!(connection_matches(&config, "postgres"));
        assert!(connection_matches(&config, "db.internal"));
        assert!(!connection_matches(&config, "sqlite"));

        let mut layout = Vec::new();
        reconcile(&mut layout, &[config]);
        assert!(matches!(layout.as_slice(), [SidebarItem::Connection { id }] if id == "prod"));
    }
}
