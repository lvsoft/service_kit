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

这是一个通过 `xtask` 模式实现的命令行工具，封装了微服务开发、测试和构建的完整流程。通过 `cargo forge` 调用。

-   `cargo forge generate-ts`: 扫描项目，为所有 `#[api_dto]` 结构体生成 TypeScript 类型定义。
-   `cargo forge lint`: 使用 `cargo clippy` 对整个工作区进行严格的代码质量检查。
-   `cargo forge test`: 运行工作区内的所有单元和集成测试。

### 3. `service-template` 服务模板

一个标准的 `cargo-generator` 模板，允许开发者通过一条命令快速初始化一个全新的、符合 `service_kit` 规范的微服务项目骨架。

---

## 快速上手指南 (Getting Started)

本指南将指导你创建并运行你的第一个 `service_kit` 微服务。

### 步骤 1: 安装先决条件

你需要安装 `cargo-generate` 来使用项目模板。

```bash
cargo install cargo-generate
```

### 步骤 2: 使用模板创建新服务

使用 `cargo generate` 命令，指向本地的 `service-template` 目录来创建一个名为 `my-awesome-service` 的新项目。

```bash
# 在 service_kit 项目的根目录运行
cargo generate --path ./service-template --name my-awesome-service
```

`cargo-generate` 会提示你输入作者信息，然后一个全新的服务就会在 `my-awesome-service` 目录中被创建。

### 步骤 3: 运行服务

进入新创建的项目目录并启动服务。

```bash
cd my-awesome-service
cargo run
```

服务启动后，你应该能看到类似以下的输出：

```
🚀 Server running at http://128.0.0.1:3000
📚 Swagger UI available at http://128.0.0.1:3000/swagger-ui
```

现在，你可以访问 `http://127.0.0.1:3000/swagger-ui` 来查看自动生成的 API 文档。

---

## 开发工作流

一个典型的开发周期如下：

1.  **定义 DTO**: 在 `src/dtos.rs` 中使用 `#[api_dto]` 定义你的数据结构。

    ```rust
    // src/dtos.rs
    use service_kit::api_dto;

    #[api_dto]
    pub struct User {
        pub user_id: String,
        pub username: String,
    }
    ```

2.  **编写 Handler**: 在 `src/handlers.rs` 中实现你的业务逻辑。

3.  **注册路由**: 在 `src/main.rs` 中将新的 handler 添加到 `axum` 路由和 `#[openapi]` 宏中。

4.  **验证与生成**:
    -   运行 `cargo forge lint` 和 `cargo forge test` 确保代码质量和正确性。
    -   运行 `cargo forge generate-ts` 来为前端生成最新的 TypeScript 类型。

## 示例项目

本项目包含一个更完整的示例项目，位于 `examples/product-service`。它展示了更复杂的 DTO、递归结构和自定义命名策略的用法，可作为开发的参考。
