use axum::{routing::get, Router};
use tokio::net::TcpListener;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;
use rust_embed::RustEmbed;
use axum_embed::ServeEmbed;
use tower_http::cors::{CorsLayer, Any};
use rmcp::transport::streamable_http_server::{
    StreamableHttpService, session::local::LocalSessionManager,
};

mod dtos;
mod handlers;
mod mcp_server;

#[derive(RustEmbed, Clone)]
#[folder = "../../service_kit/frontend-wasm-cli/"]
struct Assets;


#[derive(OpenApi)]
#[openapi(
    paths(),
    components(
        schemas(dtos::Product, dtos::Category, dtos::LegacyData)
    ),
    tags(
        (name = "product-service", description = "Product Service API")
    ),
    servers(
        (url = "/api", description = "Local server")
    )
)]
struct ApiDoc;


#[tokio::main]
async fn main() {
    let app = api_router();

    println!("🚀 Server running at http://127.0.0.1:3000");
    println!("📚 Swagger UI available at http://127.0.0.1:3000/swagger-ui");
    println!("💻 Forge CLI UI available at http://127.0.0.1:3000/cli-ui");


    let listener = TcpListener::bind("127.0.0.1:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

/// Constructs the main Axum router for the application.
pub fn api_router() -> Router {
    let assets_router = Router::new().nest_service("/cli-ui", ServeEmbed::<Assets>::new());

    let mcp_service = StreamableHttpService::new(
        || Ok(mcp_server::McpServerImpl::new()),
        LocalSessionManager::default().into(),
        Default::default(),
    );

    Router::new()
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
        // .route("/v1/products/{id}", get(handlers::get_product))
        .nest_service("/mcp", mcp_service)
        .merge(assets_router)
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
}
