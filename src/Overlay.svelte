<script lang="ts">
  import { onMount } from "svelte";
  import { listen } from "@tauri-apps/api/event";
  import { closeOverlay, getOverlayContext, insertTemplate, loadTemplateStore } from "./lib/api";
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
    if (oneLine.length <= 48) {
      return oneLine;
    }
    return `${oneLine.slice(0, 48)}...`;
  };
</script>

<main class="overlay">
  <section class="panel">
    <header class="head">
      <input
        bind:this={searchInput}
        bind:value={query}
        placeholder="搜索文件夹 / 模板名称 / key / 内容"
        on:input={() => (focusPane = "templates")}
      />
      <button class="ghost" disabled={busy} on:click={() => void cancelOverlay()}>Esc</button>
    </header>

    {#if loading}
      <div class="loading">加载模板中...</div>
    {:else}
      <section class="body">
        <section class={`pane ${focusPane === "folders" ? "active" : ""}`}>
          <h2>文件夹</h2>
          <ul>
            {#each filteredFolders as folder}
              <li class:selected={folder.id === selectedFolderId}>
                <button on:click={() => selectFolder(folder.id)}>{folder.name}</button>
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
                  on:click={() => selectTemplate(template.id)}
                  on:dblclick={() => void confirmInsert()}
                >
                  <div class="template-row">
                    <strong>{template.name}</strong>
                    {#if template.key}
                      <span>key: {template.key}</span>
                    {/if}
                  </div>
                  <small>{compact(template.content)}</small>
                </button>
              </li>
            {/each}
          </ul>
        </section>
      </section>
    {/if}

    <footer class="tips">
      <span>{hint}</span>
      <span>上/下选择，左/右切换，Enter 插入，Esc 取消</span>
    </footer>
  </section>
</main>

<style>
  .overlay {
    width: 100vw;
    height: 100vh;
    margin: 0;
    padding: 0;
    background: transparent;
    display: grid;
    place-items: center;
  }

  .panel {
    width: min(720px, calc(100vw - 24px));
    height: min(420px, calc(100vh - 24px));
    border-radius: 12px;
    border: 1px solid #96b9d0;
    background: linear-gradient(160deg, #f1faf8 0%, #f2f7ff 100%);
    box-shadow: 0 14px 44px #214d6f3a;
    backdrop-filter: blur(12px);
    padding: 10px;
    display: grid;
    grid-template-rows: auto minmax(0, 1fr) auto;
    gap: 8px;
  }

  .head {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    gap: 8px;
  }

  input {
    border: 1px solid #a9c3d8;
    border-radius: 8px;
    padding: 8px 10px;
    background: #ffffffd8;
    color: #1a3652;
    outline: none;
  }

  input:focus {
    border-color: #3f7db5;
    box-shadow: 0 0 0 2px #d8e9fb;
  }

  .body {
    min-height: 0;
    display: grid;
    grid-template-columns: 220px minmax(0, 1fr);
    gap: 8px;
  }

  .pane {
    border: 1px solid #c4d8e8;
    border-radius: 10px;
    background: #ffffffcc;
    padding: 8px;
    display: grid;
    grid-template-rows: auto minmax(0, 1fr);
    gap: 6px;
    min-height: 0;
  }

  .pane.active {
    border-color: #63a0c4;
    box-shadow: 0 0 0 2px #d4ecf8;
  }

  h2 {
    margin: 0;
    font-size: 13px;
    color: #466b8f;
    font-weight: 600;
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

  li button {
    width: 100%;
    text-align: left;
    border: 1px solid #d0deea;
    border-radius: 8px;
    background: #fff;
    color: #1d3a58;
    padding: 7px 8px;
    cursor: pointer;
  }

  li.selected button {
    border-color: #6aa5c4;
    background: #eaf6fc;
    color: #0f3a57;
  }

  .template-row {
    display: flex;
    justify-content: space-between;
    gap: 8px;
  }

  strong {
    font-size: 13px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  span {
    font-size: 11px;
    color: #4c6f8f;
    white-space: nowrap;
  }

  small {
    display: block;
    margin-top: 2px;
    color: #4f6f8d;
    font-size: 11px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .tips {
    display: flex;
    justify-content: space-between;
    gap: 8px;
    font-size: 11px;
    color: #537291;
  }

  .loading {
    display: grid;
    place-items: center;
    border: 1px dashed #9cb8cc;
    border-radius: 10px;
    color: #44627f;
    font-size: 13px;
  }

  .ghost {
    border: 1px solid #9fbad2;
    border-radius: 8px;
    background: #eff6ff;
    color: #2a5078;
    padding: 0 10px;
    cursor: pointer;
  }

  @media (max-width: 780px) {
    .panel {
      width: calc(100vw - 14px);
      height: calc(100vh - 14px);
      padding: 8px;
    }

    .body {
      grid-template-columns: 1fr;
    }

    .tips {
      flex-direction: column;
    }
  }
</style>
