use axum::{routing::get, Router};
use tokio::net::TcpListener;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

mod dtos;
mod handlers;

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
struct ApiDoc;


#[tokio::main]
async fn main() {
    let app = api_router();

    println!("🚀 Server running at http://127.0.0.1:3000");
    println!("📚 Swagger UI available at http://127.0.0.1:3000/swagger-ui");

    let listener = TcpListener::bind("127.0.0.1:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

/// Constructs the main Axum router for the application.
pub fn api_router() -> Router {
    Router::new()
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .route("/v1/hello", get(handlers::hello))
}
