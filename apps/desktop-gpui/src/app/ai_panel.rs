use gpui::{
    div, prelude::*, relative, AnyElement, ClipboardItem, Context, Corner, KeyDownEvent,
    MouseButton, MouseDownEvent, SharedString, Window,
};
use gpui_component::{
    button::{Button, ButtonVariants},
    input::{Enter, Input},
    menu::{DropdownMenu, PopupMenuItem},
    scroll::Scrollbar,
    Disableable, Icon, Sizable,
};

use super::{
    ai::{AiMessage, AiRole},
    CellarApp,
};
use cellar_desktop_gpui::theme::{
    ui_px, ACCENT, ACCENT_FG, BORDER, BORDER_SEPARATOR, BORDER_STRONG, FG, FG_MUTED, FG_SECONDARY,
    INSET, PANEL, PANEL_RAISED, PROD, WARN,
};

impl CellarApp {
    pub(super) fn render_ai_panel(
        &self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let ready = self.ai.configured && self.ai.model_id.is_some();
        let (_, chips) = self.ai_context(cx);
        // Follow new content (replies, late markdown layout) while the reader
        // is at the bottom; leave them alone once they scroll up.
        let scroll = self.ai.messages_scroll.clone();
        if scroll.offset().y <= -scroll.max_offset().height + ui_px(4.) {
            scroll.scroll_to_bottom();
        }
        div()
            .relative()
            .w(ui_px(self.right_panel_width))
            .h_full()
            .flex_shrink_0()
            .flex()
            .flex_col()
            .bg(PANEL)
            .border_l_1()
            .border_color(BORDER)
            .text_size(ui_px(13.))
            .line_height(ui_px(20.))
            .text_color(FG)
            .child(self.ai_panel_resizer(cx))
            .child(self.ai_panel_header(cx))
            .child(
                div()
                    .flex_shrink_0()
                    .flex()
                    .items_center()
                    .flex_wrap()
                    .gap(ui_px(4.))
                    .border_b_1()
                    .border_color(BORDER)
                    .px(ui_px(10.))
                    .py(ui_px(6.))
                    .text_size(ui_px(11.5))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(ui_px(4.))
                            .mr(ui_px(2.))
                            .text_color(FG_MUTED)
                            .child(Icon::empty().path("icons/context.svg").size(ui_px(11.)))
                            .child("Context"),
                    )
                    .when(chips.is_empty(), |element| {
                        element.child(div().text_color(FG_MUTED).child("no active connection"))
                    })
                    .children(chips.into_iter().map(|chip| {
                        div()
                            .h(ui_px(20.))
                            .flex()
                            .items_center()
                            .gap(ui_px(4.))
                            .rounded(ui_px(4.))
                            .bg(PANEL_RAISED)
                            .px(ui_px(6.))
                            .child(div().text_color(FG_MUTED).child(chip.kind))
                            .child(
                                div()
                                    .font_family(cellar_desktop_gpui::theme::mono_font())
                                    .text_color(FG_SECONDARY)
                                    .child(chip.value),
                            )
                    })),
            )
            .child(
                div()
                    .relative()
                    .flex_1()
                    .min_h_0()
                    .child(
                        div()
                            .id("ai-messages")
                            .size_full()
                            .overflow_y_scroll()
                            .track_scroll(&scroll)
                            .px(ui_px(14.))
                            .pt(ui_px(14.))
                            .pb(ui_px(8.))
                            .flex()
                            .flex_col()
                            .gap(ui_px(18.))
                            .when(self.ai.messages.is_empty(), |element| {
                                element.child(empty_ai(ready, cx))
                            })
                            .children(self.ai.messages.iter().enumerate().map(
                                |(index, message)| self.ai_message(index, message, window, cx),
                            ))
                            .when(self.ai.sending, |element| {
                                element.child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap(ui_px(8.))
                                        .text_color(FG_MUTED)
                                        .child(
                                            Icon::empty()
                                                .path("icons/asterisk.svg")
                                                .size(ui_px(13.))
                                                .text_color(ACCENT),
                                        )
                                        .child("Thinking…"),
                                )
                            }),
                    )
                    .child(
                        div()
                            .absolute()
                            .top_0()
                            .right_0()
                            .bottom_0()
                            .w(ui_px(10.))
                            .child(Scrollbar::vertical(&scroll)),
                    ),
            )
            .child(self.ai_composer(ready, cx))
            .into_any_element()
    }

    fn ai_panel_resizer(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .absolute()
            .left(ui_px(-3.))
            .top_0()
            .bottom_0()
            .w(ui_px(7.))
            .group("right-panel-resizer")
            .cursor_col_resize()
            .child(
                div()
                    .absolute()
                    .left(ui_px(3.))
                    .top_0()
                    .bottom_0()
                    .w(ui_px(1.))
                    .bg(if self.right_panel_resize.is_some() {
                        ACCENT.rgba()
                    } else {
                        BORDER_SEPARATOR.rgba()
                    })
                    .group_hover("right-panel-resizer", |style| {
                        style.bg(cellar_desktop_gpui::theme::accent(0.32))
                    }),
            )
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, event: &MouseDownEvent, _, cx| {
                    this.right_panel_resize =
                        Some((f32::from(event.position.x), this.right_panel_width));
                    cx.notify();
                }),
            )
    }

    fn ai_panel_header(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .h(ui_px(32.))
            .flex_shrink_0()
            .flex()
            .items_center()
            .justify_between()
            .pl(ui_px(10.))
            .pr(ui_px(8.))
            .border_b_1()
            .border_color(BORDER)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(ui_px(6.))
                    .child(
                        Icon::empty()
                            .path("icons/asterisk.svg")
                            .size(ui_px(12.))
                            .text_color(ACCENT),
                    )
                    .child(
                        div()
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .child("AI Assistant"),
                    ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(ui_px(1.))
                    .child(
                        panel_icon(
                            "ai-new-thread",
                            "icons/plus.svg",
                            !self.ai.messages.is_empty(),
                        )
                        .on_click(cx.listener(|this, _, _, cx| this.new_ai_thread(cx))),
                    )
                    .child(
                        panel_icon("ai-settings", "icons/settings.svg", true).on_click(
                            cx.listener(|this, _, _, cx| {
                                this.open_settings(super::settings::SettingsCategory::Ai, cx)
                            }),
                        ),
                    )
                    .child(
                        panel_icon("close-ai-panel", "icons/close.svg", true).on_click(
                            cx.listener(|this, _, _, cx| {
                                this.right_panel_open = false;
                                cx.notify();
                            }),
                        ),
                    ),
            )
    }

    fn ai_message(
        &self,
        index: usize,
        message: &AiMessage,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        if message.role == AiRole::User {
            return div()
                .id(SharedString::from(format!("ai-message-{index}")))
                .flex()
                .justify_end()
                .child(
                    div()
                        .max_w(relative(0.85))
                        .rounded(ui_px(12.))
                        .bg(PANEL_RAISED)
                        .px(ui_px(12.))
                        .py(ui_px(8.))
                        .child(message.content.clone()),
                )
                .into_any_element();
        }
        let meta = [
            message.model.clone(),
            message
                .total_tokens
                .map(|tokens| format!("{tokens} tokens")),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(" · ");
        div()
            .id(SharedString::from(format!("ai-message-{index}")))
            .flex()
            .flex_col()
            .gap(ui_px(6.))
            .child(if message.error {
                div()
                    .rounded(ui_px(8.))
                    .border_1()
                    .border_color(PROD)
                    .px(ui_px(12.))
                    .py(ui_px(8.))
                    .text_color(PROD)
                    .child(message.content.clone())
                    .into_any_element()
            } else {
                self.render_ai_message(&message.content, index, window, cx)
            })
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(ui_px(6.))
                    .text_size(ui_px(11.))
                    .text_color(FG_MUTED)
                    .when(!message.error, |element| {
                        let content = message.content.clone();
                        element.child(
                            panel_icon(
                                SharedString::from(format!("ai-copy-reply-{index}")),
                                "icons/copy.svg",
                                true,
                            )
                            .on_click(move |_, _, cx| {
                                cx.write_to_clipboard(ClipboardItem::new_string(content.clone()))
                            }),
                        )
                    })
                    .child(
                        div()
                            .min_w_0()
                            .truncate()
                            .font_family(cellar_desktop_gpui::theme::mono_font())
                            .child(meta),
                    ),
            )
            .into_any_element()
    }

    fn ai_composer(&self, ready: bool, cx: &mut Context<Self>) -> impl IntoElement {
        let can_send =
            ready && !self.ai.sending && !self.ai.draft.read(cx).value().trim().is_empty();
        let app = cx.weak_entity();
        let model_label = self
            .ai
            .models
            .iter()
            .find(|model| Some(&model.id) == self.ai.model_id.as_ref())
            .map(|model| model.label.clone())
            .or_else(|| self.ai.model_id.clone())
            .unwrap_or_else(|| "No model".into());
        let models = self.ai.models.clone();
        let selected = self.ai.model_id.clone();
        let allow_writes = self.ai.allow_writes;
        div().flex_shrink_0().p(ui_px(8.)).child(
            div()
                .rounded(ui_px(10.))
                .border_1()
                .border_color(BORDER_STRONG)
                .bg(INSET)
                // Enter sends; the input's own Enter action would insert a
                // newline, so intercept it before the input handles it.
                .capture_action(cx.listener(|this, _: &Enter, window, cx| {
                    this.send_ai(window, cx);
                    cx.stop_propagation();
                }))
                // Shift+Enter has no input binding, so add the newline here.
                .capture_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                    let keystroke = &event.keystroke;
                    if keystroke.key == "enter" && keystroke.modifiers.shift {
                        this.ai
                            .draft
                            .update(cx, |draft, cx| draft.insert("\n", window, cx));
                        cx.stop_propagation();
                    }
                }))
                .child(
                    div().px(ui_px(4.)).pt(ui_px(4.)).child(
                        Input::new(&self.ai.draft)
                            .appearance(false)
                            .w_full()
                            .text_size(ui_px(13.))
                            .line_height(ui_px(20.))
                            .text_color(FG),
                    ),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(ui_px(2.))
                        .px(ui_px(6.))
                        .pb(ui_px(6.))
                        .child(
                            Button::new("ai-model")
                                .label(model_label)
                                .ghost()
                                .xsmall()
                                .dropdown_caret(true)
                                .disabled(models.is_empty())
                                .dropdown_menu_with_anchor(Corner::BottomLeft, {
                                    let app = app.clone();
                                    move |menu, _, _| {
                                        models.iter().fold(menu, |menu, model| {
                                            let app = app.clone();
                                            let id = model.id.clone();
                                            menu.item(
                                                PopupMenuItem::new(model.label.clone())
                                                    .checked(selected.as_ref() == Some(&model.id))
                                                    .on_click(move |_, _, cx| {
                                                        app.update(cx, |this, cx| {
                                                            this.select_ai_model(id.clone(), cx)
                                                        })
                                                        .ok();
                                                    }),
                                            )
                                        })
                                    }
                                }),
                        )
                        .child(
                            Button::new("ai-access")
                                .label(if allow_writes {
                                    "Read-write"
                                } else {
                                    "Read-only"
                                })
                                .ghost()
                                .xsmall()
                                .dropdown_caret(true)
                                .when(allow_writes, |button| button.text_color(WARN))
                                .dropdown_menu_with_anchor(
                                    Corner::BottomLeft,
                                    move |menu, _, _| {
                                        [(false, "Read-only"), (true, "Read-write")]
                                            .into_iter()
                                            .fold(menu, |menu, (writes, label)| {
                                                let app = app.clone();
                                                menu.item(
                                                    PopupMenuItem::new(label)
                                                        .checked(writes == allow_writes)
                                                        .on_click(move |_, _, cx| {
                                                            app.update(cx, |this, cx| {
                                                                this.set_ai_allow_writes(writes, cx)
                                                            })
                                                            .ok();
                                                        }),
                                                )
                                            })
                                    },
                                ),
                        )
                        .child(div().flex_1())
                        .child(
                            div()
                                .id("ai-send")
                                .size(ui_px(26.))
                                .flex()
                                .items_center()
                                .justify_center()
                                .rounded(ui_px(13.))
                                .bg(if can_send { ACCENT } else { PANEL_RAISED })
                                .text_color(if can_send { ACCENT_FG } else { FG_MUTED })
                                .child(Icon::empty().path("icons/send.svg").size(ui_px(12.)))
                                .when(can_send, |element| {
                                    element
                                        .tab_index(0)
                                        .cursor_pointer()
                                        .hover(|style| {
                                            style.bg(cellar_desktop_gpui::theme::hover_bright(
                                                ACCENT.rgba(),
                                            ))
                                        })
                                        .on_click(cx.listener(|this, _, window, cx| {
                                            this.send_ai(window, cx)
                                        }))
                                }),
                        ),
                ),
        )
    }
}

