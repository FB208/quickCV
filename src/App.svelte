<script lang="ts">
  import { onMount } from "svelte";
  import {
    checkUpdate,
    downloadAndInstallUpdate,
    getAppVersion,
    loadSettings,
    loadTemplateStore,
    openOverlay,
    saveSettings,
    saveTemplateStore,
    syncPull,
    syncPush,
    testWebDav
  } from "./lib/api";
  import { formatHotkey } from "./lib/hotkey";
  import type {
    Folder,
    Settings,
    SyncResult,
    TemplateItem,
    TemplateStore,
    UpdateCheckResult
  } from "./lib/types";

  type TabKey = "general" | "templates" | "sync";
  type NoticeType = "info" | "success" | "error";

  const now = (): number => Date.now();
  const newId = (): string => {
    if (typeof crypto !== "undefined" && "randomUUID" in crypto) {
      return crypto.randomUUID();
    }
    return `${Date.now()}-${Math.random().toString(16).slice(2)}`;
  };

  const defaultSettings = (): Settings => ({
    shortcut: "Ctrl+Shift+Space",
    launchAtStartup: false,
    webdav: {
      url: "",
      username: "",
      password: "",
      remoteFile: "quickcv-data.json"
    },
    updater: {
      endpoint: "",
      pubkey: ""
    },
    lastSyncedVersion: 0,
    deviceId: ""
  });

  const defaultStore = (): TemplateStore => ({
    datasetVersion: 0,
    folders: [],
    templates: []
  });

  let tab: TabKey = "general";
  let settings = defaultSettings();
  let store = defaultStore();
  let appVersion = "--";
  let loading = true;
  let busy = false;

  let notice = "";
  let noticeType: NoticeType = "info";

  let recordingShortcut = false;

  let folderSearch = "";
  let templateSearch = "";
  let newFolderName = "";
  let newTemplateName = "";

  let selectedFolderId = "";
  let selectedTemplateId = "";
  let templateDraft: TemplateItem | null = null;

  $: activeFolders = store.folders.filter((item) => item.deletedAt === null);
  $: if (!selectedFolderId || !activeFolders.some((item) => item.id === selectedFolderId)) {
    selectedFolderId = activeFolders[0]?.id || "";
  }

  $: filteredFolders = activeFolders.filter((item) => {
    if (!folderSearch.trim()) {
      return true;
    }
    return item.name.toLowerCase().includes(folderSearch.trim().toLowerCase());
  });

  $: activeTemplates = store.templates.filter(
    (item) => item.deletedAt === null && item.folderId === selectedFolderId
  );

  $: filteredTemplates = activeTemplates.filter((item) => {
    const keyword = templateSearch.trim().toLowerCase();
    if (!keyword) {
      return true;
    }
    return (
      item.name.toLowerCase().includes(keyword) ||
      (item.key || "").toLowerCase().includes(keyword) ||
      item.content.toLowerCase().includes(keyword)
    );
  });

  $: if (!selectedTemplateId || !filteredTemplates.some((item) => item.id === selectedTemplateId)) {
    selectedTemplateId = filteredTemplates[0]?.id || "";
  }

  $: {
    const selected = store.templates.find((item) => item.id === selectedTemplateId && item.deletedAt === null);
    if (selected && templateDraft?.id !== selected.id) {
      templateDraft = { ...selected };
    }
    if (!selected) {
      templateDraft = null;
    }
  }

  onMount(() => {
    const keyRecorder = (event: KeyboardEvent): void => {
      if (!recordingShortcut) {
        return;
      }
      event.preventDefault();
      event.stopPropagation();

      if (event.key === "Escape") {
        recordingShortcut = false;
        setNotice("info", "已取消快捷键录制");
        return;
      }

      const hotkey = formatHotkey(event);
      if (!hotkey || onlyModifier(hotkey)) {
        return;
      }

      settings = { ...settings, shortcut: hotkey };
      recordingShortcut = false;
      setNotice("success", `快捷键已录制为 ${hotkey}`);
    };

    window.addEventListener("keydown", keyRecorder, true);
    void bootstrap();
    return () => {
      window.removeEventListener("keydown", keyRecorder, true);
    };
  });

  const bootstrap = async (): Promise<void> => {
    loading = true;
    try {
      const [loadedSettings, loadedStore, version] = await Promise.all([
        loadSettings(),
        loadTemplateStore(),
        getAppVersion()
      ]);
      settings = loadedSettings;
      store = loadedStore;
      appVersion = version;
      void runStartupUpdateCheck();
    } catch (error) {
      setNotice("error", asErrorMessage(error));
    } finally {
      loading = false;
    }
  };

  const onlyModifier = (value: string): boolean => {
    const tokens = value.split("+");
    return tokens.every((token) => ["Ctrl", "Alt", "Shift", "Meta"].includes(token));
  };

  const asErrorMessage = (error: unknown): string => {
    if (error instanceof Error) {
      return error.message;
    }
    return "发生未知错误，请查看日志";
  };

  const setNotice = (type: NoticeType, message: string): void => {
    noticeType = type;
    notice = message;
  };

  const persistSettings = async (): Promise<void> => {
    busy = true;
    try {
      settings = await saveSettings(settings);
      setNotice("success", "设置已保存");
    } catch (error) {
      setNotice("error", asErrorMessage(error));
    } finally {
      busy = false;
    }
  };

  const persistStore = async (successMessage: string): Promise<void> => {
    busy = true;
    try {
      store = await saveTemplateStore(store);
      setNotice("success", successMessage);
    } catch (error) {
      setNotice("error", asErrorMessage(error));
    } finally {
      busy = false;
    }
  };

  const createFolder = async (): Promise<void> => {
    const name = newFolderName.trim();
    if (!name) {
      setNotice("error", "请输入文件夹名称");
      return;
    }

    const folder: Folder = {
      id: newId(),
      name,
      updatedAt: now(),
      deletedAt: null,
      deviceId: settings.deviceId || "desktop"
    };

    store = {
      ...store,
      folders: [...store.folders, folder]
    };
    selectedFolderId = folder.id;
    newFolderName = "";
    await persistStore("已新增文件夹");
  };

  const renameFolder = async (folderId: string): Promise<void> => {
    const folder = store.folders.find((item) => item.id === folderId && item.deletedAt === null);
    if (!folder) {
      return;
    }

    const name = window.prompt("请输入新的文件夹名称", folder.name);
    if (name === null) {
      return;
    }

    const trimmed = name.trim();
    if (!trimmed) {
      setNotice("error", "文件夹名称不能为空");
      return;
    }

    folder.name = trimmed;
    folder.updatedAt = now();
    folder.deviceId = settings.deviceId || folder.deviceId;
    store = { ...store, folders: [...store.folders] };
    await persistStore("文件夹已重命名");
  };

  const removeFolder = async (folderId: string): Promise<void> => {
    const folder = store.folders.find((item) => item.id === folderId && item.deletedAt === null);
    if (!folder) {
      return;
    }

    if (!window.confirm(`确认删除文件夹「${folder.name}」及其模板吗？`)) {
      return;
    }

    const timestamp = now();
    folder.deletedAt = timestamp;
    folder.updatedAt = timestamp;
    folder.deviceId = settings.deviceId || folder.deviceId;

    store.templates
      .filter((item) => item.folderId === folderId && item.deletedAt === null)
      .forEach((item) => {
        item.deletedAt = timestamp;
        item.updatedAt = timestamp;
        item.deviceId = settings.deviceId || item.deviceId;
      });

    store = { ...store, folders: [...store.folders], templates: [...store.templates] };
    await persistStore("文件夹已删除");
  };

  const createTemplate = async (): Promise<void> => {
    if (!selectedFolderId) {
      setNotice("error", "请先选择文件夹");
      return;
    }

    const name = newTemplateName.trim() || "未命名模板";
    const template: TemplateItem = {
      id: newId(),
      folderId: selectedFolderId,
      name,
      key: null,
      content: "",
      updatedAt: now(),
      deletedAt: null,
      deviceId: settings.deviceId || "desktop"
    };

    store = {
      ...store,
      templates: [...store.templates, template]
    };
    selectedTemplateId = template.id;
    templateDraft = { ...template };
    newTemplateName = "";
    await persistStore("模板已创建");
  };

  const removeTemplate = async (templateId: string): Promise<void> => {
    const template = store.templates.find((item) => item.id === templateId && item.deletedAt === null);
    if (!template) {
      return;
    }

    if (!window.confirm(`确认删除模板「${template.name}」吗？`)) {
      return;
    }

    template.deletedAt = now();
    template.updatedAt = template.deletedAt;
    template.deviceId = settings.deviceId || template.deviceId;
    store = { ...store, templates: [...store.templates] };
    selectedTemplateId = "";
    templateDraft = null;
    await persistStore("模板已删除");
  };

  const saveTemplateDraft = async (): Promise<void> => {
    if (!templateDraft) {
      setNotice("error", "当前没有可保存的模板");
      return;
    }

    const key = templateDraft.key?.trim() || null;
    if (key) {
      const exists = store.templates.some(
        (item) => item.deletedAt === null && item.id !== templateDraft?.id && (item.key || "").trim() === key
      );
      if (exists) {
        setNotice("error", `模板 key「${key}」已存在，请更换`);
        return;
      }
    }

    const target = store.templates.find((item) => item.id === templateDraft?.id && item.deletedAt === null);
    if (!target) {
      setNotice("error", "模板不存在或已删除");
      return;
    }

    target.name = templateDraft.name.trim() || "未命名模板";
    target.key = key;
    target.content = templateDraft.content;
    target.updatedAt = now();
    target.deviceId = settings.deviceId || target.deviceId;
    store = { ...store, templates: [...store.templates] };
    templateDraft = { ...target };
    await persistStore("模板已保存");
  };

  const runWebDavTest = async (): Promise<void> => {
    busy = true;
    try {
      const message = await testWebDav(settings);
      setNotice("success", message);
    } catch (error) {
      setNotice("error", asErrorMessage(error));
    } finally {
      busy = false;
    }
  };

  const runSync = async (mode: "pull" | "push"): Promise<void> => {
    busy = true;
    try {
      const result: SyncResult = mode === "pull" ? await syncPull() : await syncPush();
      settings = await loadSettings();
      store = await loadTemplateStore();
      setNotice(result.blocked ? "error" : "success", makeSyncMessage(result));
    } catch (error) {
      setNotice("error", asErrorMessage(error));
    } finally {
      busy = false;
    }
  };

  const makeSyncMessage = (result: SyncResult): string => {
    const details: string[] = [result.message];
    if (result.conflictCopies.length > 0) {
      details.push(`冲突副本 ${result.conflictCopies.length} 条`);
    }
    if (result.keyConflicts.length > 0) {
      details.push(`key 冲突 ${result.keyConflicts.length} 条`);
    }
    return details.join("，");
  };

  const runManualUpdateCheck = async (): Promise<void> => {
    await runUpdateCheck(true, false);
  };

  const previewOverlay = async (): Promise<void> => {
    try {
      await openOverlay();
    } catch (error) {
      setNotice("error", asErrorMessage(error));
    }
  };

  const runStartupUpdateCheck = async (): Promise<void> => {
    await runUpdateCheck(true, true);
  };

  const runUpdateCheck = async (promptInstall: boolean, silentIfNoUpdate: boolean): Promise<void> => {
    const lockUi = !silentIfNoUpdate;
    if (lockUi) {
      busy = true;
    }

    try {
      const result = await checkUpdate();

      if (result.status === "available") {
        setNotice("info", result.message);
        if (promptInstall) {
          await askAndInstallUpdate(result);
          return;
        }
      }

      if (!silentIfNoUpdate) {
        const noticeLevel = result.status === "error" ? "error" : "info";
        setNotice(noticeLevel, result.message);
      }
    } catch {
      if (!silentIfNoUpdate) {
        setNotice("error", "检查更新失败，请稍后重试");
      }
    } finally {
      if (lockUi) {
        busy = false;
      }
    }
  };

  const askAndInstallUpdate = async (result: UpdateCheckResult): Promise<void> => {
    const nextVersion = result.latestVersion || "未知版本";
    const confirmed = window.confirm(
      `检测到新版本 ${nextVersion}（当前 ${result.currentVersion}）。是否立即下载并安装？`
    );

    if (!confirmed) {
      setNotice("info", "已取消更新安装");
      return;
    }

    busy = true;
    try {
      const message = await downloadAndInstallUpdate();
      setNotice("info", message);
    } catch (error) {
      setNotice("error", asErrorMessage(error));
    } finally {
      busy = false;
    }
  };
