//! # Forge CLI - A built-in task runner for `service_kit`.
//!
//! This binary provides a set of commands to automate common development
//! and CI/CD tasks for services built with `service_kit`. It is intended to be
//! invoked via a local `cargo forge` alias in the generated service project.

use anyhow::{Context, Result};
use clap::{Args, Parser, Subcommand};
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[cfg(feature = "api-cli")]
mod repl;

// This function now handles the full logic for the `api-cli` command,
// including parsing arguments and deciding whether to start the REPL.
#[cfg(feature = "api-cli")]
async fn api_cli(args: Vec<String>) -> Result<()> {
    // Manual lightweight parsing to support forwarding unknown subcommands/args
    // Accept: --url <URL> or --url=<URL> or env API_URL
    let mut forwarded: Vec<String> = Vec::new();
    let mut iter = args.into_iter();
    let mut url_opt: Option<String> = std::env::var("API_URL").ok();
    while let Some(arg) = iter.next() {
        if arg == "--url" {
            if let Some(v) = iter.next() { url_opt = Some(v); }
        } else if let Some(rest) = arg.strip_prefix("--url=") {
            url_opt = Some(rest.to_string());
        } else {
            forwarded.push(arg);
        }
    }

    let url = match url_opt {
        Some(u) => u,
        None => {
            eprintln!("Usage: cargo forge api-cli --url <URL> [<subcommand> ...]\n       or set API_URL env var");
            return Ok(());
        }
    };

    let spec = service_kit::client::fetch_openapi_spec(&url).await?;

    if forwarded.is_empty() {
        // No subcommand provided: start REPL
        repl::start_repl(&url, &spec).await?;
        return Ok(());
    }

    // Pure CLI mode: build dynamic CLI from spec and execute once
    let command = service_kit::cli::build_cli_from_spec(&spec);
    let mut argv = vec!["forge-api-cli".to_string()];
    argv.extend(forwarded);
    match command.clone().try_get_matches_from(argv) {
        Ok(matches) => {
            if let Some((subcommand_name, subcommand_matches)) = matches.subcommand() {
                service_kit::client::execute_request(&url, subcommand_name, subcommand_matches, &spec).await?;
            } else {
                // If nothing matched, fall back to REPL
                repl::start_repl(&url, &spec).await?;
            }
        }
        Err(e) => {
            eprintln!("{}", e);
        }
    }
    Ok(())
}


/// The main CLI entry point for `cargo forge`.
#[derive(Parser, Debug)]
#[command(
    author,
    version,
    about = "A custom build and task runner for projects using service_kit.",
    after_help = r#"
Additional usage:

  api-cli (Interactive / Pure CLI OpenAPI client)
    - Start interactive REPL:
        cargo forge api-cli --url http://127.0.0.1:3000
    - Run a single GET endpoint directly:
        cargo forge api-cli --url http://127.0.0.1:3000 v1.hello.get
    - Run a single POST endpoint with JSON body:
        cargo forge api-cli --url http://127.0.0.1:3000 v1.add.post --body '{"a":1,"b":2}'

  generate-types (OpenAPI -> TypeScript)
    - Usage:
        cargo forge generate-types --input <URL_OR_PATH_TO_OPENAPI_JSON> --output <TS_FILE_PATH>
    - Example:
        cargo forge generate-types \
          --input http://127.0.0.1:3000/api-docs/openapi.json \
          --output src/frontend/types/api.ts
    - Note: requires Node.js with `npx` available; `openapi-typescript` will be run via `npx`.
"#
)]
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
    println!("   Running 'cargo clippy' on current package only with -D warnings...");
    let project_root = get_project_root()?;
    let manifest_path = project_root.join("Cargo.toml");
    let manifest_str = fs::read_to_string(&manifest_path)
        .context("Failed to read Cargo.toml in current directory")?;
    let manifest_value: toml::Value = toml::from_str(&manifest_str)
        .context("Failed to parse Cargo.toml")?;
    let package_name = manifest_value
        .get("package")
        .and_then(|p| p.get("name"))
        .and_then(|n| n.as_str())
        .map(|s| s.to_string())
        .context("`package.name` not found in Cargo.toml")?;

    let manifest_arg = manifest_path.display().to_string();
    run_cargo_command(
        &[
            "clippy",
            "--manifest-path",
            &manifest_arg,
            "-p",
            &package_name,
            "--no-deps",
            "--",
            "-D",
            "warnings",
        ],
        "Failed to run cargo clippy",
    )?;
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
