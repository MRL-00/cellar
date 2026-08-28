use cellar_ai::openai::OpenAiLoginMethod;
use gpui::{div, prelude::*, px, AnyElement, Context, SharedString};
use gpui_component::{input::Input, Icon};

use super::{
    ai::AiProvider,
    settings_workspace::{
        action_button, content, row, section, section_separator, section_with_sub, toggle,
    },
    CellarApp,
};
use cellar_desktop_gpui::theme::{
    ACCENT, BORDER, FG, FG_MUTED, FG_SECONDARY, INSET, PANEL, PANEL_RAISED, PROD,
};

const PROVIDERS: &[(&str, &str, Option<AiProvider>)] = &[
    ("Google", "gemini-* family", Some(AiProvider::Google)),
    ("Anthropic", "claude-* family", None),
    ("OpenAI", "GPT + ChatGPT", Some(AiProvider::Openai)),
    ("DeepSeek", "V4 Flash + Pro", Some(AiProvider::Deepseek)),
    ("Local", "Ollama, LM Studio", None),
    ("Custom", "OpenAI-compatible URL", None),
];

impl CellarApp {
    pub(super) fn ai_settings(&self, cx: &mut Context<Self>) -> AnyElement {
        let endpoint = match self.ai.provider {
            AiProvider::Google => "https://generativelanguage.googleapis.com/v1beta",
            AiProvider::Openai => "https://api.openai.com/v1",
            AiProvider::Deepseek => "https://api.deepseek.com",
        };
        content()
            .child(section_with_sub(
                "AI Assistant",
                Some("Connect directly with a provider key or use your ChatGPT subscription. Cellar never proxies AI traffic through a hosted service."),
                vec![privacy_banner()],
            ))
            .child(section_separator())
            .child(section(
                "Provider",
                vec![
                    row("Provider", None, self.provider_cards(cx)),
                    if self.ai.provider == AiProvider::Openai {
                        row("Authentication", Some("choose API billing or ChatGPT subscription access"), self.openai_auth_control(cx))
                    } else { div().into_any_element() },
                    if self.ai.provider == AiProvider::Deepseek {
                        row(
                            "Thinking mode",
                            Some("use DeepSeek's reasoning mode for more complex requests"),
                            div().flex().items_center().gap_2().child(toggle("deepseek-thinking", self.ai.deepseek_thinking, true).on_click(cx.listener(|this, _, _, cx| { this.ai.deepseek_thinking = !this.ai.deepseek_thinking; this.persist_ai(); cx.notify(); }))).child(if self.ai.deepseek_thinking { "enabled" } else { "disabled" }).into_any_element(),
                        )
                    } else { div().into_any_element() },
                    if self.ai.provider == AiProvider::Openai && self.ai.openai_chatgpt {
                        row("ChatGPT account", Some("OAuth tokens stay in Cellar's isolated local Codex runtime"), self.chatgpt_control(cx))
                    } else {
                        row("API key", Some("stored in OS keychain, never written to disk"), self.api_key_control(cx))
                    },
                    row("Model", Some("discovered from your API key"), self.model_control(cx)),
                    row("Endpoint", Some("direct, not proxied through Cellar"), readonly_endpoint(endpoint)),
                ],
            ))
            .child(section_separator())
            .child(section(
                "Danger zone",
                vec![
                    row(
                        "Clear AI conversation",
                        Some(if self.ai.messages.is_empty() { "no active conversation" } else { "clear the current local thread" }),
                        danger_button("ai-clear", "icons/trash.svg", "Clear", !self.ai.messages.is_empty()).when(!self.ai.messages.is_empty(), |button| button.on_click(cx.listener(|this, _, _, cx| this.new_ai_thread(cx)))).into_any_element(),
                    ),
                    if self.ai.provider == AiProvider::Openai && self.ai.openai_chatgpt {
                        row("Sign out of ChatGPT", Some("remove Cellar's local OAuth session"), danger_button("ai-logout", "icons/close.svg", "Sign out", self.ai.oauth_status.as_ref().is_some_and(|status| status.signed_in)).when(self.ai.oauth_status.as_ref().is_some_and(|status| status.signed_in), |button| button.on_click(cx.listener(|this, _, _, cx| this.logout_openai(cx)))).into_any_element())
                    } else {
                        row("Revoke API key", Some("remove from keychain; does not affect provider"), danger_button("ai-revoke", "icons/close.svg", "Revoke", self.ai.configured).when(self.ai.configured, |button| button.on_click(cx.listener(|this, _, _, cx| this.delete_ai_key(cx)))).into_any_element())
                    },
                ],
            ))
            .into_any_element()
    }

