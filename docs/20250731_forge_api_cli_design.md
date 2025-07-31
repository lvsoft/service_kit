# `forge-api-cli`: 动态 OpenAPI 命令行客户端设计文档

## 1. 概述 (Overview)

`forge-api-cli` 是一个独立的、纯 Rust 实现的动态 API 命令行客户端。它通过读取一个正在运行的服务的 OpenAPI v3 规范，自动生成一套完整的命令行工具，用于与该服务的 API 进行交互。

此工具旨在取代 `oas-cli` 等外部依赖，为 `service_kit` 生态提供一个无缝、高效且技术栈统一的 API 测试与探索解决方案。

**核心价值**:

-   **独立部署**: 作为一个独立的 Crate，`forge-api-cli` 可以被单独编译、安装和使用。
-   **纯 Rust 实现**: 保持了工具链的技术纯粹性，通过 `cargo install` 分发，无需 `npm` 等外部运行时。
-   **运行时动态生成**: CLI 的命令和参数完全在**运行时**根据目标服务的 OpenAPI 规范动态生成，确保了测试工具与 API 实现的绝对同步。
-   **双模式交互**: 同时支持用于脚本的**直接命令模式**和用于探索的**交互模式 (REPL)**，并由 `reedline` 提供强大的补全和历史记录功能。

---

## 2. 架构与集成

`forge-api-cli` 与 `service_kit` 的生态系统协同工作，但保持松耦合。

1.  **服务**: 使用 `service_kit` 构建的微服务，它通过 `utoipa` 在 `/api-docs/openapi.json` 路径上暴露其 OpenAPI 规范。
2.  **`forge-cli` (可选)**: `service_kit` 内置的 `forge-cli` 可以提供一个别名或辅助命令来调用 `forge-api-cli`，但这并非强制要求。
3.  **`forge-api-cli`**: 核心工具。它是一个独立的二进制文件，接收一个目标服务的 URL 作为参数。

### 工作流程

`forge-api-cli` 采用巧妙的**双重解析 (Two-Pass Parsing)** 机制：

1.  **初次解析 (Pass 1)**: `clap` 首先进行一次最小化的解析，只为了识别出用户提供的**目标服务 URL**。
2.  **获取 Spec**: 工具使用该 URL，通过 HTTP 请求获取服务的 `/api-docs/openapi.json` 文件内容。
3.  **动态构建 CLI**: `forge-api-cli` 在内存中解析 OpenAPI Spec，并根据其 `paths` 和 `components` 动态地构建一个完整的 `clap` 命令树。
4.  **二次解析 (Pass 2)**: 使用这个动态生成的 `clap` 应用，来解析用户在 URL 之后提供的所有参数（如 `v1.health.get` 等）。
5.  **执行**: 根据二次解析的结果，判断是进入**直接命令模式**还是**交互模式**。

---

## 3. 用户体验 (User Experience)

### 3.1. 模式一: 直接命令模式

用于快速执行单次 API 调用。

```bash
# 1. 启动你的服务
cargo run

# 2. 在另一个终端调用 forge-api-cli
# 格式: forge-api-cli <BASE_URL> <API_COMMAND> [OPTIONS]
cargo run --bin forge-api-cli http://127.0.0.1:3000 v1.hello.get
```

### 3.2. 模式二: 交互模式 (REPL)

当用户只提供 URL 而不带任何 API 命令时，进入交互模式。

```bash
# 1. 启动服务
cargo run

# 2. 进入 REPL
cargo run --bin forge-api-cli http://127.0.0.1:3000
```

```
(api-cli) > help  # 显示所有可用的命令
Available commands:
  v1.hello.get
  ...

(api-cli) > v1.hello.get <TAB>  # 按下 Tab 键，自动补全参数
(api-cli) > v1.hello.get

{
  "message": "Hello, World!"
}

(api-cli) > exit
```

**交互模式由 `reedline` 驱动，提供以下核心特性**:

-   **命令历史**: 跨会话的历史记录。
-   **智能补全**: 基于 OpenAPI 规范动态生成命令和参数的补全规则。
-   **用户友好的编辑体验**: 支持 Emacs/Vi 模式，多行输入等。

---

## 4. 技术实现方案

### 4.1. 依赖

-   `clap`: 用于命令行参数的双重解析。
-   `reqwest`: 用于获取 OpenAPI Spec 和执行 API 调用。
-   `oas` (或类似的 OpenAPI 解析库): 用于将 JSON Spec 解析为强类型的 Rust 结构体。
-   `reedline`: 用于构建交互式 REPL 环境。
-   `serde`, `serde_json`: 核心序列化/反序列化工具。
-   `colored`, `nu-ansi-term`: 用于美化终端输出。
-   `shlex`: 用于在 REPL 模式下安全地解析用户输入的命令行字符串。

### 4.2. 核心模块

-   `main.rs`: 负责双重解析和模式分发。
-   `client.rs`: 负责所有 HTTP 相关的逻辑（获取 Spec，发送 API 请求）。
-   `cli.rs`: 负责动态构建 `clap` 应用的核心逻辑。
-   `repl.rs`: 负责 `reedline` REPL 的所有逻辑，包括提示符、历史和调用执行。
-   `completer.rs`: 实现 `reedline` 的 `Completer` trait，提供动态命令补全。
-   `error.rs`: 定义项目专属的 `Error` 和 `Result` 类型。

---

## 5. 开发路线图

### MVP (已完成)

-   [x] **独立 Crate**: 项目已作为一个独立的 `forge-api-cli` crate 存在。
-   [x] **双重解析**: 已实现基于 `clap` 的双重解析机制。
-   [x] **Spec 解析**: 已实现通过 `reqwest` 和 `oas` 库获取并解析 OpenAPI Spec。
-   [x] **动态命令构建**: 已实现从 Spec 动态生成 `clap` 命令树。
-   [x] **直接命令模式**: 支持通过命令行直接调用 API。
-   [x] **交互模式 (REPL)**: 已通过 `reedline` 实现了一个功能性的 REPL 环境。
-   [x] **智能补全**: 已为 REPL 提供了基本的命令和参数自动补全。

### 未来扩展

-   [ ] **更完善的参数支持**: 支持文件上传 (`multipart/form-data`)。
-   [ ] **认证支持**: 为 REPL 添加登录/认证命令，以便在后续请求中自动附加 `Authorization` 头。
-   [ ] **环境管理**: 允许用户保存多个服务 URL，并方便地在它们之间切换。
-   [ ] **响应处理**: 支持对响应进行 `jq` 风格的过滤和查询。
