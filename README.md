# Keydock for Codex

Keydock for Codex is a lightweight desktop app for storing, checking, and switching Codex API keys.

## Why this name

Keydock is a small dock for API keys: park multiple Codex keys, check them, and switch the active one without touching Codex internals.

## Native macOS build

```sh
make app
```

The native macOS app bundle is created at `build/Keydock for Codex.app`.

## Run

```sh
make run
```

## Test

```sh
make test
```

## Cross-platform Electron build

```sh
npm install
npm test
npm run dist
```

GitHub Actions builds release artifacts on macOS, Windows, and Linux whenever code is pushed to `main`.

## Notes

- Native macOS secrets are stored in macOS Keychain under the `KeydockForCodex` service.
- Native macOS metadata is stored in `~/Library/Application Support/Keydock for Codex/keys.json`.
- The cross-platform Electron app uses Electron `safeStorage` when available.
- Switching uses `codex login --with-api-key` with the key passed through stdin.
- After switching, Codex Desktop is restarted. Existing terminal Codex sessions need to be reopened.
