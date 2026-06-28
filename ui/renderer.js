let keys = [];
let busy = false;
let filterText = '';
let draftValidation = null;
let editValidation = null;
let editingId = null;
let codexInfo = null;

const tauriInvoke = window.__TAURI__?.core?.invoke || window.__TAURI__?.invoke;
if (!window.keydock && typeof tauriInvoke === 'function') {
  window.keydock = {
    listKeys: () => tauriInvoke('list_keys'),
    testDraftKey: ({ id, baseUrl, apiKey, model }) => tauriInvoke('test_draft_key', { id, baseUrl, apiKey, model }),
    addKey: ({ label, baseUrl, apiKey, model, validation }) => tauriInvoke('add_key', {
      label,
      baseUrl,
      apiKey,
      model,
      validation
    }),
    updateName: ({ id, label }) => tauriInvoke('update_name', { id, label }),
    exportKeys: () => tauriInvoke('export_keys'),
    importKeys: ({ content }) => tauriInvoke('import_keys', { content }),
    updateMetadata: ({ id, label, baseUrl, model, apiKey, validation }) => tauriInvoke('update_metadata', {
      id,
      label,
      baseUrl,
      model,
      apiKey,
      validation
    }),
    deleteKey: ({ id }) => tauriInvoke('delete_key', { id }),
    validateKey: ({ id }) => tauriInvoke('validate_key_cmd', { id }),
    validateKeyClient: ({ id, client }) => tauriInvoke('validate_key_client_cmd', { id, client }),
    switchKey: ({ id }) => tauriInvoke('switch_key', { id }),
    switchKeyClient: ({ id, client }) => tauriInvoke('switch_key_client', { id, client }),
    diagnostics: () => tauriInvoke('diagnostics')
  };
}

const STORAGE_LANGUAGE_KEY = 'keydock.language';
const DEFAULT_BASE_URL = 'https://api.openai.com/v1';
const SUPPORTED_LANGUAGES = ['en', 'zh', 'ja'];
const CLIENT_OPTIONS = [
  { id: 'codex', label: 'Codex', icon: './assets/clients/codex.png' },
  { id: 'openclaw', label: 'OpenClaw', icon: './assets/clients/openclaw.png' },
  { id: 'hermes', label: 'Hermes', icon: './assets/clients/hermes.png' }
];

const clientFilters = new Set();
const selectedClients = new Map();

