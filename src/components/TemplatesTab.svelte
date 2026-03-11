<script lang="ts">
  import { onDestroy } from "svelte";
  import type { DropPosition } from "../lib/templateOrder";
  import type { Folder, Settings, TemplateItem, TemplateStore } from "../lib/types";

  export let busy = false;
  export let syncMode: "pull" | "push" | null = null;
  export let syncStage = "";
  export let store: TemplateStore;
  export let settings: Settings;
  export let activeFolders: Folder[] = [];
  export let filteredFolders: Folder[] = [];
  export let filteredTemplates: TemplateItem[] = [];
  export let selectedFolderId = "";
  export let selectedTemplateId = "";
  export let folderSearch = "";
  export let templateSearch = "";
  export let templateDraft: TemplateItem | null = null;
  export let templateCountByFolderId: Record<string, number> = {};

  export let onCreateFolder: (name: string) => void;
  export let onRenameFolder: (folderId: string) => void;
  export let onRemoveFolder: (folderId: string) => void;
  export let onReorderFolders: (sourceId: string, targetId: string, position: DropPosition) => void;
  export let onRunSync: (mode: "pull" | "push") => void;
  export let onSelectFolder: (folderId: string) => void;
  export let onFolderSearchChange: (value: string) => void;
  export let onTemplateSearchChange: (value: string) => void;
  export let onCreateTemplate: () => void;
  export let onSelectTemplate: (templateId: string) => void;
  export let onRemoveTemplate: (templateId: string) => void;
  export let onReorderTemplates: (sourceId: string, targetId: string, position: DropPosition) => void;
  export let onCloseTemplateEditor: () => void;
  export let onUpdateTemplateDraft: (patch: Partial<TemplateItem>) => void;
  export let onSaveTemplateDraft: () => void;

  $: selectedFolderName = activeFolders.find((item) => item.id === selectedFolderId)?.name || "模板";
  $: folderDragEnabled = !busy && !folderSearch.trim();
  $: templateDragEnabled = !busy && !templateSearch.trim() && !!selectedFolderId;

  type DropState = {
    id: string;
    position: DropPosition;
  } | null;

  type ActiveDrag = {
    kind: "folder" | "template";
    id: string;
  } | null;

  let activeDrag: ActiveDrag = null;
  let folderDropState: DropState = null;
  let templateDropState: DropState = null;

  $: draggingFolderId = activeDrag?.kind === "folder" ? activeDrag.id : "";
  $: draggingTemplateId = activeDrag?.kind === "template" ? activeDrag.id : "";

  const asInputValue = (event: Event): string => {
    return (event.currentTarget as HTMLInputElement).value;
  };

  const asTextareaValue = (event: Event): string => {
    return (event.currentTarget as HTMLTextAreaElement).value;
  };

  const getDropPosition = (element: HTMLElement, clientY: number): DropPosition => {
    const rect = element.getBoundingClientRect();
    return clientY < rect.top + rect.height / 2 ? "before" : "after";
  };

  const getDropTarget = (event: PointerEvent, kind: "folder" | "template"): DropState => {
    const selector = kind === "folder" ? "[data-folder-id]" : "[data-template-id]";
    const element = document.elementFromPoint(event.clientX, event.clientY)?.closest(selector) as HTMLElement | null;
    if (!element) {
      return null;
    }

    const id = kind === "folder" ? element.dataset.folderId : element.dataset.templateId;
    if (!id || activeDrag?.id === id) {
      return null;
    }

    return {
      id,
      position: getDropPosition(element, event.clientY)
    };
  };

  const removeGlobalDragListeners = (): void => {
    window.removeEventListener("pointermove", handleGlobalPointerMove, true);
    window.removeEventListener("pointerup", handleGlobalPointerUp, true);
    window.removeEventListener("pointercancel", cancelActiveDrag, true);
    document.body.classList.remove("qc-sorting-active");
  };

  const resetDragState = (): void => {
    activeDrag = null;
    folderDropState = null;
    templateDropState = null;
    removeGlobalDragListeners();
  };

  const handleGlobalPointerMove = (event: PointerEvent): void => {
    if (!activeDrag) {
      return;
    }

    event.preventDefault();
    if (activeDrag.kind === "folder") {
      folderDropState = getDropTarget(event, "folder");
      return;
    }

    templateDropState = getDropTarget(event, "template");
  };

  const handleGlobalPointerUp = (event: PointerEvent): void => {
    if (!activeDrag) {
      return;
    }

    event.preventDefault();
    const currentDrag = activeDrag;
    const dropState = currentDrag.kind === "folder" ? folderDropState : templateDropState;
    resetDragState();

    if (!dropState || dropState.id === currentDrag.id) {
      return;
    }

    if (currentDrag.kind === "folder") {
      onReorderFolders(currentDrag.id, dropState.id, dropState.position);
      return;
    }

    onReorderTemplates(currentDrag.id, dropState.id, dropState.position);
  };

  const cancelActiveDrag = (): void => {
    if (!activeDrag) {
      return;
    }
    resetDragState();
  };

  const startDrag = (event: PointerEvent, kind: "folder" | "template", id: string): void => {
    event.preventDefault();
    event.stopPropagation();

    resetDragState();
    activeDrag = { kind, id };
    document.body.classList.add("qc-sorting-active");
    window.addEventListener("pointermove", handleGlobalPointerMove, true);
    window.addEventListener("pointerup", handleGlobalPointerUp, true);
    window.addEventListener("pointercancel", cancelActiveDrag, true);
  };

  const startFolderDrag = (event: PointerEvent, folderId: string): void => {
    if (!folderDragEnabled) {
      return;
    }
    startDrag(event, "folder", folderId);
  };

  const startTemplateDrag = (event: PointerEvent, templateId: string): void => {
    if (!templateDragEnabled) {
      return;
    }
    startDrag(event, "template", templateId);
  };

  onDestroy(() => {
    removeGlobalDragListeners();
  });
