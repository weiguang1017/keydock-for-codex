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
