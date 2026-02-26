<script lang="ts">
  import { onMount } from "svelte";
  import { listen } from "@tauri-apps/api/event";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { closeOverlay, getOverlayContext, insertTemplate, loadTemplateStore, setOverlayDragging } from "./lib/api";
  import type { OverlayContext, TemplateItem, TemplateStore } from "./lib/types";

  type FocusPane = "folders" | "templates";

  const defaultStore = (): TemplateStore => ({
    datasetVersion: 0,
    folders: [],
    templates: []
  });

  let store = defaultStore();
  let query = "";
  let selectedFolderId = "";
  let selectedTemplateId = "";
  let focusPane: FocusPane = "templates";
  let loading = true;
  let busy = false;
  let hint = "输入关键词搜索模板";
  let searchInput: HTMLInputElement | null = null;
  const currentWindow = getCurrentWindow();

  $: keyword = query.trim().toLowerCase();
  $: activeFolders = store.folders.filter((item) => item.deletedAt === null);
  $: activeTemplates = store.templates.filter((item) => item.deletedAt === null);

  $: filteredFolders = activeFolders.filter((folder) => {
    if (!keyword) {
      return true;
    }

    if (folder.name.toLowerCase().includes(keyword)) {
      return true;
    }

    return activeTemplates.some((template) => {
      if (template.folderId !== folder.id) {
        return false;
      }
      return templateMatch(template, keyword);
    });
  });

  $: if (!selectedFolderId || !filteredFolders.some((item) => item.id === selectedFolderId)) {
    selectedFolderId = filteredFolders[0]?.id || "";
  }

  $: templatesInFolder = activeTemplates.filter((item) => item.folderId === selectedFolderId);

  $: filteredTemplates = templatesInFolder.filter((item) => {
    if (!keyword) {
      return true;
    }
    return templateMatch(item, keyword);
  });

  $: if (!selectedTemplateId || !filteredTemplates.some((item) => item.id === selectedTemplateId)) {
    selectedTemplateId = filteredTemplates[0]?.id || "";
  }

  onMount(() => {
    let unlisten: (() => void) | undefined;
    const keyboardListener = (event: KeyboardEvent): void => {
      handleKeydown(event);
    };

    const run = async (): Promise<void> => {
      await bootstrap();
      unlisten = await listen<OverlayContext>("overlay_context", (event) => {
        void applyContext(event.payload);
      });
    };

    void run();
    window.addEventListener("keydown", keyboardListener, true);

    return () => {
      if (unlisten) {
        unlisten();
      }
      window.removeEventListener("keydown", keyboardListener, true);
    };
  });

  const bootstrap = async (): Promise<void> => {
    loading = true;
    try {
      await reloadStore();
      const context = await getOverlayContext();
      await applyContext(context, false);
    } finally {
      loading = false;
    }
  };

  const reloadStore = async (): Promise<void> => {
    store = await loadTemplateStore();
  };

  const applyContext = async (context: OverlayContext, refresh = true): Promise<void> => {
    if (refresh) {
      await reloadStore();
    }

    query = context.query || "";
    focusPane = "templates";
    hint = "使用回车直接插入模板";

    setTimeout(() => {
      searchInput?.focus();
      searchInput?.select();
    }, 10);
  };

  const templateMatch = (item: TemplateItem, value: string): boolean => {
    if (!value) {
      return true;
    }

    return (
      item.name.toLowerCase().includes(value) ||
      (item.key || "").toLowerCase().includes(value) ||
      item.content.toLowerCase().includes(value)
    );
  };

  const handleKeydown = (event: KeyboardEvent): void => {
    if (busy) {
      return;
    }

    if (event.key === "Escape") {
      event.preventDefault();
      void cancelOverlay();
      return;
    }

    if (event.key === "ArrowLeft") {
      event.preventDefault();
      focusPane = "folders";
      return;
    }

    if (event.key === "ArrowRight") {
      event.preventDefault();
      focusPane = "templates";
      return;
    }

    if (event.key === "ArrowUp") {
      event.preventDefault();
      moveSelection(-1);
      return;
    }

    if (event.key === "ArrowDown") {
      event.preventDefault();
      moveSelection(1);
      return;
    }

    if (event.key === "Enter") {
      event.preventDefault();
      void confirmInsert();
    }
  };

  const moveSelection = (delta: number): void => {
    if (focusPane === "folders") {
      const index = filteredFolders.findIndex((item) => item.id === selectedFolderId);
      if (index < 0) {
        selectedFolderId = filteredFolders[0]?.id || "";
        return;
      }

      const nextIndex = clamp(index + delta, 0, filteredFolders.length - 1);
      selectedFolderId = filteredFolders[nextIndex]?.id || selectedFolderId;
      return;
    }

    const index = filteredTemplates.findIndex((item) => item.id === selectedTemplateId);
    if (index < 0) {
      selectedTemplateId = filteredTemplates[0]?.id || "";
      return;
    }

    const nextIndex = clamp(index + delta, 0, filteredTemplates.length - 1);
    selectedTemplateId = filteredTemplates[nextIndex]?.id || selectedTemplateId;
  };

  const clamp = (value: number, min: number, max: number): number => {
    if (max < min) {
      return min;
    }
    return Math.max(min, Math.min(value, max));
  };

  const selectFolder = (folderId: string): void => {
    selectedFolderId = folderId;
    focusPane = "folders";
  };

  const selectTemplate = (templateId: string): void => {
    selectedTemplateId = templateId;
    focusPane = "templates";
  };

  const confirmInsert = async (): Promise<void> => {
    if (!selectedTemplateId) {
      return;
    }

    busy = true;
    try {
      await insertTemplate(selectedTemplateId);
    } catch (error) {
      hint = error instanceof Error ? error.message : "插入失败";
    } finally {
      busy = false;
    }
  };

  const cancelOverlay = async (): Promise<void> => {
    try {
      await closeOverlay();
    } catch (error) {
      hint = error instanceof Error ? error.message : "关闭浮窗失败";
    }
  };

  const compact = (value: string): string => {
    const oneLine = value.replace(/\s+/g, " ").trim();
    if (!oneLine) {
      return "(空内容)";
    }
    if (oneLine.length <= 72) {
      return oneLine;
    }
    return `${oneLine.slice(0, 72)}...`;
  };

  const startDrag = async (event: PointerEvent): Promise<void> => {
    if (event.button !== 0) {
      return;
    }
    event.preventDefault();
    try {
      await setOverlayDragging(true);
      await currentWindow.startDragging();
    } catch {
      // 忽略拖动失败，不影响主流程
    } finally {
      await setOverlayDragging(false);
    }
  };
