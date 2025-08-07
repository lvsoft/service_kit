use std::process::Command;
use std::path::Path;
use std::fs;

fn main() {
    // 告诉Cargo当WASM源码变化时重新运行构建脚本
    println!("cargo:rerun-if-changed=forge-cli-wasm/src");
    println!("cargo:rerun-if-changed=forge-cli-wasm/Cargo.toml");
    
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
            } else {
                println!("cargo:warning=WASM build failed with exit code: {:?}", status.code());
            }
        }
        Err(e) => {
            println!("cargo:warning=Failed to execute wasm-pack: {}", e);
        }
    }
}

fn is_wasm_pack_available() -> bool {
    Command::new("wasm-pack")
        .arg("--version")
        .output()
        .is_ok()
}