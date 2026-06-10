import fs from 'node:fs';
import path from 'node:path';
import os from 'node:os';
import { spawn } from 'node:child_process';
import http from 'node:http';
import https from 'node:https';
import { randomUUID } from 'node:crypto';

export const APP_NAME = 'Keydock for Codex';
export const SECRET_SERVICE = 'KeydockForCodex';
export const DEFAULT_BASE_URL = 'https://api.openai.com/v1';
export const DEFAULT_CODEX_PROFILE_LABEL = 'Codex CLI';

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

function uniqueStrings(values) {
  return [...new Set((Array.isArray(values) ? values : [])
    .map(trim)
    .filter(Boolean))];
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

function stripTomlComment(line) {
  let quote = '';
  let escaped = false;
  for (let index = 0; index < line.length; index += 1) {
    const char = line[index];
    if (escaped) {
      escaped = false;
      continue;
    }
    if (quote === '"' && char === '\\') {
      escaped = true;
      continue;
    }
    if ((char === '"' || char === "'") && !quote) {
      quote = char;
      continue;
    }
    if (char === quote) {
      quote = '';
      continue;
    }
    if (char === '#' && !quote) return line.slice(0, index);
  }
  return line;
}

function unquoteToml(value) {
  const input = trim(value);
  if (input.length < 2) return input;
  const quote = input[0];
  if ((quote !== '"' && quote !== "'") || input[input.length - 1] !== quote) return input;
  const body = input.slice(1, -1);
  if (quote === "'") return body;
  return body
    .replaceAll('\\"', '"')
    .replaceAll('\\\\', '\\')
    .replaceAll('\\n', '\n')
    .replaceAll('\\r', '\r')
    .replaceAll('\\t', '\t');
}

function splitTomlPath(value) {
  const parts = [];
  let current = '';
  let quote = '';
  let escaped = false;
  for (const char of value) {
    if (escaped) {
      current += char;
      escaped = false;
      continue;
    }
    if (quote === '"' && char === '\\') {
      current += char;
      escaped = true;
      continue;
    }
    if ((char === '"' || char === "'") && !quote) {
      quote = char;
      current += char;
      continue;
    }
    if (char === quote) {
      quote = '';
      current += char;
      continue;
    }
    if (char === '.' && !quote) {
      parts.push(unquoteToml(current));
      current = '';
      continue;
    }
    current += char;
  }
  if (current) parts.push(unquoteToml(current));
  return parts.map(trim).filter(Boolean);
}

function parseTomlValue(rawValue) {
  const value = trim(rawValue);
  if (!value) return '';
  if (value.startsWith('"') || value.startsWith("'")) return unquoteToml(value);
  if (value.startsWith('[') && value.endsWith(']')) {
    const body = value.slice(1, -1);
    return body
      .split(',')
      .map((item) => parseTomlValue(item))
      .filter((item) => item !== '');
  }
  if (value === 'true') return true;
  if (value === 'false') return false;
  return value;
}

export function parseCodexConfig(content) {
  const root = {};
  const providers = {};
  let section = [];

  for (const rawLine of String(content || '').split(/\r?\n/)) {
    const line = trim(stripTomlComment(rawLine));
    if (!line) continue;
    const sectionMatch = line.match(/^\[([^\]]+)]$/);
    if (sectionMatch) {
      section = splitTomlPath(sectionMatch[1]);
      continue;
    }
    const separator = line.indexOf('=');
    if (separator < 1) continue;
    const key = trim(line.slice(0, separator));
    const value = parseTomlValue(line.slice(separator + 1));
    if (section[0] === 'model_providers' && section.length >= 2) {
      const providerName = section.slice(1).join('.');
      providers[providerName] = providers[providerName] || {};
      providers[providerName][key] = value;
    } else if (section.length === 0) {
      root[key] = value;
    }
  }

  return { root, providers };
}

function firstString(...values) {
  return values.find((value) => typeof value === 'string' && trim(value)) || '';
}

function findApiKeyInAuth(value, keyName = '') {
  if (typeof value === 'string') {
    if (/api[_-]?key|openai_api_key/i.test(keyName) || /^sk-[A-Za-z0-9._-]+/.test(value)) {
      return trim(value);
    }
    return '';
  }
  if (!value || typeof value !== 'object') return '';
  for (const [key, child] of Object.entries(value)) {
    if (/access[_-]?token|refresh[_-]?token|id[_-]?token/i.test(key)) continue;
    const found = findApiKeyInAuth(child, key);
    if (found) return found;
  }
  return '';
}

