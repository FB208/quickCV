<script lang="ts">
  import type { Settings } from "../lib/types";

  export let busy = false;
  export let recordingShortcut = false;
  export let settings: Settings;

  export let onStartRecording: () => void;
  export let onToggleLaunchAtStartup: (value: boolean) => void;
  export let onSaveSettings: () => void;
  export let onPreviewOverlay: () => void;
  export let onSetWebDavField: (field: "url" | "username" | "password" | "remoteFile", value: string) => void;
  export let onRunWebDavTest: () => void;

  const readInput = (event: Event): string => {
    return (event.currentTarget as HTMLInputElement).value;
  };

  const readChecked = (event: Event): boolean => {
    return (event.currentTarget as HTMLInputElement).checked;
  };
</script>

<section class="qc-panel qc-panel-page">
  <div class="qc-page-head">
    <h2 class="qc-page-title"><span class="ms-icon">tune</span>常规设置</h2>
    <p class="qc-page-desc">统一管理快捷键、启动行为与 WebDAV 同步参数。</p>
  </div>

  <div class="general-grid">
    <section class="qc-card general-card">
      <h3 class="qc-card-title"><span class="ms-icon">keyboard</span>快捷入口</h3>
      <label class="qc-field">
        <span>全局快捷键</span>
        <div class="row">
          <input class="qc-input" type="text" value={recordingShortcut ? "请按下快捷键..." : settings.shortcut} readonly />
          <button class="qc-btn" disabled={busy} on:click={onStartRecording}>
            <span class="ms-icon">keyboard</span>
            {recordingShortcut ? "录制中" : "录制快捷键"}
          </button>
        </div>
      </label>

      <div class="startup-card">
        <div class="startup-text">
          <strong>开机自动启动</strong>
          <small>启动后自动在系统托盘待命</small>
          {#if settings.launchAtStartup !== settings.launchAtStartupEffective}
            <small class="startup-warning">系统当前状态未生效，请保存后重试</small>
          {/if}
        </div>
        <label class="switch" title="开机自动启动">
          <input
            type="checkbox"
            checked={settings.launchAtStartup}
            on:change={(event) => onToggleLaunchAtStartup(readChecked(event))}
          />
          <span class="slider"></span>
        </label>
      </div>

      <div class="qc-actions">
        <button class="qc-btn" disabled={busy} on:click={onSaveSettings}>
          <span class="ms-icon">save</span>
          保存设置
        </button>
        <button class="qc-btn qc-btn-subtle" disabled={busy} on:click={onPreviewOverlay}>
          <span class="ms-icon">visibility</span>
          预览快捷浮窗
        </button>
      </div>
    </section>

    <section class="qc-card general-card">
      <h3 class="qc-card-title"><span class="ms-icon">cloud_sync</span>WebDAV 配置</h3>
      <label class="qc-field">
        <span>地址</span>
        <input
          class="qc-input"
          type="text"
          value={settings.webdav.url}
          on:input={(event) => onSetWebDavField("url", readInput(event))}
          placeholder="https://dav.example.com/path"
        />
      </label>

      <label class="qc-field">
        <span>用户名</span>
        <input
          class="qc-input"
          type="text"
          value={settings.webdav.username}
          on:input={(event) => onSetWebDavField("username", readInput(event))}
          autocomplete="off"
        />
      </label>

      <label class="qc-field">
        <span>密码</span>
        <input
          class="qc-input"
          type="password"
          value={settings.webdav.password}
          on:input={(event) => onSetWebDavField("password", readInput(event))}
          autocomplete="off"
        />
      </label>

      <label class="qc-field">
        <span>远端文件名</span>
        <input
          class="qc-input"
          type="text"
          value={settings.webdav.remoteFile}
          on:input={(event) => onSetWebDavField("remoteFile", readInput(event))}
          placeholder="quickcv-data.json"
        />
      </label>

      <div class="qc-actions">
        <button class="qc-btn qc-btn-subtle" disabled={busy} on:click={onRunWebDavTest}>
          <span class="ms-icon">network_check</span>
          测试 WebDAV 连通性
        </button>
      </div>
    </section>
  </div>
</section>

<style>
  .general-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 12px;
    align-items: start;
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

  .startup-text .startup-warning {
    color: #b24f2e;
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

  @media (max-width: 1100px) {
    .general-grid {
      grid-template-columns: 1fr;
    }

    .row {
      grid-template-columns: 1fr;
    }
  }
</style>
