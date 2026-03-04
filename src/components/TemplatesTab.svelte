<script lang="ts">
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
  export let onRunSync: (mode: "pull" | "push") => void;
  export let onSelectFolder: (folderId: string) => void;
  export let onFolderSearchChange: (value: string) => void;
  export let onTemplateSearchChange: (value: string) => void;
  export let onCreateTemplate: () => void;
  export let onSelectTemplate: (templateId: string) => void;
  export let onRemoveTemplate: (templateId: string) => void;
  export let onCloseTemplateEditor: () => void;
  export let onUpdateTemplateDraft: (patch: Partial<TemplateItem>) => void;
  export let onSaveTemplateDraft: () => void;

  $: selectedFolderName = activeFolders.find((item) => item.id === selectedFolderId)?.name || "模板";

  const asInputValue = (event: Event): string => {
    return (event.currentTarget as HTMLInputElement).value;
  };

  const asTextareaValue = (event: Event): string => {
    return (event.currentTarget as HTMLTextAreaElement).value;
  };
</script>

<section class="panel tpl-panel">
  <section class="tpl-sync-bar">
    <div class="tpl-sync-left">
      <span class="ms-icon tpl-sync-icon">cloud_sync</span>
      <span class="tpl-sync-ver">本地 <strong>{store.datasetVersion || 0}</strong></span>
      <span class="tpl-sync-sep">|</span>
      <span class="tpl-sync-ver">同步基线 <strong>{settings.lastSyncedVersion || 0}</strong></span>
    </div>
    <div class="tpl-sync-right">
      <button class="tpl-sync-btn" disabled={busy} on:click={() => onRunSync("pull")}>
        <span class="ms-icon" class:syncing={busy && syncMode === "pull"}>{busy && syncMode === "pull" ? "sync" : "cloud_download"}</span>
        {busy && syncMode === "pull" ? "拉取中..." : "拉取合并"}
      </button>
      <button class="tpl-sync-btn" disabled={busy} on:click={() => onRunSync("push")}>
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
        <h3>
          <span class="ms-icon">folder</span> 文件夹
        </h3>
        <button class="tpl-add-btn" title="新建文件夹" disabled={busy} on:click={() => onCreateFolder("新文件夹")}>
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
            class:active={folder.id === selectedFolderId}
            on:click={() => onSelectFolder(folder.id)}
            on:keydown={(event) => event.key === "Enter" && onSelectFolder(folder.id)}
            role="button"
            tabindex="0"
          >
            <span class="ms-icon tpl-folder-icon">{folder.id === selectedFolderId ? "folder_open" : "folder"}</span>
            <span class="tpl-folder-name">{folder.name}</span>
            <span class="tpl-folder-count">{templateCountByFolderId[folder.id] ?? 0}</span>
            <div class="tpl-folder-actions">
              <button class="tpl-icon-btn" title="重命名" on:click|stopPropagation={() => onRenameFolder(folder.id)}>
                <span class="ms-icon">edit</span>
              </button>
              <button class="tpl-icon-btn danger" title="删除" on:click|stopPropagation={() => onRemoveFolder(folder.id)}>
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
          <button class="tpl-create-btn" disabled={busy || !selectedFolderId} on:click={onCreateTemplate}>
            <span class="ms-icon">add_circle</span> 新建模板
          </button>
        </div>
      </div>

      <div class="tpl-content-area" class:has-editor={templateDraft !== null}>
        <div class="tpl-card-grid">
          {#each filteredTemplates as item}
            <div
              class="tpl-card"
              class:selected={item.id === selectedTemplateId}
              on:click={() => onSelectTemplate(item.id)}
              on:keydown={(event) => event.key === "Enter" && onSelectTemplate(item.id)}
              role="button"
              tabindex="0"
            >
              <div class="tpl-card-header">
                <span class="tpl-card-name">{item.name}</span>
                <button class="tpl-icon-btn danger sm" title="删除" on:click|stopPropagation={() => onRemoveTemplate(item.id)}>
                  <span class="ms-icon">close</span>
                </button>
              </div>
              {#if item.key}
                <div class="tpl-card-key">
                  <span class="ms-icon">key</span> {item.key}
                </div>
              {/if}
              <div class="tpl-card-preview">{item.content || "（空内容）"}</div>
            </div>
          {/each}
          {#if filteredTemplates.length === 0}
            <div class="tpl-empty-card">
              <span class="ms-icon">note_add</span>
              <p>{selectedFolderId ? "暂无模板，点击上方按钮创建" : "请先选择一个文件夹"}</p>
            </div>
          {/if}
        </div>

        {#if templateDraft}
          <div class="tpl-editor">
            <div class="tpl-editor-header">
              <h4>
                <span class="ms-icon">edit_note</span> 编辑模板
              </h4>
              <button class="tpl-icon-btn" title="关闭编辑器" on:click={onCloseTemplateEditor}>
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
                  class="tpl-input"
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
                  class="tpl-input"
                />
              </label>

              <label class="tpl-field tpl-field-grow">
                <span class="tpl-field-label">
                  <span class="ms-icon">article</span> 模板内容
                </span>
                <textarea
                  value={templateDraft.content}
                  on:input={(event) => onUpdateTemplateDraft({ content: asTextareaValue(event) })}
                  class="tpl-textarea"
                ></textarea>
              </label>
            </div>
            <div class="tpl-editor-footer">
              <button class="tpl-save-btn" disabled={busy} on:click={onSaveTemplateDraft}>
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
  .panel {
    border-radius: 12px;
    border: 1px solid var(--qc-border-soft);
    background: linear-gradient(160deg, #ffffffe8 0%, #f6fbffe8 100%);
    box-shadow: var(--qc-shadow-soft);
    overflow: auto;
    min-height: 0;
  }

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
    border: 1px solid #b8d4ea;
    background: rgba(255, 255, 255, 0.85);
    color: #2a6090;
    border-radius: 8px;
    padding: 5px 12px;
    font-size: 12px;
    display: inline-flex;
    align-items: center;
    gap: 4px;
    cursor: pointer;
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
    grid-template-columns: 220px minmax(0, 1fr);
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
    align-items: center;
    justify-content: space-between;
    padding: 10px 12px 6px;
    flex-shrink: 0;
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
    margin-bottom: 2px;
    border: 1px solid transparent;
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

  .tpl-main-actions .tpl-search-wrap {
    margin-bottom: 0;
    padding: 0;
    width: 200px;
  }

  .tpl-content-area {
    flex: 1;
    overflow: hidden;
    display: grid;
    grid-template-columns: 1fr;
    min-height: 0;
  }

  .tpl-content-area.has-editor {
    grid-template-columns: 1fr 340px;
  }

  .tpl-card-grid {
    padding: 12px 14px;
    overflow-y: auto;
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
    gap: 10px;
    align-content: start;
  }

  .tpl-card {
    border: 1px solid #dde8f2;
    border-radius: 10px;
    background: #fff;
    padding: 10px 12px;
    cursor: pointer;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .tpl-card.selected {
    border-color: #6aafe0;
    background: linear-gradient(160deg, #f4faff 0%, #eaf4fb 100%);
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
    font-size: 10px;
    color: #6a8ea8;
    background: #eef5fb;
    border-radius: 4px;
    padding: 1px 6px;
    width: fit-content;
  }

  .tpl-card-preview {
    font-size: 11px;
    color: #7a96b0;
    line-height: 1.5;
    max-height: 48px;
    overflow: hidden;
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
  }

  .tpl-field {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .tpl-field-grow {
    flex: 1;
  }

  .tpl-input,
  .tpl-textarea {
    border: 1px solid #d5e0ec;
    border-radius: 8px;
    padding: 7px 10px;
    font-size: 13px;
    background: #fff;
  }

  .tpl-textarea {
    min-height: 80px;
    resize: none;
    line-height: 1.6;
  }

  .tpl-save-btn,
  .tpl-create-btn,
  .tpl-add-btn,
  .tpl-icon-btn {
    cursor: pointer;
  }

  .tpl-empty-card,
  .tpl-empty-hint {
    color: #96aec4;
    text-align: center;
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

    .tpl-content-area.has-editor {
      grid-template-columns: 1fr;
    }
  }
</style>
