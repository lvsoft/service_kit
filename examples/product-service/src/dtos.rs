use service_kit_macros::api_dto;

/// Parameters for adding two numbers.
#[api_dto]
pub struct AddParams {
    pub a: f64,
    pub b: f64,
}

/// Represents a product in the system.
#[api_dto]
pub struct Product {
    pub id: String,
    pub product_code: String,
    pub name: String,
    pub description: Option<String>,
    pub price: f64,
    pub category: Category,
}

#[api_dto]
pub struct Category {
    pub id: String,
    pub name: String,
    /// A category can have a parent, creating a recursive structure.
    pub parent: Option<Box<Category>>,
}

/// DTO for updating a product.
#[api_dto]
pub struct ProductUpdate {
    pub name: Option<String>,
    pub description: Option<String>,
    pub price: Option<f64>,
}

/// A simple greeting response.
#[api_dto]
pub struct Greeting {
    pub message: String,
}

/// This is a sample DTO with a different naming convention.
#[api_dto(rename_all = "snake_case")]
pub struct LegacyData {
    pub user_id: String,
    pub transaction_amount: f64,
}
