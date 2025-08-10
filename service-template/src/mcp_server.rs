use rmcp::{
    handler::server::router::tool::ToolRouter, tool_handler, ServerHandler,
};
use rmcp::model::*;

/// A generic MCP Server implementation that holds a dynamically built ToolRouter.
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

#[tool_handler]
impl ServerHandler for McpServerImpl {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities::builder()
                .enable_tools()
                .build(),
            server_info: Implementation::from_build_env(),
            instructions: Some(
                "This is a service generated from service-template.".to_string(),
            ),
        }
    }
}
