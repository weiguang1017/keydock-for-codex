let keys = [];
let busy = false;
let filterText = '';
let draftValidation = null;
let editingId = null;
let codexInfo = null;

const tauriInvoke = window.__TAURI__?.core?.invoke || window.__TAURI__?.invoke;
if (!window.keydock && typeof tauriInvoke === 'function') {
  window.keydock = {
    listKeys: () => tauriInvoke('list_keys'),
    testDraftKey: ({ baseUrl, apiKey }) => tauriInvoke('test_draft_key', { baseUrl, apiKey }),
    addKey: ({ label, baseUrl, apiKey, model, validation }) => tauriInvoke('add_key', {
      label,
      baseUrl,
      apiKey,
      model,
      validation
    }),
    updateName: ({ id, label }) => tauriInvoke('update_name', { id, label }),
    updateMetadata: ({ id, label, baseUrl, model }) => tauriInvoke('update_metadata', {
      id,
      label,
      baseUrl,
      model
    }),
    deleteKey: ({ id }) => tauriInvoke('delete_key', { id }),
    validateKey: ({ id }) => tauriInvoke('validate_key_cmd', { id }),
    switchKey: ({ id }) => tauriInvoke('switch_key', { id }),
    diagnostics: () => tauriInvoke('diagnostics')
  };
}

const STORAGE_LANGUAGE_KEY = 'keydock.language';
const DEFAULT_BASE_URL = 'https://api.openai.com/v1';
const SUPPORTED_LANGUAGES = ['en', 'zh', 'ja'];

