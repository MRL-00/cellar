//! OpenAI provider services.
//!
//! API-key requests use the Responses API directly from Rust so the key never
//! crosses into the webview. ChatGPT subscription access is delegated to an
//! isolated Codex app-server process, which owns the OAuth tokens and refresh
//! flow described by OpenAI's supported client integration protocol.

mod api;
mod app_server;

use cellar_core::error::{CellarError, CellarResult};
use serde::{Deserialize, Serialize};
use specta::Type;
use tokio::sync::Mutex;

use app_server::CodexAppServer;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum OpenAiAuthMode {
    ApiKey,
    Chatgpt,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum OpenAiLoginMethod {
    Browser,
    DeviceCode,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct OpenAiModel {
    pub id: String,
    pub label: String,
    pub description: Option<String>,
    pub is_default: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct OpenAiOAuthStatus {
    pub signed_in: bool,
    pub email: Option<String>,
    pub plan_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct OpenAiLoginStart {
    pub login_id: String,
    pub auth_url: String,
    pub user_code: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct OpenAiChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct OpenAiGenerateRequest {
    pub model: String,
    pub messages: Vec<OpenAiChatMessage>,
    pub system_instruction: Option<String>,
    pub thread_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct OpenAiTokenUsage {
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub total_tokens: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct OpenAiGenerateResult {
    pub text: String,
    pub usage: Option<OpenAiTokenUsage>,
    pub thread_id: Option<String>,
}

pub struct OpenAiService {
    http: reqwest::Client,
    codex: Mutex<Option<CodexAppServer>>,
}

impl Default for OpenAiService {
    fn default() -> Self {
        Self {
            http: reqwest::Client::builder()
                .user_agent(concat!("Cellar/", env!("CARGO_PKG_VERSION")))
                .build()
                .expect("valid OpenAI HTTP client"),
            codex: Mutex::new(None),
        }
    }
}

impl OpenAiService {
    pub async fn list_models(&self, mode: OpenAiAuthMode) -> CellarResult<Vec<OpenAiModel>> {
        match mode {
            OpenAiAuthMode::ApiKey => {
                let key = load_api_key()?;
                api::list_models(&self.http, &key).await
            }
            OpenAiAuthMode::Chatgpt => self.codex_server().await?.list_models().await,
        }
    }

    pub async fn generate(
        &self,
        mode: OpenAiAuthMode,
        request: OpenAiGenerateRequest,
    ) -> CellarResult<OpenAiGenerateResult> {
        if request.model.trim().is_empty() {
            return Err(CellarError::invalid_config("no OpenAI model selected"));
        }
        if request.messages.is_empty() {
            return Err(CellarError::invalid_config(
                "the OpenAI request has no messages",
            ));
        }

        match mode {
            OpenAiAuthMode::ApiKey => {
                let key = load_api_key()?;
                api::generate(&self.http, &key, request).await
            }
            OpenAiAuthMode::Chatgpt => self.codex_server().await?.generate(request).await,
        }
    }

    pub async fn oauth_status(&self) -> CellarResult<OpenAiOAuthStatus> {
        self.codex_server().await?.account_status().await
    }

    pub async fn start_login(&self, method: OpenAiLoginMethod) -> CellarResult<OpenAiLoginStart> {
        self.codex_server().await?.start_login(method).await
    }

    pub async fn cancel_login(&self, login_id: &str) -> CellarResult<()> {
        self.codex_server().await?.cancel_login(login_id).await
    }

    pub async fn logout(&self) -> CellarResult<()> {
        self.codex_server().await?.logout().await
    }

    async fn codex_server(
        &self,
    ) -> CellarResult<tokio::sync::MappedMutexGuard<'_, CodexAppServer>> {
        let mut guard = self.codex.lock().await;
        let restart = match guard.as_mut() {
            Some(server) => !server.is_running()?,
            None => true,
        };
        if restart {
            *guard = Some(CodexAppServer::spawn().await?);
        }
        Ok(tokio::sync::MutexGuard::map(guard, |server| {
            server.as_mut().expect("Codex app-server initialized")
        }))
    }
}

fn load_api_key() -> CellarResult<String> {
    cellar_secrets::load("ai:openai")?.ok_or_else(|| {
        CellarError::Authentication(
            "No OpenAI API key is configured. Add one in AI settings.".into(),
        )
    })
}
