<script lang="ts">
  export let appVersion = "--";
  export let latestVersion = "--";
  export let checkingVersion = false;

  export let onRunReleaseCheck: () => void;
  export let onOpenReleasePage: () => void;
  export let onOpenConfigFolder: () => void;
</script>

<section class="qc-panel qc-panel-page">
  <div class="qc-page-head">
    <h2 class="qc-page-title"><span class="ms-icon">computer</span>系统</h2>
    <p class="qc-page-desc">查看版本状态并处理更新与配置入口。</p>
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

    <section class="qc-card system-card">
      <h3 class="qc-card-title"><span class="ms-icon">system_update</span>更新与维护</h3>
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
  }
</style>