const translations = {
  en: {
    appSubtitle: 'for Codex',
    addApiKey: 'Add API key',
    noKeys: 'No keys yet.',
    emptyTitle: 'Add your first key',
    emptyHint: 'No saved keys were found. Add one to start switching quickly.',
    activeBadge: 'ACTIVE',
    modelsBadge: '{count} models',
    currentActive: 'Current key',
    allKeys: 'All keys',
    activeFromConfig: 'Detected from ~/.codex',
    noActiveTitle: 'No Codex key configured',
    language: 'Language',
    searchKeys: 'Search keys',
    searchPlaceholder: 'Search name, URL, model',
    languageAuto: 'Auto',
    languageEnglish: 'English',
    languageChinese: '中文',
    languageJapanese: '日本語',
    name: 'Name',
    baseUrl: 'Base URL',
    apiKey: 'API key',
    maskedKey: 'Masked key',
    model: 'Model',
    models: 'Models',
    status: 'Status',
    lastChecked: 'Last checked',
    save: 'Save',
    editKey: 'Edit key',
    switchAction: 'Switch',
    check: 'Check',
    edit: 'Edit',
    delete: 'Delete',
    cancel: 'Cancel',
    add: 'Add',
    test: 'Test',
    never: 'Never',
    unavailable: 'Unavailable',
    available: 'Available',
    activeInCodex: 'Active in Codex',
    notChecked: 'Not checked',
    noModels: 'No models reported yet',
    keyWillBeChecked: 'Test the key to load available models, then add it to Keydock.',
    footerMessage: 'Terminal Codex sessions must be reopened after switching.',
    checkingBeforeSave: 'Checking key before saving...',
    testingDraft: 'Testing key...',
    testPassed: 'Key is valid. Models loaded.',
    testFailed: 'Key test failed.',
    testBeforeAdd: 'Test this key before adding it.',
    savedValidated: 'Key saved and validated.',
    savingDetails: 'Saving details...',
    detailsSaved: 'Details saved.',
    checkingKey: 'Checking key...',
    keyValid: 'Key is valid.',
    actionFailed: 'Action failed.',
    switching: 'Switching Codex key and writing config...',
    switchedOk: 'Switched. Codex config updated and Desktop restarted.',
    switchedManual: 'Switched. Codex config updated — restart Codex to apply.',
    deleteConfirm: 'Delete this key from Keydock?',
    deletingKey: 'Deleting key...',
    keyDeleted: 'Key deleted.',
    missingName: 'Name is required.',
    missingBaseUrl: 'Base URL is required.',
    missingApiKey: 'API key is required.',
    codexConfigured: 'Codex config: {maskedKey} · {baseUrl} · {model}',
    codexNeedsSetup: 'No configured Codex key found. Add an API key to start.',
    currentConfiguredKey: 'Current Codex key: {maskedKey}',
    noBridge: 'Desktop bridge is unavailable. Launch the built app, not the browser preview.',
    modelPlaceholder: 'Test to load models',
    copyBaseUrl: 'Copy Base URL',
    copyMaskedKey: 'Copy masked key',
    copied: 'Copied.',
    noMatches: 'No matching keys.'
  },
  zh: {
    appSubtitle: 'for Codex',
    addApiKey: '添加 Key',
    noKeys: '还没有 Key。',
    emptyTitle: '添加第一个 Key',
    emptyHint: '当前没有已保存的 Key，先添加一个，后面切换会轻松很多。',
    activeBadge: '当前',
    modelsBadge: '{count} 个模型',
    currentActive: '当前生效',
    allKeys: '所有 Key',
    activeFromConfig: '来自 ~/.codex',
    noActiveTitle: '尚未配置 Codex Key',
    language: '语言',
    searchKeys: '搜索 Key',
    searchPlaceholder: '搜索名称、URL、模型',
    languageAuto: '自动',
    languageEnglish: 'English',
    languageChinese: '中文',
    languageJapanese: '日本語',
    name: '名称',
    baseUrl: 'Base URL',
    apiKey: 'API Key',
    maskedKey: '隐藏后的 Key',
    model: '模型',
    models: '模型',
    status: '状态',
    lastChecked: '上次检查',
    save: '保存',
    editKey: '编辑 Key',
    switchAction: '切换',
    check: '检测',
    edit: '编辑',
    delete: '删除',
    cancel: '取消',
    add: '添加',
    test: '测试',
    never: '从未',
    unavailable: '不可用',
    available: '可用',
    activeInCodex: 'Codex 当前使用',
    notChecked: '未检查',
    noModels: '还没有返回模型',
    keyWillBeChecked: '先测试 Key 并加载可用模型，然后再添加到 Keydock。',
    footerMessage: '切换后需要重新打开终端里的 Codex 会话。',
    checkingBeforeSave: '正在检查 Key，检查通过后保存...',
    testingDraft: '正在测试 Key...',
    testPassed: 'Key 可用，已加载模型。',
    testFailed: 'Key 测试失败。',
    testBeforeAdd: '请先测试这个 Key，再添加。',
    savedValidated: 'Key 已保存并验证通过。',
    savingDetails: '正在保存信息...',
    detailsSaved: '信息已保存。',
    checkingKey: '正在检查 Key...',
    keyValid: 'Key 可用。',
    actionFailed: '操作失败。',
    switching: '正在切换 Codex Key 并写入配置...',
    switchedOk: '已切换，Codex 配置已更新并重启了 Desktop。',
    switchedManual: '已切换，Codex 配置已更新——请手动重启 Codex 生效。',
    deleteConfirm: '确定要从 Keydock 删除这个 Key 吗？',
    deletingKey: '正在删除 Key...',
    keyDeleted: 'Key 已删除。',
    missingName: '名称为必填项。',
    missingBaseUrl: 'Base URL 为必填项。',
    missingApiKey: 'API Key 为必填项。',
    codexConfigured: 'Codex 配置：{maskedKey} · {baseUrl} · {model}',
    codexNeedsSetup: '没有找到已配置的 Codex Key，请添加 API Key 开始使用。',
    currentConfiguredKey: '当前 Codex Key：{maskedKey}',
    noBridge: '桌面桥接不可用。请启动你构建出的 app，而不是浏览器预览页。',
    modelPlaceholder: '测试后加载模型',
    copyBaseUrl: '复制 Base URL',
    copyMaskedKey: '复制隐藏后的 Key',
    copied: '已复制。',
    noMatches: '没有匹配的 Key。'
  },
  ja: {
    appSubtitle: 'for Codex',
    addApiKey: 'APIキーを追加',
    noKeys: 'キーはまだありません。',
    emptyTitle: '最初のキーを追加',
    emptyHint: '保存済みキーがありません。まず 1 つ追加すると切り替えが楽になります。',
    activeBadge: '使用中',
    modelsBadge: '{count} 個のモデル',
    currentActive: '現在のキー',
    allKeys: 'すべてのキー',
    activeFromConfig: '~/.codex から検出',
    noActiveTitle: 'Codex キーが未設定です',
    language: '言語',
    searchKeys: 'キーを検索',
    searchPlaceholder: '名前、URL、モデルを検索',
    languageAuto: '自動',
    languageEnglish: 'English',
    languageChinese: '中文',
    languageJapanese: '日本語',
    name: '名前',
    baseUrl: 'Base URL',
    apiKey: 'APIキー',
    maskedKey: 'マスク済みキー',
    model: 'モデル',
    models: 'モデル',
    status: '状態',
    lastChecked: '最終確認',
    save: '保存',
    editKey: 'キーを編集',
    switchAction: '切替',
    check: '確認',
    edit: '編集',
    delete: '削除',
    cancel: 'キャンセル',
    add: '追加',
    test: 'テスト',
    never: '未実行',
    unavailable: '利用不可',
    available: '利用可能',
    activeInCodex: 'Codex で使用中',
    notChecked: '未確認',
    noModels: 'モデルはまだ取得されていません',
    keyWillBeChecked: 'キーをテストして利用可能なモデルを読み込み、その後 Keydock に追加します。',
    footerMessage: '切替後、ターミナルの Codex セッションは開き直してください。',
    checkingBeforeSave: '保存前にキーを確認しています...',
    testingDraft: 'キーをテストしています...',
    testPassed: 'キーは有効です。モデルを読み込みました。',
    testFailed: 'キーテストに失敗しました。',
    testBeforeAdd: '追加する前にこのキーをテストしてください。',
    savedValidated: 'キーを保存し、確認しました。',
    savingDetails: '詳細を保存しています...',
    detailsSaved: '詳細を保存しました。',
    checkingKey: 'キーを確認しています...',
    keyValid: 'キーは利用可能です。',
    actionFailed: '操作に失敗しました。',
    switching: 'Codex キーを切り替えて設定を書き込んでいます...',
    switchedOk: '切替しました。Codex 設定を更新し、Desktop を再起動しました。',
    switchedManual: '切替しました。Codex 設定を更新しました——反映には Codex を再起動してください。',
    deleteConfirm: 'このキーを Keydock から削除しますか？',
    deletingKey: 'キーを削除しています...',
    keyDeleted: 'キーを削除しました。',
    missingName: '名前は必須です。',
    missingBaseUrl: 'Base URL は必須です。',
    missingApiKey: 'APIキーは必須です。',
    codexConfigured: 'Codex 設定: {maskedKey} · {baseUrl} · {model}',
    codexNeedsSetup: '設定済みの Codex キーが見つかりません。API キーを追加してください。',
    currentConfiguredKey: '現在の Codex キー: {maskedKey}',
    noBridge: 'デスクトップブリッジが使えません。ブラウザプレビューではなく、ビルドした app を起動してください。',
    modelPlaceholder: 'テスト後にモデルを読み込み',
    copyBaseUrl: 'Base URLをコピー',
    copyMaskedKey: 'マスク済みキーをコピー',
    copied: 'コピーしました。',
    noMatches: '一致するキーはありません。'
  }
};

