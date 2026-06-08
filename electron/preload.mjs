import { contextBridge, ipcRenderer } from 'electron';

contextBridge.exposeInMainWorld('keydock', {
  listKeys: () => ipcRenderer.invoke('keys:list'),
  addKey: (payload) => ipcRenderer.invoke('keys:add', payload),
  updateName: (payload) => ipcRenderer.invoke('keys:updateName', payload),
  deleteKey: (payload) => ipcRenderer.invoke('keys:delete', payload),
  validateKey: (payload) => ipcRenderer.invoke('keys:validate', payload),
  switchKey: (payload) => ipcRenderer.invoke('keys:switch', payload),
  diagnostics: () => ipcRenderer.invoke('app:diagnostics')
});
