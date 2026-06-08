let keys = [];
let selectedId = null;
let busy = false;

const $ = (id) => document.getElementById(id);

function formatDate(value) {
  if (!value) return 'Never';
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  return date.toLocaleString();
}

function selectedKey() {
  return keys.find((key) => key.id === selectedId) || null;
}

function setBusy(value, message) {
  busy = value;
  $('busy').classList.toggle('hidden', !value);
  if (message) $('message').textContent = message;
  renderDetail();
}

function renderList() {
  $('keyList').innerHTML = '';
  if (keys.length === 0) {
    const empty = document.createElement('p');
    empty.className = 'diag';
    empty.textContent = 'No keys yet.';
    $('keyList').append(empty);
    return;
  }
  for (const key of keys) {
    const row = document.createElement('button');
    row.className = `key-row${key.id === selectedId ? ' selected' : ''}`;
    row.innerHTML = `<strong>${key.active ? '<span class="badge">ACTIVE</span>' : ''}${escapeHtml(key.label)}</strong><code>${escapeHtml(key.maskedKey)}</code>`;
    row.addEventListener('click', () => {
      selectedId = key.id;
      render();
    });
    $('keyList').append(row);
  }
}

function renderDetail() {
  const key = selectedKey();
  const disabled = !key || busy;
  $('detailTitle').textContent = key ? key.label : 'No key selected';
  $('nameField').value = key?.label || '';
  $('maskedField').value = key?.maskedKey || '-';
  $('statusField').value = key ? (key.active ? 'Active in Codex' : 'Validated') : '-';
  $('checkedField').value = key ? formatDate(key.lastValidatedAt) : '-';
  for (const id of ['nameField', 'saveNameButton', 'checkButton', 'switchButton', 'deleteButton']) {
    $(id).disabled = disabled;
  }
}

function render() {
  renderList();
  renderDetail();
}

function escapeHtml(value) {
  return String(value)
    .replaceAll('&', '&amp;')
    .replaceAll('<', '&lt;')
    .replaceAll('>', '&gt;')
    .replaceAll('"', '&quot;');
}

async function refresh() {
  keys = await window.keydock.listKeys();
  if (!selectedId && keys.length > 0) selectedId = keys[0].id;
  if (selectedId && !keys.some((key) => key.id === selectedId)) selectedId = keys[0]?.id || null;
  render();
}

async function withAction(message, action) {
  try {
    setBusy(true, message);
    const result = await action();
    await refresh();
    return result;
  } catch (error) {
    $('message').textContent = error.message || 'Action failed.';
    return null;
  } finally {
    setBusy(false);
  }
}

$('addButton').addEventListener('click', () => {
  $('newName').value = 'OpenAI key';
  $('newKey').value = '';
  $('addDialog').showModal();
});

$('confirmAdd').addEventListener('click', async (event) => {
  event.preventDefault();
  const label = $('newName').value;
  const apiKey = $('newKey').value;
  $('addDialog').close();
  const record = await withAction('Checking key before saving...', () => window.keydock.addKey({ label, apiKey }));
  if (record) {
    selectedId = record.id;
    $('message').textContent = 'Key saved and validated.';
    await refresh();
  }
});

$('saveNameButton').addEventListener('click', async () => {
  if (!selectedId) return;
  await withAction('Saving name...', () => window.keydock.updateName({ id: selectedId, label: $('nameField').value }));
  $('message').textContent = 'Name saved.';
});

$('checkButton').addEventListener('click', async () => {
  if (!selectedId) return;
  const result = await withAction('Checking key...', () => window.keydock.validateKey({ id: selectedId }));
  $('message').textContent = result?.valid ? 'Key is valid.' : (result?.message || 'Key check failed.');
});

$('switchButton').addEventListener('click', async () => {
  if (!selectedId) return;
  const result = await withAction('Switching Codex key and restarting Desktop...', () => window.keydock.switchKey({ id: selectedId }));
  if (result) $('message').textContent = `Switched. ${result.status} Terminal Codex sessions must be reopened.`;
});

$('deleteButton').addEventListener('click', async () => {
  if (!selectedId) return;
  if (!confirm('Delete this key from Keydock?')) return;
  await withAction('Deleting key...', () => window.keydock.deleteKey({ id: selectedId }));
  selectedId = null;
  $('message').textContent = 'Key deleted.';
});

window.keydock.diagnostics().then((info) => {
  $('diag').textContent = info.codexPath
    ? `Codex CLI: ${info.codexPath} · Secrets: ${info.encryption}`
    : `${info.message} · Secrets: ${info.encryption}`;
});

refresh();
