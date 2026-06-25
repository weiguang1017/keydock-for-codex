use serde::Serialize;

use crate::cli;
use crate::codex::{self, CodexProfile};
use crate::store::{ExportBundle, ImportSummary, KeyRecord, KeydockStore, MetadataUpdate};
use crate::validate::{
    normalize_or_default, normalize_supported_clients, trim, validate_clients_key, ValidationResult,
};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SwitchResult {
    pub restarted: bool,
    pub warning: String,
    pub base_url: String,
    pub model: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CurrentKey {
    pub status: String,
    pub masked_key: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Diagnostics {
    #[serde(skip_serializing_if = "String::is_empty")]
    pub message: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub codex_path: String,
    pub current_key: Option<CurrentKey>,
    pub codex_profile: CodexProfile,
    pub encryption: String,
    /// True when the Codex desktop client is currently running.
    pub codex_desktop_running: bool,
    /// True when a Codex CLI process is currently running.
    pub codex_cli_running: bool,
    /// True when the Codex CLI binary is installed and resolvable.
    pub codex_cli_available: bool,
}

#[tauri::command]
pub fn list_keys() -> Result<Vec<KeyRecord>, String> {
    let store = KeydockStore::default();
    if let Ok(profile) = sync_codex_profile(&store) {
        if profile.configured {
            let _ = store.upsert_codex_profile(&profile);
        }
    }
    store.list()
}

#[tauri::command]
pub fn test_draft_key(
    id: Option<String>,
    base_url: Option<String>,
    api_key: String,
    model: Option<String>,
) -> ValidationResult {
    // In the edit dialog the API key field may be left blank to keep the current
    // key — fall back to the stored secret so detection still works.
    let mut key = trim(&api_key);
    if key.is_empty() {
        if let Some(id) = id.as_ref() {
            if let Ok(secret) = KeydockStore::default().secret(id) {
                key = secret;
            }
        }
    }

    let model = model.unwrap_or_default();
    let result = validate_clients_key(&key, base_url.as_deref(), &model);
    trust_current_codex_config(
        result,
        &codex::read_codex_profile(None),
        &key,
        base_url.as_deref(),
        &model,
    )
}

#[tauri::command]
pub fn add_key(
    label: String,
    base_url: Option<String>,
    api_key: String,
    model: Option<String>,
    validation: Option<ValidationResult>,
) -> Result<KeyRecord, String> {
    let mut check = if validation.as_ref().map(|item| item.valid).unwrap_or(false) {
        let mut result = validation.unwrap();
        if result.status_code == 0 {
            result.status_code = 200;
        }
        if result.message.is_empty() {
            result.message = "The platform accepted this key.".to_string();
        }
        result
    } else {
        validate_clients_key(
            &api_key,
            base_url.as_deref(),
            model.clone().unwrap_or_default(),
        )
    };
    check = trust_current_codex_config(
        check,
        &codex::read_codex_profile(None),
        &api_key,
        base_url.as_deref(),
        model.as_deref().unwrap_or_default(),
    );
    if !check.valid {
        return Err(check.message);
    }
    if let Some(model) = model {
        if !trim(&model).is_empty() {
            check.model = trim(model);
        }
    }
    let store = KeydockStore::default();
    let base_url = base_url.unwrap_or_default();
    store.add(label, base_url, api_key, &check)
}

#[tauri::command]
pub fn update_name(id: String, label: String) -> Result<KeyRecord, String> {
    KeydockStore::default().update_name(id, label)
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportFileResult {
    pub path: String,
    pub count: usize,
    pub cancelled: bool,
}

#[tauri::command]
pub fn export_keys() -> Result<ExportFileResult, String> {
    let bundle = KeydockStore::default().export_bundle()?;
    let count = bundle.keys.len();
    if count == 0 {
        return Ok(ExportFileResult {
            path: String::new(),
            count: 0,
            cancelled: false,
        });
    }
    let json = serde_json::to_string_pretty(&bundle).map_err(|error| error.to_string())?;
    let default_name = format!(
        "keydock-export-{}.json",
        chrono::Local::now().format("%Y%m%d-%H%M%S")
    );

    // Blob downloads do not work inside the Tauri webview, so ask the OS for a
    // save location (editable default file name). If no dialog mechanism exists
    // (e.g. a Linux box without zenity), fall back to the Downloads folder.
    let path = match cli::choose_save_path(&default_name) {
        Ok(Some(path)) => path,
        Ok(None) => {
            return Ok(ExportFileResult {
                path: String::new(),
                count,
                cancelled: true,
            })
        }
        Err(_) => dirs::download_dir()
            .or_else(dirs::home_dir)
            .unwrap_or_else(std::env::temp_dir)
            .join(&default_name),
    };

    std::fs::write(&path, format!("{json}\n")).map_err(|error| error.to_string())?;
    Ok(ExportFileResult {
        path: path.display().to_string(),
        count,
        cancelled: false,
    })
}

#[tauri::command]
pub fn import_keys(content: String) -> Result<ImportSummary, String> {
    let bundle: ExportBundle = serde_json::from_str(&content)
        .map_err(|_| "Could not read this file. Make sure it is a Keydock export.".to_string())?;
    KeydockStore::default().import_bundle(bundle)
}

#[tauri::command]
pub fn update_metadata(
    id: String,
    label: Option<String>,
    base_url: Option<String>,
    model: Option<String>,
    api_key: Option<String>,
    validation: Option<ValidationResult>,
) -> Result<KeyRecord, String> {
    let store = KeydockStore::default();
    let previous = store.list()?.into_iter().find(|item| item.id == id);

    // When a new API key is supplied, replace the stored secret and force a
    // re-check. Leaving the field blank keeps the existing key untouched.
    let key_changed = api_key
        .as_ref()
        .map(|key| !trim(key).is_empty())
        .unwrap_or(false);
    if key_changed {
        store.set_secret(&id, api_key.as_ref().unwrap())?;
    }

    let mut record = store.update_metadata(
        &id,
        MetadataUpdate {
            label,
            base_url,
            model,
            available: if key_changed { Some(false) } else { None },
            validation_message: if key_changed {
                Some("Key changed. Check it again.".to_string())
            } else {
                None
            },
            supported_clients: if key_changed { Some(Vec::new()) } else { None },
            client_support_checked_at: if key_changed {
                Some(String::new())
            } else {
                None
            },
            ..MetadataUpdate::default()
        },
    )?;

    if let Some(result) = validation.as_ref().filter(|result| result.valid) {
        record = store.mark_validation(&id, result)?;
    }

    // For the active key, keep Codex's own config (~/.codex) in sync so the edit
    // actually applies and is not reverted by the next list_keys() profile sync.
    if record.active {
        let base_changed = previous
            .as_ref()
            .map(|item| item.base_url != record.base_url)
            .unwrap_or(true);
        let model_changed = previous
            .as_ref()
            .map(|item| item.model != record.model)
            .unwrap_or(true);
        if key_changed || base_changed || model_changed {
            let secret = store.secret(&id)?;
            codex::apply_codex_profile(None, &secret, &record.base_url, &record.model, None)?;
        }
    }

    Ok(record)
}

#[tauri::command]
pub fn delete_key(id: String) -> Result<bool, String> {
    let store = KeydockStore::default();
    if let Some(record) = store.list()?.into_iter().find(|item| item.id == id) {
        if record.active {
            return Err(
                "The key in use cannot be deleted. Switch to another key first.".to_string(),
            );
        }
    }
    store.remove(id)?;
    Ok(true)
}

#[tauri::command]
pub fn validate_key_cmd(id: String) -> Result<ValidationResult, String> {
    let store = KeydockStore::default();
    let api_key = store.secret(&id)?;
    let record = store.list()?.into_iter().find(|item| item.id == id);
    let base_url = record.as_ref().map(|item| item.base_url.clone());
    let model = record
        .as_ref()
        .map(|item| {
            if trim(&item.model).is_empty() {
                item.models.first().cloned().unwrap_or_default()
            } else {
                item.model.clone()
            }
        })
        .unwrap_or_default();
    // Probe each supported client family with the real endpoint shape it uses.
    let check = trust_current_codex_config(
        validate_clients_key(&api_key, base_url.as_deref(), &model),
        &codex::read_codex_profile(None),
        &api_key,
        base_url.as_deref(),
        &model,
    );
    store.mark_validation(id, &check)?;
    Ok(check)
}

#[tauri::command]
pub fn switch_key(id: String) -> Result<SwitchResult, String> {
    let store = KeydockStore::default();
    let api_key = store.secret(&id)?;
    let record = store
        .list()?
        .into_iter()
        .find(|item| item.id == id)
        .ok_or_else(|| "Key not found.".to_string())?;
    // Apply the config directly. Availability is checked separately via the
    // card "Check" action — a transient upstream error must not block switching.
    codex::apply_codex_profile(None, &api_key, &record.base_url, &record.model, None)?;

    let mut restarted = false;
    let mut warning = String::new();
    let codex_path = cli::find_codex_path().ok();
    match cli::restart_codex_desktop(codex_path.as_ref()) {
        Ok(did_restart) => restarted = did_restart,
        Err(error) => warning = error,
    }
    store.mark_active(id)?;
    Ok(SwitchResult {
        restarted,
        warning,
        base_url: record.base_url,
        model: record.model,
    })
}

#[tauri::command]
pub fn diagnostics() -> Diagnostics {
    let store = KeydockStore::default();
    let profile = sync_codex_profile(&store).unwrap_or_else(|error| {
        let mut profile = codex::read_codex_profile(None);
        profile.configured = false;
        profile.message = error;
        profile
    });

    let desktop_running = cli::is_codex_desktop_running();
    let cli_running = cli::is_codex_cli_running();

    match cli::find_codex_path() {
        Ok(path) => {
            let current_key = cli::read_codex_login(&path).ok().and_then(|status| {
                let masked_key = cli::extract_masked_key_from_status(&status);
                if masked_key.is_empty() {
                    None
                } else {
                    Some(CurrentKey { status, masked_key })
                }
            });
            Diagnostics {
                message: String::new(),
                codex_path: path.to_string_lossy().into_owned(),
                current_key,
                codex_profile: profile,
                encryption: "local fallback".to_string(),
                codex_desktop_running: desktop_running,
                codex_cli_running: cli_running,
                codex_cli_available: true,
            }
        }
        Err(error) => Diagnostics {
            message: error,
            codex_path: String::new(),
            current_key: None,
            codex_profile: profile,
            encryption: "local fallback".to_string(),
            codex_desktop_running: desktop_running,
            codex_cli_running: cli_running,
            codex_cli_available: false,
        },
    }
}

fn sync_codex_profile(store: &KeydockStore) -> Result<CodexProfile, String> {
    let profile = codex::read_codex_profile(None);
    if profile.configured {
        let _ = store.upsert_codex_profile(&profile)?;
    }
    Ok(profile)
}

fn trust_current_codex_config(
    mut result: ValidationResult,
    profile: &CodexProfile,
    api_key: &str,
    base_url: Option<&str>,
    model: &str,
) -> ValidationResult {
    if !codex_profile_matches_key(profile, api_key, base_url, model) {
        return result;
    }

    let mut clients = result.supported_clients.clone();
    clients.push("codex".to_string());
    result.supported_clients = normalize_supported_clients(clients);
    if result.model.is_empty() && !trim(model).is_empty() {
        result.model = trim(model);
    }

    if !result.valid {
        let probe_detail = trim(&result.message);
        result.valid = true;
        result.status_code = 200;
        result.message = if probe_detail.is_empty() {
            "Codex is configured to use this key.".to_string()
        } else {
            format!("Codex is configured to use this key. Other probe result: {probe_detail}")
        };
    }
    result
}

fn codex_profile_matches_key(
    profile: &CodexProfile,
    api_key: &str,
    base_url: Option<&str>,
    model: &str,
) -> bool {
    if !profile.configured || trim(&profile.api_key) != trim(api_key) {
        return false;
    }
    let profile_base = normalize_or_default(&profile.base_url).ok();
    let target_base = normalize_or_default(base_url.unwrap_or_default()).ok();
    if profile_base != target_base {
        return false;
    }
    let profile_model = trim(&profile.model);
    let target_model = trim(model);
    profile_model.is_empty() || target_model.is_empty() || profile_model == target_model
}

#[cfg(test)]
mod tests {
    use super::*;

    fn matching_profile() -> CodexProfile {
        CodexProfile {
            directory: String::new(),
            config_path: String::new(),
            auth_path: String::new(),
            has_directory: true,
            has_config: true,
            has_auth: true,
            configured: true,
            label: "Codex OpenAI".to_string(),
            provider_name: "OpenAI".to_string(),
            base_url: "https://new.sharedchat.cc/codex".to_string(),
            has_provider_base_url: true,
            model: "gpt-5.5".to_string(),
            models: vec![],
            masked_key: "sk-profile...1234".to_string(),
            message: String::new(),
            api_key: "sk-profile-1111111111".to_string(),
        }
    }

    #[test]
    fn trusts_current_codex_config_when_probe_fails() {
        let result = trust_current_codex_config(
            ValidationResult::fail(400, "HTTP 400: unsupported parameter", vec![]),
            &matching_profile(),
            "sk-profile-1111111111",
            Some("https://new.sharedchat.cc/codex"),
            "gpt-5.5",
        );

        assert!(result.valid);
        assert_eq!(result.status_code, 200);
        assert_eq!(result.supported_clients, vec!["codex"]);
        assert!(result.message.contains("Codex is configured"));
        assert!(result.message.contains("unsupported parameter"));
    }

    #[test]
    fn does_not_trust_unmatched_codex_config() {
        let result = trust_current_codex_config(
            ValidationResult::fail(400, "HTTP 400: unsupported parameter", vec![]),
            &matching_profile(),
            "sk-other-1111111111",
            Some("https://new.sharedchat.cc/codex"),
            "gpt-5.5",
        );

        assert!(!result.valid);
        assert!(result.supported_clients.is_empty());
    }
}
