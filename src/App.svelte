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
  import GeneralTab from "./components/GeneralTab.svelte";
  import SystemTab from "./components/SystemTab.svelte";
  import TemplatesTab from "./components/TemplatesTab.svelte";
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
    launchAtStartupEffective: false,
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

  $: templateCountByFolderId = store.templates.reduce<Record<string, number>>((acc, item) => {
    if (item.deletedAt === null) {
      acc[item.folderId] = (acc[item.folderId] ?? 0) + 1;
    }
    return acc;
  }, {});

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
      if (
        requestedLaunchAtStartup !== settings.launchAtStartup ||
        settings.launchAtStartup !== settings.launchAtStartupEffective
      ) {
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

  const createFolder = async (presetName?: string): Promise<void> => {
    const baseName = presetName ?? newFolderName;
    const name = baseName.trim();
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

<main class="qc-shell">
  <header class="qc-topbar">
    <div class="qc-brand">
      <h1>quickCV</h1>
      <p>轻量级模板插入管理工具</p>
    </div>
    <div class="qc-version-pill">版本 {appVersion}</div>
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
    <section class="qc-content">
      <nav class="qc-tabs" aria-label="设置分组">
        <button class="qc-tab-btn" class:active={tab === "general"} on:click={() => (tab = "general")}>
          <span class="ms-icon">tune</span>常规设置
        </button>
        <button class="qc-tab-btn" class:active={tab === "templates"} on:click={() => (tab = "templates")}>
          <span class="ms-icon">dashboard</span>模板管理
        </button>
        <button class="qc-tab-btn" class:active={tab === "system"} on:click={() => (tab = "system")}>
          <span class="ms-icon">computer</span>系统
        </button>
      </nav>

      {#if tab === "general"}
        <GeneralTab
          {busy}
          {recordingShortcut}
          {settings}
          onStartRecording={() => (recordingShortcut = true)}
          onToggleLaunchAtStartup={(value) => {
            settings = {
              ...settings,
              launchAtStartup: value
            };
          }}
          onSaveSettings={() => void persistSettings()}
          onPreviewOverlay={() => void previewOverlay()}
          onSetWebDavField={(field, value) => {
            settings = {
              ...settings,
              webdav: {
                ...settings.webdav,
                [field]: value
              }
            };
          }}
          onRunWebDavTest={() => void runWebDavTest()}
        />
      {/if}

      {#if tab === "templates"}
        <TemplatesTab
          busy={busy}
          syncMode={syncMode}
          syncStage={syncStage}
          store={store}
          settings={settings}
          activeFolders={activeFolders}
          filteredFolders={filteredFolders}
          filteredTemplates={filteredTemplates}
          selectedFolderId={selectedFolderId}
          selectedTemplateId={selectedTemplateId}
          folderSearch={folderSearch}
          templateSearch={templateSearch}
          templateDraft={templateDraft}
          templateCountByFolderId={templateCountByFolderId}
          onCreateFolder={(name) => void createFolder(name)}
          onRenameFolder={(folderId) => void renameFolder(folderId)}
          onRemoveFolder={(folderId) => void removeFolder(folderId)}
          onRunSync={(mode) => void runSync(mode)}
          onSelectFolder={(folderId) => (selectedFolderId = folderId)}
          onFolderSearchChange={(value) => (folderSearch = value)}
          onTemplateSearchChange={(value) => (templateSearch = value)}
          onCreateTemplate={() => void createTemplate()}
          onSelectTemplate={(templateId) => (selectedTemplateId = templateId)}
          onRemoveTemplate={(templateId) => void removeTemplate(templateId)}
          onCloseTemplateEditor={() => {
            selectedTemplateId = "";
            templateDraft = null;
          }}
          onUpdateTemplateDraft={(patch) => {
            if (!templateDraft) {
              return;
            }
            templateDraft = {
              ...templateDraft,
              ...patch
            };
          }}
          onSaveTemplateDraft={() => void saveTemplateDraft()}
        />
      {/if}

      {#if tab === "system"}
        <SystemTab
          {appVersion}
          {latestVersion}
          {checkingVersion}
          onRunReleaseCheck={() => void runReleaseCheck(false)}
          onOpenReleasePage={() => void openReleaseByResult()}
          onOpenConfigFolder={() => void openConfigFolderHandler()}
        />
      {/if}
    </section>
  {/if}
</main>

<style>
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

</style>
