//! Error types for the TUI.

use crate::api_client::ApiClientError;
use crate::config::ConfigError;

#[derive(Debug, thiserror::Error)]
pub enum TuiError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Config(#[from] ConfigError),
    #[error(transparent)]
    Api(#[from] ApiClientError),
}
