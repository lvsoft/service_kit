use axum::Json;
use crate::dtos::Greeting;

/// Returns a simple greeting.
#[utoipa::path(
    get,
    path = "/v1/hello",
    responses(
        (status = 200, description = "Successful greeting", body = Greeting)
    )
)]
pub async fn hello() -> Json<Greeting> {
    let greeting = Greeting {
        message: "Hello, World!".to_string(),
    };
    Json(greeting)
}
