use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;

fn main() {
    // 防止在自身依赖链中递归触发构建造成死锁/卡住
    if env::var("SERVICE_KIT_BUILDING_WASM").ok().as_deref() == Some("1") {
        println!("cargo:warning=Detected recursive wasm build, skipping in build.rs");
        return;
    }

    // 仅当显式开启时才进行 WASM 构建，避免 `cargo check` 等常规操作卡住
    if env::var("SERVICE_KIT_BUILD_WASM").ok().as_deref() != Some("1") {
        println!("cargo:warning=Skipping WASM build (set SERVICE_KIT_BUILD_WASM=1 to enable)");
        return;
    }

    // 当目标为 wasm32 时跳过，防止在 wasm 目标上再次触发构建
    if let Ok(target) = env::var("TARGET") {
        if target.contains("wasm32") {
            println!("cargo:warning=TARGET is wasm32; skipping WASM build to avoid recursion");
            return;
        }
    }

    // 监听源码变化以便需要时重新构建（不监听输出目录，避免无限触发）
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

    // 执行wasm-pack构建，并在子进程中标记递归环境变量
    let status = Command::new("wasm-pack")
        .arg("build")
        .arg("--target")
        .arg("web")
        .arg("--out-dir")
        .arg("../frontend-wasm-cli")
        .current_dir(wasm_project_dir)
        .env("SERVICE_KIT_BUILDING_WASM", "1")
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