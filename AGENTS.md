# AGENTS 指南（quickCV）

本文件面向在本仓库工作的自动化 coding agents。
目标：统一构建/测试流程与代码风格，减少无效改动。

## 0) 仓库与技术栈
- 前端：Svelte 4 + TypeScript + Vite
- 桌面端：Tauri v2（Rust）
- 平台重点：Windows（兼容后续 macOS）
- 包管理：npm（存在 `package-lock.json`）

## 1) 外部规则文件检查结果
已检查以下来源：
- `.cursor/rules/`：未找到
- `.cursorrules`：未找到
- `.github/copilot-instructions.md`：未找到

结论：当前以仓库现有风格与本文件约定为准。

## 2) 开发环境与编码要求
- Node.js：建议 16+（CI 使用 20）
- Rust：`1.86+`（见 `src-tauri/Cargo.toml`）
- 操作系统：优先 Windows
- 文本编码：统一 UTF-8（避免 ANSI/GBK）
- 新增脚本需在 Windows PowerShell/cmd 可运行

## 3) 安装、启动、构建命令
在仓库根目录 `D:\CodeSpace\quickCV` 执行：

```bash
npm install

# 前端开发
npm run dev

# Tauri 联调（前后端）
npm run tauri dev

# 前端构建
npm run build

# Tauri 打包
npm run tauri build
```

## 4) Lint / 格式化 / 类型检查现状
当前 `package.json` 未提供 `lint`、`test`、`typecheck` 脚本。

可用“等价检查”命令：

```bash
# 前端类型检查
npx tsc --noEmit

# Rust 编译检查
cargo check --manifest-path src-tauri/Cargo.toml

# Rust 格式检查（若 rustfmt 已安装）
cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check

# Rust Clippy（若 clippy 已安装）
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
```

说明：
- 未引入前端 lint 工具前，不要伪造 `npm run lint`。
- 若后续引入 ESLint/Prettier，请同步更新本文件和脚本。

## 5) 测试命令（重点：单测）
当前测试集中在 Rust（`src-tauri/src/sync.rs`）。

```bash
# 全量 Rust 测试
cargo test --manifest-path src-tauri/Cargo.toml

# 跑某个模块（含 tests 子模块）
cargo test --manifest-path src-tauri/Cargo.toml sync::tests

# 跑单个测试（推荐 --exact）
cargo test --manifest-path src-tauri/Cargo.toml merge_resolves_duplicate_keys -- --exact
cargo test --manifest-path src-tauri/Cargo.toml merge_creates_conflict_copy_when_both_sides_changed -- --exact
```

补充：当前未配置前端测试框架（Vitest/Jest/Playwright 均未发现）。

## 6) 提交前最小自检建议

```bash
npx tsc --noEmit
cargo test --manifest-path src-tauri/Cargo.toml
npm run build
```

若改动 Tauri/Rust 逻辑，再执行：

```bash
cargo check --manifest-path src-tauri/Cargo.toml
```

## 7) 代码组织约定
- 前端入口：`src/main.ts`
- 主设置页：`src/App.svelte`
- 浮窗页：`src/Overlay.svelte`
- 前端 API 边界：`src/lib/api.ts`
- 前端类型：`src/lib/types.ts`
- Rust 入口：`src-tauri/src/main.rs`
- Rust 子模块：`models.rs`、`storage.rs`、`sync.rs`、`webdav.rs`

功能放置原则：
- UI/交互放 Svelte
- Tauri invoke 交互放 `src/lib/api.ts`
- 跨端数据结构放 `types.ts` + `models.rs`
- 持久化/同步/网络逻辑放 Rust 模块

## 8) TypeScript / Svelte 风格
从现有代码归纳：
- 使用双引号 `"` 与分号 `;`
- 缩进 2 空格
- 函数尽量显式返回类型（如 `(): Promise<void>`）
- 使用 `type`/`interface` 明确数据形状，避免 `any`
- 变量/函数用 `camelCase`，类型用 `PascalCase`
- 常量优先 `const`，必要时才用 `let`
- 复杂异步流程统一 `try/catch/finally` 管理状态位（`busy`/`loading`）
- 错误提示需用户可读（中文友好）
- Svelte 派生状态使用 `$:`，保持单向可推导
- DOM 事件处理函数用动词前缀（`runXxx`/`openXxx`/`saveXxx`）

导入顺序建议：
1. 框架/三方包
2. 本地模块
3. `import type` 类型导入

## 9) Rust 风格
从现有代码归纳：
- 缩进 4 空格，遵循 `rustfmt`
- 函数/变量使用 `snake_case`
- 结构体/枚举使用 `PascalCase`
- 常量使用全大写下划线（如 `RELEASE_API_URL`）
- Tauri 命令使用 `#[tauri::command]`
- 可失败路径优先 `Result<T, String>`，并给中文上下文
- 使用 `map_err(|e| format!("...: {e}"))` 保留底层错误
- 避免 `unwrap()`；可用 `unwrap_or`/`unwrap_or_default` 做受控降级
- 跨端字段统一 `#[serde(rename_all = "camelCase")]`
- 新增数据结构时同步更新 `src/lib/types.ts`

## 10) 命名与数据一致性
- TS 字段：`camelCase`（如 `lastSyncedVersion`）
- Rust 字段：`snake_case`，通过 serde 对齐到 camelCase
- 修改字段名时必须同步修改：
  - Rust 模型
  - TS 类型
  - invoke 参数与返回值消费点
- `TemplateStore` / `Settings` 为跨端核心模型，改动需谨慎

## 11) 错误处理与用户提示
- 前端统一将 `unknown` 转成可展示文案（参考 `asErrorMessage`）
- 前端失败时不要吞错，至少更新 notice/hint
- Rust 错误优先提供可行动建议（如“请先填写 WebDAV 地址”）
- 网络错误应包含状态码与上下文

## 12) 变更边界与安全
- 不要提交密钥、账号、密码等敏感信息
- 不要无关改动 `dist/`、`target/` 产物（除非任务明确要求）
- 修改发布流程前先读 `.github/workflows/release.yml`
- 避免大规模纯格式重排，优先最小必要改动

## 13) Agent 执行建议
- 先读相关模块再改，避免破坏 Tauri 前后端契约
- 优先小步迭代，保证可回滚
- 完成后至少报告：执行命令、结果、未覆盖风险

如后续新增 Cursor/Copilot 规则文件，请将其摘要合并进本 AGENTS.md。
