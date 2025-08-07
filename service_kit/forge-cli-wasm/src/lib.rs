use wasm_bindgen::prelude::*;
use std::sync::Mutex;
use once_cell::sync::Lazy;
use clap::Command;
use oas::OpenAPIV3;
use std::collections::VecDeque;

// We use a global static variable to hold the initialized CLI command.
// This is safe in WASM's single-threaded environment.
static CLI_COMMAND: Lazy<Mutex<Option<Command>>> = Lazy::new(|| Mutex::new(None));
static SPEC: Lazy<Mutex<Option<OpenAPIV3>>> = Lazy::new(|| Mutex::new(None));
static HISTORY: Lazy<Mutex<VecDeque<String>>> = Lazy::new(|| Mutex::new(VecDeque::new()));


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
    let mut history_guard = HISTORY.lock().unwrap();

    // Ensure the CLI has been initialized.
    let cli_command = match &mut *cli_command_guard {
        Some(cmd) => cmd,
        None => return "Error: CLI not initialized. Call init_cli(spec) first.".to_string(),
    };
    
    let _spec = match &*spec_guard {
        Some(s) => s,
        None => return "Error: Spec not initialized.".to_string(),
    };

    // 添加到历史记录 (只记录非空命令)
    let trimmed_command = command_line.trim();
    if !trimmed_command.is_empty() && (history_guard.is_empty() || history_guard.back() != Some(&trimmed_command.to_string())) {
        history_guard.push_back(trimmed_command.to_string());
        // 限制历史记录长度
        if history_guard.len() > 1000 {
            history_guard.pop_front();
        }
    }

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

/// 补全建议的JSON表示（用于与JavaScript交互）
#[wasm_bindgen]
pub struct CompletionResult {
    suggestions: String, // JSON序列化的建议列表
}

#[wasm_bindgen]
impl CompletionResult {
    #[wasm_bindgen(getter)]
    pub fn suggestions(&self) -> String {
        self.suggestions.clone()
    }
}

/// 获取Tab补全建议
#[wasm_bindgen]
pub fn get_completions(line: &str, cursor_pos: usize) -> CompletionResult {
    use forge_core::wasm_completer::WasmCompleter;
    
    let cli_command_guard = CLI_COMMAND.lock().unwrap();
    
    let cli_command = match &*cli_command_guard {
        Some(cmd) => cmd,
        None => {
            return CompletionResult {
                suggestions: "[]".to_string(),
            };
        }
    };
    
    let completer = WasmCompleter::new(cli_command.clone());
    let suggestions = completer.complete(line, cursor_pos);
    
    // 将建议转换为JSON格式
    let json_suggestions: Vec<serde_json::Value> = suggestions.into_iter().map(|s| {
        serde_json::json!({
            "value": s.value,
            "description": s.description,
            "start_pos": s.start_pos,
            "end_pos": s.end_pos
        })
    }).collect();
    
    CompletionResult {
        suggestions: serde_json::to_string(&json_suggestions).unwrap_or_else(|_| "[]".to_string()),
    }
}

/// 获取历史记录
#[wasm_bindgen]
pub fn get_history() -> String {
    let history_guard = HISTORY.lock().unwrap();
    let history: Vec<&String> = history_guard.iter().collect();
    serde_json::to_string(&history).unwrap_or_else(|_| "[]".to_string())
}

/// 根据索引获取历史记录项 (0为最新，负数从后往前)
#[wasm_bindgen]
pub fn get_history_item(index: i32) -> Option<String> {
    let history_guard = HISTORY.lock().unwrap();
    if history_guard.is_empty() {
        return None;
    }
    
    let len = history_guard.len() as i32;
    let actual_index = if index < 0 {
        len + index
    } else {
        len - 1 - index
    };
    
    if actual_index >= 0 && actual_index < len {
        history_guard.get(actual_index as usize).cloned()
    } else {
        None
    }
}

/// 在历史记录中搜索 (类似Ctrl+r功能)
#[wasm_bindgen]
pub fn search_history(query: &str) -> String {
    let history_guard = HISTORY.lock().unwrap();
    let matches: Vec<&String> = history_guard.iter()
        .rev() // 从最新的开始搜索
        .filter(|item| item.contains(query))
        .take(10) // 限制结果数量
        .collect();
    
    serde_json::to_string(&matches).unwrap_or_else(|_| "[]".to_string())
}

/// 清空历史记录
#[wasm_bindgen]
pub fn clear_history() {
    let mut history_guard = HISTORY.lock().unwrap();
    history_guard.clear();
}