</script>

<section class="qc-panel tpl-panel">
  <section class="tpl-sync-bar">
    <div class="tpl-sync-left">
      <span class="ms-icon tpl-sync-icon">cloud_sync</span>
      <span class="tpl-sync-ver">本地 <strong>{store.datasetVersion || 0}</strong></span>
      <span class="tpl-sync-sep">|</span>
      <span class="tpl-sync-ver">同步基线 <strong>{settings.lastSyncedVersion || 0}</strong></span>
    </div>
    <div class="tpl-sync-right">
      <button class="qc-btn qc-btn-subtle tpl-sync-btn" disabled={busy} on:click={() => onRunSync("pull")}>
        <span class="ms-icon" class:syncing={busy && syncMode === "pull"}>{busy && syncMode === "pull" ? "sync" : "cloud_download"}</span>
        {busy && syncMode === "pull" ? "拉取中..." : "拉取合并"}
      </button>
      <button class="qc-btn qc-btn-subtle tpl-sync-btn" disabled={busy} on:click={() => onRunSync("push")}>
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

  <div class="tpl-body">
    <aside class="tpl-sidebar">
      <div class="tpl-sidebar-header">
        <div>
          <h3>
            <span class="ms-icon">folder</span> 文件夹
          </h3>
          <p class="tpl-sort-note">{folderDragEnabled ? "可拖动排序" : "清空搜索后可拖动排序"}</p>
        </div>
        <button class="qc-icon-btn tpl-add-btn" title="新建文件夹" disabled={busy} on:click={() => onCreateFolder("新文件夹")}>
          <span class="ms-icon">add</span>
        </button>
      </div>
      <div class="tpl-search-wrap">
        <span class="ms-icon tpl-search-icon">search</span>
        <input
          type="text"
          value={folderSearch}
          on:input={(event) => onFolderSearchChange(asInputValue(event))}
          placeholder="搜索文件夹…"
          class="tpl-search"
        />
      </div>
      <div class="tpl-folder-list">
        {#each filteredFolders as folder}
          <div
            class="tpl-folder-item"
            data-folder-id={folder.id}
            class:active={folder.id === selectedFolderId}
            class:dragging={draggingFolderId === folder.id}
            class:drop-before={folderDropState?.id === folder.id && folderDropState?.position === "before"}
            class:drop-after={folderDropState?.id === folder.id && folderDropState?.position === "after"}
            on:click={() => onSelectFolder(folder.id)}
            on:keydown={(event) => event.key === "Enter" && onSelectFolder(folder.id)}
            role="button"
            tabindex="0"
          >
            <button
              type="button"
              class="qc-icon-btn tpl-drag-handle-btn"
              title={folderDragEnabled ? "拖动排序" : "清空搜索后可拖动排序"}
              on:pointerdown={(event) => startFolderDrag(event, folder.id)}
              on:click|preventDefault|stopPropagation={() => undefined}
            >
              <span class="ms-icon tpl-drag-handle">drag_indicator</span>
            </button>
            <span class="ms-icon tpl-folder-icon">{folder.id === selectedFolderId ? "folder_open" : "folder"}</span>
            <span class="tpl-folder-name">{folder.name}</span>
            <span class="tpl-folder-count">{templateCountByFolderId[folder.id] ?? 0}</span>
            <div class="tpl-folder-actions">
              <button class="qc-icon-btn tpl-icon-btn" title="重命名" draggable={false} on:click|stopPropagation={() => onRenameFolder(folder.id)}>
                <span class="ms-icon">edit</span>
              </button>
              <button class="qc-icon-btn qc-icon-btn-danger tpl-icon-btn danger" title="删除" draggable={false} on:click|stopPropagation={() => onRemoveFolder(folder.id)}>
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

    <div class="tpl-main">
      <div class="tpl-main-header">
        <div class="tpl-main-title">
          <h3>
            <span class="ms-icon">description</span>
            {selectedFolderName}
          </h3>
          <span class="tpl-template-count">{filteredTemplates.length} 个模板</span>
        </div>
        <div class="tpl-main-actions">
          <div class="tpl-search-wrap">
            <span class="ms-icon tpl-search-icon">search</span>
            <input
              type="text"
              value={templateSearch}
              on:input={(event) => onTemplateSearchChange(asInputValue(event))}
              placeholder="搜索模板名、key、内容…"
              class="tpl-search"
            />
          </div>
          <button class="qc-btn tpl-create-btn" disabled={busy || !selectedFolderId} on:click={onCreateTemplate}>
            <span class="ms-icon">add_circle</span> 新建模板
          </button>
        </div>
      </div>

      <div class="tpl-content-area" class:has-editor={templateDraft !== null}>
        <div class="tpl-template-panel">
          <div class="tpl-list-status">
            <span class="ms-icon">reorder</span>
            <span>{templateDragEnabled ? "单列列表，可拖动排序" : "清空搜索后可拖动排序"}</span>
          </div>

          <div class="tpl-template-list">
          {#each filteredTemplates as item}
            <div
              class="tpl-template-row"
              data-template-id={item.id}
              class:selected={item.id === selectedTemplateId}
              class:dragging={draggingTemplateId === item.id}
              class:drop-before={templateDropState?.id === item.id && templateDropState?.position === "before"}
              class:drop-after={templateDropState?.id === item.id && templateDropState?.position === "after"}
              on:click={() => onSelectTemplate(item.id)}
              on:keydown={(event) => event.key === "Enter" && onSelectTemplate(item.id)}
              role="button"
              tabindex="0"
            >
              <div class="tpl-template-row-head">
                <button
                  type="button"
                  class="qc-icon-btn tpl-drag-handle-btn"
                  title={templateDragEnabled ? "拖动排序" : "清空搜索后可拖动排序"}
                  on:pointerdown={(event) => startTemplateDrag(event, item.id)}
                  on:click|preventDefault|stopPropagation={() => undefined}
                >
                  <span class="ms-icon tpl-drag-handle">drag_indicator</span>
                </button>
                <div class="tpl-template-texts">
                  <div class="tpl-template-title-line">
                    <span class="tpl-template-name">{item.name}</span>
                    {#if item.key}
                      <div class="tpl-template-key">
                      <span class="ms-icon">key</span>
                      <span>{item.key}</span>
                      </div>
                    {/if}
                  </div>
                </div>
                <button class="qc-icon-btn qc-icon-btn-danger qc-icon-btn-sm tpl-icon-btn danger sm" title="删除" draggable={false} on:click|stopPropagation={() => onRemoveTemplate(item.id)}>
                  <span class="ms-icon">close</span>
                </button>
              </div>
              <div class="tpl-template-preview">{item.content || "（空内容）"}</div>
            </div>
          {/each}
          {#if filteredTemplates.length === 0}
            <div class="tpl-empty-card tpl-empty-list">
              <span class="ms-icon">note_add</span>
              <p>{selectedFolderId ? "暂无模板，点击上方按钮创建" : "请先选择一个文件夹"}</p>
            </div>
          {/if}
          </div>
        </div>

        {#if templateDraft}
          <div class="tpl-editor">
            <div class="tpl-editor-header">
              <h4>
                <span class="ms-icon">edit_note</span> 编辑模板
              </h4>
              <button class="qc-icon-btn tpl-icon-btn" title="关闭编辑器" on:click={onCloseTemplateEditor}>
                <span class="ms-icon">close</span>
              </button>
            </div>
            <div class="tpl-editor-body">
              <label class="tpl-field">
                <span class="tpl-field-label">
                  <span class="ms-icon">badge</span> 模板名称
                </span>
                <input
                  type="text"
                  value={templateDraft.name}
                  on:input={(event) => onUpdateTemplateDraft({ name: asInputValue(event) })}
                  class="qc-input tpl-input"
                />
              </label>

              <label class="tpl-field">
                <span class="tpl-field-label">
                  <span class="ms-icon">key</span> 触发 Key <small>（可选，全局唯一）</small>
                </span>
                <input
                  type="text"
                  value={templateDraft.key ?? ""}
                  on:input={(event) => onUpdateTemplateDraft({ key: asInputValue(event) })}
                  placeholder="例如 addr"
                  class="qc-input tpl-input"
                />
              </label>

              <label class="tpl-field tpl-field-grow">
                <span class="tpl-field-label">
                  <span class="ms-icon">article</span> 模板内容
                </span>
                <textarea
                  value={templateDraft.content}
                  on:input={(event) => onUpdateTemplateDraft({ content: asTextareaValue(event) })}
                  class="qc-textarea tpl-textarea"
                ></textarea>
              </label>
            </div>
            <div class="tpl-editor-footer">
              <button class="qc-btn tpl-save-btn" disabled={busy} on:click={onSaveTemplateDraft}>
                <span class="ms-icon">save</span> 保存模板
              </button>
            </div>
          </div>
        {/if}
      </div>
    </div>
  </div>
</section>

<style>
  .tpl-panel {
    display: flex;
    flex-direction: column;
    height: 100%;
    padding: 0;
    overflow: hidden;
  }

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

  .tpl-sync-left,
  .tpl-sync-right {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .tpl-sync-left {
    font-size: 12px;
    color: #4a6a8a;
  }

  .tpl-sync-icon {
    font-size: 20px;
    color: #3a8fd4;
  }

  .tpl-sync-sep {
    color: #c0d0e0;
  }

  .tpl-sync-btn {
    padding: 5px 12px;
    font-size: 12px;
    min-height: 30px;
  }

  .tpl-sync-btn .ms-icon.syncing {
    animation: syncSpin 0.9s linear infinite;
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
    animation: syncDotPulse 1.2s ease-in-out infinite;
  }

  .tpl-body {
    display: grid;
    grid-template-columns: 272px minmax(0, 1fr);
    flex: 1;
    min-height: 0;
  }

  .tpl-sidebar {
    display: flex;
    flex-direction: column;
    border-right: 1px solid #dde8f2;
    background: linear-gradient(180deg, #f8fbff 0%, #f2f6fb 100%);
    overflow: hidden;
  }

  .tpl-sidebar-header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    padding: 10px 12px 6px;
    flex-shrink: 0;
  }

  .tpl-sidebar-header h3 {
    margin: 0;
    font-size: 13px;
    color: #1d3a58;
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }

  .tpl-sort-note {
    margin: 4px 0 0;
    font-size: 11px;
    color: #7a95af;
  }

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
    font-size: 16px;
    color: #96aec4;
    pointer-events: none;
  }

  .tpl-search {
    width: 100%;
    padding-left: 32px;
    border-radius: 8px;
    border: 1px solid #d5e0ec;
    background: #fff;
    font-size: 12px;
    height: 32px;
  }

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
    margin-bottom: 4px;
    border: 1px solid transparent;
    background: rgba(255, 255, 255, 0.74);
    position: relative;
    transition: border-color 0.15s ease, box-shadow 0.15s ease, transform 0.15s ease;
  }

  .tpl-folder-item.active {
    background: linear-gradient(135deg, #dbeaf8 0%, #d0e8f5 100%);
    border-color: #a0c8e4;
  }

  .tpl-folder-name {
    flex: 1;
    font-size: 13px;
    color: #2a4a68;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .tpl-folder-count {
    font-size: 10px;
    color: #8aa4be;
    background: #eaf0f8;
    border-radius: 10px;
    padding: 1px 7px;
  }

  .tpl-folder-actions {
    display: flex;
    gap: 2px;
  }

  .tpl-add-btn {
    border-style: dashed;
  }

  .tpl-drag-handle {
    color: #89a2ba;
    cursor: grab;
    flex-shrink: 0;
  }

  .tpl-drag-handle-btn {
    width: 22px;
    height: 22px;
    padding: 0;
    border-radius: 6px;
    touch-action: none;
    flex-shrink: 0;
  }

  .tpl-drag-handle-btn:hover .tpl-drag-handle {
    color: #5f84a5;
  }

  .tpl-folder-item.dragging,
  .tpl-template-row.dragging {
    opacity: 0.58;
  }

  :global(body.qc-sorting-active) {
    user-select: none;
    cursor: grabbing;
  }

  .tpl-folder-item.drop-before {
    box-shadow: inset 0 3px 0 #4a9ad4;
  }

  .tpl-folder-item.drop-after {
    box-shadow: inset 0 -3px 0 #4a9ad4;
  }

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
    flex-wrap: wrap;
  }

  .tpl-main-actions {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .tpl-main-title {
    display: inline-flex;
    align-items: center;
    gap: 8px;
  }

  .tpl-main-title h3 {
    margin: 0;
    font-size: 14px;
    color: #1d3a58;
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }

  .tpl-template-count {
    font-size: 11px;
    color: #6484a3;
    background: #eef5fd;
    border: 1px solid #d5e3f1;
    border-radius: 999px;
    padding: 2px 9px;
  }

  .tpl-main-actions .tpl-search-wrap {
    margin-bottom: 0;
    padding: 0;
    width: 200px;
  }

  .tpl-content-area {
    flex: 1;
    overflow: hidden;
    display: grid;
    grid-template-columns: minmax(0, 1fr);
    min-height: 0;
  }

  .tpl-content-area.has-editor {
    grid-template-columns: minmax(280px, 320px) minmax(0, 1fr);
  }

  .tpl-template-panel {
    padding: 12px 14px;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 10px;
    min-width: 0;
    width: min(100%, 320px);
  }

  .tpl-list-status {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 7px 10px;
    border-radius: 9px;
    background: linear-gradient(135deg, #f5faff 0%, #edf6fd 100%);
    border: 1px solid #d5e3f1;
    color: #5d7f9f;
    font-size: 11px;
  }

  .tpl-template-list {
    display: flex;
    flex-direction: column;
    gap: 8px;
    min-height: 0;
  }

  .tpl-template-row {
    border: 1px solid #dde8f2;
    border-radius: 10px;
    background: linear-gradient(160deg, #ffffff 0%, #f6fbff 100%);
    padding: 8px 10px;
    cursor: pointer;
    display: flex;
    flex-direction: column;
    gap: 5px;
    position: relative;
    transition: border-color 0.15s ease, box-shadow 0.15s ease, transform 0.15s ease;
  }

  .tpl-template-row::before,
  .tpl-template-row::after {
    content: "";
    position: absolute;
    left: 14px;
    right: 14px;
    height: 4px;
    border-radius: 999px;
    background: linear-gradient(90deg, #72b3df 0%, #3e92d0 100%);
    box-shadow: 0 0 0 1px rgba(119, 177, 220, 0.25), 0 6px 16px rgba(62, 146, 208, 0.2);
    opacity: 0;
    transform: scaleX(0.45);
    transition: opacity 0.16s ease, transform 0.16s ease;
    pointer-events: none;
  }

  .tpl-template-row::before {
    top: -3px;
    transform-origin: left center;
  }

  .tpl-template-row::after {
    bottom: -3px;
    transform-origin: right center;
  }

  .tpl-template-row.drop-before::before,
  .tpl-template-row.drop-after::after {
    opacity: 1;
    transform: scaleX(1);
  }

  .tpl-template-row.drop-before,
  .tpl-template-row.drop-after {
    border-color: #9cc8e8;
  }

  .tpl-template-row:hover {
    border-color: #bdd5e8;
    transform: translateY(-1px);
  }

  .tpl-template-row.selected {
    border-color: #6aafe0;
    background: linear-gradient(160deg, #f4faff 0%, #eaf4fb 100%);
    box-shadow: 0 10px 24px rgba(77, 135, 181, 0.12);
  }

  .tpl-template-row.selected.drop-before,
  .tpl-template-row.selected.drop-after {
    box-shadow: 0 10px 24px rgba(77, 135, 181, 0.12), 0 0 0 1px rgba(114, 179, 223, 0.28);
  }

  .tpl-template-row-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 6px;
  }

  .tpl-template-texts {
    min-width: 0;
    flex: 1;
  }

  .tpl-template-title-line {
    display: flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
  }

  .tpl-template-name {
    flex: 1;
    font-size: 13px;
    font-weight: 600;
    color: #1d3a58;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .tpl-template-key {
    font-size: 10px;
    color: #5d809d;
    background: #eef5fb;
    border-radius: 999px;
    padding: 1px 7px;
    width: fit-content;
    display: inline-flex;
    align-items: center;
    gap: 4px;
    flex-shrink: 0;
  }

  .tpl-template-key .ms-icon {
    font-size: 12px;
  }

  .tpl-template-preview {
    font-size: 11px;
    color: #7a96b0;
    line-height: 1.45;
    min-width: 0;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }

  .tpl-editor {
    border-left: 1px solid #dde8f2;
    background: linear-gradient(180deg, #fafcff 0%, #f5f9fe 100%);
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .tpl-editor-header,
  .tpl-editor-footer {
    padding: 10px 12px;
    border-bottom: 1px solid #e6eef6;
  }

  .tpl-editor-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .tpl-editor-header h4 {
    margin: 0;
    font-size: 13px;
    color: #1f4267;
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }

  .tpl-editor-footer {
    border-bottom: none;
    border-top: 1px solid #e6eef6;
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
    min-height: 0;
  }

  .tpl-field-label {
    font-size: 12px;
    color: #4d6f8f;
    display: inline-flex;
    align-items: center;
    gap: 5px;
  }

  .tpl-field-label small {
    color: #7e9ab5;
  }

  .tpl-field-grow {
    flex: 1;
    min-height: 0;
  }

  .tpl-input {
    min-height: 33px;
  }

  .tpl-textarea {
    flex: 1;
    min-height: 0;
    height: 100%;
    resize: none;
    line-height: 1.6;
  }

  .tpl-create-btn {
    min-height: 32px;
    padding-inline: 14px;
  }

  .tpl-save-btn {
    width: 100%;
    min-height: 34px;
  }

  .tpl-icon-btn {
    flex-shrink: 0;
  }

  .tpl-empty-card,
  .tpl-empty-hint {
    color: #96aec4;
    text-align: center;
  }

  .tpl-empty-card {
    border: 1px dashed #cfdfed;
    border-radius: 10px;
    background: #f9fcff;
    display: grid;
    place-items: center;
    gap: 6px;
    padding: 24px 12px;
  }

  .tpl-empty-list {
    min-height: 180px;
  }

  .tpl-empty-card p {
    margin: 0;
    font-size: 12px;
  }

  @keyframes syncSpin {
    from { transform: rotate(0deg); }
    to { transform: rotate(360deg); }
  }

  @keyframes syncDotPulse {
    0%, 100% { transform: scale(0.86); }
    50% { transform: scale(1); }
  }

  @media (max-width: 1100px) {
    .tpl-body {
      grid-template-columns: 1fr;
    }

    .tpl-sidebar {
      border-right: none;
      border-bottom: 1px solid #dde8f2;
      max-height: 200px;
    }

    .tpl-content-area,
    .tpl-content-area.has-editor {
      grid-template-columns: 1fr;
    }

    .tpl-editor {
      border-left: none;
      border-top: 1px solid #dde8f2;
    }
  }
</style>
