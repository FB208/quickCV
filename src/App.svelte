<script lang="ts">
  import { onMount } from "svelte";
  import { listen } from "@tauri-apps/api/event";
  import {
    acknowledgeCurrentAppVersion,
    checkAppUpdate,
    getAppVersion,
    installAppUpdate,
    loadSettings,
    loadTemplateStore,
    openReleasePage,
    openConfigFolder,
    openOverlay,
    peekAppUpdateWelcome,
    saveSettings,
    saveTemplateStore,
    syncPull,
    syncPush,
    testWebDav
  } from "./lib/api";
  import { asErrorMessage } from "./lib/errors";
  import { formatHotkey } from "./lib/hotkey";
  import {
    SORT_GAP,
    moveIds,
    nextSortOrder,
    sortFolders,
    sortTemplates,
    type DropPosition
  } from "./lib/templateOrder";
  import GeneralTab from "./components/GeneralTab.svelte";
  import SystemTab from "./components/SystemTab.svelte";
  import TemplatesTab from "./components/TemplatesTab.svelte";
  import type {
    AppUpdateCheckResult,
    AppUpdateProgressEvent,
    AppUpdateWelcome,
    Folder,
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
    lastUpdateCheckAt: 0,
    lastSeenAppVersion: "",
    deviceId: ""
  });

  const defaultStore = (): TemplateStore => ({
    datasetVersion: 0,
    folders: [],
    templates: []
  });

  let tab: TabKey = "general";
  let settings = defaultSettings();
  let savedSettingsSnapshot = defaultSettings();
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
  let syncMode: "pull" | "push" | null = null;
  let syncStage = "";

  let updateInfo: AppUpdateCheckResult | null = null;
  let updateBanner: AppUpdateCheckResult | null = null;
  let updateProgress: AppUpdateProgressEvent | null = null;
  let updateState: "idle" | "checking" | "available" | "latest" | "downloading" | "installing" | "error" = "idle";
  let updatePendingReady = false;
  let updateNoticeHandled = false;
  let recentlyUpdatedFromVersion = "";
  const isDevBuild = import.meta.env.DEV;

  $: activeFolders = sortFolders(store.folders.filter((item) => item.deletedAt === null));
  $: if (!selectedFolderId || !activeFolders.some((item) => item.id === selectedFolderId)) {
    selectedFolderId = activeFolders[0]?.id || "";
  }

  $: filteredFolders = activeFolders.filter((item) => {
    if (!folderSearch.trim()) {
      return true;
    }
    return item.name.toLowerCase().includes(folderSearch.trim().toLowerCase());
  });

  $: activeTemplates = sortTemplates(
    store.templates.filter((item) => item.deletedAt === null && item.folderId === selectedFolderId)
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

  $: selectedTemplate = store.templates.find((item) => item.id === selectedTemplateId && item.deletedAt === null) || null;

  $: {
    if (selectedTemplate && templateDraft?.id !== selectedTemplate.id) {
      templateDraft = { ...selectedTemplate };
    }
    if (!selectedTemplate) {
      templateDraft = null;
    }
  }

  $: hasUnsavedSettings = !areSettingsEqual(settings, savedSettingsSnapshot);
  $: hasUnsavedTemplateDraft = isTemplateDraftDirty(templateDraft, selectedTemplate);
  $: updateBlockedReason = getUpdateBlockedReason();

  const areSettingsEqual = (left: Settings, right: Settings): boolean => {
    return JSON.stringify(left) === JSON.stringify(right);
  };

  const isTemplateDraftDirty = (
    draft: TemplateItem | null,
    source: TemplateItem | null,
  ): boolean => {
    if (!draft || !source) {
      return false;
    }

    return (
      draft.name !== source.name ||
      (draft.key || "") !== (source.key || "") ||
      draft.content !== source.content
    );
  };

  const getUpdateBlockedReason = (): string => {
    if (syncMode) {
      return "正在执行 WebDAV 同步，请稍后再更新";
    }
    if (busy && (updateState === "downloading" || updateState === "installing")) {
      return "更新处理中，请勿重复操作";
    }
    if (hasUnsavedSettings) {
      return "请先保存常规设置后再更新";
    }
    if (hasUnsavedTemplateDraft) {
      return "请先保存当前模板后再更新";
    }
    return "";
  };

  const syncRuntimeSettings = (nextSettings: Settings): void => {
    settings = nextSettings;
    savedSettingsSnapshot = { ...nextSettings };
  };

  const refreshRuntimeUpdateMetadata = (patch: Partial<Settings>): void => {
    settings = { ...settings, ...patch };
    savedSettingsSnapshot = { ...savedSettingsSnapshot, ...patch };
  };

  onMount(() => {
    let unlistenTabNavigate: (() => void) | undefined;
    let unlistenUpdateProgress: (() => void) | undefined;
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

      unlistenUpdateProgress = await listen<AppUpdateProgressEvent>("app_update_progress", (event) => {
        updateProgress = event.payload;
        if (event.payload.phase === "downloading") {
          updateState = "downloading";
          return;
        }
        if (event.payload.phase === "installing") {
          updateState = "installing";
          return;
        }
        if (event.payload.phase === "finished") {
          updateState = "latest";
          busy = false;
          return;
        }
        if (event.payload.phase === "error") {
          updateState = "error";
          busy = false;
          updatePendingReady = false;
          setNotice("error", event.payload.message);
        }
      });
    };

    const handleVisibilityChange = (): void => {
      if (document.visibilityState === "visible") {
        void maybeShowUpdateWelcome();
      }
    };

    window.addEventListener("keydown", keyRecorder, true);
    document.addEventListener("visibilitychange", handleVisibilityChange);
    void bindTabNavigate();
    void bootstrap();
    return () => {
      if (unlistenTabNavigate) {
        unlistenTabNavigate();
      }
      if (unlistenUpdateProgress) {
        unlistenUpdateProgress();
      }
      window.removeEventListener("keydown", keyRecorder, true);
      document.removeEventListener("visibilitychange", handleVisibilityChange);
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
      syncRuntimeSettings(loadedSettings);
      store = loadedStore;
      appVersion = version;
      updateInfo = {
        status: "latest",
        hasUpdate: false,
        message: `当前版本 v${version}`,
        currentVersion: version,
        latestVersion: version,
        releaseUrl: "https://github.com/FB208/quickCV/releases",
        releaseNotes: "",
        publishedAt: null,
        lastCheckAt: loadedSettings.lastUpdateCheckAt || 0
      };
      if (!isDevBuild) {
        void runAppUpdateCheck(true);
      }
      void maybeShowUpdateWelcome();
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

  const formatBytes = (value: number): string => {
    if (value <= 0) {
      return "0 B";
    }

    const units = ["B", "KB", "MB", "GB"];
    let current = value;
    let index = 0;
    while (current >= 1024 && index < units.length - 1) {
      current /= 1024;
      index += 1;
    }

    return `${current.toFixed(index === 0 ? 0 : 1)} ${units[index]}`;
  };

  const persistSettings = async (): Promise<void> => {
    busy = true;
    try {
      const requestedLaunchAtStartup = settings.launchAtStartup;
      const savedSettings = await saveSettings(settings);
      syncRuntimeSettings(savedSettings);
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
      sortOrder: nextSortOrder(activeFolders),
      sortUpdatedAt: now(),
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
      sortOrder: nextSortOrder(activeTemplates),
      sortUpdatedAt: now(),
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

  const reorderFolders = async (
    sourceId: string,
    targetId: string,
    position: DropPosition,
  ): Promise<void> => {
    const nextIds = moveIds(
      activeFolders.map((item) => item.id),
      sourceId,
      targetId,
      position,
    );

    if (nextIds.every((id, index) => id === activeFolders[index]?.id)) {
      return;
    }

    const timestamp = now();
    const sortOrderById = new Map(nextIds.map((id, index) => [id, (index + 1) * SORT_GAP]));

    store = {
      ...store,
      folders: store.folders.map((item) => {
        const nextOrder = item.deletedAt === null ? sortOrderById.get(item.id) : undefined;
        if (nextOrder === undefined) {
          return item;
        }
        return {
          ...item,
          sortOrder: nextOrder,
          sortUpdatedAt: timestamp,
          deviceId: settings.deviceId || item.deviceId
        };
      })
    };

    await persistStore("文件夹顺序已更新");
  };

  const reorderTemplates = async (
    sourceId: string,
    targetId: string,
    position: DropPosition,
  ): Promise<void> => {
    if (!selectedFolderId) {
      return;
    }

    const nextIds = moveIds(
      activeTemplates.map((item) => item.id),
      sourceId,
      targetId,
      position,
    );

    if (nextIds.every((id, index) => id === activeTemplates[index]?.id)) {
      return;
    }

    const timestamp = now();
    const sortOrderById = new Map(nextIds.map((id, index) => [id, (index + 1) * SORT_GAP]));

    store = {
      ...store,
      templates: store.templates.map((item) => {
        if (item.deletedAt !== null || item.folderId !== selectedFolderId) {
          return item;
        }

        const nextOrder = sortOrderById.get(item.id);
        if (nextOrder === undefined) {
          return item;
        }

        return {
          ...item,
          sortOrder: nextOrder,
          sortUpdatedAt: timestamp,
          deviceId: settings.deviceId || item.deviceId
        };
      })
    };

    await persistStore("模板顺序已更新");
  };

  const handleReorderFolders = (
    sourceId: string,
    targetId: string,
    position: DropPosition,
  ): void => {
    void reorderFolders(sourceId, targetId, position);
  };

  const handleReorderTemplates = (
    sourceId: string,
    targetId: string,
    position: DropPosition,
  ): void => {
    void reorderTemplates(sourceId, targetId, position);
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
      syncRuntimeSettings(await loadSettings());
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

  const maybeShowUpdateWelcome = async (): Promise<void> => {
    if (updateNoticeHandled || document.visibilityState !== "visible") {
      return;
    }

    try {
      const welcome: AppUpdateWelcome | null = await peekAppUpdateWelcome();
      await acknowledgeCurrentAppVersion();
      refreshRuntimeUpdateMetadata({ lastSeenAppVersion: appVersion });
      updateNoticeHandled = true;

      if (welcome) {
        recentlyUpdatedFromVersion = welcome.previousVersion;
        setNotice(
          "success",
          `已更新到 v${welcome.currentVersion}（上一版本 v${welcome.previousVersion}）`,
        );
      }
    } catch {
      // 首次启动提示失败不影响主流程
    }
  };

  const runAppUpdateCheck = async (silent: boolean): Promise<void> => {
    if (updateState === "checking") {
      return;
    }

    updateState = "checking";
    updateProgress = null;
    try {
      const result = await checkAppUpdate();
      updateInfo = result;
      refreshRuntimeUpdateMetadata({ lastUpdateCheckAt: result.lastCheckAt });

      if (result.status === "error") {
        updateState = "error";
        updatePendingReady = false;
        if (!silent) {
          setNotice("error", result.message);
        }
        return;
      }

      if (result.hasUpdate) {
        updateState = "available";
        updatePendingReady = true;
        updateBanner = result;
        if (!silent) {
          setNotice("info", result.message);
          tab = "system";
        }
        return;
      }

      updateState = "latest";
      updatePendingReady = false;
      updateBanner = null;
      if (!silent) {
        setNotice("success", result.message);
      }
    } catch (error) {
      updateState = "error";
      updatePendingReady = false;
      if (!silent) {
        setNotice("error", asErrorMessage(error));
      }
    }
  };

  const ensureUpdateCanProceed = (): boolean => {
    if (!updatePendingReady) {
      setNotice("info", "请先检查更新并确认有新版本可安装");
      tab = "system";
      return false;
    }

    if (!updateBlockedReason) {
      return true;
    }

    if (hasUnsavedSettings) {
      tab = "general";
    } else {
      tab = "templates";
    }
    setNotice("error", updateBlockedReason);
    return false;
  };

  const startAppUpdateInstall = async (): Promise<void> => {
    if (!ensureUpdateCanProceed()) {
      return;
    }

    busy = true;
    updateBanner = null;
    updatePendingReady = false;
    updateState = "downloading";
    updateProgress = {
      phase: "downloading",
      version: updateInfo?.latestVersion || appVersion,
      downloadedBytes: 0,
      totalBytes: null,
      message: `正在准备下载 v${updateInfo?.latestVersion || appVersion} 更新包...`
    };

    try {
      await installAppUpdate();
      setNotice("info", "更新包已开始安装，应用可能会自动关闭，请稍候");
    } catch (error) {
      busy = false;
      updateState = "error";
      updateProgress = {
        phase: "error",
        version: updateInfo?.latestVersion || appVersion,
        downloadedBytes: 0,
        totalBytes: null,
        message: asErrorMessage(error, "安装更新失败，请重新检查后重试")
      };
      setNotice("error", updateProgress.message);
    }
  };

  const dismissUpdateBanner = (): void => {
    updateBanner = null;
  };

  const confirmUpdateBanner = async (): Promise<void> => {
    if (!updateBanner) {
      return;
    }
    await startAppUpdateInstall();
  };

  const openReleaseByVersion = async (version?: string | null): Promise<void> => {
    const targetVersion = version || updateInfo?.latestVersion || undefined;
    try {
      await openReleasePage(targetVersion || undefined);
      setNotice("info", targetVersion ? `已打开 v${targetVersion} 的 Release 页面` : "已打开发布页");
    } catch (error) {
      const fallback = targetVersion
        ? `https://github.com/FB208/quickCV/releases/tag/v${targetVersion}`
        : "https://github.com/FB208/quickCV/releases";
      window.open(fallback, "_blank", "noopener,noreferrer");
      setNotice("info", "已通过浏览器打开发布页");
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

  {#if updateState === "downloading" || updateState === "installing"}
    <div class="update-banner progress">
      <span class="ms-icon banner-icon">system_update</span>
      <span class="banner-text">{updateProgress?.message || "正在处理更新，请稍候..."}</span>
      {#if updateProgress?.phase === "downloading"}
        <span class="banner-progress">
          {formatBytes(updateProgress.downloadedBytes)}
          {#if updateProgress.totalBytes}
            / {formatBytes(updateProgress.totalBytes)}
          {/if}
        </span>
      {/if}
    </div>
  {:else if updateBanner}
    <div class="update-banner">
      <span class="ms-icon banner-icon">system_update</span>
      <span class="banner-text">检测到新版本 <strong>{updateBanner.latestVersion || "--"}</strong>（当前 {updateBanner.currentVersion}）</span>
      <button class="banner-btn confirm" on:click={() => void confirmUpdateBanner()}>立即更新</button>
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
          onReorderFolders={handleReorderFolders}
          onRunSync={(mode) => void runSync(mode)}
          onSelectFolder={(folderId) => (selectedFolderId = folderId)}
          onFolderSearchChange={(value) => (folderSearch = value)}
          onTemplateSearchChange={(value) => (templateSearch = value)}
          onCreateTemplate={() => void createTemplate()}
          onSelectTemplate={(templateId) => (selectedTemplateId = templateId)}
          onRemoveTemplate={(templateId) => void removeTemplate(templateId)}
          onReorderTemplates={handleReorderTemplates}
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
          isDevBuild={isDevBuild}
          updateState={updateState}
          updateInfo={updateInfo}
          updateProgress={updateProgress}
          lastUpdateCheckAt={settings.lastUpdateCheckAt}
          recentlyUpdatedFromVersion={recentlyUpdatedFromVersion}
          updatePendingReady={updatePendingReady}
          updateBlockedReason={updateBlockedReason}
          onRunAppUpdateCheck={() => void runAppUpdateCheck(false)}
          onInstallAppUpdate={() => void startAppUpdateInstall()}
          onOpenReleasePage={(version) => void openReleaseByVersion(version)}
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

  .update-banner.progress {
    border-color: #8bb9e2;
    background: linear-gradient(135deg, #e7f4ff 0%, #dff1ff 100%);
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

  .banner-progress {
    font-size: 12px;
    color: #3d6487;
    white-space: nowrap;
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
