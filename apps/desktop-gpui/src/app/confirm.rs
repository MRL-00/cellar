use cellar_core::query::PlanMode;
use gpui::{div, prelude::*, AnyElement, Context, KeyDownEvent, MouseButton, Window};

use super::CellarApp;
use cellar_desktop_gpui::model::ConnectionState;
use cellar_desktop_gpui::theme::{
    accent, ui_px, ACCENT, ACCENT_FG, BORDER, BORDER_STRONG, FG, FG_SECONDARY, PANEL, PANEL_RAISED,
    WARN,
};

pub(super) enum ConfirmAction {
    Dismiss,
    RemoveConnection(String),
    Analyze(u64),
    Reconnect(String),
}

pub(super) struct Confirmation {
    pub(super) title: String,
    pub(super) message: String,
    pub(super) confirm_label: &'static str,
    pub(super) cancel_label: &'static str,
    pub(super) danger: bool,
    pub(super) action: ConfirmAction,
}

impl Confirmation {
    pub(super) fn connection_error(id: String, name: &str, error: String) -> Self {
        Self {
            title: format!("Could not connect to {name}"),
            message: error,
            confirm_label: "Retry",
            cancel_label: "Close",
            danger: false,
            action: ConfirmAction::Reconnect(id),
        }
    }
}

impl CellarApp {
    pub(super) fn ask_confirmation(
        &mut self,
        confirmation: Confirmation,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.confirmation = Some(confirmation);
        self.confirmation_focus.focus(window);
        cx.notify();
    }

    pub(super) fn resolve_confirmation(
        &mut self,
        confirmed: bool,
        window: &mut gpui::Window,
        cx: &mut Context<Self>,
    ) {
        let Some(confirmation) = self.confirmation.take() else {
            return;
        };
        if confirmed {
            match confirmation.action {
                ConfirmAction::Dismiss => {}
                ConfirmAction::RemoveConnection(id) => self.delete_connection_confirmed(id, cx),
                ConfirmAction::Analyze(tab_id) => {
                    self.explain_query(tab_id, PlanMode::Analyze, window, cx)
                }
                ConfirmAction::Reconnect(id) => self.reconnect(id, cx),
            }
        }
        cx.notify();
    }

    pub(super) fn show_connection_error(
        &mut self,
        id: &str,
        window: Option<&mut Window>,
        cx: &mut Context<Self>,
    ) {
        if !can_replace_confirmation(self.confirmation.as_ref()) {
            return;
        }
        let name = self
            .model
            .connections()
            .iter()
            .find(|config| config.id == id)
            .map(|config| config.name.clone())
            .unwrap_or_else(|| id.to_owned());
        let error = match self.model.connection_state(id) {
            ConnectionState::Error(error) => error.clone(),
            _ => return,
        };
        self.confirmation = Some(Confirmation::connection_error(id.to_owned(), &name, error));
        if let Some(window) = window {
            self.confirmation_focus.focus(window);
        }
        cx.notify();
    }

