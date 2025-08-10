use axum::Router;
use service_kit::{
    ApiDtoMetadata, ApiMetadata, inventory, utoipa::{self, OpenApi},
};
use rmcp::transport::streamable_http_server::{
    session::local::LocalSessionManager, StreamableHttpService,
};
use rust_embed::RustEmbed;
use std::{collections::HashMap, sync::Arc};
use tower_http::cors::{Any, CorsLayer};
use utoipa::openapi::{self};
use utoipa_swagger_ui::SwaggerUi;
use axum_embed::ServeEmbed;

// We need to bring the handlers module into scope for the linker to pick up the inventory registrations.
// use crate::handlers; // This is now incorrect, we use the library.

// mod dtos; // No longer needed, it's in lib.rs
// mod handlers; // No longer needed, it's in lib.rs
mod mcp_server;

#[derive(RustEmbed, Clone)]
#[folder = "../../service_kit/frontend-wasm-cli/"]
struct Assets;

fn build_openapi_spec() -> utoipa::openapi::OpenApi {
    service_kit::openapi_utils::build_openapi_basic("Product Service API", env!("CARGO_PKG_VERSION"), "All endpoints for the product service.", "App")
}

#[tokio::main]
async fn main() {
    // This function call is a trick to ensure the linker
    // doesn't optimize away the handlers module, which contains
    // the inventory::submit! calls for our API endpoints.
    product_service::handlers::load();
    
    let openapi = Arc::new(build_openapi_spec());

    // Utility: print OpenAPI and exit if env is set
    if std::env::var("PRINT_OPENAPI").is_ok() {
        println!("{}", openapi.to_pretty_json().unwrap_or_else(|_| "{}".to_string()));
        return;
    }

    // --- Build REST Router ---
    let rest_router = service_kit::rest_router_builder::RestRouterBuilder::new()
        .openapi((*openapi).clone())
        .build()
        .expect("Failed to build REST router");

    // --- Build MCP Router ---
    let mcp_tool_router = service_kit::openapi_to_mcp::OpenApiMcpRouterBuilder::new()
        .openapi((*openapi).clone())
        .build()
        .expect("Failed to build MCP router");
    
    let mcp_server = mcp_server::McpServerImpl::new(mcp_tool_router);
    let mcp_service = StreamableHttpService::new(
        move || Ok(mcp_server.clone()),
        LocalSessionManager::default().into(),
        Default::default(),
    );

    // --- Combine all routers ---
    let assets_router = Router::new().nest_service("/cli-ui", ServeEmbed::<Assets>::new());
    let swagger_ui = SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", (*openapi).clone());

    let app = rest_router
        .merge(swagger_ui)
        .nest_service("/mcp", mcp_service)
        .merge(assets_router)
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        );

    println!("🚀 Server running at http://127.0.0.1:3000");
    println!("📚 Swagger UI available at http://127.0.0.1:3000/swagger-ui");
    println!("💻 Forge CLI UI available at http://127.0.0.1:3000/cli-ui");

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();
    axum::serve(listener, app).await.unwrap();
}
