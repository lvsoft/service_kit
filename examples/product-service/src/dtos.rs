use serde::{Serialize, Deserialize};
use utoipa::ToSchema;

#[derive(Serialize, Deserialize, ToSchema, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Product {
    pub id: String,
    pub product_code: String,
    pub name: String,
    pub description: Option<String>,
    pub price: f64,
    pub category: Category,
}

#[derive(Serialize, Deserialize, ToSchema, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Category {
    pub id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent: Option<Box<Category>>,
}

// For testing snake_case override
#[derive(Serialize, Deserialize, ToSchema, Clone)]
#[serde(rename_all = "snake_case")]
pub struct LegacyData {
    pub user_id: String,
    pub transaction_amount: f64,
}