const $ = (id) => document.getElementById(id);

function systemLanguage() {
  const language = navigator.language || navigator.userLanguage || 'en';
  if (language.toLowerCase().startsWith('zh')) return 'zh';
  if (language.toLowerCase().startsWith('ja')) return 'ja';
  return 'en';
}

function languageMode() {
  return localStorage.getItem(STORAGE_LANGUAGE_KEY) || 'auto';
}

function currentLanguage() {
  const saved = languageMode();
  return SUPPORTED_LANGUAGES.includes(saved) ? saved : systemLanguage();
}

function t(key, values = {}) {
  const dictionary = translations[currentLanguage()] || translations.en;
  const template = dictionary[key] || translations.en[key] || key;
  return template.replace(/\{(\w+)\}/g, (_match, name) => values[name] ?? '');
}

function formatDate(value) {
  if (!value) return t('never');
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  return date.toLocaleString(currentLanguage());
}

function escapeHtml(value) {
  return String(value)
    .replaceAll('&', '&amp;')
    .replaceAll('<', '&lt;')
    .replaceAll('>', '&gt;')
    .replaceAll('"', '&quot;');
}

function applyStaticText() {
  document.documentElement.lang = currentLanguage();
  document.querySelectorAll('[data-i18n]').forEach((node) => {
    node.textContent = t(node.dataset.i18n);
  });
  document.querySelectorAll('[data-i18n-title]').forEach((node) => {
    node.title = t(node.dataset.i18nTitle);
  });
  document.querySelectorAll('[data-i18n-placeholder]').forEach((node) => {
    node.placeholder = t(node.dataset.i18nPlaceholder);
  });
  $('languageSelect').value = languageMode();
}

