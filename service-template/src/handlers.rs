use axum::{extract::Query, Json};
use crate::dtos::{AddParams, AddResponse, Greeting};
use service_kit::api_route;

/// Returns a simple greeting.
#[api_route(get, "/v1/hello")]
pub async fn hello() -> Json<Greeting> {
    let greeting = Greeting {
        message: "Hello, World!".to_string(),
    };
    Json(greeting)
}

/// Adds two numbers.
#[api_route(get, "/v1/add")]
pub async fn add(Query(params): Query<AddParams>) -> Json<AddResponse> {
    let result = params.a + params.b;
    Json(AddResponse { result })
}