const translations = {
  en: {
    appSubtitle: 'for Codex',
    addApiKey: 'Add API key',
    importKeys: 'Import',
    exportKeys: 'Export',
    exportEmpty: 'No keys to export.',
    exportDone: 'Exported {count} key(s) to {path}',
    exportCancelled: 'Export cancelled.',
    exportFailed: 'Export failed: {message}',
    importDone: 'Imported {added} key(s), skipped {skipped}.',
    importFailed: 'Import failed: {message}',
    noKeys: 'No keys yet.',
    emptyTitle: 'Add your first key',
    emptyHint: 'No saved keys were found. Add one to start switching quickly.',
    activeBadge: 'ACTIVE',
    modelsBadge: '{count} models',
    currentActive: 'Current key',
    allKeys: 'All keys',
    filterByClient: 'Filter keys by supported client',
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
    supportedClients: 'Supported clients',
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
    activeInCodexApp: 'In use · Codex app running',
    activeInCodexCli: 'In use · Codex CLI running',
    activeInCodexBoth: 'In use · Codex app + CLI running',
    activeInCodexConfig: 'In use · written to config',
    activeClientsRunning: 'In use · {clients} running',
    activeClientsConfig: 'In use · {clients} configured',
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
    checkingClient: 'Checking {client} support...',
    keyValid: 'Key is valid.',
    clientValid: '{client} is supported.',
    actionFailed: 'Action failed.',
    switching: 'Switching Codex key and writing config...',
    switchingClient: 'Switching {client} configuration...',
    switchClientTitle: 'Switch this key into {client}',
    selectClientTitle: 'Select {client} for switch/check',
    clientSupported: 'Supported',
    clientUnsupported: 'Not supported or not checked',
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
    modelInputPlaceholder: 'Select or type a model name',
    testPassedNoModels: 'Key is valid. No models were returned — type the model name manually.',
    showKey: 'Show key',
    hideKey: 'Hide key',
    copyBaseUrl: 'Copy Base URL',
    copyMaskedKey: 'Copy masked key',
    copied: 'Copied.',
    noMatches: 'No matching keys.',
    switchedRestarted: 'Switched. Config updated and the Codex client was restarted.',
    switchedNoClient: 'Switched. Codex config updated.',
    switchedClientRestarted: 'Switched. {client} config updated and service restarted.',
    switchedClientNoRestart: 'Switched. {client} config updated.',
    cliReminder: 'Codex CLI sessions only take effect after you exit and reopen the terminal.',
    editKeyPlaceholder: 'Leave blank to keep the current key',
    confirmTitle: 'Please confirm',
    detailTitle: 'Check details',
    close: 'Close',
    cannotDeleteActive: 'The key in use cannot be deleted. Switch to another key first.'
  },
  zh: {
    appSubtitle: 'for Codex',
    addApiKey: '添加 Key',
    importKeys: '导入',
    exportKeys: '导出',
    exportEmpty: '没有可导出的 Key。',
    exportDone: '已导出 {count} 个 Key 到 {path}',
    exportCancelled: '已取消导出。',
    exportFailed: '导出失败：{message}',
    importDone: '已导入 {added} 个，跳过 {skipped} 个。',
    importFailed: '导入失败：{message}',
    noKeys: '还没有 Key。',
    emptyTitle: '添加第一个 Key',
    emptyHint: '当前没有已保存的 Key，先添加一个，后面切换会轻松很多。',
    activeBadge: '当前',
    modelsBadge: '{count} 个模型',
    currentActive: '当前生效',
    allKeys: '所有 Key',
    filterByClient: '按支持的客户端过滤 Key',
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
    supportedClients: '支持的客户端',
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
    activeInCodexApp: '使用中 · Codex 应用运行中',
    activeInCodexCli: '使用中 · Codex CLI 运行中',
    activeInCodexBoth: '使用中 · Codex 应用 + CLI 运行中',
    activeInCodexConfig: '使用中 · 已写入配置',
    activeClientsRunning: '使用中 · {clients} 运行中',
    activeClientsConfig: '使用中 · 已写入 {clients} 配置',
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
    checkingClient: '正在检测 {client} 支持...',
    keyValid: 'Key 可用。',
    clientValid: '{client} 可用。',
    actionFailed: '操作失败。',
    switching: '正在切换 Codex Key 并写入配置...',
    switchingClient: '正在切换 {client} 配置...',
    switchClientTitle: '将这个 Key 切换到 {client}',
    selectClientTitle: '选择 {client} 作为切换/检测目标',
    clientSupported: '已支持',
    clientUnsupported: '未支持或未检测',
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
    modelInputPlaceholder: '选择或手动输入模型名',
    testPassedNoModels: 'Key 可用。未自动获取到模型，请手动输入模型名。',
    showKey: '显示明文',
    hideKey: '隐藏',
    copyBaseUrl: '复制 Base URL',
    copyMaskedKey: '复制隐藏后的 Key',
    copied: '已复制。',
    noMatches: '没有匹配的 Key。',
    switchedRestarted: '已切换，配置已更新并重启了 Codex 客户端。',
    switchedNoClient: '已切换，Codex 配置已更新。',
    switchedClientRestarted: '已切换，{client} 配置已更新并重启了服务。',
    switchedClientNoRestart: '已切换，{client} 配置已更新。',
    cliReminder: 'Codex CLI 需退出当前终端会话后重新进入才生效。',
    editKeyPlaceholder: '留空则保持当前 Key',
    confirmTitle: '请确认',
    detailTitle: '检测详情',
    close: '关闭',
    cannotDeleteActive: '当前使用的 Key 不能删除，请先切换到其它 Key。'
  },
  ja: {
    appSubtitle: 'for Codex',
    addApiKey: 'APIキーを追加',
    importKeys: 'インポート',
    exportKeys: 'エクスポート',
    exportEmpty: 'エクスポートする Key がありません。',
    exportDone: '{count} 件を {path} にエクスポートしました。',
    exportCancelled: 'エクスポートをキャンセルしました。',
    exportFailed: 'エクスポート失敗：{message}',
    importDone: '{added} 件をインポート、{skipped} 件をスキップしました。',
    importFailed: 'インポート失敗：{message}',
    noKeys: 'キーはまだありません。',
    emptyTitle: '最初のキーを追加',
    emptyHint: '保存済みキーがありません。まず 1 つ追加すると切り替えが楽になります。',
    activeBadge: '使用中',
    modelsBadge: '{count} 個のモデル',
    currentActive: '現在のキー',
    allKeys: 'すべてのキー',
    filterByClient: '対応クライアントでキーを絞り込み',
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
    supportedClients: '対応クライアント',
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
    activeInCodexApp: '使用中 · Codex アプリ起動中',
    activeInCodexCli: '使用中 · Codex CLI 実行中',
    activeInCodexBoth: '使用中 · Codex アプリ + CLI 実行中',
    activeInCodexConfig: '使用中 · 設定に書き込み済み',
    activeClientsRunning: '使用中 · {clients} 実行中',
    activeClientsConfig: '使用中 · {clients} 設定済み',
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
    checkingClient: '{client} 対応を確認しています...',
    keyValid: 'キーは利用可能です。',
    clientValid: '{client} は対応しています。',
    actionFailed: '操作に失敗しました。',
    switching: 'Codex キーを切り替えて設定を書き込んでいます...',
    switchingClient: '{client} の設定を切り替えています...',
    switchClientTitle: 'このキーを {client} に切り替え',
    selectClientTitle: '切替/確認対象に {client} を選択',
    clientSupported: '対応済み',
    clientUnsupported: '未対応または未確認',
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
    modelInputPlaceholder: 'モデルを選択または入力',
    testPassedNoModels: 'キーは有効です。モデルが取得できませんでした。モデル名を手動で入力してください。',
    showKey: 'キーを表示',
    hideKey: 'キーを隠す',
    copyBaseUrl: 'Base URLをコピー',
    copyMaskedKey: 'マスク済みキーをコピー',
    copied: 'コピーしました。',
    noMatches: '一致するキーはありません。',
    switchedRestarted: '切替しました。設定を更新し、Codex クライアントを再起動しました。',
    switchedNoClient: '切替しました。Codex 設定を更新しました。',
    switchedClientRestarted: '切替しました。{client} 設定を更新し、サービスを再起動しました。',
    switchedClientNoRestart: '切替しました。{client} 設定を更新しました。',
    cliReminder: 'Codex CLI はターミナルのセッションを終了して開き直すと反映されます。',
    editKeyPlaceholder: '空欄なら現在のキーを保持します',
    confirmTitle: '確認してください',
    detailTitle: 'チェック詳細',
    close: '閉じる',
    cannotDeleteActive: '使用中のキーは削除できません。先に別のキーへ切り替えてください。'
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

function errorText(error, fallbackKey = 'actionFailed') {
  if (typeof error === 'string') return error.trim() || t(fallbackKey);
  if (error?.message) return String(error.message).trim() || t(fallbackKey);
  if (error && typeof error === 'object') {
    try {
      const serialized = JSON.stringify(error);
      if (serialized && serialized !== '{}') return serialized;
    } catch (_ignored) {
      // Fall through to the localized fallback.
    }
  }
  return t(fallbackKey);
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

function normalizeClientIds(values, fallback = []) {
  const allowed = new Set(CLIENT_OPTIONS.map((option) => option.id));
  const seen = new Set();
  const source = Array.isArray(values) ? values : fallback;
  const normalized = [];
  for (const value of source) {
    const id = String(value || '').trim().toLowerCase();
    if (!allowed.has(id) || seen.has(id)) continue;
    seen.add(id);
    normalized.push(id);
  }
  return normalized;
}

function clientOption(id) {
  return CLIENT_OPTIONS.find((option) => option.id === id) || null;
}

function supportedClientIdsForKey(key) {
  if (!String(key?.clientSupportCheckedAt || '').trim()) return [];
  return normalizeClientIds(key?.supportedClients);
}

function clientIdsForKey(key) {
  return supportedClientIdsForKey(key);
}

function activeClientIdsForKey(key) {
  return normalizeClientIds(key?.activeClients);
}

function runningClientIdsForKey(key) {
  return normalizeClientIds(key?.runningClients);
}

function clientLabels(ids) {
  return normalizeClientIds(ids)
    .map((id) => clientOption(id)?.label || id)
    .join(' / ');
}

function selectedClientForKey(key) {
  const explicit = normalizeClientIds([selectedClients.get(key?.id)])[0];
  if (explicit) return explicit;
  const active = activeClientIdsForKey(key)[0];
  if (active) return active;
  const supported = supportedClientIdsForKey(key)[0];
  if (supported) return supported;
  return CLIENT_OPTIONS[0].id;
}

function setSelectedClientForKey(id, client) {
  const normalized = normalizeClientIds([client])[0];
  if (!id || !normalized) return;
  selectedClients.set(id, normalized);
  render();
}

function pruneSelectedClients() {
  const ids = new Set(keys.map((key) => key.id));
  for (const id of selectedClients.keys()) {
    if (!ids.has(id)) selectedClients.delete(id);
  }
}

function renderClientIcons(key, { interactive = true } = {}) {
  const supportedIds = new Set(supportedClientIdsForKey(key));
  const selectedIds = new Set(activeClientIdsForKey(key));
  const selectedClient = selectedClientForKey(key);
  const icons = CLIENT_OPTIONS
    .map((option) => {
      const supported = supportedIds.has(option.id);
      const active = selectedIds.has(option.id);
      const chosen = interactive && selectedClient === option.id;
      const title = `${t('selectClientTitle', { client: option.label })} · ${supported ? t('clientSupported') : t('clientUnsupported')}`;
      const classes = [
        'client-icon',
        option.id,
        supported ? 'supported' : 'unsupported',
        active ? 'active-client' : '',
        chosen ? 'chosen' : ''
      ].filter(Boolean).join(' ');
      if (!interactive) {
        return `
          <span
            class="${escapeHtml(classes)}"
            title="${escapeHtml(title)}"
            aria-label="${escapeHtml(title)}"
            role="img"
          >
            <img src="${escapeHtml(option.icon)}" alt="" aria-hidden="true">
          </span>
        `;
      }
      return `
        <button
          type="button"
          class="${escapeHtml(classes)}"
          data-act="select-client"
          data-client="${escapeHtml(option.id)}"
          title="${escapeHtml(title)}"
          aria-label="${escapeHtml(title)}"
          aria-pressed="${chosen ? 'true' : 'false'}"
        >
          <img src="${escapeHtml(option.icon)}" alt="" aria-hidden="true">
        </button>
      `;
    })
    .join('');
  const labels = CLIENT_OPTIONS
    .map((option) => `${option.label}: ${supportedIds.has(option.id) ? t('clientSupported') : t('clientUnsupported')}`)
    .join(', ');
  return `<div class="client-icons" aria-label="${escapeHtml(t('supportedClients'))}: ${escapeHtml(labels)}">${icons}</div>`;
}

function renderClientFilterBar() {
  const bar = $('clientFilterBar');
  if (!bar) return;
  const allButton = $('allKeysFilter');
  if (allButton) {
    const allSelected = clientFilters.size === 0;
    allButton.classList.toggle('selected', allSelected);
    allButton.setAttribute('aria-pressed', allSelected ? 'true' : 'false');
  }
  bar.setAttribute('aria-label', t('filterByClient'));
  bar.innerHTML = CLIENT_OPTIONS.map((option) => {
    const selected = clientFilters.has(option.id);
    return `
      <button
        type="button"
        class="client-filter-btn${selected ? ' selected' : ''}"
        data-client="${escapeHtml(option.id)}"
        aria-pressed="${selected ? 'true' : 'false'}"
        title="${escapeHtml(option.label)}"
        aria-label="${escapeHtml(option.label)}"
      >
        <img src="${escapeHtml(option.icon)}" alt="" aria-hidden="true">
      </button>
    `;
  }).join('');
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
  document.querySelectorAll('.key-card button, .active-card button').forEach((node) => {
    node.disabled = value;
  });
}

function keyStatus(key) {
  if (!key) return '-';
  if (activeClientIdsForKey(key).length > 0) return activeUsageLabel(key);
  if (key.active) return activeUsageLabel(key);
  if (key.available) return t('available');
  if (key.lastValidatedAt) return t('unavailable');
  return t('notChecked');
}

// Describe where the active key is currently consumed, based on which Codex
// surfaces are actually running on this machine. Falls back to "written to
// config" when nothing is detected running.
function activeUsageLabel(key = null) {
  const activeIds = activeClientIdsForKey(key);
  if (activeIds.length > 0) {
    const runningIds = runningClientIdsForKey(key);
    const clients = clientLabels(runningIds.length > 0 ? runningIds : activeIds);
    return t(runningIds.length > 0 ? 'activeClientsRunning' : 'activeClientsConfig', { clients });
  }
  const appRunning = !!codexInfo?.codexDesktopRunning;
  const cliRunning = !!codexInfo?.codexCliRunning;
  if (appRunning && cliRunning) return t('activeInCodexBoth');
  if (appRunning) return t('activeInCodexApp');
  if (cliRunning) return t('activeInCodexCli');
  return t('activeInCodexConfig');
}

function keyStatusClass(key) {
  if (!key) return 'idle';
  if (key.active || activeClientIdsForKey(key).length > 0 || key.available) return 'ok';
  if (key.lastValidatedAt) return 'bad';
  return 'idle';
}

function activeKey() {
  return keys.find((key) => key.active) || keys.find((key) => activeClientIdsForKey(key).length > 0) || null;
}

function renderActiveCard() {
  const card = $('activeCard');
  const active = activeKey();
  const profile = codexInfo?.codexProfile;

  if (active) {
    const modelText = active.model || t('noModels');
    const clientIcons = renderClientIcons(active, { interactive: false });
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
      <div class="active-client-row">${clientIcons}</div>
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
  const selectedClients = Array.from(clientFilters);
  return keys.filter((key) => {
    const ids = clientIdsForKey(key);
    if (selectedClients.length > 0 && !selectedClients.every((id) => ids.includes(id))) {
      return false;
    }
    if (!query) return true;
    const models = Array.isArray(key.models) ? key.models.join(' ') : '';
    const clients = ids.map((id) => clientOption(id)?.label || id).join(' ');
    return [key.label, key.baseUrl, key.maskedKey, key.model, models, clients]
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
    const statusClass = keyStatusClass(key);
    const showDetail = statusClass === 'bad' && !!(key.validationMessage || '').trim();
    const inUse = key.active || activeClientIdsForKey(key).length > 0;
    const card = document.createElement('article');
    card.className = `key-card${inUse ? ' active' : ''}`;
    card.dataset.id = key.id;
    card.innerHTML = `
      <div class="key-card-head">
        <span class="dot ${statusClass}"></span>
        <strong>${escapeHtml(key.label || '-')}</strong>
        ${inUse ? `<span class="badge">${escapeHtml(t('activeBadge'))}</span>` : ''}
      </div>
      <code class="masked">${escapeHtml(key.maskedKey || '-')}</code>
      <div class="key-meta">
        <span class="url" title="${escapeHtml(key.baseUrl || '')}">${escapeHtml(key.baseUrl || DEFAULT_BASE_URL)}</span>
        <span class="model">${escapeHtml(key.model || (modelCount ? t('modelsBadge', { count: modelCount }) : '-'))}</span>
      </div>
      ${renderClientIcons(key)}
      <div class="status-line ${statusClass}">
        <span class="status-text">${escapeHtml(keyStatus(key))}</span>
        ${showDetail ? `<button type="button" class="detail-btn" data-act="detail" title="${escapeHtml(t('detailTitle'))}" aria-label="${escapeHtml(t('detailTitle'))}">&#9432;</button>` : ''}
      </div>
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
  renderClientFilterBar();
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
  pruneSelectedClients();
  render();
}

async function withAction(message, action) {
  try {
    setBusy(true, message);
    const result = await action();
    await refresh();
    return result;
  } catch (error) {
    $('message').textContent = errorText(error);
    return null;
  } finally {
    setBusy(false);
  }
}

async function exportKeysToFile() {
  if (!requireBridge()) return;
  try {
    setBusy(true, t('exportKeys'));
    // The backend writes the file into the Downloads folder directly because
    // blob downloads do not work inside the Tauri webview.
    const result = await window.keydock.exportKeys();
    if (!result || result.count === 0) {
      $('message').textContent = t('exportEmpty');
      return;
    }
    if (result.cancelled) {
      $('message').textContent = t('exportCancelled');
      return;
    }
    $('message').textContent = t('exportDone', { count: result.count, path: result.path });
  } catch (error) {
    $('message').textContent = t('exportFailed', { message: errorText(error) });
  } finally {
    setBusy(false);
  }
}

async function importKeysFromFile(event) {
  const input = event.target;
  const file = input.files && input.files[0];
  // Reset so selecting the same file again still fires `change`.
  input.value = '';
  if (!file) return;
  if (!requireBridge()) return;
  try {
    setBusy(true, t('importKeys'));
    const content = await file.text();
    const summary = await window.keydock.importKeys({ content });
    await refresh();
    await loadDiagnostics();
    $('message').textContent = t('importDone', {
      added: summary?.added ?? 0,
      skipped: summary?.skipped ?? 0
    });
  } catch (error) {
    $('message').textContent = t('importFailed', { message: errorText(error) });
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

/* ---------- Model combobox ---------- */

const comboModels = new WeakMap();

function comboMenu(combo) {
  return combo.querySelector('.combo-menu');
}

function comboInput(combo) {
  return combo.querySelector('.combo-input');
}

function renderComboMenu(combo) {
  const menu = comboMenu(combo);
  const input = comboInput(combo);
  if (!menu || !input) return;
  const models = comboModels.get(combo) || [];
  const filter = (input.value || '').trim().toLowerCase();
  const matches = filter ? models.filter((model) => model.toLowerCase().includes(filter)) : models;
  menu.innerHTML = '';
  if (matches.length === 0) {
    const empty = document.createElement('li');
    empty.className = 'combo-empty';
    empty.textContent = t('noModels');
    menu.append(empty);
    return;
  }
  for (const model of matches) {
    const option = document.createElement('li');
    option.className = 'combo-option';
    option.textContent = model;
    option.dataset.value = model;
    if (model === input.value) option.classList.add('selected');
    menu.append(option);
  }
}

function closeAllCombos(except) {
  document.querySelectorAll('.combo').forEach((combo) => {
    if (combo === except) return;
    const menu = comboMenu(combo);
    if (menu) menu.classList.add('hidden');
    combo.classList.remove('open');
  });
}

function openCombo(combo) {
  closeAllCombos(combo);
  renderComboMenu(combo);
  comboMenu(combo)?.classList.remove('hidden');
  combo.classList.add('open');
}

function closeCombo(combo) {
  comboMenu(combo)?.classList.add('hidden');
  combo.classList.remove('open');
}

function initCombo(combo) {
  if (!combo) return;
  const input = comboInput(combo);
  const toggle = combo.querySelector('.combo-toggle');
  const menu = comboMenu(combo);
  if (!input || !toggle || !menu) return;
  toggle.addEventListener('click', (event) => {
    event.preventDefault();
    if (combo.classList.contains('open')) {
      closeCombo(combo);
    } else {
      openCombo(combo);
      input.focus();
    }
  });
  input.addEventListener('click', () => openCombo(combo));
  input.addEventListener('input', () => openCombo(combo));
  // Use mousedown so the value is set before the input loses focus.
  menu.addEventListener('mousedown', (event) => {
    const option = event.target.closest('.combo-option');
    if (!option) return;
    event.preventDefault();
    input.value = option.dataset.value || '';
    closeCombo(combo);
  });
}

function setModelOptions(input, models, selectedModel) {
  const normalized = Array.isArray(models) ? models.filter(Boolean) : [];
  const combo = input.closest('.combo');
  if (combo) comboModels.set(combo, normalized);
  if (selectedModel && (normalized.length === 0 || normalized.includes(selectedModel))) {
    input.value = selectedModel;
  } else if (normalized.length > 0) {
    input.value = normalized[0];
  } else {
    input.value = selectedModel || '';
  }
  if (combo) renderComboMenu(combo);
}

/* ---------- Add dialog ---------- */

function draftFingerprint() {
  return [$('newBaseUrl').value.trim(), $('newKey').value.trim(), $('newModelField').value.trim()].join('\n');
}

function setAddStatus(text, state = '') {
  const node = $('addStatus');
  if (!node) return;
  if (!text) {
    node.textContent = '';
    node.className = 'dialog-status hidden';
    return;
  }
  node.textContent = text;
  node.className = `dialog-status${state ? ` ${state}` : ''}`;
}

function clearDraftValidation() {
  draftValidation = null;
  setModelOptions($('newModelField'), [], '');
  setAddStatus('');
  $('confirmAdd').disabled = true;
}

function setDraftModels(models, selectedModel = '') {
  setModelOptions($('newModelField'), models, selectedModel);
}

async function testDraftKey() {
  if (!requireBridge()) {
    setAddStatus(t('noBridge'), 'bad');
    return null;
  }
  const label = $('newName').value.trim();
  const baseUrl = $('newBaseUrl').value.trim();
  const apiKey = $('newKey').value.trim();
  if (!label) {
    $('newName').focus();
    setAddStatus(t('missingName'), 'bad');
    return null;
  }
  if (!baseUrl) {
    $('newBaseUrl').focus();
    setAddStatus(t('missingBaseUrl'), 'bad');
    return null;
  }
  if (!apiKey) {
    $('newKey').focus();
    setAddStatus(t('missingApiKey'), 'bad');
    return null;
  }
  setAddStatus(t('testingDraft'), 'pending');
  // Probe with exactly what is typed in the form; do not refresh the list (that
  // would trigger a profile sync and is unrelated to testing a draft key).
  try {
    setBusy(true, t('testingDraft'));
    const model = $('newModelField').value.trim();
    const result = await window.keydock.testDraftKey({ baseUrl, apiKey, model });
    if (!result?.valid) {
      clearDraftValidation();
      setAddStatus(result?.message || t('testFailed'), 'bad');
      return null;
    }
    setDraftModels(result.models || [], result.model || '');
    draftValidation = { ...result, fingerprint: draftFingerprint() };
    const modelCount = Array.isArray(result.models) ? result.models.length : 0;
    setAddStatus(
      modelCount > 0 ? `${t('testPassed')} (${t('modelsBadge', { count: modelCount })})` : t('testPassedNoModels'),
      'ok'
    );
    return result;
  } catch (error) {
    clearDraftValidation();
    setAddStatus(errorText(error, 'testFailed'), 'bad');
    return null;
  } finally {
    setBusy(false);
  }
}

async function addDraftKey() {
  const label = requireValue('newName', 'missingName');
  const baseUrl = requireValue('newBaseUrl', 'missingBaseUrl');
  const apiKey = requireValue('newKey', 'missingApiKey');
  if (!label || !baseUrl || !apiKey) return null;
  if (!draftValidation?.valid || draftValidation.fingerprint !== draftFingerprint()) {
    setAddStatus(t('testBeforeAdd'), 'pending');
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
  $('newKey').type = 'password';
  updateRevealIcon();
  clearDraftValidation();
}

function openAddDialog() {
  resetAddForm();
  $('addDialog').showModal();
  $('newKey').focus();
}

/* ---------- Edit dialog ---------- */

function setEditStatus(text, state = '') {
  const node = $('editStatus');
  if (!node) return;
  if (!text) {
    node.textContent = '';
    node.className = 'dialog-status hidden';
    return;
  }
  node.textContent = text;
  node.className = `dialog-status${state ? ` ${state}` : ''}`;
}

function editFingerprint() {
  return [
    editingId || '',
    $('editBaseUrl').value.trim(),
    $('editKey').value.trim(),
    $('editModelField').value.trim()
  ].join('\n');
}

function clearEditValidation() {
  editValidation = null;
}

function openEditDialog(id) {
  const key = keys.find((item) => item.id === id);
  if (!key) return;
  editingId = id;
  $('editName').value = key.label || '';
  $('editBaseUrl').value = key.baseUrl || DEFAULT_BASE_URL;
  $('editKey').value = '';
  $('editKey').type = 'password';
  updateEditRevealIcon();
  setModelOptions($('editModelField'), key.models, key.model);
  clearEditValidation();
  setEditStatus('');
  $('editDialog').showModal();
}

async function testEditKey() {
  if (!editingId) return null;
  if (!requireBridge()) {
    setEditStatus(t('noBridge'), 'bad');
    return null;
  }
  const baseUrl = $('editBaseUrl').value.trim();
  const apiKey = $('editKey').value.trim();
  if (!baseUrl) {
    $('editBaseUrl').focus();
    setEditStatus(t('missingBaseUrl'), 'bad');
    return null;
  }
  setEditStatus(t('testingDraft'), 'pending');
  // Probe with the values currently typed in the edit form (a blank API key
  // falls back to the stored secret). Do not refresh the list while testing.
  try {
    setBusy(true, t('testingDraft'));
    const model = $('editModelField').value.trim();
    const result = await window.keydock.testDraftKey({ id: editingId, baseUrl, apiKey, model });
    if (!result?.valid) {
      clearEditValidation();
      setEditStatus(result?.message || t('testFailed'), 'bad');
      return null;
    }
    setModelOptions($('editModelField'), result.models || [], $('editModelField').value || result.model || '');
    editValidation = { ...result, fingerprint: editFingerprint() };
    const modelCount = Array.isArray(result.models) ? result.models.length : 0;
    setEditStatus(
      modelCount > 0 ? `${t('testPassed')} (${t('modelsBadge', { count: modelCount })})` : t('testPassedNoModels'),
      'ok'
    );
    return result;
  } catch (error) {
    clearEditValidation();
    setEditStatus(errorText(error, 'testFailed'), 'bad');
    return null;
  } finally {
    setBusy(false);
  }
}

async function saveEdit() {
  if (!editingId) return;
  const label = requireValue('editName', 'missingName');
  const baseUrl = requireValue('editBaseUrl', 'missingBaseUrl');
  if (!label || !baseUrl) return;
  const apiKey = $('editKey').value.trim();
  const validation = editValidation?.valid && editValidation.fingerprint === editFingerprint()
    ? editValidation
    : undefined;
  const record = await withAction(t('savingDetails'), () => window.keydock.updateMetadata({
    id: editingId,
    label,
    baseUrl,
    model: $('editModelField').value,
    apiKey: apiKey || undefined,
    validation
  }));
  if (!record) return;
  $('message').textContent = record.active ? `${t('detailsSaved')} ${t('cliReminder')}` : t('detailsSaved');
  $('editDialog').close();
  editingId = null;
  clearEditValidation();
}

/* ---------- Detail & confirm dialogs ---------- */

function openDetailDialog(id) {
  const key = keys.find((item) => item.id === id);
  const message = (key?.validationMessage || '').trim() || t('notChecked');
  $('detailBody').textContent = message;
  $('detailDialog').showModal();
}

function customConfirm(message, confirmLabel) {
  return new Promise((resolve) => {
    const dialog = $('confirmDialog');
    $('confirmMessage').textContent = message;
    if (confirmLabel) $('confirmOk').textContent = confirmLabel;
    let decided = false;
    const finish = (result) => {
      if (decided) return;
      decided = true;
      $('confirmOk').removeEventListener('click', onOk);
      $('confirmCancel').removeEventListener('click', onCancel);
      dialog.removeEventListener('cancel', onCancelEvent);
      if (dialog.open) dialog.close();
      resolve(result);
    };
    const onOk = () => finish(true);
    const onCancel = () => finish(false);
    const onCancelEvent = (event) => {
      event.preventDefault();
      finish(false);
    };
    $('confirmOk').addEventListener('click', onOk);
    $('confirmCancel').addEventListener('click', onCancel);
    dialog.addEventListener('cancel', onCancelEvent);
    dialog.showModal();
  });
}

/* ---------- Card actions ---------- */

async function switchKeyClient(id, client) {
  const option = clientOption(client);
  if (!option) return;
  const result = await withAction(
    client === 'codex' ? t('switching') : t('switchingClient', { client: option.label }),
    () => window.keydock.switchKeyClient({ id, client })
  );
  if (result) {
    if (client === 'codex') {
      let head;
      if (result.warning) head = result.warning;
      else if (result.restarted) head = t('switchedRestarted');
      else head = t('switchedNoClient');
      $('message').textContent = `${head} ${t('cliReminder')}`;
      return;
    }
    const head = result.warning
      ? result.warning
      : result.restarted
        ? t('switchedClientRestarted', { client: option.label })
        : t('switchedClientNoRestart', { client: option.label });
    $('message').textContent = head;
  }
}

async function checkKeyClient(id, client) {
  const option = clientOption(client);
  if (!option) return;
  const result = await withAction(
    t('checkingClient', { client: option.label }),
    () => window.keydock.validateKeyClient({ id, client })
  );
  $('message').textContent = result?.supportedClients?.includes(client)
    ? t('clientValid', { client: option.label })
    : (result?.message || t('actionFailed'));
}

async function deleteKey(id) {
  const key = keys.find((item) => item.id === id);
  if (key?.active || activeClientIdsForKey(key).length > 0) {
    $('message').textContent = t('cannotDeleteActive');
    return;
  }
  const confirmed = await customConfirm(t('deleteConfirm'), t('delete'));
  if (!confirmed) return;
  await withAction(t('deletingKey'), () => window.keydock.deleteKey({ id }));
  $('message').textContent = t('keyDeleted');
}

/* ---------- Wiring ---------- */

const EYE_OPEN = '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z"/><circle cx="12" cy="12" r="3"/></svg>';
const EYE_OFF = '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M17.94 17.94A10.07 10.07 0 0 1 12 20c-7 0-11-8-11-8a18.45 18.45 0 0 1 5.06-5.94M9.9 4.24A9.12 9.12 0 0 1 12 4c7 0 11 8 11 8a18.5 18.5 0 0 1-2.16 3.19m-6.72-1.07a3 3 0 1 1-4.24-4.24"/><line x1="1" y1="1" x2="23" y2="23"/></svg>';

function updateRevealIconFor(inputId, buttonId) {
  const input = $(inputId);
  const button = $(buttonId);
  if (!input || !button) return;
  const revealed = input.type === 'text';
  button.innerHTML = revealed ? EYE_OFF : EYE_OPEN;
  const label = t(revealed ? 'hideKey' : 'showKey');
  button.title = label;
  button.setAttribute('aria-label', label);
}

function updateRevealIcon() {
  updateRevealIconFor('newKey', 'toggleNewKey');
}

function updateEditRevealIcon() {
  updateRevealIconFor('editKey', 'toggleEditKey');
}

$('toggleNewKey').addEventListener('click', () => {
  const input = $('newKey');
  input.type = input.type === 'password' ? 'text' : 'password';
  updateRevealIcon();
  input.focus();
});

$('toggleEditKey').addEventListener('click', () => {
  const input = $('editKey');
  input.type = input.type === 'password' ? 'text' : 'password';
  updateEditRevealIcon();
  input.focus();
});

$('languageSelect').addEventListener('change', () => {
  localStorage.setItem(STORAGE_LANGUAGE_KEY, $('languageSelect').value);
  render();
  loadDiagnostics();
});

$('searchField').addEventListener('input', () => {
  filterText = $('searchField').value;
  renderKeyGrid();
});

$('allKeysFilter').addEventListener('click', () => {
  clientFilters.clear();
  render();
});

$('clientFilterBar').addEventListener('click', (event) => {
  const button = event.target.closest('button[data-client]');
  if (!button) return;
  const id = button.dataset.client;
  if (clientFilters.has(id)) {
    clientFilters.clear();
  } else {
    clientFilters.clear();
    clientFilters.add(id);
  }
  render();
});

$('addButton').addEventListener('click', openAddDialog);
$('emptyAddButton').addEventListener('click', openAddDialog);
$('exportButton').addEventListener('click', exportKeysToFile);
$('importButton').addEventListener('click', () => $('importFileInput').click());
$('importFileInput').addEventListener('change', importKeysFromFile);
$('cancelAdd').addEventListener('click', () => $('addDialog').close());
$('cancelEdit').addEventListener('click', () => {
  $('editDialog').close();
  editingId = null;
  clearEditValidation();
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

$('validateEditButton').addEventListener('click', () => {
  testEditKey();
});

$('detailClose').addEventListener('click', () => {
  $('detailDialog').close();
});

for (const id of ['newBaseUrl', 'newKey', 'newModelField']) {
  $(id).addEventListener('input', clearDraftValidation);
}

for (const id of ['editBaseUrl', 'editKey', 'editModelField']) {
  $(id).addEventListener('input', clearEditValidation);
}

$('keyGrid').addEventListener('click', (event) => {
  const button = event.target.closest('button[data-act]');
  if (!button) return;
  const card = button.closest('.key-card');
  const id = card?.dataset.id;
  if (!id) return;
  const action = button.dataset.act;
  if (action === 'detail') {
    openDetailDialog(id);
    return;
  }
  if (busy) return;
  if (action === 'select-client') setSelectedClientForKey(id, button.dataset.client);
  else if (action === 'switch') {
    const key = keys.find((item) => item.id === id);
    switchKeyClient(id, selectedClientForKey(key));
  } else if (action === 'check') {
    const key = keys.find((item) => item.id === id);
    checkKeyClient(id, selectedClientForKey(key));
  }
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

initCombo($('newModelCombo'));
initCombo($('editModelCombo'));
document.addEventListener('click', (event) => {
  if (!event.target.closest('.combo')) closeAllCombos();
});

applyStaticText();
renderClientFilterBar();
updateRevealIcon();
updateEditRevealIcon();
$('message').textContent = t('footerMessage');
refresh().then(loadDiagnostics);