function requireBridge() {
  if (window.keydock && typeof window.keydock.addKey === 'function') return true;
  $('message').textContent = t('noBridge');
  return false;
}

function setBusy(value, message) {
  busy = value;
  $('busy').classList.toggle('hidden', !value);
  if (message) $('message').textContent = message;
  $('confirmAdd').disabled = value || !draftValidation?.valid;
  $('validateNewButton').disabled = value;
  $('addButton').disabled = value;
  document.querySelectorAll('.key-card button').forEach((node) => {
    node.disabled = value;
  });
}

function keyStatus(key) {
  if (!key) return '-';
  if (key.active) return t('activeInCodex');
  if (key.available) return t('available');
  if (key.lastValidatedAt) return t('unavailable');
  return t('notChecked');
}

function keyStatusClass(key) {
  if (!key) return 'idle';
  if (key.active || key.available) return 'ok';
  if (key.lastValidatedAt) return 'bad';
  return 'idle';
}

function activeKey() {
  return keys.find((key) => key.active) || null;
}

function renderActiveCard() {
  const card = $('activeCard');
  const active = activeKey();
  const profile = codexInfo?.codexProfile;

  if (active) {
    const modelText = active.model || t('noModels');
    card.className = `active-card ok`;
    card.innerHTML = `
      <div class="active-top">
        <div class="active-name">
          <span class="dot ok"></span>
          <strong>${escapeHtml(active.label || '-')}</strong>
          <span class="badge">${escapeHtml(t('activeBadge'))}</span>
        </div>
        <span class="active-status">${escapeHtml(keyStatus(active))}</span>
      </div>
      <div class="active-grid">
        <div><span>${escapeHtml(t('baseUrl'))}</span><code>${escapeHtml(active.baseUrl || DEFAULT_BASE_URL)}</code></div>
        <div><span>${escapeHtml(t('apiKey'))}</span><code>${escapeHtml(active.maskedKey || '-')}</code></div>
        <div><span>${escapeHtml(t('model'))}</span><code>${escapeHtml(modelText)}</code></div>
      </div>
      <div class="active-actions">
        <button type="button" data-copy="baseUrl">${escapeHtml(t('copyBaseUrl'))}</button>
        <button type="button" data-copy="maskedKey">${escapeHtml(t('copyMaskedKey'))}</button>
      </div>
    `;
    return;
  }

  if (profile?.configured) {
    card.className = 'active-card ok';
    card.innerHTML = `
      <div class="active-top">
        <div class="active-name">
          <span class="dot ok"></span>
          <strong>${escapeHtml(profile.label || 'Codex')}</strong>
          <span class="badge subtle">${escapeHtml(t('activeFromConfig'))}</span>
        </div>
      </div>
      <div class="active-grid">
        <div><span>${escapeHtml(t('baseUrl'))}</span><code>${escapeHtml(profile.baseUrl || DEFAULT_BASE_URL)}</code></div>
        <div><span>${escapeHtml(t('apiKey'))}</span><code>${escapeHtml(profile.maskedKey || '-')}</code></div>
        <div><span>${escapeHtml(t('model'))}</span><code>${escapeHtml(profile.model || t('noModels'))}</code></div>
      </div>
    `;
    return;
  }

  card.className = 'active-card empty';
  card.innerHTML = `
    <div class="active-empty">
      <span class="dot idle"></span>
      <div>
        <strong>${escapeHtml(t('noActiveTitle'))}</strong>
        <p>${escapeHtml(t('codexNeedsSetup'))}</p>
      </div>
      <button type="button" id="activeAddButton" class="primary">${escapeHtml(t('addApiKey'))}</button>
    </div>
  `;
  const addBtn = $('activeAddButton');
  if (addBtn) addBtn.addEventListener('click', openAddDialog);
}

