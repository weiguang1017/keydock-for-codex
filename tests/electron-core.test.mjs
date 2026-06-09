import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import {
  KeydockStore,
  apiKeyStdin,
  commandInvocation,
  extractModels,
  findCodexPath,
  loginWithCodex,
  maskKey,
  modelEndpoint,
  normalizeBaseUrl,
  parseCodexConfig,
  readCodexProfile,
  validateKey
} from '../electron/keydock-core.mjs';

function tempDir(name) {
  return fs.mkdtempSync(path.join(os.tmpdir(), `keydock-${name}-`));
}

function quoteForCmd(value) {
  return `"${String(value).replaceAll('"', '""')}"`;
}

function quoteForSh(value) {
  return `'${String(value).replaceAll("'", "'\\''")}'`;
}

function writeFakeCodex(directory) {
  const codexPath = path.join(directory, process.platform === 'win32' ? 'codex.cmd' : 'codex');
  const capturePath = path.join(directory, 'stdin.txt');
  const scriptPath = path.join(directory, 'fake-codex.mjs');
  const script = `import fs from 'node:fs';\n\nconst capturePath = ${JSON.stringify(capturePath)};\nconst args = process.argv.slice(2);\n\nif (args[0] === 'login' && args[1] === '--with-api-key') {\n  const chunks = [];\n  for await (const chunk of process.stdin) chunks.push(Buffer.from(chunk));\n  const input = Buffer.concat(chunks).toString('utf8').replace(/(?:\\r\\n|\\n|\\r)$/, '');\n  fs.writeFileSync(capturePath, input);\n  console.log('login ok');\n  process.exit(0);\n}\n\nif (args[0] === 'login' && args[1] === 'status') {\n  console.log('Logged in using an API key - sk-test***7890');\n  process.exit(0);\n}\n\nconsole.error('unexpected args');\nprocess.exit(2);\n`;
  const launcher = process.platform === 'win32'
    ? `@echo off\r\n${quoteForCmd(process.execPath)} ${quoteForCmd(scriptPath)} %*\r\nexit /b %ERRORLEVEL%\r\n`
    : `#!/bin/sh\nexec ${quoteForSh(process.execPath)} ${quoteForSh(scriptPath)} "$@"\n`;
  fs.writeFileSync(scriptPath, script);
  fs.writeFileSync(codexPath, launcher, { mode: 0o755 });
  return { codexPath, capturePath };
}

function prependToPath(directory) {
  const pathKey = Object.keys(process.env).find((key) => key.toLowerCase() === 'path') || 'PATH';
  process.env[pathKey] = `${directory}${path.delimiter}${process.env[pathKey] || ''}`;
}

assert.equal(maskKey('sk-1234567890abcdef'), 'sk-1234...cdef');
assert.equal(maskKey('abcd1234'), '***');
assert.equal(apiKeyStdin('  sk-test-newline  ', 'darwin'), 'sk-test-newline\n');
assert.equal(apiKeyStdin('  sk-test-newline  ', 'win32'), 'sk-test-newline\r\n');
assert.equal(normalizeBaseUrl('api.example.com/v1/'), 'https://api.example.com/v1');
assert.equal(modelEndpoint('https://api.example.com/v1').toString(), 'https://api.example.com/v1/models');
assert.deepEqual(extractModels({ data: [{ id: 'gpt-z' }, { id: 'gpt-a' }] }), ['gpt-a', 'gpt-z']);
const codexConfig = parseCodexConfig('model_provider = "OpenAI"\nmodel = "gpt-5.5"\n[model_providers.OpenAI]\nbase_url = "https://cursorvip.com"\n');
assert.equal(codexConfig.root.model_provider, 'OpenAI');
assert.equal(codexConfig.root.model, 'gpt-5.5');
assert.equal(codexConfig.providers.OpenAI.base_url, 'https://cursorvip.com');

const winInvocation = commandInvocation('C:\\Tools\\codex.cmd', ['login', '--with-api-key'], 'win32', 'C:\\Windows\\System32\\cmd.exe');
assert.equal(winInvocation.command, 'C:\\Windows\\System32\\cmd.exe');
assert.deepEqual(winInvocation.args, ['/d', '/c', 'call', 'C:\\Tools\\codex.cmd', 'login', '--with-api-key']);
assert.equal(winInvocation.windowsVerbatimArguments, false);

const storeDir = tempDir('store');
const store = new KeydockStore(storeDir, null);
const record = store.add('Work', 'https://api.example.com/v1', 'sk-test-1234567890', {
  valid: true,
  statusCode: 200,
  message: 'ok',
  models: ['gpt-a', 'gpt-b']
});
assert.equal(store.list().length, 1);
assert.equal(store.secret(record.id), 'sk-test-1234567890');
assert.equal(record.baseUrl, 'https://api.example.com/v1');
assert.equal(record.available, true);
assert.equal(record.model, 'gpt-a');
assert.deepEqual(record.models, ['gpt-a', 'gpt-b']);
const metadata = fs.readFileSync(path.join(storeDir, 'keys.json'), 'utf8');
assert.equal(metadata.includes('sk-test-1234567890'), false);

const codexHome = tempDir('codex-home');
fs.writeFileSync(path.join(codexHome, 'config.toml'), 'model_provider = "OpenAI"\nmodel = "gpt-5.5"\n[model_providers.OpenAI]\nbase_url = "https://cursorvip.com"\n');
fs.writeFileSync(path.join(codexHome, 'auth.json'), JSON.stringify({ OPENAI_API_KEY: 'sk-codex-home-1234567890' }));
fs.writeFileSync(path.join(codexHome, 'models_cache.json'), JSON.stringify({ models: [{ slug: 'gpt-5.5' }, { slug: 'gpt-4.1' }] }));
const profile = readCodexProfile(codexHome);
assert.equal(profile.configured, true);
assert.equal(profile.baseUrl, 'https://cursorvip.com');
assert.equal(profile.model, 'gpt-5.5');
assert.equal(profile.maskedKey, 'sk-code...7890');
assert.deepEqual(profile.models.slice(0, 2), ['gpt-5.5', 'gpt-4.1']);
const imported = store.upsertCodexProfile(profile);
assert.equal(imported.active, true);
assert.equal(store.secret(imported.id), 'sk-codex-home-1234567890');
assert.equal(store.list().filter((item) => item.source === 'codex-config').length, 1);

const check = await validateKey('sk-test-123', { skipNetwork: true });
assert.equal(check.valid, true);
assert.deepEqual(check.models, ['gpt-4.1', 'gpt-4.1-mini']);

const fakeDir = tempDir('codex');
const { codexPath, capturePath } = writeFakeCodex(fakeDir);
prependToPath(fakeDir);
process.env.CKM_CODEX_PATH = codexPath;
const found = await findCodexPath();
assert.equal(fs.existsSync(found), true);
assert.equal(path.basename(found).toLowerCase(), path.basename(codexPath).toLowerCase());
const status = await loginWithCodex('sk-test-stdin-1234567890', found);
assert.match(status, /Logged in using an API key/);
assert.equal(fs.readFileSync(capturePath, 'utf8'), 'sk-test-stdin-1234567890');

console.log('PASS: Keydock for Codex Electron core tests completed.');
