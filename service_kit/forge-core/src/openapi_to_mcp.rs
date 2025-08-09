//! # OpenAPI to MCP Router Builder
//!
//! This module is responsible for dynamically generating an `rmcp` Tool Router
//! based on an OpenAPI specification and a registry of compiled-in API handlers.

use crate::error::{Error, Result};
use crate::handler::{get_api_handlers, ApiMethodHandler};
use axum::response::Response;
use rmcp::handler::server::router::tool::{ToolRoute, ToolRouter};
use rmcp::model::{CallToolResult, Content, Tool};
use serde_json::{json, Map, Value};
use std::borrow::Cow;
use std::sync::Arc;
use utoipa::openapi::{OpenApi, PathItem, RefOr};
use utoipa::openapi::path::Operation;

/// A builder that creates an `rmcp::ToolRouter` from an OpenAPI document.
#[derive(Default, Clone)]
pub struct OpenApiMcpRouterBuilder {
    openapi: Option<OpenApi>,
}

impl OpenApiMcpRouterBuilder {
    /// Creates a new, empty builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the OpenAPI document to be used for building the router.
    pub fn openapi(mut self, openapi: OpenApi) -> Self {
        self.openapi = Some(openapi);
        self
    }

    /// Builds the `ToolRouter`.
    ///
    /// This is the core logic that iterates through the OpenAPI paths,
    /// finds the corresponding registered handler for each operation,
    /// and constructs an `rmcp` Tool for it.
    ///
    /// The generic type `S` is the state that the MCP server will hold.
    /// Our dynamically generated handlers do not require any state, so `S`
    /// can be any type that implements `Send + Sync + 'static`.
    pub fn build<S: Send + Sync + 'static>(self) -> Result<ToolRouter<S>> {
    let openapi = self.openapi.ok_or_else(|| {
        Error::SpecError("OpenAPI document not provided".to_string())
    })?;
    let handlers = get_api_handlers();
    let handlers_lock = handlers.lock().expect("Failed to lock handlers");

    let mut router = ToolRouter::new();

    for (_path, path_item) in openapi.paths.paths.iter() {
        for operation in operations_from_path_item(path_item) {
            if let Some(op_id) = operation.operation_id.as_deref() {
                if let Some(handler) = handlers_lock.get(op_id) {
                    let tool_route =
                        create_tool_route_for_handler(handler.clone_for_mcp(), operation)?;
                    router.add_route(tool_route);
                }
            }
        }
    }

    Ok(router)
}
}

/// Creates a `ToolRoute` from a registered `ApiMethodHandler` and OpenAPI `Operation`.
fn create_tool_route_for_handler<S: Send + Sync + 'static>(
    (operation_id, handler_fn): (
        String,
        Arc<
            dyn for<'a> Fn(
                    &'a Value,
                ) -> crate::handler::BoxFuture<'a, Result<Response>>
                + Send
                + Sync,
        >,
    ),
    operation: &Operation,
) -> Result<ToolRoute<S>> {
    // Extract input schema from the operation's parameters.
    // This is a simplified conversion. A full implementation would need to handle
    // all parameter types (header, cookie, etc.) and combine them into one schema.
    let input_schema = operation
        .parameters
        .as_ref()
        .and_then(|params| {
            params.iter().find_map(|p| {
                p.schema.as_ref().and_then(|s| {
                    if let RefOr::T(schema) = s {
                        Some(serde_json::to_value(schema).unwrap_or(json!({})))
                    } else {
                        None
                    }
                })
            })
        })
        .unwrap_or(json!({ "type": "object" }));

    let input_schema_map = if let Value::Object(map) = input_schema {
        Arc::new(map)
    } else {
        Arc::new(Map::new())
    };

    let tool_def = Tool {
        name: operation_id.clone().into(),
        description: operation.description.clone().map(Cow::from),
        input_schema: input_schema_map,
        output_schema: None,
        annotations: Default::default(),
    };

    // This is the core of the dynamic dispatch.
    // We create a closure that rmcp can call. Inside this closure,
    // we call our type-erased handler.
    let route = ToolRoute::new_dyn(tool_def, move |ctx| {
        let handler_clone = handler_fn.clone();
        Box::pin(async move {
            let params = ctx
                .arguments
                .as_ref()
                .map(|v| Value::Object(v.clone()))
                .unwrap_or(json!({}));

            match handler_clone(&params).await {
                Ok(response) => {
                    // Convert Axum response to MCP CallToolResult
                    let (parts, body) = response.into_parts();
                    let body_bytes =
                        axum::body::to_bytes(body, usize::MAX).await.unwrap_or_default();
                    let body_str = String::from_utf8_lossy(&body_bytes).to_string();

                    if parts.status.is_success() {
                        Ok(CallToolResult::success(vec![Content::text(body_str)]))
                    } else {
                        let err_msg =
                            format!("Handler failed with status {}: {}", parts.status, body_str);
                        Ok(CallToolResult::error(vec![Content::text(err_msg)]))
                    }
                }
                Err(e) => {
                    // Convert our internal error to MCP error
                    let err_msg = format!("Handler execution failed: {}", e);
                    Ok(CallToolResult::error(vec![Content::text(err_msg)]))
                }
            }
        })
    });

    Ok(route)
}

/// Helper to iterate over the defined operations in a PathItem.
fn operations_from_path_item(path_item: &PathItem) -> Vec<&Operation> {
    [
        &path_item.get,
        &path_item.post,
        &path_item.put,
        &path_item.delete,
        &path_item.options,
        &path_item.head,
        &path_item.patch,
        &path_item.trace,
    ]
    .iter()
    .filter_map(|op| op.as_ref())
    .collect()
}
