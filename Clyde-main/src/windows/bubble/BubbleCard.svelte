<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';

  type SuggestionView = {
    title: string;
    subtitle: string;
  };

  type JsonSchema = {
    type?: string;
    title?: string;
    description?: string;
    enum?: unknown[];
    oneOf?: unknown[];
    anyOf?: unknown[];
    const?: unknown;
    default?: unknown;
    format?: string;
    maxLength?: number;
    properties?: Record<string, unknown>;
    required?: string[];
  };

  type ElicitationField = {
    key: string;
    schema: JsonSchema;
    required: boolean;
  };

  type ChoiceOption = {
    value: unknown;
    label: string;
    description: string;
  };

  const ROOT_FIELD_KEY = '__root__';

  let {
    id,
    windowKind = 'ApprovalRequest',
    toolName = '',
    toolInput = {},
    suggestions = [],
    sessionId,
    lang = 'en',
    agentLabel = 'Claude',
    sessionSummary = '',
    sessionProject = '',
    sessionShortId = '',
    isElicitation = false,
    elicitationMessage = '',
    elicitationSchema = null,
    elicitationMode = '',
    elicitationUrl = '',
    elicitationServerName = '',
    modeLabel = '',
    modeDescription = '',
    updateVersion = '',
    updateUrl = '',
    updateNotes = '',
  }: {
    id: string;
    windowKind?: string;
    toolName?: string;
    toolInput?: Record<string, unknown>;
    suggestions?: unknown[];
    sessionId: string;
    lang?: string;
    agentLabel?: string;
    sessionSummary?: string;
    sessionProject?: string;
    sessionShortId?: string;
    isElicitation?: boolean;
    elicitationMessage?: string;
    elicitationSchema?: unknown;
    elicitationMode?: string;
    elicitationUrl?: string;
    elicitationServerName?: string;
    modeLabel?: string;
    modeDescription?: string;
    updateVersion?: string;
    updateUrl?: string;
    updateNotes?: string;
  } = $props();

  let elicitationValues = $state<Record<string, unknown>>({});

  const isModeNotice = $derived(windowKind === 'ModeNotice');
  const isUpdateNotice = $derived(windowKind === 'UpdateNotice');
  const commandText = $derived(extractCommand(toolInput));
  const headerMeta = $derived([sessionProject, sessionShortId].filter(Boolean).join(' · '));
  const shellName = $derived(detectShell(toolInput, toolName));
  const cwdLabel = $derived(
    compactPath(getString(toolInput, ['cwd', 'workingDirectory', 'working_directory', 'dir', 'path'])),
  );
  const reasonText = $derived(
    compactReason(getString(toolInput, ['justification', 'reason', 'description'])),
  );
  const normalizedElicitationSchema = $derived(normalizeSchema(elicitationSchema));
  const elicitationFields = $derived(extractElicitationFields(normalizedElicitationSchema));
  const singleChoiceField = $derived(getSingleChoiceField(elicitationFields));
  const singleChoiceOptions = $derived(
    singleChoiceField ? extractChoiceOptions(singleChoiceField.schema) : [],
  );
  const badge = $derived(resolveBadge(toolName, isElicitation));
  const canSubmitElicitation = $derived(checkElicitationValidity(elicitationFields, elicitationValues));

  const TOOL_BADGES: Record<string, string> = {
    Bash: 'BASH',
    Read: 'READ',
    Write: 'WRITE',
    Edit: 'EDIT',
    Glob: 'GLOB',
    Grep: 'GREP',
    Agent: 'AGENT',
    WebFetch: 'WEB',
    WebSearch: 'WEB',
    NotebookEdit: 'NB',
  };

  const I18N: Record<string, Record<string, string>> = {
    en: {
      applySuggestedPermissionChange: 'Apply the suggested permission change',
      applySuggestedPermission: 'Apply suggested permission',
      applySuggestedRule: 'Apply the suggested rule',
      approveOnlyThisRequest: 'Approve only this request',
      allowOnce: 'Allow Once',
      allowTool: 'Allow {agent} to use {tool}?',
      allowCommand: 'Allow {agent} to run this command?',
      blockThisRequest: 'Block this request',
      cancel: 'Cancel',
      chooseReply: 'Choose a reply here or jump back to the terminal session.',
      closeWithoutAnswering: 'Close without answering',
      continueInBrowser: 'Continue in the browser',
      createPersistentAllowRule: 'Create a persistent allow rule',
      decline: 'Decline',
      deny: 'Deny',
      dismiss: 'Dismiss',
      futureEditRequestsAutoApproved: 'Future edit requests can be approved automatically',
      goDownload: 'Download',
      mode: 'Mode',
      modeChanged: 'Mode Changed',
      needsReply: '{agent} Needs Reply',
      no: 'No',
      openLink: 'Open Link',
      openTerminal: 'Open Terminal',
      options: 'Options',
      permissionRequest: 'Permission Request',
      prompt: 'Prompt',
      reason: 'Reason',
      refuseThisPrompt: 'Refuse this prompt',
      remember: 'Remember',
      reply: 'Reply',
      replyRequired: 'Reply Required',
      replyThereInstead: 'Reply there instead',
      required: 'Required',
      returnAnswerToAgent: 'Return this answer to {agent}',
      rule: 'Rule',
      sendReply: 'Send Reply',
      server: 'Server',
      skipVersion: 'Skip',
      switchToAcceptEdits: 'Switch to Accept Edits',
      terminal: 'terminal',
      thisOnlyAffectsCurrentRequest: 'This only affects the current request.',
      thisTool: 'this tool',
      updateAvailable: 'Update Available',
      wantsAccess: '{agent} Wants Access',
      yes: 'Yes',
    },
    zh: {
      applySuggestedPermissionChange: '应用建议的权限变更',
      applySuggestedPermission: '应用权限建议',
      applySuggestedRule: '应用建议规则',
      approveOnlyThisRequest: '仅批准本次请求',
      allowOnce: '允许一次',
      allowTool: '允许 {agent} 使用 {tool} 吗？',
      allowCommand: '允许 {agent} 运行这条命令吗？',
      blockThisRequest: '阻止本次请求',
      cancel: '取消',
      chooseReply: '在这里选择回复，或回到终端会话处理。',
      closeWithoutAnswering: '关闭且不回答',
      continueInBrowser: '在浏览器中继续',
      createPersistentAllowRule: '创建持久允许规则',
      decline: '拒绝',
      deny: '拒绝',
      dismiss: '关闭',
      futureEditRequestsAutoApproved: '后续编辑请求可以自动批准',
      goDownload: '前往下载',
      mode: '模式',
      modeChanged: '模式已变更',
      needsReply: '{agent} 需要回复',
      no: '否',
      openLink: '打开链接',
      openTerminal: '打开终端',
      options: '选项',
      permissionRequest: '权限申请',
      prompt: '提示',
      reason: '原因',
      refuseThisPrompt: '拒绝这个提示',
      remember: '记住',
      reply: '回复',
      replyRequired: '需要回复',
      replyThereInstead: '改在终端回复',
      required: '必填',
      returnAnswerToAgent: '将此回答返回给 {agent}',
      rule: '规则',
      sendReply: '发送回复',
      server: '服务',
      skipVersion: '跳过此版本',
      switchToAcceptEdits: '切换到自动编辑',
      terminal: '终端',
      thisOnlyAffectsCurrentRequest: '仅影响当前请求。',
      thisTool: '这个工具',
      updateAvailable: '发现新版本',
      wantsAccess: '{agent} 请求访问',
      yes: '是',
    },
  };

  function t(key: string, values: Record<string, string> = {}): string {
    const table = lang === 'zh' ? I18N.zh : I18N.en;
    let text = table[key] ?? I18N.en[key] ?? key;
    for (const [name, value] of Object.entries(values)) {
      text = text.replaceAll(`{${name}}`, value);
    }
    return text;
  }

  $effect(() => {
    if (!isElicitation) return;
    elicitationValues = buildInitialValues(elicitationFields);
  });

  function resolveBadge(tool: string, elicitation: boolean): string {
    if (elicitation) return 'ASK';
    if (!tool) return 'TASK';
    return TOOL_BADGES[tool] ?? tool.slice(0, 5).toUpperCase();
  }

  function getString(input: Record<string, unknown>, keys: string[]): string {
    for (const key of keys) {
      const value = input[key];
      if (typeof value === 'string' && value.trim()) return value.trim();
    }
    return '';
  }

  function humanizeKey(key: string): string {
    return key
      .replace(/([a-z0-9])([A-Z])/g, '$1 $2')
      .replace(/[_-]+/g, ' ')
      .replace(/\b\w/g, (match) => match.toUpperCase());
  }

  function normalizeShell(value: string): string {
    const cleaned = value.trim().split(/\s+/)[0].split('/').pop() ?? value.trim();
    return cleaned.replace(/\.exe$/i, '');
  }

  function inferShellFromCommand(command: string): string {
    const match = command.match(
      /^\s*(?:\/[^\s]+\/)?(bash|zsh|sh|fish|pwsh|powershell|cmd)(?:\.exe)?\b/i,
    );
    if (!match) return '';
    return normalizeShell(match[1]);
  }

  function detectShell(input: Record<string, unknown>, tool: string): string {
    const explicit = getString(input, ['shell', 'shellType', 'shell_type', 'executable', 'program']);
    if (explicit) return normalizeShell(explicit);

    const command = extractCommand(input);
    const inferred = command ? inferShellFromCommand(command) : '';
    if (inferred) return `${inferred} (inferred)`;

    if (tool === 'Bash') return 'Default shell';
    return '';
  }

  function extractCommand(input: Record<string, unknown>): string {
    return getString(input, ['command', 'cmd', 'script', 'input']);
  }

  function compactPath(path: string): string {
    if (!path) return '';
    const trimmed = path.replace(/\/+$/, '');
    const name = trimmed.split('/').pop() ?? trimmed;
    return name || trimmed;
  }

  function compactReason(reason: string): string {
    if (!reason) return '';
    const singleLine = reason.replace(/\s+/g, ' ').trim();
    return singleLine.length > 88 ? `${singleLine.slice(0, 88).trimEnd()}...` : singleLine;
  }

  function normalizeSchema(value: unknown): JsonSchema | null {
    if (!value || typeof value !== 'object' || Array.isArray(value)) return null;
    return value as JsonSchema;
  }

  function extractElicitationFields(schema: JsonSchema | null): ElicitationField[] {
    if (!schema) return [];

    if (schema.properties && typeof schema.properties === 'object') {
      const required = new Set(schema.required ?? []);
      return Object.entries(schema.properties).map(([key, value]) => ({
        key,
        schema: normalizeSchema(value) ?? {},
        required: required.has(key),
      }));
    }

    return [{ key: ROOT_FIELD_KEY, schema, required: true }];
  }

  function extractChoiceOptions(schema: JsonSchema): ChoiceOption[] {
    if (Array.isArray(schema.enum)) {
      return schema.enum.map((value) => ({
        value,
        label: formatChoiceValue(value),
        description: '',
      }));
    }

    const variants = Array.isArray(schema.oneOf)
      ? schema.oneOf
      : Array.isArray(schema.anyOf)
        ? schema.anyOf
        : [];

    const options: ChoiceOption[] = [];
    for (const item of variants) {
      const option = normalizeSchema(item);
      if (!option) continue;
      const value =
        option.const !== undefined
          ? option.const
          : Array.isArray(option.enum) && option.enum.length === 1
            ? option.enum[0]
            : undefined;
      if (value === undefined) continue;
      options.push({
        value,
        label: option.title || formatChoiceValue(value),
        description: option.description || '',
      });
    }

    return options;
  }

  function getSingleChoiceField(fields: ElicitationField[]): ElicitationField | null {
    if (fields.length !== 1) return null;
    return extractChoiceOptions(fields[0].schema).length > 0 ? fields[0] : null;
  }

  function defaultValueForField(field: ElicitationField): unknown {
    if (field.schema.default !== undefined) return field.schema.default;

    const options = extractChoiceOptions(field.schema);
    if (options.length > 0) return options[0].value;

    if (field.schema.type === 'boolean') return false;
    if (field.schema.type === 'number' || field.schema.type === 'integer') return '';
    return '';
  }

  function buildInitialValues(fields: ElicitationField[]): Record<string, unknown> {
    const next: Record<string, unknown> = {};
    for (const field of fields) {
      next[field.key] = defaultValueForField(field);
    }
    return next;
  }

  function fieldTitle(field: ElicitationField): string {
    if (field.key === ROOT_FIELD_KEY) return field.schema.title || 'Response';
    return field.schema.title || humanizeKey(field.key);
  }

  function fieldDescription(field: ElicitationField): string {
    return field.schema.description || '';
  }

  function fieldKind(field: ElicitationField): 'choice' | 'boolean' | 'number' | 'textarea' | 'text' {
    if (extractChoiceOptions(field.schema).length > 0) return 'choice';
    if (field.schema.type === 'boolean') return 'boolean';
    if (field.schema.type === 'number' || field.schema.type === 'integer') return 'number';
    if (field.schema.format === 'textarea' || (field.schema.maxLength ?? 0) > 120) return 'textarea';
    return 'text';
  }

  function formatChoiceValue(value: unknown): string {
    if (typeof value === 'string') return value;
    if (typeof value === 'number' || typeof value === 'boolean') return String(value);
    return JSON.stringify(value);
  }

  function stringifyFieldValue(value: unknown): string {
    if (typeof value === 'string') return value;
    if (typeof value === 'number') return String(value);
    return '';
  }

  function parseNumberValue(raw: string): unknown {
    const trimmed = raw.trim();
    if (!trimmed) return '';
    const numeric = Number(trimmed);
    return Number.isFinite(numeric) ? numeric : '';
  }

  function updateFieldValue(key: string, value: unknown) {
    elicitationValues = {
      ...elicitationValues,
      [key]: value,
    };
  }

  function isFilled(value: unknown, field: ElicitationField): boolean {
    if (field.schema.type === 'boolean') return typeof value === 'boolean';
    if (field.schema.type === 'number' || field.schema.type === 'integer') return value !== '';
    if (typeof value === 'string') return value.trim().length > 0;
    return value !== undefined && value !== null;
  }

  function checkElicitationValidity(fields: ElicitationField[], values: Record<string, unknown>): boolean {
    if (fields.length === 0) return false;
    return fields.every((field) => !field.required || isFilled(values[field.key], field));
  }

  function buildElicitationContent(
    override?: { key: string; value: unknown } | null,
  ): unknown {
    const values = override
      ? {
          ...elicitationValues,
          [override.key]: override.value,
        }
      : elicitationValues;

    if (elicitationFields.length === 1 && elicitationFields[0].key === ROOT_FIELD_KEY) {
      return values[ROOT_FIELD_KEY];
    }

    const content: Record<string, unknown> = {};
    for (const field of elicitationFields) {
      const value = values[field.key];
      if (!isFilled(value, field) && !field.required) continue;
      content[field.key] = value;
    }
    return content;
  }

  function requestTitle(): string {
    if (isElicitation) return t('needsReply', { agent: agentLabel });
    if (toolName === 'Bash' && commandText) return t('allowCommand', { agent: agentLabel });
    return t('allowTool', { agent: agentLabel, tool: toolName || t('thisTool') });
  }

  async function allow() {
    await invoke('resolve_permission', { id, decision: 'allow' });
  }

  async function deny() {
    await invoke('resolve_permission', { id, decision: 'deny' });
  }

  async function applySuggestion(suggestion: unknown) {
    await invoke('resolve_permission', { id, decision: 'allow', selectedSuggestion: suggestion });
  }

  async function acceptElicitation() {
    await invoke('resolve_permission', {
      id,
      decision: 'accept',
      elicitationContent: buildElicitationContent(),
    });
  }

  async function acceptChoice(field: ElicitationField, value: unknown) {
    await invoke('resolve_permission', {
      id,
      decision: 'accept',
      elicitationContent: buildElicitationContent({ key: field.key, value }),
    });
  }

  async function declineElicitation() {
    await invoke('resolve_permission', { id, decision: 'decline' });
  }

  async function cancelElicitation() {
    await invoke('resolve_permission', { id, decision: 'cancel' });
  }

  function suggestionLabel(sug: unknown): string {
    return describeSuggestion(sug).title;
  }

  function describeSuggestion(sug: unknown): SuggestionView {
    if (typeof sug !== 'object' || sug === null) {
      return {
        title: String(sug),
        subtitle: t('applySuggestedPermissionChange'),
      };
    }
    const obj = sug as Record<string, unknown>;
    const type = obj.type as string | undefined;
    const suggestionTool =
      (obj.toolName as string | undefined) ??
      (obj.tool_name as string | undefined) ??
      toolName;

    if (type === 'addRules' && obj.behavior === 'allow') {
      const rule = typeof obj.ruleContent === 'string' ? obj.ruleContent : '';
      return {
        title: lang === 'zh' ? `始终允许匹配的 ${suggestionTool}` : `Always allow matching ${suggestionTool}`,
        subtitle: rule ? `${t('rule')}: ${rule}` : t('createPersistentAllowRule'),
      };
    }
    if (type === 'setMode' && obj.mode === 'acceptEdits') {
      return {
        title: t('switchToAcceptEdits'),
        subtitle: t('futureEditRequestsAutoApproved'),
      };
    }
    if (type === 'addRules') {
      const behavior = typeof obj.behavior === 'string' ? obj.behavior : 'allow';
      return {
        title: `${permissionBehaviorLabel(behavior)} ${suggestionTool}`,
        subtitle: t('applySuggestedRule'),
      };
    }
    return {
      title: t('applySuggestedPermission'),
      subtitle: t('applySuggestedPermissionChange'),
    };
  }

  function permissionBehaviorLabel(behavior: string): string {
    if (lang !== 'zh') return humanizeKey(behavior);
    if (behavior === 'allow') return '允许';
    if (behavior === 'deny') return '拒绝';
    return humanizeKey(behavior);
  }

  async function focusTerminal() {
    await invoke('focus_terminal_for_session', { sessionId });
  }

  async function openElicitationLink() {
    if (!elicitationUrl) return;
    window.open(elicitationUrl, '_blank', 'noopener,noreferrer');
  }

  async function openUpdate() {
    await invoke('open_update_url', { url: updateUrl });
  }

  async function dismissUpdate() {
    await invoke('dismiss_update_version', { version: updateVersion });
  }

  async function dismiss() {
    // Mode notice: close bubble via Rust to properly clean up BubbleMap
    await invoke('dismiss_bubble', { id });
  }
