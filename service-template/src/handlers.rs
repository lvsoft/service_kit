use axum::Json;
use crate::dtos::{AddParams, AddResponse, Greeting};
use service_kit::api;

/// Returns a simple greeting.
#[api(GET, "/v1/hello")]
pub async fn hello() -> Json<Greeting> {
    let greeting = Greeting {
        message: "Hello, World!".to_string(),
    };
    Json(greeting)
}

/// Adds two numbers.
#[api(POST, "/v1/add")]
pub async fn add(Json(params): Json<AddParams>) -> Json<AddResponse> {
    let result = params.a + params.b;
    Json(AddResponse { result })
}

/// A dummy function to ensure the linker includes this module.
pub fn load() {}