    fn provider_cards(&self, cx: &mut Context<Self>) -> AnyElement {
        div()
            .flex()
            .flex_wrap()
            .gap_1()
            .children(PROVIDERS.iter().map(|(label, sub, provider)| {
                let active = provider.is_some_and(|provider| self.ai.provider == provider);
                let enabled = provider.is_some();
                let select = *provider;
                div()
                    .id(SharedString::from(format!("ai-provider-{label}")))
                    .relative()
                    .w(px(150.))
                    .h(px(48.))
                    .flex()
                    .flex_col()
                    .justify_center()
                    .rounded(px(5.))
                    .border_1()
                    .border_color(if active { ACCENT } else { BORDER })
                    .bg(if active { INSET } else { PANEL_RAISED })
                    .px_2()
                    .opacity(if enabled { 1. } else { 0.55 })
                    .when_some(select, |element, provider| {
                        element.tab_index(0).cursor_pointer().on_click(
                            cx.listener(move |this, _, _, cx| {
                                this.select_ai_provider(provider, cx)
                            }),
                        )
                    })
                    .child(
                        div()
                            .font_weight(gpui::FontWeight::MEDIUM)
                            .text_color(FG)
                            .child(*label),
                    )
                    .child(
                        div()
                            .font_family(cellar_desktop_gpui::theme::mono_font())
                            .text_size(px(10.5))
                            .text_color(if active { ACCENT } else { FG_MUTED })
                            .child(*sub),
                    )
                    .when(!enabled, |element| {
                        element.child(
                            div()
                                .absolute()
                                .right(px(5.))
                                .top(px(5.))
                                .rounded(px(3.))
                                .bg(INSET)
                                .px_1()
                                .text_size(px(9.5))
                                .text_color(FG_MUTED)
                                .child("soon"),
                        )
                    })
                    .when(active, |element| {
                        element.child(
                            div()
                                .absolute()
                                .right(px(5.))
                                .top(px(5.))
                                .size(px(12.))
                                .rounded(px(6.))
                                .bg(ACCENT)
                                .flex()
                                .items_center()
                                .justify_center()
                                .child(
                                    Icon::empty()
                                        .path("icons/check.svg")
                                        .size(px(9.))
                                        .text_color(INSET),
                                ),
                        )
                    })
            }))
            .into_any_element()
    }

    fn api_key_control(&self, cx: &mut Context<Self>) -> AnyElement {
        let configured = self.ai.configured;
        let has_draft = !self.ai.key.read(cx).unmask_value().trim().is_empty();
        div()
            .flex()
            .flex_col()
            .gap_1()
            .child(
                div()
                    .flex()
                    .gap_1()
                    .child(
                        div()
                            .h(px(26.))
                            .min_w_0()
                            .flex_1()
                            .rounded(px(4.))
                            .border_1()
                            .border_color(BORDER)
                            .bg(INSET)
                            .px_1()
                            .child(Input::new(&self.ai.key).h_full().appearance(false)),
                    )
                    .child(
                        action_button(
                            "ai-reveal",
                            if self.ai.key_visible {
                                "hide"
                            } else {
                                "reveal"
                            },
                        )
                        .on_click(cx.listener(|this, _, window, cx| {
                            this.ai.key_visible = !this.ai.key_visible;
                            this.ai.key.update(cx, |key, cx| {
                                key.set_masked(!this.ai.key_visible, window, cx)
                            });
                            cx.notify();
                        })),
                    )
                    .child(
                        action_button(
                            "ai-save",
                            if self.ai.models_loading {
                                "saving…"
                            } else {
                                "save"
                            },
                        )
                        .opacity(if has_draft && !self.ai.models_loading {
                            1.
                        } else {
                            0.4
                        })
                        .when(
                            has_draft && !self.ai.models_loading,
                            |button| {
                                button.on_click(
                                    cx.listener(|this, _, window, cx| this.save_ai_key(window, cx)),
                                )
                            },
                        ),
                    ),
            )
            .child(
                div()
                    .text_size(px(11.5))
                    .text_color(if configured {
                        cellar_desktop_gpui::theme::INSERT
                    } else {
                        FG_MUTED
                    })
                    .child(if configured {
                        "✓ key configured"
                    } else {
                        "no key configured"
                    }),
            )
            .into_any_element()
    }

    fn openai_auth_control(&self, cx: &mut Context<Self>) -> AnyElement {
        div()
            .flex()
            .gap_1()
            .child(
                auth_card(
                    "openai-api-key",
                    "API key",
                    "OpenAI Platform billing",
                    !self.ai.openai_chatgpt,
                )
                .on_click(cx.listener(|this, _, _, cx| this.set_openai_chatgpt(false, cx))),
            )
            .child(
                auth_card(
                    "openai-chatgpt",
                    "ChatGPT",
                    "Subscription access",
                    self.ai.openai_chatgpt,
                )
                .on_click(cx.listener(|this, _, _, cx| this.set_openai_chatgpt(true, cx))),
            )
            .into_any_element()
    }

