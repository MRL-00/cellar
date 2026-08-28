use gpui::{div, prelude::*, AnyElement, Context, Entity, Pixels, Point, SharedString, Window};
use gpui_component::input::{InputEvent, InputState};

use super::{sidebar_layout::SidebarItem, CellarApp};
use cellar_desktop_gpui::theme::{
    accent_soft, ui_px, ACCENT, BORDER_STRONG, FG_SECONDARY, PANEL_MUTED, PROD, WARN_SOFT,
};

pub(super) struct FolderMenu {
    pub(super) id: String,
    pub(super) position: Point<Pixels>,
    pub(super) color_page: bool,
}

pub(super) struct FolderRename {
    pub(super) id: String,
    pub(super) input: Entity<InputState>,
}

const COLORS: [(&str, &str); 7] = [
    ("#4f8ff7", "Blue"),
    ("#f6a44a", "Orange"),
    ("#d97a5a", "Coral"),
    ("#5bb8e0", "Cyan"),
    ("#a78bfa", "Purple"),
    ("#4ade80", "Green"),
    ("#f87171", "Red"),
];

impl CellarApp {
    pub(super) fn start_new_sidebar_folder(
        &mut self,
        connection_id: Option<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let id = format!("folder-{}", chrono::Utc::now().timestamp_millis());
        if let Some(connection_id) = connection_id.as_deref() {
            remove_connection_from_layout(&mut self.sidebar_layout, connection_id);
        }
        self.sidebar_layout.push(SidebarItem::Folder {
            id: id.clone(),
            name: "New folder".into(),
            collapsed: false,
            children: connection_id.into_iter().collect(),
            color: None,
        });
        self.begin_folder_rename(id, "New folder".into(), window, cx);
    }

    pub(super) fn begin_folder_rename(
        &mut self,
        id: String,
        name: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let input = cx.new(|cx| InputState::new(window, cx).default_value(name));
        input.update(cx, |input, cx| input.focus(window, cx));
        self.folder_rename_subscription = Some(cx.subscribe(&input, |this, _, event, cx| {
            if matches!(event, InputEvent::PressEnter { .. } | InputEvent::Blur) {
                this.commit_folder_rename(cx);
            }
        }));
        self.folder_rename = Some(FolderRename { id, input });
        self.sidebar_menu = None;
        self.folder_menu = None;
        cx.notify();
    }

    pub(super) fn commit_folder_rename(&mut self, cx: &mut Context<Self>) {
        let Some(rename) = self.folder_rename.take() else {
            return;
        };
        let name = rename.input.read(cx).value().trim().to_owned();
        if let Some(SidebarItem::Folder { name: current, .. }) = self
            .sidebar_layout
            .iter_mut()
            .find(|item| matches!(item, SidebarItem::Folder { id, .. } if id == &rename.id))
        {
            *current = if name.is_empty() {
                "Untitled folder".into()
            } else {
                name
            };
        }
        self.folder_rename_subscription = None;
        cx.notify();
    }

    pub(super) fn cancel_folder_rename(&mut self, cx: &mut Context<Self>) {
        self.folder_rename = None;
        self.folder_rename_subscription = None;
        cx.notify();
    }

    pub(super) fn move_connection_to_folder(
        &mut self,
        connection_id: String,
        folder_id: Option<String>,
        cx: &mut Context<Self>,
    ) {
        remove_connection_from_layout(&mut self.sidebar_layout, &connection_id);
        if let Some(folder_id) = folder_id {
            if let Some(SidebarItem::Folder { children, .. }) = self
                .sidebar_layout
                .iter_mut()
                .find(|item| matches!(item, SidebarItem::Folder { id, .. } if id == &folder_id))
            {
                children.push(connection_id);
            }
        } else {
            self.sidebar_layout
                .push(SidebarItem::Connection { id: connection_id });
        }
        self.connection_menu = None;
        cx.notify();
    }

