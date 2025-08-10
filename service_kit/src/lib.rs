//! # Service Kit - A Foundational Toolkit for Rust-Based Microservices
//!
//! `service_kit` offers a collection of tools and macros to accelerate the
//! development of high-performance, modular services in Rust. It aims to reduce
//! boilerplate, enforce best practices, and streamline common tasks like API
//! documentation and DTO creation.
//!
//! ## Core Features:
//!
//! - **`#[api_dto]`**: A procedural macro to automatically derive essential traits
//!   (`serde::Serialize`, `serde::Deserialize`, `utoipa::ToSchema`, etc.) for your
//!   Data Transfer Objects. It intelligently handles recursive types and provides
//!   sensible defaults for JSON serialization.
//!
//! - **`#[api_route]`**: (Work in Progress) An attribute macro designed to simplify
//!   `axum` route handlers by automatically generating `utoipa` OpenAPI path
//!   definitions from the function signature.
//!
//! - **`ApiDocBuilder`**: (Planned) A builder to automatically discover all `#[api_route]`
//!   and `#[api_dto]` definitions within your project to generate a complete
//!   OpenAPI specification with minimal manual effort.
//!

// Here, we will later add the ApiDocBuilder and other runtime utilities.

// --- Unified facade exports and modules ---
#[cfg(feature = "macros")]
pub use service_kit_macros::{api, api_dto};

pub use inventory;
pub use utoipa;

pub mod error;
pub mod handler;

// 仅在启用 mcp 特性且非 wasm 目标时提供
#[cfg(all(not(target_arch = "wasm32"), feature = "mcp"))]
pub mod openapi_to_mcp;

// REST 路由构建器（保持原样，仅非 wasm）
#[cfg(not(target_arch = "wasm32"))]
pub mod rest_router_builder;

// 仅在启用 api-cli 特性且非 wasm 目标时提供（需要 reqwest/tokio 等）
#[cfg(all(not(target_arch = "wasm32"), feature = "api-cli"))]
pub mod client;

// CLI 构建与补全：在启用 cli-core 特性时提供（兼容 wasm 与 native）
#[cfg(feature = "cli-core")]
pub mod cli;
#[cfg(feature = "cli-core")]
pub mod wasm_completer;
pub mod openapi_utils;
pub mod bootstrap;

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
