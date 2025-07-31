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
use std::process::{Command, Stdio};
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
    
    /// Generates the OpenAPI specification file.
    GenerateOpenapiSpec,

    /// Interact with the API using a generated command-line client.
    ///
    /// This command acts as a wrapper around an OpenAPI client tool (e.g., `oas-cli`).
    /// It first generates the latest OpenAPI spec, then passes all subsequent
    /// arguments to the client tool.
    ///
    /// You must have an OpenAPI client tool installed and available in your PATH.
    /// We recommend `oas-cli`: `npm install -g oas-cli`
    #[command(external_subcommand)]
    ApiCli(Vec<String>),
}

fn main() -> Result<()> {
    let mut args: Vec<String> = env::args().collect();
    if args.get(1).map(|s| s.as_str()) == Some("forge") {
        args.remove(1);
    }
    
    let cli = Cli::parse_from(args);

    match cli.command {
        Commands::GenerateTs => generate_ts()?,
        Commands::Lint => lint()?,
        Commands::Test => test()?,
        Commands::GenerateOpenapiSpec => {
            generate_openapi_spec()?;
        }
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

/// Handler for the `generate-openapi-spec` command.
fn generate_openapi_spec() -> Result<PathBuf> {
    println!("▶️  Generating OpenAPI specification...");

    let project_root = get_project_root()?;
    let target_dir = project_root.join("target");
    let spec_path = target_dir.join("openapi.json");
    fs::create_dir_all(&target_dir)?;

    let output = Command::new("cargo")
        .current_dir(&project_root)
        .args(["run", "--bin", "openapi-spec-generator"])
        .stdout(Stdio::piped())
        .spawn()?
        .wait_with_output()
        .context("Failed to run openapi-spec-generator binary")?;
        
    if !output.status.success() {
        anyhow::bail!("Failed to generate OpenAPI spec. Error: {}", String::from_utf8_lossy(&output.stderr));
    }

    fs::write(&spec_path, output.stdout)?;
    println!("✅ OpenAPI specification generated at: {}", spec_path.display());
    Ok(spec_path)
}

/// Handler for the `api-cli` command.
fn api_cli(args: Vec<String>) -> Result<()> {
    let spec_path = generate_openapi_spec()?;

    let cli_tool = "oas"; // The binary name for `oas-cli`
    println!("▶️  Invoking `{}` with the generated spec...", cli_tool);

    let status = Command::new(cli_tool)
        .arg(spec_path)
        .args(args)
        .status()
        .context(format!("Failed to execute '{}'. Is it installed and in your PATH? Try `npm install -g oas-cli`", cli_tool))?;

    if !status.success() {
        anyhow::bail!("API CLI command failed.");
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
