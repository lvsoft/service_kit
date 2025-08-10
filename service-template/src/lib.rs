use axum::Router;
use service_kit::{rest_router_builder::RestRouterBuilder};
use rmcp::transport::streamable_http_server::{session::local::LocalSessionManager, StreamableHttpService};
use rust_embed::RustEmbed;
use tower_http::cors::{Any, CorsLayer};
use utoipa::openapi::OpenApi;
#[cfg(feature = "swagger-ui")]
use utoipa_swagger_ui::SwaggerUi;
#[cfg(feature = "wasm-cli")]
use axum_embed::ServeEmbed;


pub mod dtos;
pub mod handlers;
pub mod mcp_server;

#[cfg(feature = "wasm-cli")]
#[derive(RustEmbed, Clone)]
#[folder = "assets/"]
struct Assets;

pub fn build_openapi_spec() -> OpenApi {
    // Keep this in sync with the template's package version for clarity.
    service_kit::openapi_utils::build_openapi_basic("{{project-name}}", env!("CARGO_PKG_VERSION"), "{{project-name}} API", "App")
}

/// 仅注册 handlers，让 inventory 完整。
pub fn load() { handlers::load(); }

/// 构建 REST 路由（不启动服务，不绑定端口）。
pub fn build_rest_router(openapi: OpenApi) -> service_kit::error::Result<Router> {
    RestRouterBuilder::new().openapi(openapi).build()
}

/// 构建 Swagger UI（用户自行 merge 到 app）。
#[cfg(feature = "swagger-ui")]
pub fn build_swagger_ui(openapi: OpenApi) -> SwaggerUi {
    SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", openapi)
}

/// 构建 CLI WASM 资源路由（/cli-ui）。
#[cfg(feature = "wasm-cli")]
pub fn build_cli_assets_router() -> Router {
    Router::new().nest_service("/cli-ui", ServeEmbed::<Assets>::new())
}

/// 构建一个常用的 CORS Layer（可选）。
pub fn default_cors_layer() -> CorsLayer {
    CorsLayer::new().allow_origin(Any).allow_methods(Any).allow_headers(Any)
}

/// 构建 MCP Tool 服务（需启用 mcp 特性）。
#[cfg(feature = "mcp")]
pub fn build_mcp_service(openapi: OpenApi) -> service_kit::error::Result<StreamableHttpService<mcp_server::McpServerImpl>> {
    let mcp_tool_router = service_kit::bootstrap::mcp_router_from_openapi::<mcp_server::McpServerImpl>(openapi)?;
    let mcp_server = mcp_server::McpServerImpl::new(mcp_tool_router);
    let svc = StreamableHttpService::new(
        move || Ok(mcp_server.clone()),
        LocalSessionManager::default().into(),
        Default::default(),
    );
    Ok(svc)
}
