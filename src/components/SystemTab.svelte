<script lang="ts">
  export let appVersion = "--";
  export let latestVersion = "--";
  export let checkingVersion = false;

  export let onRunReleaseCheck: () => void;
  export let onOpenReleasePage: () => void;
  export let onOpenConfigFolder: () => void;
</script>

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
        <button class="qc-btn qc-btn-subtle" disabled={checkingVersion} on:click={onRunReleaseCheck}>
          <span class="ms-icon">system_update</span>
          {checkingVersion ? "检查中..." : "检查版本"}
        </button>
        <button class="qc-btn qc-btn-subtle" on:click={onOpenReleasePage}>
          <span class="ms-icon">open_in_new</span>
          打开 Release 页面
        </button>
        <button class="qc-btn qc-btn-subtle" on:click={onOpenConfigFolder}>
          <span class="ms-icon">folder_open</span>
          打开配置文件
        </button>
      </div>
      <p class="hint">系统页保留版本信息与更新入口；模板同步入口已合并到「模板管理」。</p>
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

  .hint {
    font-size: 12px;
    color: #5f7d99;
    margin: 0;
    line-height: 1.5;
  }

  @media (max-width: 1100px) {
    .system-grid {
      grid-template-columns: 1fr;
    }

    .page-head {
      align-items: flex-start;
    }
  }
</style>
