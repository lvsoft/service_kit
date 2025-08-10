use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use axum::response::Response;
use once_cell::sync::Lazy;
use serde_json::Value;

pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

static API_HANDLERS: Lazy<Arc<Mutex<HashMap<&'static str, ApiMethodHandler>>>> =
    Lazy::new(|| Arc::new(Mutex::new(HashMap::new())));

pub fn register_handler(handler: ApiMethodHandler) {
    let operation_id = handler.operation_id;
    API_HANDLERS
        .lock()
        .expect("Failed to lock API_HANDLERS mutex")
        .insert(handler.operation_id, handler);
    println!("[service_kit] Registered API handler: {}", operation_id);
}

pub fn get_api_handlers() -> Arc<Mutex<HashMap<&'static str, ApiMethodHandler>>> {
    API_HANDLERS.clone()
}

pub struct ApiMethodHandler {
    pub operation_id: &'static str,
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

pub type DynHandlerFuture = Pin<Box<dyn Future<Output = crate::error::Result<Response>> + Send + 'static>>;

pub struct ApiHandlerInventory {
    pub operation_id: &'static str,
    pub handler: fn(&Value) -> DynHandlerFuture,
}

inventory::collect!(ApiHandlerInventory);


