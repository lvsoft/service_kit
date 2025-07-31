use clap::{Parser, Subcommand, CommandFactory};
use crate::error::Result;

pub mod cli;
pub mod client;
pub mod completer;
pub mod error;
pub mod repl;

/// A dynamic, OpenAPI-driven CLI client and REPL.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Cli {
    /// The base URL of the service.
    /// Can also be set via the `API_URL` environment variable.
    #[arg(short, long, global = true, env = "API_URL")]
    url: Option<String>, // Make it optional to satisfy clap's rule

    /// The API command to execute.
    /// If no command is provided, starts an interactive REPL session.
    #[command(subcommand)]
    command: Option<ApiCommand>,
}

/// Holds the external command arguments.
#[derive(Subcommand, Debug)]
enum ApiCommand {
    #[command(external_subcommand)]
    External(Vec<String>),
}

/// The main entry point for the `forge-api-cli` library.
#[tokio::main]
pub async fn run() -> Result<()> {
    // Use clap's derive-based parser for robust argument handling.
    let cli = Cli::parse();
    
    // Manually check for the URL, as it cannot be both `global` and `required`.
    let url = match cli.url {
        Some(url) => url,
        None => {
            // If no URL was provided via --url or env var, print help and exit.
            Cli::command().print_help()?;
            eprintln!("\n\nError: Missing required argument --url <URL> or API_URL environment variable.");
            return Ok(());
        }
    };
    
    // Fetch the spec based on the provided URL.
    println!("--> Fetching OpenAPI spec from: {}/api-docs/openapi.json", &url);
    let spec = client::fetch_openapi_spec(&url).await?;

    // Dynamically build the full CLI with all the API commands from the spec.
    let mut full_cli = cli::build_cli_from_spec(&spec);

    match cli.command {
        // Direct command execution mode
        Some(ApiCommand::External(args)) => {
            // Prepend the program name to the args for clap parsing
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
        // Interactive (REPL) mode
        None => {
            repl::start_repl(&url, &spec).await?;
        }
    }

    Ok(())
}
