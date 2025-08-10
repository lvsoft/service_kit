// Copyright 2024 The Forgejo Authors. All rights reserved.
// SPDX-License-Identifier: MIT

pub mod cli;
#[cfg(not(target_arch = "wasm32"))]
pub mod client;
#[cfg(not(target_arch = "wasm32"))]
pub mod completer;
pub mod error;
#[cfg(not(target_arch = "wasm32"))]
pub mod handler;
#[cfg(not(target_arch = "wasm32"))]
pub mod openapi_to_mcp;
#[cfg(not(target_arch = "wasm32"))]
pub mod repl;
pub mod wasm_completer;
#[cfg(not(target_arch = "wasm32"))]
pub mod rest_router_builder;

// Re-export key dependencies so that macros can use them
pub use inventory;
pub use utoipa;

// --- Shared Metadata Structs ---

#[derive(Debug, Clone, Copy)]
pub enum ParamIn {
    Query,
    Path,
}

#[derive(Debug)]
pub struct ApiParameter {
    pub name: &'static str,
    pub param_in: ParamIn,
    pub description: &'static str,
    pub required: bool,
    pub type_name: &'static str,
}

#[derive(Debug)]
pub struct ApiRequestBody {
    pub description: &'static str,
    pub required: bool,
    pub type_name: &'static str,
}

#[derive(Debug)]
pub struct ApiResponse {
    pub status_code: u16,
    pub description: &'static str,
    pub type_name: Option<&'static str>,
}

#[derive(Debug)]
pub struct ApiMetadata {
    pub operation_id: &'static str,
    pub method: &'static str,
    pub path: &'static str,
    pub summary: &'static str,
    pub description: &'static str,
    pub parameters: &'static [ApiParameter],
    pub request_body: Option<&'static ApiRequestBody>,
    pub responses: &'static [ApiResponse],
}
inventory::collect!(ApiMetadata);

pub struct ApiDtoMetadata {
    pub name: &'static str,
    pub schema_provider: fn() -> (String, utoipa::openapi::RefOr<utoipa::openapi::Schema>),
}
inventory::collect!(ApiDtoMetadata);


