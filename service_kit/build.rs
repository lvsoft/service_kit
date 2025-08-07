use std::process::Command;
use std::path::Path;
use std::fs;

fn main() {
    // 告诉Cargo当WASM源码变化时重新运行构建脚本
    println!("cargo:rerun-if-changed=forge-cli-wasm/src");
    println!("cargo:rerun-if-changed=forge-cli-wasm/Cargo.toml");
    println!("cargo:rerun-if-changed=frontend-wasm-cli");
    
    // 检查wasm-pack是否可用
    if !is_wasm_pack_available() {
        println!("cargo:warning=wasm-pack not found, skipping WASM build. Install with: curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh");
        return;
    }

    let wasm_project_dir = Path::new("forge-cli-wasm");
    let output_dir = Path::new("frontend-wasm-cli");
    
    // 确保输出目录存在
    if let Err(e) = fs::create_dir_all(output_dir) {
        println!("cargo:warning=Failed to create output directory: {}", e);
        return;
    }

    println!("cargo:warning=Building WASM project...");
    
    // 执行wasm-pack构建
    let status = Command::new("wasm-pack")
        .arg("build")
        .arg("--target")
        .arg("web")
        .arg("--out-dir")
        .arg("../frontend-wasm-cli")
        .current_dir(wasm_project_dir)
        .status();

    match status {
        Ok(status) => {
            if status.success() {
                println!("cargo:warning=WASM build completed successfully");
                
                // 同步WASM文件到service-template
                sync_wasm_to_template();
            } else {
                println!("cargo:warning=WASM build failed with exit code: {:?}", status.code());
            }
        }
        Err(e) => {
            println!("cargo:warning=Failed to execute wasm-pack: {}", e);
        }
    }
}

fn sync_wasm_to_template() {
    let source_dir = Path::new("frontend-wasm-cli");
    let target_dir = Path::new("../service-template/assets");
    
    // 确保目标目录存在
    if let Err(e) = fs::create_dir_all(target_dir) {
        println!("cargo:warning=Failed to create template assets directory: {}", e);
        return;
    }
    
    // 需要同步的文件列表
    let files_to_sync = [
        "forge_cli_wasm.js",
        "forge_cli_wasm_bg.wasm", 
        "index.html",
        "main.js",
        "style.css",
        "package.json",
        "README.md",
    ];
    
    for file in &files_to_sync {
        let source_path = source_dir.join(file);
        let target_path = target_dir.join(file);
        
        if source_path.exists() {
            if let Err(e) = fs::copy(&source_path, &target_path) {
                println!("cargo:warning=Failed to copy {}: {}", file, e);
            } else {
                println!("cargo:warning=Synced {} to template", file);
            }
        }
    }
}

fn is_wasm_pack_available() -> bool {
    Command::new("wasm-pack")
        .arg("--version")
        .output()
        .is_ok()
}