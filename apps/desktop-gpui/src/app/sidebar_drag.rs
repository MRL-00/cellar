use gpui::{div, prelude::*, Context, Pixels, Point, Render, SharedString, Window};

use super::sidebar_layout::SidebarItem;
use cellar_desktop_gpui::theme::{ui_px, ACCENT, FG, PANEL_RAISED};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum SidebarDragKind {
    Connection,
    Folder,
}

#[derive(Clone)]
pub(super) struct DraggedSidebarItem {
    pub(super) kind: SidebarDragKind,
    pub(super) id: String,
    pub(super) label: SharedString,
    pub(super) position: Point<Pixels>,
}

impl Render for DraggedSidebarItem {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .pl(self.position.x - ui_px(70.))
            .pt(self.position.y - ui_px(14.))
            .child(
                div()
                    .max_w(ui_px(220.))
                    .px_3()
                    .py_1()
                    .rounded(ui_px(4.))
                    .border_1()
                    .border_color(ACCENT)
                    .bg(PANEL_RAISED)
                    .text_color(FG)
                    .truncate()
                    .child(self.label.clone()),
            )
    }
}

#[derive(Clone)]
pub(super) enum SidebarDropTarget {
    Root(usize),
    Folder(String, usize),
}

pub(super) fn apply_sidebar_drop(
    items: &mut Vec<SidebarItem>,
    drag: &DraggedSidebarItem,
    target: SidebarDropTarget,
) -> bool {
    match drag.kind {
        SidebarDragKind::Connection => move_connection(items, &drag.id, target),
        SidebarDragKind::Folder => match target {
            SidebarDropTarget::Root(index) => move_folder(items, &drag.id, index),
            SidebarDropTarget::Folder(_, _) => false,
        },
    }
}

fn move_connection(
    items: &mut Vec<SidebarItem>,
    connection_id: &str,
    target: SidebarDropTarget,
) -> bool {
    let from = items
        .iter()
        .enumerate()
        .find_map(|(root, item)| match item {
            SidebarItem::Connection { id } if id == connection_id => Some((None, root)),
            SidebarItem::Folder { id, children, .. } => children
                .iter()
                .position(|child| child == connection_id)
                .map(|index| (Some(id.clone()), index)),
            _ => None,
        });
    let Some((from_container, from_index)) = from else {
        return false;
    };
    let (target_container, mut target_index) = match &target {
        SidebarDropTarget::Root(index) => (None, *index),
        SidebarDropTarget::Folder(id, index) => {
            if !items
                .iter()
                .any(|item| matches!(item, SidebarItem::Folder { id: folder, .. } if folder == id))
            {
                return false;
            }
            (Some(id.clone()), *index)
        }
    };
    if from_container == target_container && from_index < target_index {
        target_index -= 1;
    }
    remove_connection(items, connection_id);
    match target {
        SidebarDropTarget::Root(_) => items.insert(
            target_index.min(items.len()),
            SidebarItem::Connection {
                id: connection_id.into(),
            },
        ),
        SidebarDropTarget::Folder(folder_id, _) => {
            let Some(SidebarItem::Folder { children, .. }) = items
                .iter_mut()
                .find(|item| matches!(item, SidebarItem::Folder { id, .. } if id == &folder_id))
            else {
                return false;
            };
            children.insert(target_index.min(children.len()), connection_id.into());
        }
    }
    true
}

fn move_folder(items: &mut Vec<SidebarItem>, folder_id: &str, index: usize) -> bool {
    let Some(from) = items
        .iter()
        .position(|item| matches!(item, SidebarItem::Folder { id, .. } if id == folder_id))
    else {
        return false;
    };
    let target = if index > from { index - 1 } else { index }.min(items.len() - 1);
    if target == from {
        return false;
    }
    let folder = items.remove(from);
    items.insert(target, folder);
    true
}

fn remove_connection(items: &mut Vec<SidebarItem>, connection_id: &str) {
    items.retain(|item| !matches!(item, SidebarItem::Connection { id } if id == connection_id));
    for item in items {
        if let SidebarItem::Folder { children, .. } = item {
            children.retain(|id| id != connection_id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{apply_sidebar_drop, DraggedSidebarItem, SidebarDragKind, SidebarDropTarget};
    use crate::app::sidebar_layout::SidebarItem;
    use gpui::Point;

    #[test]
    fn sidebar_drops_preserve_one_location_and_display_order() {
        let mut items = vec![
            SidebarItem::Connection { id: "one".into() },
            SidebarItem::Folder {
                id: "folder".into(),
                name: "Work".into(),
                collapsed: false,
                children: vec!["two".into()],
                color: None,
            },
            SidebarItem::Connection { id: "three".into() },
        ];
        let drag = DraggedSidebarItem {
            kind: SidebarDragKind::Connection,
            id: "one".into(),
            label: "One".into(),
            position: Point::default(),
        };
        assert!(apply_sidebar_drop(
            &mut items,
            &drag,
            SidebarDropTarget::Folder("folder".into(), 1),
        ));
        assert!(matches!(
            &items[0],
            SidebarItem::Folder { children, .. } if children == &["two", "one"]
        ));

        let drag = DraggedSidebarItem {
            kind: SidebarDragKind::Folder,
            id: "folder".into(),
            label: "Work".into(),
            position: Point::default(),
        };
        let end = items.len();
        assert!(apply_sidebar_drop(
            &mut items,
            &drag,
            SidebarDropTarget::Root(end),
        ));
        assert!(matches!(items.last(), Some(SidebarItem::Folder { .. })));
    }
}
