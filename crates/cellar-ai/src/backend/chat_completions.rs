use cellar_core::error::{CellarError, CellarResult};
use serde_json::{json, Value};

use super::{
    AiThinkingMode, BackendAiGenerateRequest, BackendAiGenerateResult, BackendAiModel,
    BackendAiTokenUsage, ProviderConfig,
};

pub(super) async fn list_models(
    client: &reqwest::Client,
    config: ProviderConfig,
    api_key: &str,
) -> CellarResult<Vec<BackendAiModel>> {
    let response = client
        .get(format!("{}/models", config.base_url))
        .bearer_auth(api_key)
        .send()
        .await
        .map_err(|error| network_error(config, error))?;
    let body = response_json(response, config, "Failed to list models").await?;
    parse_models(&body)
}

pub(super) async fn generate(
    client: &reqwest::Client,
    config: ProviderConfig,
    api_key: &str,
    request: BackendAiGenerateRequest,
) -> CellarResult<BackendAiGenerateResult> {
    let payload = chat_payload(config, &request);
    let response = client
        .post(format!("{}/chat/completions", config.base_url))
        .bearer_auth(api_key)
        .json(&payload)
        .send()
        .await
        .map_err(|error| network_error(config, error))?;
    let body = response_json(response, config, "Generation failed").await?;
    parse_generation(config, &body)
}

async fn response_json(
    response: reqwest::Response,
    config: ProviderConfig,
    failure_fallback: &str,
) -> CellarResult<Value> {
    let status = response.status();
    let text = response
        .text()
        .await
        .map_err(|error| network_error(config, error))?;
    if !status.is_success() {
        let body = serde_json::from_str::<Value>(&text).ok();
        return Err(provider_error(
            config,
            status.as_u16(),
            body.as_ref(),
            failure_fallback,
        ));
    }
    serde_json::from_str(&text).map_err(|error| {
        CellarError::decode(format!(
            "Could not decode the {} response: {error}",
            config.label
        ))
    })
}

fn chat_payload(config: ProviderConfig, request: &BackendAiGenerateRequest) -> Value {
    let mut messages = Vec::with_capacity(request.messages.len() + 1);
    if let Some(instruction) = request
        .system_instruction
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        messages.push(json!({ "role": "system", "content": instruction }));
    }
    messages.extend(request.messages.iter().filter_map(|message| {
        let content = message.content.trim();
        if content.is_empty() {
            return None;
        }
        let role = match message.role.as_str() {
            "model" | "assistant" => "assistant",
            _ => "user",
        };
        Some(json!({ "role": role, "content": content }))
    }));

    let mut payload = json!({
        "model": request.model,
        "messages": messages,
        "stream": false
    });
    if config.supports_thinking {
        if let Some(mode) = request.thinking {
            payload["thinking"] = json!({
                "type": match mode {
                    AiThinkingMode::Enabled => "enabled",
                    AiThinkingMode::Disabled => "disabled",
                }
            });
        }
    }
    payload
}

fn parse_models(body: &Value) -> CellarResult<Vec<BackendAiModel>> {
    let data = body
        .get("data")
        .and_then(Value::as_array)
        .ok_or_else(|| CellarError::decode("Provider model response did not include data"))?;
    let mut models = data
        .iter()
        .filter_map(|item| item.get("id").and_then(Value::as_str))
        .map(|id| BackendAiModel {
            id: id.to_string(),
            label: id.to_string(),
            description: None,
            is_default: false,
        })
        .collect::<Vec<_>>();
    models.sort_by(|a, b| a.id.cmp(&b.id));
    models.dedup_by(|a, b| a.id == b.id);
    Ok(models)
}

