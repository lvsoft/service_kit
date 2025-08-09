use rmcp::{
    handler::server::router::tool::ToolRouter, tool_handler, ServerHandler,
};
use rmcp::model::*;

/// A generic MCP Server implementation that holds a dynamically built ToolRouter.
///
/// This server is "state-agnostic" in this context, so we use `()` as the state type `S`.
/// If the MCP server itself needed state, we would replace `()` with a state struct.
#[derive(Clone)]
pub struct McpServerImpl {
    tool_router: ToolRouter<McpServerImpl>,
}

impl McpServerImpl {
    /// Creates a new McpServerImpl with a pre-built ToolRouter.
    pub fn new(tool_router: ToolRouter<McpServerImpl>) -> Self {
        Self { tool_router }
    }
}

// The tool_handler macro now helps implement the `call_tool` method
// by delegating the call to the contained `tool_router`.
#[tool_handler]
impl ServerHandler for McpServerImpl {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities::builder()
                .enable_tools()
                // We could dynamically report tools here if needed
                .build(),
            server_info: Implementation::from_build_env(),
            instructions: Some(
                "This is a product service MCP server, with tools dynamically generated from OpenAPI.".to_string(),
            ),
        }
    }
}
