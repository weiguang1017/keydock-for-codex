use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::Duration;

use crate::DEFAULT_BASE_URL;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidationResult {
    pub valid: bool,
    pub status_code: u16,
    pub message: String,
    pub models: Vec<String>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub model: String,
}

impl ValidationResult {
    pub fn ok(message: impl Into<String>, models: Vec<String>) -> Self {
        Self {
            valid: true,
            status_code: 200,
            message: message.into(),
            models,
            model: String::new(),
        }
    }

    pub fn fail(status_code: u16, message: impl Into<String>, models: Vec<String>) -> Self {
        Self {
            valid: false,
            status_code,
            message: message.into(),
            models,
            model: String::new(),
        }
    }
}

pub fn trim(value: impl AsRef<str>) -> String {
    value.as_ref().trim().to_string()
}

pub fn mask_key(key: impl AsRef<str>) -> String {
    let value = trim(key);
    if value.is_empty() {
        String::new()
    } else if value.len() <= 8 {
        "***".to_string()
    } else if value.len() <= 14 {
        format!("{}...{}", &value[..3], &value[value.len() - 3..])
    } else {
        format!("{}...{}", &value[..7], &value[value.len() - 4..])
    }
}

pub fn normalize_base_url(value: impl AsRef<str>) -> Result<String, String> {
    let input = trim(value);
    if input.is_empty() {
        return Ok(String::new());
    }
    let with_protocol = if input.contains("://") {
        input
    } else {
        format!("https://{input}")
    };
    let mut url = reqwest::Url::parse(&with_protocol)
        .map_err(|error| error.to_string())?;
    url.set_fragment(None);
    Ok(url.to_string().trim_end_matches('/').to_string())
}

pub fn normalize_or_default(value: impl AsRef<str>) -> Result<String, String> {
    let normalized = normalize_base_url(value)?;
    if normalized.is_empty() {
        normalize_base_url(DEFAULT_BASE_URL)
    } else {
        Ok(normalized)
    }
}

pub fn model_endpoint(base_url: impl AsRef<str>) -> Result<reqwest::Url, String> {
    let normalized = normalize_base_url(base_url)?;
    if normalized.is_empty() {
        return Err("Base URL is required.".to_string());
    }
    let mut url = reqwest::Url::parse(&normalized)
        .map_err(|error| error.to_string())?;
    let current_path = url.path().trim_end_matches('/');
    if !current_path.ends_with("/models") && current_path != "/models" {
        let next_path = if current_path.is_empty() {
            "/models".to_string()
        } else {
            format!("{current_path}/models")
        };
        url.set_path(&next_path);
    }
    Ok(url)
}

pub fn v1_model_endpoint(base_url: impl AsRef<str>) -> Result<reqwest::Url, String> {
    let normalized = normalize_base_url(base_url)?;
    if normalized.is_empty() {
        return Err("Base URL is required.".to_string());
    }
    let mut url = reqwest::Url::parse(&normalized)
        .map_err(|error| error.to_string())?;
    let current_path = url.path().trim_end_matches('/');
    let base_path = current_path.trim_end_matches("/models").trim_end_matches('/');
    let next_path = if base_path.is_empty() {
        "/v1/models".to_string()
    } else if base_path.ends_with("/v1") {
        format!("{base_path}/models")
    } else {
        format!("{base_path}/v1/models")
    };
    url.set_path(&next_path);
    Ok(url)
}

pub fn unique_strings(values: impl IntoIterator<Item = String>) -> Vec<String> {
    let mut result = Vec::new();
    for value in values {
        let value = trim(value);
        if !value.is_empty() && !result.iter().any(|item| item == &value) {
            result.push(value);
        }
    }
    result
}

pub fn extract_models(payload: &Value) -> Vec<String> {
    let items = payload
        .get("data")
        .and_then(Value::as_array)
        .or_else(|| payload.get("models").and_then(Value::as_array));

    let mut models = Vec::new();
    if let Some(items) = items {
        for item in items {
            if let Some(value) = item.as_str() {
                models.push(value.to_string());
                continue;
            }
            if let Some(value) = item.get("id").and_then(Value::as_str) {
                models.push(value.to_string());
                continue;
            }
            if let Some(value) = item.get("name").and_then(Value::as_str) {
                models.push(value.to_string());
            }
        }
    }
    models = unique_strings(models);
    models.sort();
    models
}

