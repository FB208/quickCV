<script lang="ts">
  import type { AppUpdateCheckResult, AppUpdateProgressEvent } from "../lib/types";

  export let appVersion = "--";
  export let isDevBuild = false;
  export let updateState: "idle" | "checking" | "available" | "latest" | "downloading" | "installing" | "error" = "idle";
  export let updateInfo: AppUpdateCheckResult | null = null;
  export let updateProgress: AppUpdateProgressEvent | null = null;
  export let lastUpdateCheckAt = 0;
  export let recentlyUpdatedFromVersion = "";
  export let updatePendingReady = false;
  export let updateBlockedReason = "";

  export let onRunAppUpdateCheck: () => void;
  export let onInstallAppUpdate: () => void;
  export let onOpenReleasePage: (version?: string) => void;
  export let onOpenConfigFolder: () => void;

  const formatTimestamp = (value: number | string | null): string => {
    if (!value) {
      return "尚未检查";
    }

    const date = typeof value === "number" ? new Date(value) : new Date(value);
    if (Number.isNaN(date.getTime())) {
      return "时间未知";
    }

    return date.toLocaleString("zh-CN", {
      hour12: false,
      year: "numeric",
      month: "2-digit",
      day: "2-digit",
      hour: "2-digit",
      minute: "2-digit"
    });
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

  const resolveStatusText = (): string => {
    if (updateState === "checking") {
      return "正在检查最新版本...";
    }
    if (updateState === "downloading") {
      return updateProgress?.message || "正在下载更新包...";
    }
    if (updateState === "installing") {
      return updateProgress?.message || "更新包已下载完成，正在安装...";
    }
    if (updateInfo) {
      return updateInfo.message;
    }
    if (isDevBuild) {
      return "当前版本启动时不会自动检查更新。";
    }
    return "你可以手动检查更新，应用也会在启动后自动检查。";
  };

  const resolveStatusTone = (): "neutral" | "success" | "warn" | "error" => {
    if (updateState === "error") {
      return "error";
    }
    if (updateState === "available" || updateState === "downloading" || updateState === "installing") {
      return "warn";
    }
    if (updateState === "latest") {
      return "success";
    }
    return "neutral";
  };

  $: latestVersionText = updateInfo?.latestVersion ? `v${updateInfo.latestVersion}` : "--";
  $: publishedAtText = formatTimestamp(updateInfo?.publishedAt || null);
  $: lastCheckAtText = formatTimestamp(lastUpdateCheckAt || 0);
  $: releaseNotes = updateInfo?.releaseNotes?.trim() || "暂无更新说明。";
  $: progressPercent =
    updateProgress?.totalBytes && updateProgress.totalBytes > 0
      ? Math.min(100, Math.round((updateProgress.downloadedBytes / updateProgress.totalBytes) * 100))
      : null;
  $: installButtonDisabled =
    updateState === "checking" ||
    updateState === "downloading" ||
    updateState === "installing" ||
    !updatePendingReady ||
    !!updateBlockedReason;
  $: installButtonText =
    updateState === "downloading"
      ? "下载中..."
      : updateState === "installing"
        ? "安装中..."
        : "立即更新";
  $: statusTone = resolveStatusTone();
  $: statusText = resolveStatusText();
</script>

<section class="qc-panel qc-panel-page">
  <div class="qc-page-head">
    <h2 class="qc-page-title"><span class="ms-icon">computer</span>系统</h2>
    <p class="qc-page-desc">查看当前版本、检查更新，并在需要时前往下载页获取新版。</p>
  </div>

  <div class="system-grid">
    <div class="version-card">
      <span class="version-label">当前软件版本</span>
      <strong>v{appVersion}</strong>
      <small>已安装到本机的版本</small>
    </div>

    <div class="version-card">
      <span class="version-label">最新线上版本</span>
      <strong>{latestVersionText}</strong>
      <small>来自官方稳定版本</small>
    </div>

    {#if recentlyUpdatedFromVersion}
      <section class="welcome-card">
        <span class="ms-icon">celebration</span>
        <div>
          <strong>已完成版本更新</strong>
          <p>你刚刚从 v{recentlyUpdatedFromVersion} 升级到 v{appVersion}。</p>
        </div>
      </section>
    {/if}

    <section class="qc-card system-card">
      <div class="system-card-head">
        <div>
          <h3 class="qc-card-title"><span class="ms-icon">system_update</span>软件更新</h3>
          <p class={`status-pill ${statusTone}`}>{statusText}</p>
        </div>
        <div class="meta-stack">
          <span>上次检查：{lastCheckAtText}</span>
          {#if isDevBuild}
            <span>当前版本启动时不会自动检查更新</span>
          {/if}
        </div>
      </div>

      {#if updateBlockedReason}
        <div class="warning-box">
          <span class="ms-icon">warning</span>
          <span>{updateBlockedReason}</span>
        </div>
      {/if}

      <div class="qc-actions system-actions">
        <button class="qc-btn qc-btn-subtle" disabled={updateState === "checking" || updateState === "downloading" || updateState === "installing"} on:click={onRunAppUpdateCheck}>
          <span class="ms-icon">system_update</span>
          {updateState === "checking" ? "检查中..." : "检查更新"}
        </button>
        <button class="qc-btn" disabled={installButtonDisabled} on:click={onInstallAppUpdate}>
          <span class="ms-icon">download</span>
          {installButtonText}
        </button>
        <button class="qc-btn qc-btn-subtle" on:click={() => onOpenReleasePage(updateInfo?.latestVersion || undefined)}>
          <span class="ms-icon">open_in_new</span>
          打开版本下载页
        </button>
        <button class="qc-btn qc-btn-subtle" on:click={onOpenConfigFolder}>
          <span class="ms-icon">folder_open</span>
          查看本地数据位置
        </button>
      </div>

      {#if updateProgress && (updateState === "downloading" || updateState === "installing")}
        <div class="progress-card">
          <div class="progress-head">
            <strong>{updateProgress.message}</strong>
            {#if updateState === "downloading"}
              <span>
                {formatBytes(updateProgress.downloadedBytes)}
                {#if updateProgress.totalBytes}
                  / {formatBytes(updateProgress.totalBytes)}
                {/if}
              </span>
            {/if}
          </div>
          {#if progressPercent !== null}
            <div class="progress-track" aria-hidden="true">
              <span class="progress-fill" style={`width: ${progressPercent}%`}></span>
            </div>
            <small>{progressPercent}%</small>
          {/if}
          {#if updateState === "installing"}
            <small>Windows 安装阶段会自动关闭 quickCV，请勿手动结束进程。</small>
          {/if}
        </div>
      {/if}

      <div class="detail-grid">
        <div class="detail-item">
          <span>当前版本</span>
          <strong>v{updateInfo?.currentVersion || appVersion}</strong>
        </div>
        <div class="detail-item">
          <span>最新版本</span>
          <strong>{latestVersionText}</strong>
        </div>
        <div class="detail-item">
          <span>发布时间</span>
          <strong>{publishedAtText}</strong>
        </div>
        <div class="detail-item">
          <span>更新源</span>
          <strong>官方稳定版本</strong>
        </div>
      </div>

      <div class="notes-card">
        <div class="notes-head">
          <strong>更新说明</strong>
          <small>检测到新版本后，这里会显示对应的更新说明。</small>
        </div>
        <pre>{releaseNotes}</pre>
      </div>

      <p class="hint">如果应用内更新失败，你也可以前往版本下载页手动安装最新版本。</p>
    </section>
  </div>
</section>

<style>
  .system-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 10px;
  }

  .version-card,
  .welcome-card {
    border: 1px solid #c8ddef;
    border-radius: 12px;
    background: linear-gradient(140deg, #f7fbff 0%, #eff7ff 100%);
    padding: 10px 12px;
    display: flex;
    flex-direction: column;
    gap: 3px;
  }

  .welcome-card {
    grid-column: 1 / -1;
    flex-direction: row;
    align-items: flex-start;
    gap: 10px;
    background: linear-gradient(140deg, #eefaf0 0%, #f7fffa 100%);
    border-color: #c6e7cf;
  }

  .welcome-card .ms-icon {
    color: #22815d;
    font-size: 20px;
  }

  .welcome-card strong {
    color: #19553e;
  }

  .welcome-card p {
    margin: 2px 0 0;
    font-size: 12px;
    color: #43715f;
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

  .system-card-head {
    display: flex;
    justify-content: space-between;
    gap: 12px;
    align-items: flex-start;
    margin-bottom: 12px;
  }

  .meta-stack {
    display: flex;
    flex-direction: column;
    gap: 4px;
    font-size: 11px;
    color: #64809d;
    text-align: right;
  }

  .status-pill {
    margin: 6px 0 0;
    padding: 6px 10px;
    border-radius: 999px;
    font-size: 12px;
    line-height: 1.4;
    display: inline-flex;
    align-items: center;
  }

  .status-pill.neutral {
    background: #eef3f8;
    color: #4d6780;
  }

  .status-pill.success {
    background: #e6f8ef;
    color: #1f6548;
  }

  .status-pill.warn {
    background: #fff4df;
    color: #976329;
  }

  .status-pill.error {
    background: #ffe9ea;
    color: #8d2e35;
  }

  .warning-box {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 12px;
    padding: 10px 12px;
    border-radius: 10px;
    background: #fff4df;
    border: 1px solid #f1d39c;
    color: #865726;
    font-size: 12px;
  }

  .system-actions {
    margin-bottom: 12px;
    flex-wrap: wrap;
  }

  .progress-card {
    padding: 12px;
    border-radius: 12px;
    border: 1px solid #bfd8f1;
    background: linear-gradient(140deg, #f6fbff 0%, #eef6ff 100%);
    display: flex;
    flex-direction: column;
    gap: 8px;
    margin-bottom: 12px;
  }

  .progress-head {
    display: flex;
    justify-content: space-between;
    gap: 10px;
    font-size: 12px;
    color: #3f6588;
  }

  .progress-track {
    height: 10px;
    background: #dce9f5;
    border-radius: 999px;
    overflow: hidden;
  }

  .progress-fill {
    display: block;
    height: 100%;
    background: linear-gradient(90deg, #3d85c7 0%, #5ea5e1 100%);
    border-radius: inherit;
  }

  .progress-card small {
    color: #6683a1;
    font-size: 11px;
  }

  .detail-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 10px;
    margin-bottom: 12px;
  }

  .detail-item {
    padding: 10px 12px;
    border-radius: 10px;
    border: 1px solid #d9e6f1;
    background: #fbfdff;
    display: flex;
    flex-direction: column;
    gap: 3px;
  }

  .detail-item span {
    font-size: 11px;
    color: #6b84a0;
  }

  .detail-item strong {
    color: #27496a;
    font-size: 13px;
  }

  .notes-card {
    border: 1px solid #d8e4ef;
    border-radius: 12px;
    background: #fcfeff;
    padding: 12px;
  }

  .notes-head {
    display: flex;
    justify-content: space-between;
    gap: 12px;
    align-items: baseline;
    margin-bottom: 8px;
  }

  .notes-head strong {
    color: #20405f;
  }

  .notes-head small {
    color: #7590aa;
    font-size: 11px;
  }

  .notes-card pre {
    margin: 0;
    white-space: pre-wrap;
    word-break: break-word;
    font-family: "Consolas", "Microsoft YaHei UI", monospace;
    font-size: 12px;
    line-height: 1.6;
    color: #385b7d;
    max-height: 220px;
    overflow: auto;
  }

  .hint {
    font-size: 12px;
    color: #5f7d99;
    margin: 12px 0 0;
    line-height: 1.5;
  }

  @media (max-width: 1100px) {
    .system-grid,
    .detail-grid {
      grid-template-columns: 1fr;
    }

    .system-card-head,
    .notes-head,
    .progress-head {
      flex-direction: column;
      align-items: flex-start;
    }

    .meta-stack {
      text-align: left;
    }
  }
</style>