fn panel_icon(
    id: impl Into<SharedString>,
    path: &'static str,
    enabled: bool,
) -> gpui::Stateful<gpui::Div> {
    div()
        .id(id.into())
        .size(ui_px(22.))
        .flex()
        .items_center()
        .justify_center()
        .rounded(ui_px(4.))
        .text_color(FG_MUTED)
        .opacity(if enabled { 1. } else { 0.45 })
        .when(enabled, |element| {
            element
                .tab_index(0)
                .cursor_pointer()
                .hover(|style| style.bg(PANEL_RAISED).text_color(FG))
        })
        .child(Icon::empty().path(path).size(ui_px(12.)))
}

fn empty_ai(ready: bool, cx: &mut Context<CellarApp>) -> AnyElement {
    div()
        .flex_1()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap(ui_px(8.))
        .px(ui_px(24.))
        .text_center()
        .text_color(FG_MUTED)
        .child(
            Icon::empty()
                .path("icons/asterisk.svg")
                .size(ui_px(22.))
                .text_color(ACCENT),
        )
        .child(
            div()
                .text_size(ui_px(14.))
                .font_weight(gpui::FontWeight::MEDIUM)
                .text_color(FG_SECONDARY)
                .child("Ask Cellar AI"),
        )
        .child(div().max_w(ui_px(280.)).text_size(ui_px(12.)).child(
            "Write, explain, or optimize SQL with your schema as context. Bring your own API key; Cellar never proxies.",
        ))
        .when(!ready, |element| {
            element.child(
                div()
                    .id("configure-ai-provider")
                    .tab_index(0)
                    .cursor_pointer()
                    .mt(ui_px(8.))
                    .h(ui_px(26.))
                    .flex()
                    .items_center()
                    .gap(ui_px(4.))
                    .rounded(ui_px(5.))
                    .border_1()
                    .border_color(ACCENT)
                    .px(ui_px(8.))
                    .text_color(ACCENT)
                    .child(Icon::empty().path("icons/settings.svg").size(ui_px(11.)))
                    .child("Configure a provider")
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.open_settings(super::settings::SettingsCategory::Ai, cx)
                    })),
            )
        })
        .into_any_element()
}
