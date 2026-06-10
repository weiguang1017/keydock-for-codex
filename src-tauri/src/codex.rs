use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;

use crate::store::now_iso;
use crate::validate::{mask_key, normalize_or_default, trim, unique_strings};
use crate::{DEFAULT_BASE_URL, DEFAULT_CODEX_PROFILE_LABEL};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexProfile {
    pub directory: String,
    pub config_path: String,
    pub auth_path: String,
    pub has_directory: bool,
    pub has_config: bool,
    pub has_auth: bool,
    pub configured: bool,
    pub label: String,
    pub provider_name: String,
    pub base_url: String,
    pub has_provider_base_url: bool,
    pub model: String,
    pub models: Vec<String>,
    pub masked_key: String,
    pub message: String,
    #[serde(skip)]
    pub api_key: String,
}

#[derive(Debug, Clone, Default)]
pub struct ParsedCodexConfig {
    pub root: HashMap<String, Value>,
    pub providers: HashMap<String, HashMap<String, Value>>,
}

pub fn codex_home() -> PathBuf {
    std::env::var("CODEX_HOME")
        .ok()
        .filter(|value| !trim(value).is_empty())
        .map(PathBuf::from)
        .or_else(|| dirs::home_dir().map(|home| home.join(".codex")))
        .unwrap_or_else(|| std::env::temp_dir().join(".codex"))
}

pub fn read_codex_profile(directory: Option<PathBuf>) -> CodexProfile {
    let directory = directory.unwrap_or_else(codex_home);
    let config_path = directory.join("config.toml");
    let auth_path = directory.join("auth.json");
    let mut profile = CodexProfile {
        directory: path_string(&directory),
        config_path: path_string(&config_path),
        auth_path: path_string(&auth_path),
        has_directory: directory.exists(),
        has_config: config_path.exists(),
        has_auth: auth_path.exists(),
        configured: false,
        label: DEFAULT_CODEX_PROFILE_LABEL.to_string(),
        provider_name: String::new(),
        base_url: DEFAULT_BASE_URL.to_string(),
        has_provider_base_url: false,
        model: String::new(),
        models: Vec::new(),
        masked_key: String::new(),
        message: String::new(),
        api_key: String::new(),
    };

    let mut parsed = ParsedCodexConfig::default();
    if profile.has_config {
        match fs::read_to_string(&config_path) {
            Ok(content) => parsed = parse_codex_config(&content),
            Err(error) => profile.message = error.to_string(),
        }
    }

    let provider_name = first_string([
        parsed.root.get("model_provider"),
        parsed.root.get("default_model_provider"),
        parsed.root.get("provider"),
    ]);
    let provider = provider_name
        .as_ref()
        .and_then(|name| parsed.providers.get(name))
        .or_else(|| parsed.providers.get("OpenAI"))
        .or_else(|| parsed.providers.values().next());

    profile.provider_name = provider_name
        .clone()
        .or_else(|| provider.and_then(|item| item.get("name").and_then(value_string)))
        .unwrap_or_else(|| "OpenAI".to_string());
    profile.label = if profile.provider_name.is_empty() {
        DEFAULT_CODEX_PROFILE_LABEL.to_string()
    } else {
        format!("Codex {}", profile.provider_name)
    };

    let provider_base_url = first_string([
        provider.and_then(|item| item.get("base_url")),
        provider.and_then(|item| item.get("baseURL")),
        provider.and_then(|item| item.get("api_base")),
        provider.and_then(|item| item.get("api_base_url")),
        parsed.root.get("base_url"),
        parsed.root.get("api_base"),
    ]);
    profile.has_provider_base_url = provider_base_url.is_some();
    profile.base_url = provider_base_url.unwrap_or_else(|| DEFAULT_BASE_URL.to_string());
    profile.model = first_string([
        parsed.root.get("model"),
        provider.and_then(|item| item.get("model")),
        parsed.root.get("default_model"),
    ])
    .unwrap_or_default();

    let auth = if profile.has_auth {
        match fs::read_to_string(&auth_path) {
            Ok(content) => serde_json::from_str::<Value>(&content).unwrap_or(Value::Null),
            Err(error) => {
                profile.message = error.to_string();
                Value::Null
            }
        }
    } else {
        Value::Null
    };
    profile.api_key = find_api_key_in_auth(&auth, "");
    profile.masked_key = mask_key(&profile.api_key);
    profile.models = read_codex_model_cache(&directory, &profile.model);
    profile.configured = !profile.api_key.is_empty();

    if !profile.configured && profile.message.is_empty() {
        profile.message = if !profile.has_directory {
            "Codex config directory was not found.".to_string()
        } else if !profile.has_auth {
            "Codex auth.json was not found.".to_string()
        } else {
            "Codex auth.json does not contain an API key.".to_string()
        };
    }

    profile
}

