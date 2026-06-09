let keys = [];
let selectedId = null;
let busy = false;
let filterText = '';

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
    noKeySelected: 'No key selected',
    activeStation: 'Active key station',
    checkingCodex: 'Checking Codex CLI...',
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
    saveDetails: 'Save details',
    checkKey: 'Check key',
    switchRestart: 'Switch & restart',
    delete: 'Delete',
    cancel: 'Cancel',
    add: 'Add',
    never: 'Never',
    unavailable: 'Unavailable',
    available: 'Available',
    activeInCodex: 'Active in Codex',
    validated: 'Validated',
    notChecked: 'Not checked',
    noModels: 'No models reported yet',
    keyWillBeChecked: 'The key will be checked before it is saved.',
    footerMessage: 'Terminal Codex sessions must be reopened after switching.',
    checkingBeforeSave: 'Checking key before saving...',
    validateLoadModels: 'Validate & load models',
    savedValidated: 'Key saved and validated.',
    savingDetails: 'Saving details...',
    detailsSaved: 'Details saved.',
    checkingKey: 'Checking key...',
    keyValid: 'Key is valid.',
    actionFailed: 'Action failed.',
    switching: 'Switching Codex key and restarting Desktop...',
    switched: 'Switched. {status} Terminal Codex sessions must be reopened.',
    deleteConfirm: 'Delete this key from Keydock?',
    deletingKey: 'Deleting key...',
    keyDeleted: 'Key deleted.',
    missingName: 'Name is required.',
    missingBaseUrl: 'Base URL is required.',
    missingApiKey: 'API key is required.',
    codexCli: 'Codex CLI: {path} · Secrets: {encryption}',
    codexMissing: '{message} · Secrets: {encryption}',
    currentConfiguredKey: 'Current Codex key: {maskedKey}',
    noBridge: 'Desktop bridge is unavailable. Restart the app package you built, not the browser preview.',
    modelPlaceholder: 'Validate to load models',
    copyModels: 'Copy models',
    copyBaseUrl: 'Copy Base URL',
    copyMaskedKey: 'Copy masked key',
    copied: 'Copied.',
    noMatches: 'No matching keys.'
  },
  zh: {
    appSubtitle: 'for Codex',
    addApiKey: '添加 API Key',
    noKeys: '还没有 Key。',
    emptyTitle: '添加第一个 Key',
    emptyHint: '当前没有已保存的 Key，先添加一个，后面切换会轻松很多。',
    activeBadge: '当前',
    modelsBadge: '{count} 个模型',
    noKeySelected: '未选择 Key',
    activeStation: '当前 Key 工作台',
    checkingCodex: '正在检查 Codex CLI...',
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
    saveDetails: '保存信息',
    checkKey: '检查 Key',
    switchRestart: '切换并重启',
    delete: '删除',
    cancel: '取消',
    add: '添加',
    never: '从未',
    unavailable: '不可用',
    available: '可用',
    activeInCodex: 'Codex 当前使用',
    validated: '已验证',
    notChecked: '未检查',
    noModels: '还没有返回模型',
    keyWillBeChecked: '保存前会先检查 Key 是否可用。',
    footerMessage: '切换后需要重新打开终端里的 Codex 会话。',
    checkingBeforeSave: '正在检查 Key，检查通过后保存...',
    validateLoadModels: '验证并加载模型',
    savedValidated: 'Key 已保存并验证通过。',
    savingDetails: '正在保存信息...',
    detailsSaved: '信息已保存。',
    checkingKey: '正在检查 Key...',
    keyValid: 'Key 可用。',
    actionFailed: '操作失败。',
    switching: '正在切换 Codex Key 并重启 Desktop...',
    switched: '已切换。{status} 终端里的 Codex 会话需要重新打开。',
    deleteConfirm: '确定要从 Keydock 删除这个 Key 吗？',
    deletingKey: '正在删除 Key...',
    keyDeleted: 'Key 已删除。',
    missingName: '名称为必填项。',
    missingBaseUrl: 'Base URL 为必填项。',
    missingApiKey: 'API Key 为必填项。',
    codexCli: 'Codex CLI: {path} · 密钥存储: {encryption}',
    codexMissing: '{message} · 密钥存储: {encryption}',
    currentConfiguredKey: '当前 Codex Key：{maskedKey}',
    noBridge: '桌面桥接不可用。请启动你构建出的 app，而不是浏览器预览页。',
    modelPlaceholder: '验证后加载模型',
    copyModels: '复制模型',
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
    noKeySelected: 'キーが選択されていません',
    activeStation: 'アクティブキー',
    checkingCodex: 'Codex CLI を確認中...',
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
    saveDetails: '詳細を保存',
    checkKey: 'キーを確認',
    switchRestart: '切替して再起動',
    delete: '削除',
    cancel: 'キャンセル',
    add: '追加',
    never: '未実行',
    unavailable: '利用不可',
    available: '利用可能',
    activeInCodex: 'Codex で使用中',
    validated: '確認済み',
    notChecked: '未確認',
    noModels: 'モデルはまだ取得されていません',
    keyWillBeChecked: '保存前にキーを確認します。',
    footerMessage: '切替後、ターミナルの Codex セッションは開き直してください。',
    checkingBeforeSave: '保存前にキーを確認しています...',
    validateLoadModels: '確認してモデルを読み込み',
    savedValidated: 'キーを保存し、確認しました。',
    savingDetails: '詳細を保存しています...',
    detailsSaved: '詳細を保存しました。',
    checkingKey: 'キーを確認しています...',
    keyValid: 'キーは利用可能です。',
    actionFailed: '操作に失敗しました。',
    switching: 'Codex キーを切り替えて Desktop を再起動しています...',
    switched: '切替完了。{status} ターミナルの Codex セッションは開き直してください。',
    deleteConfirm: 'このキーを Keydock から削除しますか？',
    deletingKey: 'キーを削除しています...',
    keyDeleted: 'キーを削除しました。',
    missingName: '名前は必須です。',
    missingBaseUrl: 'Base URL は必須です。',
    missingApiKey: 'APIキーは必須です。',
    codexCli: 'Codex CLI: {path} · シークレット: {encryption}',
    codexMissing: '{message} · シークレット: {encryption}',
    currentConfiguredKey: '現在の Codex キー: {maskedKey}',
    noBridge: 'デスクトップブリッジが使えません。ブラウザプレビューではなく、ビルドした app を起動してください。',
    modelPlaceholder: '確認後にモデルを読み込み',
    copyModels: 'モデルをコピー',
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

function selectedKey() {
  return keys.find((key) => key.id === selectedId) || null;
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
  renderDetail();
}

function keyStatus(key) {
  if (!key) return '-';
  if (key.active) return t('activeInCodex');
  if (key.available) return t('available');
  if (key.lastValidatedAt) return t('unavailable');
  return t('notChecked');
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

function renderList() {
  $('keyList').innerHTML = '';
  const list = visibleKeys();
  $('emptyState').classList.toggle('hidden', keys.length !== 0);
  if (keys.length === 0 || list.length === 0) {
    const empty = document.createElement('p');
    empty.className = 'diag empty-list';
    empty.textContent = keys.length === 0 ? t('noKeys') : t('noMatches');
    $('keyList').append(empty);
    return;
  }
  for (const key of list) {
    const row = document.createElement('button');
    const modelCount = Array.isArray(key.models) ? key.models.length : 0;
    row.className = `key-row${key.id === selectedId ? ' selected' : ''}`;
    row.innerHTML = `
      <strong>
        <span>${escapeHtml(key.label || t('noKeySelected'))}</span>
        ${key.active ? `<span class="badge">${escapeHtml(t('activeBadge'))}</span>` : ''}
      </strong>
      <code>${escapeHtml(key.maskedKey || '')}</code>
      <small>${escapeHtml(key.baseUrl || DEFAULT_BASE_URL)}${modelCount ? ` · ${escapeHtml(t('modelsBadge', { count: modelCount }))}` : ''}</small>
    `;
    row.addEventListener('click', () => {
      selectedId = key.id;
      render();
    });
    $('keyList').append(row);
  }
}

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

function renderModelList(key) {
  $('modelList').innerHTML = '';
  const models = Array.isArray(key?.models) ? key.models : [];
  if (models.length === 0) {
    $('modelList').textContent = t('noModels');
    $('modelList').className = 'model-list muted-box';
    return;
  }
  $('modelList').className = 'model-list';
  for (const model of models.slice(0, 48)) {
    const chip = document.createElement('span');
    chip.textContent = model;
    $('modelList').append(chip);
  }
}

function renderDetail() {
  const key = selectedKey();
  const disabled = !key || busy;
  $('detailTitle').textContent = key ? key.label : t('noKeySelected');
  $('statusSummary').textContent = key ? keyStatus(key) : '-';
  $('baseUrlSummary').textContent = key?.baseUrl || '-';
  $('modelSummary').textContent = key?.model || '-';
  $('nameField').value = key?.label || '';
  $('baseUrlField').value = key?.baseUrl || DEFAULT_BASE_URL;
  $('maskedField').value = key?.maskedKey || '-';
  $('statusField').value = key ? keyStatus(key) : '-';
  $('checkedField').value = key ? formatDate(key.lastValidatedAt) : '-';
  setModelOptions($('modelField'), key?.models, key?.model);
  renderModelList(key);
  for (const id of ['nameField', 'baseUrlField', 'modelField', 'saveDetailsButton', 'checkButton', 'switchButton', 'deleteButton', 'copyBaseUrlButton', 'copyMaskedButton', 'copyModelsButton']) {
    $(id).disabled = disabled;
  }
  $('copyModelsButton').disabled = disabled || !Array.isArray(key?.models) || key.models.length === 0;
}

function render() {
  applyStaticText();
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
    selectedId = null;
    render();
    return;
  }
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

async function validateDraftKey() {
  const label = requireValue('newName', 'missingName');
  const baseUrl = requireValue('newBaseUrl', 'missingBaseUrl');
  const apiKey = requireValue('newKey', 'missingApiKey');
  if (!label || !baseUrl || !apiKey) return null;
  if (!requireBridge()) return null;
  const record = await withAction(t('checkingKey'), () => window.keydock.addKey({ label, baseUrl, apiKey }));
  if (!record) return null;
  $('newModelStatus').textContent = (record.models && record.models.length > 0)
    ? record.models.slice(0, 6).join(', ')
    : t('noModels');
  selectedId = record.id;
  await refresh();
  return record;
}

function resetAddForm() {
  $('newName').value = 'OpenAI';
  $('newBaseUrl').value = DEFAULT_BASE_URL;
  $('newKey').value = '';
  $('newModelStatus').textContent = t('modelPlaceholder');
}

$('languageSelect').addEventListener('change', () => {
  localStorage.setItem(STORAGE_LANGUAGE_KEY, $('languageSelect').value);
  $('message').textContent = t('footerMessage');
  render();
  loadDiagnostics();
});

$('searchField').addEventListener('input', () => {
  filterText = $('searchField').value;
  const list = visibleKeys();
  if (selectedId && !list.some((key) => key.id === selectedId)) {
    selectedId = list[0]?.id || selectedId;
  }
  render();
});

$('addButton').addEventListener('click', () => {
  resetAddForm();
  $('addDialog').showModal();
});

$('cancelAdd').addEventListener('click', () => {
  $('addDialog').close();
});

$('validateNewButton').addEventListener('click', async () => {
  const record = await validateDraftKey();
  if (record) {
    $('message').textContent = t('keyValid');
  }
});

$('confirmAdd').addEventListener('click', async (event) => {
  event.preventDefault();
  const record = await validateDraftKey();
  if (record) {
    $('addDialog').close();
    selectedId = record.id;
    $('message').textContent = t('savedValidated');
    await refresh();
  }
});

$('saveDetailsButton').addEventListener('click', async () => {
  if (!selectedId) return;
  const label = requireValue('nameField', 'missingName');
  const baseUrl = requireValue('baseUrlField', 'missingBaseUrl');
  if (!label || !baseUrl) return;
  await withAction(t('savingDetails'), () => window.keydock.updateMetadata({
    id: selectedId,
    label,
    baseUrl,
    model: $('modelField').value
  }));
  $('message').textContent = t('detailsSaved');
});

$('checkButton').addEventListener('click', async () => {
  if (!selectedId) return;
  const result = await withAction(t('checkingKey'), () => window.keydock.validateKey({ id: selectedId }));
  $('message').textContent = result?.valid ? t('keyValid') : (result?.message || t('actionFailed'));
});

$('switchButton').addEventListener('click', async () => {
  if (!selectedId) return;
  const result = await withAction(t('switching'), () => window.keydock.switchKey({ id: selectedId }));
  if (result) $('message').textContent = t('switched', { status: result.status });
});

$('deleteButton').addEventListener('click', async () => {
  if (!selectedId) return;
  if (!confirm(t('deleteConfirm'))) return;
  await withAction(t('deletingKey'), () => window.keydock.deleteKey({ id: selectedId }));
  selectedId = null;
  $('message').textContent = t('keyDeleted');
});

$('copyBaseUrlButton').addEventListener('click', () => {
  copyText(selectedKey()?.baseUrl || '');
});

$('copyMaskedButton').addEventListener('click', () => {
  copyText(selectedKey()?.maskedKey || '');
});

$('copyModelsButton').addEventListener('click', () => {
  copyText((selectedKey()?.models || []).join('\n'));
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
    resetAddForm();
    $('addDialog').showModal();
  } else if (key === 'enter' && selectedId && !busy) {
    event.preventDefault();
    $('switchButton').click();
  }
});

async function loadDiagnostics() {
  if (!requireBridge()) return;
  const info = await window.keydock.diagnostics();
  $('diag').textContent = info.codexPath
    ? t('codexCli', { path: info.codexPath, encryption: info.encryption })
    : t('codexMissing', { message: info.message, encryption: info.encryption });
  if (info.currentKey?.maskedKey && keys.length === 0) {
    $('emptyHint').textContent = t('currentConfiguredKey', { maskedKey: info.currentKey.maskedKey });
  }
}

window.addEventListener('languagechange', () => {
  if (languageMode() === 'auto') render();
});

applyStaticText();
$('message').textContent = t('footerMessage');
loadDiagnostics();
refresh();
