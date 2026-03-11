# quickCV

quickCV 是一个面向中文场景的轻量级模板插入管理工具，当前使用 **Tauri v2 + Svelte + TypeScript + Rust** 构建，优先支持 Windows，并兼容后续 macOS 适配。

## 已实现（当前）

- 设置页基础框架（简体中文）
- 快捷键录制与全局快捷键注册（触发无边框浮窗）
- WebDAV 配置与连通性测试
- 模板管理（文件夹、模板、key 唯一校验、内容编辑）
- WebDAV 拉取/推送与自动合并（冲突副本 + key 冲突自动处理）
- 系统托盘（设置、退出）
- 关闭主窗口后最小化到托盘（隐藏窗口）
- 浮窗通过全局快捷键触发
- 无边框浮窗模板选择（搜索、上下选择、左右切换、回车插入、Esc 取消）
- 生产环境启动后静默检查新版本，支持软件内下载并安装更新
- 剪贴板恢复增强（文本 / HTML / 图片 / 文件列表备份恢复）
- GitHub Actions tag 自动打包发布工作流

## 开发环境

- Node.js 16+
- Rust 1.86+

> 如果你本地使用更高版本 Node.js / Rust，也可以直接运行。

## 本地运行

```bash
npm install
npm run tauri dev
```

仅调试前端：

```bash
npm run dev
```

## 打包

```bash
npm run tauri build
```

- 本地打包已启用 updater 产物生成；若要实际生成可安装更新包签名，请在环境变量中提供签名私钥。
- 构建结果位于 `src-tauri/target/release/bundle/`。

PowerShell 示例：

```powershell
$env:TAURI_SIGNING_PRIVATE_KEY_PATH="$env:USERPROFILE\.tauri\quickcv-updater.key"
npm run tauri build
```

## 自动发布

- 工作流文件：`.github/workflows/release.yml`
- 触发方式：推送标签（例如 `v0.1.0`）
- 产物发布到 GitHub Releases，并生成 `latest.json` 供软件内更新使用
- 需要配置 GitHub Secret：`TAURI_SIGNING_PRIVATE_KEY`
- 当前工作流按“无密码私钥”设计，不需要也不要配置 `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`
- 工作流会显式传入空字符串 `TAURI_SIGNING_PRIVATE_KEY_PASSWORD=""`，用于兼容这类“无密码生成但仍需空密码解锁”的 updater 私钥
- `TAURI_SIGNING_PRIVATE_KEY` 应填写私钥文件内容本身，不是 `.pub`，也不是文件路径
- 首个支持软件内更新的版本属于桥接版，老用户仍需手动安装一次；此后即可在软件内更新

## 目录说明

- `src/`：Svelte 前端设置界面
- `src-tauri/src/`：Rust 核心逻辑（配置、存储、同步、托盘）
- `src-tauri/capabilities/`：Tauri 能力声明
- `static/logo.png`：应用原始图标

## 下一阶段计划

- Windows 场景稳定性回归测试与细节修复
- 浮窗交互细节优化（分栏焦点、动画、更多快捷操作）
- macOS 权限引导与输入注入适配
