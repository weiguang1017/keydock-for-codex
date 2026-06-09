import fs from 'node:fs';
import path from 'node:path';
import os from 'node:os';
import { spawn } from 'node:child_process';
import http from 'node:http';
import https from 'node:https';
import { randomUUID } from 'node:crypto';

export const APP_NAME = 'Keydock for Codex';
export const SECRET_SERVICE = 'KeydockForCodex';

export function trim(value) {
  return String(value || '').trim();
}

export function maskKey(key) {
  const value = trim(key);
  if (!value) return '';
  if (value.length <= 8) return '***';
  if (value.length <= 14) return `${value.slice(0, 3)}...${value.slice(-3)}`;
  return `${value.slice(0, 7)}...${value.slice(-4)}`;
}

export function normalizeBaseUrl(value) {
  const input = trim(value);
  if (!input) return '';
  const withProtocol = /^[a-z][a-z\d+\-.]*:\/\//i.test(input) ? input : `https://${input}`;
  const url = new URL(withProtocol);
  url.hash = '';
  return url.toString().replace(/\/$/, '');
}

export function modelEndpoint(baseUrl) {
  const normalized = normalizeBaseUrl(baseUrl);
  if (!normalized) throw new Error('Base URL is required.');
  const url = new URL(normalized);
  if (!url.pathname.endsWith('/models')) {
    url.pathname = `${url.pathname.replace(/\/$/, '')}/models`;
  }
  return url;
}

export function extractModels(payload) {
  if (!payload || typeof payload !== 'object') return [];
  const items = Array.isArray(payload.data) ? payload.data : Array.isArray(payload.models) ? payload.models : [];
  return items
    .map((item) => {
      if (typeof item === 'string') return item;
      if (item && typeof item.id === 'string') return item.id;
      if (item && typeof item.name === 'string') return item.name;
      return '';
    })
    .map(trim)
    .filter(Boolean)
    .sort((left, right) => left.localeCompare(right));
}

export function nowIso() {
  return new Date().toISOString();
}

export function appDataDir() {
  return path.join(os.homedir(), 'Library', 'Application Support', APP_NAME);
}

export function ensureDir(directory) {
  fs.mkdirSync(directory, { recursive: true, mode: 0o700 });
}

function readJson(filePath, fallback) {
  try {
    return JSON.parse(fs.readFileSync(filePath, 'utf8'));
  } catch (error) {
    if (error.code === 'ENOENT') return fallback;
    throw error;
  }
}

function writeJson(filePath, value) {
  ensureDir(path.dirname(filePath));
  fs.writeFileSync(filePath, `${JSON.stringify(value, null, 2)}\n`, { mode: 0o600 });
}

export class KeydockStore {
  constructor(directory, safeStorage = null) {
    this.directory = directory;
    this.safeStorage = safeStorage;
    this.keysPath = path.join(directory, 'keys.json');
    this.secretsPath = path.join(directory, 'secrets.json');
  }

  list() {
    const root = readJson(this.keysPath, { keys: [] });
    return Array.isArray(root.keys) ? root.keys : [];
  }

  saveList(records) {
    writeJson(this.keysPath, { keys: records });
  }

  secrets() {
    return readJson(this.secretsPath, { secrets: {} });
  }

  saveSecrets(root) {
    writeJson(this.secretsPath, root);
  }

  protect(secret) {
    if (this.safeStorage?.isEncryptionAvailable?.()) {
      const encrypted = this.safeStorage.encryptString(secret);
      return { mode: 'safeStorage', value: Buffer.from(encrypted).toString('base64') };
    }
    return { mode: 'plain-fallback', value: Buffer.from(secret, 'utf8').toString('base64') };
  }

  reveal(payload) {
    if (!payload) return null;
    if (payload.mode === 'safeStorage') {
      return this.safeStorage.decryptString(Buffer.from(payload.value, 'base64'));
    }
    if (payload.mode === 'plain-fallback') {
      return Buffer.from(payload.value, 'base64').toString('utf8');
    }
    return null;
  }

  add(label, baseUrl, apiKey, validation = {}) {
    const records = this.list();
    const id = randomUUID();
    const timestamp = nowIso();
    const models = Array.isArray(validation.models) ? validation.models : [];
    const selectedModel = trim(validation.model) || models[0] || '';
    const record = {
      id,
      label: trim(label) || 'Untitled key',
      baseUrl: normalizeBaseUrl(baseUrl || 'https://api.openai.com/v1'),
      maskedKey: maskKey(apiKey),
      model: selectedModel,
      models,
      available: validation.valid === true,
      statusCode: Number.isFinite(validation.statusCode) ? validation.statusCode : 0,
      validationMessage: trim(validation.message),
      active: false,
      lastValidatedAt: timestamp,
      createdAt: timestamp,
      updatedAt: timestamp
    };
    records.push(record);
    const secrets = this.secrets();
    secrets.secrets[id] = this.protect(apiKey);
    this.saveSecrets(secrets);
    this.saveList(records);
    return record;
  }