function visibleKeys() {
  const query = filterText.trim().toLowerCase();
  if (!query) return keys;
  return keys.filter((key) => {
    const models = Array.isArray(key.models) ? key.models.join(' ') : '';
    return [key.label, key.baseUrl, key.maskedKey, key.model, models]
      .join(' ')
      .toLowerCase()
      .includes(query);
  });
}

function renderKeyGrid() {
  const grid = $('keyGrid');
  grid.innerHTML = '';
  const list = visibleKeys();
  $('emptyState').classList.toggle('hidden', keys.length !== 0);

  if (keys.length === 0) return;
  if (list.length === 0) {
    const empty = document.createElement('p');
    empty.className = 'muted-box';
    empty.textContent = t('noMatches');
    grid.append(empty);
    return;
  }

  for (const key of list) {
    const modelCount = Array.isArray(key.models) ? key.models.length : 0;
    const card = document.createElement('article');
    card.className = `key-card${key.active ? ' active' : ''}`;
    card.dataset.id = key.id;
    card.innerHTML = `
      <div class="key-card-head">
        <span class="dot ${keyStatusClass(key)}"></span>
        <strong>${escapeHtml(key.label || '-')}</strong>
        ${key.active ? `<span class="badge">${escapeHtml(t('activeBadge'))}</span>` : ''}
      </div>
      <code class="masked">${escapeHtml(key.maskedKey || '-')}</code>
      <div class="key-meta">
        <span class="url" title="${escapeHtml(key.baseUrl || '')}">${escapeHtml(key.baseUrl || DEFAULT_BASE_URL)}</span>
        <span class="model">${escapeHtml(key.model || (modelCount ? t('modelsBadge', { count: modelCount }) : '-'))}</span>
      </div>
      <div class="status-line ${keyStatusClass(key)}">${escapeHtml(keyStatus(key))}</div>
      <div class="card-actions">
        <button type="button" class="primary" data-act="switch">${escapeHtml(t('switchAction'))}</button>
        <button type="button" data-act="check">${escapeHtml(t('check'))}</button>
        <button type="button" data-act="edit">${escapeHtml(t('edit'))}</button>
        <button type="button" class="danger" data-act="delete">${escapeHtml(t('delete'))}</button>
      </div>
    `;
    grid.append(card);
  }
}

function render() {
  applyStaticText();
  renderActiveCard();
  renderKeyGrid();
}

function requireValue(id, messageKey) {
  const value = $(id).value.trim();
  if (!value) {
    $(id).focus();
    $('message').textContent = t(messageKey);
    return null;
  }
  return value;
}

async function refresh() {
  if (!requireBridge()) {
    keys = [];
    render();
    return;
  }
  keys = await window.keydock.listKeys();
  render();
}

async function withAction(message, action) {
  try {
    setBusy(true, message);
    const result = await action();
    await refresh();
    return result;
  } catch (error) {
    $('message').textContent = error.message || t('actionFailed');
    return null;
  } finally {
    setBusy(false);
  }
}

