use std::{fs, path::PathBuf, sync::Arc};

use cellar_ai::{
    backend::{
        AiThinkingMode, BackendAiChatMessage, BackendAiGenerateRequest, BackendAiProvider,
        BackendAiService,
    },
    google::{GoogleChatMessage, GoogleGenerateRequest, GoogleService},
    openai::{
        OpenAiAuthMode, OpenAiChatMessage, OpenAiGenerateRequest, OpenAiLoginMethod,
        OpenAiLoginStart, OpenAiOAuthStatus, OpenAiService,
    },
};
use gpui::{AppContext, Context, Entity, Subscription, Window};
use gpui_component::input::InputState;
use serde::{Deserialize, Serialize};

use super::ai_history::clean_history;
use super::CellarApp;

pub(super) const SYSTEM_PROMPT: &str = r#"You are Cellar's built-in SQL assistant, embedded in a desktop database client.

Rules:
- The user works against a real database. Prefer correct, dialect-appropriate SQL over generic advice.
- When you return SQL, put it in a single fenced ```sql code block so the app can render and gate it.
- Use the schema context provided. Never invent table or column names; ask when one is missing.
- Cellar runs queries read-only by default. Flag destructive statements explicitly.
- Be concise and technical. The user understands SQL."#;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(super) enum AiProvider {
    #[default]
    Google,
    Openai,
    Deepseek,
}

impl AiProvider {
    pub(super) fn id(self) -> &'static str {
        match self {
            Self::Google => "google",
            Self::Openai => "openai",
            Self::Deepseek => "deepseek",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum AiTopic {
    Generate,
    Explain,
    Optimize,
    Migrate,
    #[default]
    Ask,
}

impl AiTopic {
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Generate => "generate",
            Self::Explain => "explain",
            Self::Optimize => "optimize",
            Self::Migrate => "migrate",
            Self::Ask => "ask",
        }
    }
}

#[derive(Clone, Debug)]
pub(super) struct AiModel {
    pub id: String,
    pub label: String,
}

#[derive(Clone, Debug)]
pub(super) struct AiMessage {
    pub role: AiRole,
    pub topic: AiTopic,
    pub content: String,
    pub api_content: Option<String>,
    pub error: bool,
    pub total_tokens: Option<u64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum AiRole {
    User,
    Model,
}

#[derive(Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct PersistedAi {
    provider: AiProvider,
    model_id: Option<String>,
    openai_chatgpt: bool,
    deepseek_thinking: bool,
}

impl Default for PersistedAi {
    fn default() -> Self {
        Self {
            provider: AiProvider::Google,
            model_id: None,
            openai_chatgpt: false,
            deepseek_thinking: true,
        }
    }
}

pub(super) struct AiState {
    pub provider: AiProvider,
    pub model_id: Option<String>,
    pub models: Vec<AiModel>,
    pub configured: bool,
    pub models_loading: bool,
    pub error: Option<String>,
    pub messages: Vec<AiMessage>,
    pub sending: bool,
    pub topic: AiTopic,
    pub draft: Entity<InputState>,
    pub key: Entity<InputState>,
    pub key_visible: bool,
    pub openai_chatgpt: bool,
    pub oauth_status: Option<OpenAiOAuthStatus>,
    pub login: Option<OpenAiLoginStart>,
    pub auth_loading: bool,
    pub openai_thread_id: Option<String>,
    pub deepseek_thinking: bool,
    google: Arc<GoogleService>,
    backend: Arc<BackendAiService>,
    pub(super) openai: Arc<OpenAiService>,
    _draft_subscription: Subscription,
    _key_subscription: Subscription,
}

impl AiState {
    pub(super) fn new(window: &mut Window, cx: &mut Context<CellarApp>) -> Self {
        let saved = load_ai();
        let draft = cx.new(|cx| {
            InputState::new(window, cx)
                .multi_line(true)
                .placeholder("Configure a provider in AI settings to start…")
        });
        let key = cx.new(|cx| {
            InputState::new(window, cx)
                .masked(true)
                .placeholder("API key")
        });
        let draft_subscription = cx.observe(&draft, |_, _, cx| cx.notify());
        let key_subscription = cx.observe(&key, |_, _, cx| cx.notify());
        Self {
            provider: saved.provider,
            model_id: saved.model_id,
            models: Vec::new(),
            configured: false,
            models_loading: false,
            error: None,
            messages: Vec::new(),
            sending: false,
            topic: AiTopic::Ask,
            draft,
            key,
            key_visible: false,
            openai_chatgpt: saved.openai_chatgpt,
            oauth_status: None,
            login: None,
            auth_loading: false,
            openai_thread_id: None,
            deepseek_thinking: saved.deepseek_thinking,
            google: Arc::new(GoogleService::default()),
            backend: Arc::new(BackendAiService::default()),
            openai: Arc::new(OpenAiService::default()),
            _draft_subscription: draft_subscription,
            _key_subscription: key_subscription,
        }
    }
}

impl CellarApp {
    pub(crate) fn initialize_ai(&mut self, cx: &mut Context<Self>) {
        if self.ai.provider == AiProvider::Openai && self.ai.openai_chatgpt {
            self.refresh_openai_status(cx);
        } else {
            self.refresh_ai_models(cx);
        }
    }

    pub(super) fn select_ai_provider(&mut self, provider: AiProvider, cx: &mut Context<Self>) {
        if self.ai.provider == provider {
            return;
        }
        self.ai.provider = provider;
        self.ai.model_id = None;
        self.ai.models.clear();
        self.ai.configured = false;
        self.ai.error = None;
        self.ai.openai_thread_id = None;
        self.ai.oauth_status = None;
        self.ai.login = None;
        self.ai_auth_poll = None;
        self.ai_generation += 1;
        self.persist_ai();
        if provider == AiProvider::Openai && self.ai.openai_chatgpt {
            self.refresh_openai_status(cx);
        } else {
            self.refresh_ai_models(cx);
        }
    }

    pub(super) fn set_openai_chatgpt(&mut self, chatgpt: bool, cx: &mut Context<Self>) {
        if self.ai.openai_chatgpt == chatgpt {
            return;
        }
        self.ai.openai_chatgpt = chatgpt;
        self.ai.model_id = None;
        self.ai.models.clear();
        self.ai.configured = false;
        self.ai.error = None;
        self.ai.openai_thread_id = None;
        self.ai_auth_poll = None;
        self.ai_generation += 1;
        self.persist_ai();
        if chatgpt {
            self.refresh_openai_status(cx);
        } else {
            self.refresh_ai_models(cx);
        }
    }

    pub(super) fn refresh_openai_status(&mut self, cx: &mut Context<Self>) {
        let service = Arc::clone(&self.ai.openai);
        let runtime = Arc::clone(&self.runtime);
        self.ai.auth_loading = true;
        self.ai.error = None;
        cx.spawn(async move |this, cx| {
            let result = runtime
                .spawn(async move {
                    service
                        .oauth_status()
                        .await
                        .map_err(|error| error.to_string())
                })
                .await
                .map_err(|error| error.to_string())
                .and_then(|result| result);
            this.update(cx, |this, cx| {
                this.ai.auth_loading = false;
                match result {
                    Ok(status) => {
                        let signed_in = status.signed_in;
                        this.ai.oauth_status = Some(status);
                        this.ai.configured = signed_in;
                        if signed_in {
                            this.ai.login = None;
                            this.refresh_ai_models(cx);
                        }
                    }
                    Err(error) => {
                        this.ai.configured = false;
                        this.ai.error = Some(error);
                    }
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
        cx.notify();
    }

    pub(super) fn start_openai_login(&mut self, method: OpenAiLoginMethod, cx: &mut Context<Self>) {
        let service = Arc::clone(&self.ai.openai);
        let runtime = Arc::clone(&self.runtime);
        self.ai.auth_loading = true;
        self.ai.error = None;
        cx.spawn(async move |this, cx| {
            let result = runtime
                .spawn(async move {
                    service
                        .start_login(method)
                        .await
                        .map_err(|error| error.to_string())
                })
                .await
                .map_err(|error| error.to_string())
                .and_then(|result| result);
            this.update(cx, |this, cx| {
                this.ai.auth_loading = false;
                match result {
                    Ok(login) => {
                        cx.open_url(&login.auth_url);
                        this.ai.login = Some(login);
                        this.start_openai_auth_poll(cx);
                    }
                    Err(error) => this.ai.error = Some(error),
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
        cx.notify();
    }

    pub(super) fn cancel_openai_login(&mut self, cx: &mut Context<Self>) {
        let Some(login_id) = self.ai.login.as_ref().map(|login| login.login_id.clone()) else {
            return;
        };
        let service = Arc::clone(&self.ai.openai);
        let runtime = Arc::clone(&self.runtime);
        self.ai.auth_loading = true;
        self.ai_auth_poll = None;
        cx.spawn(async move |this, cx| {
            let result = runtime
                .spawn(async move {
                    service
                        .cancel_login(&login_id)
                        .await
                        .map_err(|error| error.to_string())
                })
                .await
                .map_err(|error| error.to_string())
                .and_then(|result| result);
            this.update(cx, |this, cx| {
                this.ai.auth_loading = false;
                match result {
                    Ok(()) => this.ai.login = None,
                    Err(error) => this.ai.error = Some(error),
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    pub(super) fn logout_openai(&mut self, cx: &mut Context<Self>) {
        let service = Arc::clone(&self.ai.openai);
        let runtime = Arc::clone(&self.runtime);
        self.ai.auth_loading = true;
        self.ai_auth_poll = None;
        cx.spawn(async move |this, cx| {
            let result = runtime
                .spawn(async move { service.logout().await.map_err(|error| error.to_string()) })
                .await
                .map_err(|error| error.to_string())
                .and_then(|result| result);
            this.update(cx, |this, cx| {
                this.ai.auth_loading = false;
                match result {
                    Ok(()) => {
                        this.ai.oauth_status = None;
                        this.ai.login = None;
                        this.ai.configured = false;
                        this.ai.models.clear();
                        this.ai.model_id = None;
                        this.ai.openai_thread_id = None;
                    }
                    Err(error) => this.ai.error = Some(error),
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    pub(super) fn save_ai_key(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let key = self.ai.key.read(cx).unmask_value().trim().to_owned();
        if key.is_empty() {
            self.ai.error = Some("Enter an API key first.".into());
            cx.notify();
            return;
        }
        let provider = self.ai.provider.id().to_owned();
        let runtime = Arc::clone(&self.runtime);
        let window_handle = window.window_handle();
        self.ai.models_loading = true;
        self.ai.error = None;
        cx.spawn(async move |this, cx| {
            let result = runtime
                .spawn(async move {
                    cellar_ai::store_key(&provider, &key).map_err(|error| error.to_string())
                })
                .await
                .map_err(|error| error.to_string())
                .and_then(|result| result);
            let _ = cx.update_window(window_handle, |view, window, cx| {
                let Ok(app) = view.downcast::<CellarApp>() else {
                    return;
                };
                app.update(cx, |app, cx| match result {
                    Ok(()) => {
                        app.ai
                            .key
                            .update(cx, |key, cx| key.set_value("", window, cx));
                        app.refresh_ai_models(cx);
                    }
                    Err(error) => {
                        app.ai.models_loading = false;
                        app.ai.error = Some(error);
                        cx.notify();
                    }
                });
            });
            drop(this);
        })
        .detach();
        cx.notify();
    }

    pub(super) fn delete_ai_key(&mut self, cx: &mut Context<Self>) {
        let provider = self.ai.provider.id().to_owned();
        let runtime = Arc::clone(&self.runtime);
        self.ai_generation += 1;
        self.ai.configured = false;
        self.ai.models.clear();
        self.ai.model_id = None;
        cx.spawn(async move |this, cx| {
            let result = runtime
                .spawn(async move {
                    cellar_ai::delete_key(&provider).map_err(|error| error.to_string())
                })
                .await
                .map_err(|error| error.to_string())
                .and_then(|result| result);
            this.update(cx, |this, cx| {
                this.ai.error = result.err();
                this.persist_ai();
                cx.notify();
            })
            .ok();
        })
        .detach();
        cx.notify();
    }

    pub(super) fn refresh_ai_models(&mut self, cx: &mut Context<Self>) {
        self.ai_generation += 1;
        let generation = self.ai_generation;
        let provider = self.ai.provider;
        let chatgpt = self.ai.openai_chatgpt;
        let google = Arc::clone(&self.ai.google);
        let backend = Arc::clone(&self.ai.backend);
        let openai = Arc::clone(&self.ai.openai);
        let runtime = Arc::clone(&self.runtime);
        self.ai.models_loading = true;
        self.ai.error = None;
        cx.spawn(async move |this, cx| {
            let result = runtime
                .spawn(async move {
                    if provider != AiProvider::Openai || !chatgpt {
                        let has_key =
                            cellar_ai::has_key(provider.id()).map_err(|error| error.to_string())?;
                        if !has_key {
                            return Ok(Vec::new());
                        }
                    }
                    match provider {
                        AiProvider::Google => google
                            .list_models()
                            .await
                            .map(|models| {
                                models
                                    .into_iter()
                                    .map(|model| AiModel {
                                        id: model.id,
                                        label: model.label,
                                    })
                                    .collect()
                            })
                            .map_err(|error| error.to_string()),
                        AiProvider::Deepseek => backend
                            .list_models(BackendAiProvider::Deepseek)
                            .await
                            .map(|models| {
                                models
                                    .into_iter()
                                    .map(|model| AiModel {
                                        id: model.id,
                                        label: model.label,
                                    })
                                    .collect()
                            })
                            .map_err(|error| error.to_string()),
                        AiProvider::Openai => openai
                            .list_models(if chatgpt {
                                OpenAiAuthMode::Chatgpt
                            } else {
                                OpenAiAuthMode::ApiKey
                            })
                            .await
                            .map(|models| {
                                models
                                    .into_iter()
                                    .map(|model| AiModel {
                                        id: model.id,
                                        label: model.label,
                                    })
                                    .collect()
                            })
                            .map_err(|error| error.to_string()),
                    }
                })
                .await
                .map_err(|error| error.to_string())
                .and_then(|result| result);
            this.update(cx, |this, cx| {
                if this.ai_generation != generation || this.ai.provider != provider {
                    return;
                }
                this.ai.models_loading = false;
                match result {
                    Ok(models) => {
                        this.ai.configured = !models.is_empty();
                        this.ai.models = models;
                        if !this
                            .ai
                            .models
                            .iter()
                            .any(|model| Some(&model.id) == this.ai.model_id.as_ref())
                        {
                            this.ai.model_id = this.ai.models.first().map(|model| model.id.clone());
                        }
                        this.persist_ai();
                    }
                    Err(error) => {
                        this.ai.configured = false;
                        this.ai.error = Some(error);
                    }
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
        cx.notify();
    }

    pub(super) fn cycle_ai_model(&mut self, cx: &mut Context<Self>) {
        if self.ai.models.is_empty() {
            return;
        }
        let index = self
            .ai
            .models
            .iter()
            .position(|model| Some(&model.id) == self.ai.model_id.as_ref())
            .unwrap_or(0);
        self.ai.model_id = Some(
            self.ai.models[(index + 1) % self.ai.models.len()]
                .id
                .clone(),
        );
        self.ai.openai_thread_id = None;
        self.persist_ai();
        cx.notify();
    }

    pub(super) fn set_ai_topic(&mut self, topic: AiTopic, cx: &mut Context<Self>) {
        self.ai.topic = if self.ai.topic == topic {
            AiTopic::Ask
        } else {
            topic
        };
        cx.notify();
    }

    pub(super) fn new_ai_thread(&mut self, cx: &mut Context<Self>) {
        self.ai_task.take();
        self.ai.messages.clear();
        self.ai.sending = false;
        self.ai.openai_thread_id = None;
        cx.notify();
    }

    pub(super) fn send_ai(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.ai.sending || !self.ai.configured {
            return;
        }
        let text = self.ai.draft.read(cx).value().trim().to_owned();
        let Some(model) = self.ai.model_id.clone() else {
            return;
        };
        if text.is_empty() {
            return;
        }
        let topic = self.ai.topic;
        let prompt = self.build_ai_prompt(topic, &text, cx);
        self.ai.messages.push(AiMessage {
            role: AiRole::User,
            topic,
            content: text,
            api_content: Some(prompt),
            error: false,
            total_tokens: None,
        });
        let history = clean_history(&self.ai.messages);
        let provider = self.ai.provider;
        let chatgpt = self.ai.openai_chatgpt;
        let thread_id = self.ai.openai_thread_id.clone();
        let thinking = self.ai.deepseek_thinking;
        let google = Arc::clone(&self.ai.google);
        let backend = Arc::clone(&self.ai.backend);
        let openai = Arc::clone(&self.ai.openai);
        let runtime = Arc::clone(&self.runtime);
        self.ai
            .draft
            .update(cx, |draft, cx| draft.set_value("", window, cx));
        self.ai.sending = true;
        self.ai_task = Some(cx.spawn(async move |this, cx| {
            let result = runtime
                .spawn(async move {
                    match provider {
                        AiProvider::Google => google
                            .generate(GoogleGenerateRequest {
                                model,
                                messages: history
                                    .iter()
                                    .map(|(role, content)| GoogleChatMessage {
                                        role: if *role == AiRole::Model {
                                            "model"
                                        } else {
                                            "user"
                                        }
                                        .into(),
                                        content: content.clone(),
                                    })
                                    .collect(),
                                system_instruction: Some(SYSTEM_PROMPT.into()),
                            })
                            .await
                            .map(|result| (result.text, Some(result.total_tokens), None))
                            .map_err(|error| error.to_string()),
                        AiProvider::Deepseek => backend
                            .generate(
                                BackendAiProvider::Deepseek,
                                BackendAiGenerateRequest {
                                    model,
                                    messages: history
                                        .iter()
                                        .map(|(role, content)| BackendAiChatMessage {
                                            role: if *role == AiRole::Model {
                                                "assistant"
                                            } else {
                                                "user"
                                            }
                                            .into(),
                                            content: content.clone(),
                                        })
                                        .collect(),
                                    system_instruction: Some(SYSTEM_PROMPT.into()),
                                    thinking: Some(if thinking {
                                        AiThinkingMode::Enabled
                                    } else {
                                        AiThinkingMode::Disabled
                                    }),
                                },
                            )
                            .await
                            .map(|result| {
                                (
                                    result.text,
                                    result.usage.map(|usage| usage.total_tokens),
                                    None,
                                )
                            })
                            .map_err(|error| error.to_string()),
                        AiProvider::Openai => openai
                            .generate(
                                if chatgpt {
                                    OpenAiAuthMode::Chatgpt
                                } else {
                                    OpenAiAuthMode::ApiKey
                                },
                                OpenAiGenerateRequest {
                                    model,
                                    messages: history
                                        .iter()
                                        .map(|(role, content)| OpenAiChatMessage {
                                            role: if *role == AiRole::Model {
                                                "assistant"
                                            } else {
                                                "user"
                                            }
                                            .into(),
                                            content: content.clone(),
                                        })
                                        .collect(),
                                    system_instruction: Some(SYSTEM_PROMPT.into()),
                                    thread_id,
                                },
                            )
                            .await
                            .map(|result| {
                                (
                                    result.text,
                                    result.usage.map(|usage| usage.total_tokens),
                                    result.thread_id,
                                )
                            })
                            .map_err(|error| error.to_string()),
                    }
                })
                .await
                .map_err(|error| error.to_string())
                .and_then(|result| result);
            this.update(cx, |this, cx| {
                this.ai.sending = false;
                match result {
                    Ok((content, tokens, thread)) => {
                        this.ai.openai_thread_id = thread;
                        this.ai.messages.push(AiMessage {
                            role: AiRole::Model,
                            topic,
                            content,
                            api_content: None,
                            error: false,
                            total_tokens: tokens,
                        });
                    }
                    Err(error) => this.ai.messages.push(AiMessage {
                        role: AiRole::Model,
                        topic,
                        content: error,
                        api_content: None,
                        error: true,
                        total_tokens: None,
                    }),
                }
                this.ai_task = None;
                cx.notify();
            })
            .ok();
        }));
        cx.notify();
    }

    pub(super) fn persist_ai(&self) {
        let value = PersistedAi {
            provider: self.ai.provider,
            model_id: self.ai.model_id.clone(),
            openai_chatgpt: self.ai.openai_chatgpt,
            deepseek_thinking: self.ai.deepseek_thinking,
        };
        if let (Some(path), Ok(bytes)) = (ai_path(), serde_json::to_vec_pretty(&value)) {
            let _ = super::setup_transfer::write_setup(&path, &bytes);
        }
    }
}

fn ai_path() -> Option<PathBuf> {
    Some(cellar_runtime::cellar_dir()?.join("ai.json"))
}
fn load_ai() -> PersistedAi {
    ai_path()
        .and_then(|path| fs::read(path).ok())
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default()
}