</script>

<main class="overlay">
  <section class="panel overlay-panel">
    <div class="drag-bar" data-tauri-drag-region on:pointerdown={startDrag}>
      <span class="drag-title" data-tauri-drag-region>
        <span class="ms-icon" data-tauri-drag-region>drag_indicator</span>
        quickCV 快捷模板
      </span>
      <span data-tauri-drag-region>可拖动浮窗</span>
    </div>

    <header class="head">
      <input
        class="qc-input"
        bind:this={searchInput}
        bind:value={query}
        placeholder="搜索文件夹 / 模板名称 / key / 内容"
        on:input={() => (focusPane = "templates")}
      />
      <button class="qc-btn qc-btn-subtle ghost" disabled={busy} on:click={() => void cancelOverlay()}>
        <span class="ms-icon">close</span>Esc
      </button>
    </header>

    <div class="meta">
      <span class="qc-chip"><span class="ms-icon">folder</span>文件夹 {filteredFolders.length}</span>
      <span class="qc-chip"><span class="ms-icon">description</span>模板 {filteredTemplates.length}</span>
      <span class="meta-text">{hint}</span>
      <span class="meta-text">Enter 插入 · Esc 取消</span>
    </div>

    {#if loading}
      <div class="loading">加载模板中...</div>
    {:else}
      <section class="body">
        <section class={`pane ${focusPane === "folders" ? "active" : ""}`}>
          <h2>文件夹</h2>
          <ul>
            {#each filteredFolders as folder}
              <li class:selected={folder.id === selectedFolderId}>
                <button class="pane-btn" on:click={() => selectFolder(folder.id)}>{folder.name}</button>
              </li>
            {/each}
          </ul>
        </section>

        <section class={`pane ${focusPane === "templates" ? "active" : ""}`}>
          <h2>模板</h2>
          <ul>
            {#each filteredTemplates as template}
              <li class:selected={template.id === selectedTemplateId}>
                <button
                  class="pane-btn"
                  on:click={() => selectTemplate(template.id)}
                  on:dblclick={() => void confirmInsert()}
                >
                  <div class="template-row">
                    <strong>{template.name}</strong>
                    {#if template.key}
                      <span class="key">{template.key}</span>
                    {/if}
                    <small>{compact(template.content)}</small>
                  </div>
                </button>
              </li>
            {/each}
          </ul>
        </section>
      </section>
    {/if}
  </section>
</main>

<style>
  .overlay {
    width: 100vw;
    height: 100vh;
    margin: 0;
    padding: 6px;
    background: transparent;
    display: grid;
    place-items: center;
  }

  .overlay-panel {
    width: 100%;
    height: 100%;
    border-radius: 12px;
    border: 1px solid var(--qc-border-strong);
    background: linear-gradient(160deg, #f1f8ffed 0%, #eef8f5eb 100%);
    box-shadow: var(--qc-shadow-strong);
    backdrop-filter: blur(14px);
    padding: 8px;
    display: grid;
    grid-template-rows: auto auto auto minmax(0, 1fr);
    gap: 6px;
  }

  .drag-bar {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 8px;
    border: 1px dashed #9ec2da;
    border-radius: 8px;
    background: linear-gradient(145deg, #ebf5ffcf 0%, #e6f2f8cf 100%);
    color: #3f6286;
    font-size: 10.5px;
    padding: 4px 8px;
    cursor: move;
    user-select: none;
  }

  .drag-title {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    font-weight: 600;
    color: #2d557c;
  }

  .drag-title .ms-icon {
    font-size: 14px;
  }

  .head {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    gap: 6px;
  }

  .head .qc-input {
    min-height: 34px;
  }

  .ghost {
    min-width: 74px;
    padding-inline: 9px;
  }

  .meta {
    display: flex;
    gap: 6px;
    flex-wrap: wrap;
    padding: 1px 2px;
    line-height: 1.2;
  }

  .meta .qc-chip {
    padding: 3px 9px;
    background: #eef5ff;
  }

  .meta .qc-chip .ms-icon {
    font-size: 13px;
  }

  .meta-text {
    font-size: 11px;
    color: #567594;
    display: inline-flex;
    align-items: center;
  }

  .body {
    min-height: 0;
    display: grid;
    grid-template-columns: 190px minmax(0, 1fr);
    gap: 6px;
  }

  .pane {
    border: 1px solid #cbdeed;
    border-radius: 10px;
    background: linear-gradient(160deg, #ffffffeb 0%, #f6fbffde 100%);
    box-shadow: 0 4px 16px rgba(56, 109, 156, 0.08);
    padding: 6px;
    display: grid;
    grid-template-rows: auto minmax(0, 1fr);
    gap: 5px;
    min-height: 0;
  }

  .pane.active {
    border-color: #6eaad0;
    box-shadow: 0 0 0 2px #d8ecf8, 0 7px 18px rgba(68, 128, 179, 0.16);
  }

  h2 {
    margin: 0;
    font-size: 12px;
    color: #3f6387;
    font-weight: 600;
    display: inline-flex;
    align-items: center;
    gap: 4px;
  }

  ul {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 3px;
    overflow: auto;
  }

  .pane-btn {
    width: 100%;
    text-align: left;
    border: 1px solid #d2e2ee;
    border-radius: 7px;
    background: #fff;
    color: #1d3a58;
    padding: 5px 7px;
    cursor: pointer;
    transition: all 0.16s ease;
  }

  .pane-btn:hover {
    border-color: #b2cde3;
    background: #f6fbff;
  }

  li.selected .pane-btn {
    border-color: #6aa5c4;
    background: linear-gradient(150deg, #eaf6fc 0%, #e9f2ff 100%);
    color: #0f3a57;
    box-shadow: 0 1px 7px rgba(72, 132, 184, 0.15);
  }

  .template-row {
    display: grid;
    grid-template-columns: auto auto minmax(0, 1fr);
    align-items: center;
    gap: 5px;
  }

  strong {
    font-size: 12px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    min-width: 0;
  }

  .key {
    font-size: 10px;
    border: 1px solid #b8cde2;
    border-radius: 10px;
    padding: 1px 5px;
    background: #f1f8ff;
    color: #4c6f8f;
    white-space: nowrap;
  }

  small {
    display: inline-block;
    margin-top: 0;
    color: #4f6f8d;
    font-size: 10.5px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    min-width: 0;
  }

  .loading {
    display: grid;
    place-items: center;
    border: 1px dashed #a5c1d8;
    border-radius: 10px;
    background: #f7fbffb0;
    color: #44627f;
    font-size: 12px;
  }

  @media (max-width: 620px) {
    .overlay {
      padding: 4px;
    }

    .overlay-panel {
      padding: 5px;
    }

    .body {
      grid-template-columns: 1fr;
    }
  }
</style>