async function copyText(value) {
  const text = String(value || '').trim();
  if (!text) return;
  await navigator.clipboard.writeText(text);
  $('message').textContent = t('copied');
}

/* ---------- Add dialog ---------- */

function setModelOptions(select, models, selectedModel) {
  select.innerHTML = '';
  const normalized = Array.isArray(models) ? models : [];
  if (normalized.length === 0) {
    const option = document.createElement('option');
    option.value = '';
    option.textContent = t('modelPlaceholder');
    select.append(option);
    return;
  }
  for (const model of normalized) {
    const option = document.createElement('option');
    option.value = model;
    option.textContent = model;
    select.append(option);
  }
  select.value = normalized.includes(selectedModel) ? selectedModel : normalized[0];
}

function draftFingerprint() {
  return [$('newBaseUrl').value.trim(), $('newKey').value.trim()].join('\n');
}

function clearDraftValidation() {
  draftValidation = null;
  setModelOptions($('newModelField'), [], '');
  $('newModelStatus').textContent = t('modelPlaceholder');
  $('confirmAdd').disabled = true;
}

function setDraftModels(models, selectedModel = '') {
  setModelOptions($('newModelField'), models, selectedModel);
  const normalized = Array.isArray(models) ? models : [];
  $('newModelStatus').textContent = normalized.length > 0
    ? normalized.slice(0, 8).join(', ')
    : t('noModels');
}

async function testDraftKey() {
  const label = requireValue('newName', 'missingName');
  const baseUrl = requireValue('newBaseUrl', 'missingBaseUrl');
  const apiKey = requireValue('newKey', 'missingApiKey');
  if (!label || !baseUrl || !apiKey) return null;
  if (!requireBridge()) return null;
  const fingerprint = draftFingerprint();
  const result = await withAction(t('testingDraft'), () => window.keydock.testDraftKey({ baseUrl, apiKey }));
  if (!result?.valid) {
    clearDraftValidation();
    $('message').textContent = result?.message || t('testFailed');
    return null;
  }
  draftValidation = { ...result, fingerprint };
  setDraftModels(result.models || [], result.model || '');
  $('confirmAdd').disabled = false;
  $('message').textContent = t('testPassed');
  return result;
}

async function addDraftKey() {
  const label = requireValue('newName', 'missingName');
  const baseUrl = requireValue('newBaseUrl', 'missingBaseUrl');
  const apiKey = requireValue('newKey', 'missingApiKey');
  if (!label || !baseUrl || !apiKey) return null;
  if (!draftValidation?.valid || draftValidation.fingerprint !== draftFingerprint()) {
    $('message').textContent = t('testBeforeAdd');
    await testDraftKey();
    if (!draftValidation?.valid || draftValidation.fingerprint !== draftFingerprint()) return null;
  }
  if (!requireBridge()) return null;
  return withAction(t('checkingBeforeSave'), () => window.keydock.addKey({
    label,
    baseUrl,
    apiKey,
    model: $('newModelField').value,
    validation: draftValidation
  }));
}

function resetAddForm() {
  $('newName').value = 'OpenAI';
  $('newBaseUrl').value = DEFAULT_BASE_URL;
  $('newKey').value = '';
  clearDraftValidation();
}

function openAddDialog() {
  resetAddForm();
  $('addDialog').showModal();
  $('newKey').focus();
}

/* ---------- Edit dialog ---------- */

function openEditDialog(id) {
  const key = keys.find((item) => item.id === id);
  if (!key) return;
  editingId = id;
  $('editName').value = key.label || '';
  $('editBaseUrl').value = key.baseUrl || DEFAULT_BASE_URL;
  setModelOptions($('editModelField'), key.models, key.model);
  $('editDialog').showModal();
}

async function saveEdit() {
  if (!editingId) return;
  const label = requireValue('editName', 'missingName');
  const baseUrl = requireValue('editBaseUrl', 'missingBaseUrl');
  if (!label || !baseUrl) return;
  await withAction(t('savingDetails'), () => window.keydock.updateMetadata({
    id: editingId,
    label,
    baseUrl,
    model: $('editModelField').value
  }));
  $('message').textContent = t('detailsSaved');
  $('editDialog').close();
  editingId = null;
}

