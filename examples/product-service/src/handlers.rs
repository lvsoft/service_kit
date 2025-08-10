use axum::{extract::{Path, Query}, Json, response::IntoResponse};
use service_kit::api;
use crate::dtos::{AddParams, Product, ProductUpdate};

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

/// Add two numbers
/// This endpoint is a simple calculator to add two numbers.
#[api(GET, "/v1/add")]
pub async fn add(Query(params): Query<AddParams>) -> Json<f64> {
    Json(params.a + params.b)
}

/// List all products
/// This endpoint returns a list of all products in the system.
#[api(GET, "/v1/products")]
pub async fn list_products() -> Json<Vec<Product>> {
    Json(vec![
        Product {
            id: "prod-001".to_string(),
            product_code: "P-12345".to_string(),
            name: "Example Product 1".to_string(),
            description: Some("This is product 1.".to_string()),
            price: 99.99,
            category: crate::dtos::Category {
                id: "cat-01".to_string(),
                name: "Electronics".to_string(),
                parent: None,
            },
        },
        Product {
            id: "prod-002".to_string(),
            product_code: "P-67890".to_string(),
            name: "Example Product 2".to_string(),
            description: Some("This is product 2.".to_string()),
            price: 149.99,
            category: crate::dtos::Category {
                id: "cat-02".to_string(),
                name: "Books".to_string(),
                parent: None,
            },
        },
    ])
}

/// Update a product
/// This endpoint updates a product's information.
#[api(PATCH, "/v1/products/{id}")]
pub async fn update_product(
    Path(id): Path<String>,
    Json(payload): Json<ProductUpdate>,
) -> Json<Product> {
    // In a real implementation, you would fetch the product, update it, and save it.
    Json(Product {
        id,
        product_code: "P-UPDATED".to_string(),
        name: payload.name.unwrap_or_else(|| "Old Name".to_string()),
        description: payload.description,
        price: payload.price.unwrap_or(0.0),
        category: crate::dtos::Category {
            id: "cat-01".to_string(),
            name: "Electronics".to_string(),
            parent: None,
        },
    })
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

        assert!(json_value.get("user_id").is_some());
        assert!(json_value.get("transaction_amount").is_some());
        assert!(json_value.get("userId").is_none());
    }
}

/// A dummy function to ensure the linker includes this module.
pub fn load() {}
