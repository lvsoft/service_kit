# Service Kit 统一 API 设计最终版：OpenAPI 即服务契约

**作者**: Gemini Pro (AI Assistant)
**日期**: 2025-08-08
**状态**: **最终版** (采纳 OpenAPI as Contract 思想)

## 1. 核心理念：OpenAPI 是唯一的服务契约

此前的设计方案虽然力求统一，但仍停留在“为不同协议生成不同代码”的层面。根据最新的讨论，我们采纳一个更强大、更优雅的架构理念：

**`openapi.json` 文件不再仅仅是文档产物，它将成为驱动所有 API 生成的、唯一的、权威的服务契约 (Single Source of Truth)。**

业务逻辑函数通过 `utoipa` 生成 OpenAPI 描述，而 `service_kit` 的职责是读取这份 OpenAPI 描述，并将其自动转换为其他协议的实现（如 MCP Server）。

### 架构流程：

```mermaid
graph TD
    A[开发者编写的业务逻辑函数<br>+ Rust Doc 注释] -->|使用 `#[api]` 宏| B(编译时);
    B --> C{utoipa};
    C --> D[生成的 `openapi.json`];
    
    subgraph service_kit 运行时
        E[OpenAPI-to-MCP Generator]
    end

    D --> |读取| E;
    
    subgraph 编译时注册 (The Bridge)
        B --> F[将 `operationId` 和函数指针<br>注册到全局清单];
    end
    
    F --> |运行时查询| E;

    E --> G[动态生成的 MCP Server];

    style D fill:#f9f,stroke:#333,stroke-width:2px
    style E fill:#ccf,stroke:#333,stroke-width:2px
```

## 2. `#[api]` 宏：专注、简单、强大

`#[api]` 宏的职责被简化到极致：**为 `utoipa` 生成最完美的 `#[utoipa::path]` 信息**。它不再需要任何 `rest(...)` 或 `mcp(...)` 参数。

```rust
use service_kit::api;
use axum::extract::{Query, Path};
use crate::dtos::{AddParams, AddResponse, Greeting};

/// Returns a simple greeting.
/// This is the detailed description for the endpoint.
#[api(GET, "/v1/hello")]
pub async fn hello() -> Json<Greeting> {
    // ... 业务逻辑 ...
}

/// Adds two numbers.
#[api(GET, "/v1/add")]
pub async fn add(Query(params): Query<AddParams>) -> Json<AddResponse> {
    // ... 业务逻辑 ...
}
```

### `#[api]` 宏的幕后工作:

1.  **生成 `utoipa::path`**:
    -   从函数的 `///` 文档注释中提取 `summary` 和 `description`。
    -   解析参数 (`Query<AddParams>`) 和返回值 (`Json<AddResponse>`)，生成 OpenAPI 的 `parameters` 和 `responses` 部分。
2.  **建立“桥梁”**:
    -   宏会根据函数名（如 `add`）生成一个唯一的 `operationId`。
    -   **关键步骤**: 使用 `inventory` crate，在编译时将这个 `operationId` 和函数自身的指针 (`&add`) 注册到一个全局的、对用户透明的清单中。这个清单是连接 OpenAPI 定义和实际 Rust 代码的桥梁。

## 3. `ApiRouterBuilder` 与 `OpenAPI-to-MCP Generator`

`ApiRouterBuilder` 内部包含一个全新的核心组件：`OpenAPI-to-MCP Generator`。当 `.discover_apis()` 被调用时，它执行以下操作：

1.  **加载 OpenAPI 契约**: 调用 `utoipa` 的 `ApiDoc::openapi()` 方法，在内存中获得完整的 `OpenAPI` 对象。
2.  **装配 REST 路由**: 遍历 `inventory` 清单，将 REST handlers 注册到 `axum` 路由中（此部分逻辑与之前类似）。
3.  **动态生成 MCP Server**:
    -   **遍历 `paths`**: 迭代 `OpenAPI` 对象中的所有 `paths`。
    -   **为每个 `operation` 生成 MCP Tool**:
        -   **Tool Name**: 使用 `operation.operation_id` 作为 MCP Tool 的名称。
        -   **Tool Description**: 使用 `operation.summary` 或 `operation.description`。
        -   **Input Schema**: 直接使用 `operation.parameters` 或 `operation.request_body` 中定义的 JSON Schema。`rmcp` 原生就支持 JSON Schema，无需转换。
        -   **Tool Implementation (调用桥梁)**:
            -   当 MCP Server 接收到一个 `call_tool` 请求（例如调用 `add` tool）...
            -   它会拿着 `add` 这个 `operationId` 去查询**编译时生成的全局函数指针清单**。
            -   从清单中找到匹配的函数指针 (`&add`)。
            -   **动态调用**: 将 MCP 请求中的参数反序列化后，调用该函数指针执行真正的业务逻辑。
            -   将函数的 `Result` 返回值适配成 MCP 的响应格式。

## 4. 实施路线图

1.  **Phase 1: 升级 `#[api]` 宏**
    -   [ ] **(Task 1)** 移除 `rest()` 和 `mcp()` 参数，简化宏接口。
    -   [ ] **(Task 2)** 增强 `utoipa` 信息生成，特别是从文档注释中提取 `summary` 和 `description`。
    -   [ ] **(Task 3)** 引入 `inventory` crate，实现 `operationId`到函数指针的编译时注册机制。

2.  **Phase 2: 实现 `OpenAPI-to-MCP` 生成器**
    -   [ ] **(Task 4)** 在 `ApiRouterBuilder` 中，实现加载和解析 `utoipa::OpenAPI` 对象的功能。
    -   [ ] **(Task 5)** 编写核心转换逻辑，将 `OpenAPI` 的 `PathItem` 映射为 `rmcp` 的 `Tool` 定义。
    -   [ ] **(Task 6)** 实现动态调用机制，通过查询 `inventory` 清单来执行对应的 Rust 函数。

3.  **Phase 3: 端到端验证**
    -   [ ] **(Task 7)** 使用新模式重构 `product-service` 示例。
    -   [ ] **(Task 8)** 编写测试，确保仅通过 `#[api]` 宏和文档注释，就能成功生成功能完备的 REST API, Swagger UI, 和 MCP Server。

## 5. 最终优势

-   **绝对的单一事实来源**: Rust 代码和它的文档注释是驱动一切的唯一源头。
-   **零信息冗余**: 开发者无需为不同协议写任何重复信息。
-   **协议无关的业务逻辑**: `hello` 和 `add` 函数完全不知道 MCP 的存在。
-   **自动一致性**: 由于 MCP Server 是 OpenAPI 的直接映射，所以两者之间永远不会出现不一致的情况。
-   **未来扩展性**: 将来如果想支持 gRPC 或 GraphQL，我们只需再写一个 `OpenAPI-to-gRPC` 的生成器即可，而业务代码无需任何改动。

这是一个真正具备工程美感的终极方案。它将使 `service_kit` 成为一个在架构上极其先进的微服务框架。
