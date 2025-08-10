use crate::{openapi_utils, rest_router_builder::RestRouterBuilder};
use utoipa::openapi::OpenApi;

/// 从 inventory 元数据构建 OpenAPI 文档
pub fn build_openapi_from_inventory(title: &str, version: &str, description: &str, tag: &str) -> OpenApi {
    openapi_utils::build_openapi_basic(title, version, description, tag)
}

/// 从 inventory 元数据直接构建 REST Router
pub fn rest_router_from_inventory(title: &str, version: &str, description: &str, tag: &str) -> crate::error::Result<axum::Router> {
    let openapi = build_openapi_from_inventory(title, version, description, tag);
    RestRouterBuilder::new().openapi(openapi).build()
}

/// 从已有的 OpenAPI 文档构建 REST Router
pub fn rest_router_from_openapi(openapi: OpenApi) -> crate::error::Result<axum::Router> {
    RestRouterBuilder::new().openapi(openapi).build()
}

#[cfg(all(not(target_arch = "wasm32"), feature = "mcp"))]
/// 从 inventory 元数据直接构建 MCP ToolRouter
pub fn mcp_router_from_inventory<S: Send + Sync + 'static>(title: &str, version: &str, description: &str, tag: &str) -> crate::error::Result<rmcp::handler::server::router::tool::ToolRouter<S>> {
    let openapi = build_openapi_from_inventory(title, version, description, tag);
    crate::openapi_to_mcp::OpenApiMcpRouterBuilder::new().openapi(openapi).build()
}

#[cfg(all(not(target_arch = "wasm32"), feature = "mcp"))]
/// 从已有的 OpenAPI 文档构建 MCP ToolRouter
pub fn mcp_router_from_openapi<S: Send + Sync + 'static>(openapi: OpenApi) -> crate::error::Result<rmcp::handler::server::router::tool::ToolRouter<S>> {
    crate::openapi_to_mcp::OpenApiMcpRouterBuilder::new().openapi(openapi).build()
}


