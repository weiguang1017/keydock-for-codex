import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import {
  KeydockStore,
  findCodexPath,
  loginWithCodex,
  maskKey,
  validateKey
} from '../electron/keydock-core.mjs';

function tempDir(name) {
  return fs.mkdtempSync(path.join(os.tmpdir(), `keydock-${name}-`));
}

function writeFakeCodex(directory) {
  const codexPath = path.join(directory, process.platform === 'win32' ? 'codex.cmd' : 'codex');
  const capturePath = path.join(directory, 'stdin.txt');
  const script = process.platform === 'win32'
    ? `@echo off\r\nif "%1"=="login" if "%2"=="--with-api-key" (set /p KEY=& <nul set /p=%KEY% > "${capturePath}"& echo login ok& exit /b 0)\r\nif "%1"=="login" if "%2"=="status" (echo Logged in using an API key - sk-test***7890& exit /b 0)\r\necho unexpected args 1>&2\r\nexit /b 2\r\n`
    : `#!/bin/sh\nif [ "$1" = "login" ] && [ "$2" = "--with-api-key" ]; then\n  IFS= read -r KEY\n  printf '%s' "$KEY" > '${capturePath}'\n  printf 'login ok\\n'\n  exit 0\nfi\nif [ "$1" = "login" ] && [ "$2" = "status" ]; then\n  printf 'Logged in using an API key - sk-test***7890\\n'\n  exit 0\nfi\nprintf 'unexpected args\\n' >&2\nexit 2\n`;
  fs.writeFileSync(codexPath, script, { mode: 0o755 });
  return { codexPath, capturePath };
}

assert.equal(maskKey('sk-1234567890abcdef'), 'sk-1234...cdef');
assert.equal(maskKey('abcd1234'), '***');

const storeDir = tempDir('store');
const store = new KeydockStore(storeDir, null);
const record = store.add('Work', 'sk-test-1234567890');
assert.equal(store.list().length, 1);
assert.equal(store.secret(record.id), 'sk-test-1234567890');
const metadata = fs.readFileSync(path.join(storeDir, 'keys.json'), 'utf8');
assert.equal(metadata.includes('sk-test-1234567890'), false);

const check = await validateKey('sk-test-123', { skipNetwork: true });
assert.equal(check.valid, true);

const fakeDir = tempDir('codex');
const { codexPath, capturePath } = writeFakeCodex(fakeDir);
process.env.PATH = `${fakeDir}${path.delimiter}${process.env.PATH || ''}`;
const found = await findCodexPath();
assert.equal(path.resolve(found), path.resolve(codexPath));
const status = await loginWithCodex('sk-test-stdin-1234567890', found);
assert.match(status, /Logged in using an API key/);
assert.equal(fs.readFileSync(capturePath, 'utf8'), 'sk-test-stdin-1234567890');

console.log('PASS: Keydock for Codex Electron core tests completed.');
