# Progress

## 2026-06-09
- Started v1.0.1 task plan.
- Confirmed repository files and clean git status.
- Updated Electron core to save base URL, models, selected model, availability, and validation message.
- Added renderer i18n for English, Chinese, Japanese with auto/manual language selection.
- Updated add/detail UI with required name, base URL, API key, model selector, status, and model chips.
- Updated package version to 1.0.1 and README feature notes.
- Updated native macOS app plist version to 1.0.1.
- Ran `npm test` and `make test`; both passed.
- Attempted browser preview, but local preview was blocked by browser/environment policy.
- Started icon redesign away from the current orbit icon, using the local Codex.app icon as visual reference.
- User clarified that the desktop app UI should suit programmer usage habits.
- Replaced generated icon assets with a Codex-inspired blue command cloud plus white key mark.
- Optimized Electron renderer for developer workflows: dark compact layout, key search, copy Base URL/masked key/models, compact summaries, and keyboard shortcuts.
- Ran `npm test`, `node --check`, Python compile check, and `make app`; all passed.
- Started fix for Codex config auto-detection and add-key test/add flow.
- Implemented Electron Codex config import from `~/.codex/config.toml`, `auth.json`, and optional `models_cache.json`.
- Changed add-key dialog so "测试/Test" validates without saving, loads returned models, and enables Add only after a valid test.
- Added empty-detail guidance with direct add buttons when no system or saved key is available.
- Added `CKM_CODEX_PATH` support to the native Codex finder for stable tests.
- Ran `npm test`, `make test`, `node --check` on Electron scripts, and `make app`; all passed.
