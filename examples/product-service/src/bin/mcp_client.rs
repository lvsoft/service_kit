use rmcp::{
    model::{CallToolRequestParam, ClientCapabilities, ClientInfo, Implementation},
    transport::StreamableHttpClientTransport,
    ServiceExt,
};

#[tokio::main]
async fn main() {
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
    let Ok(client) = client_info.serve(transport).await else {
        eprintln!("❌ Failed to connect to MCP server");
        return;
    };

    println!("✅ Connected. Peer info: {:#?}", client.peer_info());

    println!("\n📋 Listing available tools...");
    let tool_list = match client.list_tools(Default::default()).await {
        Ok(tools) => { println!("Available tools: {:#?}", tools); tools }
        Err(e) => { eprintln!("❌ list_tools failed: {}", e); return; }
    };

    // Prefer operation_id tool names: "add", "get_product"
    let mut has_add = false;
    let mut has_get_product = false;
    for t in &tool_list.tools {
        let name = &t.name;
        if name == "add" { has_add = true; }
        if name == "get_product" { has_get_product = true; }
    }

    if has_add {
        println!("\n🔧 Call tool: add");
        match client
            .call_tool(CallToolRequestParam {
                name: "add".into(),
                arguments: serde_json::json!({ "a": 1.0, "b": 2.0 }).as_object().cloned(),
            })
            .await
        {
            Ok(res) => println!("add result: {:#?}", res),
            Err(e) => eprintln!("❌ call add failed: {}", e),
        }
    } else {
        eprintln!("⚠️ tool 'add' not found in server tool list");
    }

    if has_get_product {
        println!("\n🔧 Call tool: get_product");
        match client
            .call_tool(CallToolRequestParam {
                name: "get_product".into(),
                arguments: serde_json::json!({ "id": "1" }).as_object().cloned(),
            })
            .await
        {
            Ok(res) => println!("get_product result: {:#?}", res),
            Err(e) => eprintln!("❌ call get_product failed: {}", e),
        }
    } else {
        eprintln!("⚠️ tool 'get_product' not found in server tool list");
    }

    println!("\n✅ MCP Client test completed.");
}
