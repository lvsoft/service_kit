use service_kit::ApiDto;

#[ApiDto]
pub struct Product {
    pub id: String,
    pub product_code: String, // Added for testing camelCase
    pub name: String,
    pub description: Option<String>,
    pub price: f64,
    pub category: Category,
}

#[ApiDto]
#[derive(PartialEq, Eq)] // Example of preserving other derives
pub struct Category {
    pub id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")] // Example of preserving field attributes
    pub parent: Option<Box<Category>>,
}

// For testing snake_case override
#[ApiDto(rename_all = "snake_case")]
pub struct LegacyData {
    pub user_id: String,
    pub transaction_amount: f64,
}


#[cfg(test)]
mod tests {
    use super::*;
    use ts_rs::TS;
    use std::fs;
    use std::path::Path;

    #[test]
    fn export_ts_bindings() {
        // This path should now match the one in Cargo.toml
        let out_dir = Path::new("frontend/src/generated/types");
        if out_dir.exists() {
            fs::remove_dir_all(out_dir).unwrap();
        }
        fs::create_dir_all(out_dir).unwrap();

        let product_ts = Product::export_to_string().unwrap();
        fs::write(out_dir.join("Product.ts"), product_ts).unwrap();

        let category_ts = Category::export_to_string().unwrap();
        fs::write(out_dir.join("Category.ts"), category_ts).unwrap();

        let legacy_data_ts = LegacyData::export_to_string().unwrap();
        fs::write(out_dir.join("LegacyData.ts"), legacy_data_ts).unwrap();

        // Verify that the files were created
        assert!(out_dir.join("Product.ts").exists());
        assert!(out_dir.join("Category.ts").exists());
        assert!(out_dir.join("LegacyData.ts").exists());
    }
}
