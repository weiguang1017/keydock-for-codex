import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { app, BrowserWindow, ipcMain, safeStorage } from 'electron';
import {
  APP_NAME,
  KeydockStore,
  findCodexPath,
  loginWithCodex,
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

ipcMain.handle('keys:add', async (_event, { label, apiKey }) => {
  const check = await validateKey(apiKey);
  if (!check.valid) throw new Error(check.message);
  return store.add(label, apiKey);
});

ipcMain.handle('keys:updateName', async (_event, { id, label }) => store.updateName(id, label));

ipcMain.handle('keys:delete', async (_event, { id }) => {
  store.remove(id);
  return true;
});

ipcMain.handle('keys:validate', async (_event, { id }) => {
  const apiKey = store.secret(id);
  const check = await validateKey(apiKey);
  if (check.valid) store.markValidated(id);
  return check;
});

ipcMain.handle('keys:switch', async (_event, { id }) => {
  const apiKey = store.secret(id);
  const check = await validateKey(apiKey);
  if (!check.valid) throw new Error(check.message);
  const codexPath = await findCodexPath();
  const status = await loginWithCodex(apiKey, codexPath);
  await restartCodexDesktop(codexPath);
  store.markActive(id);
  return { status };
});

ipcMain.handle('app:diagnostics', async () => {
  try {
    return {
      codexPath: await findCodexPath(),
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
