# Service Kit: 微服务开发套件设计文档

## 1. 概述 (Overview)

`service_kit` 是一个为本项目量身打造的、一站式的 Rust 微服务开发套件。其核心目标是**将最佳实践固化为工具，将重复工作自动化**，从而让开发者能专注于核心业务逻辑的实现。

通过引入 `service_kit`，我们旨在建立一套标准化的微服务开发范式，确保所有服务在 API 规范、代码质量、类型安全和开发流程上保持高度一致。开发者只需学习 `service_kit` 的简单约定，即可快速、高效地构建出健壮、可维护的微服务。

**核心价值**:
- **提升效率**: 用一个宏代替繁琐的样板代码。
- **保证质量**: 自动化执行 API 契约验证和 100% 强类型约束检查。
- **降低心智负担**: 封装底层工具的复杂性，提供统一、简单的开发接口。
- **根治痛点**: 内建机制解决 `utoipa` 在处理递归结构时的编译崩溃问题。

---

## 2. 核心功能 (Core Features)

`service_kit` 主要由以下三个核心组件构成：

### 2.1. `#[ApiDto]` 过程宏
这是 `service_kit` 的灵魂。它是一个过程宏 (Procedural Macro)，旨在取代开发中常用的多个 `derive` 宏。开发者只需在数据传输对象（DTO）结构体上添加 `#[ApiDto]`，即可自动获得：
- `serde` 的序列化/反序列化能力 (`Serialize`, `Deserialize`)。
- `utoipa` 的 OpenAPI Schema 生成能力 (`ToSchema`)。
- `ts-rs` 的 TypeScript 类型定义生成能力 (`TS`)。
- 常用的调试和克隆能力 (`Debug`, `Clone`)。
- **内置的递归问题解决方案**。
- **灵活的属性定制与透传能力**。

### 2.2. `forge_cli` 集成构建工具
这是一个通过 `xtask` 模式实现的命令行工具，封装了微服务开发、测试和构建的完整流程。它提供了一系列简单的 `cargo forge` 命令，作为标准 `cargo` 命令的替代和增强。

**主要命令**:
- `cargo forge test`: 运行单元测试，并自动执行 API 契约验证。
- `cargo forge generate-ts`: 手动为项目中的所有 DTO 生成 TypeScript 类型定义。
- `cargo forge lint`: 执行静态代码检查，包括强制的强类型约束检查。

### 2.3. `cargo-generator` 服务模板
这是一个标准的 `cargo-generator` 模板，允许开发者通过一条命令快速初始化一个全新的、符合 `service_kit` 规范的微服务项目骨架。

---

## 3. `#[ApiDto]` 宏详解

`#[ApiDto]` 宏是为解决 DTO 定义过程中的重复性工作和常见陷阱而设计的。

### 3.1. 使用示例

**开发者只需编写**:
```rust
use service_kit::prelude::*;

#[ApiDto]
pub struct UserProfile {
    id: String,
    name: String,
    // 一个递归字段
    manager: Option<Box<UserProfile>>,
    // 一个包含其他 DTO 的字段
    team: Team,
}

#[ApiDto]
pub struct Team {
    id: String,
    team_name: String,
}
```

**宏在编译时会自动展开为 (部分)**:
```rust
// 宏展开后的代码，对开发者透明
use serde::{Serialize, Deserialize};
use utoipa::ToSchema;
use ts_rs::TS;

// --- UserProfile ---
#[derive(Serialize, Deserialize, ToSchema, TS, Debug, Clone)]
#[serde(rename_all = "camelCase")] // <-- 默认命名策略
#[ts(export, export_to = "generated/ts/")] // <-- 默认输出路径
pub struct UserProfile {
    id: String,
    name: String,
    #[schema(value_type = Object)] // <-- 自动注入，解决递归！
    manager: Option<Box<UserProfile>>,
    team: Team,
}
// ... Team 的展开类似 ...
```

### 3.2. 核心优势与定制化 (Core Advantages & Customization)

1.  **消除样板代码**: 自动派生所有必要的 trait。

2.  **统一规范**:
    - 默认强制使用 `camelCase` 的 JSON 命名策略，确保 API 风格一致。
    - 默认将生成的 TypeScript 类型输出到固定的目录 (`generated/ts/`)。

3.  **解决递归引用崩溃问题 (核心)**:
    - **原理**: `ApiDto` 宏的实现会使用 `syn` crate 解析结构体的语法树，当检测到字段类型是对结构体自身的递归引用时（如 `Box<Self>`），宏会自动为该字段注入 `#[schema(value_type = Object)]` 属性，从根本上避免了 `utoipa` 展开时的编译器崩溃。

4.  **灵活的定制与扩展 (Attribute Customization & Passthrough)**:
    为了在保持规范的同时提供必要的灵活性，`#[ApiDto]` 宏支持丰富的定制能力。

    -   **覆盖命名策略**: 允许通过宏参数覆盖默认的 `camelCase` 策略。
        ```rust
        #[ApiDto(rename_all = "snake_case")]
        pub struct LegacySystemData {
            user_id: String,
        }
        ```
    -   **属性透传**: 宏会智能地保留开发者添加的其他 `derive` 宏和属性，确保与生态中其他工具（如 `async_trait` 等）的兼容性，并允许进行精细控制。
        ```rust
        use service_kit::prelude::*;
        
        #[ApiDto]
        #[derive(PartialEq, Eq, Hash)] // <-- 其他 derive 宏会被保留
        pub struct AdvancedConfig {
            id: String,
            #[serde(skip_serializing_if = "Option::is_none")] // <-- 字段级属性会被保留
            optional_setting: Option<String>,
            
            #[serde(skip)] // <-- 完全跳过某个字段
            internal_state: u64,
        }
        ```

