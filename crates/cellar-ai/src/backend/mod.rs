//! Backend-only transports for API-key AI providers.
//!
//! Provider keys are loaded from `cellar-secrets` and never cross into the
//! renderer. The public IPC contract stays provider-neutral so additional
//! OpenAI-compatible Chat Completions providers can reuse this transport.

mod chat_completions;

use std::time::Duration;

use cellar_core::error::{CellarError, CellarResult};
use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum BackendAiProvider {
    Deepseek,
}

impl BackendAiProvider {
    fn config(self) -> ProviderConfig {
        match self {
            Self::Deepseek => ProviderConfig {
                id: "deepseek",
                label: "DeepSeek",
                base_url: "https://api.deepseek.com",
                supports_thinking: true,
            },
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum AiThinkingMode {
    Enabled,
    Disabled,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct BackendAiModel {
    pub id: String,
    pub label: String,
    pub description: Option<String>,
    pub is_default: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct BackendAiChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct BackendAiGenerateRequest {
    pub model: String,
    pub messages: Vec<BackendAiChatMessage>,
    pub system_instruction: Option<String>,
    pub thinking: Option<AiThinkingMode>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct BackendAiTokenUsage {
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub total_tokens: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct BackendAiGenerateResult {
    pub text: String,
    pub usage: Option<BackendAiTokenUsage>,
}

#[derive(Clone, Copy)]
struct ProviderConfig {
    id: &'static str,
    label: &'static str,
    base_url: &'static str,
    supports_thinking: bool,
}

pub struct BackendAiService {
    http: reqwest::Client,
}

impl Default for BackendAiService {
    fn default() -> Self {
        Self {
            http: reqwest::Client::builder()
                .user_agent(concat!("Cellar/", env!("CARGO_PKG_VERSION")))
                .timeout(Duration::from_secs(300))
                .build()
                .expect("valid backend AI HTTP client"),
        }
    }
}

impl BackendAiService {
    pub async fn list_models(
        &self,
        provider: BackendAiProvider,
    ) -> CellarResult<Vec<BackendAiModel>> {
        let config = provider.config();
        let key = load_api_key(config)?;
        chat_completions::list_models(&self.http, config, &key).await
    }

    pub async fn generate(
        &self,
        provider: BackendAiProvider,
        request: BackendAiGenerateRequest,
    ) -> CellarResult<BackendAiGenerateResult> {
        if request.model.trim().is_empty() {
            return Err(CellarError::invalid_config("no AI model selected"));
        }
        if request.messages.is_empty() {
            return Err(CellarError::invalid_config(
                "the AI request has no messages",
            ));
        }
        let config = provider.config();
        let key = load_api_key(config)?;
        chat_completions::generate(&self.http, config, &key, request).await
    }
}

fn load_api_key(config: ProviderConfig) -> CellarResult<String> {
    cellar_secrets::load(&format!("ai:{}", config.id))?.ok_or_else(|| {
        CellarError::Authentication(format!(
            "No {} API key is configured. Add one in AI settings.",
            config.label
        ))
    })
}
