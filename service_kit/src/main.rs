//! # Forge CLI - A built-in task runner for `service_kit`.
//!
//! This binary provides a set of commands to automate common development
//! and CI/CD tasks for services built with `service_kit`. It is intended to be
//! invoked via a local `cargo forge` alias in the generated service project.

use anyhow::{Context, Result};
use clap::{Args, Parser, Subcommand};
use std::env;
use std::path::PathBuf;
use std::process::Command;

#[cfg(feature = "api-cli")]
mod repl;

// This function now handles the full logic for the `api-cli` command,
// including parsing arguments and deciding whether to start the REPL.
#[cfg(feature = "api-cli")]
async fn api_cli(args: Vec<String>) -> Result<()> {
    use forge_core::Cli as ApiCli;
    use forge_core::ApiCommand;
    use clap::CommandFactory;

    let mut full_args = vec!["forge-api-cli".to_string()];
    full_args.extend(args);
    
    // Parse arguments using the struct from forge-core
    let cli_args = ApiCli::try_parse_from(full_args)?;
    
    // Extract the URL, or print help and exit if it's missing.
    let url = match &cli_args.url {
        Some(url) => url.clone(),
        None => {
            ApiCli::command().print_help()?;
            eprintln!("\n\nError: Missing required argument --url <URL> or API_URL environment variable.");
            return Ok(());
        }
    };

    // Decide whether to execute a direct command or start the REPL.
    match &cli_args.command {
        Some(ApiCommand::External(_)) => {
            // If there's a command, let forge-core handle it.
            forge_core::run_with_cli_args(cli_args).await?;
        },
        None => {
            // If there's no command, start the native REPL.
            let spec = forge_core::client::fetch_openapi_spec(&url).await?;
            repl::start_repl(&url, &spec).await?;
        }
    }
    Ok(())
}


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
    /// Generates TypeScript type definitions from an OpenAPI specification.
    GenerateTypes(GenerateTypesArgs),

    /// Lints the codebase using `cargo clippy`.
    Lint,

    /// Runs all unit and integration tests.
    Test,

    // Note: `api-cli` is handled manually before clap parsing,
    // so it doesn't appear here as a regular subcommand.
}

/// Arguments for the `generate-types` command.
#[derive(Args, Debug)]
struct GenerateTypesArgs {
    /// The path or URL to the OpenAPI v3 specification file.
    #[arg(short, long)]
    input: String,

    /// The path to the output TypeScript file.
    #[arg(short, long)]
    output: PathBuf,
}


#[tokio::main]
async fn main() -> Result<()> {
    let mut args: Vec<String> = env::args().collect();
    if args.get(1).map(|s| s.as_str()) == Some("forge") {
        args.remove(1);
    }

    // Manual dispatch for `api-cli` to ensure raw argument forwarding.
    if args.get(1).map(|s| s.as_str()) == Some("api-cli") {
        return api_cli(args.into_iter().skip(2).collect()).await;
    }

    // If not `api-cli`, parse with clap for the other commands.
    let cli = Cli::parse_from(args);

    match cli.command {
        Commands::GenerateTypes(args) => generate_types(args)?,
        Commands::Lint => lint()?,
        Commands::Test => test()?,
    }

    Ok(())
}

/// Handler for the `generate-types` command.
fn generate_types(args: GenerateTypesArgs) -> Result<()> {
    println!("▶️  Generating TypeScript types from OpenAPI spec...");
    println!("   Input: {}", args.input);
    println!("   Output: {}", args.output.display());

    let mut command = Command::new("npx");
    command
        .arg("openapi-typescript")
        .arg(&args.input)
        .arg("--output")
        .arg(&args.output)
        .arg("--enum");

    let status = command
        .status()
        .context("Failed to execute openapi-typescript. Make sure Node.js, npm, and openapi-typescript are installed and in your PATH.")?;

    if !status.success() {
        anyhow::bail!("openapi-typescript command failed.");
    }
    
    println!("✅ TypeScript types generated successfully.");
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