pub fn apply_codex_profile(
    directory: Option<PathBuf>,
    api_key: impl AsRef<str>,
    base_url: impl AsRef<str>,
    model: impl AsRef<str>,
    provider_name: Option<String>,
) -> Result<AppliedCodexProfile, String> {
    let directory = directory.unwrap_or_else(codex_home);
    let api_key = trim(api_key);
    if api_key.is_empty() {
        return Err("API key is required to update Codex configuration.".to_string());
    }
    let base_url = normalize_or_default(base_url)?;
    let model = trim(model);
    let config_path = directory.join("config.toml");
    let auth_path = directory.join("auth.json");
    fs::create_dir_all(&directory).map_err(|error| error.to_string())?;

    let mut config_content = String::new();
    let mut provider_name = provider_name.map(trim).filter(|value| !value.is_empty());
    if config_path.exists() {
        config_content = fs::read_to_string(&config_path).map_err(|error| error.to_string())?;
        if provider_name.is_none() {
            let parsed = parse_codex_config(&config_content);
            provider_name = first_string([
                parsed.root.get("model_provider"),
                parsed.root.get("default_model_provider"),
                parsed.root.get("provider"),
            ]);
        }
        backup_file(&config_path);
    }
    let provider_name = provider_name.unwrap_or_else(|| "OpenAI".to_string());
    if trim(&config_content).is_empty() {
        config_content = default_config_template(&provider_name, &base_url);
    }

    let mut next_config = set_toml_value(&config_content, &[], "model_provider", &provider_name);
    if !model.is_empty() {
        next_config = set_toml_value(&next_config, &[], "model", &model);
    }
    next_config = set_toml_value(
        &next_config,
        &["model_providers", &provider_name],
        "base_url",
        &base_url,
    );
    if !next_config.ends_with('\n') {
        next_config.push('\n');
    }
    atomic_write(&config_path, &next_config)?;

    let mut auth = if auth_path.exists() {
        backup_file(&auth_path);
        fs::read_to_string(&auth_path)
            .ok()
            .and_then(|content| serde_json::from_str::<Value>(&content).ok())
            .unwrap_or_else(|| Value::Object(Default::default()))
    } else {
        Value::Object(Default::default())
    };
    if !auth.is_object() {
        auth = Value::Object(Default::default());
    }
    if let Some(map) = auth.as_object_mut() {
        map.insert("OPENAI_API_KEY".to_string(), Value::String(api_key));
    }
    let auth_content = serde_json::to_string_pretty(&auth).map_err(|error| error.to_string())?;
    atomic_write(&auth_path, &format!("{auth_content}\n"))?;

    Ok(AppliedCodexProfile {
        directory: path_string(&directory),
        config_path: path_string(&config_path),
        auth_path: path_string(&auth_path),
        provider_name,
        base_url,
        model,
    })
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppliedCodexProfile {
    pub directory: String,
    pub config_path: String,
    pub auth_path: String,
    pub provider_name: String,
    pub base_url: String,
    pub model: String,
}

pub fn parse_codex_config(content: &str) -> ParsedCodexConfig {
    let mut parsed = ParsedCodexConfig::default();
    let mut section: Vec<String> = Vec::new();

    for raw_line in content.lines() {
        let line = trim(strip_toml_comment(raw_line));
        if line.is_empty() {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            section = split_toml_path(&line[1..line.len() - 1]);
            continue;
        }
        let Some((key, raw_value)) = line.split_once('=') else {
            continue;
        };
        let key = trim(key);
        let value = parse_toml_value(raw_value);
        if section.first().map(String::as_str) == Some("model_providers") && section.len() >= 2 {
            let provider_name = section[1..].join(".");
            parsed
                .providers
                .entry(provider_name)
                .or_default()
                .insert(key, value);
        } else if section.is_empty() {
            parsed.root.insert(key, value);
        }
    }
    parsed
}

pub fn set_toml_value(content: &str, section_path: &[&str], key: &str, value: &str) -> String {
    let target_key = trim(key);
    let target_section = section_fingerprint(section_path.iter().copied());
    let is_root_target = section_path.is_empty();
    let newline = if content.contains("\r\n") { "\r\n" } else { "\n" };
    let mut lines = content
        .split_inclusive('\n')
        .map(|line| line.trim_end_matches(['\r', '\n']).to_string())
        .collect::<Vec<_>>();
    if lines.is_empty() {
        lines.push(String::new());
    }
    let formatted = format!("{target_key} = {}", format_toml_string(value));
    let mut in_target = is_root_target;
    let mut section_found = is_root_target;
    let mut last_content_in_section: isize = -1;
    let mut first_header_index: isize = -1;

    for index in 0..lines.len() {
        let stripped = trim(strip_toml_comment(&lines[index]));
        if stripped.starts_with('[') && stripped.ends_with(']') {
            if first_header_index == -1 {
                first_header_index = index as isize;
            }
            let section = split_toml_path(&stripped[1..stripped.len() - 1]);
            in_target = !is_root_target
                && section_fingerprint(section.iter().map(String::as_str)) == target_section;
            if in_target {
                section_found = true;
                last_content_in_section = index as isize;
            }
            continue;
        }
        if !in_target {
            continue;
        }
        if !stripped.is_empty() {
            last_content_in_section = index as isize;
        }
        if let Some((line_key, _)) = stripped.split_once('=') {
            if trim(line_key) == target_key {
                lines[index] = formatted;
                return lines.join(newline);
            }
        }
    }

    if is_root_target {
        let insert_at = if first_header_index == -1 {
            lines.len()
        } else {
            first_header_index as usize
        };
        lines.insert(insert_at, formatted);
        return lines.join(newline);
    }
    if section_found {
        lines.insert((last_content_in_section + 1) as usize, formatted);
        return lines.join(newline);
    }

    if lines.last().map(|line| !trim(line).is_empty()).unwrap_or(false) {
        lines.push(String::new());
    }
    lines.push(format!("[{}]", section_path.join(".")));
    lines.push(formatted);
    lines.join(newline)
}

fn read_codex_model_cache(directory: &Path, selected_model: &str) -> Vec<String> {
    let cache_path = directory.join("models_cache.json");
    let mut models = vec![selected_model.to_string()];
    if let Ok(content) = fs::read_to_string(cache_path) {
        if let Ok(payload) = serde_json::from_str::<Value>(&content) {
            if let Some(items) = payload.get("models").and_then(Value::as_array) {
                for item in items {
                    if let Some(value) = item.as_str() {
                        models.push(value.to_string());
                    }
                    if let Some(value) = item.get("slug").and_then(Value::as_str) {
                        models.push(value.to_string());
                    }
                    if let Some(value) = item.get("id").and_then(Value::as_str) {
                        models.push(value.to_string());
                    }
                }
            }
        }
    }
    unique_strings(models)
}

fn find_api_key_in_auth(value: &Value, key_name: &str) -> String {
    if let Some(text) = value.as_str() {
        let lower = key_name.to_ascii_lowercase();
        if lower.contains("api_key")
            || lower.contains("api-key")
            || lower.contains("openai_api_key")
            || text.starts_with("sk-")
        {
            return trim(text);
        }
        return String::new();
    }
    let Some(map) = value.as_object() else {
        return String::new();
    };
    for (key, child) in map {
        let lower = key.to_ascii_lowercase();
        if lower.contains("access_token")
            || lower.contains("access-token")
            || lower.contains("refresh_token")
            || lower.contains("refresh-token")
            || lower.contains("id_token")
            || lower.contains("id-token")
        {
            continue;
        }
        let found = find_api_key_in_auth(child, key);
        if !found.is_empty() {
            return found;
        }
    }
    String::new()
}

fn strip_toml_comment(line: &str) -> String {
    let mut quote = '\0';
    let mut escaped = false;
    let mut result = String::new();
    for ch in line.chars() {
        if escaped {
            result.push(ch);
            escaped = false;
            continue;
        }
        if quote == '"' && ch == '\\' {
            result.push(ch);
            escaped = true;
            continue;
        }
        if (ch == '"' || ch == '\'') && quote == '\0' {
            quote = ch;
            result.push(ch);
            continue;
        }
        if ch == quote {
            quote = '\0';
            result.push(ch);
            continue;
        }
        if ch == '#' && quote == '\0' {
            break;
        }
        result.push(ch);
    }
    result
}

fn split_toml_path(value: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut quote = '\0';
    let mut escaped = false;
    for ch in value.chars() {
        if escaped {
            current.push(ch);
            escaped = false;
            continue;
        }
        if quote == '"' && ch == '\\' {
            current.push(ch);
            escaped = true;
            continue;
        }
        if (ch == '"' || ch == '\'') && quote == '\0' {
            quote = ch;
            current.push(ch);
            continue;
        }
        if ch == quote {
            quote = '\0';
            current.push(ch);
            continue;
        }
        if ch == '.' && quote == '\0' {
            parts.push(unquote_toml(&current));
            current.clear();
            continue;
        }
        current.push(ch);
    }
    if !current.is_empty() {
        parts.push(unquote_toml(&current));
    }
    parts.into_iter().map(trim).filter(|item| !item.is_empty()).collect()
}

fn parse_toml_value(raw_value: &str) -> Value {
    let value = trim(raw_value);
    if value.starts_with('"') || value.starts_with('\'') {
        Value::String(unquote_toml(&value))
    } else if value == "true" {
        Value::Bool(true)
    } else if value == "false" {
        Value::Bool(false)
    } else if value.starts_with('[') && value.ends_with(']') {
        let body = &value[1..value.len() - 1];
        Value::Array(
            body.split(',')
                .map(parse_toml_value)
                .filter(|item| !item.as_str().unwrap_or("").is_empty())
                .collect(),
        )
    } else {
        Value::String(value)
    }
}

fn unquote_toml(value: &str) -> String {
    let input = trim(value);
    if input.len() < 2 {
        return input;
    }
    let quote = input.chars().next().unwrap_or_default();
    if (quote != '"' && quote != '\'') || !input.ends_with(quote) {
        return input;
    }
    let body = &input[1..input.len() - 1];
    if quote == '\'' {
        return body.to_string();
    }
    body.replace("\\\"", "\"")
        .replace("\\\\", "\\")
        .replace("\\n", "\n")
        .replace("\\r", "\r")
        .replace("\\t", "\t")
}

fn format_toml_string(value: &str) -> String {
    let escaped = value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t");
    format!("\"{escaped}\"")
}

fn section_fingerprint<'a>(parts: impl IntoIterator<Item = &'a str>) -> String {
    parts
        .into_iter()
        .map(trim)
        .collect::<Vec<_>>()
        .join("\u{1f}")
}