    pub(super) fn sidebar_context_menu(&self, cx: &mut Context<Self>) -> AnyElement {
        let position = self.sidebar_menu.expect("sidebar menu requires state");
        let connected = self
            .model
            .connections()
            .iter()
            .filter(|connection| {
                matches!(
                    self.model.connection_state(&connection.id),
                    cellar_desktop_gpui::model::ConnectionState::Connected
                )
            })
            .map(|connection| connection.id.clone())
            .collect::<Vec<_>>();
        overlay("sidebar-actions-backdrop", cx)
            .child(
                menu(position)
                    .child(
                        item("sidebar-new-connection", "icons/plus.svg", "New connection")
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.sidebar_menu = None;
                                this.open_connection_editor(None, window, cx);
                            })),
                    )
                    .child(
                        item("sidebar-new-folder", "icons/folder-plus.svg", "New folder").on_click(
                            cx.listener(|this, _, window, cx| {
                                this.start_new_sidebar_folder(None, window, cx);
                            }),
                        ),
                    )
                    .child(
                        item(
                            "sidebar-import",
                            "icons/database.svg",
                            "Import from DataGrip",
                        )
                        .on_click(cx.listener(|this, _, window, cx| {
                            this.sidebar_menu = None;
                            this.scan_datagrip(window, cx);
                        })),
                    )
                    .child(
                        item(
                            "sidebar-refresh",
                            "icons/history.svg",
                            "Refresh connected schemas",
                        )
                        .opacity(if connected.is_empty() { 0.45 } else { 1. })
                        .when(!connected.is_empty(), |element| {
                            element.on_click(cx.listener(move |this, _, window, cx| {
                                this.sidebar_menu = None;
                                for id in connected.clone() {
                                    this.refresh_schema(id, window, cx);
                                }
                            }))
                        }),
                    ),
            )
            .into_any_element()
    }

    pub(super) fn folder_context_menu(&self, cx: &mut Context<Self>) -> AnyElement {
        let state = self
            .folder_menu
            .as_ref()
            .expect("folder menu requires state");
        let id = state.id.clone();
        let position = state.position;
        let (name, color, has_children) = self
            .sidebar_layout
            .iter()
            .find_map(|item| match item {
                SidebarItem::Folder {
                    id: candidate,
                    name,
                    color,
                    children,
                    ..
                } if candidate == &id => Some((name.clone(), color.clone(), !children.is_empty())),
                _ => None,
            })
            .unwrap_or_default();
        let content = if state.color_page {
            let mut menu = menu(position);
            let selected_color = color.clone();
            for (color, label) in COLORS {
                let folder = id.clone();
                let current = selected_color.as_deref() == Some(color);
                menu = menu.child(
                    color_item(
                        color,
                        if current {
                            format!("{label} (current)")
                        } else {
                            label.into()
                        },
                    )
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.set_folder_color(&folder, Some(color.into()), cx);
                    })),
                );
            }
            let folder = id.clone();
            menu.child(
                item("folder-clear-color", "icons/close.svg", "Clear color")
                    .opacity(if color.is_some() { 1. } else { 0.45 })
                    .when(color.is_some(), |element| {
                        element.on_click(cx.listener(move |this, _, _, cx| {
                            this.set_folder_color(&folder, None, cx);
                        }))
                    }),
            )
        } else {
            let rename_id = id.clone();
            let rename_name = name.clone();
            let color_id = id.clone();
            let remove_id = id.clone();
            menu(position)
                .child(
                    item("folder-rename", "icons/edit.svg", "Rename folder").on_click(cx.listener(
                        move |this, _, window, cx| {
                            this.begin_folder_rename(
                                rename_id.clone(),
                                rename_name.clone(),
                                window,
                                cx,
                            );
                        },
                    )),
                )
                .child(
                    color
                        .as_deref()
                        .map_or_else(
                            || item("folder-color", "icons/folder.svg", "Set color…"),
                            |color| color_item(color, "Set color…"),
                        )
                        .on_click(cx.listener(move |this, _, _, cx| {
                            if let Some(menu) =
                                this.folder_menu.as_mut().filter(|menu| menu.id == color_id)
                            {
                                menu.color_page = true;
                            }
                            cx.notify();
                        })),
                )
                .child(
                    item(
                        "folder-remove",
                        "icons/trash.svg",
                        if has_children {
                            "Remove folder (keep connections)"
                        } else {
                            "Remove folder"
                        },
                    )
                    .text_color(PROD)
                    .hover(|style| style.bg(WARN_SOFT).text_color(PROD))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.remove_sidebar_folder(&remove_id, cx);
                    })),
                )
        };
        overlay("folder-menu-backdrop", cx)
            .child(content)
            .into_any_element()
    }

    fn set_folder_color(&mut self, id: &str, color: Option<String>, cx: &mut Context<Self>) {
        if let Some(SidebarItem::Folder { color: current, .. }) =
            self.sidebar_layout.iter_mut().find(
                |item| matches!(item, SidebarItem::Folder { id: candidate, .. } if candidate == id),
            )
        {
            *current = color;
        }
        self.folder_menu = None;
        cx.notify();
    }

    fn remove_sidebar_folder(&mut self, id: &str, cx: &mut Context<Self>) {
        let Some(index) = self.sidebar_layout.iter().position(
            |item| matches!(item, SidebarItem::Folder { id: candidate, .. } if candidate == id),
        ) else {
            return;
        };
        let SidebarItem::Folder { children, .. } = self.sidebar_layout.remove(index) else {
            unreachable!();
        };
        self.sidebar_layout.splice(
            index..index,
            children
                .into_iter()
                .map(|id| SidebarItem::Connection { id }),
        );
        self.folder_menu = None;
        cx.notify();
    }
}

