use cellar_core::error::{CellarError, CellarResult};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

const BASE_URL: &str = "https://generativelanguage.googleapis.com/v1beta";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GoogleModel {
    pub id: String,
    pub label: String,
    pub context: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GoogleChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GoogleGenerateRequest {
    pub model: String,
    pub messages: Vec<GoogleChatMessage>,
    pub system_instruction: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GoogleGenerateResult {
    pub text: String,
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub total_tokens: u64,
}

#[derive(Clone)]
pub struct GoogleService {
    http: reqwest::Client,
}

impl Default for GoogleService {
    fn default() -> Self {
        Self {
            http: reqwest::Client::builder()
                .user_agent(concat!("Cellar/", env!("CARGO_PKG_VERSION")))
                .build()
                .expect("valid Google AI HTTP client"),
        }
    }
}

impl GoogleService {
    pub async fn list_models(&self) -> CellarResult<Vec<GoogleModel>> {
        let key = load_key()?;
        let response = self
            .http
            .get(format!("{BASE_URL}/models?pageSize=1000&key={key}"))
            .send()
            .await
            .map_err(network_error)?;
        let status = response.status();
        let body: Value = response.json().await.map_err(network_error)?;
        if !status.is_success() {
            return Err(provider_error(
                status.as_u16(),
                &body,
                "Failed to list models",
            ));
        }
        let mut models = body
            .get("models")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter(|model| {
                model
                    .get("supportedGenerationMethods")
                    .and_then(Value::as_array)
                    .is_some_and(|methods| methods.iter().any(|method| method == "generateContent"))
            })
            .filter_map(|model| {
                let id = model.get("name")?.as_str()?.trim_start_matches("models/");
                id.starts_with("gemini").then(|| GoogleModel {
                    id: id.into(),
                    label: model
                        .get("displayName")
                        .and_then(Value::as_str)
                        .unwrap_or(id)
                        .into(),
                    context: model
                        .get("inputTokenLimit")
                        .and_then(Value::as_u64)
                        .map(humanize_tokens),
                })
            })
            .collect::<Vec<_>>();
        models.sort_by(|a, b| {
            model_version(&b.id)
                .total_cmp(&model_version(&a.id))
                .then(a.id.cmp(&b.id))
        });
        Ok(models)
    }

    pub async fn generate(
        &self,
        request: GoogleGenerateRequest,
    ) -> CellarResult<GoogleGenerateResult> {
        if request.model.trim().is_empty() || request.messages.is_empty() {
            return Err(CellarError::invalid_config(
                "Google generation requires a model and messages",
            ));
        }
        let key = load_key()?;
        let mut payload = json!({
            "contents": request.messages.into_iter().map(|message| json!({
                "role": message.role,
                "parts": [{ "text": message.content }]
            })).collect::<Vec<_>>()
        });
        if let Some(instruction) = request.system_instruction.filter(|text| !text.is_empty()) {
            payload["systemInstruction"] = json!({ "parts": [{ "text": instruction }] });
        }
        let response = self
            .http
            .post(format!(
                "{BASE_URL}/models/{}:generateContent?key={key}",
                request.model
            ))
            .json(&payload)
            .send()
            .await
            .map_err(network_error)?;
        let status = response.status();
        let body: Value = response.json().await.map_err(network_error)?;
        if !status.is_success() {
            return Err(provider_error(status.as_u16(), &body, "Generation failed"));
        }
        if let Some(reason) = body
            .pointer("/promptFeedback/blockReason")
            .and_then(Value::as_str)
        {
            return Err(CellarError::query(format!(
                "Request blocked by the provider ({reason})."
            )));
        }
        let text = body
            .pointer("/candidates/0/content/parts")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|part| part.get("text").and_then(Value::as_str))
            .collect::<String>();
        if text.trim().is_empty() {
            return Err(CellarError::query("Google returned an empty response"));
        }
        Ok(GoogleGenerateResult {
            text,
            prompt_tokens: body
                .pointer("/usageMetadata/promptTokenCount")
                .and_then(Value::as_u64)
                .unwrap_or(0),
            completion_tokens: body
                .pointer("/usageMetadata/candidatesTokenCount")
                .and_then(Value::as_u64)
                .unwrap_or(0),
            total_tokens: body
                .pointer("/usageMetadata/totalTokenCount")
                .and_then(Value::as_u64)
                .unwrap_or(0),
        })
    }
}

fn load_key() -> CellarResult<String> {
    cellar_secrets::load("ai:google")?.ok_or_else(|| {
        CellarError::Authentication(
            "No Google API key is configured. Add one in AI settings.".into(),
        )
    })
}

fn network_error(error: reqwest::Error) -> CellarError {
    if error.is_timeout() {
        CellarError::Timeout("The Google request timed out.".into())
    } else {
        CellarError::Connection(format!("Google request failed: {error}"))
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

fn humanize_tokens(tokens: u64) -> String {
    if tokens >= 1_000_000 {
        format!("{}m", (tokens + 500_000) / 1_000_000)
    } else if tokens >= 1_000 {
        format!("{}k", (tokens + 500) / 1_000)
    } else {
        tokens.to_string()
    }
}

fn model_version(id: &str) -> f32 {
    id.split('-')
        .find_map(|part| part.parse::<f32>().ok())
        .unwrap_or(-1.)
}

#[cfg(test)]
mod tests {
    use super::{humanize_tokens, model_version};

    #[test]
    fn google_model_metadata_matches_classic_ordering_and_labels() {
        assert_eq!(humanize_tokens(1_048_576), "1m");
        assert_eq!(humanize_tokens(200_000), "200k");
        assert_eq!(model_version("gemini-2.5-pro"), 2.5);
    }
}
