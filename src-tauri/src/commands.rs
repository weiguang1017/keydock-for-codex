use serde::Serialize;

use crate::cli;
use crate::codex::{self, CodexProfile};
use crate::store::{KeyRecord, KeydockStore, MetadataUpdate};
use crate::validate::{trim, validate_key, ValidationResult};

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
pub fn test_draft_key(base_url: Option<String>, api_key: String) -> ValidationResult {
    validate_key(api_key, base_url.as_deref())
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
        validate_key(&api_key, base_url.as_deref())
    };
    if !check.valid {
        return Err(check.message);
    }
    if let Some(model) = model {
        if !trim(&model).is_empty() {
            check.model = trim(model);
        }
    }
    KeydockStore::default().add(label, base_url.unwrap_or_default(), api_key, &check)
}

#[tauri::command]
pub fn update_name(id: String, label: String) -> Result<KeyRecord, String> {
    KeydockStore::default().update_name(id, label)
}

#[tauri::command]
pub fn update_metadata(
    id: String,
    label: Option<String>,
    base_url: Option<String>,
    model: Option<String>,
) -> Result<KeyRecord, String> {
    KeydockStore::default().update_metadata(
        id,
        MetadataUpdate {
            label,
            base_url,
            model,
            ..MetadataUpdate::default()
        },
    )
}

#[tauri::command]
pub fn delete_key(id: String) -> Result<bool, String> {
    KeydockStore::default().remove(id)?;
    Ok(true)
}

#[tauri::command]
pub fn validate_key_cmd(id: String) -> Result<ValidationResult, String> {
    let store = KeydockStore::default();
    let api_key = store.secret(&id)?;
    let record = store.list()?.into_iter().find(|item| item.id == id);
    let check = validate_key(api_key, record.as_ref().map(|item| item.base_url.as_str()));
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
    let check = validate_key(&api_key, Some(&record.base_url));
    if !check.valid {
        return Err(check.message);
    }
    store.mark_validation(&id, &check)?;
    codex::apply_codex_profile(
        None,
        &api_key,
        &record.base_url,
        &record.model,
        None,
    )?;

    let mut restarted = false;
    let mut warning = String::new();
    let codex_path = cli::find_codex_path().ok();
    match cli::restart_codex_desktop(codex_path.as_ref()) {
        Ok(()) => restarted = true,
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

    match cli::find_codex_path() {
        Ok(path) => {
            let current_key = cli::read_codex_login(&path)
                .ok()
                .and_then(|status| {
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
            }
        }
        Err(error) => Diagnostics {
            message: error,
            codex_path: String::new(),
            current_key: None,
            codex_profile: profile,
            encryption: "local fallback".to_string(),
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
