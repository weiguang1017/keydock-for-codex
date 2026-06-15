#!/bin/bash
# Keydock for Codex —— macOS 一键打开助手
#
# 用途：未签名/未公证的应用从网络下载后，会被 macOS Gatekeeper 拦截
# （提示“已损坏”或“无法验证开发者”）。本脚本移除隔离标记后即可正常双击打开。
#
# 用法：双击本文件，或在终端执行 `bash macos-open.command`。

set -euo pipefail

APP_NAME="Keydock for Codex.app"

# 依次在常见位置查找已安装的 App。
CANDIDATES=(
  "/Applications/${APP_NAME}"
  "${HOME}/Applications/${APP_NAME}"
  "${HOME}/Downloads/${APP_NAME}"
  "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/${APP_NAME}"
)

APP_PATH=""
for path in "${CANDIDATES[@]}"; do
  if [ -d "$path" ]; then
    APP_PATH="$path"
    break
  fi
done

if [ -z "$APP_PATH" ]; then
  echo "未找到「${APP_NAME}」。"
  echo "请先把它拖入「应用程序」文件夹，再运行本脚本。"
  exit 1
fi

echo "正在解除隔离：$APP_PATH"
xattr -dr com.apple.quarantine "$APP_PATH" || true

echo "完成，正在打开应用…"
open "$APP_PATH"
echo "如果仍未打开，请到「系统设置 → 隐私与安全性」中点「仍要打开」。"
