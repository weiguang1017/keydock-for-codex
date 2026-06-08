# Keydock for Codex

[中文](#中文) | [English](#english)

## 中文

Keydock for Codex 是一款桌面端 Codex API Key 管理器，解决多账号、多项目、多额度场景下频繁手动切换 Key 的麻烦。

### 它解决什么问题

- 不再手动复制、粘贴、覆盖 Codex Key。
- 切换前自动检查 Key 是否有效，避免切到不可用 Key。
- 切换后自动重启 Codex Desktop，让新 Key 立即生效。
- 多个 Key 可保存、命名、查看状态，并安全隐藏完整 Key。

### 主要功能

- 管理多个 Codex API Key。
- 新增 Key 时先校验，失败不保存。
- 一键切换当前 Key，并通过 `codex login --with-api-key` 生效。
- macOS 原生版使用 Keychain；跨平台 Electron 版使用系统安全存储能力。
- GitHub Actions 自动构建 macOS、Windows、Linux 版本。

### 构建

macOS 原生版：

```sh
make app
make run
make test
```

跨平台 Electron 版：

```sh
npm install
npm test
npm run pack
```

### 自动构建

推送到 `main` 后，GitHub Actions 会自动构建：

- Native macOS app
- Electron macOS app
- Electron Windows app
- Electron Linux app

构建结果在 GitHub 仓库的 `Actions` 页面下载。

## English

Keydock for Codex is a desktop API key manager for Codex. It helps users who work with multiple accounts, projects, or quotas switch keys without manually editing Codex authentication state.

### What it solves

- No more copying, pasting, or overwriting Codex keys by hand.
- Keys are checked before switching, so broken keys are blocked early.
- Codex Desktop restarts after switching, so the new key takes effect immediately.
- Multiple keys can be saved, named, checked, and shown safely with masking.

### Features

- Manage multiple Codex API keys.
- Validate a key before saving it.
- Switch the active key via `codex login --with-api-key`.
- Native macOS version stores secrets in Keychain; Electron version uses OS-backed secure storage when available.
- GitHub Actions builds macOS, Windows, and Linux desktop artifacts automatically.

### Build

Native macOS:

```sh
make app
make run
make test
```

Cross-platform Electron:

```sh
npm install
npm test
npm run pack
```

### CI Builds

Every push to `main` triggers GitHub Actions builds for:

- Native macOS app
- Electron macOS app
- Electron Windows app
- Electron Linux app

Download artifacts from the repository's `Actions` page.
