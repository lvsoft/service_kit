use axum::Router;
use forge_core::{
    rest_router_builder::RestRouterBuilder,
    openapi_to_mcp::OpenApiMcpRouterBuilder,
    ApiDtoMetadata, ApiMetadata, inventory,
};
use rmcp::transport::streamable_http_server::{
    session::local::LocalSessionManager, StreamableHttpService,
};
use rust_embed::RustEmbed;
use std::{collections::HashMap, sync::Arc, env};
use tokio::net::TcpListener;
use tower_http::cors::{Any, CorsLayer};
use utoipa::openapi::{
    self, ComponentsBuilder, path::{OperationBuilder, PathItem, ParameterBuilder, ParameterIn}, Schema, Required,
};
use utoipa_swagger_ui::SwaggerUi;
use axum_embed::ServeEmbed;


pub mod dtos;
pub mod handlers;
pub mod mcp_server;

#[derive(RustEmbed, Clone)]
#[folder = "assets/"]
struct Assets;

fn build_openapi_spec() -> utoipa::openapi::OpenApi {
    let mut openapi = utoipa::openapi::OpenApiBuilder::new()
        .info(
            utoipa::openapi::InfoBuilder::new()
                .title("{{project-name}}")
                .version("0.1.0")
                .description(Some("{{project-name}} API"))
                .build(),
        )
        .paths(utoipa::openapi::Paths::new())
        .build();

    let mut schemas: HashMap<String, openapi::RefOr<Schema>> = inventory::iter::<ApiDtoMetadata>
        .into_iter()
        .map(|dto| (dto.schema_provider)())
        .collect();

    use utoipa::openapi::schema::{ObjectBuilder, Type};
    let string_schema = openapi::RefOr::T(Schema::Object(ObjectBuilder::new().schema_type(Type::String).build()));
    let integer_schema = openapi::RefOr::T(Schema::Object(ObjectBuilder::new().schema_type(Type::Integer).build()));
    let number_schema = openapi::RefOr::T(Schema::Object(ObjectBuilder::new().schema_type(Type::Number).build()));
    let boolean_schema = openapi::RefOr::T(Schema::Object(ObjectBuilder::new().schema_type(Type::Boolean).build()));
    schemas.entry("String".into()).or_insert(string_schema.clone());
    schemas.entry("&str".into()).or_insert(string_schema.clone());
    schemas.entry("i32".into()).or_insert(integer_schema.clone());
    schemas.entry("i64".into()).or_insert(integer_schema.clone());
    schemas.entry("u32".into()).or_insert(integer_schema.clone());
    schemas.entry("u64".into()).or_insert(integer_schema.clone());
    schemas.entry("f32".into()).or_insert(number_schema.clone());
    schemas.entry("f64".into()).or_insert(number_schema.clone());
    schemas.entry("bool".into()).or_insert(boolean_schema.clone());

    for metadata in inventory::iter::<ApiMetadata> {
        let mut operation_builder = OperationBuilder::new()
            .operation_id(Some(metadata.operation_id.to_string()))
            .summary(Some(metadata.summary.to_string()))
            .description(Some(metadata.description.to_string()))
            .tag("App");

        for param in metadata.parameters {
            let schema_ref = schemas
                .get(param.type_name)
                .cloned()
                .unwrap_or_else(|| openapi::RefOr::T(Schema::default()));

            match param.param_in {
                forge_core::ParamIn::Path => {
                    let built_parameter = ParameterBuilder::new()
                        .name(param.name)
                        .required(Required::True)
                        .description(Some(param.description))
                        .parameter_in(ParameterIn::Path)
                        .schema(Some(schema_ref))
                        .build();
                    operation_builder = operation_builder.parameter(built_parameter);
                }
                forge_core::ParamIn::Query => {
                    if let openapi::RefOr::T(Schema::Object(obj)) = &schema_ref {
                        for (prop_name, prop_schema) in obj.properties.iter() {
                            let is_required = obj.required.iter().any(|r| r == prop_name);
                            let built_parameter = ParameterBuilder::new()
                                .name(prop_name)
                                .required(if is_required { Required::True } else { Required::False })
                                .description(None::<&str>)
                                .parameter_in(ParameterIn::Query)
                                .schema(Some(prop_schema.clone()))
                                .build();
                            operation_builder = operation_builder.parameter(built_parameter);
                        }
                        if obj.properties.is_empty() {
                            let built_parameter = ParameterBuilder::new()
                                .name(param.name)
                                .required(if param.required { Required::True } else { Required::False })
                                .description(Some(param.description))
                                .parameter_in(ParameterIn::Query)
                                .schema(Some(schema_ref))
                                .build();
                            operation_builder = operation_builder.parameter(built_parameter);
                        }
                    } else {
                        let built_parameter = ParameterBuilder::new()
                            .name(param.name)
                            .required(if param.required { Required::True } else { Required::False })
                            .description(Some(param.description))
                            .parameter_in(ParameterIn::Query)
                            .schema(Some(schema_ref))
                            .build();
                        operation_builder = operation_builder.parameter(built_parameter);
                    }
                }
            }
        }

        if let Some(req_body_meta) = metadata.request_body {
            let schema_ref = schemas
                .get(req_body_meta.type_name)
                .cloned()
                .unwrap_or_else(|| openapi::RefOr::T(Schema::default()));
                
            let request_body = utoipa::openapi::request_body::RequestBodyBuilder::new()
                .description(Some(req_body_meta.description))
                .required(Some(if req_body_meta.required { Required::True } else { Required::False }))
                .content(
                    "application/json",
                    utoipa::openapi::ContentBuilder::new()
                        .schema(Some(schema_ref))
                        .build(),
                )
                .build();
            operation_builder = operation_builder.request_body(Some(request_body));
        }

        let mut responses_builder = utoipa::openapi::ResponsesBuilder::new();
        for resp in metadata.responses {
            let mut response_builder = utoipa::openapi::ResponseBuilder::new()
                .description(resp.description);
            
            if let Some(type_name) = resp.type_name {
                 if let Some(schema_ref) = schemas.get(type_name) {
                    response_builder = response_builder.content(
                        "application/json",
                        utoipa::openapi::ContentBuilder::new().schema(Some(schema_ref.clone())).build()
                    );
                 }
            }
            
            responses_builder = responses_builder.response(resp.status_code.to_string(), response_builder.build());
        }
        operation_builder = operation_builder.responses(responses_builder.build());

        let http_method = match metadata.method.to_lowercase().as_str() {
            "get" => utoipa::openapi::path::HttpMethod::Get,
            "post" => utoipa::openapi::path::HttpMethod::Post,
            "put" => utoipa::openapi::path::HttpMethod::Put,
            "delete" => utoipa::openapi::path::HttpMethod::Delete,
            "patch" => utoipa::openapi::path::HttpMethod::Patch,
            "options" => utoipa::openapi::path::HttpMethod::Options,
            "head" => utoipa::openapi::path::HttpMethod::Head,
            "trace" => utoipa::openapi::path::HttpMethod::Trace,
            _ => continue,
        };

        let operation = operation_builder.build();
        let path_item = openapi
            .paths
            .paths
            .entry(metadata.path.to_string())
            .or_default();
        
        match http_method {
            utoipa::openapi::path::HttpMethod::Get => path_item.get = Some(operation),
            utoipa::openapi::path::HttpMethod::Post => path_item.post = Some(operation),
            utoipa::openapi::path::HttpMethod::Put => path_item.put = Some(operation),
            utoipa::openapi::path::HttpMethod::Delete => path_item.delete = Some(operation),
            utoipa::openapi::path::HttpMethod::Options => path_item.options = Some(operation),
            utoipa::openapi::path::HttpMethod::Head => path_item.head = Some(operation),
            utoipa::openapi::path::HttpMethod::Patch => path_item.patch = Some(operation),
            utoipa::openapi::path::HttpMethod::Trace => path_item.trace = Some(operation),
        }
    }

    let components = ComponentsBuilder::new()
        .schemas_from_iter(schemas)
        .build();
    openapi.components = Some(components);

    openapi
}

