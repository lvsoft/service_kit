use anyhow::Result;
use rmcp::{
    ServiceExt,
    model::{CallToolRequestParam, ClientCapabilities, ClientInfo, Implementation},
    transport::StreamableHttpClientTransport,
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| format!("info,{}=debug", env!("CARGO_CRATE_NAME")).into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    println!("🚀 Starting MCP Client to test product-service...");
    
    // Connect to our product-service MCP endpoint
    let transport = StreamableHttpClientTransport::from_uri("http://127.0.0.1:3000/mcp");
    
    let client_info = ClientInfo {
        protocol_version: Default::default(),
        capabilities: ClientCapabilities::default(),
        client_info: Implementation {
            name: "product-service-test-client".to_string(),
            version: "0.1.0".to_string(),
        },
    };

    println!("📡 Connecting to MCP server at http://127.0.0.1:3000/mcp...");
    let client = client_info.serve(transport).await.inspect_err(|e| {
        tracing::error!("client connection error: {:?}", e);
    })?;

    // Get server info
    let server_info = client.peer_info();
    println!("✅ Connected to server: {:#?}", server_info);

    // List available tools
    println!("\n📋 Listing available tools...");
    let tools = client.list_tools(Default::default()).await?;
    println!("Available tools: {:#?}", tools);

    // Test increment tool
    println!("\n🔧 Testing increment tool...");
    let increment_result = client
        .call_tool(CallToolRequestParam {
            name: "increment".into(),
            arguments: serde_json::json!({}).as_object().cloned(),
        })
        .await?;
    println!("Increment result: {:#?}", increment_result);

    // Test get tool
    println!("\n📊 Testing get tool...");
    let get_result = client
        .call_tool(CallToolRequestParam {
            name: "get".into(),
            arguments: serde_json::json!({}).as_object().cloned(),
        })
        .await?;
    println!("Get result: {:#?}", get_result);

    // Test increment again to see counter change
    println!("\n🔧 Testing increment tool again...");
    let increment_result2 = client
        .call_tool(CallToolRequestParam {
            name: "increment".into(),
            arguments: serde_json::json!({}).as_object().cloned(),
        })
        .await?;
    println!("Increment result 2: {:#?}", increment_result2);

    // Get final value
    println!("\n📊 Getting final counter value...");
    let final_get_result = client
        .call_tool(CallToolRequestParam {
            name: "get".into(),
            arguments: serde_json::json!({}).as_object().cloned(),
        })
        .await?;
    println!("Final get result: {:#?}", final_get_result);

    // Clean up
    client.cancel().await?;
    println!("\n✅ MCP Client test completed successfully!");

    Ok(())
}