fn parse_generation(config: ProviderConfig, body: &Value) -> CellarResult<BackendAiGenerateResult> {
    let text = body
        .pointer("/choices/0/message/content")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    if text.trim().is_empty() {
        return Err(CellarError::query(format!(
            "{} returned an empty response",
            config.label
        )));
    }
    let usage = body.get("usage").map(|usage| BackendAiTokenUsage {
        prompt_tokens: value_u64(usage, "prompt_tokens"),
        completion_tokens: value_u64(usage, "completion_tokens"),
        total_tokens: value_u64(usage, "total_tokens"),
    });
    Ok(BackendAiGenerateResult { text, usage })
}

fn value_u64(value: &Value, field: &str) -> u64 {
    value.get(field).and_then(Value::as_u64).unwrap_or(0)
}

fn network_error(config: ProviderConfig, error: reqwest::Error) -> CellarError {
    if error.is_timeout() {
        CellarError::Timeout(format!("The {} request timed out.", config.label))
    } else {
        CellarError::Connection(format!("{} request failed: {error}", config.label))
    }
}

fn provider_error(
    config: ProviderConfig,
    status: u16,
    body: Option<&Value>,
    fallback: &str,
) -> CellarError {
    let message = body
        .and_then(|body| body.pointer("/error/message"))
        .and_then(Value::as_str)
        .unwrap_or(fallback);
    match status {
        401 | 403 => CellarError::Authentication(message.into()),
        408 => CellarError::Timeout(message.into()),
        _ => CellarError::Query(format!("{}: {message} (HTTP {status})", config.label)),
    }
}

#[cfg(test)]
mod tests {
    use super::{chat_payload, parse_generation, parse_models, provider_error};
    use crate::backend::{
        AiThinkingMode, BackendAiChatMessage, BackendAiGenerateRequest, BackendAiProvider,
    };
    use cellar_core::error::CellarError;
    use serde_json::json;

    #[test]
    fn discovers_models_without_a_hardcoded_allowlist() {
        let models = parse_models(&json!({
            "data": [
                { "id": "deepseek-v4-pro" },
                { "id": "deepseek-v4-flash" },
                { "id": "future-model" }
            ]
        }))
        .unwrap();
        assert_eq!(
            models
                .iter()
                .map(|model| model.id.as_str())
                .collect::<Vec<_>>(),
            ["deepseek-v4-flash", "deepseek-v4-pro", "future-model"]
        );
    }

    #[test]
    fn builds_role_aware_payload_with_thinking_mode() {
        let request = BackendAiGenerateRequest {
            model: "deepseek-v4-pro".into(),
            messages: vec![
                BackendAiChatMessage {
                    role: "user".into(),
                    content: "question".into(),
                },
                BackendAiChatMessage {
                    role: "model".into(),
                    content: "answer".into(),
                },
            ],
            system_instruction: Some("be useful".into()),
            thinking: Some(AiThinkingMode::Disabled),
        };
        let payload = chat_payload(BackendAiProvider::Deepseek.config(), &request);
        assert_eq!(payload["thinking"]["type"], "disabled");
        assert_eq!(payload["messages"][0]["role"], "system");
        assert_eq!(payload["messages"][2]["role"], "assistant");
    }

    #[test]
    fn extracts_text_and_usage() {
        let result = parse_generation(
            BackendAiProvider::Deepseek.config(),
            &json!({
                "choices": [{ "message": { "content": "SELECT 1" } }],
                "usage": {
                    "prompt_tokens": 4,
                    "completion_tokens": 2,
                    "total_tokens": 6
                }
            }),
        )
        .unwrap();
        assert_eq!(result.text, "SELECT 1");
        assert_eq!(result.usage.unwrap().total_tokens, 6);
    }

    #[test]
    fn preserves_status_when_provider_error_is_not_json() {
        let config = BackendAiProvider::Deepseek.config();
        let authentication = provider_error(config, 401, None, "Generation failed");
        assert!(matches!(
            authentication,
            CellarError::Authentication(message) if message == "Generation failed"
        ));

        let provider = provider_error(config, 502, None, "Generation failed");
        assert!(matches!(
            provider,
            CellarError::Query(message)
                if message == "DeepSeek: Generation failed (HTTP 502)"
        ));
    }
}
