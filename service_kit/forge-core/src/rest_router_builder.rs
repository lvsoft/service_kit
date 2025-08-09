//! # REST Router Builder from OpenAPI
// Copyright 2024 The Forgejo Authors. All rights reserved.
// SPDX-License-Identifier: MIT

use crate::error::{Error, Result};
use crate::handler::get_api_handlers;
use axum::{
    body::Body,
    extract::{FromRequest, Path},
    response::IntoResponse,
    routing::{on, MethodFilter},
    Router,
};
use axum::http::Request;
use std::collections::HashMap;
use utoipa::openapi::{OpenApi, PathItem};

/// A builder that creates an `axum::Router` for REST APIs from an OpenAPI document.
#[derive(Default, Clone)]
pub struct RestRouterBuilder {
    openapi: Option<OpenApi>,
}

impl RestRouterBuilder {
    /// Creates a new, empty builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the OpenAPI document to be used for building the router.
    pub fn openapi(mut self, openapi: OpenApi) -> Self {
        self.openapi = Some(openapi);
        self
    }

    /// Builds the `axum::Router`.
    ///
    /// This function iterates through the OpenAPI paths, finds the corresponding
    /// registered handler for each operation, and constructs an Axum route for it.
    pub fn build(self) -> Result<Router> {
        let openapi = self.openapi.ok_or_else(|| {
            Error::SpecError("OpenAPI document not provided".to_string())
        })?;
        let handlers = get_api_handlers();
        let handlers_lock = handlers.lock().expect("Failed to lock handlers");

        let mut router = Router::new();

        for (path, path_item) in openapi.paths.paths.iter() {
            for (method, operation) in operations_from_path_item(path_item) {
                if let Some(op_id) = operation.operation_id.as_deref() {
                    if let Some(handler_info) = handlers_lock.get(op_id) {
                        let handler = handler_info.handler.clone();
                        let route_handler = move |req: Request<Body>| async move {
                            // This is a simplified parameter extraction logic.
                            // It assumes path parameters are the primary input.
                            let path_params: HashMap<String, String> =
                                Path::from_request(req, &())
                                    .await
                                    .map(|path: Path<HashMap<String, String>>| path.0)
                                    .unwrap_or_default();

                            let params = serde_json::to_value(path_params)
                                .unwrap_or(serde_json::Value::Null);

                            handler(&params)
                                .await
                                .unwrap_or_else(|e| e.into_response())
                        };

                        // Axum path parameters use `:name` syntax.
                        let axum_path = path.replace('{', ":").replace('}', "");
                        router = router.route(&axum_path, on(method, route_handler));
                    }
                }
            }
        }
        Ok(router)
    }
}

/// Helper to iterate over the defined operations in a PathItem.
fn operations_from_path_item(path_item: &PathItem) -> Vec<(MethodFilter, &utoipa::openapi::path::Operation)> {
    let mut operations = Vec::new();
    if let Some(op) = &path_item.get { operations.push((MethodFilter::GET, op)); }
    if let Some(op) = &path_item.post { operations.push((MethodFilter::POST, op)); }
    if let Some(op) = &path_item.put { operations.push((MethodFilter::PUT, op)); }
    if let Some(op) = &path_item.delete { operations.push((MethodFilter::DELETE, op)); }
    if let Some(op) = &path_item.patch { operations.push((MethodFilter::PATCH, op)); }
    // Add other methods as needed
    operations
}
