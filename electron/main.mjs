import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { app, BrowserWindow, ipcMain, safeStorage } from 'electron';
import {
  APP_NAME,
  applyCodexProfile,
  extractMaskedKeyFromStatus,
  KeydockStore,
  findCodexPath,
  publicCodexProfile,
  readCodexProfile,
  readCodexLogin,
  restartCodexDesktop,
  validateKey
} from './keydock-core.mjs';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
let store;

function createWindow() {
  const window = new BrowserWindow({
    width: 980,
    height: 640,
    minWidth: 820,
    minHeight: 540,
    title: APP_NAME,
    backgroundColor: '#eef2ef',
    webPreferences: {
      preload: path.join(__dirname, 'preload.mjs'),
      contextIsolation: true,
      nodeIntegration: false
    }
  });
  window.loadFile(path.join(__dirname, 'renderer', 'index.html'));
}

function serializeError(error) {
  return { message: error?.message || 'Unknown error' };
}

async function syncCodexProfile() {
  const profile = readCodexProfile();
  if (profile.configured) {
    store.upsertCodexProfile(profile);
  }
  return profile;
}

ipcMain.handle('keys:list', async () => {
  try {
    await syncCodexProfile();
  } catch {
    // Listing locally saved keys should still work if Codex config is unreadable.
  }
  return store.list();
});

ipcMain.handle('keys:testDraft', async (_event, { baseUrl, apiKey }) => validateKey(apiKey, { baseUrl }));

ipcMain.handle('keys:add', async (_event, { label, baseUrl, apiKey, model, validation }) => {
  const check = validation?.valid === true
    ? {
        valid: true,
        statusCode: Number.isFinite(validation.statusCode) ? validation.statusCode : 200,
        message: validation.message || 'The platform accepted this key.',
        models: Array.isArray(validation.models) ? validation.models : []
      }
    : await validateKey(apiKey, { baseUrl });
  if (!check.valid) throw new Error(check.message);
  if (model) check.model = model;
  return store.add(label, baseUrl, apiKey, check);
});

ipcMain.handle('keys:updateName', async (_event, { id, label }) => store.updateName(id, label));

ipcMain.handle('keys:updateMetadata', async (_event, { id, label, baseUrl, model }) => {
  const record = store.updateMetadata(id, { label, baseUrl, model });
  return record;
});

ipcMain.handle('keys:delete', async (_event, { id }) => {
  store.remove(id);
  return true;
});

ipcMain.handle('keys:validate', async (_event, { id }) => {
  const apiKey = store.secret(id);
  const record = store.list().find((item) => item.id === id);
  const check = await validateKey(apiKey, { baseUrl: record?.baseUrl });
  store.markValidation(id, check);
  return check;
});

ipcMain.handle('keys:switch', async (_event, { id }) => {
  const apiKey = store.secret(id);
  const record = store.list().find((item) => item.id === id);
  const check = await validateKey(apiKey, { baseUrl: record?.baseUrl });
  if (!check.valid) throw new Error(check.message);
  store.markValidation(id, check);
  // Switch by writing base_url + model into config.toml and the key into auth.json.
  applyCodexProfile({
    baseUrl: record?.baseUrl,
    apiKey,
    model: record?.model
  });
  let restarted = false;
  let warning = '';
  try {
    const codexPath = await findCodexPath().catch(() => null);
    await restartCodexDesktop(codexPath);
    restarted = true;
  } catch (error) {
    warning = error?.message || 'Codex restart failed; please restart Codex manually.';
  }
  store.markActive(id);
  return { restarted, warning, baseUrl: record?.baseUrl, model: record?.model };
});

ipcMain.handle('app:diagnostics', async () => {
  const codexProfile = syncCodexProfile()
    .catch((error) => ({ configured: false, message: error.message }));
  try {
    const profile = await codexProfile;
    const codexPath = await findCodexPath();
    let currentKey = null;
    try {
      const status = await readCodexLogin(codexPath);
      const maskedKey = extractMaskedKeyFromStatus(status);
      if (maskedKey) {
        currentKey = {
          status,
          maskedKey
        };
      }
    } catch {
      currentKey = null;
    }
    return {
      codexPath,
      currentKey,
      codexProfile: publicCodexProfile(profile),
      encryption: safeStorage.isEncryptionAvailable() ? 'OS secure storage' : 'local fallback'
    };
  } catch (error) {
    const profile = await codexProfile;
    return {
      ...serializeError(error),
      codexProfile: publicCodexProfile(profile),
      encryption: safeStorage.isEncryptionAvailable() ? 'OS secure storage' : 'local fallback'
    };
  }
});

app.whenReady().then(() => {
  app.setName(APP_NAME);
  store = new KeydockStore(app.getPath('userData'), safeStorage);
  createWindow();
  app.on('activate', () => {
    if (BrowserWindow.getAllWindows().length === 0) createWindow();
  });
});

app.on('window-all-closed', () => {
  if (process.platform !== 'darwin') app.quit();
});
