use service_kit::api_dto;

#[api_dto]
pub struct Product {
    pub id: String,
    pub product_code: String, // Added for testing camelCase
    pub name: String,
    pub description: Option<String>,
    pub price: f64,
    pub category: Category,
}

#[api_dto]
#[derive(PartialEq, Eq)] // Example of preserving other derives
pub struct Category {
    pub id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")] // Example of preserving field attributes
    pub parent: Option<Box<Category>>,
}

// For testing snake_case override
#[api_dto(rename_all = "snake_case")]
pub struct LegacyData {
    pub user_id: String,
    pub transaction_amount: f64,
}
