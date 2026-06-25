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
    #[serde(default)]
    pub supported_clients: Vec<String>,
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
            supported_clients: Vec::new(),
            model: String::new(),
        }
    }

    pub fn fail(status_code: u16, message: impl Into<String>, models: Vec<String>) -> Self {
        Self {
            valid: false,
            status_code,
            message: message.into(),
            models,
            supported_clients: Vec::new(),
            model: String::new(),
        }
    }
}

const CLIENT_CODEX: &str = "codex";
const CLIENT_OPENCLAW: &str = "openclaw";
const CLIENT_HERMES: &str = "hermes";

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
    let mut url = reqwest::Url::parse(&with_protocol).map_err(|error| error.to_string())?;
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
    let mut url = reqwest::Url::parse(&normalized).map_err(|error| error.to_string())?;
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
    v1_endpoint(base_url, "models")
}

pub fn responses_endpoint(base_url: impl AsRef<str>) -> Result<reqwest::Url, String> {
    let normalized = normalize_base_url(base_url)?;
    if normalized.is_empty() {
        return Err("Base URL is required.".to_string());
    }
    let mut url = reqwest::Url::parse(&normalized).map_err(|error| error.to_string())?;
    let current_path = url.path().trim_end_matches('/');
    if !current_path.ends_with("/responses") {
        let next_path = if current_path.is_empty() {
            "/responses".to_string()
        } else {
            format!("{current_path}/responses")
        };
        url.set_path(&next_path);
    }
    Ok(url)
}

pub fn v1_responses_endpoint(base_url: impl AsRef<str>) -> Result<reqwest::Url, String> {
    v1_endpoint(base_url, "responses")
}

pub fn chat_completions_endpoint(base_url: impl AsRef<str>) -> Result<reqwest::Url, String> {
    let normalized = normalize_base_url(base_url)?;
    if normalized.is_empty() {
        return Err("Base URL is required.".to_string());
    }
    let mut url = reqwest::Url::parse(&normalized).map_err(|error| error.to_string())?;
    let current_path = url.path().trim_end_matches('/');
    if !current_path.ends_with("/chat/completions") {
        let next_path = if current_path.is_empty() {
            "/chat/completions".to_string()
        } else {
            format!("{current_path}/chat/completions")
        };
        url.set_path(&next_path);
    }
    Ok(url)
}

pub fn v1_chat_completions_endpoint(base_url: impl AsRef<str>) -> Result<reqwest::Url, String> {
    v1_endpoint(base_url, "chat/completions")
}

fn v1_endpoint(base_url: impl AsRef<str>, suffix: &str) -> Result<reqwest::Url, String> {
    let normalized = normalize_base_url(base_url)?;
    if normalized.is_empty() {
        return Err("Base URL is required.".to_string());
    }
    let mut url = reqwest::Url::parse(&normalized).map_err(|error| error.to_string())?;
    let suffix = suffix.trim_matches('/');
    let current_path = url.path().trim_end_matches('/');
    let without_suffix = current_path
        .strip_suffix(&format!("/{suffix}"))
        .or_else(|| current_path.strip_suffix(suffix))
        .unwrap_or(current_path)
        .trim_end_matches('/');
    let next_path = if without_suffix.is_empty() {
        format!("/v1/{suffix}")
    } else if without_suffix.ends_with("/v1") {
        format!("{without_suffix}/{suffix}")
    } else {
        format!("{without_suffix}/v1/{suffix}")
    };
    url.set_path(&next_path);
    Ok(url)
}

fn unique_urls(
    values: impl IntoIterator<Item = Result<reqwest::Url, String>>,
) -> Result<Vec<reqwest::Url>, String> {
    let mut urls = Vec::new();
    for value in values {
        let url = value?;
        if !urls
            .iter()
            .any(|item: &reqwest::Url| item.as_str() == url.as_str())
        {
            urls.push(url);
        }
    }
    Ok(urls)
}

