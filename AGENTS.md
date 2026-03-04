# AGENTS 指南（quickCV）
本文件面向在本仓库工作的自动化 coding agents。
目标：统一构建/测试流程与代码风格，减少无关改动。

## 0) 项目概览
- 前端：Svelte 4 + TypeScript + Vite
- 桌面端：Tauri v2（Rust）
- 主要平台：Windows（需兼容后续 macOS）
- 包管理：npm（使用 `package-lock.json`）

## 1) 外部规则文件状态（已检查）
- `.cursor/rules/`：未找到
- `.cursorrules`：未找到
- `.github/copilot-instructions.md`：未找到
- 结论：当前以仓库既有风格 + 本文件约定为准

## 2) 开发环境要求
- Node.js：建议 16+（CI 为 20）
- Rust：`1.86+`（见 `src-tauri/Cargo.toml`）
- 操作系统：优先 Windows
- 文件编码：统一 UTF-8（避免 ANSI/GBK）
- 新增脚本须可在 PowerShell/cmd 直接运行

## 3) 安装、启动、构建命令
在仓库根目录 `D:\CodeSpace\quickCV` 执行：

```bash
npm install
npm run dev
npm run tauri dev
npm run build
npm run tauri build
```

说明：
- `npm run dev`：仅前端开发服务器
- `npm run tauri dev`：前后端联调入口

## 4) Lint / 类型检查 / 格式化
当前 `package.json` 未定义 `lint`、`test`、`typecheck` 脚本，不要虚构命令。

```bash
npx tsc --noEmit
cargo check --manifest-path src-tauri/Cargo.toml
cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
```

## 5) 测试命令（重点：单测）
当前测试主要在 Rust：`src-tauri/src/sync.rs`。

```bash
# 全量测试
cargo test --manifest-path src-tauri/Cargo.toml

# 跑模块测试（含 tests 子模块）
cargo test --manifest-path src-tauri/Cargo.toml sync::tests

# 跑单个测试（推荐 --exact）
cargo test --manifest-path src-tauri/Cargo.toml sync::tests::merge_resolves_duplicate_keys -- --exact
cargo test --manifest-path src-tauri/Cargo.toml sync::tests::merge_creates_conflict_copy_when_both_sides_changed -- --exact
```

补充：未发现前端测试框架（Vitest/Jest/Playwright）。

## 6) 提交前最小自检
建议至少执行：

```bash
npx tsc --noEmit
cargo test --manifest-path src-tauri/Cargo.toml sync::tests
npm run build
```

若改了 Rust/Tauri 逻辑，再补：

```bash
cargo check --manifest-path src-tauri/Cargo.toml
```

## 7) 目录与职责边界
- 前端入口：`src/main.ts`（根据窗口标签挂载 `App`/`Overlay`）
- 主设置页：`src/App.svelte`
- 子组件：`src/components/GeneralTab.svelte`、`src/components/TemplatesTab.svelte`、`src/components/SystemTab.svelte`
- 浮窗页：`src/Overlay.svelte`
- 前端 API 边界：`src/lib/api.ts`
- 前端类型：`src/lib/types.ts`
- Rust 入口：`src-tauri/src/main.rs`（命令注册 + builder 组装）
- 启动编排：`src-tauri/src/app_bootstrap.rs`
- 浮窗状态与窗口行为：`src-tauri/src/overlay_window.rs`
- 服务层：`src-tauri/src/services/*.rs`
- 核心基础模块：`src-tauri/src/models.rs`、`src-tauri/src/storage.rs`、`src-tauri/src/sync.rs`、`src-tauri/src/webdav.rs`

放置原则：
- UI 结构与交互放 Svelte
- Tauri invoke 调用封装放 `src/lib/api.ts`
- 跨端模型同时维护于 TS 与 Rust
- 命令入口尽量薄，业务下沉到 `services/`

## 8) TypeScript / Svelte 风格

### 8.1 格式与导入
- 使用双引号 `"` 与分号 `;`
- 缩进 2 空格
- 避免无意义格式化大改
- 导入顺序建议：第三方 -> 本地模块 -> `import type`

### 8.2 类型规范
- `strict` 已开启，新增代码必须通过 strict
- 函数尽量写显式返回类型（如 `(): Promise<void>`）
- 用 `type`/`interface` 明确数据结构
- 避免 `any`，必要时用 `unknown` + 类型收窄
- 公共数据结构优先放 `src/lib/types.ts`

### 8.3 命名规范
- 变量/函数：`camelCase`
- 类型/接口：`PascalCase`
- 常量：优先 `const`，必要时再 `let`
- 事件处理函数使用动词前缀（如 `openXxx`、`runXxx`、`saveXxx`）

### 8.4 Svelte 约定
- 派生状态使用 `$:`，保持数据流可推导
- 异步流程使用 `try/catch/finally` 管理 `busy`/`loading`
- 输入事件中需要类型断言时，优先封装辅助函数复用
- 避免把复杂计算直接放模板表达式

## 9) Rust 风格
- 缩进 4 空格，交由 `rustfmt`
- 导入顺序建议：`std` -> 第三方 crate -> `crate::` 模块
- 函数/变量：`snake_case`
- 结构体/枚举：`PascalCase`
- 常量：全大写下划线
- Tauri 命令统一 `#[tauri::command]`
- 业务逻辑优先放 `services::*`，`main.rs` 仅做命令转发
- 可失败路径优先 `Result<T, String>`，错误文案要带中文上下文
- 使用 `map_err(|e| format!("...: {e}"))` 保留底层错误
- 避免 `unwrap()`；可用 `unwrap_or` / `unwrap_or_default` 做可控降级
- 跨端字段统一 `#[serde(rename_all = "camelCase")]`

## 10) 跨端数据一致性（重要）
- TS 字段使用 `camelCase`（如 `lastSyncedVersion`）
- Rust 字段使用 `snake_case`，通过 serde 映射到 camelCase
- 改字段名必须同步修改：Rust 模型、TS 类型、invoke 参数、消费端逻辑
- `TemplateStore` / `Settings` 属于核心模型，改动前先全链路检查

## 11) 错误处理与日志
- 前端统一使用 `asErrorMessage` 把 `unknown` 转可读文案
- 前端失败时不要吞错，至少更新 `notice` 或 `hint`
- Rust 使用 `logger::info/warn/error` 记录关键流程与失败原因
- 错误文案尽量可执行（例如提示先配置 WebDAV）
- 网络/同步错误应包含必要上下文（阶段、状态、目标）

## 12) 安全与变更边界
- 禁止提交密钥、账号、密码、token 等敏感信息
- 不要顺手提交 `dist/`、`target/` 等构建产物（除非任务明确要求）
- 发布流程变更前先查看 `.github/workflows/release.yml`
- 优先最小必要改动，避免与任务无关的重构或格式刷屏

## 13) Agent 执行建议
- 修改前先阅读相关模块，确认边界与调用链
- 保持小步迭代，便于回滚和定位问题
- 完成后至少汇报：执行命令、结果、未覆盖风险
- 新增规则文件（Cursor/Copilot）后，需同步更新本文件
