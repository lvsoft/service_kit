use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use toml::Value;

#[derive(Parser, Debug)]
#[command(author, version, about = "A custom build and task runner for the service_kit project.")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Generates TypeScript type definitions from Rust DTOs.
    GenerateTs,
}

fn main() -> Result<()> {
    let mut args: Vec<String> = env::args().collect();
    if args.get(1).map(|s| s.as_str()) == Some("forge") {
        args.remove(1);
    }
    
    let cli = Cli::parse_from(args);

    match cli.command {
        Commands::GenerateTs => generate_ts()?,
    }

    Ok(())
}

fn generate_ts() -> Result<()> {
    println!("▶️  Generating TypeScript types...");

    let project_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .context("Failed to get project root")?
        .to_path_buf();
    
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

/// Reads a workspace member's Cargo.toml and gets the ts_output_dir from metadata.
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
