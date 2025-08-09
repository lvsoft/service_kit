use axum::extract::{FromRequestParts, Path};
use axum::response::IntoResponse;
use axum::{routing::any, Router, extract::Request};
use forge_core::handler::get_api_handlers;
use forge_core::openapi_to_mcp::OpenApiMcpRouterBuilder;
use rmcp::transport::streamable_http_server::{
    session::local::LocalSessionManager, StreamableHttpService,
};
use rust_embed::RustEmbed;
use tower_http::cors::{Any, CorsLayer};
use utoipa::openapi::OpenApi;
use utoipa_swagger_ui::SwaggerUi;
use axum_embed::ServeEmbed;
use axum::body::Body;
use utoipa::openapi::schema::{ObjectBuilder, SchemaType, Type};
use utoipa::openapi::path::HttpMethod;

mod dtos;
mod handlers;
mod mcp_server;

#[derive(RustEmbed, Clone)]
#[folder = "../../service_kit/frontend-wasm-cli/"]
struct Assets;

/// The main builder for the entire API service.
/// It dynamically constructs the REST router, MCP router, and OpenAPI specification.
pub struct ApiRouterBuilder {
    openapi: OpenApi,
}

impl ApiRouterBuilder {
    /// Creates a new builder, initializing it with a base OpenAPI document.
    pub fn new() -> Self {
        // Create a base OpenAPI doc. In a real scenario, this could come from a file.
        let mut openapi = OpenApi::default();
        openapi.info = utoipa::openapi::InfoBuilder::new()
            .title("Product Service API")
            .version("0.1.0")
            .build();
        openapi.servers = Some(vec![utoipa::openapi::ServerBuilder::new()
            .url("/api")
            .build()]);
        Self { openapi }
    }

    /// Discovers all `#[api]` annotated handlers and builds the final `Router`.
    pub fn discover_apis(mut self) -> Result<Router, forge_core::error::Error> {
        println!("[discover_apis] Starting discovery...");
        let mut rest_router = Router::new();

        {
            let handlers = get_api_handlers();
            let handlers_lock = handlers.lock().expect("Failed to lock handlers");
            println!(
                "[discover_apis] Locked handlers, found {} handlers.",
                handlers_lock.len()
            );

            // This loop is the heart of the dynamic router.
            for (_operation_id, handler_info) in handlers_lock.iter() {
                println!(
                    "[discover_apis] Processing handler: {}",
                    handler_info.operation_id
                );
                // 1. Build REST route
                let rest_handler = handler_info.handler.clone();
                let route_handler = move |req: Request<Body>| async move {
                    let (mut parts, _body) = req.into_parts();
                    let path_params: Path<String> =
                        Path::from_request_parts(&mut parts, &()).await.unwrap();
                    let params = serde_json::json!({ "id": path_params.0 });
                    rest_handler(&params)
                        .await
                        .unwrap_or_else(|e| e.into_response())
                };

                // This is still manual, a real implementation would get this from the handler_info
                let path = "/v1/products/{id}";
                rest_router = rest_router.route(path, any(route_handler));

                // In a full implementation, the openapi PathItem would be built here
                // and added to `self.openapi`.
                let mut operation = utoipa::openapi::path::OperationBuilder::new()
                    .operation_id(Some(handler_info.operation_id.to_string()))
                    .description(Some("A dynamically added operation"))
                    .build();
                let param = utoipa::openapi::path::ParameterBuilder::new()
                    .name("id")
                    .parameter_in(utoipa::openapi::path::ParameterIn::Path)
                    .required(utoipa::openapi::Required::True)
                    .schema(Some(utoipa::openapi::RefOr::T(utoipa::openapi::Schema::from(
                        ObjectBuilder::new().schema_type(SchemaType::Type(Type::String)),
                    ))))
                    .build();
                operation.parameters = Some(vec![param.into()]);

                let path_item = utoipa::openapi::path::PathItemBuilder::new()
                    .operation(HttpMethod::Get, operation)
                    .build();

                self.openapi.paths.paths.insert(path.to_string(), path_item);
            }
        }
        println!("[discover_apis] Finished processing handlers.");
        
        // This part remains the same: build MCP router from the spec.
        println!("[discover_apis] Building MCP tool router...");
        let mcp_tool_router = OpenApiMcpRouterBuilder::new()
            .openapi(self.openapi.clone()) // Pass a clone of the spec
            .build()?;
        println!("[discover_apis] MCP tool router built successfully.");

        let mcp_server = mcp_server::McpServerImpl::new(mcp_tool_router);
        println!("[discover_apis] Creating MCP service...");
        let mcp_service = StreamableHttpService::new(
            move || Ok(mcp_server.clone()),
            LocalSessionManager::default().into(),
            Default::default(),
        );
        println!("[discover_apis] MCP service created.");

        let assets_router = Router::new().nest_service("/cli-ui", ServeEmbed::<Assets>::new());
        let swagger_ui = SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", self.openapi);

        let final_router = Router::new()
            .merge(rest_router)
            .merge(swagger_ui)
            .nest_service("/mcp", mcp_service)
            .merge(assets_router)
            .layer(
                CorsLayer::new()
                    .allow_origin(Any)
                    .allow_methods(Any)
                    .allow_headers(Any),
            );

        println!("[discover_apis] Discovery finished, returning router.");
        Ok(final_router)
    }
}


#[tokio::main]
async fn main() {
    let app = ApiRouterBuilder::new()
        .discover_apis()
        .expect("Failed to build API router");

    println!("🚀 Server running at http://127.0.0.1:3000");
    println!("📚 Swagger UI available at http://127.0.0.1:3000/swagger-ui");
    println!("💻 Forge CLI UI available at http://127.0.0.1:3000/cli-ui");

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();
    axum::serve(listener, app).await.unwrap();
}