fn remove_connection_from_layout(items: &mut Vec<SidebarItem>, connection_id: &str) {
    items.retain(|item| !matches!(item, SidebarItem::Connection { id } if id == connection_id));
    for item in items {
        if let SidebarItem::Folder { children, .. } = item {
            children.retain(|id| id != connection_id);
        }
    }
}

fn overlay(id: &'static str, cx: &mut Context<CellarApp>) -> gpui::Stateful<gpui::Div> {
    div().id(id).absolute().inset_0().on_mouse_down(
        gpui::MouseButton::Left,
        cx.listener(|this, _, _, cx| {
            this.sidebar_menu = None;
            this.folder_menu = None;
            cx.notify();
        }),
    )
}

fn menu(position: Point<Pixels>) -> gpui::Div {
    div()
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

fn item(
    id: &'static str,
    icon: &'static str,
    label: impl Into<SharedString>,
) -> gpui::Stateful<gpui::Div> {
    super::context_menu::menu_item(id, icon, label)
}

fn color_item(color: &str, label: impl Into<SharedString>) -> gpui::Stateful<gpui::Div> {
    let label = label.into();
    div()
        .id(SharedString::from(format!("folder-color:{color}")))
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
                .child(
                    div()
                        .size(ui_px(10.))
                        .rounded(ui_px(5.))
                        .bg(parse_color(color)),
                ),
        )
        .child(label)
}

fn parse_color(color: &str) -> gpui::Rgba {
    let value = u32::from_str_radix(color.trim_start_matches('#'), 16).unwrap_or(0);
    gpui::rgba((value << 8) | 0xff)
}

#[cfg(test)]
mod tests {
    use super::remove_connection_from_layout;
    use crate::app::sidebar_layout::SidebarItem;

    #[test]
    fn moving_a_connection_removes_its_previous_sidebar_location() {
        let mut items = vec![
            SidebarItem::Connection { id: "one".into() },
            SidebarItem::Folder {
                id: "folder".into(),
                name: "Work".into(),
                collapsed: false,
                children: vec!["two".into(), "one".into()],
                color: None,
            },
        ];
        remove_connection_from_layout(&mut items, "one");
        assert!(items.iter().all(|item| match item {
            SidebarItem::Connection { id } => id != "one",
            SidebarItem::Folder { children, .. } => !children.contains(&"one".to_owned()),
        }));
    }
}