  updateMetadata(id, updates = {}) {
    const records = this.list();
    const record = records.find((item) => item.id === id);
    if (!record) throw new Error('Key not found.');
    if ('label' in updates) record.label = trim(updates.label) || 'Untitled key';
    if ('baseUrl' in updates) {
      const nextBaseUrl = normalizeBaseUrl(updates.baseUrl || record.baseUrl || 'https://api.openai.com/v1');
      if (nextBaseUrl !== record.baseUrl) {
        record.available = false;
        record.models = [];
        record.model = '';
        record.validationMessage = 'Base URL changed. Check the key again.';
      }
      record.baseUrl = nextBaseUrl;
    }
    if ('model' in updates) record.model = trim(updates.model);
    if (Array.isArray(updates.models)) record.models = updates.models;
    if ('available' in updates) record.available = updates.available === true;
    if ('statusCode' in updates) record.statusCode = Number.isFinite(updates.statusCode) ? updates.statusCode : 0;
    if ('validationMessage' in updates) record.validationMessage = trim(updates.validationMessage);
    if ('lastValidatedAt' in updates) record.lastValidatedAt = updates.lastValidatedAt;
    record.updatedAt = nowIso();
    this.saveList(records);
    return record;
  }

  updateName(id, label) {
    return this.updateMetadata(id, { label });
  }

  remove(id) {
    const records = this.list().filter((item) => item.id !== id);
    const secrets = this.secrets();
    delete secrets.secrets[id];
    this.saveSecrets(secrets);
    this.saveList(records);
  }

  secret(id) {
    const payload = this.secrets().secrets[id];
    const secret = this.reveal(payload);
    if (!secret) throw new Error('Secret was not found.');
    return secret;
  }

  markValidated(id) {
    const records = this.list();
    const record = records.find((item) => item.id === id);
    if (!record) throw new Error('Key not found.');
    record.available = true;
    record.lastValidatedAt = nowIso();
    record.updatedAt = nowIso();
    this.saveList(records);
    return record;
  }

  markValidation(id, result) {
    return this.updateMetadata(id, {
      available: result.valid === true,
      statusCode: result.statusCode || 0,
      validationMessage: result.message || '',
      models: result.models || [],
      model: result.model || (result.models || [])[0] || '',
      lastValidatedAt: nowIso()
    });
  }

  markActive(id) {
    const records = this.list();
    const timestamp = nowIso();
    for (const record of records) {
      record.active = record.id === id;
      record.updatedAt = timestamp;
      if (record.id === id) record.lastValidatedAt = timestamp;
    }
    this.saveList(records);
    return records;
  }
}

export function validateKey(apiKey, options = {}) {
  const key = trim(apiKey);
  if (!key) {
    return Promise.resolve({ valid: false, statusCode: 0, message: 'API key is required.', models: [] });
  }
  let validationUrl;
  try {
    validationUrl = options.url || process.env.CKM_VALIDATION_URL
      ? new URL(options.url || process.env.CKM_VALIDATION_URL)
      : modelEndpoint(options.baseUrl || process.env.CKM_BASE_URL || 'https://api.openai.com/v1');
  } catch (error) {
    return Promise.resolve({ valid: false, statusCode: 0, message: error.message || 'Base URL is invalid.', models: [] });
  }
  if (options.skipNetwork || process.env.CKM_SKIP_NETWORK_VALIDATION_FOR_TESTS === '1') {
    return Promise.resolve({
      valid: true,
      statusCode: 200,
      message: 'Test validation passed.',
      models: Array.isArray(options.models) ? options.models : ['gpt-4.1', 'gpt-4.1-mini']
    });
  }

  return new Promise((resolve) => {
    const transport = validationUrl.protocol === 'http:' ? http : https;
    const request = transport.request({
      method: 'GET',
      protocol: validationUrl.protocol,
      hostname: validationUrl.hostname,
      port: validationUrl.port || (validationUrl.protocol === 'http:' ? 80 : 443),
      path: `${validationUrl.pathname}${validationUrl.search}`,
      timeout: 20000,
      headers: {
        Authorization: `Bearer ${key}`,
        Accept: 'application/json'
      }
    }, (response) => {
      const chunks = [];
      response.on('data', (chunk) => chunks.push(Buffer.from(chunk)));
      response.on('end', () => {
        let models = [];
        const body = Buffer.concat(chunks).toString('utf8');
        if (body) {
          try {
            models = extractModels(JSON.parse(body));
          } catch {
            models = [];
          }
        }
        if (response.statusCode === 200) {
          resolve({ valid: true, statusCode: 200, message: 'The platform accepted this key.', models });
        } else if (response.statusCode === 401) {
          resolve({ valid: false, statusCode: 401, message: 'The platform rejected this key.', models });
        } else if (response.statusCode === 403) {
          resolve({ valid: false, statusCode: 403, message: 'This key is not permitted to access the model endpoint.', models });
        } else {
          resolve({ valid: false, statusCode: response.statusCode || 0, message: `Validation failed with HTTP ${response.statusCode || 0}.`, models });
        }
      });
    });
    request.on('timeout', () => {
      request.destroy(new Error('Validation timed out.'));
    });
    request.on('error', (error) => {
      resolve({ valid: false, statusCode: 0, message: error.message });
    });
    request.end();
  });
}

