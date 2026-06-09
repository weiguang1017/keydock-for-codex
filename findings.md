# Findings

## 2026-06-09
- Repository is an Electron-style client with renderer files under `electron/renderer`.
- Need inspect current key data shape before changing persistence.
- Electron core previously validated only fixed OpenAI `/v1/models` and required `sk-` prefix.
- v1.0.1 needs platform-compatible validation from user-provided base URL and model list parsing.
- Current Electron diagnostics only run Codex CLI lookup/login status; they do not read `~/.codex/config.toml` or `~/.codex/auth.json`.
- Local Codex config shape has top-level `model_provider` and `model`, a `[model_providers.<name>]` table with `base_url`, and `auth.json` stores `OPENAI_API_KEY`.
- The add-key dialog's "Validate & load models" path currently calls `keys:add`, so validation immediately persists the key; the Add button repeats that same operation.
