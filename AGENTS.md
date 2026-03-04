# AGENTS 指南（quickCV）
本文件给在本仓库工作的自动化 coding agents 使用。
目标：统一构建/测试与代码风格，减少无关改动与返工。

## 0) 项目概览
- 前端：Svelte 4 + TypeScript + Vite
- 桌面端：Tauri v2（Rust）
- 主要平台：Windows（需兼容后续 macOS）
- 包管理：npm（以 `package-lock.json` 为准）

## 1) 外部规则文件状态（已检查）
- `.cursor/rules/`：未找到
- `.cursorrules`：未找到
- `.github/copilot-instructions.md`：未找到
- 结论：当前以仓库既有风格 + 本文件约定为准。

## 2) 开发环境要求
- Node.js：建议 16+（CI 使用 20）
- Rust：`1.86+`（见 `src-tauri/Cargo.toml`）
- 操作系统：优先 Windows
- 编码：统一 UTF-8（避免 ANSI/GBK）
- 新增脚本必须可在 PowerShell/cmd 直接运行

## 3) 安装、启动、构建命令
在仓库根目录 `D:\CodeSpace\quickCV` 运行：

```bash
npm install
npm run dev
npm run tauri dev
npm run build
npm run tauri build
```

说明：
- `npm run dev` 仅前端开发服务器
- `npm run tauri dev` 为前后端联调入口

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
cargo test --manifest-path src-tauri/Cargo.toml
npm run build
```

若改了 Rust/Tauri 逻辑，再补：

```bash
cargo check --manifest-path src-tauri/Cargo.toml
```

## 7) 目录与模块边界
- 前端入口：`src/main.ts`
- 设置页：`src/App.svelte`
- 浮窗页：`src/Overlay.svelte`
- 前端调用边界：`src/lib/api.ts`
- 前端类型：`src/lib/types.ts`
- Rust 入口：`src-tauri/src/main.rs`
- Rust 核心模块：`models.rs`、`storage.rs`、`sync.rs`、`webdav.rs`

放置原则：
- UI/交互放 Svelte
- Tauri invoke 封装放 `src/lib/api.ts`
- 跨端模型同步维护在 `src/lib/types.ts` + `src-tauri/src/models.rs`
- 存储/同步/网络逻辑放 Rust 模块

## 8) TypeScript / Svelte 风格

### 8.1 格式与导入
- 使用双引号 `"` 与分号 `;`
- 缩进 2 空格
- 避免无意义格式重排
- 导入顺序建议：第三方包 -> 本地模块 -> `import type`

### 8.2 类型规范
- `strict` 已开启（见 `tsconfig.json`），新增代码必须通过 strict
- 函数尽量写显式返回类型（如 `(): Promise<void>`）
- 使用 `type`/`interface` 描述数据结构
- 避免 `any`；必要时用 `unknown` + 类型收窄
- 公共数据结构优先放 `src/lib/types.ts`

### 8.3 命名规范
- 变量/函数：`camelCase`
- 类型/接口：`PascalCase`
- 常量：优先 `const`，必要时再 `let`
- 事件处理函数建议用动词前缀：`runXxx` / `openXxx` / `saveXxx`

### 8.4 Svelte 约定
- 派生状态使用 `$:`，保持单向可推导
- 异步流程统一 `try/catch/finally` 管理 `busy` / `loading`
- UI 错误提示使用用户可读中文
- 避免在模板中放复杂计算，复杂逻辑下沉到 `<script>`

## 9) Rust 风格
- 缩进 4 空格，交给 `rustfmt`
- 函数/变量：`snake_case`
- 结构体/枚举：`PascalCase`
- 常量：全大写下划线（如 `RELEASE_API_URL`）
- Tauri 命令使用 `#[tauri::command]`
- 可失败路径优先 `Result<T, String>`，错误文案给中文上下文
- 使用 `map_err(|e| format!("...: {e}"))` 保留底层错误
- 避免 `unwrap()`；可用 `unwrap_or` / `unwrap_or_default` 做受控降级
- 跨端字段统一 `#[serde(rename_all = "camelCase")]`

## 10) 跨端数据一致性（重要）
- TS 字段为 `camelCase`（如 `lastSyncedVersion`）
- Rust 字段为 `snake_case`，通过 serde 映射到 camelCase
- 改字段名时必须同步修改：Rust 模型、TS 类型、invoke 参数与消费点
- `TemplateStore` / `Settings` 为核心模型，改动前先全链路检查

## 11) 错误处理规范
- 前端把 `unknown` 转为可展示文案（参考 `asErrorMessage`）
- 前端失败时不要吞错，至少更新 `notice`/`hint`
- Rust 报错优先提供可执行建议（如“请先填写 WebDAV 地址”）
- 网络错误要包含状态码或上下文信息

## 12) 安全与变更边界
- 禁止提交密钥、账号、密码、token 等敏感信息
- 不要顺手提交 `dist/`、`target/` 等构建产物（任务明确要求除外）
- 修改发布逻辑前先查看 `.github/workflows/release.yml`
- 优先最小必要改动，避免纯格式化大改

## 13) Agent 执行建议
- 先读相关模块再修改，避免破坏前后端契约
- 优先小步迭代，便于回滚与定位问题
- 完成后至少汇报：执行命令、结果、未覆盖风险
- 若将来新增 Cursor/Copilot 规则文件，请把摘要同步到本文件