function readCodexModelCache(directory, selectedModel) {
  const models = [];
  const cachePath = path.join(directory, 'models_cache.json');
  try {
    const cache = readJson(cachePath, {});
    const items = Array.isArray(cache.models) ? cache.models : [];
    for (const item of items) {
      if (typeof item === 'string') models.push(item);
      if (item && typeof item.slug === 'string') models.push(item.slug);
      if (item && typeof item.id === 'string') models.push(item.id);
    }
  } catch {
    // The cache is optional; the configured model is enough for startup.
  }
  return uniqueStrings([selectedModel, ...models]);
}

function formatTomlString(value) {
  const text = String(value ?? '');
  const escaped = text
    .replaceAll('\\', '\\\\')
    .replaceAll('"', '\\"')
    .replaceAll('\n', '\\n')
    .replaceAll('\r', '\\r')
    .replaceAll('\t', '\\t');
  return `"${escaped}"`;
}

function sectionFingerprint(parts) {
  return JSON.stringify((Array.isArray(parts) ? parts : []).map(trim));
}

// Edit a single `key = value` line in place inside the given TOML section,
// preserving every other line, comment, and the original layout.
// sectionPath is [] for the root table, or e.g. ['model_providers', 'OpenAI'].
export function setTomlValue(content, sectionPath, key, value) {
  const targetKey = trim(key);
  const targetSection = sectionFingerprint(sectionPath);
  const isRootTarget = sectionPath.length === 0;
  const newline = content.includes('\r\n') ? '\r\n' : '\n';
  const lines = String(content || '').split(/\r?\n/);
  const formatted = `${targetKey} = ${formatTomlString(value)}`;

  let inTarget = isRootTarget;
  let sectionFound = isRootTarget;
  let lastContentInSection = -1;
  let firstHeaderIndex = -1;

  for (let i = 0; i < lines.length; i += 1) {
    const stripped = trim(stripTomlComment(lines[i]));
    const headerMatch = stripped.match(/^\[([^\]]+)]$/);
    if (headerMatch) {
      if (firstHeaderIndex === -1) firstHeaderIndex = i;
      const section = splitTomlPath(headerMatch[1]);
      inTarget = !isRootTarget && sectionFingerprint(section) === targetSection;
      if (inTarget) {
        sectionFound = true;
        lastContentInSection = i;
      }
      continue;
    }
    if (!inTarget) continue;
    if (stripped) lastContentInSection = i;
    const separator = stripped.indexOf('=');
    if (separator > 0 && trim(stripped.slice(0, separator)) === targetKey) {
      lines[i] = formatted;
      return lines.join(newline);
    }
  }

  if (isRootTarget) {
    const insertAt = firstHeaderIndex === -1 ? lines.length : firstHeaderIndex;
    lines.splice(insertAt, 0, formatted);
    return lines.join(newline);
  }
  if (sectionFound) {
    lines.splice(lastContentInSection + 1, 0, formatted);
    return lines.join(newline);
  }
  const block = [];
  if (lines.length && trim(lines[lines.length - 1]) !== '') block.push('');
  block.push(`[${sectionPath.map(trim).join('.')}]`);
  block.push(formatted);
  return [...lines, ...block].join(newline);
}

function defaultConfigTemplate(provider, baseUrl) {
  return [
    `model_provider = "${provider}"`,
    '',
    `[model_providers.${provider}]`,
    `name = "${provider}"`,
    `base_url = "${baseUrl}"`,
    'wire_api = "responses"',
    'requires_openai_auth = true',
    ''
  ].join('\n');
}

function backupFile(filePath) {
  try {
    if (fs.existsSync(filePath)) fs.copyFileSync(filePath, `${filePath}.bak`);
  } catch {
    // Backups are best-effort; a failed copy must not block switching.
  }
}

function atomicWrite(filePath, content, mode = 0o600) {
  ensureDir(path.dirname(filePath));
  const tmpPath = `${filePath}.tmp-${randomUUID()}`;
  fs.writeFileSync(tmpPath, content, { mode });
  fs.renameSync(tmpPath, filePath);
}

