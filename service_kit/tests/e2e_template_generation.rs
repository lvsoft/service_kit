use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use toml::Value;

#[test]
fn test_template_generation_and_forge_test_pass() {
    // 1. Create a temporary directory for the new project
    let temp_dir = env::temp_dir().join("service_kit_e2e_test");
    if temp_dir.exists() {
        fs::remove_dir_all(&temp_dir).expect("Failed to clean up old temp dir");
    }
    fs::create_dir_all(&temp_dir).expect("Failed to create temp dir");

    let project_name = "test-service";
    let project_path = temp_dir.join(project_name);

    // 2. Run `cargo generate`
    let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf();
    let template_path = workspace_root.join("service-template");

    let generate_status = Command::new("cargo")
        .args([
            "generate",
            "--path",
            template_path.to_str().unwrap(),
            "--name",
            project_name,
            "--force",
            "--destination",
            temp_dir.to_str().unwrap(),
        ])
        .status()
        .expect("Failed to execute cargo generate");

    assert!(generate_status.success(), "Failed to generate project from template");

    // 3. Dynamically update the path to `service_kit` in the generated project's Cargo.toml
    let cargo_toml_path = project_path.join("Cargo.toml");
    let toml_content = fs::read_to_string(&cargo_toml_path).expect("Failed to read generated Cargo.toml");
    let mut toml_value: Value = toml::from_str(&toml_content).expect("Failed to parse generated Cargo.toml");

    let service_kit_path = workspace_root.join("service_kit");
    
    if let Some(deps) = toml_value.get_mut("dependencies") {
        if let Some(service_kit_dep) = deps.get_mut("service_kit") {
            if let Some(table) = service_kit_dep.as_table_mut() {
                table.insert("path".to_string(), Value::String(service_kit_path.to_str().unwrap().to_string()));
            }
        }
    }
    fs::write(&cargo_toml_path, toml::to_string(&toml_value).unwrap()).expect("Failed to write updated Cargo.toml");

    // 4. Run `cargo forge test` in the new project
    // We need to build `forge-cli` first in our main workspace so it's available.
    let build_status = Command::new("cargo")
        .arg("build")
        .arg("--bin")
        .arg("forge-cli")
        .arg("--features")
        .arg("cli")
        .status()
        .expect("Failed to build forge-cli");
    assert!(build_status.success(), "Failed to build forge-cli");

    // The path to our freshly built `forge-cli`
    let forge_cli_path = workspace_root.join("target/debug/forge-cli");

    let forge_test_output = Command::new(forge_cli_path)
        .arg("test")
        .current_dir(&project_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("Failed to execute 'forge-cli test' in generated project");

    let stdout = String::from_utf8_lossy(&forge_test_output.stdout);
    let stderr = String::from_utf8_lossy(&forge_test_output.stderr);
    println!("--- forge-cli stdout ---\n{}", stdout);
    println!("--- forge-cli stderr ---\n{}", stderr);

    assert!(forge_test_output.status.success(), "forge-cli test command failed in the generated project");
    assert!(stdout.contains("▶️  Running all tests..."));
    assert!(stdout.contains("✅ All tests passed."));

    // 5. Clean up
    fs::remove_dir_all(&temp_dir).expect("Failed to clean up temp dir after test");
}