export function commandInvocation(command, args, platform = process.platform, comSpec = process.env.ComSpec) {
  if (platform !== 'win32') {
    return { command, args, windowsVerbatimArguments: false };
  }
  const extension = path.extname(command).toLowerCase();
  if (extension !== '.cmd' && extension !== '.bat') {
    return { command, args, windowsVerbatimArguments: false };
  }
  return {
    command: comSpec || 'cmd.exe',
    args: ['/d', '/c', 'call', command, ...args],
    windowsVerbatimArguments: false
  };
}

export function apiKeyStdin(apiKey, platform = process.platform) {
  const lineEnding = platform === 'win32' ? '\r\n' : '\n';
  return `${trim(apiKey)}${lineEnding}`;
}

export function runCommand(command, args = [], options = {}) {
  return new Promise((resolve, reject) => {
    const invocation = commandInvocation(command, args);
    const child = spawn(invocation.command, invocation.args, {
      env: { ...process.env, ...(options.env || {}) },
      shell: false,
      windowsVerbatimArguments: invocation.windowsVerbatimArguments,
      windowsHide: true
    });
    let stdout = '';
    let stderr = '';
    const timer = setTimeout(() => {
      child.kill('SIGTERM');
      reject(new Error('Command timed out.'));
    }, options.timeoutMs || 30000);
    child.stdout.on('data', (chunk) => { stdout += chunk.toString(); });
    child.stderr.on('data', (chunk) => { stderr += chunk.toString(); });
    child.on('error', (error) => {
      clearTimeout(timer);
      reject(error);
    });
    child.on('close', (status) => {
      clearTimeout(timer);
      resolve({ status, stdout, stderr });
    });
    if (options.stdin) {
      child.stdin.write(options.stdin);
    }
    child.stdin.end();
  });
}

async function shellLookup() {
  if (process.platform === 'win32') {
    const result = await runCommand('cmd.exe', ['/c', 'where codex'], { timeoutMs: 8000 });
    return trim(result.stdout).split(/\r?\n/)[0];
  }
  const shell = process.env.SHELL || '/bin/zsh';
  const result = await runCommand(shell, ['-lc', 'command -v codex'], { timeoutMs: 8000 });
  return trim(result.stdout).split(/\r?\n/)[0];
}

export async function findCodexPath() {
  if (process.env.CKM_CODEX_PATH && fs.existsSync(process.env.CKM_CODEX_PATH)) {
    return process.env.CKM_CODEX_PATH;
  }
  try {
    const found = await shellLookup();
    if (found && fs.existsSync(found)) return found;
  } catch {
    // Fall back to common install locations below.
  }

  const home = os.homedir();
  const candidates = process.platform === 'win32'
    ? [
        path.join(process.env.LOCALAPPDATA || '', 'Programs', 'nodejs', 'codex.cmd'),
        path.join(process.env.APPDATA || '', 'npm', 'codex.cmd')
      ]
    : [
        '/opt/homebrew/bin/codex',
        '/usr/local/bin/codex',
        path.join(home, '.local/bin/codex'),
        path.join(home, '.nvm/current/bin/codex')
      ];

  const nvmRoot = path.join(home, '.nvm/versions/node');
  if (fs.existsSync(nvmRoot)) {
    for (const version of fs.readdirSync(nvmRoot).sort().reverse()) {
      candidates.push(path.join(nvmRoot, version, 'bin/codex'));
    }
  }

  const found = candidates.find((candidate) => candidate && fs.existsSync(candidate));
  if (found) return found;
  throw new Error('Codex CLI was not found. Configure codex in your shell first.');
}

export async function loginWithCodex(apiKey, codexPath) {
  const login = await runCommand(codexPath, ['login', '--with-api-key'], {
    stdin: apiKeyStdin(apiKey),
    timeoutMs: 30000
  });
  if (login.status !== 0) {
    throw new Error(trim(login.stderr) || 'codex login --with-api-key failed.');
  }
  const status = await runCommand(codexPath, ['login', 'status'], { timeoutMs: 15000 });
  if (status.status !== 0) {
    throw new Error(trim(status.stderr) || 'codex login status failed after switching.');
  }
  return trim(status.stdout);
}

export async function restartCodexDesktop(codexPath) {
  if (process.env.CKM_DISABLE_RESTART === '1') return;
  if (process.platform === 'darwin') {
    await runCommand('/usr/bin/osascript', ['-e', 'tell application id "com.openai.codex" to quit'], { timeoutMs: 5000 }).catch(() => null);
    await runCommand('/usr/bin/open', ['-b', 'com.openai.codex'], { timeoutMs: 10000 });
    return;
  }
  if (process.platform === 'win32') {
    await runCommand('taskkill.exe', ['/IM', 'Codex.exe', '/F'], { timeoutMs: 5000 }).catch(() => null);
  } else {
    await runCommand('/usr/bin/pkill', ['-x', 'Codex'], { timeoutMs: 5000 }).catch(() => null);
  }
  await runCommand(codexPath, ['app'], { timeoutMs: 15000 }).catch(() => null);
}
