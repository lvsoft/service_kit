use service_kit::api_dto;

#[api_dto]
pub struct Greeting {
    pub message: String,
}

#[api_dto]
pub struct AddParams {
    pub a: f64,
    pub b: f64,
}

#[api_dto]
pub struct AddResponse {
    pub result: f64,
}