/* ---------- Card actions ---------- */

async function switchKey(id) {
  const result = await withAction(t('switching'), () => window.keydock.switchKey({ id }));
  if (result) {
    $('message').textContent = result.restarted ? t('switchedOk') : t('switchedManual');
  }
}

async function checkKey(id) {
  const result = await withAction(t('checkingKey'), () => window.keydock.validateKey({ id }));
  $('message').textContent = result?.valid ? t('keyValid') : (result?.message || t('actionFailed'));
}

async function deleteKey(id) {
  if (!confirm(t('deleteConfirm'))) return;
  await withAction(t('deletingKey'), () => window.keydock.deleteKey({ id }));
  $('message').textContent = t('keyDeleted');
}

/* ---------- Wiring ---------- */

$('languageSelect').addEventListener('change', () => {
  localStorage.setItem(STORAGE_LANGUAGE_KEY, $('languageSelect').value);
  render();
  loadDiagnostics();
});

$('searchField').addEventListener('input', () => {
  filterText = $('searchField').value;
  renderKeyGrid();
});

$('addButton').addEventListener('click', openAddDialog);
$('emptyAddButton').addEventListener('click', openAddDialog);
$('cancelAdd').addEventListener('click', () => $('addDialog').close());
$('cancelEdit').addEventListener('click', () => {
  $('editDialog').close();
  editingId = null;
});

$('validateNewButton').addEventListener('click', () => {
  testDraftKey();
});

$('confirmAdd').addEventListener('click', async (event) => {
  event.preventDefault();
  const record = await addDraftKey();
  if (record) {
    $('addDialog').close();
    $('message').textContent = t('savedValidated');
  }
});

$('saveEdit').addEventListener('click', (event) => {
  event.preventDefault();
  saveEdit();
});

for (const id of ['newBaseUrl', 'newKey']) {
  $(id).addEventListener('input', clearDraftValidation);
}

$('keyGrid').addEventListener('click', (event) => {
  const button = event.target.closest('button[data-act]');
  if (!button || busy) return;
  const card = button.closest('.key-card');
  const id = card?.dataset.id;
  if (!id) return;
  const action = button.dataset.act;
  if (action === 'switch') switchKey(id);
  else if (action === 'check') checkKey(id);
  else if (action === 'edit') openEditDialog(id);
  else if (action === 'delete') deleteKey(id);
});

$('activeCard').addEventListener('click', (event) => {
  const button = event.target.closest('button[data-copy]');
  if (!button) return;
  const active = activeKey();
  const field = button.dataset.copy;
  if (field === 'baseUrl') copyText(active?.baseUrl || codexInfo?.codexProfile?.baseUrl || '');
  else if (field === 'maskedKey') copyText(active?.maskedKey || codexInfo?.codexProfile?.maskedKey || '');
});

window.addEventListener('keydown', (event) => {
  if (!event.metaKey && !event.ctrlKey) return;
  const key = event.key.toLowerCase();
  if (key === 'k') {
    event.preventDefault();
    $('searchField').focus();
    $('searchField').select();
  } else if (key === 'n') {
    event.preventDefault();
    openAddDialog();
  }
});

async function loadDiagnostics() {
  if (!requireBridge()) return;
  codexInfo = await window.keydock.diagnostics();
  const profile = codexInfo?.codexProfile;
  if (profile?.configured) {
    $('message').textContent = t('codexConfigured', {
      maskedKey: profile.maskedKey || '-',
      baseUrl: profile.baseUrl || DEFAULT_BASE_URL,
      model: profile.model || '-'
    });
  } else if (keys.length === 0) {
    $('emptyHint').textContent = t('codexNeedsSetup');
    $('message').textContent = t('codexNeedsSetup');
  }
  render();
}

window.addEventListener('languagechange', () => {
  if (languageMode() === 'auto') render();
});

applyStaticText();
$('message').textContent = t('footerMessage');
refresh().then(loadDiagnostics);