5.  **全局配置 (Global Configuration)**:
    为了避免在每个 DTO 中重复配置，`service_kit` 支持在项目的 `Cargo.toml` 中进行全局配置。
    
    ```toml
    # 在 Cargo.toml 中
    [package.metadata.service_kit]
    ts_output_dir = "frontend/src/generated/types/"
    ```
    
    当存在此配置时，`#[ApiDto]` 宏会自动将 `#[ts(export_to = "...")]` 的路径替换为配置的值。

---

## 4. `ForgeCli` 工具链

`forge_cli` 是一个 `xtask`，它为开发者提供了一套更高层、更智能的命令行接口。

### 4.1. 命令详解

-   **`cargo forge test`**
    -   **功能**: 在 CI/CD 环境中，这是保证 API 质量的核心命令。
    -   **执行流程**:
        1.  执行 `cargo test` 运行所有单元和集成测试。
        2.  启动一个临时的 `axum` 服务器实例（不监听端口）。
        3.  调用 `utoipa::openapi::OpenApi::verify(...)`，检查所有 Axum 路由是否与 OpenAPI 声明完全匹配。若不匹配，则测试失败。

-   **`cargo forge lint`**
    -   **功能**: 执行静态代码质量检查，捍卫项目的类型安全底线。
    -   **执行流程**:
        1.  运行 `cargo clippy`。
        2.  运行自定义的静态分析脚本，该脚本会扫描所有被 `#[ApiDto]` 标记的结构体，并检查是否存在类型为 `serde_json::Value` 的字段。
        3.  **核心设计哲学**: 如果存在 `serde_json::Value` 字段，lint 将**无条件失败**。`service_kit` 的核心使命之一就是提供 100% 的端到端类型安全，杜绝动态类型带来的潜在运行时错误和维护噩梦。我们认为，任何需要动态 JSON 的场景都应通过更明确的结构体或枚举来建模。**此规则不存在豁免机制**。

-   **`cargo forge generate-ts`**
    -   **功能**: 手动触发一次 TypeScript 类型的生成。
    -   **执行流程**: 该命令会扫描整个项目，找到所有 `#[ApiDto]` 结构体，并调用 `ts-rs` 的逻辑，将它们导出到配置的目录。

---

## 5. 开发者工作流 (Developer Workflow)

设想一位开发者要创建一个新的 `products` 微服务。

1.  **初始化项目**:
    ```bash
    cargo generate --git <template_repo_url> --name product-service
    cd product-service
    ```
2.  **定义 DTO**: 在 `src/dtos.rs` 中定义产品相关的结构体。
    ```rust
    use service_kit::prelude::*;

    #[ApiDto]
    pub struct Product {
        id: String,
        name: String,
        price: f64,
    }
    ```
3.  **定义 API Handler**: 在 `src/handlers.rs` 中编写业务逻辑。
    ```rust
    use service_kit::prelude::*;
    // ...

    #[utoipa::path(
        get,
        path = "/v1/products/{id}",
        responses((status = 200, body = Product))
    )]
    pub async fn get_product(Path(id): Path<String>) -> Json<Product> {
        // ... 业务逻辑 ...
    }
    ```
4.  **注册路由**: 在 `main.rs` 中将 handler 添加到 `axum` 路由。

5.  **验证与生成**:
    - 运行 `cargo forge test`。CI 会自动验证路由是否正确注册，以及 API 文档和实现是否一致。
    - 运行 `cargo forge generate-ts`。`generated/ts/index.ts` 中会自动出现 `Product` 的 TypeScript interface。

6.  **提交代码**: 开发者可以满怀信心地提交代码，因为核心的 API 质量和类型安全已经由 `service_kit` 自动保证。

---

## 6. 技术实现细节 (Technical Implementation)

`service_kit` Crate 本身的开发将依赖以下关键技术：

-   **过程宏**:
    -   `syn`: 用于将 Rust 代码解析成语法树。
    -   `quote`: 用于根据语法树生成新的 Rust 代码。
    -   `proc-macro2`: 提供了更稳定和功能更丰富的过程宏 API。
    -   **高质量错误提示**: 实现中将大量使用 `syn::Error::new_spanned`，将编译错误精确地关联到用户的代码位置，提供IDE友好的诊断信息，极大改善宏的使用体验。
-   **构建工具 (`xtask`)**:
    -   `xtask` 是一个放置在项目根目录 `xtask/` 中的独立 crate，它作为项目的自定义构建脚本和任务运行器。
-   **核心依赖**:
    -   `utoipa`: 用于 OpenAPI 生成和验证。
    -   `ts-rs`: 用于 TypeScript 类型生成。
    -   `serde`: 用于序列化和反序列化。
    -   `axum`: 作为我们推荐的 Web 框架。