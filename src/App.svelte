<script lang="ts">
  import { onMount } from "svelte";
  import {
    checkReleaseVersion,
    getAppVersion,
    loadSettings,
    loadTemplateStore,
    openReleasePage,
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
    ReleaseCheckResult,
    Settings,
    SyncResult,
    TemplateItem,
    TemplateStore
  } from "./lib/types";

  type TabKey = "general" | "templates" | "system";
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
  let latestVersion = "--";
  let checkingVersion = false;

  let updateBanner: ReleaseCheckResult | null = null;

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
      latestVersion = version;
      void runReleaseCheck(true);
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

  const previewOverlay = async (): Promise<void> => {
    try {
      await openOverlay();
    } catch (error) {
      setNotice("error", asErrorMessage(error));
    }
  };

  const onEnterRun = (event: KeyboardEvent, action: () => Promise<void>): void => {
    if (event.key !== "Enter") {
      return;
    }
    event.preventDefault();
    void action();
  };

  const runReleaseCheck = async (silent: boolean): Promise<void> => {
    if (checkingVersion) {
      return;
    }

    checkingVersion = true;
    try {
      const result = await checkReleaseVersion();
      latestVersion = result.latestVersion || result.currentVersion;

      if (result.status === "error") {
        if (!silent) {
          setNotice("error", result.message);
        }
        return;
      }

      if (result.hasUpdate) {
        if (silent) {
          // 启动时静默检查：只显示非阻塞横幅，不弹窗、不自动打开网页
          updateBanner = result;
        } else {
          // 用户手动检查：用 confirm 询问
          setNotice("info", result.message);
          if (window.confirm(`发现新版本 ${result.latestVersion || "--"}，是否立即打开发布页？`)) {
            await openReleaseByResult(result);
          }
        }
        return;
      }

      if (!silent) {
        setNotice("info", result.message);
      }
    } catch (error) {
      if (!silent) {
        setNotice("error", asErrorMessage(error));
      }
    } finally {
      checkingVersion = false;
    }
  };

  const dismissUpdateBanner = (): void => {
    updateBanner = null;
  };

  const confirmUpdateBanner = async (): Promise<void> => {
    const banner = updateBanner;
    updateBanner = null;
    await openReleaseByResult(banner ?? undefined);
  };

  const openReleaseByResult = async (result?: ReleaseCheckResult): Promise<void> => {
    try {
      await openReleasePage();
      if (!result) {
        setNotice("info", "已打开发布页，请下载最新安装包");
      }
    } catch (error) {
      const fallback = result?.releaseUrl || "https://github.com/FB208/quickCV/releases";
      window.open(fallback, "_blank", "noopener,noreferrer");
      if (!result) {
        setNotice("info", "已通过浏览器打开发布页");
      }
      if (error) {
        // fallback already handled
      }
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

  {#if updateBanner}
    <div class="update-banner">
      <span class="ms-icon banner-icon">system_update</span>
      <span class="banner-text">检测到新版本 <strong>{updateBanner.latestVersion || "--"}</strong>（当前 {updateBanner.currentVersion}）</span>
      <button class="banner-btn confirm" on:click={() => void confirmUpdateBanner()}>前往下载</button>
      <button class="banner-btn dismiss" on:click={dismissUpdateBanner}>忽略</button>
    </div>
  {/if}

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
        <button class:active={tab === "system"} on:click={() => (tab = "system")}>系统</button>
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

          <div class="startup-card">
            <div class="startup-text">
              <strong>开机自动启动</strong>
              <small>启动后自动在系统托盘待命</small>
            </div>
            <label class="switch" title="开机自动启动">
              <input type="checkbox" bind:checked={settings.launchAtStartup} />
              <span class="slider"></span>
            </label>
          </div>

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
          <section class="template-sync-strip">
            <div class="sync-stats">
              <span class="sync-badge">模板数据同步</span>
              <span>本地版本 {store.datasetVersion || 0}</span>
              <span>云端基线 {settings.lastSyncedVersion || 0}</span>
            </div>
            <div class="sync-actions">
              <button class="subtle" disabled={busy} on:click={() => void runSync("pull")}>从云端拉取并合并</button>
              <button class="subtle" disabled={busy} on:click={() => void runSync("push")}>推送模板到云端</button>
            </div>
          </section>

          <div class="column folders">
            <h2>文件夹</h2>
            <input type="text" bind:value={folderSearch} placeholder="搜索文件夹" />
            <div class="inline-actions">
              <input
                type="text"
                bind:value={newFolderName}
                placeholder="新建文件夹"
                on:keydown={(event) => onEnterRun(event, createFolder)}
              />
              <button disabled={busy} on:click={createFolder}>
                <span class="ms-icon">create_new_folder</span>
                新增
              </button>
            </div>
            <ul>
              {#each filteredFolders as folder}
                <li class:active={folder.id === selectedFolderId}>
                  <button class="item" on:click={() => (selectedFolderId = folder.id)}>{folder.name}</button>
                  <button class="icon" title="重命名" on:click={() => void renameFolder(folder.id)}>
                    <span class="ms-icon">edit</span>
                  </button>
                  <button class="icon danger" title="删除" on:click={() => void removeFolder(folder.id)}>
                    <span class="ms-icon">delete</span>
                  </button>
                </li>
              {/each}
            </ul>
          </div>

          <div class="column template-list">
            <h2>模板列表</h2>
            <input type="text" bind:value={templateSearch} placeholder="搜索模板、key、内容" />
            <div class="inline-actions">
              <input
                type="text"
                bind:value={newTemplateName}
                placeholder="新建模板"
                on:keydown={(event) => onEnterRun(event, createTemplate)}
              />
              <button disabled={busy} on:click={createTemplate}>
                <span class="ms-icon">add_notes</span>
                新增
              </button>
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
                  <button class="icon danger" title="删除" on:click={() => void removeTemplate(item.id)}>
                    <span class="ms-icon">delete</span>
                  </button>
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

      {#if tab === "system"}
        <section class="panel">
          <h2>系统</h2>
          <div class="version-grid">
            <div>
              <span class="version-label">当前软件版本</span>
              <strong>v{appVersion}</strong>
            </div>
            <div>
              <span class="version-label">最新线上版本</span>
              <strong>{latestVersion === "--" ? "--" : `v${latestVersion}`}</strong>
            </div>
          </div>

          <div class="actions">
            <button class="subtle" disabled={checkingVersion} on:click={() => void runReleaseCheck(false)}>
              <span class="ms-icon">system_update</span>
              {checkingVersion ? "检查中..." : "检查版本"}
            </button>
            <button class="subtle" on:click={() => void openReleaseByResult()}>
              <span class="ms-icon">open_in_new</span>
              打开 Release 页面
            </button>
          </div>
          <p class="hint">系统页仅保留版本信息与更新入口；模板数据同步已移到「模板管理」。</p>
        </section>
      {/if}
    </section>
  {/if}
</main>

<style>
  .shell {
    width: min(1320px, 100%);
    margin: 0 auto;
    padding: 10px;
    color: #132237;
  }

  .topbar {
    display: flex;
    justify-content: space-between;
    align-items: flex-end;
    margin-bottom: 6px;
  }

  h1 {
    margin: 0;
    font-size: 24px;
    letter-spacing: 0.25px;
  }

  h2 {
    margin: 0 0 8px;
    font-size: 16px;
  }

  h3 {
    margin: 8px 0 6px;
    font-size: 14px;
  }

  p {
    margin: 0 0 6px;
  }

  .hint {
    font-size: 11px;
    color: #4c6786;
    margin-top: 6px;
  }

  .version-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 8px;
    margin-bottom: 8px;
  }

  .version-grid > div {
    border: 1px solid #c9d8e8;
    border-radius: 8px;
    background: #f6fbff;
    padding: 7px 8px;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .version-label {
    font-size: 11px;
    color: #5b7695;
  }

  .version-grid strong {
    font-size: 14px;
    color: #12385b;
  }

  .startup-card {
    display: flex;
    justify-content: space-between;
    align-items: center;
    border: 1px solid #c9d9e9;
    border-radius: 9px;
    background: linear-gradient(135deg, #f7fbff 0%, #eef8f4 100%);
    padding: 8px 10px;
    margin-bottom: 8px;
    gap: 10px;
  }

  .startup-text {
    display: flex;
    flex-direction: column;
    min-width: 0;
    white-space: nowrap;
  }

  .startup-text strong {
    font-size: 13px;
    color: #173b5f;
    font-weight: 600;
  }

  .startup-text small {
    font-size: 11px;
    color: #5a7594;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .switch {
    position: relative;
    width: 44px;
    height: 24px;
    flex: 0 0 auto;
  }

  .switch input {
    opacity: 0;
    width: 0;
    height: 0;
  }

  .slider {
    position: absolute;
    inset: 0;
    border-radius: 999px;
    background: #c3d2e1;
    transition: 0.2s ease;
    cursor: pointer;
  }

  .slider::before {
    content: "";
    position: absolute;
    width: 18px;
    height: 18px;
    left: 3px;
    top: 3px;
    border-radius: 50%;
    background: #fff;
    box-shadow: 0 1px 4px #20384f3a;
    transition: 0.2s ease;
  }

  .switch input:checked + .slider {
    background: #2f76bc;
  }

  .switch input:checked + .slider::before {
    transform: translateX(20px);
  }

  .version {
    border-radius: 999px;
    background: #dbf4ee;
    color: #1e5f50;
    padding: 4px 10px;
    font-size: 12px;
  }

  .update-banner {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 6px;
    padding: 9px 12px;
    border-radius: 9px;
    background: linear-gradient(135deg, #e7f4ff 0%, #ddf0f8 100%);
    border: 1px solid #b3d8f0;
    animation: bannerSlideIn 0.35s ease-out;
  }

  @keyframes bannerSlideIn {
    from {
      opacity: 0;
      transform: translateY(-8px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }

  .banner-icon {
    font-size: 20px;
    color: #2277bb;
    flex-shrink: 0;
  }

  .banner-text {
    flex: 1;
    font-size: 13px;
    color: #1a4a6e;
  }

  .banner-text strong {
    color: #125a90;
  }

  .banner-btn {
    padding: 4px 12px;
    border-radius: 6px;
    font-size: 12px;
    cursor: pointer;
    border: none;
    flex-shrink: 0;
    transition: background 0.15s ease, transform 0.1s ease;
  }

  .banner-btn:active {
    transform: scale(0.96);
  }

  .banner-btn.confirm {
    background: #2277bb;
    color: #fff;
  }

  .banner-btn.confirm:hover {
    background: #1b6aaa;
  }

  .banner-btn.dismiss {
    background: transparent;
    color: #5b7f9e;
    border: 1px solid #b3c8da;
  }

  .banner-btn.dismiss:hover {
    background: #f0f6fb;
  }

  .notice {
    margin-bottom: 6px;
    border-radius: 8px;
    padding: 8px 10px;
    font-size: 13px;
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
    border-radius: 10px;
    background: #ffffffb8;
    padding: 20px;
    text-align: center;
  }

  .content {
    display: grid;
    grid-template-columns: 146px minmax(0, 1fr);
    gap: 8px;
    height: calc(100vh - 92px);
  }

  .tabs {
    display: flex;
    flex-direction: column;
    gap: 6px;
    border-radius: 10px;
    background: linear-gradient(180deg, #ffffffd1 0%, #f1f7ffbf 100%);
    border: 1px solid #d0deeb;
    padding: 8px;
  }

  .tabs button {
    text-align: left;
    border: 1px solid transparent;
    border-radius: 7px;
    padding: 8px 10px;
    background: #f4f9ff;
    color: #26405f;
    cursor: pointer;
    font-size: 13px;
  }

  .tabs button.active {
    background: linear-gradient(145deg, #d8eee7 0%, #d6eaf9 100%);
    color: #184a3f;
    border-color: #8fc5b7;
  }

  .panel {
    border-radius: 10px;
    border: 1px solid #d0deeb;
    background: linear-gradient(160deg, #ffffffe6 0%, #f6fbffe8 100%);
    padding: 10px;
    overflow: auto;
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 4px;
    margin-bottom: 8px;
  }

  .field > span {
    font-size: 12px;
    color: #45607f;
  }

  input,
  textarea {
    width: 100%;
    border: 1px solid #b9ccdf;
    border-radius: 7px;
    padding: 6px 8px;
    outline: none;
    background: #fff;
    color: #132237;
    font-size: 13px;
  }

  input:focus,
  textarea:focus {
    border-color: #4f8ac4;
    box-shadow: 0 0 0 2px #dcecff;
  }

  .row {
    display: grid;
    grid-template-columns: 1fr 118px;
    gap: 6px;
  }

  .actions {
    display: flex;
    gap: 6px;
    margin-top: 8px;
    flex-wrap: wrap;
  }

  button {
    border: 1px solid #2e6ba8;
    background: #2f76bc;
    color: #fff;
    border-radius: 7px;
    padding: 6px 10px;
    cursor: pointer;
    font-size: 12px;
    display: inline-flex;
    align-items: center;
    gap: 4px;
    line-height: 1;
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
    grid-template-columns: 240px 300px minmax(0, 1fr);
    grid-template-rows: auto minmax(0, 1fr);
    gap: 8px;
    min-height: 100%;
  }

  .template-sync-strip {
    grid-column: 1 / -1;
    border: 1px solid #c9d8e8;
    border-radius: 8px;
    background: linear-gradient(90deg, #f8fcff 0%, #eef7ff 100%);
    padding: 6px 8px;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    flex-wrap: wrap;
  }

  .sync-stats {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
    font-size: 11px;
    color: #4d6d8f;
  }

  .sync-badge {
    border: 1px solid #9fc1df;
    border-radius: 999px;
    background: #e9f4ff;
    color: #215179;
    padding: 2px 8px;
    font-weight: 600;
  }

  .sync-actions {
    display: inline-flex;
    gap: 6px;
    flex-wrap: wrap;
  }

  .column {
    border: 1px solid #d0deeb;
    border-radius: 8px;
    padding: 8px;
    background: linear-gradient(180deg, #f8fbff 0%, #f5f9ff 100%);
    display: flex;
    flex-direction: column;
    min-height: 0;
  }

  .column h2 {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 6px;
  }

  .inline-actions {
    display: grid;
    grid-template-columns: 1fr auto;
    gap: 5px;
    margin-bottom: 6px;
  }

  ul {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 4px;
    overflow: auto;
  }

  li {
    display: grid;
    grid-template-columns: 1fr auto auto;
    gap: 4px;
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
    padding: 5px 8px;
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 6px;
    font-size: 12px;
    border-radius: 6px;
  }

  .item small {
    color: #4e6a8b;
    font-size: 10px;
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
    width: 30px;
    padding: 5px 0;
    display: inline-flex;
    justify-content: center;
    align-items: center;
  }

  .icon .ms-icon {
    font-size: 16px;
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
    margin-top: 12px;
    color: #5e7998;
    font-size: 13px;
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
      padding: 8px;
    }
  }
</style>
