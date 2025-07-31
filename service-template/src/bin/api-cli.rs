//! This binary acts as a simple entry point to the `service_kit::api_cli` module.
//!
//! When a new service is generated from the template, it includes this binary.
//! The `cargo forge api-cli` command from the `service_kit` dependency then
//! simply invokes `cargo run --bin api-cli`, executing the logic from the
//! `service_kit::api_cli` module in the context of the current service.

fn main() -> anyhow::Result<()> {
    // 只有在 feature 启用时才可用
    #[cfg(feature = "api-cli")]
    {
        // 使用 tokio runtime 来运行异步函数
        let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
        rt.block_on(async {
            service_kit::api_cli::run().await
        })
    }

    #[cfg(not(feature = "api-cli"))]
    {
        panic!("api-cli requires the 'api-cli' feature to be enabled in service_kit.");
    }
}
