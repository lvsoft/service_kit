# Service Kit 架构重构最终方案：编译时收集，运行时构建

**作者**: Gemini Pro (AI Assistant)
**日期**: 2025-08-09
**状态**: **最终方案** (基于 2025-08-08 设计的演进)

## 1. 背景：回顾初心与遭遇的困境

此前的 `20250808_mcp_integration_design.md` 设计方案确立了一个优雅的核心目标：将 `openapi.json` 作为驱动所有 API 生成的唯一服务契约 (Single Source of Truth)，并通过一个 `#[api]` 宏自动完成所有繁杂的工作。

然而，在实施过程中，我们遭遇了一个由 Rust 过程宏（procedural macro）系统本身引发的、难以调和的根本性技术冲突。

### 1.1. 核心冲突点

-   **Rust 宏规则**: 作用于函数的属性宏（`#[proc_macro_attribute]`）必须返回一个有效的、单一的函数项来**替换**原始函数。它不能同时返回一个函数和一个额外的 `struct`。
-   **`utoipa` 的 `derive` 机制**: `#[derive(OpenApi)]` 宏通过 `paths(...)` 发现 API 时，它并不是直接解析函数，而是在对应的模块中寻找一个由 `#[utoipa::path]` 宏生成的、名为 `__path_函数名` 的特殊**结构体**。

这两点构成了无法绕过的矛盾：`#[api]` 宏如果想保留原始函数以供路由和 `ctor` 注册使用，就不能把自己变成一个 `struct`；如果它想生成一个 `struct` 以便被 `utoipa` 的 `derive` 宏发现，原始的函数就会被替换掉，导致编译失败。我们之前反复的编译错误（`expected fn`）正是这个根本矛盾的直接体现。

## 2. 新架构：编译时收集，运行时构建

为了突破这一困境，我们决定转变思路，不再试图让我们的宏去“欺骗”或适配 `utoipa` 的 `derive` 宏，而是采用一个更清晰、更可控的架构。

**核心理念：**
1.  **编译时**：`#[api]` 宏的职责被进一步简化，它只负责**收集信息**。它会解析函数及其元数据，并将这些信息注册到一个全局的、静态的清单中。完成注册后，它将**原封不动地返回原始函数**。
2.  **运行时**：我们不再使用 `#[derive(OpenApi)]`。取而代之，我们将编写一个普通的函数（例如 `build_openapi_spec()`），它在程序启动时被调用。此函数会遍历编译时收集到的全局 API 清单，并使用 `utoipa` 提供的**构建器 API**（`PathItemBuilder`, `OperationBuilder`等）来**动态地、程序化地**创建出完整的 `OpenApi` 对象。

### 架构流程：

```mermaid
graph TD
    A[开发者编写的业务逻辑函数<br>+ `#[api]` 宏] -->|在编译时| B(inventory::submit);
    
    subgraph 编译时
        B --> C{全局 API 元数据清单<br>(由 `inventory` 管理)};
    end

    subgraph 运行时
        D[主程序 `main.rs`] --> E{调用 build_openapi_spec()};
        E -->|读取| C;
        E --> F[使用 `utoipa` 的 Builder API<br>循环构建 OpenAPI 对象];
        F --> G[动态生成的 `openapi.json`<br><b>(Single Source of Truth)</b>];
        
        G --> H[OpenAPI-to-MCP Generator];
        H --> I[动态生成的 MCP Server];

        G --> J[RestRouterBuilder];
        J --> K[动态生成的 REST Router];

        G --> L[SwaggerUI];
    end

    style G fill:#f9f,stroke:#333,stroke-width:2px
```

## 3. 实施细节

### 3.1. `#[api]` 宏与 `inventory`

`#[api]` 宏将是新架构的基石。

```rust
// In service-kit-macros/src/lib.rs

// 1. 定义一个结构体来承载所有 API 元数据
pub struct ApiMetadata {
    pub operation_id: &'static str,
    pub method: &'static str,
    pub path: &'static str,
    pub summary: &'static str,
    pub description: &'static str,
    // ... 其他需要的信息，如参数schema, 返回值schema等
    // 这些 schema 可以序列化为字符串存储
}

// 2. 使用 inventory 宏来定义一个可供提交的类型
inventory::collect!(ApiMetadata);

// 3. #[api] 宏的实现
#[proc_macro_attribute]
pub fn api(args: TokenStream, input: TokenStream) -> TokenStream {
    let item_fn = parse_macro_input!(input as ItemFn);
    let fn_name = item_fn.sig.ident.to_string();
    
    // ... 解析 args, 文档注释, 函数签名等 ...
    
    // 在编译时提交元数据到全局清单
    let registration = quote! {
        inventory::submit! {
            ApiMetadata {
                operation_id: #fn_name,
                // ... 填充所有其他解析出来的数据 ...
            }
        }
    };

    // 将注册代码和原始函数一起返回
    let output = quote! {
        #registration
        #item_fn
    };

    output.into()
}
```
**关键点**：
- 我们将引入 `inventory` crate。
- `#[api]` 宏在解析完所有需要的信息后，通过 `inventory::submit!` 将其注册。
- 最重要的是，宏的输出**包含了原始的函数 `item_fn`**，确保了函数本身依然可用。

