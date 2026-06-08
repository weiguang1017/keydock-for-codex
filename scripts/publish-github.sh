#!/usr/bin/env bash
set -euo pipefail

REPO_NAME="${1:-keydock-for-codex}"
DESCRIPTION="Keydock for Codex: desktop API key manager and switcher for Codex."

export https_proxy="${https_proxy:-http://127.0.0.1:7897}"
export http_proxy="${http_proxy:-http://127.0.0.1:7897}"
export all_proxy="${all_proxy:-socks5://127.0.0.1:7897}"

if ! command -v gh >/dev/null 2>&1; then
  echo "gh is required. Install GitHub CLI first." >&2
  exit 1
fi

if ! gh auth status >/dev/null 2>&1; then
  echo "GitHub CLI is not authenticated. Run: gh auth login -h github.com" >&2
  exit 1
fi

if [ ! -d .git ]; then
  git init -b main
fi

git add .
if git diff --cached --quiet; then
  echo "No staged changes to commit."
else
  git commit -m "Initial Keydock for Codex app"
fi

if gh repo view "$REPO_NAME" >/dev/null 2>&1; then
  echo "Repository $REPO_NAME already exists."
else
  gh repo create "$REPO_NAME" --public --description "$DESCRIPTION" --source=. --remote=origin --push
fi

if ! git remote get-url origin >/dev/null 2>&1; then
  OWNER="$(gh api user --jq .login)"
  git remote add origin "https://github.com/${OWNER}/${REPO_NAME}.git"
fi

git push -u origin main
gh repo view "$REPO_NAME" --web
