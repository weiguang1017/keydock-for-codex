// CommonJS preload: a sandboxed renderer can only load a CommonJS preload, so
// this must stay `.cjs` even though the rest of the app is ESM. It bridges the
// renderer to the main process over a fixed, allow-listed set of IPC channels.
const { contextBridge, ipcRenderer } = require('electron');

contextBridge.exposeInMainWorld('keydock', {
  listKeys: () => ipcRenderer.invoke('keys:list'),
  testDraftKey: (payload) => ipcRenderer.invoke('keys:testDraft', payload),
  addKey: (payload) => ipcRenderer.invoke('keys:add', payload),
  updateName: (payload) => ipcRenderer.invoke('keys:updateName', payload),
  updateMetadata: (payload) => ipcRenderer.invoke('keys:updateMetadata', payload),
  deleteKey: (payload) => ipcRenderer.invoke('keys:delete', payload),
  validateKey: (payload) => ipcRenderer.invoke('keys:validate', payload),
  switchKey: (payload) => ipcRenderer.invoke('keys:switch', payload),
  diagnostics: () => ipcRenderer.invoke('app:diagnostics')
});