fn first_string<'a>(values: impl IntoIterator<Item = Option<&'a Value>>) -> Option<String> {
    values.into_iter().find_map(|value| value.and_then(value_string))
}

fn value_string(value: &Value) -> Option<String> {
    value.as_str().map(trim).filter(|value| !value.is_empty())
}

fn default_config_template(provider: &str, base_url: &str) -> String {
    [
        format!("model_provider = {}", format_toml_string(provider)),
        String::new(),
        format!("[model_providers.{provider}]"),
        format!("name = {}", format_toml_string(provider)),
        format!("base_url = {}", format_toml_string(base_url)),
        "wire_api = \"responses\"".to_string(),
        "requires_openai_auth = true".to_string(),
        String::new(),
    ]
    .join("\n")
}

fn backup_file(path: &Path) {
    if path.exists() {
        let _ = fs::copy(path, path.with_extension(format!(
            "{}bak",
            path.extension()
                .and_then(|value| value.to_str())
                .map(|value| format!("{value}."))
                .unwrap_or_default()
        )));
    }
}

fn atomic_write(path: &Path, content: &str) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let tmp_path = path.with_extension(format!("tmp-{}", Uuid::new_v4()));
    fs::write(&tmp_path, content).map_err(|error| error.to_string())?;
    fs::rename(tmp_path, path).map_err(|error| error.to_string())
}

fn path_string(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

#[allow(dead_code)]
fn _timestamp_for_debug() -> String {
    now_iso()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_codex_config() {
        let parsed = parse_codex_config(
            "model_provider = \"OpenAI\"\nmodel = \"gpt-5.5\"\n[model_providers.OpenAI]\nbase_url = \"https://cursorvip.com\"\n",
        );
        assert_eq!(value_string(parsed.root.get("model_provider").unwrap()).unwrap(), "OpenAI");
        assert_eq!(
            value_string(
                parsed
                    .providers
                    .get("OpenAI")
                    .unwrap()
                    .get("base_url")
                    .unwrap()
            )
            .unwrap(),
            "https://cursorvip.com"
        );
    }

    #[test]
    fn edits_toml_in_place() {
        let updated = set_toml_value(
            "[model_providers.OpenAI]\nbase_url = \"https://x\"\n",
            &["model_providers", "OpenAI"],
            "name",
            "OpenAI",
        );
        let parsed = parse_codex_config(&updated);
        assert_eq!(
            value_string(parsed.providers.get("OpenAI").unwrap().get("name").unwrap()).unwrap(),
            "OpenAI"
        );
    }
}
