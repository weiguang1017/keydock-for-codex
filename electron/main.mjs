import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { app, BrowserWindow, ipcMain, safeStorage } from 'electron';
import {
  APP_NAME,
  extractMaskedKeyFromStatus,
  KeydockStore,
  findCodexPath,
  loginWithCodex,
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

ipcMain.handle('keys:list', async () => store.list());

ipcMain.handle('keys:add', async (_event, { label, baseUrl, apiKey }) => {
  const check = await validateKey(apiKey, { baseUrl });
  if (!check.valid) throw new Error(check.message);
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
  const codexPath = await findCodexPath();
  const status = await loginWithCodex(apiKey, codexPath);
  await restartCodexDesktop(codexPath);
  store.markActive(id);
  return { status };
});

ipcMain.handle('app:diagnostics', async () => {
  try {
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
      encryption: safeStorage.isEncryptionAvailable() ? 'OS secure storage' : 'local fallback'
    };
  } catch (error) {
    return {
      ...serializeError(error),
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
