use gpui::{
    div, prelude::*, px, AnyElement, Context, KeyDownEvent, MouseButton, MouseDownEvent,
    SharedString,
};
use gpui_component::{scroll::ScrollableElement, Icon};

use super::{
    ai::{AiRole, AiTopic},
    CellarApp,
};
use cellar_desktop_gpui::theme::{
    ui_px, ACCENT, BORDER, BORDER_SEPARATOR, FG, FG_MUTED, FG_SECONDARY, INSET, PANEL,
    PANEL_RAISED, PROD,
};
use cellar_desktop_gpui::widgets::compact_input;

impl CellarApp {
    pub(super) fn render_ai_panel(&self, cx: &mut Context<Self>) -> AnyElement {
        let model = self.ai.model_id.as_deref().unwrap_or("not configured");
        let ready = self.ai.configured && self.ai.model_id.is_some();
        let can_send =
            ready && !self.ai.sending && !self.ai.draft.read(cx).value().trim().is_empty();
        let (_, chips) = self.ai_context(cx);
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
            .child(
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
                    ),
            )
            .child(
                div()
                    .h(px(32.))
                    .flex_shrink_0()
                    .flex()
                    .items_center()
                    .justify_between()
                    .pl(px(10.))
                    .pr(px(8.))
                    .border_b_1()
                    .border_color(BORDER)
                    .child(
                        div()
                            .min_w_0()
                            .flex()
                            .items_center()
                            .gap(px(6.))
                            .child(
                                Icon::empty()
                                    .path("icons/asterisk.svg")
                                    .size(px(12.))
                                    .text_color(ACCENT),
                            )
                            .child(
                                div()
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .text_color(FG)
                                    .child("AI Assistant"),
                            )
                            .child(
                                div()
                                    .min_w_0()
                                    .truncate()
                                    .font_family(cellar_desktop_gpui::theme::mono_font())
                                    .text_size(px(11.5))
                                    .text_color(FG_MUTED)
                                    .child(format!("· {model}")),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(1.))
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
                                        this.open_settings(
                                            super::settings::SettingsCategory::Ai,
                                            cx,
                                        )
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
                    ),
            )
            .child(
                div()
                    .flex_shrink_0()
                    .flex()
                    .items_start()
                    .gap(px(6.))
                    .border_b_1()
                    .border_color(BORDER)
                    .bg(PANEL_RAISED)
                    .px_2()
                    .py(px(6.))
                    .child(
                        div()
                            .h(px(20.))
                            .flex()
                            .items_center()
                            .gap_1()
                            .text_size(px(10.5))
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_color(FG_MUTED)
                            .child(Icon::empty().path("icons/context.svg").size(px(10.)))
                            .child("CONTEXT"),
                    )
                    .child(
                        div()
                            .flex_1()
                            .flex()
                            .flex_wrap()
                            .gap_1()
                            .when(chips.is_empty(), |element| {
                                element.child(
                                    div()
                                        .h(px(20.))
                                        .flex()
                                        .items_center()
                                        .text_size(px(11.5))
                                        .text_color(FG_MUTED)
                                        .child("no active connection"),
                                )
                            })
                            .children(chips.into_iter().map(|chip| {
                                div()
                                    .h(px(20.))
                                    .flex()
                                    .items_center()
                                    .gap_1()
                                    .rounded(px(4.))
                                    .border_1()
                                    .border_color(BORDER)
                                    .bg(PANEL)
                                    .px(px(6.))
                                    .text_size(px(11.5))
                                    .child(
                                        div()
                                            .text_size(px(10.5))
                                            .text_color(FG_MUTED)
                                            .child(chip.kind),
                                    )
                                    .child(":")
                                    .child(
                                        div()
                                            .font_family(cellar_desktop_gpui::theme::mono_font())
                                            .text_color(FG)
                                            .child(chip.value),
                                    )
                            })),
                    ),
            )
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scrollbar()
                    .px(px(10.))
                    .pt_3()
                    .pb_4()
                    .when(self.ai.messages.is_empty(), |element| {
                        element.child(empty_ai(ready, cx))
                    })
                    .children(self.ai.messages.iter().enumerate().map(|(index, message)| {
                        let user = message.role == AiRole::User;
                        div()
                            .id(SharedString::from(format!("ai-message-{index}")))
                            .mb_3()
                            .flex()
                            .justify_end()
                            .when(!user, |element| element.justify_start())
                            .child(
                                div()
                                    .max_w(px(if user { 290. } else { 340. }))
                                    .rounded(px(6.))
                                    .border_1()
                                    .border_color(if message.error { PROD } else { BORDER })
                                    .bg(if user { PANEL_RAISED } else { INSET })
                                    .px_3()
                                    .py_2()
                                    .child(
                                        div()
                                            .mb_1()
                                            .flex()
                                            .items_center()
                                            .justify_between()
                                            .text_size(px(10.5))
                                            .text_color(if message.error { PROD } else { FG_MUTED })
                                            .child(format!(
                                                "{} · {}",
                                                if user { "you" } else { "cellar ai" },
                                                message.topic.label()
                                            ))
                                            .when_some(message.total_tokens, |element, tokens| {
                                                element.child(format!("{tokens} tokens"))
                                            }),
                                    )
                                    .child(if !user && !message.error {
                                        self.render_ai_message(&message.content, index, cx)
                                    } else {
                                        div()
                                            .text_color(if message.error {
                                                PROD
                                            } else {
                                                FG_SECONDARY
                                            })
                                            .child(message.content.clone())
                                            .into_any_element()
                                    }),
                            )
                    }))
                    .when(self.ai.sending, |element| {
                        element.child(
                            div()
                                .flex()
                                .items_center()
                                .gap_2()
                                .px_1()
                                .text_color(FG_MUTED)
                                .child(
                                    Icon::empty()
                                        .path("icons/asterisk.svg")
                                        .size(px(13.))
                                        .text_color(ACCENT),
                                )
                                .child("thinking…"),
                        )
                    }),
            )
            .child(
                div()
                    .flex_shrink_0()
                    .border_t_1()
                    .border_color(BORDER)
                    .child(
                        div()
                            .h(px(32.))
                            .flex()
                            .items_center()
                            .gap(px(2.))
                            .border_b_1()
                            .border_color(BORDER)
                            .px(px(6.))
                            .children(
                                [
                                    AiTopic::Generate,
                                    AiTopic::Explain,
                                    AiTopic::Optimize,
                                    AiTopic::Migrate,
                                ]
                                .into_iter()
                                .map(|topic| {
                                    topic_button(topic, self.ai.topic == topic).on_click(
                                        cx.listener(move |this, _, _, cx| {
                                            this.set_ai_topic(topic, cx)
                                        }),
                                    )
                                }),
                            )
                            .child(div().flex_1())
                            .child(
                                div()
                                    .px_2()
                                    .text_size(px(11.5))
                                    .text_color(FG_MUTED)
                                    .child("ask · read-only"),
                            ),
                    )
                    .child(
                        div()
                            .px(px(6.))
                            .py(px(6.))
                            .child(
                                div()
                                    .h(px(58.))
                                    .rounded(px(5.))
                                    .border_1()
                                    .border_color(BORDER)
                                    .bg(INSET)
                                    .on_key_down(cx.listener(
                                        |this, event: &KeyDownEvent, window, cx| {
                                            if event.keystroke.key == "enter"
                                                && !event.keystroke.modifiers.shift
                                            {
                                                this.send_ai(window, cx);
                                                cx.stop_propagation();
                                            }
                                        },
                                    ))
                                    .child(compact_input(&self.ai.draft)),
                            )
                            .child(
                                div()
                                    .mt_1()
                                    .h(px(24.))
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap_1()
                                            .text_size(px(11.5))
                                            .text_color(FG_MUTED)
                                            .child(
                                                Icon::empty()
                                                    .path("icons/paperclip.svg")
                                                    .size(px(11.))
                                                    .opacity(0.45),
                                            )
                                            .child(self.ai.topic.label())
                                            .child("·")
                                            .child(
                                                div()
                                                    .id("ai-cycle-model")
                                                    .cursor_pointer()
                                                    .max_w(px(180.))
                                                    .truncate()
                                                    .font_family(
                                                        cellar_desktop_gpui::theme::mono_font(),
                                                    )
                                                    .child(model.to_owned())
                                                    .when(self.ai.models.len() > 1, |element| {
                                                        element.tab_index(0).on_click(cx.listener(
                                                            |this, _, _, cx| {
                                                                this.cycle_ai_model(cx)
                                                            },
                                                        ))
                                                    }),
                                            ),
                                    )
                                    .child(
                                        div()
                                            .id("ai-send")
                                            .h(px(22.))
                                            .flex()
                                            .items_center()
                                            .gap_1()
                                            .rounded(px(4.))
                                            .bg(if can_send { ACCENT } else { PANEL_RAISED })
                                            .px_2()
                                            .text_color(if can_send {
                                                cellar_desktop_gpui::theme::ACCENT_FG
                                            } else {
                                                FG_MUTED
                                            })
                                            .opacity(if can_send { 1. } else { 0.45 })
                                            .child("Send")
                                            .child(
                                                Icon::empty()
                                                    .path("icons/send.svg")
                                                    .size(px(11.))
                                                    .when(can_send, |icon| icon),
                                            )
                                            .when(can_send, |element| {
                                                element
                                                    .tab_index(0)
                                                    .cursor_pointer()
                                                    .hover(|style| {
                                                        style.bg(
                                                            cellar_desktop_gpui::theme::hover_bright(
                                                                ACCENT.rgba(),
                                                            ),
                                                        )
                                                    })
                                                    .on_click(cx.listener(
                                                        |this, _, window, cx| {
                                                            this.send_ai(window, cx)
                                                        },
                                                    ))
                                            }),
                                    ),
                            ),
                    ),
            )
            .into_any_element()
    }
}

