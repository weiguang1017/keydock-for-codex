mod cli;
mod codex;
mod commands;
mod store;
mod validate;

pub const APP_NAME: &str = "Keydock for Codex";
pub const DEFAULT_BASE_URL: &str = "https://api.openai.com/v1";
pub const DEFAULT_CODEX_PROFILE_LABEL: &str = "Codex CLI";

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            commands::list_keys,
            commands::test_draft_key,
            commands::add_key,
            commands::update_name,
            commands::export_keys,
            commands::import_keys,
            commands::update_metadata,
            commands::delete_key,
            commands::validate_key_cmd,
            commands::switch_key,
            commands::diagnostics,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
