use axum::{routing::get, Router};
use std::env;
use tokio::net::TcpListener;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

pub mod dtos;
pub mod handlers;

/// The main OpenAPI documentation structure for the service.
#[derive(OpenApi)]
#[openapi(
    paths(
        handlers::hello,
    ),
    components(
        schemas(dtos::Greeting)
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
    Router::new()
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .route("/v1/hello", get(handlers::hello))
}

/// Starts the web server.
pub async fn run_server() {
    // Load environment variables from .env file, if it exists.
    dotenvy::dotenv().ok();
    
    let app = api_router();

    // Get the port from the environment or default to 3000.
    let port = env::var("PORT").unwrap_or_else(|_| "3000".to_string());
    let address = format!("127.0.0.1:{}", port);

    println!("🚀 Server running at http://{}", address);
    println!("📚 Swagger UI available at http://{}/swagger-ui", address);

    let listener = TcpListener::bind(&address).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