/// Starts the web server.
pub async fn run_server() {
    dotenvy::dotenv().ok();
    handlers::load();
    
    let openapi = Arc::new(build_openapi_spec());

    if std::env::var("PRINT_OPENAPI").is_ok() {
        println!("{}", openapi.to_pretty_json().unwrap_or_else(|_| "{}".to_string()));
        return;
    }

    let rest_router = RestRouterBuilder::new()
        .openapi((*openapi).clone())
        .build()
        .expect("Failed to build REST router");

    let mcp_tool_router = OpenApiMcpRouterBuilder::new()
        .openapi((*openapi).clone())
        .build()
        .expect("Failed to build MCP router");
    
    let mcp_server = mcp_server::McpServerImpl::new(mcp_tool_router);
    let mcp_service = StreamableHttpService::new(
        move || Ok(mcp_server.clone()),
        LocalSessionManager::default().into(),
        Default::default(),
    );

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

    let port = env::var("PORT").unwrap_or_else(|_| "3000".to_string());
    let host = env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let address = format!("{}:{}", host, port);

    println!("🚀 Server running at http://{}", address);
    println!("📚 Swagger UI available at http://{}/swagger-ui", address);
    println!("💻 Forge CLI UI available at http://{}/cli-ui", address);

    let listener = TcpListener::bind(&address).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