    fn chatgpt_control(&self, cx: &mut Context<Self>) -> AnyElement {
        if self
            .ai
            .oauth_status
            .as_ref()
            .is_some_and(|status| status.signed_in)
        {
            let status = self.ai.oauth_status.as_ref().expect("signed-in status");
            return div()
                .flex()
                .items_center()
                .justify_between()
                .rounded(px(5.))
                .border_1()
                .border_color(cellar_desktop_gpui::theme::INSERT)
                .bg(INSET)
                .px_2()
                .py_2()
                .child(
                    div()
                        .child(
                            div()
                                .text_color(cellar_desktop_gpui::theme::INSERT)
                                .child("✓ Signed in"),
                        )
                        .child(div().text_size(px(11.)).text_color(FG_MUTED).child(format!(
                                "{}{}",
                                status.email.as_deref().unwrap_or("ChatGPT account"),
                                status
                                    .plan_type
                                    .as_ref()
                                    .map_or(String::new(), |plan| format!(" · {plan}"))
                            ))),
                )
                .child(
                    action_button("chatgpt-refresh", "refresh")
                        .on_click(cx.listener(|this, _, _, cx| this.refresh_openai_status(cx))),
                )
                .into_any_element();
        }
        div()
            .flex()
            .flex_col()
            .gap_2()
            .child(
                div()
                    .flex()
                    .gap_1()
                    .child(
                        action_button(
                            "chatgpt-sign-in",
                            if self.ai.auth_loading {
                                "starting…"
                            } else {
                                "Sign in with ChatGPT"
                            },
                        )
                        .opacity(if self.ai.auth_loading || self.ai.login.is_some() {
                            0.4
                        } else {
                            1.
                        })
                        .when(
                            !self.ai.auth_loading && self.ai.login.is_none(),
                            |button| {
                                button.on_click(cx.listener(|this, _, _, cx| {
                                    this.start_openai_login(OpenAiLoginMethod::Browser, cx)
                                }))
                            },
                        ),
                    )
                    .child(
                        action_button("chatgpt-device-code", "device code")
                            .opacity(if self.ai.auth_loading || self.ai.login.is_some() {
                                0.4
                            } else {
                                1.
                            })
                            .when(!self.ai.auth_loading && self.ai.login.is_none(), |button| {
                                button.on_click(cx.listener(|this, _, _, cx| {
                                    this.start_openai_login(OpenAiLoginMethod::DeviceCode, cx)
                                }))
                            }),
                    )
                    .child(
                        action_button("chatgpt-status", "check status")
                            .on_click(cx.listener(|this, _, _, cx| this.refresh_openai_status(cx))),
                    ),
            )
            .when_some(self.ai.login.clone(), |element, login| {
                element.child(
                    div()
                        .rounded(px(4.))
                        .border_1()
                        .border_color(ACCENT)
                        .bg(INSET)
                        .px_2()
                        .py_2()
                        .text_size(px(11.5))
                        .child("Complete sign-in in your browser.")
                        .when_some(login.user_code, |element, code| {
                            element.child(
                                div()
                                    .mt_1()
                                    .font_family(cellar_desktop_gpui::theme::mono_font())
                                    .text_color(ACCENT)
                                    .child(format!("code: {code}")),
                            )
                        })
                        .child(
                            div()
                                .mt_1()
                                .flex()
                                .gap_2()
                                .child(
                                    action_button("chatgpt-open-again", "open again")
                                        .on_click(move |_, _, cx| cx.open_url(&login.auth_url)),
                                )
                                .child(
                                    danger_button(
                                        "chatgpt-cancel",
                                        "icons/close.svg",
                                        "cancel",
                                        true,
                                    )
                                    .on_click(
                                        cx.listener(|this, _, _, cx| this.cancel_openai_login(cx)),
                                    ),
                                ),
                        ),
                )
            })
            .into_any_element()
    }

