# Findings

## 2026-06-09
- Repository is an Electron-style client with renderer files under `electron/renderer`.
- Need inspect current key data shape before changing persistence.
- Electron core previously validated only fixed OpenAI `/v1/models` and required `sk-` prefix.
- v1.0.1 needs platform-compatible validation from user-provided base URL and model list parsing.