pub fn extract_error_message(body: &str) -> String {
    let trimmed = trim(body);
    if trimmed.is_empty() {
        return "No response body was returned.".to_string();
    }
    if let Ok(value) = serde_json::from_str::<Value>(&trimmed) {
        if let Some(message) = value
            .get("error")
            .and_then(|error| error.get("message"))
            .and_then(Value::as_str)
        {
            return trim(message);
        }
        if let Some(message) = value.get("message").and_then(Value::as_str) {
            return trim(message);
        }
        if let Some(error) = value.get("error").and_then(Value::as_str) {
            return trim(error);
        }
    }
    truncate_chars(&trimmed, 800)
}

fn truncate_chars(value: &str, max: usize) -> String {
    if value.chars().count() <= max {
        value.to_string()
    } else {
        let mut result: String = value.chars().take(max).collect();
        result.push('…');
        result
    }
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

pub fn normalize_supported_clients(values: impl IntoIterator<Item = String>) -> Vec<String> {
    let mut result = Vec::new();
    for value in values {
        let id = trim(value).to_lowercase();
        let known = matches!(id.as_str(), CLIENT_CODEX | CLIENT_OPENCLAW | CLIENT_HERMES);
        if known && !result.iter().any(|item| item == &id) {
            result.push(id);
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

    if std::env::var("CKM_SKIP_NETWORK_VALIDATION_FOR_TESTS")
        .ok()
        .as_deref()
        == Some("1")
    {
        return ValidationResult::ok(
            "Test validation passed.",
            vec!["gpt-4.1".to_string(), "gpt-4.1-mini".to_string()],
        );
    }

    let base = base_url.unwrap_or(DEFAULT_BASE_URL);
    let endpoints = match unique_urls([model_endpoint(base), v1_model_endpoint(base)]) {
        Ok(endpoints) => endpoints,
        Err(message) => return ValidationResult::fail(0, message, Vec::new()),
    };

    let client = match reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(20))
        .build()
    {
        Ok(client) => client,
        Err(error) => return ValidationResult::fail(0, error.to_string(), Vec::new()),
    };

    let mut last_failure = ValidationResult::fail(0, "No model endpoint was tried.", Vec::new());
    let mut empty_success = None;
    let endpoint_count = endpoints.len();
    for (index, endpoint) in endpoints.into_iter().enumerate() {
        let response = match client
            .get(endpoint)
            .bearer_auth(&key)
            .header(reqwest::header::ACCEPT, "application/json")
            .send()
        {
            Ok(response) => response,
            Err(error) => {
                last_failure = ValidationResult::fail(0, error.to_string(), Vec::new());
                continue;
            }
        };

        let status = response.status().as_u16();
        let body = response.text().unwrap_or_default();
        let models = serde_json::from_str::<Value>(&body)
            .map(|payload| extract_models(&payload))
            .unwrap_or_default();

        last_failure = match status {
            // CDN-fronted proxies can answer HTTP 200 with an HTML page or an embedded
            // JSON error for any path, so a 200 alone is not proof of a working endpoint.
            200 => match serde_json::from_str::<Value>(&body) {
                Err(_) => ValidationResult::fail(
                    200,
                    "The Base URL did not return an API response (likely an HTML page). Check the Base URL.",
                    Vec::new(),
                ),
                Ok(value) if value.get("error").map(|e| !e.is_null()).unwrap_or(false) => {
                    ValidationResult::fail(200, extract_error_message(&body), models)
                }
                Ok(_) if models.is_empty() && index + 1 < endpoint_count => {
                    let result = ValidationResult::ok("The platform accepted this key.", models);
                    empty_success = Some(result.clone());
                    result
                }
                Ok(_) => return ValidationResult::ok("The platform accepted this key.", models),
            },
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
        };

        if status == 401 || status == 403 {
            return last_failure;
        }
    }

    empty_success.unwrap_or(last_failure)
}

fn post_probe(
    client: &reqwest::blocking::Client,
    endpoints: Vec<reqwest::Url>,
    key: &str,
    payloads: &[Value],
    assess_body: fn(&str) -> Result<(), String>,
    success_message: &str,
    model: &str,
) -> ValidationResult {
    let mut last_failure = ValidationResult::fail(0, "No endpoint was tried.", Vec::new());
    for endpoint in endpoints {
        for payload in payloads {
            let response = match client
                .post(endpoint.clone())
                .bearer_auth(key)
                .header(reqwest::header::CONTENT_TYPE, "application/json")
                .header(reqwest::header::ACCEPT, "application/json")
                .json(payload)
                .send()
            {
                Ok(response) => response,
                Err(error) => {
                    last_failure =
                        ValidationResult::fail(0, format!("Request failed: {error}"), Vec::new());
                    continue;
                }
            };

            let status = response.status().as_u16();
            let body = response.text().unwrap_or_default();
            if (200..300).contains(&status) {
                match assess_body(&body) {
                    Ok(()) => {
                        let mut result = ValidationResult::ok(
                            format!("{success_message} (HTTP {status})."),
                            Vec::new(),
                        );
                        result.status_code = status;
                        result.model = model.to_string();
                        return result;
                    }
                    Err(message) => {
                        last_failure = ValidationResult::fail(
                            status,
                            format!("HTTP {status}: {message}"),
                            Vec::new(),
                        )
                    }
                }
            } else {
                let detail = extract_error_message(&body);
                last_failure =
                    ValidationResult::fail(status, format!("HTTP {status}: {detail}"), Vec::new());
            }

            if status == 401 || status == 403 {
                return last_failure;
            }
        }
    }

    last_failure
}

pub fn validate_clients_key(
    api_key: impl AsRef<str>,
    base_url: Option<&str>,
    model: impl AsRef<str>,
) -> ValidationResult {
    let key = trim(api_key);
    if key.is_empty() {
        return ValidationResult::fail(0, "API key is required.", Vec::new());
    }

    let requested_model = trim(model);
    if std::env::var("CKM_SKIP_NETWORK_VALIDATION_FOR_TESTS")
        .ok()
        .as_deref()
        == Some("1")
    {
        let selected_model = if requested_model.is_empty() {
            "gpt-4.1".to_string()
        } else {
            requested_model
        };
        let mut result = ValidationResult::ok(
            "Test validation passed.",
            vec!["gpt-4.1".to_string(), "gpt-4.1-mini".to_string()],
        );
        result.model = selected_model;
        result.supported_clients = normalize_supported_clients([
            CLIENT_CODEX.to_string(),
            CLIENT_OPENCLAW.to_string(),
            CLIENT_HERMES.to_string(),
        ]);
        return result;
    }

    let listing = validate_key(&key, base_url);
    let selected_model = if !requested_model.is_empty() {
        requested_model
    } else {
        listing.models.first().cloned().unwrap_or_default()
    };

    if selected_model.is_empty() {
        let mut result = if listing.valid {
            ValidationResult::fail(
                listing.status_code,
                "A model is required to probe client support.",
                listing.models,
            )
        } else {
            listing
        };
        result.supported_clients = Vec::new();
        return result;
    }

    let mut models = listing.models.clone();
    if !models.iter().any(|item| item == &selected_model) {
        models.push(selected_model.clone());
    }
    models = unique_strings(models);
    models.sort();

    let codex = probe_responses_key(&key, base_url, &selected_model);
    let chat = probe_chat_completions_key(&key, base_url, &selected_model);

    let mut supported = Vec::new();
    if codex.valid {
        supported.push(CLIENT_CODEX.to_string());
    }
    if chat.valid {
        supported.push(CLIENT_OPENCLAW.to_string());
        supported.push(CLIENT_HERMES.to_string());
    }
    supported = normalize_supported_clients(supported);

    if !supported.is_empty() {
        let primary = if codex.valid { &codex } else { &chat };
        let mut result = ValidationResult::ok(supported_clients_message(&supported), models);
        result.status_code = primary.status_code;
        result.model = selected_model;
        result.supported_clients = supported;
        result
    } else {
        let mut result = ValidationResult::fail(
            if codex.status_code != 0 {
                codex.status_code
            } else {
                chat.status_code
            },
            format!(
                "No supported client detected. Codex: {}; OpenClaw/Hermes: {}",
                trim(&codex.message),
                trim(&chat.message)
            ),
            models,
        );
        result.model = selected_model;
        result
    }
}

fn supported_clients_message(clients: &[String]) -> String {
    let labels = clients
        .iter()
        .filter_map(|id| match id.as_str() {
            CLIENT_CODEX => Some("Codex"),
            CLIENT_OPENCLAW => Some("OpenClaw"),
            CLIENT_HERMES => Some("Hermes"),
            _ => None,
        })
        .collect::<Vec<_>>();
    if labels.is_empty() {
        "No supported client detected.".to_string()
    } else {
        format!("Supported clients: {}.", labels.join(", "))
    }
}

/// Probe a key the way Codex actually uses it: a real POST to the `/responses`
/// endpoint. This reflects true availability (e.g. a 503 "no available channel")
/// far better than listing `/models`, which can succeed even when the model is
/// unusable.
pub fn probe_responses_key(
    api_key: impl AsRef<str>,
    base_url: Option<&str>,
    model: impl AsRef<str>,
) -> ValidationResult {
    let key = trim(api_key);
    if key.is_empty() {
        return ValidationResult::fail(0, "API key is required.", Vec::new());
    }

    if std::env::var("CKM_SKIP_NETWORK_VALIDATION_FOR_TESTS")
        .ok()
        .as_deref()
        == Some("1")
    {
        return ValidationResult::ok("Test validation passed.", Vec::new());
    }

    let model = trim(model);
    let base = base_url.unwrap_or(DEFAULT_BASE_URL);
    if model.is_empty() {
        // Without a model we cannot send a real request; fall back to listing models.
        return validate_key(&key, Some(base));
    }

    let endpoints = match unique_urls([responses_endpoint(base), v1_responses_endpoint(base)]) {
        Ok(endpoints) => endpoints,
        Err(message) => return ValidationResult::fail(0, message, Vec::new()),
    };

    let client = match reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(45))
        .build()
    {
        Ok(client) => client,
        Err(error) => return ValidationResult::fail(0, error.to_string(), Vec::new()),
    };

    let payloads = vec![
        serde_json::json!({
            "model": model,
            "input": "Reply with exactly: pong",
            "max_output_tokens": 16,
            "stream": false,
        }),
        serde_json::json!({
            "model": model,
            "input": "Reply with exactly: pong",
            "stream": false,
        }),
        serde_json::json!({
            "model": model,
            "input": "Reply with exactly: pong",
        }),
    ];

    post_probe(
        &client,
        endpoints,
        &key,
        &payloads,
        assess_responses_body,
        "Responses API responded successfully",
        &model,
    )
}

pub fn probe_chat_completions_key(
    api_key: impl AsRef<str>,
    base_url: Option<&str>,
    model: impl AsRef<str>,
) -> ValidationResult {
    let key = trim(api_key);
    if key.is_empty() {
        return ValidationResult::fail(0, "API key is required.", Vec::new());
    }

    if std::env::var("CKM_SKIP_NETWORK_VALIDATION_FOR_TESTS")
        .ok()
        .as_deref()
        == Some("1")
    {
        return ValidationResult::ok("Test validation passed.", Vec::new());
    }

    let model = trim(model);
    let base = base_url.unwrap_or(DEFAULT_BASE_URL);
    if model.is_empty() {
        return ValidationResult::fail(
            0,
            "A model is required to probe chat completions.",
            Vec::new(),
        );
    }

    let endpoints = match unique_urls([
        chat_completions_endpoint(base),
        v1_chat_completions_endpoint(base),
    ]) {
        Ok(endpoints) => endpoints,
        Err(message) => return ValidationResult::fail(0, message, Vec::new()),
    };

    let client = match reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(45))
        .build()
    {
        Ok(client) => client,
        Err(error) => return ValidationResult::fail(0, error.to_string(), Vec::new()),
    };

    let tool_probe = serde_json::json!([
        {
            "type": "function",
            "function": {
                "name": "keydock_ping",
                "description": "No-op connectivity probe.",
                "parameters": {
                    "type": "object",
                    "properties": {},
                    "additionalProperties": false
                }
            }
        }
    ]);
    let payloads = vec![
        serde_json::json!({
            "model": model,
            "messages": [
                { "role": "user", "content": "Reply with exactly: pong" }
            ],
            "max_completion_tokens": 16,
            "stream": false,
            "tools": tool_probe.clone(),
        }),
        serde_json::json!({
            "model": model,
            "messages": [
                { "role": "user", "content": "Reply with exactly: pong" }
            ],
            "max_tokens": 16,
            "stream": false,
            "tools": tool_probe.clone(),
        }),
        serde_json::json!({
            "model": model,
            "messages": [
                { "role": "user", "content": "Reply with exactly: pong" }
            ],
            "stream": false,
            "tools": tool_probe,
        }),
    ];

    post_probe(
        &client,
        endpoints,
        &key,
        &payloads,
        assess_chat_completions_body,
        "Chat completions responded successfully",
        &model,
    )
}

