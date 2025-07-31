//! # Forge CLI - A built-in task runner for `service_kit`.
//!
//! This binary provides a set of commands to automate common development
//! and CI/CD tasks for services built with `service_kit`. It is intended to be
//! invoked via a local `cargo forge` alias in the generated service project.

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use toml::Value;

/// The main CLI entry point for `cargo forge`.
#[derive(Parser, Debug)]
#[command(author, version, about = "A custom build and task runner for projects using service_kit.")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

/// Defines the available subcommands for `cargo forge`.
#[derive(Subcommand, Debug)]
enum Commands {
    /// Generates TypeScript type definitions from Rust DTOs.
    GenerateTs,
    
    /// Lints the codebase using `cargo clippy`.
    Lint,
    
    /// Runs all unit and integration tests.
    Test,
    
    /// Proxies commands to the service's dedicated API CLI.
    ///
    /// This command acts as a proxy to the `api-cli` binary provided by the
    /// local service. It forwards all arguments, allowing you to interact
    /// with the dynamic OpenAPI client.
    #[command(external_subcommand)]
    ApiCli(Vec<String>),
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
        Commands::ApiCli(args) => api_cli(args)?,
    }

    Ok(())
}

/// Handler for the `generate-ts` command.
fn generate_ts() -> Result<()> {
    println!("▶️  Generating TypeScript types by running tests...");
    run_cargo_command(&["test"], "Failed to run tests for TS generation")?;
    
    let project_root = get_project_root()?;
    let ts_output_dir = get_ts_output_dir_from_project(&project_root)
        .unwrap_or_else(|| project_root.join("generated/ts"));

    println!("✅ TypeScript types generated successfully.");
    println!("   You can find them in: {}", ts_output_dir.display());

    Ok(())
}

/// Handler for the `lint` command.
fn lint() -> Result<()> {
    println!("▶️  Running linter...");
    println!("   Running 'cargo clippy' with -D warnings...");
    run_cargo_command(&["clippy", "--", "-D", "warnings"], "Failed to run cargo clippy")?;
    println!("✅ All checks passed.");
    Ok(())
}

/// Handler for the `test` command.
fn test() -> Result<()> {
    println!("▶️  Running all tests...");
    run_cargo_command(&["test"], "Failed to run cargo test")?;
    println!("✅ All tests passed.");
    Ok(())
}

/// Handler for the `api-cli` proxy command.
fn api_cli(args: Vec<String>) -> Result<()> {
    println!("▶️  Proxying to the service's `api-cli`...");
    let project_root = get_project_root()?;

    let status = Command::new("cargo")
        .current_dir(&project_root)
        .arg("run")
        .arg("--bin")
        .arg("api-cli")
        .arg("--")
        .args(args)
        .status()
        .context("Failed to run the service's `api-cli` binary. Does the service provide it?")?;
    
    if !status.success() {
        anyhow::bail!("`api-cli` command failed.");
    }

    Ok(())
}


// --- Helper Functions ---

/// A generic function to run a cargo command in the current project root.
fn run_cargo_command(args: &[&str], error_msg: &'static str) -> Result<()> {
    let project_root = get_project_root()?;
    let status = Command::new("cargo")
        .current_dir(&project_root)
        .args(args)
        .status()
        .context(error_msg)?;
    
    if !status.success() {
        anyhow::bail!("{} Command failed.", error_msg);
    }
    Ok(())
}

/// Helper function to locate the root of the current project.
fn get_project_root() -> Result<PathBuf> {
    env::current_dir().context("Failed to get current directory")
}

/// Reads the project's `Cargo.toml` and extracts the `ts_output_dir`.
fn get_ts_output_dir_from_project(project_root: &Path) -> Option<PathBuf> {
    let cargo_toml_path = project_root.join("Cargo.toml");
    let toml_content = fs::read_to_string(cargo_toml_path).ok()?;
    let toml_value: Value = toml::from_str(&toml_content).ok()?;
    let output_dir_str = toml_value
        .get("package")?
        .get("metadata")?
        .get("service_kit")?
        .get("ts_output_dir")?
        .as_str()?;
    Some(project_root.join(output_dir_str))
}
