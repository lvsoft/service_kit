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