</script>

<main class="shell">
  <header class="topbar">
    <div>
      <h1>quickCV</h1>
      <p>轻量级模板插入管理工具</p>
    </div>
    <div class="version">版本 {appVersion}</div>
  </header>

  {#if notice}
    <div class={`notice ${noticeType}`}>{notice}</div>
  {/if}

  {#if loading}
    <section class="loading">正在加载设置...</section>
  {:else}
    <section class="content">
      <nav class="tabs" aria-label="设置分组">
        <button class:active={tab === "general"} on:click={() => (tab = "general")}>常规设置</button>
        <button class:active={tab === "templates"} on:click={() => (tab = "templates")}>模板管理</button>
        <button class:active={tab === "sync"} on:click={() => (tab = "sync")}>同步与更新</button>
      </nav>

      {#if tab === "general"}
        <section class="panel">
          <h2>常规设置</h2>
          <label class="field">
            <span>全局快捷键</span>
            <div class="row">
              <input type="text" value={recordingShortcut ? "请按下快捷键..." : settings.shortcut} readonly />
              <button disabled={busy} on:click={() => (recordingShortcut = true)}>
                {recordingShortcut ? "录制中" : "录制快捷键"}
              </button>
            </div>
          </label>

          <label class="checkbox">
            <input type="checkbox" bind:checked={settings.launchAtStartup} />
            开机自动启动
          </label>

          <h3>WebDAV 配置</h3>
          <label class="field">
            <span>地址</span>
            <input type="text" bind:value={settings.webdav.url} placeholder="https://dav.example.com/path" />
          </label>

          <label class="field">
            <span>用户名</span>
            <input type="text" bind:value={settings.webdav.username} autocomplete="off" />
          </label>

          <label class="field">
            <span>密码</span>
            <input type="password" bind:value={settings.webdav.password} autocomplete="off" />
          </label>

          <label class="field">
            <span>远端文件名</span>
            <input type="text" bind:value={settings.webdav.remoteFile} placeholder="quickcv-data.json" />
          </label>

          <div class="actions">
            <button disabled={busy} on:click={persistSettings}>保存设置</button>
            <button class="subtle" disabled={busy} on:click={runWebDavTest}>测试 WebDAV 连通性</button>
            <button class="subtle" disabled={busy} on:click={previewOverlay}>预览快捷浮窗</button>
          </div>
        </section>
      {/if}

      {#if tab === "templates"}
        <section class="panel templates">
          <div class="column folders">
            <h2>文件夹</h2>
            <input type="text" bind:value={folderSearch} placeholder="搜索文件夹" />
            <div class="inline-actions">
              <input type="text" bind:value={newFolderName} placeholder="新建文件夹" />
              <button disabled={busy} on:click={createFolder}>新增</button>
            </div>
            <ul>
              {#each filteredFolders as folder}
                <li class:active={folder.id === selectedFolderId}>
                  <button class="item" on:click={() => (selectedFolderId = folder.id)}>{folder.name}</button>
                  <button class="icon" title="重命名" on:click={() => void renameFolder(folder.id)}>改</button>
                  <button class="icon danger" title="删除" on:click={() => void removeFolder(folder.id)}>删</button>
                </li>
              {/each}
            </ul>
          </div>

          <div class="column template-list">
            <h2>模板列表</h2>
            <input type="text" bind:value={templateSearch} placeholder="搜索模板、key、内容" />
            <div class="inline-actions">
              <input type="text" bind:value={newTemplateName} placeholder="新建模板" />
              <button disabled={busy} on:click={createTemplate}>新增</button>
            </div>
            <ul>
              {#each filteredTemplates as item}
                <li class:active={item.id === selectedTemplateId}>
                  <button class="item" on:click={() => (selectedTemplateId = item.id)}>
                    <span>{item.name}</span>
                    {#if item.key}
                      <small>key: {item.key}</small>
                    {/if}
                  </button>
                  <button class="icon danger" title="删除" on:click={() => void removeTemplate(item.id)}>删</button>
                </li>
              {/each}
            </ul>
          </div>

          <div class="column editor">
            <h2>模板编辑</h2>
            {#if templateDraft}
              <label class="field">
                <span>模板名称</span>
                <input type="text" bind:value={templateDraft.name} />
              </label>

              <label class="field">
                <span>模板 key（可选，唯一）</span>
                <input type="text" bind:value={templateDraft.key} placeholder="例如 addr" />
              </label>

              <label class="field">
                <span>模板内容</span>
                <textarea rows="14" bind:value={templateDraft.content}></textarea>
              </label>

              <div class="actions">
                <button disabled={busy} on:click={saveTemplateDraft}>保存模板</button>
              </div>
            {:else}
              <div class="empty">请先在左侧选择模板</div>
            {/if}
          </div>
        </section>
      {/if}

      {#if tab === "sync"}
        <section class="panel">
          <h2>同步与更新</h2>
          <p>当前本地数据版本：{store.datasetVersion || 0}</p>
          <p>上次同步云端版本：{settings.lastSyncedVersion || 0}</p>

          <div class="actions">
            <button disabled={busy} on:click={() => void runSync("pull")}>从云端拉取并自动合并</button>
            <button disabled={busy} on:click={() => void runSync("push")}>推送数据到云端</button>
          </div>

          <h3>版本更新</h3>
          <p>应用启动时会自动检查更新，发现新版本会询问你是否安装。</p>

          <label class="field">
            <span>更新地址（latest.json）</span>
            <input
              type="text"
              bind:value={settings.updater.endpoint}
              placeholder="https://github.com/owner/repo/releases/latest/download/latest.json"
            />
          </label>

          <label class="field">
            <span>签名公钥</span>
            <textarea
              rows="4"
              bind:value={settings.updater.pubkey}
              placeholder="粘贴 tauri signer generate 生成的公钥"
            ></textarea>
          </label>

          <div class="actions">
            <button disabled={busy} on:click={persistSettings}>保存更新配置</button>
            <button class="subtle" disabled={busy} on:click={runManualUpdateCheck}>检查并安装更新</button>
          </div>
          <p class="hint">提示：更新地址和公钥可在 Windows 调试阶段先手动配置，验证通过后再固化到发布流程。</p>
        </section>
      {/if}
    </section>
  {/if}
</main>

<style>
  .shell {
    width: min(1320px, 100%);
    margin: 0 auto;
    padding: 16px;
    color: #132237;
  }

  .topbar {
    display: flex;
    justify-content: space-between;
    align-items: flex-end;
    margin-bottom: 10px;
  }

  h1 {
    margin: 0;
    font-size: 28px;
    letter-spacing: 0.4px;
  }

  h2 {
    margin: 0 0 10px;
    font-size: 18px;
  }

  h3 {
    margin: 12px 0 10px;
    font-size: 16px;
  }

  p {
    margin: 0 0 8px;
  }

  .hint {
    font-size: 12px;
    color: #4c6786;
    margin-top: 8px;
  }

  .version {
    border-radius: 999px;
    background: #dbf4ee;
    color: #1e5f50;
    padding: 6px 12px;
    font-size: 13px;
  }

  .notice {
    margin-bottom: 10px;
    border-radius: 10px;
    padding: 10px 12px;
    font-size: 14px;
  }

  .notice.info {
    background: #e7f0ff;
    color: #1d4b8d;
  }

  .notice.success {
    background: #e4f8ef;
    color: #1f5c45;
  }

  .notice.error {
    background: #ffe9ea;
    color: #8d2e35;
  }

  .loading {
    border: 1px solid #d0deeb;
    border-radius: 12px;
    background: #ffffffb8;
    padding: 32px;
    text-align: center;
  }

  .content {
    display: grid;
    grid-template-columns: 160px minmax(0, 1fr);
    gap: 10px;
    height: calc(100vh - 120px);
  }

  .tabs {
    display: flex;
    flex-direction: column;
    gap: 8px;
    border-radius: 12px;
    background: #ffffffba;
    border: 1px solid #d0deeb;
    padding: 10px;
  }

  .tabs button {
    text-align: left;
    border: 1px solid transparent;
    border-radius: 8px;
    padding: 10px 12px;
    background: #f4f9ff;
    color: #26405f;
    cursor: pointer;
  }

  .tabs button.active {
    background: #d9ece7;
    color: #184a3f;
    border-color: #8fc5b7;
  }

  .panel {
    border-radius: 12px;
    border: 1px solid #d0deeb;
    background: #ffffffe6;
    padding: 14px;
    overflow: auto;
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 6px;
    margin-bottom: 10px;
  }

  .field > span {
    font-size: 13px;
    color: #45607f;
  }

  input,
  textarea {
    width: 100%;
    border: 1px solid #b9ccdf;
    border-radius: 8px;
    padding: 8px 10px;
    outline: none;
    background: #fff;
    color: #132237;
  }

  input:focus,
  textarea:focus {
    border-color: #4f8ac4;
    box-shadow: 0 0 0 2px #dcecff;
  }

  .row {
    display: grid;
    grid-template-columns: 1fr 130px;
    gap: 8px;
  }

  .checkbox {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 10px;
  }

  .actions {
    display: flex;
    gap: 8px;
    margin-top: 10px;
    flex-wrap: wrap;
  }

  button {
    border: 1px solid #2e6ba8;
    background: #2f76bc;
    color: #fff;
    border-radius: 8px;
    padding: 8px 12px;
    cursor: pointer;
  }

  button.subtle {
    background: #eff6ff;
    color: #284f7f;
    border-color: #9cb9da;
  }

  button:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .templates {
    display: grid;
    grid-template-columns: 280px 320px minmax(0, 1fr);
    gap: 10px;
    min-height: 100%;
  }

  .column {
    border: 1px solid #d0deeb;
    border-radius: 10px;
    padding: 10px;
    background: #f8fbff;
    display: flex;
    flex-direction: column;
    min-height: 0;
  }

  .inline-actions {
    display: grid;
    grid-template-columns: 1fr auto;
    gap: 6px;
    margin-bottom: 8px;
  }

  ul {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 6px;
    overflow: auto;
  }

  li {
    display: grid;
    grid-template-columns: 1fr auto auto;
    gap: 6px;
    align-items: center;
  }

  .template-list li {
    grid-template-columns: 1fr auto;
  }

  .item {
    width: 100%;
    text-align: left;
    border: 1px solid #c7d8ea;
    background: #fff;
    color: #1d3858;
    padding: 7px 10px;
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 8px;
  }

  .item small {
    color: #4e6a8b;
    font-size: 11px;
  }

  li.active .item {
    border-color: #71a4c2;
    background: #eaf5fc;
    color: #163d58;
  }

  .icon {
    border: 1px solid #9ab6d4;
    background: #edf4ff;
    color: #1f4976;
    width: 34px;
    padding: 6px 0;
  }

  .icon.danger {
    border-color: #e1a3a7;
    background: #ffedf0;
    color: #8b2d34;
  }

  .editor textarea {
    resize: vertical;
  }

  .empty {
    margin-top: 20px;
    color: #5e7998;
    font-size: 14px;
  }

  @media (max-width: 1100px) {
    .content {
      grid-template-columns: 1fr;
      height: auto;
    }

    .tabs {
      flex-direction: row;
      overflow-x: auto;
    }

    .templates {
      grid-template-columns: 1fr;
    }

    .shell {
      padding: 10px;
    }
  }
</style>
