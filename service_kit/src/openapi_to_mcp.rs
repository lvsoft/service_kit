//! OpenAPI to MCP Router Builder

use crate::error::{Error, Result};
use crate::handler::ApiHandlerInventory;
use axum::response::Response;
use rmcp::handler::server::router::tool::{ToolRoute, ToolRouter};
use rmcp::model::{CallToolResult, Content, Tool};
use serde_json::{json, Map, Value};
use std::borrow::Cow;
use std::sync::Arc;
use utoipa::openapi::{OpenApi, PathItem, RefOr};
use utoipa::openapi::path::Operation;
use std::collections::HashMap;

#[derive(Default, Clone)]
pub struct OpenApiMcpRouterBuilder {
    openapi: Option<OpenApi>,
}

impl OpenApiMcpRouterBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn openapi(mut self, openapi: OpenApi) -> Self {
        self.openapi = Some(openapi);
        self
    }

    pub fn build<S: Send + Sync + 'static>(self) -> Result<ToolRouter<S>> {
        let openapi = self.openapi.ok_or_else(|| {
            Error::SpecError("OpenAPI document not provided".to_string())
        })?;
        let handlers: HashMap<&'static str, for<'a> fn(&'a Value) -> crate::handler::DynHandlerFuture> =
            crate::inventory::iter::<ApiHandlerInventory>
                .into_iter()
                .map(|inv| (inv.operation_id, inv.handler))
                .collect();

        let mut router = ToolRouter::new();

        for (_path, path_item) in openapi.paths.paths.iter() {
            for operation in operations_from_path_item(path_item) {
                if let Some(op_id) = operation.operation_id.as_deref() {
                    if let Some(handler_fn) = handlers.get(op_id).cloned() {
                        let tool_route =
                            create_tool_route_for_handler((op_id.to_string(), handler_fn), operation)?;
                        router.add_route(tool_route);
                    }
                }
            }
        }

        Ok(router)
    }
}

fn create_tool_route_for_handler<S: Send + Sync + 'static>(
    (operation_id, handler_fn): (
        String,
        for<'a> fn(&'a Value) -> crate::handler::DynHandlerFuture,
    ),
    operation: &Operation,
) -> Result<ToolRoute<S>> {
    let input_schema = operation
        .request_body
        .as_ref()
        .and_then(|body| body.content.get("application/json"))
        .and_then(|media_type| media_type.schema.as_ref())
        .and_then(|schema| match schema {
            RefOr::T(s) => serde_json::to_value(s).ok(),
            RefOr::Ref(_) => None,
        })
        .or_else(|| {
            operation
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

    let route = ToolRoute::new_dyn(tool_def, move |ctx| {
        let handler_clone = handler_fn;
        Box::pin(async move {
            let params = ctx
                .arguments
                .as_ref()
                .map(|v| Value::Object(v.clone()))
                .unwrap_or(json!({}));

            match handler_clone(&params).await {
                Ok(response) => {
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
                    let err_msg = format!("Handler execution failed: {}", e);
                    Ok(CallToolResult::error(vec![Content::text(err_msg)]))
                }
            }
        })
    });

    Ok(route)
}

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


