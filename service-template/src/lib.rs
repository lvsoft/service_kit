use axum::{routing::get, Router};
use std::env;
use tokio::net::TcpListener;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;
use rust_embed::RustEmbed;
use axum_embed::ServeEmbed;
use tower_http::cors::{CorsLayer, Any};


pub mod dtos;
pub mod handlers;

#[derive(RustEmbed, Clone)]
#[folder = "assets/"]
struct Assets;


/// The main OpenAPI documentation structure for the service.
#[derive(OpenApi)]
#[openapi(
    paths(
        handlers::hello,
        handlers::add
    ),
    components(
        schemas(dtos::Greeting, dtos::AddResponse, dtos::AddParams)
    ),
    tags(
        (name = "{{project-name}}", description = "{{project-name}} API")
    ),
    servers(
        (url = "/api", description = "Local server")
    )
)]
pub struct ApiDoc;

/// Constructs the main Axum router for the application.
pub fn api_router() -> Router {
    let assets_router = Router::new().nest_service("/cli-ui", ServeEmbed::<Assets>::new());
    
    // API routes with /api prefix to match OpenAPI server definition
    let api_routes = Router::new()
        .route("/v1/hello", get(handlers::hello))
        .route("/v1/add", get(handlers::add));

    Router::new()
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .nest("/api", api_routes)
        .merge(assets_router)
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
}

/// Starts the web server.
pub async fn run_server() {
    // Load environment variables from .env file, if it exists.
    dotenvy::dotenv().ok();
    
    let app = api_router();

    // Get the port from the environment or default to 3000.
    let port = env::var("PORT").unwrap_or_else(|_| "3000".to_string());
    // Get the host from the environment or default to 0.0.0.0 for container-friendly binding.
    let host = env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let address = format!("{}:{}", host, port);

    println!("🚀 Server running at http://{}", address);
    println!("📚 Swagger UI available at http://{}/swagger-ui", address);
    println!("💻 Forge CLI UI available at http://{}/cli-ui", address);


    let listener = TcpListener::bind(&address).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
