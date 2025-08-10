use crate::error::{Error, Result};
use crate::handler::ApiHandlerInventory;
use axum::{
    body::Body,
    extract::{FromRequestParts, Path},
    response::{IntoResponse, Response},
    routing::{on, MethodFilter},
    Router,
};
use axum::http::Request;
use serde_json::Value;
use std::collections::HashMap;
use utoipa::openapi::{OpenApi, PathItem};

async fn extract_and_merge_params(req: Request<Body>) -> std::result::Result<Value, Response> {
    let (mut parts, body) = req.into_parts();

    let path_params: HashMap<String, String> =
        match Path::<HashMap<String, String>>::from_request_parts(&mut parts, &()).await {
            Ok(path) => path.0,
            Err(e) => return Err(e.into_response()),
        };
    let mut merged_params = serde_json::to_value(path_params)
        .unwrap_or_else(|_| Value::Object(Default::default()));

    if let Some(query_str) = parts.uri.query() {
        if let Ok(pairs) = serde_urlencoded::from_str::<Vec<(String, String)>>(query_str) {
            if let Some(merged) = merged_params.as_object_mut() {
                for (k, v) in pairs {
                    if let Ok(n) = v.parse::<f64>() {
                        merged.insert(k, serde_json::json!(n));
                    } else if v.eq_ignore_ascii_case("true") || v.eq_ignore_ascii_case("false") {
                        merged.insert(k, serde_json::json!(v.eq_ignore_ascii_case("true")));
                    } else {
                        merged.insert(k, Value::String(v));
                    }
                }
            }
        }
    }

    let headers = parts.headers.clone();
    if let Some(content_type) = headers.get(axum::http::header::CONTENT_TYPE) {
        if content_type.to_str().unwrap_or("").contains("application/json") {
            let body_bytes = match axum::body::to_bytes(body, usize::MAX).await {
                Ok(bytes) => bytes,
                Err(e) => {
                    return Err((
                        axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                        format!("Failed to read request body: {}", e),
                    )
                        .into_response())
                }
            };

            if let Ok(body_json) = serde_json::from_slice::<Value>(&body_bytes) {
                if let (Some(merged), Some(body_obj)) = (merged_params.as_object_mut(), body_json.as_object()) {
                    for (k, v) in body_obj {
                        merged.insert(k.clone(), v.clone());
                    }
                }
            }
        }
    }

    Ok(merged_params)
}

#[derive(Default, Clone)]
pub struct RestRouterBuilder {
    openapi: Option<OpenApi>,
}

impl RestRouterBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn openapi(mut self, openapi: OpenApi) -> Self {
        self.openapi = Some(openapi);
        self
    }

    pub fn build(self) -> Result<Router> {
        let openapi = self.openapi.ok_or_else(|| {
            Error::SpecError("OpenAPI document not provided".to_string())
        })?;
        let mut handler_map: std::collections::HashMap<&'static str, fn(&serde_json::Value) -> crate::handler::DynHandlerFuture> = std::collections::HashMap::new();
        for inv in inventory::iter::<ApiHandlerInventory> {
            handler_map.insert(inv.operation_id, inv.handler);
        }

        let mut router = Router::new();

        for (path, path_item) in openapi.paths.paths.iter() {
            for (method, operation) in operations_from_path_item(path_item) {
                if let Some(op_id) = operation.operation_id.as_deref() {
                    if let Some(handler_fn) = handler_map.get(op_id) {
                        let handler_fn = *handler_fn;
                        let route_handler = move |req: Request<Body>| async move {
                            match extract_and_merge_params(req).await {
                                Ok(params) => match handler_fn(&params).await {
                                    Ok(resp) => resp,
                                    Err(e) => e.into_response(),
                                },
                                Err(response) => response,
                            }
                        };

                        let axum_path = path.clone();
                        router = router.route(&axum_path, on(method, route_handler));
                    }
                }
            }
        }
        Ok(router)
    }
}

fn operations_from_path_item(path_item: &PathItem) -> Vec<(MethodFilter, &utoipa::openapi::path::Operation)> {
    let mut operations = Vec::new();
    if let Some(op) = &path_item.get { operations.push((MethodFilter::GET, op)); }
    if let Some(op) = &path_item.post { operations.push((MethodFilter::POST, op)); }
    if let Some(op) = &path_item.put { operations.push((MethodFilter::PUT, op)); }
    if let Some(op) = &path_item.delete { operations.push((MethodFilter::DELETE, op)); }
    if let Some(op) = &path_item.patch { operations.push((MethodFilter::PATCH, op)); }
    operations
}


