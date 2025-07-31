use clap::{Arg, Command};
use std::env;
use crate::error::Result;

pub mod cli;
pub mod client;
pub mod completer;
pub mod error;
pub mod repl;

/// The main entry point for the `forge-api-cli` library.
///
/// This function encapsulates the entire logic of the dynamic API client,
/// including the two-pass argument parsing, spec fetching, CLI building,
/// and mode dispatch (direct command vs. REPL).
#[tokio::main]
pub async fn run() -> Result<()> {
    // --- Pass 1: Get the URL ---
    // The first argument to the binary is expected to be the URL.
    let url_parser = Command::new("forge-api-cli-launcher")
        .disable_version_flag(true)
        .arg(
            Arg::new("URL")
                .help("The base URL of the service (e.g., http://127.0.0.1:8080)")
                .required(false) 
                .index(1),
        )
        .ignore_errors(true);
        
    // We parse from `env::args().skip(1)` because when run via `cargo run --bin api-cli`,
    // the first arg is the binary name, not the URL.
    let first_pass_matches = url_parser.get_matches_from(env::args().skip(1));
    let maybe_url = first_pass_matches.get_one::<String>("URL").map(|s| s.clone());

    let (url, spec) = if let Some(url_str) = maybe_url {
        println!("--> Fetching OpenAPI spec from: {}/api-docs/openapi.json", url_str);
        let spec = client::fetch_openapi_spec(&url_str).await?;
        (url_str, spec)
    } else {
        // If no URL is provided, show a help message explaining the usage.
        Command::new("forge-api-cli")
            .version("0.1.0")
            .about("A dynamic OpenAPI CLI client.")
            .long_about("Provide a URL to an OpenAPI spec to generate commands, or run in REPL mode.")
            .arg(
                Arg::new("URL")
                    .help("The base URL of the service (e.g., http://127.0.0.1:8080)")
                    .required(true)
                    .index(1),
            )
            .print_help()?;
        return Ok(());
    };

    let mut full_cli = cli::build_cli_from_spec(&spec);

    // --- Pass 2: Parse all arguments with the full CLI ---
    // We skip the binary name and the URL for the second pass.
    let cli_args: Vec<String> = env::args().skip(2).collect();

    if !cli_args.is_empty() {
         // Direct command execution
        let final_matches = full_cli.try_get_matches_from_mut(&cli_args)?;
         if let Some((subcommand_name, subcommand_matches)) = final_matches.subcommand() {
            client::execute_request(&url, subcommand_name, subcommand_matches, &spec).await?;
        } else {
            full_cli.print_help()?;
        }
    } else {
        // No subcommands provided, so start the REPL
        repl::start_repl(&url, &spec).await?;
    }

    Ok(())
}
