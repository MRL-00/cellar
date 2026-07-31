use cellar_core::error::{CellarError, CellarResult};
use serde_json::{json, Value};

use super::{OpenAiGenerateRequest, OpenAiGenerateResult, OpenAiModel, OpenAiTokenUsage};

const BASE_URL: &str = "https://api.openai.com/v1";

pub async fn list_models(
    client: &reqwest::Client,
    api_key: &str,
) -> CellarResult<Vec<OpenAiModel>> {
    let response = client
        .get(format!("{BASE_URL}/models"))
        .bearer_auth(api_key)
        .send()
        .await
        .map_err(network_error)?;
    let status = response.status();
    let body: Value = response.json().await.map_err(network_error)?;
    if !status.is_success() {
        return Err(provider_error(
            status.as_u16(),
            &body,
            "Failed to list OpenAI models",
        ));
    }

    let mut models = body
        .get("data")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|item| item.get("id").and_then(Value::as_str))
        .filter(|id| is_chat_model(id))
        .map(|id| OpenAiModel {
            id: id.to_string(),
            label: id.to_string(),
            description: None,
            is_default: false,
        })
        .collect::<Vec<_>>();
    models.sort_by(|a, b| b.id.cmp(&a.id));
    models.dedup_by(|a, b| a.id == b.id);
    if let Some(first) = models.first_mut() {
        first.is_default = true;
    }
    Ok(models)
}

pub async fn generate(
    client: &reqwest::Client,
    api_key: &str,
    request: OpenAiGenerateRequest,
) -> CellarResult<OpenAiGenerateResult> {
    let input = request
        .messages
        .iter()
        .map(|message| {
            let role = match message.role.as_str() {
                "model" | "assistant" => "assistant",
                _ => "user",
            };
            json!({
                "role": role,
                "content": message.content
            })
        })
        .collect::<Vec<_>>();
    let mut payload = json!({
        "model": request.model,
        "input": input,
        "store": false,
        "tools": []
    });
    if let Some(instructions) = request.system_instruction.filter(|value| !value.is_empty()) {
        payload["instructions"] = Value::String(instructions);
    }

    let response = client
        .post(format!("{BASE_URL}/responses"))
        .bearer_auth(api_key)
        .json(&payload)
        .send()
        .await
        .map_err(network_error)?;
    let status = response.status();
    let body: Value = response.json().await.map_err(network_error)?;
    if !status.is_success() {
        return Err(provider_error(
            status.as_u16(),
            &body,
            "OpenAI generation failed",
        ));
    }

    let text = response_text(&body);
    if text.trim().is_empty() {
        return Err(CellarError::query("OpenAI returned an empty response"));
    }
    let usage = body.get("usage").map(|usage| OpenAiTokenUsage {
        prompt_tokens: usage
            .get("input_tokens")
            .and_then(Value::as_u64)
            .unwrap_or(0),
        completion_tokens: usage
            .get("output_tokens")
            .and_then(Value::as_u64)
            .unwrap_or(0),
        total_tokens: usage
            .get("total_tokens")
            .and_then(Value::as_u64)
            .unwrap_or(0),
    });

    Ok(OpenAiGenerateResult {
        text,
        usage,
        thread_id: None,
    })
}

fn response_text(body: &Value) -> String {
    body.get("output")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|item| item.get("type").and_then(Value::as_str) == Some("message"))
        .flat_map(|item| {
            item.get("content")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
        })
        .filter(|part| part.get("type").and_then(Value::as_str) == Some("output_text"))
        .filter_map(|part| part.get("text").and_then(Value::as_str))
        .collect::<Vec<_>>()
        .join("")
}

fn is_chat_model(id: &str) -> bool {
    id.starts_with("gpt-")
        && !["audio", "realtime", "transcribe", "tts", "image", "search"]
            .iter()
            .any(|suffix| id.contains(suffix))
}

fn network_error(error: reqwest::Error) -> CellarError {
    if error.is_timeout() {
        CellarError::Timeout("The OpenAI request timed out.".into())
    } else {
        CellarError::Connection(format!("OpenAI request failed: {error}"))
    }
}

fn provider_error(status: u16, body: &Value, fallback: &str) -> CellarError {
    let message = body
        .pointer("/error/message")
        .and_then(Value::as_str)
        .unwrap_or(fallback);
    match status {
        401 | 403 => CellarError::Authentication(message.into()),
        408 => CellarError::Timeout(message.into()),
        _ => CellarError::Query(format!("{message} (HTTP {status})")),
    }
}

#[cfg(test)]
mod tests {
    use super::{is_chat_model, response_text};
    use serde_json::json;

    #[test]
    fn filters_non_text_openai_models() {
        assert!(is_chat_model("gpt-5.6"));
        assert!(is_chat_model("gpt-5.6-mini"));
        assert!(!is_chat_model("gpt-4o-realtime-preview"));
        assert!(!is_chat_model("text-embedding-3-large"));
    }

    #[test]
    fn extracts_all_output_text_parts() {
        let body = json!({
            "output": [{
                "type": "message",
                "content": [
                    {"type": "output_text", "text": "hello "},
                    {"type": "output_text", "text": "world"}
                ]
            }]
        });
        assert_eq!(response_text(&body), "hello world");
    }
}