    fn model_control(&self, cx: &mut Context<Self>) -> AnyElement {
        div()
            .flex()
            .flex_col()
            .gap_1()
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .text_size(px(11.5))
                    .text_color(FG_MUTED)
                    .child(if self.ai.models_loading {
                        "loading models…".into()
                    } else {
                        format!("{} models available", self.ai.models.len())
                    })
                    .child(
                        action_button("ai-model-refresh", "refresh")
                            .opacity(if self.ai.configured && !self.ai.models_loading {
                                1.
                            } else {
                                0.4
                            })
                            .when(self.ai.configured && !self.ai.models_loading, |button| {
                                button.on_click(
                                    cx.listener(|this, _, _, cx| this.refresh_ai_models(cx)),
                                )
                            }),
                    ),
            )
            .when_some(self.ai.error.clone(), |element, error| {
                element.child(
                    div()
                        .rounded(px(4.))
                        .border_1()
                        .border_color(PROD)
                        .px_2()
                        .py_1()
                        .text_color(PROD)
                        .child(error),
                )
            })
            .when(
                self.ai.models.is_empty() && self.ai.error.is_none(),
                |element| {
                    element.child(
                        div()
                            .rounded(px(4.))
                            .border_1()
                            .border_color(BORDER)
                            .px_2()
                            .py_2()
                            .text_size(px(11.5))
                            .text_color(FG_MUTED)
                            .child("Add an API key above to discover available models."),
                    )
                },
            )
            .children(self.ai.models.iter().map(|model| {
                let active = self.ai.model_id.as_deref() == Some(model.id.as_str());
                let id = model.id.clone();
                div()
                    .id(SharedString::from(format!("ai-model:{id}")))
                    .tab_index(0)
                    .cursor_pointer()
                    .flex()
                    .items_center()
                    .gap_2()
                    .rounded(px(4.))
                    .border_1()
                    .border_color(if active { ACCENT } else { BORDER })
                    .bg(if active { INSET } else { PANEL_RAISED })
                    .px_2()
                    .py_1()
                    .child(
                        div()
                            .size(px(12.))
                            .rounded(px(6.))
                            .border_1()
                            .border_color(if active { ACCENT } else { BORDER })
                            .when(active, |element| {
                                element
                                    .child(div().m(px(2.)).size(px(6.)).rounded(px(3.)).bg(ACCENT))
                            }),
                    )
                    .child(
                        div()
                            .min_w_0()
                            .flex_1()
                            .truncate()
                            .font_family(cellar_desktop_gpui::theme::mono_font())
                            .child(model.id.clone()),
                    )
                    .child(
                        div()
                            .text_size(px(11.))
                            .text_color(FG_MUTED)
                            .child(model.label.clone()),
                    )
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.ai.model_id = Some(id.clone());
                        this.ai.openai_thread_id = None;
                        this.persist_ai();
                        cx.notify();
                    }))
            }))
            .into_any_element()
    }
}

fn privacy_banner() -> AnyElement {
    div().flex().gap_2().rounded(px(6.)).border_1().border_color(ACCENT).bg(INSET).px_3().py_3().child(div().size(px(22.)).flex_shrink_0().flex().items_center().justify_center().rounded(px(5.)).border_1().border_color(ACCENT).bg(PANEL).child(Icon::empty().path("icons/sparkles.svg").size(px(12.)).text_color(ACCENT))).child(div().child(div().mb(px(2.)).font_weight(gpui::FontWeight::SEMIBOLD).text_color(FG).child("Local credentials, by design")).child(div().text_color(FG_SECONDARY).child("Provider keys stay in the OS keychain. OpenAI and DeepSeek requests run in Rust; credentials never enter query context or setup exports."))).into_any_element()
}

fn readonly_endpoint(endpoint: &'static str) -> AnyElement {
    div()
        .h(px(26.))
        .flex()
        .items_center()
        .rounded(px(4.))
        .border_1()
        .border_color(BORDER)
        .bg(INSET)
        .px_2()
        .font_family(cellar_desktop_gpui::theme::mono_font())
        .text_color(FG_SECONDARY)
        .opacity(0.8)
        .child(endpoint)
        .into_any_element()
}

fn auth_card(
    id: &'static str,
    label: &'static str,
    hint: &'static str,
    active: bool,
) -> gpui::Stateful<gpui::Div> {
    div()
        .id(id)
        .tab_index(0)
        .cursor_pointer()
        .flex_1()
        .rounded(px(5.))
        .border_1()
        .border_color(if active { ACCENT } else { BORDER })
        .bg(if active { INSET } else { PANEL_RAISED })
        .px_2()
        .py_2()
        .child(
            div()
                .font_weight(gpui::FontWeight::MEDIUM)
                .text_color(FG)
                .child(label),
        )
        .child(div().text_size(px(11.)).text_color(FG_MUTED).child(hint))
}

fn danger_button(
    id: &'static str,
    icon: &'static str,
    label: &'static str,
    enabled: bool,
) -> gpui::Stateful<gpui::Div> {
    div()
        .id(id)
        .h(px(26.))
        .flex()
        .items_center()
        .gap_1()
        .rounded(px(4.))
        .border_1()
        .border_color(PROD)
        .px_2()
        .text_color(PROD)
        .opacity(if enabled { 1. } else { 0.45 })
        .when(enabled, |element| element.tab_index(0).cursor_pointer())
        .child(Icon::empty().path(icon).size(px(11.)))
        .child(label)
}
