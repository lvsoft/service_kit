use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use axum::response::Response;
use once_cell::sync::Lazy;
use serde_json::Value;

pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// The global, thread-safe registry for all API method handlers.
///
/// It's a map from an `operation_id` to its corresponding `ApiMethodHandler`.
/// `once_cell::sync::Lazy` ensures that the HashMap is initialized exactly once.
static API_HANDLERS: Lazy<Arc<Mutex<HashMap<&'static str, ApiMethodHandler>>>> =
    Lazy::new(|| Arc::new(Mutex::new(HashMap::new())));

/// Registers a new `ApiMethodHandler` into the global registry.
///
/// This function is intended to be called by the `#[ctor]` functions generated
/// by the `#[api]` procedural macro at program startup.
pub fn register_handler(handler: ApiMethodHandler) {
    let operation_id = handler.operation_id;
    API_HANDLERS
        .lock()
        .expect("Failed to lock API_HANDLERS mutex")
        .insert(handler.operation_id, handler);
    // For debugging purposes, let's print the registered handler.
    println!("[service_kit] Registered API handler: {}", operation_id);
}

/// Retrieves a clone of the global API handler registry.
pub fn get_api_handlers() -> Arc<Mutex<HashMap<&'static str, ApiMethodHandler>>> {
    API_HANDLERS.clone()
}


/// A type-erased handler for an API method.
///
/// This struct holds the necessary information to dynamically call a business logic function.
/// It's designed to be registered at compile-time via the `#[ctor]` functions.
pub struct ApiMethodHandler {
    /// The unique identifier for the API operation, typically derived from the function name.
    /// This ID is used to link the handler to its OpenAPI specification.
    pub operation_id: &'static str,

    /// A type-erased closure that takes a `serde_json::Value` as input,
    /// calls the actual business logic function, and returns an Axum `Response`.
    ///
    /// The `Value` is expected to be a JSON object where keys correspond to the
    /// parameter names of the target function. The handler is responsible for
    /// deserializing these parameters into the correct types.
    pub handler: Arc<
        dyn for<'a> Fn(&'a Value) -> BoxFuture<'a, crate::error::Result<Response>> + Send + Sync,
    >,
}

impl Clone for ApiMethodHandler {
    fn clone(&self) -> Self {
        Self {
            operation_id: self.operation_id,
            handler: self.handler.clone(),
        }
    }
}

impl ApiMethodHandler {
    pub(crate) fn clone_for_mcp(
        &self,
    ) -> (
        String,
        Arc<
            dyn for<'a> Fn(
                    &'a Value,
                ) -> crate::handler::BoxFuture<'a, crate::error::Result<Response>>
                + Send
                + Sync,
        >,
    ) {
        (self.operation_id.to_string(), self.handler.clone())
    }
}

impl std::fmt::Debug for ApiMethodHandler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ApiMethodHandler")
            .field("operation_id", &self.operation_id)
            .finish()
    }
}

// Inventory-based handler discovery used by router builders
pub type DynHandlerFuture = Pin<Box<dyn Future<Output = crate::error::Result<Response>> + Send + 'static>>;

pub struct ApiHandlerInventory {
    pub operation_id: &'static str,
    pub handler: fn(&Value) -> DynHandlerFuture,
}

inventory::collect!(ApiHandlerInventory);
