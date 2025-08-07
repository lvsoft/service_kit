//! # Forge Core - The core logic for the dynamic API CLI.
//!
//! This crate contains the shared logic for both the native `forge-api-cli`
//! and the `forge-cli-wasm` version. It handles OpenAPI spec parsing,
//! dynamic `clap` command generation, and request execution.

pub mod cli;
#[cfg(feature = "client")]
pub mod client;
pub mod error;
pub mod wasm_completer;
// `completer` and `repl` are now part of the native CLI.

use clap::{CommandFactory, Parser, Subcommand};
use error::Result;

/// A dynamic, OpenAPI-driven CLI client and REPL.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Cli {
    /// The base URL of the service.
    /// Can also be set via the `API_URL` environment variable.
    #[arg(short, long, global = true, env = "API_URL")]
    pub url: Option<String>, // Make it optional to satisfy clap's rule

    /// The API command to execute.
    /// If no command is provided, starts an interactive REPL session.
    #[command(subcommand)]
    pub command: Option<ApiCommand>,
}

/// Holds the external command arguments.
#[derive(Subcommand, Debug)]
pub enum ApiCommand {
    #[command(external_subcommand)]
    External(Vec<String>),
}

/// The main entry point for the `forge-api-cli` library.
/// This function is intended for the native CLI.
#[cfg(feature = "client")]
#[tokio::main]
pub async fn run() -> Result<()> {
    let cli = Cli::parse();
    run_with_cli_args(cli).await
}

/// Run the API CLI with specific arguments.
/// This function is also intended for the native CLI.
#[cfg(feature = "client")]
pub async fn run_with_args(args: Vec<String>) -> Result<()> {
    let mut full_args = vec!["forge-api-cli".to_string()];
    full_args.extend(args);
    
    let cli = Cli::try_parse_from(full_args)?;
    run_with_cli_args(cli).await
}

/// The core logic that processes parsed CLI arguments.
/// This is where the REPL vs. Direct Command logic would be,
/// but since REPL is native-only, it will be handled by the caller (`service_kit`).
#[cfg(feature = "client")]
pub async fn run_with_cli_args(cli: Cli) -> Result<()> {
    let url = match cli.url {
        Some(url) => url,
        None => {
            Cli::command().print_help()?;
            eprintln!("\n\nError: Missing required argument --url <URL> or API_URL environment variable.");
            return Ok(());
        }
    };
    
    let spec = client::fetch_openapi_spec(&url).await?;
    let mut full_cli = cli::build_cli_from_spec(&spec);

    match cli.command {
        Some(ApiCommand::External(args)) => {
            let mut full_args = vec!["forge-api-cli".to_string()];
            full_args.extend(args);
            
            let matches = full_cli.try_get_matches_from_mut(&full_args)?;
            if let Some((subcommand_name, subcommand_matches)) = matches.subcommand() {
                client::execute_request(&url, subcommand_name, subcommand_matches, &spec).await?;
            } else {
                println!("Error: Invalid subcommand provided.\n");
                full_cli.print_help()?;
            }
        },
        // The caller (native CLI) is now responsible for handling the `None` case
        // and starting the REPL if desired.
        None => {
            // We'll signal to the caller that it's time for the REPL.
            // For now, let's print a message. A more robust solution might
            // involve a specific return type.
            println!("No command provided. The native CLI should start the REPL now.");
        }
    }

    Ok(())
}
