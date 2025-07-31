use axum::{routing::get, Router};
use tokio::net::TcpListener;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

mod dtos;
mod handlers;

#[tokio::main]
async fn main() {
    #[derive(OpenApi)]
    #[openapi(
        paths(
            handlers::get_product,
        ),
        components(
            schemas(dtos::Product, dtos::Category)
        ),
        tags(
            (name = "product-service", description = "Product Service API")
        ),
        servers(
            (url = "/api", description = "Local server")
        )
    )]
    struct ApiDoc;

    let app = Router::new()
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .route("/v1/products/:id", get(handlers::get_product));

    println!("🚀 Server running at http://127.0.0.1:3000");
    println!("📚 Swagger UI available at http://127.0.0.1:3000/swagger-ui");

    let listener = TcpListener::bind("127.0.0.1:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