    pub(super) fn confirmation_overlay(&self, cx: &mut Context<Self>) -> AnyElement {
        let confirmation = self
            .confirmation
            .as_ref()
            .expect("confirmation requires state");
        let danger = confirmation.danger;
        div()
            .id("confirmation-backdrop")
            .absolute()
            .inset_0()
            .flex()
            .items_start()
            .justify_center()
            .pt(gpui::relative(0.08))
            .bg(cellar_desktop_gpui::theme::overlay())
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _, window, cx| this.resolve_confirmation(false, window, cx)),
            )
            .child(
                div()
                    .id("confirmation-dialog")
                    .w(ui_px(420.))
                    .rounded(ui_px(8.))
                    .border_1()
                    .border_color(BORDER)
                    .bg(PANEL)
                    .shadow_lg()
                    .p_4()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .child(
                        div()
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_size(ui_px(14.))
                            .text_color(FG)
                            .child(confirmation.title.clone()),
                    )
                    .child(
                        div()
                            .text_color(FG_SECONDARY)
                            .text_size(ui_px(14.))
                            .line_height(ui_px(23.))
                            .child(confirmation.message.clone()),
                    )
                    .child(
                        div()
                            .mt_1()
                            .flex()
                            .justify_end()
                            .gap_2()
                            .child(
                                confirm_button(
                                    "confirmation-cancel",
                                    confirmation.cancel_label,
                                    false,
                                )
                                .track_focus(&self.confirmation_focus)
                                .hover(|style| {
                                    style
                                        .bg(PANEL_RAISED)
                                        .border_color(BORDER_STRONG)
                                        .text_color(FG)
                                })
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.resolve_confirmation(false, window, cx)
                                }))
                                .on_key_down(cx.listener(
                                    |this, event: &KeyDownEvent, window, cx| {
                                        if activates_button(event) {
                                            this.resolve_confirmation(false, window, cx);
                                            cx.stop_propagation();
                                        }
                                    },
                                )),
                            )
                            .child(
                                confirm_button(
                                    "confirmation-confirm",
                                    confirmation.confirm_label,
                                    true,
                                )
                                .bg(if danger { WARN.rgba() } else { ACCENT.rgba() })
                                .text_color(if danger {
                                    gpui::rgb(0xffffff)
                                } else {
                                    ACCENT_FG.rgba()
                                })
                                .hover(move |style| {
                                    style.bg(cellar_desktop_gpui::theme::hover_bright(if danger {
                                        WARN.rgba()
                                    } else {
                                        ACCENT.rgba()
                                    }))
                                })
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.resolve_confirmation(true, window, cx)
                                }))
                                .on_key_down(cx.listener(
                                    |this, event: &KeyDownEvent, window, cx| {
                                        if activates_button(event) {
                                            this.resolve_confirmation(true, window, cx);
                                            cx.stop_propagation();
                                        }
                                    },
                                )),
                            ),
                    ),
            )
            .into_any_element()
    }
}

fn confirm_button(
    id: &'static str,
    label: &'static str,
    primary: bool,
) -> gpui::Stateful<gpui::Div> {
    div()
        .id(id)
        .tab_index(0)
        .cursor_pointer()
        .h(ui_px(26.))
        .flex()
        .items_center()
        .rounded(ui_px(4.))
        .border_1()
        .border_color(if primary { accent(0.) } else { BORDER.rgba() })
        .px_3()
        .text_size(ui_px(14.))
        .font_weight(gpui::FontWeight::MEDIUM)
        .text_color(if primary { ACCENT_FG } else { FG_SECONDARY })
        .focus(|style| style.border_color(ACCENT))
        .child(label)
}

fn activates_button(event: &KeyDownEvent) -> bool {
    !event.keystroke.modifiers.modified()
        && matches!(event.keystroke.key.as_str(), "enter" | "space")
}

fn can_replace_confirmation(existing: Option<&Confirmation>) -> bool {
    matches!(
        existing.map(|confirmation| &confirmation.action),
        None | Some(ConfirmAction::Reconnect(_))
    )
}

#[cfg(test)]
mod tests {
    use gpui::{KeyDownEvent, Keystroke};

    use super::{activates_button, can_replace_confirmation, ConfirmAction, Confirmation};

    #[test]
    fn connection_error_confirmation_titles_retry_and_reconnect_id() {
        let confirmation =
            Confirmation::connection_error("conn-1".into(), "prod", "timeout".into());
        assert_eq!(confirmation.title, "Could not connect to prod");
        assert_eq!(confirmation.message, "timeout");
        assert_eq!(confirmation.confirm_label, "Retry");
        assert_eq!(confirmation.cancel_label, "Close");
        assert!(!confirmation.danger);
        assert!(matches!(
            confirmation.action,
            ConfirmAction::Reconnect(ref id) if id == "conn-1"
        ));
    }

    #[test]
    fn connection_error_does_not_replace_an_unrelated_confirmation() {
        let removal = Confirmation {
            title: "Remove connection".into(),
            message: "gone".into(),
            confirm_label: "Remove",
            cancel_label: "Cancel",
            danger: true,
            action: ConfirmAction::RemoveConnection("other".into()),
        };
        assert!(!can_replace_confirmation(Some(&removal)));
        assert!(can_replace_confirmation(None));
        let reconnect = Confirmation::connection_error("conn-1".into(), "prod", "timeout".into());
        assert!(can_replace_confirmation(Some(&reconnect)));
    }

    #[test]
    fn confirmation_buttons_support_canonical_keyboard_activation() {
        for key in ["enter", "space"] {
            assert!(activates_button(&KeyDownEvent {
                keystroke: Keystroke::parse(key).expect("valid key"),
                is_held: false,
            }));
        }
    }
}