### 3.2. `build_openapi_spec()` 函数

这个新函数将位于 `main.rs` 中，负责在运行时构建 `OpenApi` 对象。

```rust
// In examples/product-service/src/main.rs

fn build_openapi_spec() -> utoipa::OpenApi {
    let mut openapi = utoipa::OpenApi::default();
    // ... 初始化 openapi.info, openapi.servers ...

    // 遍历由 inventory 在编译时收集的所有 ApiMetadata
    for metadata in inventory::iter::<ApiMetadata> {
        // 使用 utoipa builder API 来构建 PathItem 和 Operation
        let operation = utoipa::openapi::path::OperationBuilder::new()
            .operation_id(Some(metadata.operation_id.to_string()))
            .summary(Some(metadata.summary.to_string()))
            // ... 根据 metadata 构建 parameters, request_body, responses ...
            .build();
            
        let path_item = utoipa::openapi::path::PathItemBuilder::new()
            .operation(metadata.method.parse().unwrap(), operation)
            .build();

        // 将构建好的 path_item 添加到 openapi 对象中
        openapi.paths.paths.insert(metadata.path.to_string(), path_item);
    }
    
    // ... 还需要添加 DTOs 的 schema 到 components ...
    // let components = utoipa::openapi::ComponentsBuilder::new()
    //     .schemas_from_iter( ... )
    //     .build();
    // openapi.components = Some(components);
    
    openapi
}

// 在 main 函数中调用
#[tokio::main]
async fn main() {
    let openapi = build_openapi_spec();
    
    // 后续流程与之前一致，将 openapi 对象传递给
    // RestRouterBuilder, OpenApiMcpRouterBuilder, 和 SwaggerUi
    // ...
}
```

## 4. 实施路线图

1.  **Phase 1: 依赖与元数据结构**
    -   [ ] **(Task 1)** 将 `inventory` 添加到 `service-kit-macros` 的依赖中。
    -   [ ] **(Task 2)** 定义 `ApiMetadata` 结构体，并确定需要从 `#[api]` 宏中提取的所有字段（路径、方法、参数、返回值、文档等）。

2.  **Phase 2: 重构 `#[api]` 宏**
    -   [ ] **(Task 3)** 完全重写 `#[api]` 宏的逻辑，使其专注于解析元数据并通过 `inventory::submit!` 进行注册。
    -   [ ] **(Task 4)** 确保宏的输出包含了原始函数，不破坏现有路由和 `ctor` 注册逻辑。

3.  **Phase 3: 实现 `build_openapi_spec`**
    -   [ ] **(Task 5)** 在 `product-service/main.rs` 中移除 `#[derive(OpenApi)]`。
    -   [ ] **(Task 6)** 实现 `build_openapi_spec` 函数，使其能正确遍历 `inventory` 清单并使用 `utoipa` builder API 构建 `OpenApi` 对象。
    -   [ ] **(Task 7)** 解决 `components` (DTO schemas) 的注册问题，可能需要另一个 `inventory` 清单来收集所有 DTO 类型。

4.  **Phase 4: 端到端验证**
    -   [ ] **(Task 8)** 重新编译并运行 `product-service`。
    -   [ ] **(Task 9)** 验证 Swagger UI、REST API 和 MCP Server 是否全部按预期、基于动态生成的 `OpenApi` 规范正常工作。

## 5. 最终优势

这个新方案虽然改变了实现策略，但**完美地坚守并实现了最初的架构哲学**：
-   **绝对的单一事实来源**: Rust 函数和它的 `#[api]` 宏依然是驱动一切的唯一源头。
-   **彻底解耦**: 我们不再受制于 `utoipa` `derive` 宏的内部实现，拥有了100%的控制力。
-   **健壮且透明**: “编译时收集，运行时构建”的流程比复杂的宏交互更易于理解、调试和扩展。
-   **协议无关的业务逻辑**: handler 函数依然完全纯粹，不知道任何协议的存在。

这是一个在工程上更成熟、更可持续的终极方案。
