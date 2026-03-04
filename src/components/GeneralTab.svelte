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

    <section class="general-card">
      <h3 class="card-title"><span class="ms-icon">cloud_sync</span>WebDAV 配置</h3>
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

  .general-card {
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

  @media (max-width: 1100px) {
    .general-grid {
      grid-template-columns: 1fr;
    }

    .page-head {
      align-items: flex-start;
    }

    .row {
      grid-template-columns: 1fr;
    }
  }
</style>
