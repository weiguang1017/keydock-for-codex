use base64::{engine::general_purpose::STANDARD, Engine as _};
use chrono::{SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;

use crate::codex::CodexProfile;
use crate::validate::{
    mask_key, normalize_or_default, trim, unique_strings, ValidationResult,
};
use crate::{APP_NAME, DEFAULT_CODEX_PROFILE_LABEL};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyRecord {
    pub id: String,
    pub label: String,
    pub base_url: String,
    pub masked_key: String,
    pub model: String,
    pub models: Vec<String>,
    pub available: bool,
    pub status_code: u16,
    pub validation_message: String,
    pub active: bool,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub source: String,
    pub last_validated_at: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MetadataUpdate {
    pub label: Option<String>,
    pub base_url: Option<String>,
    pub model: Option<String>,
    pub models: Option<Vec<String>>,
    pub available: Option<bool>,
    pub status_code: Option<u16>,
    pub validation_message: Option<String>,
    pub last_validated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SecretPayload {
    mode: String,
    value: String,
}

const EXPORT_KIND: &str = "keydock-export";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportedKey {
    pub label: String,
    pub base_url: String,
    pub api_key: String,
    #[serde(default)]
    pub model: String,
    #[serde(default)]
    pub models: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportBundle {
    pub kind: String,
    pub version: u32,
    #[serde(default)]
    pub exported_at: String,
    pub keys: Vec<ExportedKey>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportSummary {
    pub added: usize,
    pub skipped: usize,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct KeyList {
    keys: Vec<KeyRecord>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct SecretRoot {
    secrets: HashMap<String, SecretPayload>,
}

#[derive(Debug, Clone)]
pub struct KeydockStore {
    keys_path: PathBuf,
    secrets_path: PathBuf,
}

impl KeydockStore {
    pub fn default() -> Self {
        Self::new(default_data_dir())
    }

    pub fn new(directory: impl Into<PathBuf>) -> Self {
        let directory = directory.into();
        Self {
            keys_path: directory.join("keys.json"),
            secrets_path: directory.join("secrets.json"),
        }
    }

    pub fn list(&self) -> Result<Vec<KeyRecord>, String> {
        Ok(read_json::<KeyList>(&self.keys_path)?.keys)
    }

    fn save_list(&self, records: Vec<KeyRecord>) -> Result<(), String> {
        write_json(&self.keys_path, &KeyList { keys: records })
    }

    fn secrets(&self) -> Result<SecretRoot, String> {
        read_json::<SecretRoot>(&self.secrets_path)
    }

    fn save_secrets(&self, root: &SecretRoot) -> Result<(), String> {
        write_json(&self.secrets_path, root)
    }

    fn protect(&self, secret: &str) -> SecretPayload {
        SecretPayload {
            mode: "plain-fallback".to_string(),
            value: STANDARD.encode(secret.as_bytes()),
        }
    }

    fn reveal(&self, payload: &SecretPayload) -> Result<String, String> {
        if payload.mode != "plain-fallback" {
            return Err("Unsupported secret storage mode.".to_string());
        }
        let bytes = STANDARD
            .decode(payload.value.as_bytes())
            .map_err(|error| error.to_string())?;
        String::from_utf8(bytes).map_err(|error| error.to_string())
    }

    pub fn add(
        &self,
        label: impl AsRef<str>,
        base_url: impl AsRef<str>,
        api_key: impl AsRef<str>,
        validation: &ValidationResult,
    ) -> Result<KeyRecord, String> {
        let mut records = self.list()?;
        let id = Uuid::new_v4().to_string();
        let timestamp = now_iso();
        let models = unique_strings(validation.models.clone());
        let selected_model = if !trim(&validation.model).is_empty() {
            trim(&validation.model)
        } else {
            models.first().cloned().unwrap_or_default()
        };
        let record = KeyRecord {
            id: id.clone(),
            label: {
                let label = trim(label);
                if label.is_empty() {
                    "Untitled key".to_string()
                } else {
                    label
                }
            },
            base_url: normalize_or_default(base_url)?,
            masked_key: mask_key(&api_key),
            model: selected_model,
            models,
            available: validation.valid,
            status_code: validation.status_code,
            validation_message: trim(&validation.message),
            active: false,
            source: String::new(),
            last_validated_at: timestamp.clone(),
            created_at: timestamp.clone(),
            updated_at: timestamp,
        };
        records.push(record.clone());

        let mut secrets = self.secrets()?;
        secrets.secrets.insert(id, self.protect(api_key.as_ref()));
        self.save_secrets(&secrets)?;
        self.save_list(records)?;
        Ok(record)
    }

    pub fn upsert_codex_profile(&self, profile: &CodexProfile) -> Result<Option<KeyRecord>, String> {
        if !profile.configured || profile.api_key.is_empty() {
            return Ok(None);
        }

        let mut records = self.list()?;
        let mut secrets = self.secrets()?;
        let timestamp = now_iso();
        let base_url = normalize_or_default(&profile.base_url)?;
        let models = unique_strings(profile.models.clone());
        let selected_model = if !trim(&profile.model).is_empty() {
            trim(&profile.model)
        } else {
            models.first().cloned().unwrap_or_default()
        };
        let masked_key = mask_key(&profile.api_key);

        let matching_index = records.iter().position(|item| {
            secrets
                .secrets
                .get(&item.id)
                .and_then(|payload| self.reveal(payload).ok())
                .map(|secret| secret == profile.api_key)
                .unwrap_or(false)
        });
        let fallback_index = matching_index
            .or_else(|| records.iter().position(|item| item.source == "codex-config"))
            .or_else(|| {
                records.iter().position(|item| {
                    item.masked_key == masked_key
                        && normalize_or_default(&item.base_url).ok().as_deref()
                            == Some(base_url.as_str())
                })
            });

        let record_id;
        if let Some(index) = fallback_index {
            let record = &mut records[index];
            if record.label.is_empty() {
                record.label = if profile.label.is_empty() {
                    DEFAULT_CODEX_PROFILE_LABEL.to_string()
                } else {
                    profile.label.clone()
                };
            }
            record.base_url = base_url;
            record.masked_key = masked_key;
            if !selected_model.is_empty() {
                record.model = selected_model;
            }
            record.models = unique_strings(models.into_iter().chain(record.models.clone()));
            if !profile.message.is_empty() {
                record.validation_message = profile.message.clone();
            } else if record.validation_message.is_empty() {
                record.validation_message = "Imported from Codex config.".to_string();
            }
            if matching_index.is_none() {
                record.source = "codex-config".to_string();
            }
            record.updated_at = timestamp.clone();
            record_id = record.id.clone();
        } else {
            let record = KeyRecord {
                id: Uuid::new_v4().to_string(),
                label: if profile.label.is_empty() {
                    DEFAULT_CODEX_PROFILE_LABEL.to_string()
                } else {
                    profile.label.clone()
                },
                base_url,
                masked_key,
                model: selected_model,
                models,
                available: false,
                status_code: 0,
                validation_message: if profile.message.is_empty() {
                    "Imported from Codex config.".to_string()
                } else {
                    profile.message.clone()
                },
                active: false,
                source: "codex-config".to_string(),
                last_validated_at: String::new(),
                created_at: timestamp.clone(),
                updated_at: timestamp.clone(),
            };
            record_id = record.id.clone();
            records.push(record);
        }

        for item in &mut records {
            item.active = item.id == record_id;
            if item.active {
                item.updated_at = timestamp.clone();
            }
        }
        secrets
            .secrets
            .insert(record_id.clone(), self.protect(&profile.api_key));
        let saved = records.iter().find(|item| item.id == record_id).cloned();
        self.save_secrets(&secrets)?;
        self.save_list(records)?;
        Ok(saved)
    }

    pub fn update_metadata(
        &self,
        id: impl AsRef<str>,
        updates: MetadataUpdate,
    ) -> Result<KeyRecord, String> {
        let id = id.as_ref();
        let mut records = self.list()?;
        let record = records
            .iter_mut()
            .find(|item| item.id == id)
            .ok_or_else(|| "Key not found.".to_string())?;

        if let Some(label) = updates.label {
            record.label = {
                let label = trim(label);
                if label.is_empty() {
                    "Untitled key".to_string()
                } else {
                    label
                }
            };
        }
        if let Some(base_url) = updates.base_url {
            let next_base_url = normalize_or_default(base_url)?;
            if next_base_url != record.base_url {
                record.available = false;
                record.models = Vec::new();
                record.model = String::new();
                record.validation_message = "Base URL changed. Check the key again.".to_string();
            }
            record.base_url = next_base_url;
        }
        if let Some(model) = updates.model {
            record.model = trim(model);
        }
        if let Some(models) = updates.models {
            record.models = unique_strings(models);
        }
        if let Some(available) = updates.available {
            record.available = available;
        }
        if let Some(status_code) = updates.status_code {
            record.status_code = status_code;
        }
        if let Some(message) = updates.validation_message {
            record.validation_message = trim(message);
        }
        if let Some(last_validated_at) = updates.last_validated_at {
            record.last_validated_at = last_validated_at;
        }
        record.updated_at = now_iso();
        let record = record.clone();
        self.save_list(records)?;
        Ok(record)
    }

    pub fn update_name(&self, id: impl AsRef<str>, label: impl AsRef<str>) -> Result<KeyRecord, String> {
        self.update_metadata(
            id,
            MetadataUpdate {
                label: Some(label.as_ref().to_string()),
                ..MetadataUpdate::default()
            },
        )
    }

    pub fn remove(&self, id: impl AsRef<str>) -> Result<(), String> {
        let id = id.as_ref();
        let records = self
            .list()?
            .into_iter()
            .filter(|item| item.id != id)
            .collect::<Vec<_>>();
        let mut secrets = self.secrets()?;
        secrets.secrets.remove(id);
        self.save_secrets(&secrets)?;
        self.save_list(records)
    }

    pub fn secret(&self, id: impl AsRef<str>) -> Result<String, String> {
        let id = id.as_ref();
        let secrets = self.secrets()?;
        let payload = secrets
            .secrets
            .get(id)
            .ok_or_else(|| "Secret was not found.".to_string())?;
        self.reveal(payload)
    }

    /// Replace the stored secret for an existing key and refresh its masked
    /// representation. Used when editing a key's API key in place.
    pub fn set_secret(
        &self,
        id: impl AsRef<str>,
        api_key: impl AsRef<str>,
    ) -> Result<(), String> {
        let id = id.as_ref();
        let api_key = trim(api_key);
        if api_key.is_empty() {
            return Err("API key is required.".to_string());
        }
        let mut records = self.list()?;
        let record = records
            .iter_mut()
            .find(|item| item.id == id)
            .ok_or_else(|| "Key not found.".to_string())?;
        record.masked_key = mask_key(&api_key);
        record.updated_at = now_iso();

        let mut secrets = self.secrets()?;
        secrets.secrets.insert(id.to_string(), self.protect(&api_key));
        self.save_secrets(&secrets)?;
        self.save_list(records)
    }

    pub fn mark_validation(
        &self,
        id: impl AsRef<str>,
        result: &ValidationResult,
    ) -> Result<KeyRecord, String> {
        self.update_metadata(
            id,
            MetadataUpdate {
                available: Some(result.valid),
                status_code: Some(result.status_code),
                validation_message: Some(result.message.clone()),
                // Preserve previously stored models when this check did not fetch
                // a list (e.g. the real /responses probe only tests availability).
                models: if result.models.is_empty() {
                    None
                } else {
                    Some(result.models.clone())
                },
                model: if result.model.is_empty() {
                    None
                } else {
                    Some(result.model.clone())
                },
                last_validated_at: Some(now_iso()),
                ..MetadataUpdate::default()
            },
        )
    }

    /// Produce a portable, self-contained bundle of every stored key, including
    /// the plaintext API key so it can be restored on another machine.
    pub fn export_bundle(&self) -> Result<ExportBundle, String> {
        let records = self.list()?;
        let secrets = self.secrets()?;
        let mut items = Vec::new();
        for record in &records {
            // Skip keys whose secret can no longer be revealed rather than failing
            // the whole export.
            let api_key = match secrets.secrets.get(&record.id) {
                Some(payload) => match self.reveal(payload) {
                    Ok(secret) => secret,
                    Err(_) => continue,
                },
                None => continue,
            };
            items.push(ExportedKey {
                label: record.label.clone(),
                base_url: record.base_url.clone(),
                api_key,
                model: record.model.clone(),
                models: record.models.clone(),
            });
        }
        Ok(ExportBundle {
            kind: EXPORT_KIND.to_string(),
            version: 1,
            exported_at: now_iso(),
            keys: items,
        })
    }

    /// Import keys from a previously exported bundle. Entries whose API key plus
    /// base URL already exist are skipped. Returns how many were added and skipped.
    pub fn import_bundle(&self, bundle: ExportBundle) -> Result<ImportSummary, String> {
        if bundle.kind != EXPORT_KIND {
            return Err("Unrecognized file. This is not a Keydock export.".to_string());
        }

        let mut records = self.list()?;
        let mut secrets = self.secrets()?;

        // Build a set of existing (api_key, base_url) pairs to de-duplicate.
        let mut existing: std::collections::HashSet<(String, String)> = records
            .iter()
            .filter_map(|record| {
                secrets
                    .secrets
                    .get(&record.id)
                    .and_then(|payload| self.reveal(payload).ok())
                    .map(|secret| {
                        (
                            secret,
                            normalize_or_default(&record.base_url).unwrap_or_default(),
                        )
                    })
            })
            .collect();

        let mut added = 0usize;
        let mut skipped = 0usize;
        let timestamp = now_iso();

        for item in bundle.keys {
            let api_key = trim(&item.api_key);
            if api_key.is_empty() {
                skipped += 1;
                continue;
            }
            let base_url = normalize_or_default(&item.base_url)?;
            if !existing.insert((api_key.clone(), base_url.clone())) {
                skipped += 1;
                continue;
            }

            let models = unique_strings(item.models.clone());
            let selected_model = if !trim(&item.model).is_empty() {
                trim(&item.model)
            } else {
                models.first().cloned().unwrap_or_default()
            };
            let id = Uuid::new_v4().to_string();
            records.push(KeyRecord {
                id: id.clone(),
                label: {
                    let label = trim(&item.label);
                    if label.is_empty() {
                        "Imported key".to_string()
                    } else {
                        label
                    }
                },
                base_url,
                masked_key: mask_key(&api_key),
                model: selected_model,
                models,
                available: false,
                status_code: 0,
                validation_message: "Imported. Check the key to verify it.".to_string(),
                active: false,
                source: "imported".to_string(),
                last_validated_at: String::new(),
                created_at: timestamp.clone(),
                updated_at: timestamp.clone(),
            });
            secrets.secrets.insert(id, self.protect(&api_key));
            added += 1;
        }

        self.save_secrets(&secrets)?;
        self.save_list(records)?;
        Ok(ImportSummary { added, skipped })
    }

    pub fn mark_active(&self, id: impl AsRef<str>) -> Result<Vec<KeyRecord>, String> {
        let id = id.as_ref();
        let timestamp = now_iso();
        let mut records = self.list()?;
        for record in &mut records {
            record.active = record.id == id;
            record.updated_at = timestamp.clone();
            if record.active {
                record.last_validated_at = timestamp.clone();
            }
        }
        self.save_list(records.clone())?;
        Ok(records)
    }
}

pub fn default_data_dir() -> PathBuf {
    if let Ok(path) = std::env::var("CKM_STORE_DIR") {
        if !trim(&path).is_empty() {
            return PathBuf::from(path);
        }
    }
    dirs::data_dir()
        .or_else(dirs::home_dir)
        .unwrap_or_else(std::env::temp_dir)
        .join(APP_NAME)
}

pub fn now_iso() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
}

fn ensure_parent(path: &Path) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn read_json<T>(path: &Path) -> Result<T, String>
where
    T: for<'de> Deserialize<'de> + Default,
{
    match fs::read_to_string(path) {
        Ok(content) => serde_json::from_str(&content).map_err(|error| error.to_string()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(T::default()),
        Err(error) => Err(error.to_string()),
    }
}

fn write_json<T>(path: &Path, value: &T) -> Result<(), String>
where
    T: Serialize,
{
    ensure_parent(path)?;
    let content = serde_json::to_string_pretty(value).map_err(|error| error.to_string())?;
    fs::write(path, format!("{content}\n")).map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stores_secret_outside_metadata() {
        let dir = std::env::temp_dir().join(format!("keydock-store-{}", Uuid::new_v4()));
        let store = KeydockStore::new(&dir);
        let record = store
            .add(
                "Work",
                "https://api.example.com/v1",
                "sk-test-1234567890",
                &ValidationResult::ok("ok", vec!["gpt-a".to_string(), "gpt-b".to_string()]),
            )
            .unwrap();
        assert_eq!(store.secret(&record.id).unwrap(), "sk-test-1234567890");
        let metadata = fs::read_to_string(dir.join("keys.json")).unwrap();
        assert!(!metadata.contains("sk-test-1234567890"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn exports_and_reimports_with_dedup() {
        let src_dir = std::env::temp_dir().join(format!("keydock-export-{}", Uuid::new_v4()));
        let src = KeydockStore::new(&src_dir);
        src.add(
            "Work",
            "https://api.example.com/v1",
            "sk-aaa-1111111111",
            &ValidationResult::ok("ok", vec!["gpt-a".to_string()]),
        )
        .unwrap();
        src.add(
            "Home",
            "https://api.example.org/v1",
            "sk-bbb-2222222222",
            &ValidationResult::ok("ok", vec![]),
        )
        .unwrap();

        let bundle = src.export_bundle().unwrap();
        assert_eq!(bundle.kind, EXPORT_KIND);
        assert_eq!(bundle.keys.len(), 2);
        // Plaintext keys are present so they can be restored elsewhere.
        assert!(bundle.keys.iter().any(|k| k.api_key == "sk-aaa-1111111111"));

        // Round-trip through JSON like the command layer does.
        let json = serde_json::to_string(&bundle).unwrap();
        let parsed: ExportBundle = serde_json::from_str(&json).unwrap();

        let dst_dir = std::env::temp_dir().join(format!("keydock-import-{}", Uuid::new_v4()));
        let dst = KeydockStore::new(&dst_dir);
        let first = dst.import_bundle(parsed.clone()).unwrap();
        assert_eq!(first.added, 2);
        assert_eq!(first.skipped, 0);
        // Importing the same bundle again adds nothing (deduplicated).
        let second = dst.import_bundle(parsed).unwrap();
        assert_eq!(second.added, 0);
        assert_eq!(second.skipped, 2);

        let restored = dst.list().unwrap();
        let work = restored.iter().find(|r| r.label == "Work").unwrap();
        assert_eq!(dst.secret(&work.id).unwrap(), "sk-aaa-1111111111");

        let _ = fs::remove_dir_all(src_dir);
        let _ = fs::remove_dir_all(dst_dir);
    }

    #[test]
    fn rejects_foreign_import_payload() {
        let dir = std::env::temp_dir().join(format!("keydock-foreign-{}", Uuid::new_v4()));
        let store = KeydockStore::new(&dir);
        let bundle = ExportBundle {
            kind: "something-else".to_string(),
            version: 1,
            exported_at: String::new(),
            keys: vec![],
        };
        assert!(store.import_bundle(bundle).is_err());
        let _ = fs::remove_dir_all(dir);
    }
}
