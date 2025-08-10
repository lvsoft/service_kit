use {{crate_name}} as app;
use axum::Router;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    app::load();

    let openapi = app::build_openapi_spec();

    // 由模板演示如何“组合式”地挂载各个子服务
    let rest = app::build_rest_router(openapi.clone()).expect("build rest router");
    #[cfg(feature = "swagger-ui")]
    let swagger = app::build_swagger_ui(openapi.clone());
    #[cfg(feature = "wasm-cli")]
    let cli_assets = app::build_cli_assets_router();

    let mut app_router: Router = Router::new()
        .merge(rest)
        .layer(app::default_cors_layer());

    #[cfg(feature = "swagger-ui")]
    { app_router = app_router.merge(swagger); }

    #[cfg(feature = "wasm-cli")]
    { app_router = app_router.merge(cli_assets); }

    #[cfg(feature = "mcp")]
    {
        let mcp = app::build_mcp_service(openapi.clone()).expect("build mcp service");
        app_router = app_router.nest_service("/mcp", mcp);
    }

    let address = std::env::var("HOST").unwrap_or_else(|_| "0.0.0.0".into())
        + ":"
        + &std::env::var("PORT").unwrap_or_else(|_| "3000".into());
    println!("🚀 Server running at http://{}", address);
    println!("📚 Swagger UI available at http://{}/swagger-ui", address);
    println!("💻 Forge CLI UI available at http://{}/cli-ui", address);
    let listener = tokio::net::TcpListener::bind(&address).await.unwrap();
    axum::serve(listener, app_router).await.unwrap();
}

