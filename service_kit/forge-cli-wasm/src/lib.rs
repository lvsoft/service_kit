use wasm_bindgen::prelude::*;
use std::sync::Mutex;
use once_cell::sync::Lazy;
use clap::Command;
use oas::{OpenAPIV3, Referenceable};
use std::collections::{VecDeque, HashMap};
use serde_json::Value;

// We use a global static variable to hold the initialized CLI command.
// This is safe in WASM's single-threaded environment.
static CLI_COMMAND: Lazy<Mutex<Option<Command>>> = Lazy::new(|| Mutex::new(None));
static SPEC: Lazy<Mutex<Option<OpenAPIV3>>> = Lazy::new(|| Mutex::new(None));
static BASE_URL: Lazy<Mutex<Option<String>>> = Lazy::new(|| Mutex::new(None));
static HISTORY: Lazy<Mutex<VecDeque<String>>> = Lazy::new(|| Mutex::new(VecDeque::new()));

// External bindings to JavaScript fetch API
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}


#[wasm_bindgen]
pub fn init_cli(spec_json: &str, base_url: &str) -> Result<(), JsValue> {
    // Deserialize the JSON spec.
    let spec: OpenAPIV3 = serde_json::from_str(spec_json)
        .map_err(|e| JsValue::from_str(&format!("Spec Deserialization Error: {}", e)))?;
    
    // Build the clap command from the spec using the core logic.
    let command = service_kit::cli::build_cli_from_spec(&spec);
    
    // Store the command, spec, and base URL in our global static variables.
    *CLI_COMMAND.lock().unwrap() = Some(command);
    *SPEC.lock().unwrap() = Some(spec);
    *BASE_URL.lock().unwrap() = Some(base_url.to_string());

    Ok(())
}

// Helper function to execute HTTP requests using JavaScript fetch
async fn execute_request_wasm(
    base_url: &str,
    subcommand_name: &str,
    matches: &clap::ArgMatches,
    spec: &OpenAPIV3,
) -> Result<String, JsValue> {
    let parts: Vec<&str> = subcommand_name.split('.').collect();
    let Some(method_seg) = parts.last() else {
        return Err(JsValue::from_str("Invalid subcommand name"));
    };
    let method_str = method_seg.to_uppercase();
    
    // Resolve path template from command segments robustly
    if parts.len() < 2 {
        return Err(JsValue::from_str("Invalid subcommand name"));
    }
    let command_segments: Vec<&str> = parts[..parts.len() - 1].to_vec();
    let candidate = spec
        .paths
        .iter()
        .find(|(key, _)| {
            // Split key into segments excluding leading empty segment
            let key_segs: Vec<&str> = key.split('/')
                .filter(|s| !s.is_empty())
                .collect();
            if key_segs.len() != command_segments.len() {
                return false;
            }
            // Match each segment: exact match or OpenAPI placeholder {name}
            for (ks, cs) in key_segs.iter().zip(command_segments.iter()) {
                let is_param = ks.starts_with('{') && ks.ends_with('}');
                if !is_param && ks != cs {
                    return false;
                }
            }
            true
        });
    let (path_template, path_item) = match candidate {
        Some(found) => found,
        None => {
            // 不要 panic，返回明确错误，供前端 fallback
            let guess = format!("/{}", command_segments.join("/"));
            return Err(JsValue::from_str(&format!("Path not found for {}", guess)));
        }
    };

    let operation = match method_str.as_str() {
        "GET" => path_item.get.as_ref(),
        "POST" => path_item.post.as_ref(),
        "PUT" => path_item.put.as_ref(),
        "DELETE" => path_item.delete.as_ref(),
        "PATCH" => path_item.patch.as_ref(),
        _ => None,
    }
    .ok_or_else(|| JsValue::from_str(&format!("Operation not found for {}", subcommand_name)))?;

    let mut final_path = path_template.clone();
    let mut query_params = HashMap::new();

    // Process parameters
    if let Some(params) = &operation.parameters {
        for param_ref in params {
            match param_ref {
                Referenceable::Data(param) => {
                    if let Some(value) = matches.get_one::<String>(&param.name) {
                        match param._in {
                            oas::ParameterIn::Path => {
                                final_path = final_path.replace(&format!("{{{}}}", param.name), value);
                            }
                            oas::ParameterIn::Query => {
                                query_params.insert(param.name.clone(), value.clone());
                            }
                            _ => {}
                        }
                    }
                }
                _ => { /* ignore other variants for wasm */ }
            }
        }
    }

    // Handle OpenAPI server configuration
    let server_url = if let Some(servers) = &spec.servers {
        if let Some(first_server) = servers.first() {
            &first_server.url
        } else {
            ""
        }
    } else {
        ""
    };
    
    let mut request_url = format!("{}{}{}", base_url, server_url, final_path);
    if !query_params.is_empty() {
        let query_string = serde_urlencoded::to_string(query_params)
            .map_err(|e| JsValue::from_str(&format!("Query encoding error: {}", e)))?;
        request_url.push('?');
        request_url.push_str(&query_string);
    }

    log(&format!("--> Making {} request to: {}", method_str, request_url));

    // Create fetch request
    let mut init = web_sys::RequestInit::new();
    init.set_method(&method_str);

    // Add request body if needed
    if let Some(Referenceable::Data(request_body)) = &operation.request_body {
        if request_body.content.contains_key("application/json") {
            if let Some(body_str) = matches.get_one::<String>("body") {
                let json_body: Value = serde_json::from_str(body_str)
                    .map_err(|e| JsValue::from_str(&format!("JSON parse error: {}", e)))?;
                let body_string = serde_json::to_string(&json_body)
                    .map_err(|e| JsValue::from_str(&format!("JSON stringify error: {}", e)))?;
                init.set_body(&JsValue::from_str(&body_string));
                
                let headers = web_sys::Headers::new().unwrap();
                headers.set("Content-Type", "application/json").unwrap();
                init.set_headers(&headers);
            }
        }
    }

    let request = web_sys::Request::new_with_str_and_init(&request_url, &init)
        .map_err(|e| JsValue::from_str(&format!("Request creation error: {:?}", e)))?;

    // Get the global window and use fetch from it
    let window = web_sys::window().unwrap();
    let resp_value = wasm_bindgen_futures::JsFuture::from(window.fetch_with_request(&request)).await?;
    let response: web_sys::Response = resp_value.dyn_into().map_err(|_| JsValue::from_str("Invalid fetch response"))?;

    let status = response.status();
    log(&format!("<-- Response Status: {}", status));

    let text_promise = response.text()
        .map_err(|e| JsValue::from_str(&format!("Text conversion error: {:?}", e)))?;
    
    let text_value = wasm_bindgen_futures::JsFuture::from(text_promise).await?;
    let response_body = text_value.as_string().unwrap_or_default();

    // Try to format as JSON if possible
    if let Ok(json_body) = serde_json::from_str::<Value>(&response_body) {
        match serde_json::to_string_pretty(&json_body) {
            Ok(pretty) => Ok(pretty),
            Err(_) => Ok(response_body),
        }
    } else {
        Ok(response_body)
    }
}

