use thiserror::Error;
use std::io;

#[derive(Error, Debug)]
#[allow(dead_code)]
pub enum Error {
    #[error("HTTP request failed: {0}")]
    RequestFailed(#[from] reqwest::Error),

    #[error("JSON parsing failed: {0}")]
    JsonParseFailed(#[from] serde_json::Error),

    #[error("Invalid URL: {0}")]
    InvalidUrl(String),

    #[error("OpenAPI spec error: {0}")]
    SpecError(String),

    #[error("CLI error: {0}")]
    CliError(#[from] clap::error::Error),
    
    #[error("IO error: {0}")]
    IoError(#[from] io::Error),
}

pub type Result<T> = std::result::Result<T, Error>;