// Switch the active Codex key by rewriting ~/.codex/config.toml (base_url + model)
// and auth.json (OPENAI_API_KEY) directly, preserving unrelated content.
export function applyCodexProfile(options = {}) {
  const directory = options.directory || process.env.CODEX_HOME || path.join(os.homedir(), '.codex');
  const apiKey = trim(options.apiKey);
  if (!apiKey) throw new Error('API key is required to update Codex configuration.');
  const baseUrl = normalizeBaseUrl(options.baseUrl || DEFAULT_BASE_URL);
  const model = trim(options.model);
  const configPath = path.join(directory, 'config.toml');
  const authPath = path.join(directory, 'auth.json');
  ensureDir(directory);

  let providerName = trim(options.providerName);
  let configContent = '';
  if (fs.existsSync(configPath)) {
    configContent = fs.readFileSync(configPath, 'utf8');
    if (!providerName) {
      const parsed = parseCodexConfig(configContent);
      providerName = firstString(
        parsed.root.model_provider,
        parsed.root.default_model_provider,
        parsed.root.provider
      );
    }
    backupFile(configPath);
  }
  if (!providerName) providerName = 'OpenAI';
  if (!trim(configContent)) configContent = defaultConfigTemplate(providerName, baseUrl);

  let nextConfig = setTomlValue(configContent, [], 'model_provider', providerName);
  if (model) nextConfig = setTomlValue(nextConfig, [], 'model', model);
  nextConfig = setTomlValue(nextConfig, ['model_providers', providerName], 'base_url', baseUrl);
  if (!nextConfig.endsWith('\n')) nextConfig += '\n';
  atomicWrite(configPath, nextConfig);

  let auth = {};
  if (fs.existsSync(authPath)) {
    backupFile(authPath);
    try {
      auth = readJson(authPath, {});
    } catch {
      auth = {};
    }
  }
  if (!auth || typeof auth !== 'object') auth = {};
  auth.OPENAI_API_KEY = apiKey;
  atomicWrite(authPath, `${JSON.stringify(auth, null, 2)}\n`);

  return { directory, configPath, authPath, providerName, baseUrl, model };
}

export function readCodexProfile(directory = process.env.CODEX_HOME || path.join(os.homedir(), '.codex')) {
  const configPath = path.join(directory, 'config.toml');
  const authPath = path.join(directory, 'auth.json');
  const profile = {
    directory,
    configPath,
    authPath,
    hasDirectory: fs.existsSync(directory),
    hasConfig: fs.existsSync(configPath),
    hasAuth: fs.existsSync(authPath),
    configured: false,
    label: DEFAULT_CODEX_PROFILE_LABEL,
    providerName: '',
    baseUrl: DEFAULT_BASE_URL,
    hasProviderBaseUrl: false,
    model: '',
    models: [],
    maskedKey: '',
    message: ''
  };

  let parsedConfig = { root: {}, providers: {} };
  if (profile.hasConfig) {
    try {
      parsedConfig = parseCodexConfig(fs.readFileSync(configPath, 'utf8'));
    } catch (error) {
      profile.message = error.message || 'Unable to read Codex config.toml.';
    }
  }

  const providerName = firstString(
    parsedConfig.root.model_provider,
    parsedConfig.root.default_model_provider,
    parsedConfig.root.provider
  );
  const provider = parsedConfig.providers[providerName]
    || parsedConfig.providers.OpenAI
    || Object.values(parsedConfig.providers)[0]
    || {};
  profile.providerName = providerName || firstString(provider.name) || 'OpenAI';
  profile.label = profile.providerName ? `Codex ${profile.providerName}` : DEFAULT_CODEX_PROFILE_LABEL;
  const providerBaseUrl = firstString(
    provider.base_url,
    provider.baseURL,
    provider.api_base,
    provider.api_base_url,
    parsedConfig.root.base_url,
    parsedConfig.root.api_base
  );
  profile.hasProviderBaseUrl = Boolean(providerBaseUrl);
  profile.baseUrl = providerBaseUrl || DEFAULT_BASE_URL;
  profile.model = firstString(parsedConfig.root.model, provider.model, parsedConfig.root.default_model);

  let auth = {};
  if (profile.hasAuth) {
    try {
      auth = readJson(authPath, {});
    } catch (error) {
      profile.message = error.message || 'Unable to read Codex auth.json.';
    }
  }
  const apiKey = findApiKeyInAuth(auth);
  profile.maskedKey = maskKey(apiKey);
  profile.models = readCodexModelCache(directory, profile.model);
  // "Configured" means an actual API key is present. A lone base_url without a
  // key is not a usable Codex login, so we must not report it as configured —
  // otherwise the UI would hide the "add a key" guidance on every platform.
  profile.configured = Boolean(apiKey);
  profile.apiKey = apiKey;
  if (!profile.configured && !profile.message) {
    if (!profile.hasDirectory) profile.message = 'Codex config directory was not found.';
    else if (!profile.hasAuth) profile.message = 'Codex auth.json was not found.';
    else profile.message = 'Codex auth.json does not contain an API key.';
  }
  return profile;
}

