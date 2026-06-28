use serde::Serialize;
use std::{
    collections::HashMap,
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant},
};

use crate::cli;
use crate::codex::{self, CodexProfile};
use crate::store::{ExportBundle, ImportSummary, KeyRecord, KeydockStore, MetadataUpdate};
use crate::validate::{
    normalize_or_default, normalize_supported_clients, probe_chat_completions_key,
    probe_responses_key, trim, unique_strings, validate_clients_key, ValidationResult,
};
use crate::DEFAULT_BASE_URL;

const CLIENT_CODEX: &str = "codex";
const CLIENT_OPENCLAW: &str = "openclaw";
const CLIENT_HERMES: &str = "hermes";
const CODEX_RESPONSES_PROBE: &str = "codex:responses";
const OPENCLAW_CHAT_PROBE: &str = "openclaw:chat_completions";
const CODEX_CLI_PROBE: &str = "codex:cli_exec";
const HERMES_PROBE: &str = "hermes:cli_oneshot_custom_no_fallback_v2";

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
    let codex_profile = if let Ok(profile) = sync_codex_profile(&store) {
        if profile.configured {
            let _ = store.upsert_codex_profile(&profile);
        }
        Some(profile)
    } else {
        None
    };
    let mut records = store.list()?;
    annotate_active_clients(&store, &mut records, codex_profile.as_ref());
    sanitize_supported_clients(&mut records);
    Ok(records)
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
    validate_key_with_client_probes(&key, base_url.as_deref(), &model)
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
        validate_key_with_client_probes(
            &api_key,
            base_url.as_deref(),
            &model.clone().unwrap_or_default(),
        )
    };
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
    let codex_profile = codex::read_codex_profile(None);
    let mut records = store.list()?;
    annotate_active_clients(&store, &mut records, Some(&codex_profile));
    if let Some(record) = records.into_iter().find(|item| item.id == id) {
        if record.active || !record.active_clients.is_empty() {
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
    let check = validate_key_with_client_probes(&api_key, base_url.as_deref(), &model);
    store.mark_validation(id, &check)?;
    Ok(check)
}

#[tauri::command]
pub fn validate_key_client_cmd(id: String, client: String) -> Result<ValidationResult, String> {
    let client = normalize_client_id(&client)?;
    let store = KeydockStore::default();
    let api_key = store.secret(&id)?;
    let record = store
        .list()?
        .into_iter()
        .find(|item| item.id == id)
        .ok_or_else(|| "Key not found.".to_string())?;
    let model = selected_record_model(&record)?;
    let check = validate_key_for_client(&api_key, &record, &client, &model);
    store.mark_validation(&id, &check)?;
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
pub fn switch_key_client(id: String, client: String) -> Result<SwitchResult, String> {
    let client = normalize_client_id(&client)?;
    if client == CLIENT_CODEX {
        return switch_key(id);
    }

    let store = KeydockStore::default();
    let api_key = store.secret(&id)?;
    let record = store
        .list()?
        .into_iter()
        .find(|item| item.id == id)
        .ok_or_else(|| "Key not found.".to_string())?;
    let model = selected_record_model(&record)?;

    // Switching should be a deterministic local config update. Real API/client
    // probes stay behind the explicit "Check" action so a downloaded GUI app
    // without shell proxy variables or CLI probe access does not block switching.
    match client.as_str() {
        CLIENT_HERMES => switch_key_for_hermes(&api_key, &record, &model),
        CLIENT_OPENCLAW => switch_key_for_openclaw(&api_key, &record, &model),
        _ => unreachable!(),
    }
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

fn validate_key_with_client_probes(
    api_key: &str,
    base_url: Option<&str>,
    model: &str,
) -> ValidationResult {
    let mut result = validate_clients_key(api_key, base_url, model);
    let probe_model = selected_probe_model(&result, model);
    if probe_model.is_empty() {
        sanitize_hermes_client_support(&mut result);
        return result;
    }

    if !result
        .supported_clients
        .iter()
        .any(|client| client == CLIENT_CODEX)
        && probe_codex_cli_support(api_key, base_url, &probe_model).is_ok()
    {
        add_verified_client_support(&mut result, CLIENT_CODEX, CODEX_CLI_PROBE, &probe_model);
    }

    if probe_hermes_client_support(api_key, base_url, &probe_model).is_ok() {
        add_verified_client_support(&mut result, CLIENT_HERMES, HERMES_PROBE, &probe_model);
    } else {
        sanitize_hermes_client_support(&mut result);
    }
    result
}

fn validate_key_for_client(
    api_key: &str,
    record: &KeyRecord,
    client: &str,
    model: &str,
) -> ValidationResult {
    let mut merged = validation_from_record(record, model);
    remove_client_support(&mut merged, client);

    let probe = match client {
        CLIENT_CODEX => probe_codex_client_support(api_key, Some(&record.base_url), model),
        CLIENT_OPENCLAW => probe_openclaw_client_support(api_key, Some(&record.base_url), model),
        CLIENT_HERMES => probe_hermes_client_support_result(api_key, Some(&record.base_url), model),
        _ => ValidationResult::fail(0, "Unsupported client.", Vec::new()),
    };
    let client_supported = probe.supported_clients.iter().any(|item| item == client);
    if client_supported {
        for supported in probe.supported_clients.clone() {
            merged.supported_clients.push(supported);
        }
        for probe_name in probe.client_support_probes.clone() {
            merged.client_support_probes.push(probe_name);
        }
        merged.supported_clients = normalize_supported_clients(merged.supported_clients);
        merged.client_support_probes = unique_strings(merged.client_support_probes);
        merged.status_code = if probe.status_code == 0 {
            200
        } else {
            probe.status_code
        };
        merged.message = supported_clients_display_message(&merged.supported_clients);
    } else {
        merged.status_code = probe.status_code;
        let detail = trim(&probe.message);
        let label = client_label(client);
        merged.message = if detail.is_empty() {
            format!("{label} did not accept this key and model.")
        } else {
            format!("{label} did not accept this key and model. {detail}")
        };
        if !merged.supported_clients.is_empty() {
            merged.message = format!(
                "{} {}",
                supported_clients_display_message(&merged.supported_clients),
                merged.message
            );
        }
    }
    merged.valid = !merged.supported_clients.is_empty();
    if !trim(&probe.model).is_empty() {
        merged.model = trim(&probe.model);
    }
    if !probe.models.is_empty() {
        merged.models = probe.models;
    }
    merged
}

fn validation_from_record(record: &KeyRecord, model: &str) -> ValidationResult {
    let mut result = if record.available || !record.supported_clients.is_empty() {
        ValidationResult::ok(record.validation_message.clone(), record.models.clone())
    } else {
        ValidationResult::fail(
            record.status_code,
            record.validation_message.clone(),
            record.models.clone(),
        )
    };
    result.status_code = record.status_code;
    result.model = trim(model);
    result.supported_clients = normalize_supported_clients(record.supported_clients.clone());
    result.client_support_probes = unique_strings(record.client_support_probes.clone());
    sanitize_hermes_client_support(&mut result);
    result
}

fn probe_codex_client_support(
    api_key: &str,
    base_url: Option<&str>,
    model: &str,
) -> ValidationResult {
    let mut result = probe_responses_key(api_key, base_url, model);
    if result.valid {
        add_verified_client_support(&mut result, CLIENT_CODEX, CODEX_RESPONSES_PROBE, model);
        return result;
    }

    let responses_message = trim(&result.message);
    match probe_codex_cli_support(api_key, base_url, model) {
        Ok(()) => {
            let mut result =
                ValidationResult::ok("Codex CLI accepted this key and model.", Vec::new());
            add_verified_client_support(&mut result, CLIENT_CODEX, CODEX_CLI_PROBE, model);
            result
        }
        Err(error) => {
            result.supported_clients = Vec::new();
            result.client_support_probes = Vec::new();
            result.valid = false;
            result.message = format!(
                "Responses probe: {} Codex CLI probe: {}",
                if responses_message.is_empty() {
                    "failed.".to_string()
                } else {
                    responses_message
                },
                trim(error)
            );
            result
        }
    }
}

fn probe_openclaw_client_support(
    api_key: &str,
    base_url: Option<&str>,
    model: &str,
) -> ValidationResult {
    let mut result = probe_chat_completions_key(api_key, base_url, model);
    if result.valid {
        add_verified_client_support(&mut result, CLIENT_OPENCLAW, OPENCLAW_CHAT_PROBE, model);
    } else {
        result.supported_clients = Vec::new();
        result.client_support_probes = Vec::new();
    }
    result
}

fn probe_hermes_client_support_result(
    api_key: &str,
    base_url: Option<&str>,
    model: &str,
) -> ValidationResult {
    match probe_hermes_client_support(api_key, base_url, model) {
        Ok(()) => {
            let mut result =
                ValidationResult::ok("Hermes accepted this key and model.", Vec::new());
            add_verified_client_support(&mut result, CLIENT_HERMES, HERMES_PROBE, model);
            result
        }
        Err(error) => {
            let mut result = ValidationResult::fail(0, error, Vec::new());
            result.model = trim(model);
            result
        }
    }
}

fn remove_client_support(result: &mut ValidationResult, client: &str) {
    result.supported_clients.retain(|item| item != client);
    result
        .client_support_probes
        .retain(|probe| !probe_matches_client(probe, client));
    result.supported_clients = normalize_supported_clients(result.supported_clients.clone());
    result.client_support_probes = unique_strings(result.client_support_probes.clone());
}

fn probe_matches_client(probe: &str, client: &str) -> bool {
    probe.starts_with(&format!("{client}:"))
}

fn normalize_client_id(client: &str) -> Result<String, String> {
    let client = trim(client).to_ascii_lowercase();
    if matches!(
        client.as_str(),
        CLIENT_CODEX | CLIENT_OPENCLAW | CLIENT_HERMES
    ) {
        Ok(client)
    } else {
        Err("Unsupported client.".to_string())
    }
}

fn selected_probe_model(result: &ValidationResult, requested_model: &str) -> String {
    if !trim(requested_model).is_empty() {
        return trim(requested_model);
    }
    if !trim(&result.model).is_empty() {
        return trim(&result.model);
    }
    result.models.first().cloned().unwrap_or_default()
}

fn add_verified_client_support(
    result: &mut ValidationResult,
    client_id: &str,
    probe: &str,
    model: &str,
) {
    let mut clients = result.supported_clients.clone();
    clients.push(client_id.to_string());
    result.supported_clients = normalize_supported_clients(clients);
    let mut probes = result.client_support_probes.clone();
    probes.push(probe.to_string());
    result.client_support_probes = unique_strings(probes);
    if result.model.is_empty() && !trim(model).is_empty() {
        result.model = trim(model);
    }
    if !result.valid {
        result.valid = true;
        result.status_code = 200;
    }
    result.message = supported_clients_display_message(&result.supported_clients);
}

fn sanitize_hermes_client_support(result: &mut ValidationResult) {
    let has_verified_probe = result
        .client_support_probes
        .iter()
        .any(|probe| probe == HERMES_PROBE);
    if has_verified_probe {
        return;
    }
    result
        .supported_clients
        .retain(|client| client != CLIENT_HERMES);
    result
        .client_support_probes
        .retain(|probe| !probe.starts_with("hermes:"));
    if result.supported_clients.is_empty() && result.valid {
        result.message = "No supported client detected.".to_string();
    } else if !result.supported_clients.is_empty() {
        result.message = supported_clients_display_message(&result.supported_clients);
    }
}

fn supported_clients_display_message(clients: &[String]) -> String {
    let labels = clients
        .iter()
        .filter_map(|client| match client.as_str() {
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

fn sanitize_supported_clients(records: &mut [KeyRecord]) {
    for record in records {
        let has_verified_hermes = record
            .client_support_probes
            .iter()
            .any(|probe| probe == HERMES_PROBE);
        if !has_verified_hermes {
            record
                .supported_clients
                .retain(|client| client != CLIENT_HERMES);
            record
                .client_support_probes
                .retain(|probe| !probe.starts_with("hermes:"));
        }
        record.supported_clients = normalize_supported_clients(record.supported_clients.clone());
    }
}

fn annotate_active_clients(
    store: &KeydockStore,
    records: &mut [KeyRecord],
    codex_profile: Option<&CodexProfile>,
) {
    let hermes_profile = read_hermes_profile(None);
    let openclaw_profile = read_openclaw_profile();
    let running = RunningClients::detect();

    for record in records {
        let Ok(api_key) = store.secret(&record.id) else {
            record.active_clients = Vec::new();
            record.running_clients = Vec::new();
            continue;
        };
        let mut active = Vec::new();
        let mut running_clients = Vec::new();
        let model = selected_record_model(record).unwrap_or_default();
        if codex_profile
            .map(|profile| {
                codex_profile_matches_key(profile, &api_key, Some(&record.base_url), &model)
            })
            .unwrap_or(false)
        {
            active.push(CLIENT_CODEX.to_string());
            if running.codex {
                running_clients.push(CLIENT_CODEX.to_string());
            }
        }
        if local_client_profile_matches_key(
            &hermes_profile,
            &api_key,
            Some(&record.base_url),
            &model,
        ) {
            active.push(CLIENT_HERMES.to_string());
            if running.hermes {
                running_clients.push(CLIENT_HERMES.to_string());
            }
        }
        if local_client_profile_matches_key(
            &openclaw_profile,
            &api_key,
            Some(&record.base_url),
            &model,
        ) {
            active.push(CLIENT_OPENCLAW.to_string());
            if running.openclaw {
                running_clients.push(CLIENT_OPENCLAW.to_string());
            }
        }
        record.active_clients = normalize_supported_clients(active);
        record.running_clients = normalize_supported_clients(running_clients);
    }
}

#[derive(Debug, Clone, Copy, Default)]
struct RunningClients {
    codex: bool,
    openclaw: bool,
    hermes: bool,
}

impl RunningClients {
    fn detect() -> Self {
        let codex = cli::is_codex_desktop_running() || cli::is_codex_cli_running();
        let hermes = find_hermes_path()
            .ok()
            .map(|path| hermes_gateway_running(&path))
            .unwrap_or_else(|| process_command_contains("hermes", "gateway"));
        let openclaw = find_openclaw_path()
            .ok()
            .map(|path| openclaw_gateway_running(&path))
            .unwrap_or_else(|| process_command_contains("openclaw", "gateway"));
        Self {
            codex,
            openclaw,
            hermes,
        }
    }
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

#[derive(Default, Debug, Clone)]
struct LocalClientProfile {
    configured: bool,
    api_key: String,
    base_url: String,
    model: String,
}

fn read_hermes_profile(directory: Option<PathBuf>) -> LocalClientProfile {
    let directory = directory.unwrap_or_else(hermes_home);
    let config_path = directory.join("config.yaml");
    let content = match fs::read_to_string(&config_path) {
        Ok(content) => content,
        Err(_) => return LocalClientProfile::default(),
    };
    let model_config = read_simple_yaml_section(&content, "model");
    let mut api_key = model_config.get("api_key").cloned().unwrap_or_default();
    api_key = resolve_env_reference(&directory, &api_key);
    let provider = model_config
        .get("provider")
        .map(|value| value.to_ascii_lowercase())
        .unwrap_or_default();
    if trim(&api_key).is_empty() && provider == "openai" {
        // An implicit OpenAI provider can read Hermes' own .env file, but not
        // Keydock's process environment. Otherwise a shell-level OPENAI_API_KEY
        // can make Hermes look configured when config.yaml never opted into it.
        api_key = read_dotenv_value(&directory, "OPENAI_API_KEY");
    }

    let configured_with_key = !trim(&api_key).is_empty();
    let base_url = {
        let configured_base = model_config.get("base_url").cloned().unwrap_or_default();
        if trim(&configured_base).is_empty() && provider == "openai" && configured_with_key {
            DEFAULT_BASE_URL.to_string()
        } else {
            trim(configured_base)
        }
    };
    let model = model_config
        .get("default")
        .or_else(|| model_config.get("name"))
        .cloned()
        .unwrap_or_default();

    LocalClientProfile {
        configured: configured_with_key && !trim(&base_url).is_empty(),
        api_key: trim(api_key),
        base_url,
        model: trim(model),
    }
}

fn hermes_home() -> PathBuf {
    std::env::var("HERMES_HOME")
        .ok()
        .filter(|value| !trim(value).is_empty())
        .map(PathBuf::from)
        .or_else(|| dirs::home_dir().map(|home| home.join(".hermes")))
        .unwrap_or_else(|| std::env::temp_dir().join(".hermes"))
}

fn local_client_profile_matches_key(
    profile: &LocalClientProfile,
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
    model_names_match(&profile_model, &target_model)
}

fn model_names_match(left: &str, right: &str) -> bool {
    let left = trim(left);
    let right = trim(right);
    if left.is_empty() || right.is_empty() || left == right {
        return true;
    }
    left.rsplit('/').next() == Some(right.as_str())
        || right.rsplit('/').next() == Some(left.as_str())
}

fn selected_record_model(record: &KeyRecord) -> Result<String, String> {
    let model = if trim(&record.model).is_empty() {
        record.models.first().cloned().unwrap_or_default()
    } else {
        record.model.clone()
    };
    let model = trim(model);
    if model.is_empty() {
        Err("A model is required before switching this client.".to_string())
    } else {
        Ok(model)
    }
}

fn client_label(client: &str) -> &'static str {
    match client {
        CLIENT_CODEX => "Codex",
        CLIENT_OPENCLAW => "OpenClaw",
        CLIENT_HERMES => "Hermes",
        _ => "Client",
    }
}

fn switch_key_for_hermes(
    api_key: &str,
    record: &KeyRecord,
    model: &str,
) -> Result<SwitchResult, String> {
    apply_hermes_profile(None, api_key, &record.base_url, model)?;
    let hermes_path = find_hermes_path()?;
    let mut warning = String::new();
    let mut restarted = false;
    if hermes_gateway_running(&hermes_path) {
        match run_command(&hermes_path, &["gateway", "restart"]) {
            Ok(_) => restarted = true,
            Err(error) => warning = error,
        }
    }
    Ok(SwitchResult {
        restarted,
        warning,
        base_url: record.base_url.clone(),
        model: model.to_string(),
    })
}

fn probe_codex_cli_support(
    api_key: &str,
    base_url: Option<&str>,
    model: &str,
) -> Result<(), String> {
    if std::env::var("CKM_SKIP_NETWORK_VALIDATION_FOR_TESTS")
        .ok()
        .as_deref()
        == Some("1")
        || std::env::var("CKM_DISABLE_CODEX_CLI_PROBE").ok().as_deref() == Some("1")
    {
        return Err("Codex CLI probe skipped.".to_string());
    }
    let codex_path = cli::find_codex_path()?;
    let temp_dir =
        std::env::temp_dir().join(format!("keydock-codex-probe-{}", uuid::Uuid::new_v4()));
    fs::create_dir_all(&temp_dir).map_err(|error| error.to_string())?;
    let probe_result = (|| {
        let base_url = normalize_or_default(base_url.unwrap_or_default())?;
        codex::apply_codex_profile(
            Some(temp_dir.clone()),
            api_key,
            &base_url,
            model,
            Some("OpenAI".to_string()),
        )?;
        let mut command = Command::new(&codex_path);
        command
            .env("CODEX_HOME", &temp_dir)
            .args(codex_probe_args(&temp_dir, model));
        let output = run_command_with_timeout(&mut command, Duration::from_secs(75))?;
        if output.status_success {
            Ok(())
        } else {
            Err(command_failure_message("Codex CLI probe failed.", &output))
        }
    })();
    let _ = fs::remove_dir_all(&temp_dir);
    probe_result
}

fn codex_probe_args(temp_dir: &Path, model: &str) -> Vec<String> {
    vec![
        "--ask-for-approval".to_string(),
        "never".to_string(),
        "exec".to_string(),
        "--ignore-rules".to_string(),
        "--skip-git-repo-check".to_string(),
        "--ephemeral".to_string(),
        "--sandbox".to_string(),
        "read-only".to_string(),
        "-C".to_string(),
        temp_dir.display().to_string(),
        "-m".to_string(),
        model.to_string(),
        "Reply with exactly: OK".to_string(),
    ]
}

fn switch_key_for_openclaw(
    api_key: &str,
    record: &KeyRecord,
    model: &str,
) -> Result<SwitchResult, String> {
    let openclaw_path = find_openclaw_path()?;
    apply_openclaw_profile(&openclaw_path, api_key, &record.base_url, model)?;
    let mut warning = String::new();
    let mut restarted = false;
    if openclaw_gateway_running(&openclaw_path) {
        match run_command(&openclaw_path, &["gateway", "restart", "--safe"]) {
            Ok(_) => restarted = true,
            Err(_) => match run_command(&openclaw_path, &["gateway", "restart"]) {
                Ok(_) => restarted = true,
                Err(error) => warning = error,
            },
        }
    }
    Ok(SwitchResult {
        restarted,
        warning,
        base_url: record.base_url.clone(),
        model: model.to_string(),
    })
}

fn probe_hermes_client_support(
    api_key: &str,
    base_url: Option<&str>,
    model: &str,
) -> Result<(), String> {
    if std::env::var("CKM_SKIP_NETWORK_VALIDATION_FOR_TESTS")
        .ok()
        .as_deref()
        == Some("1")
        || std::env::var("CKM_DISABLE_HERMES_PROBE").ok().as_deref() == Some("1")
    {
        return Err("Hermes probe skipped.".to_string());
    }
    let http_probe = probe_chat_completions_key(api_key, base_url, model);
    if !http_probe.valid {
        return Err(format!(
            "Hermes chat-completions preflight failed. {}",
            trim(&http_probe.message)
        ));
    }
    let hermes_path = find_hermes_path()?;
    let temp_dir =
        std::env::temp_dir().join(format!("keydock-hermes-probe-{}", uuid::Uuid::new_v4()));
    fs::create_dir_all(&temp_dir).map_err(|error| error.to_string())?;
    let probe_result = (|| {
        let base_url = normalize_or_default(base_url.unwrap_or_default())?;
        fs::write(
            temp_dir.join("config.yaml"),
            hermes_probe_config_content(api_key, &base_url, model),
        )
        .map_err(|error| error.to_string())?;
        let mut command = Command::new(&hermes_path);
        command
            .env("HERMES_HOME", &temp_dir)
            .env("HERMES_ACCEPT_HOOKS", "1")
            .args([
                "--ignore-rules",
                "--provider",
                "custom",
                "--model",
                model,
                "-z",
                "Reply with exactly: OK",
            ]);
        let output = run_command_with_timeout(&mut command, Duration::from_secs(45))?;
        if let Some(error) = hermes_api_error_from_output(&output) {
            Err(format!("Hermes probe returned an API error. {error}"))
        } else if output.status_success && !hermes_output_looks_like_fallback(&output) {
            Ok(())
        } else if output.status_success {
            Err("Hermes answered through a fallback provider, so this key is not counted as Hermes-compatible.".to_string())
        } else {
            Err(command_failure_message("Hermes probe failed.", &output))
        }
    })();
    let _ = fs::remove_dir_all(&temp_dir);
    probe_result
}

fn apply_hermes_profile(
    directory: Option<PathBuf>,
    api_key: &str,
    base_url: &str,
    model: &str,
) -> Result<(), String> {
    let directory = directory.unwrap_or_else(hermes_home);
    let api_key = trim(api_key);
    if api_key.is_empty() {
        return Err("API key is required to update Hermes configuration.".to_string());
    }
    let base_url = normalize_or_default(base_url)?;
    let model = trim(model);
    if model.is_empty() {
        return Err("A model is required to update Hermes configuration.".to_string());
    }
    fs::create_dir_all(&directory).map_err(|error| error.to_string())?;
    let config_path = directory.join("config.yaml");
    let content = fs::read_to_string(&config_path).unwrap_or_default();
    if config_path.exists() {
        backup_file(&config_path);
    }
    let next = replace_yaml_section(
        &content,
        "model",
        &hermes_model_config_content(&api_key, &base_url, &model),
    );
    let next = replace_yaml_section(&next, "fallback_providers", "fallback_providers: []");
    atomic_write(&config_path, &next)
}

fn hermes_probe_config_content(api_key: &str, base_url: &str, model: &str) -> String {
    format!(
        "{}fallback_providers: []\n",
        hermes_model_config_content(api_key, base_url, model)
    )
}

fn hermes_model_config_content(api_key: &str, base_url: &str, model: &str) -> String {
    format!(
        "model:\n  default: {}\n  provider: custom\n  base_url: {}\n  api_key: {}\n",
        yaml_quote(model),
        yaml_quote(base_url),
        yaml_quote(api_key),
    )
}

fn hermes_output_looks_like_fallback(output: &CommandRun) -> bool {
    let combined = format!("{}\n{}", output.stdout, output.stderr).to_ascii_lowercase();
    [
        "falling back",
        "fallback provider",
        "fallback model",
        "using fallback",
        "switched to fallback",
        "trying fallback",
    ]
    .iter()
    .any(|marker| combined.contains(marker))
}

fn hermes_api_error_from_output(output: &CommandRun) -> Option<String> {
    let combined = trim(format!("{}\n{}", output.stdout, output.stderr));
    if combined.is_empty() {
        return None;
    }
    let lower = combined.to_ascii_lowercase();
    let has_api_error = [
        "api call failed",
        "http 400",
        "http 401",
        "http 403",
        "http 404",
        "http 429",
        "http 500",
        "http 502",
        "http 503",
        "http 504",
        "error code: 400",
        "error code: 401",
        "error code: 403",
        "error code: 404",
        "error code: 429",
        "error code: 500",
        "error code: 502",
        "error code: 503",
        "error code: 504",
        "notfounderror",
        "authenticationerror",
        "permissiondeniederror",
        "ratelimiterror",
        "badrequesterror",
        "api 不支持所选模型",
        "不支持所选模型",
        "model not found",
        "unsupported model",
    ]
    .iter()
    .any(|marker| lower.contains(marker));
    if !has_api_error {
        return None;
    }
    combined
        .lines()
        .map(trim)
        .find(|line| {
            let lower = line.to_ascii_lowercase();
            lower.contains("api call failed")
                || lower.contains("http 4")
                || lower.contains("http 5")
                || lower.contains("error code:")
                || lower.contains("error:")
                || lower.contains("api 不支持所选模型")
                || lower.contains("不支持所选模型")
        })
        .or_else(|| Some(combined))
}

fn replace_yaml_section(content: &str, section: &str, replacement: &str) -> String {
    let replacement = replacement.trim_end();
    if trim(content).is_empty() {
        return format!("{replacement}\n");
    }

    let mut lines = Vec::new();
    let mut replaced = false;
    let mut skipping = false;
    for line in content.lines() {
        let trimmed = line.trim();
        let indent = line.len().saturating_sub(line.trim_start().len());
        if !skipping && indent == 0 && trimmed == format!("{section}:") {
            lines.extend(replacement.lines().map(str::to_string));
            replaced = true;
            skipping = true;
            continue;
        }
        if skipping {
            if !trimmed.is_empty() && indent == 0 && !trimmed.starts_with('-') {
                skipping = false;
            } else {
                continue;
            }
        }
        lines.push(line.to_string());
    }
    if !replaced {
        lines.insert(0, String::new());
        for line in replacement.lines().rev() {
            lines.insert(0, line.to_string());
        }
    }
    let mut next = lines.join("\n");
    if !next.ends_with('\n') {
        next.push('\n');
    }
    next
}

fn yaml_quote(value: &str) -> String {
    format!("'{}'", trim(value).replace('\'', "''"))
}

fn apply_openclaw_profile(
    openclaw_path: &Path,
    api_key: &str,
    base_url: &str,
    model: &str,
) -> Result<(), String> {
    let api_key = trim(api_key);
    if api_key.is_empty() {
        return Err("API key is required to update OpenClaw configuration.".to_string());
    }
    let base_url = normalize_or_default(base_url)?;
    let model = trim(model);
    if model.is_empty() {
        return Err("A model is required to update OpenClaw configuration.".to_string());
    }
    let model_ref = openclaw_model_ref(&model);
    let provider_model = model_ref
        .rsplit('/')
        .next()
        .unwrap_or(model_ref.as_str())
        .to_string();
    let mut agent_models = serde_json::Map::new();
    agent_models.insert(model_ref.clone(), serde_json::json!({}));
    let patch = serde_json::json!({
        "env": {
            "OPENAI_API_KEY": api_key,
        },
        "models": {
            "providers": {
                "openai": {
                    "apiKey": api_key,
                    "baseUrl": base_url,
                    "models": [
                        { "id": provider_model, "name": provider_model }
                    ],
                }
            }
        },
        "agents": {
            "defaults": {
                "model": {
                    "primary": model_ref,
                },
                "models": agent_models,
            }
        }
    });
    let patch = serde_json::to_string_pretty(&patch).map_err(|error| error.to_string())?;
    let output = run_command_with_stdin(openclaw_path, &["config", "patch", "--stdin"], &patch)?;
    if output.status_success {
        Ok(())
    } else {
        Err(command_failure_message(
            "OpenClaw config update failed.",
            &output,
        ))
    }
}

fn openclaw_model_ref(model: &str) -> String {
    let model = trim(model);
    if model.contains('/') {
        model
    } else if let Some((provider, name)) = model.split_once(':') {
        format!("{}/{}", trim(provider), trim(name))
    } else {
        format!("openai/{model}")
    }
}

fn read_openclaw_profile() -> LocalClientProfile {
    let Ok(path) = find_openclaw_path() else {
        return LocalClientProfile::default();
    };
    let api_key = openclaw_config_get_first(
        &path,
        &[
            "models.providers.openai.apiKey",
            "models.providers.openai.api_key",
            "providers.openai.apiKey",
            "providers.openai.api_key",
            "env.OPENAI_API_KEY",
        ],
    )
    .unwrap_or_default();
    let api_key = resolve_env_reference(
        &dirs::home_dir().unwrap_or_else(std::env::temp_dir),
        &api_key,
    );
    let base_url = openclaw_config_get_first(
        &path,
        &[
            "models.providers.openai.baseUrl",
            "models.providers.openai.base_url",
            "providers.openai.baseUrl",
            "providers.openai.base_url",
        ],
    )
    .unwrap_or_default();
    let model = openclaw_config_get_first(
        &path,
        &[
            "agents.defaults.model.primary",
            "agents.defaults.model",
            "agents.defaults.models",
        ],
    )
    .map(|value| openclaw_primary_model_from_value(&value))
    .unwrap_or_default();

    LocalClientProfile {
        configured: !trim(&api_key).is_empty(),
        api_key: trim(api_key),
        base_url: if trim(&base_url).is_empty() {
            DEFAULT_BASE_URL.to_string()
        } else {
            trim(base_url)
        },
        model: trim(model),
    }
}

fn openclaw_config_get_first(path: &Path, keys: &[&str]) -> Option<String> {
    keys.iter().find_map(|key| openclaw_config_get(path, key))
}

fn openclaw_config_get(path: &Path, key: &str) -> Option<String> {
    let output = Command::new(path)
        .args(["config", "get", key])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let value = trim(String::from_utf8_lossy(&output.stdout));
    if value.is_empty() || value == "null" {
        return None;
    }
    Some(parse_scalar_value(&value))
}

fn openclaw_primary_model_from_value(value: &str) -> String {
    let value = trim(value);
    if value.is_empty() {
        return value;
    }
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&value) {
        if let Some(primary) = json.get("primary").and_then(serde_json::Value::as_str) {
            return trim(primary);
        }
        if let Some(map) = json.as_object() {
            if let Some(key) = map.keys().next() {
                return trim(key);
            }
        }
    }
    value
}

fn find_hermes_path() -> Result<PathBuf, String> {
    find_command_path(
        "hermes",
        "CKM_HERMES_PATH",
        &[
            hermes_home().join("hermes-agent/venv/bin/hermes"),
            PathBuf::from("/opt/homebrew/bin/hermes"),
            PathBuf::from("/usr/local/bin/hermes"),
            dirs::home_dir()
                .unwrap_or_else(std::env::temp_dir)
                .join(".local/bin/hermes"),
        ],
    )
}

fn find_openclaw_path() -> Result<PathBuf, String> {
    find_command_path(
        "openclaw",
        "CKM_OPENCLAW_PATH",
        &[
            PathBuf::from("/opt/homebrew/bin/openclaw"),
            PathBuf::from("/usr/local/bin/openclaw"),
            dirs::home_dir()
                .unwrap_or_else(std::env::temp_dir)
                .join(".local/bin/openclaw"),
        ],
    )
}

fn hermes_gateway_running(path: &Path) -> bool {
    let output = Command::new(path).args(["gateway", "status"]).output();
    if let Ok(output) = output {
        let combined = format!(
            "{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
        .to_ascii_lowercase();
        if output.status.success() && (combined.contains("running") || combined.contains("pid")) {
            return true;
        }
    }
    process_command_contains("hermes", "gateway")
}

fn openclaw_gateway_running(path: &Path) -> bool {
    let output = Command::new(path).args(["gateway", "status"]).output();
    if let Ok(output) = output {
        let combined = format!(
            "{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
        .to_ascii_lowercase();
        if output.status.success() && (combined.contains("running") || combined.contains("pid")) {
            return true;
        }
    }
    process_command_contains("openclaw", "gateway")
}

fn process_command_contains(first: &str, second: &str) -> bool {
    #[cfg(target_os = "windows")]
    {
        let output = Command::new("tasklist.exe").output();
        return output
            .map(|output| {
                String::from_utf8_lossy(&output.stdout)
                    .to_ascii_lowercase()
                    .contains(first)
            })
            .unwrap_or(false);
    }

    #[cfg(not(target_os = "windows"))]
    {
        let output = Command::new("ps").args(["-ax", "-o", "command="]).output();
        output
            .map(|output| {
                String::from_utf8_lossy(&output.stdout).lines().any(|line| {
                    let lower = line.to_ascii_lowercase();
                    lower.contains(first)
                        && lower.contains(second)
                        && !lower.contains("keydock-for-codex")
                })
            })
            .unwrap_or(false)
    }
}

#[derive(Debug)]
struct CommandRun {
    status_success: bool,
    stdout: String,
    stderr: String,
}

fn run_command(path: &Path, args: &[&str]) -> Result<String, String> {
    let output = Command::new(path)
        .args(args)
        .output()
        .map_err(|error| error.to_string())?;
    if output.status.success() {
        Ok(trim(String::from_utf8_lossy(&output.stdout)))
    } else {
        Err(command_failure_message(
            "Command failed.",
            &CommandRun {
                status_success: false,
                stdout: trim(String::from_utf8_lossy(&output.stdout)),
                stderr: trim(String::from_utf8_lossy(&output.stderr)),
            },
        ))
    }
}

fn run_command_with_stdin(path: &Path, args: &[&str], stdin: &str) -> Result<CommandRun, String> {
    let mut child = Command::new(path)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| error.to_string())?;
    if let Some(mut input) = child.stdin.take() {
        input
            .write_all(stdin.as_bytes())
            .map_err(|error| error.to_string())?;
    }
    let output = child
        .wait_with_output()
        .map_err(|error| error.to_string())?;
    Ok(CommandRun {
        status_success: output.status.success(),
        stdout: trim(String::from_utf8_lossy(&output.stdout)),
        stderr: trim(String::from_utf8_lossy(&output.stderr)),
    })
}

fn run_command_with_timeout(
    command: &mut Command,
    timeout: Duration,
) -> Result<CommandRun, String> {
    let mut child = command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| error.to_string())?;
    let deadline = Instant::now() + timeout;
    loop {
        if child
            .try_wait()
            .map_err(|error| error.to_string())?
            .is_some()
        {
            let output = child
                .wait_with_output()
                .map_err(|error| error.to_string())?;
            return Ok(CommandRun {
                status_success: output.status.success(),
                stdout: trim(String::from_utf8_lossy(&output.stdout)),
                stderr: trim(String::from_utf8_lossy(&output.stderr)),
            });
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let output = child
                .wait_with_output()
                .map_err(|error| error.to_string())?;
            return Ok(CommandRun {
                status_success: false,
                stdout: trim(String::from_utf8_lossy(&output.stdout)),
                stderr: if output.stderr.is_empty() {
                    "Command timed out.".to_string()
                } else {
                    trim(String::from_utf8_lossy(&output.stderr))
                },
            });
        }
        std::thread::sleep(Duration::from_millis(250));
    }
}

fn command_failure_message(prefix: &str, output: &CommandRun) -> String {
    let detail = if !trim(&output.stderr).is_empty() {
        trim(&output.stderr)
    } else if !trim(&output.stdout).is_empty() {
        trim(&output.stdout)
    } else {
        "No output was returned.".to_string()
    };
    format!("{prefix} {detail}")
}

fn find_command_path(name: &str, env_var: &str, candidates: &[PathBuf]) -> Result<PathBuf, String> {
    if let Ok(path) = std::env::var(env_var) {
        let path = PathBuf::from(path);
        if path.exists() {
            return Ok(path);
        }
    }
    if let Ok(path) = shell_lookup_command(name) {
        if path.exists() {
            return Ok(path);
        }
    }
    candidates
        .iter()
        .find(|path| path.exists())
        .cloned()
        .ok_or_else(|| format!("{name} CLI was not found. Install or configure it first."))
}

fn shell_lookup_command(name: &str) -> Result<PathBuf, String> {
    if !name
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_')
    {
        return Err("Invalid command name.".to_string());
    }
    #[cfg(target_os = "windows")]
    let output = Command::new("cmd.exe")
        .args(["/c", &format!("where {name}")])
        .output()
        .map_err(|error| error.to_string())?;

    #[cfg(not(target_os = "windows"))]
    let output = {
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".to_string());
        Command::new(shell)
            .args(["-lc", &format!("command -v {name}")])
            .output()
            .map_err(|error| error.to_string())?
    };

    if !output.status.success() {
        return Err(format!("{name} was not found in PATH."));
    }
    trim(String::from_utf8_lossy(&output.stdout))
        .lines()
        .next()
        .map(PathBuf::from)
        .filter(|path| !path.as_os_str().is_empty())
        .ok_or_else(|| format!("{name} was not found in PATH."))
}

fn backup_file(path: &Path) {
    if path.exists() {
        let backup = path.with_extension(format!(
            "{}.keydock.bak",
            path.extension()
                .and_then(|value| value.to_str())
                .unwrap_or("bak")
        ));
        let _ = fs::copy(path, backup);
    }
}

fn atomic_write(path: &Path, content: &str) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let temp_path = path.with_extension(format!(
        "{}.tmp",
        path.extension()
            .and_then(|value| value.to_str())
            .unwrap_or("tmp")
    ));
    fs::write(&temp_path, content).map_err(|error| error.to_string())?;
    fs::rename(&temp_path, path).map_err(|error| error.to_string())
}

fn read_simple_yaml_section(content: &str, section: &str) -> HashMap<String, String> {
    let mut values = HashMap::new();
    let mut in_section = false;
    let mut section_indent = 0usize;

    for line in content.lines() {
        let raw = line.trim_end();
        let trimmed = raw.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let indent = raw.len().saturating_sub(raw.trim_start().len());
        if !in_section {
            if indent == 0 && trimmed == format!("{section}:") {
                in_section = true;
                section_indent = indent;
            }
            continue;
        }
        if indent <= section_indent {
            break;
        }
        let Some((key, value)) = trimmed.split_once(':') else {
            continue;
        };
        let key = trim(key);
        if key.is_empty() || key.starts_with('-') || value.trim_start().starts_with('|') {
            continue;
        }
        values.insert(key, parse_scalar_value(value));
    }

    values
}

fn read_dotenv_value(directory: &Path, key: &str) -> String {
    if let Ok(content) = fs::read_to_string(directory.join(".env")) {
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            let Some((name, value)) = trimmed.split_once('=') else {
                continue;
            };
            if trim(name) == key {
                return parse_scalar_value(value);
            }
        }
    }
    String::new()
}

fn read_env_value(directory: &Path, key: &str) -> String {
    let dotenv_value = read_dotenv_value(directory, key);
    if !dotenv_value.is_empty() {
        return dotenv_value;
    }
    if let Ok(value) = std::env::var(key) {
        if !trim(&value).is_empty() {
            return trim(value);
        }
    }
    String::new()
}

fn resolve_env_reference(directory: &Path, value: &str) -> String {
    let value = trim(value);
    let env_name = value
        .strip_prefix("${")
        .and_then(|item| item.strip_suffix('}'))
        .or_else(|| value.strip_prefix('$'));
    if let Some(env_name) = env_name {
        let resolved = read_env_value(directory, env_name);
        if !resolved.is_empty() {
            return resolved;
        }
    }
    value
}

fn parse_scalar_value(value: &str) -> String {
    let mut value = value.trim().to_string();
    if let Some(index) = value.find(" #") {
        value.truncate(index);
    }
    let value = value.trim();
    value
        .strip_prefix('"')
        .and_then(|item| item.strip_suffix('"'))
        .or_else(|| {
            value
                .strip_prefix('\'')
                .and_then(|item| item.strip_suffix('\''))
        })
        .unwrap_or(value)
        .trim()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsString;
    use uuid::Uuid;

    fn restore_env_var(name: &str, value: Option<OsString>) {
        if let Some(value) = value {
            std::env::set_var(name, value);
        } else {
            std::env::remove_var(name);
        }
    }

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
    fn local_profiles_match_for_active_state_only() {
        let profile = LocalClientProfile {
            configured: true,
            api_key: "sk-hermes-1111111111".to_string(),
            base_url: "https://new.sharedchat.cc/codex".to_string(),
            model: "gpt-5.5".to_string(),
        };

        assert!(codex_profile_matches_key(
            &matching_profile(),
            "sk-profile-1111111111",
            Some("https://new.sharedchat.cc/codex"),
            "gpt-5.5",
        ));
        assert!(local_client_profile_matches_key(
            &profile,
            "sk-hermes-1111111111",
            Some("https://new.sharedchat.cc/codex"),
            "gpt-5.5",
        ));
    }

    #[test]
    fn codex_switch_writes_config_without_revalidating_client_support() {
        let store_dir = std::env::temp_dir().join(format!("keydock-store-{}", Uuid::new_v4()));
        let codex_dir = std::env::temp_dir().join(format!("keydock-codex-{}", Uuid::new_v4()));

        let previous_store_dir = std::env::var_os("CKM_STORE_DIR");
        let previous_codex_home = std::env::var_os("CODEX_HOME");
        let previous_disable_restart = std::env::var_os("CKM_DISABLE_RESTART");
        std::env::set_var("CKM_STORE_DIR", &store_dir);
        std::env::set_var("CODEX_HOME", &codex_dir);
        std::env::set_var("CKM_DISABLE_RESTART", "1");

        let result = (|| {
            let mut validation = ValidationResult::ok("Saved without client probes.", vec![]);
            validation.model = "gpt-test-switch".to_string();
            validation.supported_clients = Vec::new();
            validation.client_support_probes = Vec::new();
            let record = KeydockStore::default().add(
                "Probe-free Codex",
                "https://127.0.0.1:9/v1",
                "sk-fake-switch-1111111111",
                &validation,
            )?;

            let switched = switch_key_client(record.id.clone(), CLIENT_CODEX.to_string())?;
            let profile = codex::read_codex_profile(Some(codex_dir.clone()));
            let stored = KeydockStore::default()
                .list()?
                .into_iter()
                .find(|item| item.id == record.id)
                .ok_or_else(|| "Key not found after switch.".to_string())?;

            assert_eq!(switched.base_url, "https://127.0.0.1:9/v1");
            assert_eq!(profile.api_key, "sk-fake-switch-1111111111");
            assert_eq!(profile.base_url, "https://127.0.0.1:9/v1");
            assert_eq!(profile.model, "gpt-test-switch");
            assert!(stored.active);
            assert!(stored.supported_clients.is_empty());
            Ok::<(), String>(())
        })();

        restore_env_var("CKM_STORE_DIR", previous_store_dir);
        restore_env_var("CODEX_HOME", previous_codex_home);
        restore_env_var("CKM_DISABLE_RESTART", previous_disable_restart);
        let _ = fs::remove_dir_all(store_dir);
        let _ = fs::remove_dir_all(codex_dir);

        result.unwrap();
    }

    #[test]
    fn reads_hermes_model_config() {
        let dir = std::env::temp_dir().join(format!("keydock-hermes-{}", Uuid::new_v4()));
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join("config.yaml"),
            r#"
model:
  default: "gpt-5.5"
  provider: custom
  base_url: "https://new.sharedchat.cc/codex"
  api_key: 'sk-hermes-1111111111'
providers: {}
"#,
        )
        .unwrap();

        let profile = read_hermes_profile(Some(dir.clone()));

        assert!(profile.configured);
        assert_eq!(profile.api_key, "sk-hermes-1111111111");
        assert_eq!(profile.base_url, "https://new.sharedchat.cc/codex");
        assert_eq!(profile.model, "gpt-5.5");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn reads_hermes_openai_env_key() {
        let dir = std::env::temp_dir().join(format!("keydock-hermes-env-{}", Uuid::new_v4()));
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join("config.yaml"),
            r#"
model:
  default: gpt-5.5
  provider: openai
  base_url: https://api.openai.com/v1
"#,
        )
        .unwrap();
        fs::write(dir.join(".env"), "OPENAI_API_KEY=sk-env-1111111111\n").unwrap();

        let profile = read_hermes_profile(Some(dir.clone()));

        assert!(profile.configured);
        assert_eq!(profile.api_key, "sk-env-1111111111");
        assert_eq!(profile.base_url, "https://api.openai.com/v1");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn ignores_process_env_for_implicit_hermes_openai_key() {
        let dir =
            std::env::temp_dir().join(format!("keydock-hermes-process-env-{}", Uuid::new_v4()));
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join("config.yaml"),
            r#"
model:
  default: gpt-5.5
  provider: openai
  base_url: https://new.sharedchat.cc/codex
"#,
        )
        .unwrap();

        let previous = std::env::var_os("OPENAI_API_KEY");
        std::env::set_var("OPENAI_API_KEY", "sk-process-should-not-count");
        let profile = read_hermes_profile(Some(dir.clone()));
        if let Some(value) = previous {
            std::env::set_var("OPENAI_API_KEY", value);
        } else {
            std::env::remove_var("OPENAI_API_KEY");
        }

        assert!(!profile.configured);
        assert!(profile.api_key.is_empty());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn reads_hermes_env_reference_key() {
        let dir = std::env::temp_dir().join(format!("keydock-hermes-env-ref-{}", Uuid::new_v4()));
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join("config.yaml"),
            r#"
model:
  default: gpt-5.5
  provider: custom
  base_url: https://new.sharedchat.cc/codex
  api_key: ${HERMES_SHARED_KEY}
"#,
        )
        .unwrap();
        fs::write(
            dir.join(".env"),
            "HERMES_SHARED_KEY=sk-env-ref-1111111111\n",
        )
        .unwrap();

        let profile = read_hermes_profile(Some(dir.clone()));

        assert!(profile.configured);
        assert_eq!(profile.api_key, "sk-env-ref-1111111111");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn builds_codex_probe_args_with_root_approval_policy() {
        let temp_dir = Path::new("/tmp/keydock-codex-probe-test");
        let args = codex_probe_args(temp_dir, "gpt-5.5");
        let approval_index = args
            .iter()
            .position(|arg| arg == "--ask-for-approval")
            .unwrap();
        let exec_index = args.iter().position(|arg| arg == "exec").unwrap();

        assert!(approval_index < exec_index);
        assert_eq!(args[approval_index + 1], "never");
        assert_eq!(args[exec_index + 1], "--ignore-rules");
        assert!(args.windows(2).any(|pair| pair == ["-m", "gpt-5.5"]));
    }

    #[test]
    fn unmatched_local_profiles_do_not_match_active_state() {
        let profile = LocalClientProfile {
            configured: true,
            api_key: "sk-hermes-1111111111".to_string(),
            base_url: "https://new.sharedchat.cc/codex".to_string(),
            model: "gpt-5.5".to_string(),
        };

        assert!(!codex_profile_matches_key(
            &matching_profile(),
            "sk-other-1111111111",
            Some("https://new.sharedchat.cc/codex"),
            "gpt-5.5",
        ));
        assert!(!local_client_profile_matches_key(
            &profile,
            "sk-other-1111111111",
            Some("https://new.sharedchat.cc/codex"),
            "gpt-5.5",
        ));
    }

    #[test]
    fn matches_provider_prefixed_model_names() {
        assert!(model_names_match("openai/gpt-5.5", "gpt-5.5"));
        assert!(model_names_match("gpt-5.5", "openai/gpt-5.5"));
        assert!(model_names_match("openai/gpt-5.5", "openai/gpt-5.5"));
        assert!(!model_names_match(
            "openai/gpt-5.5",
            "anthropic/claude-sonnet"
        ));
    }

    #[test]
    fn extracts_openclaw_primary_model_from_config_values() {
        assert_eq!(
            openclaw_primary_model_from_value(r#"{"primary":"openai/gpt-5.5"}"#),
            "openai/gpt-5.5"
        );
        assert_eq!(
            openclaw_primary_model_from_value(r#"{"openai/gpt-5.5":{}}"#),
            "openai/gpt-5.5"
        );
        assert_eq!(
            openclaw_primary_model_from_value("openai/gpt-5.4"),
            "openai/gpt-5.4"
        );
    }

    #[test]
    fn strips_hermes_support_without_verified_probe() {
        let mut result = ValidationResult::ok("ok", vec![]);
        result.supported_clients = vec!["codex".to_string(), "hermes".to_string()];

        sanitize_hermes_client_support(&mut result);

        assert_eq!(result.supported_clients, vec!["codex"]);
        assert!(result.client_support_probes.is_empty());
    }

    #[test]
    fn strips_legacy_hermes_probe_marker() {
        let mut result = ValidationResult::ok("ok", vec![]);
        result.supported_clients = vec!["hermes".to_string()];
        result.client_support_probes = vec![
            "hermes:oneshot".to_string(),
            "hermes:oneshot_no_fallback".to_string(),
        ];

        sanitize_hermes_client_support(&mut result);

        assert!(result.supported_clients.is_empty());
        assert!(result.client_support_probes.is_empty());
    }

    #[test]
    fn keeps_hermes_support_with_verified_probe() {
        let mut result = ValidationResult::ok("ok", vec![]);
        add_verified_client_support(&mut result, CLIENT_HERMES, HERMES_PROBE, "gpt-5.5");

        assert_eq!(result.supported_clients, vec!["hermes"]);
        assert_eq!(result.client_support_probes, vec![HERMES_PROBE]);
        assert_eq!(result.model, "gpt-5.5");
    }

    #[test]
    fn detects_hermes_api_error_even_when_process_succeeds() {
        let output = CommandRun {
            status_success: true,
            stdout: "API call failed after 3 retries: HTTP 404: Error code: 404 - {'error': '当前 API 不支持所选模型 gpt-5.5', 'type': 'error'}".to_string(),
            stderr: String::new(),
        };

        let error = hermes_api_error_from_output(&output).unwrap();

        assert!(error.contains("HTTP 404"));
        assert!(error.contains("不支持所选模型"));
    }

    #[test]
    fn accepts_clean_hermes_probe_output() {
        let output = CommandRun {
            status_success: true,
            stdout: "OK".to_string(),
            stderr: String::new(),
        };

        assert!(hermes_api_error_from_output(&output).is_none());
    }

    #[test]
    fn remove_client_support_keeps_other_client_probes() {
        let mut result = ValidationResult::ok("ok", vec![]);
        result.supported_clients = vec![
            "codex".to_string(),
            "openclaw".to_string(),
            "hermes".to_string(),
        ];
        result.client_support_probes = vec![
            CODEX_RESPONSES_PROBE.to_string(),
            OPENCLAW_CHAT_PROBE.to_string(),
            HERMES_PROBE.to_string(),
        ];

        remove_client_support(&mut result, CLIENT_HERMES);

        assert_eq!(result.supported_clients, vec!["codex", "openclaw"]);
        assert_eq!(
            result.client_support_probes,
            vec![CODEX_RESPONSES_PROBE, OPENCLAW_CHAT_PROBE]
        );
    }

    #[test]
    fn applies_hermes_model_section_and_clears_fallback_chain() {
        let content = r#"
display:
  interface: tui
model:
  default: old-model
  provider: custom
  base_url: https://old.example/v1
  api_key: old-key
fallback_providers:
- provider: nvidia
  model: minimaxai/minimax-m2.7
"#;

        let next = replace_yaml_section(
            content,
            "model",
            &hermes_model_config_content(
                "sk-new'quoted",
                "https://new.sharedchat.cc/codex",
                "gpt-5.5",
            ),
        );
        let next = replace_yaml_section(&next, "fallback_providers", "fallback_providers: []");

        assert!(next.contains("display:\n  interface: tui"));
        assert!(next.contains("default: 'gpt-5.5'"));
        assert!(next.contains("api_key: 'sk-new''quoted'"));
        assert!(next.contains("fallback_providers: []"));
        assert!(!next.contains("minimaxai/minimax-m2.7"));
        assert!(!next.contains("old-model"));
    }

    #[test]
    fn detects_hermes_fallback_output() {
        let output = CommandRun {
            status_success: true,
            stdout: "OK".to_string(),
            stderr: "Falling back to provider nvidia".to_string(),
        };

        assert!(hermes_output_looks_like_fallback(&output));

        let output = CommandRun {
            status_success: true,
            stdout: "OK".to_string(),
            stderr: String::new(),
        };

        assert!(!hermes_output_looks_like_fallback(&output));
    }
}
