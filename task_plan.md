# v1.0.1 Client Optimization Plan

## Goal
Implement v1.0.1 optimizations for the client:
- Multi-language support for English, Chinese, and Japanese.
- Auto-match the system language and provide a manual language selector.
- Improve the key entry UI with required name, base URL, and API key fields.
- Add optional model list and availability status fields.

## Phases
| Phase | Status | Notes |
|---|---|---|
| 1. Inspect current app structure | complete | Identified Electron renderer, main IPC, core store, and native macOS app. |
| 2. Design i18n and form model changes | complete | Electron path will own v1.0.1 UI; core store now carries base URL, models, and availability. |
| 3. Implement UI and data updates | complete | Renderer, main IPC, preload, styles, README, version, and Electron tests updated. |
| 4. Verify behavior | complete | Electron and native tests passed; browser preview was blocked by local browser policy. |
| 5. Redesign app icon | complete | Replaced orbit concept with Codex-derived blue command cloud plus Keydock key mark. |
| 6. Optimize UI for programmers | complete | Added search, copy actions, compact status strip, dark tool UI, and keyboard-friendly operations. |
| 7. Fix Codex config detection and add-key flow | complete | Reads `~/.codex` config/auth, guides unconfigured users, and makes Test load models before Add saves. |
| 8. Fix src-tauri cargo test after refactor | complete | Added the missing Rust modules and Tauri icons; `cargo test` passes from `src-tauri`. |
| 9. Remove old components after Rust refactor | complete | Kept the Tauri/Rust app, migrated reusable Web UI, removed Electron/Objective-C leftovers, and updated README/CI build docs. |

## Decisions
- Treat user-facing text as localizable.
- Keep planning files in project root for this task.
- Default base URL is `https://api.openai.com/v1`; user can enter compatible platform endpoints.
- New icon should reference Codex's soft blue command-symbol language while staying distinct for Keydock.
- Programmer-focused UI should prioritize scanning, keyboard flow, copy actions, and compact diagnostics.
- Rust/Tauri is now the canonical app path; old Electron and Objective-C build paths should be removed instead of documented.

## Errors Encountered
| Error | Attempt | Resolution |
|---|---|---|
| In-app browser blocked `file://` preview | 1 | Switched to local HTTP preview attempt. |
| Local HTTP preview blocked by environment/client policy | 1 | Relied on automated tests and code-level inspection; no real key or network operation was needed. |
| Preview service process could not be terminated by tool sandbox | 2 | Report remaining PID/port to user. |
| `cargo fmt` unavailable because `rustfmt` is not installed for `stable-x86_64-apple-darwin` | 1 | Continued with compiler/test validation; no formatting command could be run in this local toolchain. |
| `cargo tauri build` failed during DMG packaging on local macOS environment | 1 | Verified app generation with `cargo tauri build --bundles app` and documented app bundle generation plus optional installer targets. |
