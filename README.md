# Service Kit: 一站式 Rust 微服务开发套件

`service_kit` 是一个为本项目量身打造的、一站式的 Rust 微服务开发套件。其核心目标是**将最佳实践固化为工具，将重复工作自动化**，从而让开发者能专注于核心业务逻辑的实现。

通过引入 `service_kit`，我们旨在建立一套标准化的微服务开发范式，确保所有服务在 API 规范、代码质量、类型安全和开发流程上保持高度一致。

## 核心组件

`service_kit` 主要由以下三个核心组件构成：

### 1. `#[api_dto]` 过程宏

这是 `service_kit` 的灵魂。开发者只需在数据传输对象（DTO）结构体上添加 `#[api_dto]`，即可自动获得：

-   `serde` 的序列化/反序列化能力 (`Serialize`, `Deserialize`)。
-   `utoipa` 的 OpenAPI Schema 生成能力 (`ToSchema`)。
-   `ts-rs` 的 TypeScript 类型定义生成能力 (`TS`)。
-   常用的调试和克隆能力 (`Debug`, `Clone`)。
-   **内置的递归问题解决方案**：自动处理 `Box<Self>` 等递归类型，避免 `utoipa` 编译失败。
-   **灵活的定制能力**：支持通过 `#[api_dto(rename_all = "...")]` 覆盖命名策略，并通过 `Cargo.toml` 进行全局配置。

### 2. `forge_cli` 集成构建工具

这是一个内置于 `service_kit` 的命令行工具，封装了微服务开发、测试和构建的完整流程。通过 `cargo forge` 调用。

-   `cargo forge generate-ts`: 扫描项目，为所有 `#[api_dto]` 结构体生成 TypeScript 类型定义。
-   `cargo forge lint`: 使用 `cargo clippy` 对整个工作区进行严格的代码质量检查。
-   `cargo forge test`: 运行工作区内的所有单元和集成测试。
-   `cargo forge api-cli`: **(新功能)** 基于 OpenAPI 规范，提供一个交互式的命令行客户端来测试 API。

### 3. `service-template` 服务模板

一个标准的 `cargo-generator` 模板，允许开发者通过一条命令快速初始化一个全新的、符合 `service_kit` 规范的微服务项目骨架。

---

## 快速上手指南 (Getting Started)

本指南将指导你创建并运行你的第一个 `service_kit` 微服务。

### 步骤 1: 安装先决条件

你需要安装 `cargo-generate` 和 `oas-cli`。

```bash
# 安装项目模板生成器
cargo install cargo-generate

# 安装 OpenAPI 命令行客户端 (用于 api-cli 功能)
npm install -g oas-cli
```

### 步骤 2: 使用模板创建新服务

使用 `cargo generate` 命令，指向本地的 `service-template` 目录来创建一个名为 `my-awesome-service` 的新项目。

```bash
# 在 service_kit 项目的根目录运行
cargo generate --path ./service-template --name my-awesome-service
```

### 步骤 3: 运行服务

进入新创建的项目目录并启动服务。

```bash
cd my-awesome-service
cargo run
```

服务启动后，你应该能看到类似以下的输出：

```
🚀 Server running at http://127.0.0.1:3000
📚 Swagger UI available at http://127.0.0.1:3000/swagger-ui
```

---

## `cargo forge` 命令演示

所有 `cargo forge` 命令都应在**你生成的服务目录**（例如 `my-awesome-service/`）下运行。

### `cargo forge test`

运行项目的所有测试。

```sh
$ cargo forge test
▶️  Running all tests...
   Finished test [unoptimized + debuginfo] target(s) in ...
     Running unittests src/lib.rs (...)
running 0 tests
...
✅ All tests passed.
```

### `cargo forge lint`

对项目进行严格的代码质量检查。

```sh
$ cargo forge lint
▶️  Running linter...
   Running 'cargo clippy' with -D warnings...
    Checking my-awesome-service v0.1.0 (...)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in ...
✅ All checks passed.
```

### `cargo forge generate-ts`

为项目中的 DTO 生成 TypeScript 类型定义。

```sh
$ cargo forge generate-ts
▶️  Generating TypeScript types by running tests...
   Finished test [unoptimized + debuginfo] target(s) in ...
     Running unittests src/lib.rs (...)
...
✅ TypeScript types generated successfully.
   You can find them in: /path/to/my-awesome-service/generated/ts
```

### `cargo forge api-cli` (API 客户端)

这是一个基于 OpenAPI 规范的交互式 API 客户端。

**前置条件**: 确保你的服务正在另一个终端中运行 (`cargo run`)。

你可以使用它来调用服务中的 API 端点。例如，模板项目包含一个 `GET /v1/hello` 端点：

```sh
$ cargo forge api-cli v1.hello.get
▶️  Generating OpenAPI specification...
✅ OpenAPI specification generated at: /path/to/my-awesome-service/target/openapi.json
▶️  Invoking `oas` with the generated spec...

{
  "message": "Hello, World!"
}
```

`oas-cli` 会自动将 OpenAPI 路径 (`/v1/hello`) 转换为 CLI 子命令 (`v1.hello.get`)。你可以使用 `--help` 查看所有可用的命令：

```sh
cargo forge api-cli --help
```

---

## 示例项目

本项目包含一个更完整的示例项目，位于 `examples/product-service`。它展示了更复杂的 DTO、递归结构和自定义命名策略的用法，可作为开发的参考。
