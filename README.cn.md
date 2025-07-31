# Service Kit: 一站式 Rust 微服务开发套件

`service_kit` 是一个为本项目量身打造的、一站式的 Rust 微服务开发套件。其核心目标是**将最佳实践固化为工具,将重复工作自动化**,从而让开发者能专注于核心业务逻辑的实现。

通过引入 `service_kit`,我们旨在建立一套标准化的微服务开发范式,确保所有服务在 API 规范、代码质量、类型安全和开发流程上保持高度一致。

## 核心组件

`service_kit` 主要由以下三个核心组件构成:

### 1. `#[api_dto]` 过程宏

这是 `service_kit` 的灵魂。开发者只需在数据传输对象(DTO)结构体上添加 `#[api_dto]`,即可自动获得:

- `serde` 的序列化/反序列化能力 (`Serialize`, `Deserialize`)。
- `utoipa` 的 OpenAPI Schema 生成能力 (`ToSchema`)。
- `ts-rs` 的 TypeScript 类型定义生成能力 (`TS`)。
- 常用的调试和克隆能力 (`Debug`, `Clone`)。
- **内置的递归问题解决方案**: 自动处理 `Box<Self>` 等递归类型,避免 `utoipa` 编译失败。
- **灵活的定制能力**: 支持通过 `#[api_dto(rename_all = "..."
)]` 覆盖命名策略,并通过 `Cargo.toml` 进行全局配置。

### 2. `forge_cli` & `forge-cli` 集成构建工具

`service_kit` 提供了一套强大的命令行工具来封装开发、测试和交互的完整流程。

- **`forge_cli`**: 内置于 `service_kit` 依赖中,通过 `cargo forge` 别名调用,提供构建与质量保障命令:
    - `cargo forge generate-ts`: 为所有 `#[api_dto]` 结构体生成 TypeScript 类型定义。
    - `cargo forge lint`: 使用 `cargo clippy` 对项目进行严格的代码质量检查。
    - `cargo forge test`: 运行项目内的所有单元和集成测试。
- **`forge-cli`**: 一个独立的、动态的 API 客户端,提供与 API 交互的能力。

### 3. `service-template` 服务模板

一个标准的 `cargo-generator` 模板,允许开发者通过一条命令快速初始化一个全新的、符合 `service_kit` 规范的微服务项目骨架。

---

## 快速上手指南 (Getting Started)

本指南将指导你创建并运行你的第一个 `service_kit` 微服务。

### 步骤 1: 安装先决条件

你需要安装 `cargo-generate`。

```bash
# 安装项目模板生成器
cargo install cargo-generate
```

### 步骤 2: 使用模板创建新服务

使用 `cargo generate` 命令，通过 Git 仓库地址来创建一个名为 `my-awesome-service` 的新项目。

```bash
# 此命令会从 GitHub 克隆 service_kit 仓库，并使用其中的 service-template 目录作为模板
cargo generate --git https://github.com/lvsoft/service_kit.git --subfolder service-template --name my-awesome-service
```

### 步骤 3: 运行服务

进入新创建的项目目录并启动服务。

```bash
cd my-awesome-service
cargo run
```

---

## `forge` 命令演示

### `cargo forge` (构建 & 质量)

所有 `cargo forge` 命令都应在**你生成的服务目录**(例如 `my-awesome-service/`)下运行。这些命令由你的项目依赖 `service_kit` 提供。

- **`cargo forge test`**: 运行项目的所有测试。
- **`cargo forge lint`**: 对项目进行严格的代码质量检查。
- **`cargo forge generate-ts`**: 为项目中的 DTO 生成 TypeScript 类型定义。

### `forge-cli` (API 客户端)

`service_kit` 提供了一个名为 `forge-cli` 的二进制程序，它是一个基于 OpenAPI 规范的交互式 API 客户端。

**安装**:
在 `service_kit` 发布到 crates.io 后, 你可以通过 `cargo install` 来安装它。请注意，需要启用 `api-cli` 功能标志。

```bash
cargo install service_kit --features api-cli
```

**前置条件**: 确保你的服务正在另一个终端中运行 (`cargo run`)。

你可以使用它来调用服务中的 API 端点。它支持两种模式:

#### 1. 直接命令模式

用于快速、一次性的 API 调用。

```sh
# 格式: forge-api-cli <BASE_URL> <API_COMMAND> [OPTIONS]
forge-api-cli http://127.0.0.1:3000 v1.hello.get
```
```json
{
  "message": "Hello, World!"
}
```

#### 2. 交互模式 (REPL)

只提供 URL 即可进入交互式环境,非常适合 API 的探索和调试。

```sh
forge-api-cli http://127.0.0.1:3000
```
```
(api-cli) > help  # 显示所有可用的命令
(api-cli) > v1.hello.get <Tab>  # 自动补全
(api-cli) > v1.hello.get
{
  "message": "Hello, World!"
}
```

---

## 示例项目

本项目包含一个更完整的示例项目,位于 `examples/product-service`。它展示了更复杂的 DTO、递归结构和自定义命名策略的用法,可作为开发的参考。
