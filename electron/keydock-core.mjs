import fs from 'node:fs';
import path from 'node:path';
import os from 'node:os';
import { spawn } from 'node:child_process';
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

  add(label, apiKey) {
    const records = this.list();
    const id = randomUUID();
    const timestamp = nowIso();
    const record = {
      id,
      label: trim(label) || 'OpenAI key',
      maskedKey: maskKey(apiKey),
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

  updateName(id, label) {
    const records = this.list();
    const record = records.find((item) => item.id === id);
    if (!record) throw new Error('Key not found.');
    record.label = trim(label) || 'Untitled key';
    record.updatedAt = nowIso();
    this.saveList(records);
    return record;
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
    record.lastValidatedAt = nowIso();
    record.updatedAt = nowIso();
    this.saveList(records);
    return record;
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
  if (!key.startsWith('sk-')) {
    return Promise.resolve({ valid: false, statusCode: 0, message: 'The key must start with sk-.' });
  }
  if (options.skipNetwork || process.env.CKM_SKIP_NETWORK_VALIDATION_FOR_TESTS === '1') {
    return Promise.resolve({ valid: true, statusCode: 200, message: 'Test validation passed.' });
  }

  const validationUrl = new URL(options.url || process.env.CKM_VALIDATION_URL || 'https://api.openai.com/v1/models');
  return new Promise((resolve) => {
    const request = https.request({
      method: 'GET',
      protocol: validationUrl.protocol,
      hostname: validationUrl.hostname,
      port: validationUrl.port || 443,
      path: `${validationUrl.pathname}${validationUrl.search}`,
      timeout: 20000,
      headers: {
        Authorization: `Bearer ${key}`,
        Accept: 'application/json'
      }
    }, (response) => {
      response.resume();
      response.on('end', () => {
        if (response.statusCode === 200) {
          resolve({ valid: true, statusCode: 200, message: 'OpenAI accepted this key.' });
        } else if (response.statusCode === 401) {
          resolve({ valid: false, statusCode: 401, message: 'OpenAI rejected this key.' });
        } else if (response.statusCode === 403) {
          resolve({ valid: false, statusCode: 403, message: 'This key is not permitted to access the validation endpoint.' });
        } else {
          resolve({ valid: false, statusCode: response.statusCode || 0, message: `Validation failed with HTTP ${response.statusCode || 0}.` });
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

function quoteForCmd(value) {
  const text = String(value);
  return `"${text.replaceAll('"', '""')}"`;
}

export function commandInvocation(command, args, platform = process.platform, comSpec = process.env.ComSpec) {
  if (platform !== 'win32') {
    return { command, args };
  }
  const extension = path.extname(command).toLowerCase();
  if (extension !== '.cmd' && extension !== '.bat') {
    return { command, args };
  }
  const commandLine = [quoteForCmd(command), ...args.map(quoteForCmd)].join(' ');
  return {
    command: comSpec || 'cmd.exe',
    args: ['/d', '/s', '/c', commandLine]
  };
}

export function runCommand(command, args = [], options = {}) {
  return new Promise((resolve, reject) => {
    const invocation = commandInvocation(command, args);
    const child = spawn(invocation.command, invocation.args, {
      env: { ...process.env, ...(options.env || {}) },
      shell: false,
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
    stdin: `${trim(apiKey)}\n`,
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