/// Check that a 2xx body from `/responses` is actually a successful model
/// response. CDN-fronted proxies often answer HTTP 200 with an HTML page (for any
/// path) or with a JSON body that embeds an error such as "insufficient balance".
/// Neither must count as a working key.
pub fn assess_responses_body(body: &str) -> Result<(), String> {
    let trimmed = trim(body);
    let value: Value = serde_json::from_str(&trimmed).map_err(|_| {
        "The endpoint returned a non-API response (likely an HTML page). Check the Base URL."
            .to_string()
    })?;
    if value.get("error").map(|e| !e.is_null()).unwrap_or(false) {
        return Err(extract_error_message(&trimmed));
    }
    let looks_like_response = value.get("object").and_then(Value::as_str) == Some("response")
        || value.get("output").is_some()
        || value.get("output_text").is_some()
        || value
            .get("id")
            .and_then(Value::as_str)
            .map(|id| id.starts_with("resp"))
            .unwrap_or(false);
    if looks_like_response {
        Ok(())
    } else {
        Err(
            "The endpoint accepted the request but did not return a model response. Check the Base URL."
                .to_string(),
        )
    }
}

pub fn assess_chat_completions_body(body: &str) -> Result<(), String> {
    let trimmed = trim(body);
    let value: Value = serde_json::from_str(&trimmed).map_err(|_| {
        "The endpoint returned a non-API response (likely an HTML page). Check the Base URL."
            .to_string()
    })?;
    if value.get("error").map(|e| !e.is_null()).unwrap_or(false) {
        return Err(extract_error_message(&trimmed));
    }
    let looks_like_chat = value.get("object").and_then(Value::as_str) == Some("chat.completion")
        || value.get("choices").and_then(Value::as_array).is_some()
        || value
            .get("id")
            .and_then(Value::as_str)
            .map(|id| id.starts_with("chatcmpl"))
            .unwrap_or(false);
    if looks_like_chat {
        Ok(())
    } else {
        Err(
            "The endpoint accepted the request but did not return a chat completion. Check the Base URL."
                .to_string(),
        )
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
            model_endpoint("https://api.example.com/v1")
                .unwrap()
                .to_string(),
            "https://api.example.com/v1/models"
        );
    }

    #[test]
    fn judges_responses_bodies() {
        // Real Responses API success shapes pass.
        assert!(
            assess_responses_body(r#"{"id":"resp_123","object":"response","output":[]}"#).is_ok()
        );
        assert!(assess_responses_body(r#"{"output_text":"pong"}"#).is_ok());
        // Chat-completions style payloads are not a Responses API success.
        assert!(assess_responses_body(r#"{"id":"chatcmpl-1","choices":[]}"#).is_err());
        // HTML page (CDN catch-all) fails even on HTTP 200.
        assert!(assess_responses_body("<!doctype html><html>welcome</html>").is_err());
        // Embedded JSON error fails and surfaces the message.
        let err = assess_responses_body(r#"{"error":{"message":"Insufficient account balance"}}"#)
            .unwrap_err();
        assert!(err.contains("Insufficient account balance"));
        // JSON that is not a model response fails.
        assert!(assess_responses_body(r#"{"status":"ok"}"#).is_err());
        // Empty body fails.
        assert!(assess_responses_body("").is_err());
    }

    #[test]
    fn judges_chat_completion_bodies() {
        assert!(assess_chat_completions_body(
            r#"{"id":"chatcmpl-1","object":"chat.completion","choices":[]}"#
        )
        .is_ok());
        assert!(
            assess_chat_completions_body(r#"{"choices":[{"message":{"content":"pong"}}]}"#).is_ok()
        );
        assert!(assess_chat_completions_body("<!doctype html><html>welcome</html>").is_err());
        let err =
            assess_chat_completions_body(r#"{"error":{"message":"tools are not supported"}}"#)
                .unwrap_err();
        assert!(err.contains("tools are not supported"));
        assert!(assess_chat_completions_body(r#"{"status":"ok"}"#).is_err());
    }

    #[test]
    fn builds_v1_fallback_endpoint() {
        assert_eq!(
            v1_model_endpoint("https://new.sharedchat.cc/codex")
                .unwrap()
                .to_string(),
            "https://new.sharedchat.cc/codex/v1/models"
        );
        assert_eq!(
            v1_model_endpoint("https://api.openai.com/v1")
                .unwrap()
                .to_string(),
            "https://api.openai.com/v1/models"
        );
        assert_eq!(
            v1_model_endpoint("https://proxy.example.com")
                .unwrap()
                .to_string(),
            "https://proxy.example.com/v1/models"
        );
    }

    #[test]
    fn extracts_models() {
        let payload = serde_json::json!({ "data": [{ "id": "gpt-z" }, { "id": "gpt-a" }] });
        assert_eq!(extract_models(&payload), vec!["gpt-a", "gpt-z"]);
    }

    #[test]
    fn builds_responses_endpoint() {
        assert_eq!(
            responses_endpoint("https://api.openai.com/v1")
                .unwrap()
                .to_string(),
            "https://api.openai.com/v1/responses"
        );
        assert_eq!(
            responses_endpoint("https://new.sharedchat.cc/codex")
                .unwrap()
                .to_string(),
            "https://new.sharedchat.cc/codex/responses"
        );
        assert_eq!(
            v1_responses_endpoint("https://new.sharedchat.cc/codex")
                .unwrap()
                .to_string(),
            "https://new.sharedchat.cc/codex/v1/responses"
        );
        assert_eq!(
            responses_endpoint("https://api.example.com/v1/responses")
                .unwrap()
                .to_string(),
            "https://api.example.com/v1/responses"
        );
    }

    #[test]
    fn builds_chat_completions_endpoint() {
        assert_eq!(
            chat_completions_endpoint("https://api.openai.com/v1")
                .unwrap()
                .to_string(),
            "https://api.openai.com/v1/chat/completions"
        );
        assert_eq!(
            chat_completions_endpoint("https://api.example.com")
                .unwrap()
                .to_string(),
            "https://api.example.com/chat/completions"
        );
        assert_eq!(
            v1_chat_completions_endpoint("https://api.example.com")
                .unwrap()
                .to_string(),
            "https://api.example.com/v1/chat/completions"
        );
        assert_eq!(
            chat_completions_endpoint("https://api.example.com/v1/chat/completions")
                .unwrap()
                .to_string(),
            "https://api.example.com/v1/chat/completions"
        );
    }

    #[test]
    fn normalizes_supported_clients() {
        assert_eq!(
            normalize_supported_clients([
                "Codex".to_string(),
                "openclaw".to_string(),
                "unknown".to_string(),
                "hermes".to_string(),
                "codex".to_string(),
            ]),
            vec!["codex", "openclaw", "hermes"]
        );
    }

    #[test]
    fn extracts_error_messages() {
        assert_eq!(
            extract_error_message(r#"{"error":{"message":"No available channel"}}"#),
            "No available channel"
        );
        assert_eq!(extract_error_message(r#"{"message":"bad key"}"#), "bad key");
        assert_eq!(
            extract_error_message("plain text failure"),
            "plain text failure"
        );
        assert_eq!(
            extract_error_message("   "),
            "No response body was returned."
        );
    }
}
