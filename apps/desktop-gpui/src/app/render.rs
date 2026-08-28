use super::*;

impl Render for CellarApp {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let app = cx.weak_entity();
        let mut interface_font = gpui::font(self.preferences.interface_font.clone());
        interface_font.features = gpui::FontFeatures(Arc::new(vec![
            ("cv11".into(), 1),
            ("ss01".into(), 1),
            ("ss03".into(), 1),
        ]));

        div()
            .id("cellar-app")
            .tab_group()
            .size_full()
            .relative()
            .flex()
            .flex_col()
            .bg(BG)
            .text_color(FG)
            .font(interface_font)
            // Classic Cellar authors body text at 14px, then zooms the whole UI
            // against its 13px settings baseline.
            .text_size(px(self.preferences.font_size_px * 14. / 13.))
            .on_action(|_: &crate::app_menu::Minimize, window, _| window.minimize_window())
            .on_action(|_: &crate::app_menu::Zoom, window, _| window.zoom_window())
            .on_action({
                let app = app.clone();
                move |_: &crate::app_menu::NewConnection, window, cx| {
                    if let Some(app) = app.upgrade() {
                        app.update(cx, |this, cx| {
                            this.open_connection_editor(None, window, cx)
                        });
                    }
                }
            })
            .on_action({
                let app = app.clone();
                move |_: &crate::app_menu::NewQuery, window, cx| {
                    if let Some(app) = app.upgrade() {
                        app.update(cx, |this, cx| this.new_query(window, cx));
                    }
                }
            })
            .on_action({
                let app = app.clone();
                move |_: &crate::app_menu::CloseTab, _, cx| {
                    if let Some(app) = app.upgrade() {
                        app.update(cx, |this, cx| {
                            if let Some(tab_id) = this.model.active_tab().map(|tab| tab.id) {
                                this.close_tab(tab_id, cx);
                            }
                        });
                    }
                }
            })
            .on_action({
                let app = app.clone();
                move |_: &crate::app_menu::ToggleSidebar, _, cx| {
                    if let Some(app) = app.upgrade() {
                        app.update(cx, |this, cx| {
                            this.sidebar_open = !this.sidebar_open;
                            cx.notify();
                        });
                    }
                }
            })
            .on_action({
                let app = app.clone();
                move |_: &crate::app_menu::ToggleAiPanel, _, cx| {
                    if let Some(app) = app.upgrade() {
                        app.update(cx, |this, cx| {
                            this.right_panel_open = !this.right_panel_open;
                            cx.notify();
                        });
                    }
                }
            })
            .on_action({
                let app = app.clone();
                move |_: &crate::app_menu::ToggleBottomPanel, _, cx| {
                    if let Some(app) = app.upgrade() {
                        app.update(cx, |this, cx| {
                            this.bottom_panel_open = !this.bottom_panel_open;
                            cx.notify();
                        });
                    }
                }
            })
            .on_mouse_move(cx.listener(Self::resize_panels))
            .on_scroll_wheel(cx.listener(|this, _, _, cx| {
                if this.dismiss_context_menus() {
                    cx.notify();
                }
            }))
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                if this.command_palette.is_some()
                    && this.handle_command_palette_key(event, window, cx)
                {
                    cx.stop_propagation();
                } else if event.keystroke.key == "enter"
                    && !event.keystroke.modifiers.modified()
                    && this.confirmation.is_some()
                {
                    // The canonical dialog autofocuses Cancel, so bare Enter is safe by default.
                    this.resolve_confirmation(false, window, cx);
                    cx.stop_propagation();
                } else if event.keystroke.key == "enter"
                    && !event.keystroke.modifiers.modified()
                    && this.confirmation.is_none()
                    && this
                        .commit_review
                        .as_ref()
                        .is_some_and(|review| review.preview.is_some() && !review.committing)
                {
                    this.start_commit(cx);
                    cx.stop_propagation();
                } else if event.keystroke.modifiers.secondary() && event.keystroke.key == "k" {
                    this.toggle_command_palette(window, cx);
                    cx.stop_propagation();
                } else if event.keystroke.modifiers.secondary() && event.keystroke.key == "," {
                    this.open_settings(settings::SettingsCategory::Appearance, cx);
                    cx.stop_propagation();
                } else if event.keystroke.modifiers.secondary() && event.keystroke.key == "n" {
                    this.open_connection_editor(None, window, cx);
                    cx.stop_propagation();
                } else if event.keystroke.modifiers.secondary() && event.keystroke.key == "t" {
                    this.new_query(window, cx);
                    cx.stop_propagation();
                } else if this.save_template_editor.is_some()
                    && event.keystroke.modifiers.secondary()
                    && event.keystroke.key == "enter"
                {
                    this.save_query_template(cx);
                    cx.stop_propagation();
                } else if event.keystroke.modifiers.secondary() && event.keystroke.key == "enter" {
                    if let Some(tab_id) = this.model.active_tab().and_then(|tab| {
                        matches!(&tab.kind, cellar_desktop_gpui::model::TabKind::Query { .. })
                            .then_some(tab.id)
                    }) {
                        if event.keystroke.modifiers.shift {
                            this.start_query_all(tab_id, window, cx);
                        } else {
                            this.start_query(tab_id, window, cx);
                        }
                    }
                    cx.stop_propagation();
                } else if event.keystroke.modifiers.secondary() && event.keystroke.key == "." {
                    if let Some(tab_id) = this.model.active_tab().map(|tab| tab.id) {
                        this.cancel_query(tab_id, cx);
                    }
                    cx.stop_propagation();
                } else if event.keystroke.modifiers.secondary() && event.keystroke.key == "w" {
                    if let Some(tab_id) = this.model.active_tab().map(|tab| tab.id) {
                        this.close_tab(tab_id, cx);
                    }
                    cx.stop_propagation();
                } else if event.keystroke.modifiers.secondary() && event.keystroke.key == "s" {
                    if let Some(grid) = this
                        .model
                        .active_tab()
                        .and_then(|tab| this.grids.get(&tab.id))
                        .cloned()
                    {
                        grid.update(cx, |grid, cx| grid.request_review(cx));
                    }
                    cx.stop_propagation();
                } else if event.keystroke.modifiers.secondary() && event.keystroke.key == "b" {
                    this.sidebar_open = !this.sidebar_open;
                    cx.notify();
                    cx.stop_propagation();
                } else if event.keystroke.modifiers.secondary() && event.keystroke.key == "j" {
                    this.right_panel_open = !this.right_panel_open;
                    cx.notify();
                    cx.stop_propagation();
                } else if event.keystroke.modifiers.secondary() && event.keystroke.key == "down" {
                    this.bottom_panel_open = !this.bottom_panel_open;
                    cx.notify();
                    cx.stop_propagation();
                } else if event.keystroke.modifiers.secondary() && event.keystroke.key == "f" {
                    if this.settings_open {
                        this.settings_search
                            .update(cx, |search, cx| search.focus(window, cx));
                    } else {
                        this.sidebar_filter
                            .update(cx, |filter, cx| filter.focus(window, cx));
                    }
                    cx.stop_propagation();
                } else if event.keystroke.key == "escape" {
                    if this.save_template_editor.take().is_some() {
                        cx.notify();
                        cx.stop_propagation();
                    } else if this.table_preset_draft.is_some() {
                        this.cancel_filter_preset_draft(cx);
                        cx.stop_propagation();
                    } else if this.folder_rename.is_some() {
                        this.cancel_folder_rename(cx);
                        cx.stop_propagation();
                    } else if this.dismiss_context_menus() {
                        cx.notify();
                        cx.stop_propagation();
                    } else if this.schema_visibility_editor.take().is_some() {
                        cx.notify();
                        cx.stop_propagation();
                    } else if this.schema_compare_dialog.take().is_some() {
                        cx.notify();
                        cx.stop_propagation();
                    } else if this.setup_transfer.take().is_some() {
                        cx.notify();
                        cx.stop_propagation();
                    } else if this.confirmation.is_some() {
                        this.resolve_confirmation(false, window, cx);
                        cx.stop_propagation();
                    } else if this.settings_open {
                        this.close_settings(window, cx);
                        cx.stop_propagation();
                    } else if this.command_palette.is_some() {
                        this.command_palette = None;
                        this.command_palette_subscription = None;
                        cx.notify();
                        cx.stop_propagation();
                    } else if this.connection_editor.take().is_some()
                        || this.connection_import.take().is_some()
                        || this.data_import.take().is_some()
                        || this.commit_review.take().is_some()
                    {
                        cx.notify();
                        cx.stop_propagation();
                    }
                }
            }))
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|this, _, _, cx| this.finish_panel_resize(cx)),
            )
            .on_mouse_up_out(MouseButton::Left, move |_, _, cx| {
                app.update(cx, |app, cx| app.finish_panel_resize(cx)).ok();
            })
            .child(self.title_bar(cx))
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .flex()
                    .when(self.show_empty_state, |element| {
                        element.child(self.canonical_empty_state(cx))
                    })
                    .when(!self.show_empty_state && self.sidebar_open, |element| {
                        element.child(self.sidebar(cx))
                    })
                    .when(!self.show_empty_state, |element| {
                        element
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .h_full()
                                    .flex()
                                    .flex_col()
                                    .child(self.workspace(window, cx))
                                    .when(self.bottom_panel_open, |element| {
                                        element.child(self.bottom_panel(cx))
                                    }),
                            )
                            .when(self.right_panel_open, |element| {
                                element.child(self.ai_panel(cx))
                            })
                    }),
            )
            .child(self.status_bar())
            .when(self.show_update_toast(), |element| {
                element.child(self.update_toast(cx))
            })
            .when(self.commit_review.is_some(), |element| {
                element.child(self.commit_overlay(cx))
            })
            .when(self.save_template_editor.is_some(), |element| {
                element.child(self.save_template_overlay(cx))
            })
            .when(self.data_import.is_some(), |element| {
                element.child(self.import_overlay(cx))
            })
            .when(self.connection_import.is_some(), |element| {
                element.child(self.connection_import_overlay(cx))
            })
            .when(self.connection_editor.is_some(), |element| {
                element.child(self.connection_editor_overlay(cx))
            })
            .when(self.command_palette.is_some(), |element| {
                element.child(self.command_palette_overlay(cx))
            })
            .when(self.settings_open, |element| {
                element.child(self.settings_overlay(cx))
            })
            .when(self.confirmation.is_some(), |element| {
                element.child(self.confirmation_overlay(cx))
            })
            .when(self.setup_transfer.is_some(), |element| {
                element.child(self.setup_transfer_overlay(cx))
            })
            .when(self.schema_compare_dialog.is_some(), |element| {
                element.child(self.schema_compare_dialog_overlay(cx))
            })
            .when(self.connection_menu.is_some(), |element| {
                element.child(self.connection_context_menu(cx))
            })
            .when(self.table_menu.is_some(), |element| {
                element.child(self.table_context_menu(cx))
            })
            .when(self.schema_menu.is_some(), |element| {
                element.child(self.schema_context_menu(cx))
            })
            .when(self.tab_menu.is_some(), |element| {
                element.child(self.tab_context_menu(cx))
            })
            .when(self.schema_visibility_editor.is_some(), |element| {
                element.child(self.schema_visibility_overlay(cx))
            })
            .when(self.sidebar_menu.is_some(), |element| {
                element.child(self.sidebar_context_menu(cx))
            })
            .when(self.folder_menu.is_some(), |element| {
                element.child(self.folder_context_menu(cx))
            })
            .when(self.query_database_menu.is_some(), |element| {
                element.child(self.query_database_menu(cx))
            })
            .when(self.table_preset_menu.is_some(), |element| {
                element.child(self.table_preset_menu_overlay(cx))
            })
    }
}
