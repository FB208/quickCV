<script lang="ts">
  import { onMount } from "svelte";
  import { listen } from "@tauri-apps/api/event";
  import {
    checkReleaseVersion,
    getAppVersion,
    loadSettings,
    loadTemplateStore,
    openReleasePage,
    openConfigFolder,
    openOverlay,
    saveSettings,
    saveTemplateStore,
    syncPull,
    syncPush,
    testWebDav
  } from "./lib/api";
  import { asErrorMessage } from "./lib/errors";
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
  let syncMode: "pull" | "push" | null = null;
  let syncStage = "";

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
    let unlistenTabNavigate: (() => void) | undefined;
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

    const bindTabNavigate = async (): Promise<void> => {
      if (!("__TAURI_INTERNALS__" in window)) {
        return;
      }
      unlistenTabNavigate = await listen<string>("navigate_main_tab", (event) => {
        if (event.payload === "general" || event.payload === "templates" || event.payload === "system") {
          tab = event.payload;
        }
      });
    };

    window.addEventListener("keydown", keyRecorder, true);
    void bindTabNavigate();
    void bootstrap();
    return () => {
      if (unlistenTabNavigate) {
        unlistenTabNavigate();
      }
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

  const setNotice = (type: NoticeType, message: string): void => {
    noticeType = type;
    notice = message;
  };

  const persistSettings = async (): Promise<void> => {
    busy = true;
    try {
      const requestedLaunchAtStartup = settings.launchAtStartup;
      settings = await saveSettings(settings);
      if (requestedLaunchAtStartup !== settings.launchAtStartup) {
        setNotice("info", "设置已保存，但开机启动状态更新失败，请查看日志");
        return;
      }
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
    syncMode = mode;
    syncStage = mode === "pull" ? "正在从云端拉取并合并数据..." : "正在推送本地数据到云端...";
    try {
      const result: SyncResult = mode === "pull" ? await syncPull() : await syncPush();
      syncStage = "正在刷新本地数据...";
      settings = await loadSettings();
      store = await loadTemplateStore();
      const noticeLevel: NoticeType = result.blocked
        ? "error"
        : result.level === "warn"
          ? "info"
          : "success";
      setNotice(noticeLevel, makeSyncMessage(result));
    } catch (error) {
      setNotice("error", asErrorMessage(error));
    } finally {
      busy = false;
      syncMode = null;
      syncStage = "";
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

  const openConfigFolderHandler = async (): Promise<void> => {
    try {
      await openConfigFolder();
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
        <button class="tab-btn" class:active={tab === "general"} on:click={() => (tab = "general")}>
          <span class="ms-icon">tune</span>常规设置
        </button>
        <button class="tab-btn" class:active={tab === "templates"} on:click={() => (tab = "templates")}>
          <span class="ms-icon">dashboard</span>模板管理
        </button>
        <button class="tab-btn" class:active={tab === "system"} on:click={() => (tab = "system")}>
          <span class="ms-icon">computer</span>系统
        </button>
      </nav>

      {#if tab === "general"}
        <section class="panel panel-page">
          <div class="page-head">
            <h2><span class="ms-icon">tune</span>常规设置</h2>
            <p>统一管理快捷键、启动行为与 WebDAV 同步参数。</p>
          </div>

          <div class="general-grid">
            <section class="general-card">
              <h3 class="card-title"><span class="ms-icon">keyboard</span>快捷入口</h3>
              <label class="qc-field">
                <span>全局快捷键</span>
                <div class="row">
                  <input class="qc-input" type="text" value={recordingShortcut ? "请按下快捷键..." : settings.shortcut} readonly />
                  <button class="qc-btn" disabled={busy} on:click={() => (recordingShortcut = true)}>
                    <span class="ms-icon">keyboard</span>
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

              <div class="qc-actions">
                <button class="qc-btn" disabled={busy} on:click={persistSettings}>
                  <span class="ms-icon">save</span>
                  保存设置
                </button>
                <button class="qc-btn qc-btn-subtle" disabled={busy} on:click={previewOverlay}>
                  <span class="ms-icon">visibility</span>
                  预览快捷浮窗
                </button>
              </div>
            </section>

            <section class="general-card">
              <h3 class="card-title"><span class="ms-icon">cloud_sync</span>WebDAV 配置</h3>
              <label class="qc-field">
                <span>地址</span>
                <input class="qc-input" type="text" bind:value={settings.webdav.url} placeholder="https://dav.example.com/path" />
              </label>

              <label class="qc-field">
                <span>用户名</span>
                <input class="qc-input" type="text" bind:value={settings.webdav.username} autocomplete="off" />
              </label>

              <label class="qc-field">
                <span>密码</span>
                <input class="qc-input" type="password" bind:value={settings.webdav.password} autocomplete="off" />
              </label>

              <label class="qc-field">
                <span>远端文件名</span>
                <input class="qc-input" type="text" bind:value={settings.webdav.remoteFile} placeholder="quickcv-data.json" />
              </label>

              <div class="qc-actions">
                <button class="qc-btn qc-btn-subtle" disabled={busy} on:click={runWebDavTest}>
                  <span class="ms-icon">network_check</span>
                  测试 WebDAV 连通性
                </button>
              </div>
            </section>
          </div>
        </section>
      {/if}

      {#if tab === "templates"}
        <section class="panel tpl-panel">
          <!-- 顶部同步栏 -->
          <section class="tpl-sync-bar">
            <div class="tpl-sync-left">
              <span class="ms-icon tpl-sync-icon">cloud_sync</span>
              <span class="tpl-sync-ver">本地 <strong>{store.datasetVersion || 0}</strong></span>
              <span class="tpl-sync-sep">|</span>
              <span class="tpl-sync-ver">云端 <strong>{settings.lastSyncedVersion || 0}</strong></span>
            </div>
            <div class="tpl-sync-right">
              <button class="tpl-sync-btn" disabled={busy} on:click={() => void runSync("pull")}>
                <span class="ms-icon" class:syncing={busy && syncMode === "pull"}>{busy && syncMode === "pull" ? "sync" : "cloud_download"}</span>
                {busy && syncMode === "pull" ? "拉取中..." : "拉取合并"}
              </button>
              <button class="tpl-sync-btn" disabled={busy} on:click={() => void runSync("push")}>
                <span class="ms-icon" class:syncing={busy && syncMode === "push"}>{busy && syncMode === "push" ? "sync" : "cloud_upload"}</span>
                {busy && syncMode === "push" ? "推送中..." : "推送云端"}
              </button>
            </div>
            {#if busy && syncMode}
              <div class="tpl-sync-progress" role="status" aria-live="polite">
                <span class="sync-dot"></span>
                <span>{syncStage}</span>
              </div>
            {/if}
          </section>

          <!-- 主体区域 -->
          <div class="tpl-body">
            <!-- 左侧：文件夹 -->
            <aside class="tpl-sidebar">
              <div class="tpl-sidebar-header">
                <h3>
                  <span class="ms-icon">folder</span> 文件夹
                </h3>
                <button class="tpl-add-btn" title="新建文件夹" disabled={busy} on:click={() => { newFolderName = "新文件夹"; void createFolder(); }}>
                  <span class="ms-icon">add</span>
                </button>
              </div>
              <div class="tpl-search-wrap">
                <span class="ms-icon tpl-search-icon">search</span>
                <input type="text" bind:value={folderSearch} placeholder="搜索文件夹…" class="tpl-search" />
              </div>
              <div class="tpl-folder-list">
                {#each filteredFolders as folder}
                  <div
                    class="tpl-folder-item"
                    class:active={folder.id === selectedFolderId}
                    on:click={() => (selectedFolderId = folder.id)}
                    on:keydown={(e) => e.key === 'Enter' && (selectedFolderId = folder.id)}
                    role="button"
                    tabindex="0"
                  >
                    <span class="ms-icon tpl-folder-icon">{folder.id === selectedFolderId ? 'folder_open' : 'folder'}</span>
                    <span class="tpl-folder-name">{folder.name}</span>
                    <span class="tpl-folder-count">{store.templates.filter(t => t.folderId === folder.id && t.deletedAt === null).length}</span>
                    <div class="tpl-folder-actions">
                      <button class="tpl-icon-btn" title="重命名" on:click|stopPropagation={() => void renameFolder(folder.id)}>
                        <span class="ms-icon">edit</span>
                      </button>
                      <button class="tpl-icon-btn danger" title="删除" on:click|stopPropagation={() => void removeFolder(folder.id)}>
                        <span class="ms-icon">delete</span>
                      </button>
                    </div>
                  </div>
                {/each}
                {#if filteredFolders.length === 0}
                  <div class="tpl-empty-hint">
                    <span class="ms-icon">folder_off</span>
                    <span>暂无文件夹</span>
                  </div>
                {/if}
              </div>
            </aside>

            <!-- 右侧：模板列表 + 编辑 -->
            <div class="tpl-main">
              <!-- 模板列表头部 -->
              <div class="tpl-main-header">
                <div class="tpl-main-title">
                  <h3>
                    <span class="ms-icon">description</span>
                    {activeFolders.find(f => f.id === selectedFolderId)?.name || '模板'}
                  </h3>
                  <span class="tpl-template-count">{filteredTemplates.length} 个模板</span>
                </div>
                <div class="tpl-main-actions">
                  <div class="tpl-search-wrap">
                    <span class="ms-icon tpl-search-icon">search</span>
                    <input type="text" bind:value={templateSearch} placeholder="搜索模板名、key、内容…" class="tpl-search" />
                  </div>
                  <button class="tpl-create-btn" disabled={busy || !selectedFolderId} on:click={createTemplate}>
                    <span class="ms-icon">add_circle</span> 新建模板
                  </button>
                </div>
              </div>

              <!-- 模板卡片网格 + 编辑器 -->
              <div class="tpl-content-area" class:has-editor={templateDraft !== null}>
                <div class="tpl-card-grid">
                  {#each filteredTemplates as item}
                    <div
                      class="tpl-card"
                      class:selected={item.id === selectedTemplateId}
                      on:click={() => (selectedTemplateId = item.id)}
                      on:keydown={(e) => e.key === 'Enter' && (selectedTemplateId = item.id)}
                      role="button"
                      tabindex="0"
                    >
                      <div class="tpl-card-header">
                        <span class="tpl-card-name">{item.name}</span>
                        <button class="tpl-icon-btn danger sm" title="删除" on:click|stopPropagation={() => void removeTemplate(item.id)}>
                          <span class="ms-icon">close</span>
                        </button>
                      </div>
                      {#if item.key}
                        <div class="tpl-card-key">
                          <span class="ms-icon">key</span> {item.key}
                        </div>
                      {/if}
                      <div class="tpl-card-preview">{item.content || '（空内容）'}</div>
                    </div>
                  {/each}
                  {#if filteredTemplates.length === 0}
                    <div class="tpl-empty-card">
                      <span class="ms-icon">note_add</span>
                      <p>{selectedFolderId ? '暂无模板，点击上方按钮创建' : '请先选择一个文件夹'}</p>
                    </div>
                  {/if}
                </div>

                <!-- 编辑面板 -->
                {#if templateDraft}
                  <div class="tpl-editor">
                    <div class="tpl-editor-header">
                      <h4>
                        <span class="ms-icon">edit_note</span> 编辑模板
                      </h4>
                      <button class="tpl-icon-btn" title="关闭编辑器" on:click={() => { selectedTemplateId = ''; templateDraft = null; }}>
                        <span class="ms-icon">close</span>
                      </button>
                    </div>
                    <div class="tpl-editor-body">
                      <label class="tpl-field">
                        <span class="tpl-field-label">
                          <span class="ms-icon">badge</span> 模板名称
                        </span>
                        <input type="text" bind:value={templateDraft.name} class="tpl-input" />
                      </label>

                      <label class="tpl-field">
                        <span class="tpl-field-label">
                          <span class="ms-icon">key</span> 触发 Key <small>（可选，全局唯一）</small>
                        </span>
                        <input type="text" bind:value={templateDraft.key} placeholder="例如 addr" class="tpl-input" />
                      </label>

                      <label class="tpl-field tpl-field-grow">
                        <span class="tpl-field-label">
                          <span class="ms-icon">article</span> 模板内容
                        </span>
                        <textarea bind:value={templateDraft.content} class="tpl-textarea"></textarea>
                      </label>
                    </div>
                    <div class="tpl-editor-footer">
                      <button class="tpl-save-btn" disabled={busy} on:click={saveTemplateDraft}>
                        <span class="ms-icon">save</span> 保存模板
                      </button>
                    </div>
                  </div>
                {/if}
              </div>
            </div>
          </div>
        </section>
      {/if}

      {#if tab === "system"}
        <section class="panel panel-page">
          <div class="page-head">
            <h2><span class="ms-icon">computer</span>系统</h2>
            <p>查看版本状态并处理更新与配置入口。</p>
          </div>

          <div class="system-grid">
            <div class="version-card">
              <span class="version-label">当前软件版本</span>
              <strong>v{appVersion}</strong>
              <small>本地安装版本</small>
            </div>
            <div class="version-card">
              <span class="version-label">最新线上版本</span>
              <strong>{latestVersion === "--" ? "--" : `v${latestVersion}`}</strong>
              <small>来自 GitHub Release</small>
            </div>

            <section class="system-card">
              <h3 class="card-title"><span class="ms-icon">system_update</span>更新与维护</h3>
              <div class="qc-actions">
                <button class="qc-btn qc-btn-subtle" disabled={checkingVersion} on:click={() => void runReleaseCheck(false)}>
                  <span class="ms-icon">system_update</span>
                  {checkingVersion ? "检查中..." : "检查版本"}
                </button>
                <button class="qc-btn qc-btn-subtle" on:click={() => void openReleaseByResult()}>
                  <span class="ms-icon">open_in_new</span>
                  打开 Release 页面
                </button>
                <button class="qc-btn qc-btn-subtle" on:click={() => void openConfigFolderHandler()}>
                  <span class="ms-icon">folder_open</span>
                  打开配置文件
                </button>
              </div>
              <p class="hint">系统页保留版本信息与更新入口；模板同步入口已合并到「模板管理」。</p>
            </section>
          </div>
        </section>
      {/if}
    </section>
  {/if}
</main>

<style>
  .shell {
    width: min(1320px, 100%);
    margin: 0 auto;
    padding: 12px;
    color: var(--qc-text-primary);
    display: flex;
    flex-direction: column;
    height: 100vh;
    gap: 8px;
  }

  .topbar {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 4px 2px;
  }

  .topbar h1 {
    margin: 0;
    font-size: 24px;
    letter-spacing: 0.3px;
    color: #16395a;
  }

  .topbar p {
    margin: 2px 0 0;
    color: #5c7b99;
    font-size: 12px;
  }

  .version {
    border-radius: 999px;
    background: linear-gradient(135deg, #ddf5ee 0%, #d9edf8 100%);
    color: #1f5f50;
    border: 1px solid #c0e2d9;
    padding: 5px 12px;
    font-size: 12px;
    font-weight: 600;
  }

  .update-banner {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 10px 12px;
    border-radius: 10px;
    background: linear-gradient(135deg, #ebf6ff 0%, #e0f1fb 100%);
    border: 1px solid #bbd9ee;
    box-shadow: 0 4px 16px rgba(44, 106, 162, 0.1);
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
    color: #1f4f75;
  }

  .banner-text strong {
    color: #135d95;
  }

  .banner-btn {
    padding: 4px 12px;
    border-radius: 7px;
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
    border-radius: 9px;
    padding: 9px 12px;
    font-size: 13px;
    box-shadow: 0 3px 12px rgba(32, 69, 105, 0.08);
  }

  .notice.info {
    background: #e8f1ff;
    color: #1d4b8d;
    border: 1px solid #c7daf6;
  }

  .notice.success {
    background: #e6f8ef;
    color: #1f5c45;
    border: 1px solid #bfebd5;
  }

  .notice.error {
    background: #ffe9ea;
    color: #8d2e35;
    border: 1px solid #f5cbd0;
  }

  .loading {
    border: 1px solid var(--qc-border-soft);
    border-radius: 14px;
    background: linear-gradient(160deg, #fffffff0 0%, #f2f8fff0 100%);
    padding: 24px;
    text-align: center;
    color: #36577a;
    box-shadow: var(--qc-shadow-soft);
  }

  .content {
    display: grid;
    grid-template-columns: 168px minmax(0, 1fr);
    gap: 8px;
    flex: 1;
    min-height: 0;
  }

  .tabs {
    display: flex;
    flex-direction: column;
    gap: 7px;
    border-radius: 12px;
    background: linear-gradient(180deg, #ffffffd1 0%, #eff6ffbf 100%);
    border: 1px solid var(--qc-border-soft);
    box-shadow: var(--qc-shadow-soft);
    padding: 8px;
  }

  .tab-btn {
    text-align: left;
    border: 1px solid transparent;
    border-radius: 8px;
    padding: 9px 10px;
    background: #f3f8ff;
    color: #284664;
    cursor: pointer;
    font-size: 13px;
    display: flex;
    align-items: center;
    gap: 6px;
    transition: all 0.2s ease;
  }

  .tab-btn .ms-icon {
    font-size: 17px;
    color: #4e89b9;
  }

  .tab-btn:hover {
    border-color: #c4d9ee;
    background: #f7fbff;
    transform: translateX(1px);
  }

  .tab-btn.active {
    background: linear-gradient(145deg, #d8eee7 0%, #d6eaf9 100%);
    color: #184a3f;
    border-color: #9bc8bd;
    box-shadow: 0 4px 10px rgba(38, 100, 154, 0.12);
  }

  .tab-btn.active .ms-icon {
    color: #2573ab;
  }

  .panel {
    border-radius: 12px;
    border: 1px solid var(--qc-border-soft);
    background: linear-gradient(160deg, #ffffffe8 0%, #f6fbffe8 100%);
    box-shadow: var(--qc-shadow-soft);
    overflow: auto;
    min-height: 0;
  }

  .panel-page {
    padding: 14px;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .page-head {
    display: flex;
    justify-content: space-between;
    align-items: center;
    flex-wrap: wrap;
    gap: 8px;
    border-bottom: 1px solid #e4eef8;
    padding-bottom: 10px;
  }

  .page-head h2 {
    margin: 0;
    font-size: 17px;
    color: #1a3e63;
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }

  .page-head h2 .ms-icon {
    font-size: 20px;
    color: #3c87c0;
  }

  .page-head p {
    margin: 0;
    font-size: 12px;
    color: #6684a3;
  }

  .general-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 12px;
    align-items: start;
  }

  .general-card,
  .system-card {
    border: 1px solid #d7e4f1;
    border-radius: 12px;
    background: linear-gradient(165deg, #ffffff 0%, #f5faff 100%);
    box-shadow: 0 6px 20px rgba(39, 96, 148, 0.08);
    padding: 12px;
    display: flex;
    flex-direction: column;
    gap: 9px;
  }

  .card-title {
    margin: 0;
    font-size: 14px;
    color: #1e476d;
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .card-title .ms-icon {
    font-size: 18px;
    color: #4a8ec3;
  }

  .row {
    display: grid;
    grid-template-columns: 1fr auto;
    gap: 8px;
  }

  .startup-card {
    display: flex;
    justify-content: space-between;
    align-items: center;
    border: 1px solid #cbdeee;
    border-radius: 10px;
    background: linear-gradient(135deg, #f6fbff 0%, #edf7f3 100%);
    padding: 9px 10px;
    gap: 10px;
  }

  .startup-text {
    display: flex;
    flex-direction: column;
    min-width: 0;
    white-space: nowrap;
  }

  .startup-text strong {
    font-size: 12px;
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

  .system-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 10px;
  }

  .version-card {
    border: 1px solid #c8ddef;
    border-radius: 12px;
    background: linear-gradient(140deg, #f7fbff 0%, #eff7ff 100%);
    padding: 10px 12px;
    display: flex;
    flex-direction: column;
    gap: 3px;
  }

  .version-label {
    font-size: 11px;
    color: #5b7695;
  }

  .version-card strong {
    font-size: 18px;
    color: #173e62;
  }

  .version-card small {
    color: #7390ad;
    font-size: 11px;
  }

  .system-card {
    grid-column: 1 / -1;
  }

  .hint {
    font-size: 12px;
    color: #5f7d99;
    margin: 0;
    line-height: 1.5;
  }

  /* ===== 模板管理 - 全新布局 ===== */

  .tpl-panel {
    display: flex;
    flex-direction: column;
    height: 100%;
    padding: 0 !important;
    overflow: hidden;
  }

  /* --- 顶部同步栏 --- */
  .tpl-sync-bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    padding: 8px 14px;
    background: linear-gradient(135deg, #f0f7ff 0%, #e8f4f8 100%);
    border-bottom: 1px solid #d5e3f0;
    flex-wrap: wrap;
    flex-shrink: 0;
  }

  .tpl-sync-left {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 12px;
    color: #4a6a8a;
  }

  .tpl-sync-icon {
    font-size: 20px;
    color: #3a8fd4;
    animation: syncPulse 3s ease-in-out infinite;
  }

  @keyframes syncPulse {
    0%, 100% { opacity: 0.7; }
    50% { opacity: 1; }
  }

  .tpl-sync-ver strong {
    color: #1a4a6e;
    font-weight: 700;
  }

  .tpl-sync-sep {
    color: #c0d0e0;
    font-weight: 300;
  }

  .tpl-sync-right {
    display: flex;
    gap: 6px;
  }

  .tpl-sync-progress {
    width: 100%;
    display: inline-flex;
    align-items: center;
    gap: 8px;
    font-size: 12px;
    color: #2f5f87;
    padding-top: 2px;
  }

  .sync-dot {
    width: 8px;
    height: 8px;
    border-radius: 999px;
    background: #2f8cd4;
    box-shadow: 0 0 0 rgba(47, 140, 212, 0.45);
    animation: syncDotPulse 1.2s ease-in-out infinite;
    flex-shrink: 0;
  }

  @keyframes syncDotPulse {
    0% {
      transform: scale(0.86);
      box-shadow: 0 0 0 0 rgba(47, 140, 212, 0.45);
    }
    70% {
      transform: scale(1);
      box-shadow: 0 0 0 8px rgba(47, 140, 212, 0);
    }
    100% {
      transform: scale(0.86);
      box-shadow: 0 0 0 0 rgba(47, 140, 212, 0);
    }
  }

  .tpl-sync-btn {
    border: 1px solid #b8d4ea !important;
    background: rgba(255,255,255,0.85) !important;
    color: #2a6090 !important;
    border-radius: 8px !important;
    padding: 5px 12px !important;
    font-size: 12px !important;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 4px;
    cursor: pointer;
    backdrop-filter: blur(6px);
    transition: all 0.2s ease;
  }

  .tpl-sync-btn .ms-icon.syncing {
    animation: syncSpin 0.9s linear infinite;
  }

  @keyframes syncSpin {
    from {
      transform: rotate(0deg);
    }
    to {
      transform: rotate(360deg);
    }
  }

  .tpl-sync-btn:hover:not(:disabled) {
    background: #fff !important;
    border-color: #7ab3da !important;
    box-shadow: 0 2px 8px rgba(42, 96, 144, 0.12);
    transform: translateY(-1px);
  }

  .tpl-sync-btn:active:not(:disabled) {
    transform: translateY(0) scale(0.98);
  }

  /* --- 主体区域 --- */
  .tpl-body {
    display: grid;
    grid-template-columns: 220px minmax(0, 1fr);
    flex: 1;
    min-height: 0;
  }

  /* --- 左侧文件夹侧栏 --- */
  .tpl-sidebar {
    display: flex;
    flex-direction: column;
    border-right: 1px solid #dde8f2;
    background: linear-gradient(180deg, #f8fbff 0%, #f2f6fb 100%);
    overflow: hidden;
  }

  .tpl-sidebar-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 10px 12px 6px;
    flex-shrink: 0;
  }

  .tpl-sidebar-header h3 {
    display: flex;
    align-items: center;
    gap: 6px;
    margin: 0;
    font-size: 13px;
    font-weight: 700;
    color: #1d3a58;
  }

  .tpl-sidebar-header h3 .ms-icon {
    font-size: 18px;
    color: #4a90c4;
  }

  .tpl-add-btn {
    width: 28px;
    height: 28px;
    border-radius: 8px !important;
    border: 1px dashed #b0c8e0 !important;
    background: transparent !important;
    color: #5a8ab5 !important;
    padding: 0 !important;
    display: inline-flex !important;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    transition: all 0.2s ease;
  }

  .tpl-add-btn:hover:not(:disabled) {
    border-style: solid !important;
    background: #e4f0fb !important;
    color: #2a6a9e !important;
    transform: scale(1.08);
  }

  .tpl-add-btn .ms-icon {
    font-size: 18px;
  }

  /* --- 搜索框 --- */
  .tpl-search-wrap {
    position: relative;
    padding: 0 10px;
    margin-bottom: 6px;
    flex-shrink: 0;
  }

  .tpl-search-icon {
    position: absolute;
    left: 18px;
    top: 50%;
    transform: translateY(-50%);
    font-size: 16px !important;
    color: #96aec4;
    pointer-events: none;
  }

  .tpl-search {
    padding-left: 32px !important;
    border-radius: 8px !important;
    border: 1px solid #d5e0ec !important;
    background: #fff !important;
    font-size: 12px !important;
    height: 32px;
    transition: all 0.2s ease;
  }

  .tpl-search:focus {
    border-color: #7ab4e0 !important;
    box-shadow: 0 0 0 3px rgba(74, 144, 196, 0.1) !important;
  }

  /* --- 文件夹列表 --- */
  .tpl-folder-list {
    flex: 1;
    overflow-y: auto;
    padding: 0 8px 8px;
  }

  .tpl-folder-item {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 7px 10px;
    border-radius: 9px;
    cursor: pointer;
    transition: all 0.18s ease;
    position: relative;
    margin-bottom: 2px;
    border: 1px solid transparent;
  }

  .tpl-folder-item:hover {
    background: #eaf1f9;
    border-color: #d0dfee;
  }

  .tpl-folder-item.active {
    background: linear-gradient(135deg, #dbeaf8 0%, #d0e8f5 100%);
    border-color: #a0c8e4;
    box-shadow: 0 1px 4px rgba(42, 100, 160, 0.1);
  }

  .tpl-folder-icon {
    font-size: 20px !important;
    color: #6a9ec4;
    flex-shrink: 0;
    transition: color 0.2s ease;
  }

  .tpl-folder-item.active .tpl-folder-icon {
    color: #2a7ab8;
  }

  .tpl-folder-name {
    flex: 1;
    font-size: 13px;
    color: #2a4a68;
    font-weight: 500;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .tpl-folder-item.active .tpl-folder-name {
    color: #1a3a5a;
    font-weight: 600;
  }

  .tpl-folder-count {
    font-size: 10px;
    color: #8aa4be;
    background: #eaf0f8;
    border-radius: 10px;
    padding: 1px 7px;
    font-weight: 600;
    flex-shrink: 0;
    transition: all 0.2s ease;
  }

  .tpl-folder-item.active .tpl-folder-count {
    background: #c0daf0;
    color: #1a4a70;
  }

  .tpl-folder-actions {
    display: flex;
    gap: 2px;
    opacity: 0;
    transition: opacity 0.18s ease;
    flex-shrink: 0;
  }

  .tpl-folder-item:hover .tpl-folder-actions {
    opacity: 1;
  }

  /* --- 通用小图标按钮 --- */
  .tpl-icon-btn {
    width: 26px;
    height: 26px;
    border-radius: 6px !important;
    border: none !important;
    background: transparent !important;
    color: #7a96b0 !important;
    padding: 0 !important;
    display: inline-flex !important;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .tpl-icon-btn .ms-icon {
    font-size: 16px;
  }

  .tpl-icon-btn:hover {
    background: #e0ecf6 !important;
    color: #3a6a90 !important;
  }

  .tpl-icon-btn.danger:hover {
    background: #fde8ea !important;
    color: #c0333c !important;
  }

  .tpl-icon-btn.sm {
    width: 22px;
    height: 22px;
  }

  .tpl-icon-btn.sm .ms-icon {
    font-size: 14px;
  }

  /* --- 空状态 --- */
  .tpl-empty-hint {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 6px;
    padding: 30px 10px;
    color: #96aec4;
    font-size: 12px;
  }

  .tpl-empty-hint .ms-icon {
    font-size: 36px;
    opacity: 0.5;
  }

  /* --- 右侧主体区域 --- */
  .tpl-main {
    display: flex;
    flex-direction: column;
    overflow: hidden;
    min-height: 0;
  }

  .tpl-main-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 10px 14px;
    border-bottom: 1px solid #e6eef6;
    background: #fff;
    flex-shrink: 0;
    flex-wrap: wrap;
  }

  .tpl-main-title {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .tpl-main-title h3 {
    display: flex;
    align-items: center;
    gap: 6px;
    margin: 0;
    font-size: 14px;
    font-weight: 700;
    color: #1d3a58;
  }

  .tpl-main-title h3 .ms-icon {
    font-size: 20px;
    color: #4a90c4;
  }

  .tpl-template-count {
    font-size: 11px;
    color: #8aa4be;
    background: #eef4fb;
    border-radius: 10px;
    padding: 2px 10px;
    font-weight: 500;
  }

  .tpl-main-actions {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .tpl-main-actions .tpl-search-wrap {
    margin-bottom: 0;
    padding: 0;
    width: 200px;
  }

  .tpl-create-btn {
    border: none !important;
    background: linear-gradient(135deg, #3a8fd4 0%, #2a7ab8 100%) !important;
    color: #fff !important;
    border-radius: 8px !important;
    padding: 6px 14px !important;
    font-size: 12px !important;
    font-weight: 600;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 4px;
    cursor: pointer;
    box-shadow: 0 2px 8px rgba(42, 122, 184, 0.25);
    transition: all 0.2s ease;
  }

  .tpl-create-btn:hover:not(:disabled) {
    box-shadow: 0 4px 14px rgba(42, 122, 184, 0.35);
    transform: translateY(-1px);
  }

  .tpl-create-btn:active:not(:disabled) {
    transform: translateY(0) scale(0.97);
  }

  /* --- 卡片 + 编辑器容器 --- */
  .tpl-content-area {
    flex: 1;
    overflow: hidden;
    display: grid;
    grid-template-columns: 1fr;
    grid-template-rows: 1fr;
    transition: grid-template-columns 0.3s cubic-bezier(0.4, 0, 0.2, 1);
    min-height: 0;
  }

  .tpl-content-area.has-editor {
    grid-template-columns: 1fr 340px;
  }

  /* --- 模板卡片网格 --- */
  .tpl-card-grid {
    padding: 12px 14px;
    overflow-y: auto;
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
    gap: 10px;
    align-content: start;
    min-height: 0;
  }

  .tpl-card {
    border: 1px solid #dde8f2;
    border-radius: 10px;
    background: #fff;
    padding: 10px 12px;
    cursor: pointer;
    transition: all 0.2s ease;
    display: flex;
    flex-direction: column;
    gap: 6px;
    position: relative;
    overflow: hidden;
  }

  .tpl-card::before {
    content: '';
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    height: 3px;
    background: linear-gradient(90deg, #4a90c4, #68b8d7);
    opacity: 0;
    transition: opacity 0.2s ease;
  }

  .tpl-card:hover {
    border-color: #b0cce5;
    box-shadow: 0 4px 16px rgba(42, 100, 160, 0.1);
    transform: translateY(-2px);
  }

  .tpl-card:hover::before {
    opacity: 1;
  }

  .tpl-card.selected {
    border-color: #6aafe0;
    background: linear-gradient(160deg, #f4faff 0%, #eaf4fb 100%);
    box-shadow: 0 2px 12px rgba(42, 120, 180, 0.15);
  }

  .tpl-card.selected::before {
    opacity: 1;
    background: linear-gradient(90deg, #2a7ab8, #3a8fd4);
  }

  .tpl-card-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 6px;
  }

  .tpl-card-name {
    font-size: 13px;
    font-weight: 600;
    color: #1d3a58;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    flex: 1;
  }

  .tpl-card-key {
    display: inline-flex;
    align-items: center;
    gap: 3px;
    font-size: 10px;
    color: #6a8ea8;
    background: #eef5fb;
    border-radius: 4px;
    padding: 1px 6px;
    width: fit-content;
  }

  .tpl-card-key .ms-icon {
    font-size: 12px;
  }

  .tpl-card-preview {
    font-size: 11px;
    color: #7a96b0;
    line-height: 1.5;
    max-height: 48px;
    overflow: hidden;
    display: -webkit-box;
    -webkit-line-clamp: 3;
    -webkit-box-orient: vertical;
    word-break: break-all;
  }

  /* --- 模板空状态 --- */
  .tpl-empty-card {
    grid-column: 1 / -1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 10px;
    padding: 60px 20px;
    color: #96aec4;
  }

  .tpl-empty-card .ms-icon {
    font-size: 48px;
    opacity: 0.4;
  }

  .tpl-empty-card p {
    font-size: 13px;
    margin: 0;
    text-align: center;
  }

  /* --- 编辑器面板 --- */
  .tpl-editor {
    border-left: 1px solid #dde8f2;
    background: linear-gradient(180deg, #fafcff 0%, #f5f9fe 100%);
    display: flex;
    flex-direction: column;
    overflow: hidden;
    min-height: 0;
    animation: editorSlideIn 0.28s cubic-bezier(0.4, 0, 0.2, 1);
  }

  @keyframes editorSlideIn {
    from {
      opacity: 0;
      transform: translateX(20px);
    }
    to {
      opacity: 1;
      transform: translateX(0);
    }
  }

  .tpl-editor-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 10px 12px;
    border-bottom: 1px solid #e6eef6;
    flex-shrink: 0;
  }

  .tpl-editor-header h4 {
    margin: 0;
    font-size: 13px;
    font-weight: 700;
    color: #1d3a58;
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .tpl-editor-header h4 .ms-icon {
    font-size: 18px;
    color: #4a90c4;
  }

  .tpl-editor-body {
    flex: 1;
    overflow-y: auto;
    padding: 12px;
    display: flex;
    flex-direction: column;
    gap: 10px;
    min-height: 0;
  }

  .tpl-field {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .tpl-field-label {
    font-size: 12px;
    color: #4a6a8a;
    font-weight: 600;
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .tpl-field-label .ms-icon {
    font-size: 15px;
    color: #6a9ec4;
  }

  .tpl-field-label small {
    font-weight: 400;
    color: #8aa4be;
  }

  .tpl-input {
    border: 1px solid #d5e0ec !important;
    border-radius: 8px !important;
    padding: 7px 10px !important;
    font-size: 13px !important;
    background: #fff !important;
    transition: all 0.2s ease;
  }

  .tpl-input:focus {
    border-color: #7ab4e0 !important;
    box-shadow: 0 0 0 3px rgba(74, 144, 196, 0.1) !important;
  }

  .tpl-field-grow {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }

  .tpl-textarea {
    flex: 1;
    min-height: 80px;
    border: 1px solid #d5e0ec !important;
    border-radius: 8px !important;
    padding: 9px 10px !important;
    font-size: 13px !important;
    background: #fff !important;
    resize: none;
    line-height: 1.6;
    transition: all 0.2s ease;
  }

  .tpl-textarea:focus {
    border-color: #7ab4e0 !important;
    box-shadow: 0 0 0 3px rgba(74, 144, 196, 0.1) !important;
  }

  .tpl-editor-footer {
    padding: 10px 12px;
    border-top: 1px solid #e6eef6;
    flex-shrink: 0;
  }

  .tpl-save-btn {
    width: 100%;
    border: none !important;
    background: linear-gradient(135deg, #2a9d5a 0%, #1f8a4e 100%) !important;
    color: #fff !important;
    border-radius: 9px !important;
    padding: 9px 16px !important;
    font-size: 13px !important;
    font-weight: 600;
    display: inline-flex;
    align-items: center;
    gap: 4px;
    cursor: pointer;
    box-shadow: 0 2px 10px rgba(31, 138, 78, 0.25);
    transition: all 0.2s ease;
    justify-content: center;
  }

  .tpl-save-btn:hover:not(:disabled) {
    box-shadow: 0 4px 16px rgba(31, 138, 78, 0.35);
    transform: translateY(-1px);
  }

  .tpl-save-btn:active:not(:disabled) {
    transform: translateY(0) scale(0.98);
  }

  /* ===== 响应式 ===== */
  @media (max-width: 1100px) {
    .content {
      grid-template-columns: 1fr;
      height: auto;
    }

    .tabs {
      flex-direction: row;
      overflow-x: auto;
    }

    .general-grid,
    .system-grid {
      grid-template-columns: 1fr;
    }

    .page-head {
      align-items: flex-start;
    }

    .row {
      grid-template-columns: 1fr;
    }

    .tpl-body {
      grid-template-columns: 1fr;
    }

    .tpl-sidebar {
      border-right: none;
      border-bottom: 1px solid #dde8f2;
      max-height: 200px;
    }

    .tpl-content-area.has-editor {
      grid-template-columns: 1fr;
    }

    .shell {
      padding: 8px;
    }
  }
</style>
