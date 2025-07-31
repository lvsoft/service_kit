//! This binary acts as a simple entry point to the `forge_api_cli` library.
//!
//! When a new service is generated from the template, it includes this binary.
//! The `cargo forge api-cli` command from the `service_kit` dependency then
//! simply invokes `cargo run --bin api-cli`, executing the logic from the
//! `forge_api_cli` library in the context of the current service.

fn main() -> anyhow::Result<()> {
    // Call the main run function from the `forge_api_cli` library.
    forge_api_cli::run()
}
