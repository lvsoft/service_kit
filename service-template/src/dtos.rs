use service_kit::api_dto;

/// An example DTO.
#[api_dto]
pub struct Greeting {
    pub message: String,
}
