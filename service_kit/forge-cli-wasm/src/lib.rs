use wasm_bindgen::prelude::*;
use std::sync::Mutex;
use once_cell::sync::Lazy;
use clap::Command;
use oas::OpenAPIV3;

// We use a global static variable to hold the initialized CLI command.
// This is safe in WASM's single-threaded environment.
static CLI_COMMAND: Lazy<Mutex<Option<Command>>> = Lazy::new(|| Mutex::new(None));
static SPEC: Lazy<Mutex<Option<OpenAPIV3>>> = Lazy::new(|| Mutex::new(None));


#[wasm_bindgen]
pub fn init_cli(spec_json: &str) -> Result<(), JsValue> {
    // Deserialize the JSON spec.
    let spec: OpenAPIV3 = serde_json::from_str(spec_json)
        .map_err(|e| JsValue::from_str(&format!("Spec Deserialization Error: {}", e)))?;
    
    // Build the clap command from the spec using the core logic.
    let command = forge_core::cli::build_cli_from_spec(&spec);
    
    // Store the command in our global static variable.
    *CLI_COMMAND.lock().unwrap() = Some(command);
    *SPEC.lock().unwrap() = Some(spec);

    Ok(())
}

#[wasm_bindgen]
pub fn run_command(command_line: &str) -> String {
    let mut cli_command_guard = CLI_COMMAND.lock().unwrap();
    let spec_guard = SPEC.lock().unwrap();

    // Ensure the CLI has been initialized.
    let cli_command = match &mut *cli_command_guard {
        Some(cmd) => cmd,
        None => return "Error: CLI not initialized. Call init_cli(spec) first.".to_string(),
    };
    
    let _spec = match &*spec_guard {
        Some(s) => s,
        None => return "Error: Spec not initialized.".to_string(),
    };

    // Parse the command line.
    let args = match shlex::split(command_line) {
        Some(args) => args,
        None => return "Error: Invalid command line input.".to_string(),
    };

    // Prepend a program name for clap.
    let mut full_args = vec!["forge-api-cli".to_string()];
    full_args.extend(args);

    // Configure clap for better terminal output
    let mut clap_cmd = cli_command.clone();
    clap_cmd = clap_cmd.term_width(80); // Set a consistent terminal width
    
    // Try to get matches. 
    match clap_cmd.try_get_matches_from(&full_args) {
        Ok(matches) => {
            // In a real async environment, we'd call `execute_request` here.
            // For now, we'll just return the matched subcommand for verification.
            if let Some((subcommand, _)) = matches.subcommand() {
                format!("Successfully matched command: {}", subcommand)
            } else {
                "No subcommand was matched.".to_string()
            }
        },
        Err(e) => {
            // Format the clap error for display in terminal
            let error_str = e.to_string();
            // Ensure proper line endings for terminal display
            error_str.replace('\n', "\r\n")
        }
    }
}
