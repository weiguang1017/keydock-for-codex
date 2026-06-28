# Keydock for Codex

[中文](#中文) | [English](#english)

## 中文

Keydock for Codex 是一款基于 Rust/Tauri 的桌面端 Codex API Key 管理器，面向需要在多个账号、项目或额度之间切换的 Codex 用户。

### 它解决什么问题

- 不再手动复制、粘贴、覆盖 Codex Key。
- 新增和手动检查时验证 Key 是否有效；切换时专注写入本地配置，避免实时网络探测失败阻断切换。
- 切换后写入 `~/.codex/config.toml` 和 `~/.codex/auth.json`，让 Codex 使用新的 Base URL、模型和 API Key。
- 多个 Key 可保存、命名、查看状态，并安全隐藏完整 Key。

### 主要功能

- 管理多个 Codex API Key。
- 支持英文、中文、日文界面，会自动匹配系统语言，也可以手动切换。
- 新增 Key 时先校验，失败不保存。
- 新增 Key 需要填写名称、Base URL、API Key，并在校验后显示平台返回的模型列表和可用状态。
- 一键切换当前 Key，写入本地配置并尽量重启 Codex Desktop；如果自动重启失败，界面会提示手动重启。
- Rust 端负责配置读写、Key 校验、本地存储和 Tauri 命令；Web UI 作为 Tauri 静态前端打包。

### 环境要求

- Rust stable toolchain。
- Tauri 构建环境：
  - macOS：Xcode Command Line Tools。
  - Windows：Microsoft C++ Build Tools 和 WebView2 Runtime。
  - Linux：WebKitGTK、GTK、AppIndicator、librsvg、patchelf 等 Tauri 桌面依赖。

### 本地开发

运行测试：

```sh
cd src-tauri
cargo test
```

运行桌面应用：

```sh
cd src-tauri
cargo run
```

只编译 release 可执行文件：

```sh
cd src-tauri
cargo build --release
```

### 生成桌面安装包

首次需要安装 Tauri CLI：

```sh
cargo install tauri-cli --locked
```

在仓库根目录生成桌面应用包。macOS 下已验证的 `.app` 生成命令：

```sh
cargo tauri build --bundles app
```

构建产物会输出到：

```text
src-tauri/target/release/bundle/
```

如果需要生成平台安装器，可以使用 Tauri 的其它 bundle 目标，例如 `cargo tauri build --bundles dmg`、`cargo tauri build --bundles msi`、`cargo tauri build --bundles deb`。具体可用格式取决于当前平台、系统依赖和 Tauri bundler 支持。

### 自动构建

推送到 `main` 或提交 Pull Request 后，GitHub Actions 会在 macOS、Windows、Linux 上执行：

- `cargo test`
- `cargo build --release`
- `cargo tauri build --bundles app`（macOS）或对应平台的 Tauri bundle 目标

构建结果在 GitHub 仓库的 `Actions` 页面下载。打 tag（例如 `v1.2.0`）会触发 release workflow，并把 Tauri bundle 产物上传到 GitHub Release。

### macOS 首次打开（未签名应用）

项目没有 Apple 付费开发者账号，发布的 macOS `.dmg` 未做签名和公证。从网络下载的应用会被系统打上「隔离（quarantine）」标记，首次打开被 Gatekeeper 拦截（提示"已损坏"或"无法验证开发者"）。**这是 Apple 对未公证应用的限制，与应用本身无关，应用是安全的。**

> 注意：macOS 15 (Sequoia) 起，对带隔离标记的未公证应用，常常**直接报"已损坏"且不再提供「仍要打开」按钮**，「右键 → 打开」的旧绕过方式也已失效。此时只能用下面的方法解除隔离。

**方法一（推荐，一行命令）**：先把 App 拖入「应用程序」，再在终端执行一次：

```sh
xattr -dr com.apple.quarantine "/Applications/Keydock for Codex.app"
```

之后即可正常双击打开。

**方法二（一键脚本）**：下载仓库里的 [`scripts/macos-open.command`](scripts/macos-open.command)，双击运行——它会自动定位 App、解除隔离并打开。若双击脚本本身也被拦截，在终端执行 `bash 路径/macos-open.command` 即可。

**方法三（图形界面）**：双击 App 被拦截后，打开「系统设置 → 隐私与安全性」，下滑找到对应提示点「仍要打开」（仅在系统提供该按钮时可用）。

> 想彻底免提示打开，需要用 Apple Developer ID 证书签名并公证（付费开发者账号 $99/年），届时可在 release workflow 中接入自动签名+公证。

## English

Keydock for Codex is a Rust/Tauri desktop API key manager for Codex. It is built for users who switch between multiple accounts, projects, or quota pools.

### What It Solves

- No more copying, pasting, or overwriting Codex keys by hand.
- Keys are validated when added or checked manually; switching focuses on writing local config so transient probe failures do not block it.
- Switching updates `~/.codex/config.toml` and `~/.codex/auth.json` so Codex uses the selected Base URL, model, and API key.
- Multiple keys can be saved, named, checked, and shown safely with masking.

### Features

- Manage multiple Codex API keys.
- English, Chinese, and Japanese UI text with automatic system-language matching and a manual selector.
- Validate a key before saving it.
- Adding a key requires name, Base URL, and API key, then shows platform-provided models and availability status after validation.
- Switch the active key, write the local config, and try to restart Codex Desktop; if automatic restart fails, the UI asks you to restart Codex manually.
- Rust owns config IO, key validation, local storage, and Tauri commands; the Web UI is packaged as the Tauri static frontend.

### Requirements

- Rust stable toolchain.
- Tauri desktop build dependencies:
  - macOS: Xcode Command Line Tools.
  - Windows: Microsoft C++ Build Tools and WebView2 Runtime.
  - Linux: WebKitGTK, GTK, AppIndicator, librsvg, patchelf, and related Tauri desktop dependencies.

### Local Development

Run tests:

```sh
cd src-tauri
cargo test
```

Run the desktop app:

```sh
cd src-tauri
cargo run
```

Compile the release executable:

```sh
cd src-tauri
cargo build --release
```

### Build Desktop Packages

Install the Tauri CLI once:

```sh
cargo install tauri-cli --locked
```

From the repository root, generate the desktop app bundle. The verified macOS `.app` command is:

```sh
cargo tauri build --bundles app
```

Build outputs are written to:

```text
src-tauri/target/release/bundle/
```

To generate platform installers, use other Tauri bundle targets such as `cargo tauri build --bundles dmg`, `cargo tauri build --bundles msi`, or `cargo tauri build --bundles deb`. Available formats depend on the current platform, system dependencies, and Tauri bundler support.

### CI Builds

Pushes to `main` and pull requests run GitHub Actions on macOS, Windows, and Linux:

- `cargo test`
- `cargo build --release`
- `cargo tauri build --bundles app` on macOS, or the matching Tauri bundle target for each platform

Artifacts are available from the repository's `Actions` page. Tags such as `v1.2.0` trigger the release workflow and upload Tauri bundle outputs to GitHub Release.

### Opening on macOS (Unsigned App)

This project has no paid Apple Developer account, so the published macOS `.dmg` is not signed or notarized. On first launch macOS Gatekeeper blocks it ("app is damaged" or "cannot verify developer"). This is an Apple restriction and cannot be fixed through build configuration. After dragging the app into `Applications`, run this once in a terminal:

```sh
xattr -dr com.apple.quarantine "/Applications/Keydock for Codex.app"
```

The app then opens normally on double-click. Alternatively, go to System Settings → Privacy & Security and click "Open Anyway" on the blocked prompt.
