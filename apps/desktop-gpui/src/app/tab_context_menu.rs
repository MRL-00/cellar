use cellar_sql::Dialect;
use gpui::{div, prelude::*, px, AnyElement, ClipboardItem, Context};

use super::{context_menu::menu_item, CellarApp};
use cellar_desktop_gpui::theme::{ui_px, BORDER, BORDER_STRONG, PANEL_MUTED};

impl CellarApp {
    pub(super) fn tab_context_menu(&self, cx: &mut Context<Self>) -> AnyElement {
        let menu = self.tab_menu.as_ref().expect("tab menu requires state");
        let Some(tab) = self
            .model
            .tabs()
            .iter()
            .find(|tab| tab.id == menu.tab_id)
            .cloned()
        else {
            return div().into_any_element();
        };
        let tab_panes = self
            .model
            .tabs()
            .iter()
            .map(|item| (item.id, self.model.tab_pane(item.id)))
            .collect::<Vec<_>>();
        let (other_ids, right_ids) = tab_menu_close_targets(&tab_panes, tab.id);
        let query_target = super::query_editor::query_target_for_tab(&tab);
        let query_disabled = query_target.is_none();
        let table_target = match &tab.kind {
            cellar_desktop_gpui::model::TabKind::Table { target, .. } => Some(target.clone()),
            _ => None,
        };
        let select_sql = table_target.as_ref().map(|target| {
            format!(
                "SELECT *\nFROM {}\nLIMIT 100;",
                Dialect::Postgres.quote_qualified(&target.schema, &target.table)
            )
        });
        let copy_label = table_target.as_ref().map_or_else(
            || tab.title.clone(),
            |target| format!("{}.{}", target.schema, target.table),
        );
        let close_id = tab.id;
        let refresh_id = tab.id;
        div()
            .id("tab-menu-backdrop")
            .absolute()
            .inset_0()
            .on_mouse_down(
                gpui::MouseButton::Left,
                cx.listener(|this, _, _, cx| {
                    this.tab_menu = None;
                    cx.notify();
                }),
            )
            .child(
                div()
                    .id("tab-context-menu")
                    .absolute()
                    .left(menu.position.x)
                    .top(menu.position.y)
                    .min_w(ui_px(176.))
                    .py(ui_px(4.))
                    .rounded(ui_px(6.))
                    .border_1()
                    .border_color(BORDER_STRONG)
                    .bg(PANEL_MUTED)
                    .shadow_lg()
                    .on_mouse_down(gpui::MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .child(
                        menu_item("tab-menu-query", "icons/terminal.svg", "New SQL query")
                            .when_some(query_target.clone(), |item, target| {
                                item.on_click(cx.listener(move |this, _, window, cx| {
                                    this.tab_menu = None;
                                    this.open_query(target.clone(), String::new(), window, cx);
                                }))
                            })
                            .when(query_disabled, |item| {
                                item.cursor_default()
                                    .opacity(0.4)
                                    .hover(|style| style.bg(PANEL_MUTED))
                            }),
                    )
                    .when(table_target.is_some(), |element| {
                        element.child(
                            menu_item("tab-menu-refresh", "icons/book-open.svg", "Refresh")
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.tab_menu = None;
                                    this.reload_table(refresh_id, cx);
                                })),
                        )
                    })
                    .when_some(select_sql.zip(query_target), |element, (sql, target)| {
                        element.child(
                            menu_item(
                                "tab-menu-select-all",
                                "icons/terminal.svg",
                                "Query SELECT *",
                            )
                            .on_click(cx.listener(
                                move |this, _, window, cx| {
                                    this.tab_menu = None;
                                    this.open_query(target.clone(), sql.clone(), window, cx);
                                },
                            )),
                        )
                    })
                    .child(
                        menu_item(
                            "tab-menu-copy",
                            "icons/copy.svg",
                            if table_target.is_some() {
                                "Copy qualified name"
                            } else {
                                "Copy title"
                            },
                        )
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.tab_menu = None;
                            cx.write_to_clipboard(ClipboardItem::new_string(copy_label.clone()));
                            cx.notify();
                        })),
                    )
                    .child(div().my_1().h(px(1.)).bg(BORDER))
                    .child(
                        menu_item("tab-menu-close", "icons/close.svg", "Close").on_click(
                            cx.listener(move |this, _, _, cx| {
                                this.tab_menu = None;
                                this.close_tab(close_id, cx);
                            }),
                        ),
                    )
                    .child(
                        menu_item("tab-menu-close-others", "icons/close.svg", "Close Others")
                            .when(other_ids.is_empty(), |item| {
                                item.cursor_default()
                                    .opacity(0.4)
                                    .hover(|style| style.bg(PANEL_MUTED))
                            })
                            .when(!other_ids.is_empty(), |item| {
                                item.on_click(cx.listener(move |this, _, _, cx| {
                                    this.tab_menu = None;
                                    for id in &other_ids {
                                        this.close_tab(*id, cx);
                                    }
                                }))
                            }),
                    )
                    .child(
                        menu_item(
                            "tab-menu-close-right",
                            "icons/close.svg",
                            "Close Tabs to the Right",
                        )
                        .when(right_ids.is_empty(), |item| {
                            item.cursor_default()
                                .opacity(0.4)
                                .hover(|style| style.bg(PANEL_MUTED))
                        })
                        .when(!right_ids.is_empty(), |item| {
                            item.on_click(cx.listener(move |this, _, _, cx| {
                                this.tab_menu = None;
                                for id in &right_ids {
                                    this.close_tab(*id, cx);
                                }
                            }))
                        }),
                    ),
            )
            .into_any_element()
    }
}

fn tab_menu_close_targets(tabs: &[(u64, u8)], active_id: u64) -> (Vec<u64>, Vec<u64>) {
    let Some((_, active_pane)) = tabs.iter().find(|(id, _)| *id == active_id) else {
        return (Vec::new(), Vec::new());
    };
    let pane_ids = tabs
        .iter()
        .filter(|(_, pane)| pane == active_pane)
        .map(|(id, _)| *id)
        .collect::<Vec<_>>();
    let index = pane_ids.iter().position(|id| *id == active_id).unwrap_or(0);
    let others = pane_ids
        .iter()
        .copied()
        .filter(|id| *id != active_id)
        .collect();
    let right = pane_ids.into_iter().skip(index + 1).collect();
    (others, right)
}

#[cfg(test)]
mod tests {
    use super::tab_menu_close_targets;

    #[test]
    fn close_actions_stay_in_the_clicked_split_pane() {
        let tabs = [(1, 0), (2, 1), (3, 0), (4, 1)];
        assert_eq!(tab_menu_close_targets(&tabs, 1), (vec![3], vec![3]));
        assert_eq!(tab_menu_close_targets(&tabs, 4), (vec![2], vec![]));
    }
}
