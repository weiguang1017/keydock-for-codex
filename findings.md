# Findings

## 2026-06-09
- Repository is an Electron-style client with renderer files under `electron/renderer`.
- Need inspect current key data shape before changing persistence.
- Electron core previously validated only fixed OpenAI `/v1/models` and required `sk-` prefix.
- v1.0.1 needs platform-compatible validation from user-provided base URL and model list parsing.
- Current Electron diagnostics only run Codex CLI lookup/login status; they do not read `~/.codex/config.toml` or `~/.codex/auth.json`.
- Local Codex config shape has top-level `model_provider` and `model`, a `[model_providers.<name>]` table with `base_url`, and `auth.json` stores `OPENAI_API_KEY`.
- The add-key dialog's "Validate & load models" path currently calls `keys:add`, so validation immediately persists the key; the Add button repeats that same operation.

## 2026-06-10
- Started investigation of `src-tauri` `cargo test` failures after project refactor.
- Existing planning files describe the earlier Electron/native v1.0.1 task; the current Rust/Tauri failure is tracked as a new phase.
- The pasted `cargo test` log shows missing Rust modules: `cli`, `codex`, `commands`, `store`, and `validate`.
- `src-tauri/src` currently contains only `lib.rs` and `main.rs`; no matching module files exist in git history for `src-tauri`.
- Tauri macro expansion also fails because `src-tauri/icons/32x32.png` and related icon assets are absent.
- Added Rust modules mirroring the Electron core responsibilities: validation, store, Codex config/profile handling, CLI helpers, and Tauri commands.
- Reused existing app icon assets under `src-tauri/icons` to satisfy `tauri::generate_context!()`.
- Current README and CI still document/build Native macOS plus Electron, which conflicts with the Rust/Tauri refactor.
- The old Electron renderer can be reused as the Tauri static frontend; Electron main/preload/core, Node packaging, Objective-C source, old tests, and old resource bundle are no longer needed.
- `cargo tauri build` reaches `.app` generation locally but exits during DMG packaging; `cargo tauri build --bundles app` succeeds and produces `src-tauri/target/release/bundle/macos/Keydock for Codex.app`.
