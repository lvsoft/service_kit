//! # Forge CLI - A custom task runner for `service_kit` development.
//!
//! This `xtask` crate provides a set of commands to automate common development
//! and CI/CD tasks for services built with `service_kit`. It is invoked via
//! the `cargo forge` alias, configured in the workspace's `.cargo/config.toml`.

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use toml::Value;

/// The main CLI entry point for `cargo forge`.
#[derive(Parser, Debug)]
#[command(author, version, about = "A custom build and task runner for the service_kit project.")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

/// Defines the available subcommands for `cargo forge`.
#[derive(Subcommand, Debug)]
enum Commands {
    /// Generates TypeScript type definitions from Rust DTOs.
    ///
    /// This command works by running `cargo test` on the target service.
    /// It relies on a specific test function (e.g., `export_ts_bindings`)
    /// within the service's test suite to perform the actual file generation.
    GenerateTs,
    
    /// Lints the codebase using `cargo clippy`.
    ///
    /// This command runs `cargo clippy` across the entire workspace with strict
    /// settings (`-D warnings`), treating all warnings as errors. This enforces
    /// high code quality and catches potential issues early.
    Lint,
    
    /// Runs all unit and integration tests in the workspace.
    Test,
}

fn main() -> Result<()> {
    // When invoked as `cargo forge`, Cargo passes "forge" as the first argument.
    // We manually remove it so that `clap` can parse the subcommands correctly.
    let mut args: Vec<String> = env::args().collect();
    if args.get(1).map(|s| s.as_str()) == Some("forge") {
        args.remove(1);
    }
    
    let cli = Cli::parse_from(args);

    match cli.command {
        Commands::GenerateTs => generate_ts()?,
        Commands::Lint => lint()?,
        Commands::Test => test()?,
    }

    Ok(())
}

/// Handler for the `generate-ts` command.
fn generate_ts() -> Result<()> {
    println!("▶️  Generating TypeScript types by running tests...");

    let project_root = get_project_root()?;
    let service_dir = project_root.join("examples/product-service");
    
    let status = Command::new("cargo")
        .current_dir(&service_dir)
        .arg("test")
        .status()
        .context("Failed to run cargo test to generate TS types")?;

    if !status.success() {
        anyhow::bail!("Failed to generate TypeScript types. The test command failed.");
    }
    
    let ts_output_dir = get_ts_output_dir_from_workspace(&service_dir)
        .unwrap_or_else(|| service_dir.join("generated/ts"));

    println!("✅ TypeScript types generated successfully.");
    println!("   You can find them in: {}", ts_output_dir.display());

    Ok(())
}

/// Handler for the `lint` command.
fn lint() -> Result<()> {
    println!("▶️  Running linter...");

    let project_root = get_project_root()?;
    
    println!("   Running 'cargo clippy' with -D warnings...");
    let clippy_status = Command::new("cargo")
        .current_dir(&project_root)
        .arg("clippy")
        .arg("--")
        .arg("-D")
        .arg("warnings")
        .status()
        .context("Failed to run cargo clippy")?;
    
    if !clippy_status.success() {
        anyhow::bail!("Clippy found errors.");
    }
    
    println!("✅ All checks passed.");
    Ok(())
}

/// Handler for the `test` command.
fn test() -> Result<()> {
    println!("▶️  Running all tests...");

    let project_root = get_project_root()?;

    let status = Command::new("cargo")
        .current_dir(&project_root)
        .arg("test")
        .status()
        .context("Failed to run cargo test")?;

    if !status.success() {
        anyhow::bail!("Tests failed.");
    }

    println!("✅ All tests passed.");
    Ok(())
}

/// Helper function to locate the root of the workspace.
fn get_project_root() -> Result<PathBuf> {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .context("Failed to get project root")
        .map(|p| p.to_path_buf())
}

/// Reads a workspace member's `Cargo.toml` and extracts the `ts_output_dir`
/// from the `[package.metadata.service_kit]` table.
fn get_ts_output_dir_from_workspace(workspace_member: &Path) -> Option<PathBuf> {
    let cargo_toml_path = workspace_member.join("Cargo.toml");
    
    let toml_content = fs::read_to_string(cargo_toml_path).ok()?;
    let toml_value: Value = toml::from_str(&toml_content).ok()?;

    let output_dir_str = toml_value
        .get("package")?
        .get("metadata")?
        .get("service_kit")?
        .get("ts_output_dir")?
        .as_str()?;
        
    Some(workspace_member.join(output_dir_str))
}
