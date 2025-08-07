#[cfg(not(target_arch = "wasm32"))]
use crate::error::{Error, Result};
#[cfg(not(target_arch = "wasm32"))]
use clap::ArgMatches;
#[cfg(not(target_arch = "wasm32"))]
use oas::OpenAPIV3;
#[cfg(not(target_arch = "wasm32"))]
use reqwest::Client;
#[cfg(not(target_arch = "wasm32"))]
use serde_json::Value;
#[cfg(not(target_arch = "wasm32"))]
use std::collections::HashMap;

#[cfg(not(target_arch = "wasm32"))]
pub async fn fetch_openapi_spec(base_url: &str) -> Result<OpenAPIV3> {
    let spec_url = format!("{}/api-docs/openapi.json", base_url.trim_end_matches('/'));
    println!("--> Fetching OpenAPI spec from: {}", spec_url);

    let response = reqwest::get(&spec_url).await?;
    if !response.status().is_success() {
        return Err(Error::SpecError(format!(
            "Failed to fetch spec, status: {}",
            response.status()
        )));
    }

    let spec: OpenAPIV3 = response.json().await?;
    Ok(spec)
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn execute_request(
    base_url: &str,
    subcommand_name: &str,
    matches: &ArgMatches,
    spec: &OpenAPIV3,
) -> Result<()> {
    let client = Client::new();

    let parts: Vec<&str> = subcommand_name.split('.').collect();
    let method_str = parts.last().unwrap().to_uppercase();
    let path_template = format!("/{}", parts[..parts.len() - 1].join("/"));

    let path_item = spec
        .paths
        .get(&path_template)
        .ok_or_else(|| Error::SpecError(format!("Path not found for {}", path_template)))?;

    let operation = match method_str.as_str() {
        "GET" => path_item.get.as_ref(),
        "POST" => path_item.post.as_ref(),
        "PUT" => path_item.put.as_ref(),
        "DELETE" => path_item.delete.as_ref(),
        "PATCH" => path_item.patch.as_ref(),
        _ => None,
    }
    .ok_or_else(|| Error::SpecError(format!("Operation not found for {}", subcommand_name)))?;

    let mut final_path = path_template.clone();
    let mut query_params = HashMap::new();

    if let Some(params) = &operation.parameters {
        for param_ref in params {
            if let oas::Referenceable::Data(param) = param_ref {
                if let Some(value) = matches.get_one::<String>(&param.name) {
                     match param._in {
                        oas::ParameterIn::Path => {
                            final_path = final_path.replace(&format!("{{{}}}", param.name), value);
                        }
                        oas::ParameterIn::Query => {
                            query_params.insert(param.name.clone(), value.clone());
                        }
                        _ => {} // TODO: Handle Header, Cookie
                    }
                }
            }
        }
    }
    
    let mut request_url = format!("{}{}", base_url, final_path);
    if !query_params.is_empty() {
        let query_string = serde_urlencoded::to_string(query_params).unwrap();
        request_url.push('?');
        request_url.push_str(&query_string);
    }

    println!("--> Making {} request to: {}", method_str, request_url);

    let mut request_builder = match method_str.as_str() {
        "GET" => client.get(&request_url),
        "POST" => client.post(&request_url),
        "PUT" => client.put(&request_url),
        "DELETE" => client.delete(&request_url),
        "PATCH" => client.patch(&request_url),
        _ => return Err(Error::SpecError(format!("Unsupported method {}", method_str))),
    };

    // Only try to access body parameter if the operation defines a request body
    if let Some(oas::Referenceable::Data(request_body)) = &operation.request_body {
        if request_body.content.contains_key("application/json") {
            if let Some(body_str) = matches.get_one::<String>("body") {
                let json_body: Value = serde_json::from_str(body_str)?;
                request_builder = request_builder.json(&json_body);
            }
        }
    }

    let response = request_builder.send().await?;
    let status = response.status();
    println!("<-- Response Status: {}", status);

    let response_body = response.text().await?;
    if let Ok(json_body) = serde_json::from_str::<Value>(&response_body) {
        println!("{}", serde_json::to_string_pretty(&json_body)?);
    } else {
        println!("{}", response_body);
    }

    Ok(())
}