export function publicCodexProfile(profile) {
  if (!profile) return null;
  const { apiKey: _apiKey, ...safeProfile } = profile;
  return safeProfile;
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
    const models = uniqueStrings(validation.models);
    const selectedModel = trim(validation.model) || models[0] || '';
    const record = {
      id,
      label: trim(label) || 'Untitled key',
      baseUrl: normalizeBaseUrl(baseUrl || DEFAULT_BASE_URL),
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

  upsertCodexProfile(profile) {
    if (!profile?.configured || !profile.apiKey) return null;
    const records = this.list();
    const secrets = this.secrets();
    const timestamp = nowIso();
    const baseUrl = normalizeBaseUrl(profile.baseUrl || DEFAULT_BASE_URL);
    const models = uniqueStrings(profile.models);
    const selectedModel = trim(profile.model) || models[0] || '';
    const maskedKey = maskKey(profile.apiKey);
    const matchingSecret = records.find((item) => {
      try {
        return this.reveal(secrets.secrets[item.id]) === profile.apiKey;
      } catch {
        return false;
      }
    });
    let record = matchingSecret
      || records.find((item) => item.source === 'codex-config')
      || records.find((item) => item.maskedKey === maskedKey && normalizeBaseUrl(item.baseUrl || DEFAULT_BASE_URL) === baseUrl);
    if (!record) {
      record = {
        id: randomUUID(),
        label: trim(profile.label) || DEFAULT_CODEX_PROFILE_LABEL,
        baseUrl,
        maskedKey,
        model: selectedModel,
        models,
        available: false,
        statusCode: 0,
        validationMessage: trim(profile.message) || 'Imported from Codex config.',
        active: false,
        source: 'codex-config',
        lastValidatedAt: '',
        createdAt: timestamp,
        updatedAt: timestamp
      };
      records.push(record);
    } else {
      record.label = record.label || trim(profile.label) || DEFAULT_CODEX_PROFILE_LABEL;
      record.baseUrl = baseUrl;
      record.maskedKey = maskedKey;
      record.model = selectedModel || record.model || '';
      record.models = uniqueStrings([...models, ...(record.models || [])]);
      record.validationMessage = trim(profile.message) || record.validationMessage || 'Imported from Codex config.';
      if (!matchingSecret) record.source = 'codex-config';
      record.updatedAt = timestamp;
    }
    for (const item of records) {
      item.active = item.id === record.id;
      if (item.id === record.id) item.updatedAt = timestamp;
    }
    secrets.secrets[record.id] = this.protect(profile.apiKey);
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
      const nextBaseUrl = normalizeBaseUrl(updates.baseUrl || record.baseUrl || DEFAULT_BASE_URL);
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
      : modelEndpoint(options.baseUrl || process.env.CKM_BASE_URL || DEFAULT_BASE_URL);
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

export async function readCodexLogin(codexPath) {
  const status = await runCommand(codexPath, ['login', 'status'], { timeoutMs: 15000 });
  if (status.status !== 0) {
    throw new Error(trim(status.stderr) || 'codex login status failed.');
  }
  return trim(status.stdout);
}

export function extractMaskedKeyFromStatus(statusOutput) {
  const text = trim(statusOutput);
  if (!text) return '';
  const masked = text.match(/sk-[A-Za-z0-9*._-]+/);
  return masked ? masked[0] : '';
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
