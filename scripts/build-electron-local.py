#!/usr/bin/env python3
import json
import plistlib
import shutil
import subprocess
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
SOURCE_APP = Path("/Applications/Codex.app")
OUTPUT_APP = ROOT / "build" / "Keydock Electron.app"


def run(args):
    subprocess.run(args, check=True)


def build():
    if OUTPUT_APP.exists():
        shutil.rmtree(OUTPUT_APP)
    shutil.copytree(SOURCE_APP, OUTPUT_APP, symlinks=True)

    contents = OUTPUT_APP / "Contents"
    resources = contents / "Resources"
    macos = contents / "MacOS"

    app_dir = resources / "app"
    if app_dir.exists():
        shutil.rmtree(app_dir)
    app_dir.mkdir(parents=True, exist_ok=True)

    package_json = {
        "name": "keydock-for-codex-local",
        "productName": "Keydock for Codex",
        "version": "1.0.1",
        "main": "main.cjs"
    }
    (app_dir / "package.json").write_text(json.dumps(package_json, indent=2) + "\n", encoding="utf-8")

    main_cjs = f"""
const path = require('node:path');
const {{ app, BrowserWindow, ipcMain, safeStorage }} = require('electron');

(async () => {{
  const workspace = {json.dumps(str(ROOT))};
  const mod = await import(path.join(workspace, 'electron', 'keydock-core.mjs'));
  const preloadPath = path.join(workspace, 'electron', 'preload.mjs');
  const htmlPath = path.join(workspace, 'electron', 'renderer', 'index.html');
  let store;

  function createWindow() {{
    const window = new BrowserWindow({{
      width: 1180,
      height: 780,
      minWidth: 960,
      minHeight: 640,
      title: mod.APP_NAME,
      backgroundColor: '#090c12',
      webPreferences: {{
        preload: preloadPath,
        contextIsolation: true,
        nodeIntegration: false
      }}
    }});
    window.loadFile(htmlPath);
  }}

  function serializeError(error) {{
    return {{ message: error?.message || 'Unknown error' }};
  }}

  ipcMain.handle('keys:list', async () => store.list());
  ipcMain.handle('keys:add', async (_event, {{ label, baseUrl, apiKey }}) => {{
    const check = await mod.validateKey(apiKey, {{ baseUrl }});
    if (!check.valid) throw new Error(check.message);
    return store.add(label, baseUrl, apiKey, check);
  }});
  ipcMain.handle('keys:updateName', async (_event, {{ id, label }}) => store.updateName(id, label));
  ipcMain.handle('keys:updateMetadata', async (_event, {{ id, label, baseUrl, model }}) =>
    store.updateMetadata(id, {{ label, baseUrl, model }})
  );
  ipcMain.handle('keys:delete', async (_event, {{ id }}) => {{
    store.remove(id);
    return true;
  }});
  ipcMain.handle('keys:validate', async (_event, {{ id }}) => {{
    const apiKey = store.secret(id);
    const record = store.list().find((item) => item.id === id);
    const check = await mod.validateKey(apiKey, {{ baseUrl: record?.baseUrl }});
    store.markValidation(id, check);
    return check;
  }});
  ipcMain.handle('keys:switch', async (_event, {{ id }}) => {{
    const apiKey = store.secret(id);
    const record = store.list().find((item) => item.id === id);
    const check = await mod.validateKey(apiKey, {{ baseUrl: record?.baseUrl }});
    if (!check.valid) throw new Error(check.message);
    store.markValidation(id, check);
    const codexPath = await mod.findCodexPath();
    const status = await mod.loginWithCodex(apiKey, codexPath);
    await mod.restartCodexDesktop(codexPath);
    store.markActive(id);
    return {{ status }};
  }});
  ipcMain.handle('app:diagnostics', async () => {{
    try {{
      const codexPath = await mod.findCodexPath();
      let currentKey = null;
      try {{
        const status = await mod.readCodexLogin(codexPath);
        const maskedKey = mod.extractMaskedKeyFromStatus(status);
        if (maskedKey) currentKey = {{ status, maskedKey }};
      }} catch {{
        currentKey = null;
      }}
      return {{
        codexPath,
        currentKey,
        encryption: safeStorage.isEncryptionAvailable() ? 'OS secure storage' : 'local fallback'
      }};
    }} catch (error) {{
      return {{
        ...serializeError(error),
        encryption: safeStorage.isEncryptionAvailable() ? 'OS secure storage' : 'local fallback'
      }};
    }}
  }});

  app.whenReady().then(() => {{
    app.setName(mod.APP_NAME);
    store = new mod.KeydockStore(app.getPath('userData'), safeStorage);
    createWindow();
    app.on('activate', () => {{
      if (BrowserWindow.getAllWindows().length === 0) createWindow();
    }});
  }});

  app.on('window-all-closed', () => {{
    if (process.platform !== 'darwin') app.quit();
  }});
}})().catch((error) => {{
  console.error(error);
  app.quit();
}});
"""
    (app_dir / "main.cjs").write_text(main_cjs, encoding="utf-8")

    asar_path = resources / "app.asar"
    if asar_path.exists():
      asar_path.unlink()

    info_path = contents / "Info.plist"
    with info_path.open("rb") as file:
        info = plistlib.load(file)
    info["CFBundleDisplayName"] = "Keydock for Codex"
    info["CFBundleName"] = "Keydock for Codex"
    info["CFBundleIdentifier"] = "local.keydock-for-codex.electron"
    info["CFBundleShortVersionString"] = "1.0.1"
    info["CFBundleVersion"] = "1"
    info["CFBundleIconFile"] = "icon"
    info.pop("ElectronAsarIntegrity", None)
    with info_path.open("wb") as file:
        plistlib.dump(info, file)

    shutil.copyfile(ROOT / "electron" / "assets" / "icon.icns", resources / "electron.icns")
    shutil.copyfile(ROOT / "electron" / "assets" / "icon.icns", resources / "icon.icns")

    run(["/usr/bin/codesign", "--force", "--deep", "--sign", "-", str(OUTPUT_APP)])
    print(OUTPUT_APP)


if __name__ == "__main__":
    build()
