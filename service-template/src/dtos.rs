use service_kit::{api_dto, api_params};

#[api_dto]
pub struct Greeting {
    pub message: String,
}

#[api_params]
pub struct AddParams {
    pub a: i32,
    pub b: i32,
}

#[api_dto]
pub struct AddResponse {
    pub result: i32,
}