</script>

{#if isUpdateNotice}
  <div class="bubble">
    <div class="glow glow-update"></div>
    <div class="header bubble-drag-handle">
      <div class="header-copy">
        <span class="eyebrow">Clyde</span>
        <span class="title">{t('updateAvailable')}</span>
      </div>
      <span class="badge badge-update">v{updateVersion}</span>
    </div>

    {#if updateNotes}
      <div class="code-block mode-block update-notes">
        <pre>{updateNotes}</pre>
      </div>
    {/if}

    <div class="actions">
      <button class="btn btn-primary" onclick={openUpdate} aria-label={t('goDownload')}>
        {t('goDownload')}
      </button>
      <button class="btn btn-secondary" onclick={dismissUpdate} aria-label={t('skipVersion')}>
        {t('skipVersion')}
      </button>
    </div>
  </div>
{:else if isModeNotice}
  <div class="bubble">
    <div class="glow glow-mode"></div>
    <div class="header bubble-drag-handle">
      <div class="header-copy">
        <span class="eyebrow">{agentLabel}</span>
        <span class="title">{sessionSummary || t('modeChanged')}</span>
        {#if headerMeta}
          <span class="meta">{headerMeta}</span>
        {/if}
      </div>
      <span class="badge badge-mode">{modeLabel}</span>
    </div>

    <div class="code-block mode-block">
      <pre>{modeDescription}</pre>
    </div>

    <div class="actions">
      <button class="btn btn-primary" onclick={dismiss} aria-label={t('dismiss')}>
        OK
      </button>
    </div>
  </div>
{:else}
  <div class="bubble">
    <div class="glow"></div>
    <div class="header bubble-drag-handle">
      <div class="header-copy">
        <span class="eyebrow">
          {isElicitation ? t('needsReply', { agent: agentLabel }) : t('wantsAccess', { agent: agentLabel })}
        </span>
        <span class="title">
          {sessionSummary || (isElicitation ? t('replyRequired') : t('permissionRequest'))}
        </span>
        {#if headerMeta}
          <span class="meta">{headerMeta}</span>
        {/if}
      </div>
      <span class="badge">{badge}</span>
    </div>

    <div class="intent">
      <div class="intent-title">{requestTitle()}</div>
      <div class="intent-copy">
        {#if isElicitation}
          {t('chooseReply')}
        {:else}
          {t('thisOnlyAffectsCurrentRequest')}
        {/if}
      </div>
    </div>

    {#if shellName || cwdLabel || elicitationServerName}
      <div class="meta-row">
        {#if shellName}
          <span class="mini-meta">{shellName}</span>
        {/if}
        {#if cwdLabel}
          <span class="mini-meta">{cwdLabel}</span>
        {/if}
        {#if elicitationServerName}
          <span class="mini-meta">{t('server')}: {elicitationServerName}</span>
        {/if}
      </div>
    {/if}

    {#if isElicitation && elicitationMessage}
      <div class="prompt-block">
        <div class="section-label">{t('prompt')}</div>
        <div class="prompt-copy">{elicitationMessage}</div>
      </div>
    {/if}

    {#if commandText}
      <div class="code-block command-block">
        <pre>{commandText}</pre>
      </div>
    {/if}

    {#if reasonText}
      <div class="reason">
        <span class="reason-label">{t('reason')}</span>
        <span class="reason-copy">{reasonText}</span>
      </div>
    {/if}

    {#if isElicitation}
      {#if singleChoiceField && singleChoiceOptions.length > 0}
        <div class="section-label">{t('options')}</div>
        <div class="choice-list">
          {#each singleChoiceOptions as option}
            <button
              class="choice-btn"
              onclick={() => acceptChoice(singleChoiceField, option.value)}
              aria-label={`${t('options')}: ${option.label}`}
            >
              <span class="choice-title">{option.label}</span>
              {#if option.description}
                <span class="choice-description">{option.description}</span>
              {/if}
            </button>
          {/each}
        </div>
      {:else if elicitationFields.length > 0}
        <div class="section-label">{t('reply')}</div>
        <div class="form-fields">
          {#each elicitationFields as field}
            {@const kind = fieldKind(field)}
            {@const options = kind === 'choice' ? extractChoiceOptions(field.schema) : []}
            <label class="field">
              <span class="field-label">
                {fieldTitle(field)}
                {#if field.required}
                  <span class="field-required">{t('required')}</span>
                {/if}
              </span>
              {#if fieldDescription(field)}
                <span class="field-description">{fieldDescription(field)}</span>
              {/if}

              {#if kind === 'choice'}
                <div class="choice-grid">
                  {#each options as option}
                    <button
                      class:selected={elicitationValues[field.key] === option.value}
                      class="choice-pill"
                      type="button"
                      onclick={() => updateFieldValue(field.key, option.value)}
                      aria-label={`${t('options')}: ${option.label}`}
                    >
                      {option.label}
                    </button>
                  {/each}
                </div>
              {:else if kind === 'boolean'}
                <button
                  class:selected={Boolean(elicitationValues[field.key])}
                  class="toggle-pill"
                  type="button"
                  onclick={() => updateFieldValue(field.key, !Boolean(elicitationValues[field.key]))}
                  aria-label={`${fieldTitle(field)}`}
                >
                  {Boolean(elicitationValues[field.key]) ? t('yes') : t('no')}
                </button>
              {:else if kind === 'textarea'}
                <textarea
                  class="input textarea"
                  rows="3"
                  value={stringifyFieldValue(elicitationValues[field.key])}
                  oninput={(event) => updateFieldValue(field.key, (event.currentTarget as HTMLTextAreaElement).value)}
                ></textarea>
              {:else}
                <input
                  class="input"
                  type={kind === 'number' ? 'number' : 'text'}
                  value={stringifyFieldValue(elicitationValues[field.key])}
                  oninput={(event) =>
                    updateFieldValue(
                      field.key,
                      kind === 'number'
                        ? parseNumberValue((event.currentTarget as HTMLInputElement).value)
                        : (event.currentTarget as HTMLInputElement).value,
                    )}
                />
              {/if}
            </label>
          {/each}
        </div>
      {/if}

      {#if elicitationMode || elicitationUrl}
        <div class="reason">
          <span class="reason-label">{t('mode')}</span>
          <span class="reason-copy">
            {elicitationMode || t('terminal')}
            {#if elicitationUrl}
              <span class="inline-link">{elicitationUrl}</span>
            {/if}
          </span>
        </div>
      {/if}

      <div class="actions actions-wrap">
        {#if !singleChoiceField && elicitationFields.length > 0}
          <button
            class="btn btn-primary btn-stacked"
            onclick={acceptElicitation}
            aria-label={t('sendReply')}
            disabled={!canSubmitElicitation}
          >
            <span>{t('sendReply')}</span>
            <small>{t('returnAnswerToAgent', { agent: agentLabel })}</small>
          </button>
        {/if}

        {#if elicitationUrl}
          <button class="btn btn-secondary btn-stacked" onclick={openElicitationLink} aria-label={t('openLink')}>
            <span>{t('openLink')}</span>
            <small>{t('continueInBrowser')}</small>
          </button>
        {/if}

        <button class="btn btn-secondary btn-stacked" onclick={focusTerminal} aria-label={t('openTerminal')}>
          <span>{t('openTerminal')}</span>
          <small>{t('replyThereInstead')}</small>
        </button>
        <button class="btn btn-secondary btn-stacked" onclick={declineElicitation} aria-label={t('decline')}>
          <span>{t('decline')}</span>
          <small>{t('refuseThisPrompt')}</small>
        </button>
        <button class="btn btn-secondary btn-stacked" onclick={cancelElicitation} aria-label={t('cancel')}>
          <span>{t('cancel')}</span>
          <small>{t('closeWithoutAnswering')}</small>
        </button>
      </div>
    {:else}
      <div class="actions">
        <button class="btn btn-primary btn-stacked" onclick={allow} aria-label={t('allowOnce')}>
          <span>{t('allowOnce')}</span>
          <small>{t('approveOnlyThisRequest')}</small>
        </button>
        <button class="btn btn-secondary btn-stacked" onclick={deny} aria-label={t('deny')}>
          <span>{t('deny')}</span>
          <small>{t('blockThisRequest')}</small>
        </button>
      </div>

      {#if suggestions.length > 0}
        <div class="section-label suggestions-label">{t('remember')}</div>
        <div class="suggestions">
          {#each suggestions as sug}
            {@const suggestion = describeSuggestion(sug)}
            <button
              class="suggestion"
              onclick={() => applySuggestion(sug)}
              aria-label={`${t('applySuggestedPermission')}: ${suggestionLabel(sug)}`}
            >
              <span class="suggestion-title">{suggestion.title}</span>
              <span class="suggestion-subtitle">{suggestion.subtitle}</span>
            </button>
          {/each}
        </div>
      {/if}
    {/if}
  </div>
{/if}

<style>
  .bubble {
    --surface-top: rgba(34, 18, 42, 0.95);
    --surface-bottom: rgba(18, 10, 30, 0.92);
    --surface-border: rgba(232, 121, 249, 0.18);
    --surface-shadow: rgba(14, 7, 22, 0.44);
    --copy-primary: #fff4fb;
    --copy-secondary: #d9bedf;
    --accent: #e879f9;
    --accent-strong: #f0abfc;
    position: relative;
    overflow: hidden;
    background:
      linear-gradient(180deg, rgba(255, 255, 255, 0.035), rgba(255, 255, 255, 0) 28%),
      linear-gradient(160deg, var(--surface-top), var(--surface-bottom));
    backdrop-filter: blur(26px) saturate(155%);
    -webkit-backdrop-filter: blur(26px) saturate(155%);
    color: var(--copy-primary);
    border-radius: 18px;
    padding: 18px;
    font-size: 13px;
    border: 1px solid var(--surface-border);
    box-shadow:
      0 22px 44px var(--surface-shadow),
      0 0 0 1px rgba(0, 0, 0, 0.24);
  }

  .glow {
    position: absolute;
    top: -34px;
    right: -30px;
    width: 128px;
    height: 128px;
    border-radius: 999px;
    background: radial-gradient(circle, rgba(232, 121, 249, 0.3), rgba(232, 121, 249, 0) 72%);
    pointer-events: none;
  }

  .glow-mode {
    background: radial-gradient(circle, rgba(106, 155, 232, 0.24), rgba(106, 155, 232, 0) 72%);
  }

  .glow-update {
    background: radial-gradient(circle, rgba(130, 210, 140, 0.24), rgba(130, 210, 140, 0) 72%);
  }

  .header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 12px;
    margin-bottom: 14px;
  }

  .header-copy {
    display: flex;
    flex-direction: column;
    gap: 4px;
    min-width: 0;
  }

  .eyebrow {
    font-size: 10px;
    font-weight: 700;
    letter-spacing: 0.12em;
    text-transform: uppercase;
    color: var(--copy-secondary);
  }

  .title {
    font-weight: 650;
    font-size: 15px;
    color: var(--copy-primary);
    letter-spacing: -0.02em;
  }

  .meta {
    font-size: 11px;
    line-height: 1.4;
    color: rgba(235, 213, 241, 0.84);
    word-break: break-word;
  }

  .meta-row {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    margin: 0 0 12px;
  }

  .mini-meta {
    display: inline-flex;
    align-items: center;
    min-height: 24px;
    padding: 0 9px;
    border-radius: 999px;
    background: rgba(255, 255, 255, 0.05);
    border: 1px solid rgba(255, 255, 255, 0.07);
    font-size: 11px;
    color: #e7d2ec;
  }

  .badge {
    flex-shrink: 0;
    font-size: 10px;
    font-weight: 700;
    letter-spacing: 0.08em;
    background: rgba(232, 121, 249, 0.16);
    color: var(--accent-strong);
    padding: 5px 9px;
    border-radius: 999px;
    border: 1px solid rgba(232, 121, 249, 0.2);
    white-space: nowrap;
  }

  .badge-mode {
    background: rgba(106, 155, 232, 0.14);
    color: #93bcff;
    border-color: rgba(106, 155, 232, 0.16);
  }

  .badge-update {
    background: rgba(130, 210, 140, 0.14);
    color: #8fd99a;
    border-color: rgba(130, 210, 140, 0.16);
  }

  .update-notes {
    max-height: 120px;
    overflow-y: auto;
  }

  .section-label {
    margin: 0 0 8px;
    font-size: 10px;
    font-weight: 700;
    letter-spacing: 0.12em;
    text-transform: uppercase;
    color: var(--copy-secondary);
  }

  .suggestions-label {
    margin-top: 14px;
  }

  .intent {
    margin-bottom: 14px;
    padding: 12px 13px;
    border-radius: 12px;
    background: linear-gradient(180deg, rgba(232, 121, 249, 0.14), rgba(192, 132, 252, 0.05));
    border: 1px solid rgba(232, 121, 249, 0.14);
  }

  .intent-title {
    font-size: 13px;
    font-weight: 650;
    color: #ffecfb;
    margin-bottom: 5px;
    letter-spacing: -0.015em;
  }

  .intent-copy {
    font-size: 11.5px;
    line-height: 1.45;
    color: #dbc4e4;
  }

  .prompt-block,
  .code-block {
    background: linear-gradient(180deg, rgba(255, 255, 255, 0.03), rgba(255, 255, 255, 0.014));
    border: 1px solid rgba(255, 255, 255, 0.075);
    border-radius: 12px;
    padding: 12px 13px;
    margin-bottom: 12px;
    overflow: hidden;
  }

  .prompt-copy {
    font-size: 12px;
    line-height: 1.55;
    color: #f0dff4;
    white-space: pre-wrap;
    word-break: break-word;
  }

  .code-block {
    max-height: 88px;
  }

  .command-block {
    border-color: rgba(232, 121, 249, 0.14);
    box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.03);
  }

  .mode-block {
    margin-bottom: 16px;
  }

  .code-block pre {
    font-family: 'Cascadia Code', 'Fira Code', 'SF Mono', 'Consolas', monospace;
    font-size: 11.5px;
    line-height: 1.6;
    color: #dfcbe7;
    white-space: pre-wrap;
    word-break: break-all;
    margin: 0;
  }

  .reason {
    display: flex;
    flex-direction: column;
    gap: 5px;
    margin-bottom: 14px;
    padding: 10px 12px;
    border-radius: 11px;
    background: rgba(255, 255, 255, 0.032);
    border: 1px solid rgba(255, 255, 255, 0.065);
  }

  .reason-label {
    font-size: 10px;
    font-weight: 700;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--copy-secondary);
  }

  .reason-copy {
    display: flex;
    flex-direction: column;
    gap: 4px;
    font-size: 11.5px;
    line-height: 1.45;
    color: #f1def4;
  }

  .inline-link {
    color: #f0abfc;
    word-break: break-all;
  }

  .choice-list,
  .form-fields,
  .suggestions {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .choice-btn,
  .suggestion {
    width: 100%;
    padding: 11px 12px;
    border-radius: 11px;
    border: 1px solid rgba(255, 255, 255, 0.065);
    background: rgba(255, 255, 255, 0.045);
    color: #ead7ef;
    text-align: left;
    font-size: 12px;
    line-height: 1.4;
    cursor: pointer;
    transition: transform 0.15s ease, background 0.15s ease, color 0.15s ease, border-color 0.15s ease;
  }

  .choice-btn:hover,
  .suggestion:hover {
    background: rgba(232, 121, 249, 0.12);
    border-color: rgba(232, 121, 249, 0.2);
    color: #fff0fb;
    transform: translateY(-1px);
  }

  .choice-title,
  .suggestion-title {
    display: block;
    font-size: 12px;
    font-weight: 620;
    color: #fae8ff;
    margin-bottom: 3px;
  }

  .choice-description,
  .suggestion-subtitle {
    display: block;
    font-size: 10.5px;
    line-height: 1.4;
    color: #cdb4d6;
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 7px;
  }

  .field-label {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 12px;
    font-weight: 620;
    color: #f7e8fb;
  }

  .field-required {
    font-size: 10px;
    color: #f0abfc;
    text-transform: uppercase;
    letter-spacing: 0.08em;
  }

  .field-description {
    font-size: 10.5px;
    line-height: 1.45;
    color: #cab2d4;
  }

  .choice-grid {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }

  .choice-pill,
  .toggle-pill {
    min-height: 32px;
    padding: 0 11px;
    border-radius: 999px;
    border: 1px solid rgba(255, 255, 255, 0.08);
    background: rgba(255, 255, 255, 0.04);
    color: #ead7ef;
    font-size: 11px;
    font-weight: 600;
    cursor: pointer;
    transition: background 0.15s ease, border-color 0.15s ease, color 0.15s ease;
  }

  .choice-pill.selected,
  .toggle-pill.selected {
    background: rgba(232, 121, 249, 0.18);
    border-color: rgba(232, 121, 249, 0.28);
    color: #ffeffb;
  }

  .input {
    width: 100%;
    min-height: 36px;
    padding: 9px 11px;
    border-radius: 10px;
    border: 1px solid rgba(255, 255, 255, 0.08);
    background: rgba(255, 255, 255, 0.04);
    color: #fae8ff;
    font-size: 12px;
    outline: none;
    box-sizing: border-box;
  }

  .textarea {
    min-height: 78px;
    resize: vertical;
    font-family: inherit;
  }

  .input:focus,
  .textarea:focus {
    border-color: rgba(232, 121, 249, 0.32);
    background: rgba(255, 255, 255, 0.05);
  }

  .actions {
    display: flex;
    gap: 10px;
    margin-bottom: 0;
  }

  .actions-wrap {
    flex-wrap: wrap;
    margin-top: 14px;
  }

  .btn {
    flex: 1;
    min-height: 40px;
    padding: 10px 0;
    border-radius: 11px;
    font-size: 13px;
    font-weight: 650;
    cursor: pointer;
    transition: transform 0.15s ease, box-shadow 0.15s ease, background 0.15s ease, border-color 0.15s ease;
    border: 1px solid transparent;
    letter-spacing: -0.015em;
  }

  .btn:disabled {
    cursor: not-allowed;
    opacity: 0.5;
    transform: none;
    box-shadow: none;
  }

  .btn-stacked {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 2px;
    min-width: 136px;
    padding: 9px 10px;
  }

  .btn-stacked span {
    line-height: 1.15;
  }

  .btn-stacked small {
    font-size: 10px;
    font-weight: 550;
    opacity: 0.78;
    line-height: 1.2;
    text-align: center;
  }

  .btn-primary {
    background: linear-gradient(135deg, #f0abfc, #c084fc);
    color: #2b0f34;
    box-shadow: 0 10px 22px rgba(192, 132, 252, 0.28);
  }

  .btn-primary:hover:not(:disabled) {
    background: linear-gradient(135deg, #f5c2ff, #d8b4fe);
    box-shadow: 0 14px 26px rgba(192, 132, 252, 0.34);
    transform: translateY(-1px);
  }

  .btn-primary:active:not(:disabled) {
    transform: translateY(0);
    box-shadow: 0 6px 14px rgba(192, 132, 252, 0.22);
  }

  .btn-secondary {
    background: rgba(255, 255, 255, 0.045);
    color: #ead7ef;
    border-color: rgba(255, 255, 255, 0.08);
  }

  .btn-secondary:hover {
    background: rgba(255, 255, 255, 0.085);
    border-color: rgba(255, 255, 255, 0.13);
    color: #fae8ff;
    transform: translateY(-1px);
  }

  .btn-secondary:active {
    transform: translateY(0);
  }

  .btn:focus-visible,
  .choice-btn:focus-visible,
  .choice-pill:focus-visible,
  .toggle-pill:focus-visible,
  .suggestion:focus-visible,
  .input:focus-visible,
  .textarea:focus-visible {
    outline: 2px solid rgba(232, 121, 249, 0.72);
    outline-offset: 2px;
  }
</style>