// New async version that actually executes API requests
#[wasm_bindgen]
pub async fn run_command_async(command_line: &str) -> JsValue {
    let mut cli_command_guard = CLI_COMMAND.lock().unwrap();
    let spec_guard = SPEC.lock().unwrap();
    let base_url_guard = BASE_URL.lock().unwrap();
    let mut history_guard = HISTORY.lock().unwrap();

    // Ensure the CLI has been initialized.
    let cli_command = match &mut *cli_command_guard {
        Some(cmd) => cmd,
        None => return JsValue::from_str("Error: CLI not initialized. Call init_cli(spec, base_url) first."),
    };
    
    let spec = match &*spec_guard {
        Some(s) => s,
        None => return JsValue::from_str("Error: Spec not initialized."),
    };

    let base_url = match &*base_url_guard {
        Some(url) => url,
        None => return JsValue::from_str("Error: Base URL not initialized."),
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
        None => return JsValue::from_str("Error: Invalid command line input."),
    };

    // Prepend a program name for clap.
    let mut full_args = vec!["forge-api-cli".to_string()];
    full_args.extend(args);

    // Configure clap for better terminal output
    let mut clap_cmd = cli_command.clone();
    clap_cmd = clap_cmd.term_width(80); // Set a consistent terminal width
    
    // Try to get matches and execute the API request
    match clap_cmd.try_get_matches_from(&full_args) {
        Ok(matches) => {
            if let Some((subcommand, sub_matches)) = matches.subcommand() {
                // Actually execute the API request
                match execute_request_wasm(base_url, subcommand, sub_matches, spec).await {
                    Ok(response) => JsValue::from_str(&response),
                    Err(e) => JsValue::from_str(&format!("API request failed: {:?}", e)),
                }
            } else {
                JsValue::from_str("No subcommand was matched.")
            }
        },
        Err(e) => {
            // Format the clap error for display in terminal
            let error_str = e.to_string();
            // Ensure proper line endings for terminal display
            JsValue::from_str(&error_str.replace('\n', "\r\n"))
        }
    }
}

// Keep the old synchronous function for backwards compatibility
#[wasm_bindgen]
pub fn run_command(_command_line: &str) -> String {
    "Error: This function is deprecated. Use run_command_async() instead.".to_string()
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
    use service_kit::wasm_completer::WasmCompleter;
    
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
