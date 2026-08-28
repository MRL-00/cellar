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
        let skip_pending = confirmed && matches!(confirmation.action, ConfirmAction::Reconnect(_));
        if confirmed {
            match confirmation.action {
                ConfirmAction::Dismiss => {}
                ConfirmAction::RemoveConnection(id) => self.delete_connection_confirmed(id, cx),
                ConfirmAction::Analyze(tab_id) => {
                    self.explain_query(tab_id, PlanMode::Analyze, window, cx)
                }
                ConfirmAction::Reconnect(id) => self.reconnect(id, window, cx),
            }
        }
        cx.notify();
        if !skip_pending {
            self.show_next_pending_connection_error(window, cx);
        }
    }

    pub(super) fn show_next_pending_connection_error(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        while self.confirmation.is_none() {
            let Some(id) = dequeue_pending_connection_error(&mut self.pending_connection_errors)
            else {
                return;
            };
            self.show_connection_error(&id, Some(window), cx);
        }
    }

    pub(super) fn show_connection_error(
        &mut self,
        id: &str,
        window: Option<&mut Window>,
        cx: &mut Context<Self>,
    ) {
        if let Some(deferred) = defer_connection_error_id(self.confirmation.as_ref(), id) {
            enqueue_pending_connection_error(&mut self.pending_connection_errors, deferred);
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
        self.pending_connection_errors.retain(|queued| queued != id);
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

fn defer_connection_error_id(existing: Option<&Confirmation>, id: &str) -> Option<String> {
    (!can_replace_confirmation(existing)).then(|| id.to_owned())
}

fn enqueue_pending_connection_error(pending: &mut Vec<String>, id: String) {
    if !pending.iter().any(|queued| queued == &id) {
        pending.push(id);
    }
}

fn dequeue_pending_connection_error(pending: &mut Vec<String>) -> Option<String> {
    (!pending.is_empty()).then(|| pending.remove(0))
}

#[cfg(test)]
mod tests {
    use gpui::{KeyDownEvent, Keystroke};

    use super::{
        activates_button, can_replace_confirmation, defer_connection_error_id,
        dequeue_pending_connection_error, enqueue_pending_connection_error, ConfirmAction,
        Confirmation,
    };

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
        assert_eq!(
            defer_connection_error_id(Some(&removal), "conn-1").as_deref(),
            Some("conn-1")
        );
        assert_eq!(defer_connection_error_id(None, "conn-1"), None);
    }

    #[test]
    fn deferred_connection_errors_queue_in_order_without_duplicates() {
        let mut pending = Vec::new();
        enqueue_pending_connection_error(&mut pending, "one".into());
        enqueue_pending_connection_error(&mut pending, "two".into());
        enqueue_pending_connection_error(&mut pending, "one".into());
        assert_eq!(
            dequeue_pending_connection_error(&mut pending).as_deref(),
            Some("one")
        );
        assert_eq!(
            dequeue_pending_connection_error(&mut pending).as_deref(),
            Some("two")
        );
        assert_eq!(dequeue_pending_connection_error(&mut pending), None);
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
