# Service Kit 增强计划

**作者**: Gemini Pro (AI Assistant)
**日期**: 2024-08-08
**状态**: 草案

## 1. 概述

本文档旨在为 `service_kit` 提出一套功能增强计划，核心目标是进一步减少微服务开发中的样板代码，自动化 API 文档生成，并提升开发体验。计划的核心是引入一个新的过程宏 `#[api_route]` 和一个辅助结构体 `ApiDoc`。

## 2. 动机

当前版本的 `service_kit` 提供了 `#[api_dto]` 宏，它成功地简化了数据传输对象（DTO）的定义。然而，在路由定义和 OpenAPI Spec 生成方面，开发者仍然需要编写大量的重复代码。

以 `user-manager-service` 为例：
-   每个 Axum handler 都需要一个匹配的 `#[utoipa::path(...)]` 宏来描述其 API 信息。这导致了代码冗余，且容易出错。
-   OpenAPI Spec 的定义是手动在 `main.rs` 中通过 `#[derive(OpenApi)]` 完成的，需要手动列出所有的 DTOs 和 handlers。当项目规模扩大时，这变得难以维护。

本计划旨在解决这些痛点。

## 3. 设计方案

### 3.1. `#[api_route]` 过程宏

`#[api_route]` 是一个属性宏，它将被附加到 Axum 的 handler 函数上，以取代手动的 `#[utoipa::path]`。

#### 3.1.1. 核心功能

1.  **方法和路径推断**:
    -   宏将能够通过解析 Axum handler 的函数名或宏参数来推断 HTTP 方法 (e.g., `get_user` -> GET, `create_user` -> POST)。
    -   宏将提供一个明确的参数来设置 API 路径，例如 `#[api_route(POST, "/users")]`。

2.  **请求体 (Request Body) 自动检测**:
    -   通过分析函数参数，宏可以自动识别 `Json<T>` 类型的参数，并将其作为 `request_body` 添加到 OpenAPI 定义中。`T` 必须是一个被 `#[api_dto]` 标记的 DTO。

3.  **路径和查询参数 (Path & Query Params) 自动检测**:
    -   宏能够识别 `axum::extract::Path<T>` 和 `axum::extract::Query<T>`，并自动生成对应的 `params` 定义。

4.  **响应 (Responses) 自动检测**:
    -   通过分析函数的返回类型，宏可以推断出成功的响应。例如，返回 `(StatusCode, Json<User>)` 将被自动转换为 `(status = 200, body = User)`。
    -   为了处理多种响应情况（如错误），宏将提供一个简洁的 `responses` 参数，例如 `#[api_route(..., responses((status = 404, description = "User not found")))]`。

#### 3.1.2. 使用示例

**之前 (在 `user-manager-service`):**
```rust
#[utoipa::path(
    post,
    path = "/api/auth/login",
    request_body = LoginPayload,
    responses(
        (status = 200, description = "Login successful", body = LoginResponse),
        (status = 401, description = "Invalid username or password"),
    )
)]
pub async fn login_handler(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<LoginPayload>,
) -> impl IntoResponse {
    // ...
}
```

**之后 (使用 `#[api_route]`):**
```rust
// #[api_route] 将自动从 Json<LoginPayload> 推断出 request_body
// 和从返回类型 -> impl IntoResponse (需要进一步解析) 推断出成功响应
// 错误响应仍然需要手动定义
#[api_route(
    POST, "/api/auth/login",
    responses(
        (status = 401, description = "Invalid username or password"),
    )
)]
pub async fn login_handler(
    State(state): State<Arc<AppState>>,
    payload: Json<LoginPayload>, // axum 0.7+ 的推荐写法
) -> (StatusCode, Json<LoginResponse>) { // 返回具体的类型
    // ...
}
```

### 3.2. `ApiDoc` 结构体

`ApiDoc` 是一个辅助结构体，旨在简化 `main.rs` 中 `utoipa::OpenApi` 的定义。

#### 3.2.1. 核心功能

1.  **自动发现**: `ApiDoc` 将被设计为能够自动扫描整个 crate，并发现所有被 `#[api_route]` 标记的函数和被 `#[api_dto]` 标记的 DTO。
2.  **动态构建**: 它会在编译时动态地构建 `utoipa::OpenApi` 所需的 `paths` 和 `components` 列表。
3.  **简化配置**: 开发者将不再需要手动维护一个长长的 DTO 和 handler 列表。

#### 3.2.2. 使用示例

**之前 (在 `user-manager-service` 的 `lib.rs`):**
```rust
#[derive(OpenApi)]
#[openapi(
    paths(
        handlers::register_user_handler,
        handlers::login_handler,
        // ... (省略大量其他 handlers) ...
    ),
    components(
        schemas(
            CreateUserPayload,
            User,
            // ... (省略大量其他 DTOs) ...
        )
    ),
    tags(
        (name = "user-manager-service", description = "User management endpoints.")
    )
)]
pub struct ApiDoc;
```

**之后 (使用 `ApiDoc` from `service_kit`):**
```rust
// service_kit 将提供一个宏来生成这个
#[derive(OpenApi)]
#[openapi(
    // ... openapi的元数据 ...
)]
pub struct ApiDoc; 

// 在 main.rs 中
// service_kit::build_api_doc() 会在编译时扫描项目，
// 找到所有相关的 DTOs 和 handlers，并填充到 ApiDoc 中。
let api_doc = service_kit::build_api_doc!();
```
*注意: `build_api_doc!` 的确切实现机制需要进一步研究，可能需要结合 build scripts 或更高级的宏技巧。一个更简单（但仍然有效）的初始版本可能是提供一个 `ApiDocBuilder`。*

```rust
// 备选方案：ApiDocBuilder
let api_doc = service_kit::ApiDocBuilder::new()
    .title("User Manager Service")
    .version("1.0.0")
    .discover() // 自动发现 #[api_route] 和 #[api_dto]
    .build();
```

## 4. 实施路线图

1.  **阶段一: 实现 `#[api_route]` (基础版)**
    -   实现对 `(method, path)` 的解析。
    -   实现将宏参数透传给 `utoipa::path`。
    -   在 `user-manager-service` 中进行初步验证（预期会编译失败，但宏能展开）。

2.  **阶段二: 增强 `#[api_route]`**
    -   添加对 `Json<T>` 的自动检测。
    -   添加对 `Path<T>` 和 `Query<T>` 的自动检测。
    -   研究如何可靠地从返回类型推断响应。

3.  **阶段三: 实现 `ApiDoc` Builder**
    -   设计并实现 `ApiDocBuilder`。
    -   编写一个 build script 或宏来扫描项目文件，收集路由和 schema 信息。

## 5. 预期收益

-   **大幅减少样板代码**: 开发者可以专注于业务逻辑，而不是 API 文档的格式。
-   **提升代码可读性**: 路由定义将更紧凑、更清晰。
-   **降低维护成本**: 新增或修改 API 时，不再需要在多个地方同步信息。
-   **加强规范**: `service_kit` 将引导开发者遵循一致的 API 设计模式。

