use axum::{extract::Path, Json, response::IntoResponse};
use service_kit_macros::api;
use crate::dtos::Product;

/// Get a product by its ID
///
/// This endpoint retrieves a specific product from the database using its unique identifier.
#[api(GET, "/v1/products/{id}")]
pub async fn get_product(Path(id): Path<String>) -> impl IntoResponse {
    let sample_product = Product {
        id,
        product_code: "P-12345".to_string(), // Added value for the new field
        name: "Example Product".to_string(),
        description: Some("This is a product from the mock handler.".to_string()),
        price: 99.99,
        category: crate::dtos::Category {
            id: "cat-01".to_string(),
            name: "Electronics".to_string(),
            parent: None,
        },
    };
    Json(sample_product)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dtos::LegacyData;
    use serde_json;

    #[test]
    fn test_product_serialization_is_camel_case() {
        let product = Product {
            id: "prod-001".to_string(),
            product_code: "PC-XYZ".to_string(),
            name: "Test Product".to_string(),
            description: None,
            price: 10.50,
            category: Category {
                id: "cat-tech".to_string(),
                name: "Technology".to_string(),
                parent: None,
            },
        };

        let json_string = serde_json::to_string(&product).unwrap();
        let json_value: serde_json::Value = serde_json::from_str(&json_string).unwrap();

        // Assert that the key is "productCode" and not "product_code"
        assert!(json_value.get("productCode").is_some());
        assert!(json_value.get("product_code").is_none());
    }
    
    #[test]
    fn test_legacy_data_serialization_is_snake_case() {
        let legacy_data = LegacyData {
            user_id: "user-123".to_string(),
            transaction_amount: 199.99,
        };

        let json_string = serde_json::to_string(&legacy_data).unwrap();
        let json_value: serde_json::Value = serde_json::from_str(&json_string).unwrap();

        // Assert that keys are "user_id" and "transaction_amount"
        assert!(json_value.get("user_id").is_some());
        assert!(json_value.get("transaction_amount").is_some());
        assert!(json_value.get("userId").is_none()); // Check that camelCase was not used
    }
}
