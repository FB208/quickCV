# AGENTS 指南（quickCV）

## 项目事实
- 桌面应用：Tauri v2 + Rust，前端：Svelte 4 + TypeScript + Vite；主要目标平台是 Windows。
- 使用 npm 和 `package-lock.json`；CI 用 `npm ci`，本地首次安装可用 `npm install`。
- Rust 最低版本来自 `src-tauri/Cargo.toml`：`rust-version = "1.86"`。
- 当前未发现 `.cursor/rules/`、`.cursorrules`、`.github/copilot-instructions.md`、`CLAUDE.md` 或 `opencode.json`。

## 常用命令
- 安装依赖：`npm install`
- 前端开发服务器：`npm run dev`
- Tauri 联调：`npm run tauri dev`（`tauri.conf.json` 会先跑 `npm run dev`）
- 前端构建：`npm run build`
- Tauri 打包：`npm run tauri build`（会先跑 `npm run build`，产物在 `src-tauri/target/release/bundle/`）
- TypeScript 检查：`npx tsc --noEmit`
- Rust 检查：`cargo check --manifest-path src-tauri/Cargo.toml`
- Rust 格式检查：`cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`
- Rust Clippy：`cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`

## 测试现状
- `package.json` 只有 `dev`、`build`、`preview`、`tauri`；不要虚构 `npm test`、`npm run lint` 或 `npm run typecheck`。
- 目前可验证的单测在 `src-tauri/src/sync.rs`。
- 全量 Rust 测试：`cargo test --manifest-path src-tauri/Cargo.toml`
- 同步模块测试：`cargo test --manifest-path src-tauri/Cargo.toml sync::tests`
- 单测示例：`cargo test --manifest-path src-tauri/Cargo.toml sync::tests::merge_resolves_duplicate_keys -- --exact`
- 改前端至少跑 `npx tsc --noEmit` 和 `npm run build`；改同步逻辑优先跑 `cargo test --manifest-path src-tauri/Cargo.toml sync::tests`。

## 运行与配置坑点
- Vite dev server 固定 `23456` 且 `strictPort: true`；Tauri `devUrl` 也指向 `http://localhost:23456`。
- `vite.config.ts` 忽略 `src-tauri/**` 的前端 watch；改 Rust 代码不要期待 Vite HMR 处理。
- `static/` 是 Vite publicDir；不要把它误当作构建输出。
- Tauri 配置包含两个窗口标签：`main` 和 `overlay`；前端根据窗口 label 在 `src/main.ts` 选择挂载 `App.svelte` 或 `Overlay.svelte`。
- Tauri updater 打包已启用；本地需要真实更新签名时设置 `TAURI_SIGNING_PRIVATE_KEY_PATH` 和 `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`。

## 代码边界
- 前端主入口：`src/main.ts`；设置页：`src/App.svelte`；浮窗：`src/Overlay.svelte`；设置页子组件在 `src/components/`。
- 前端 Tauri invoke 封装集中在 `src/lib/api.ts`；共享 TS 类型在 `src/lib/types.ts`。
- Rust 命令注册和 builder 组装在 `src-tauri/src/main.rs`；命令函数应保持薄，业务放到 `src-tauri/src/services/*.rs` 或核心模块。
- 启动、托盘、快捷键、窗口事件编排在 `src-tauri/src/app_bootstrap.rs`；浮窗状态与行为在 `src-tauri/src/overlay_window.rs`。
- 核心数据/存储/同步/WebDAV 分别在 `models.rs`、`storage.rs`、`sync.rs`、`webdav.rs`。

## 跨端模型约束
- 改 `Settings`、`TemplateStore`、`Folder`、`TemplateItem`、同步/更新返回结构时，同时检查 `src/lib/types.ts`、`src-tauri/src/models.rs`、`src/lib/api.ts` 和消费端组件。
- TS 字段是 `camelCase`；Rust 字段通常是 `snake_case` 并通过 serde 暴露为 `camelCase`。
- WebDAV 同步合并依赖 `datasetVersion`、`lastSyncedVersion`、`updatedAt`、`deletedAt`、`sortUpdatedAt`、`deviceId`；不要只改 UI 字段而忽略合并逻辑和单测。
- 模板 `key` 有唯一性处理；同步冲突会生成“冲突副本”并清理 key 冲突，改相关逻辑必须跑 `sync::tests`。

## 发布流程
- GitHub Actions 只在推送 `v*` 标签时发布 Windows 产物，工作流是 `.github/workflows/release.yml`。
- 发布工作流会用标签版本临时更新 `package.json`、`src-tauri/tauri.conf.json` 和 `src-tauri/Cargo.toml` 后构建。
- Release/updater 需要 Secrets：`TAURI_SIGNING_PRIVATE_KEY`（私钥内容，不是 `.pub` 或路径）和 `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`。
- 不要提交 `dist/`、`src-tauri/target/` 或签名私钥/密码等本地产物与敏感信息。