pub fn validate_key(api_key: impl AsRef<str>, base_url: Option<&str>) -> ValidationResult {
    let key = trim(api_key);
    if key.is_empty() {
        return ValidationResult::fail(0, "API key is required.", Vec::new());
    }

    if std::env::var("CKM_SKIP_NETWORK_VALIDATION_FOR_TESTS").ok().as_deref() == Some("1") {
        return ValidationResult::ok(
            "Test validation passed.",
            vec!["gpt-4.1".to_string(), "gpt-4.1-mini".to_string()],
        );
    }

    let base = base_url.unwrap_or(DEFAULT_BASE_URL);
    let endpoint = match model_endpoint(base) {
        Ok(endpoint) => endpoint,
        Err(message) => return ValidationResult::fail(0, message, Vec::new()),
    };

    let client = match reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(20))
        .build()
    {
        Ok(client) => client,
        Err(error) => return ValidationResult::fail(0, error.to_string(), Vec::new()),
    };

    let primary = endpoint.to_string();
    let response = match client
        .get(endpoint)
        .bearer_auth(&key)
        .header(reqwest::header::ACCEPT, "application/json")
        .send()
    {
        Ok(response) => response,
        Err(error) => return ValidationResult::fail(0, error.to_string(), Vec::new()),
    };

    let status = response.status().as_u16();
    let body = response.text().unwrap_or_default();
    let mut models = serde_json::from_str::<Value>(&body)
        .map(|payload| extract_models(&payload))
        .unwrap_or_default();

    // Some OpenAI-compatible proxies expose models under `/v1/models` even when the
    // configured base URL omits the `/v1` segment. Fall back to it when the primary
    // endpoint accepts the key but returns no model list.
    if status == 200 && models.is_empty() {
        if let Ok(fallback) = v1_model_endpoint(base) {
            if fallback.to_string() != primary {
                if let Ok(extra) = client
                    .get(fallback)
                    .bearer_auth(&key)
                    .header(reqwest::header::ACCEPT, "application/json")
                    .send()
                {
                    if extra.status().is_success() {
                        let extra_body = extra.text().unwrap_or_default();
                        let extra_models = serde_json::from_str::<Value>(&extra_body)
                            .map(|payload| extract_models(&payload))
                            .unwrap_or_default();
                        if !extra_models.is_empty() {
                            models = extra_models;
                        }
                    }
                }
            }
        }
    }

    match status {
        200 => ValidationResult::ok("The platform accepted this key.", models),
        401 => ValidationResult::fail(401, "The platform rejected this key.", models),
        403 => ValidationResult::fail(
            403,
            "This key is not permitted to access the model endpoint.",
            models,
        ),
        other => ValidationResult::fail(
            other,
            format!("Validation failed with HTTP {other}."),
            models,
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_and_masks() {
        assert_eq!(mask_key("sk-1234567890abcdef"), "sk-1234...cdef");
        assert_eq!(
            normalize_base_url("api.example.com/v1/").unwrap(),
            "https://api.example.com/v1"
        );
        assert_eq!(
            model_endpoint("https://api.example.com/v1").unwrap().to_string(),
            "https://api.example.com/v1/models"
        );
    }

    #[test]
    fn builds_v1_fallback_endpoint() {
        assert_eq!(
            v1_model_endpoint("https://new.sharedchat.cc/codex").unwrap().to_string(),
            "https://new.sharedchat.cc/codex/v1/models"
        );
        assert_eq!(
            v1_model_endpoint("https://api.openai.com/v1").unwrap().to_string(),
            "https://api.openai.com/v1/models"
        );
        assert_eq!(
            v1_model_endpoint("https://proxy.example.com").unwrap().to_string(),
            "https://proxy.example.com/v1/models"
        );
    }

    #[test]
    fn extracts_models() {
        let payload = serde_json::json!({ "data": [{ "id": "gpt-z" }, { "id": "gpt-a" }] });
        assert_eq!(extract_models(&payload), vec!["gpt-a", "gpt-z"]);
    }
}