fn panel_icon(id: &'static str, path: &'static str, enabled: bool) -> gpui::Stateful<gpui::Div> {
    div()
        .id(id)
        .size(px(22.))
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(4.))
        .text_color(FG_MUTED)
        .opacity(if enabled { 1. } else { 0.45 })
        .when(enabled, |element| {
            element
                .tab_index(0)
                .cursor_pointer()
                .hover(|style| style.bg(PANEL_RAISED).text_color(FG))
        })
        .child(Icon::empty().path(path).size(px(12.)))
}

fn topic_button(topic: AiTopic, active: bool) -> gpui::Stateful<gpui::Div> {
    div()
        .id(SharedString::from(format!("ai-topic-{}", topic.label())))
        .tab_index(0)
        .cursor_pointer()
        .h(px(22.))
        .flex()
        .items_center()
        .rounded(px(4.))
        .px_2()
        .text_size(px(12.))
        .bg(if active { PANEL_RAISED } else { PANEL })
        .text_color(if active { ACCENT } else { FG_MUTED })
        .child(topic.label())
}

fn empty_ai(ready: bool, cx: &mut Context<CellarApp>) -> AnyElement {
    div().h_full().flex().flex_col().items_center().justify_center().gap_2().px_6().text_center().text_color(FG_MUTED).child(Icon::empty().path("icons/asterisk.svg").size(px(22.)).text_color(ACCENT)).child(div().text_size(px(14.)).font_weight(gpui::FontWeight::MEDIUM).text_color(FG_SECONDARY).child("Ask Cellar AI")).child(div().max_w(px(280.)).text_size(px(11.5)).child("Generate SQL with full schema context, explain a result, or review a slow query. Bring your own API key; Cellar never proxies.")).when(!ready, |element| element.child(div().id("configure-ai-provider").tab_index(0).cursor_pointer().mt_2().h(px(24.)).flex().items_center().gap_1().rounded(px(5.)).border_1().border_color(ACCENT).px_2().text_color(ACCENT).child(Icon::empty().path("icons/settings.svg").size(px(11.))).child("Configure a provider").on_click(cx.listener(|this, _, _, cx| this.open_settings(super::settings::SettingsCategory::Ai, cx))))).into_any_element()
}
