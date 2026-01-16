//! Configuration loading for the CALIBER TUI.
//!
//! All fields are required unless explicitly marked optional. No defaults.

use serde::Deserialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TuiConfig {
    pub api_base_url: String,
    pub grpc_endpoint: String,
    pub ws_endpoint: String,
    pub tenant_id: uuid::Uuid,
    pub auth: AuthConfig,
    pub request_timeout_ms: u64,
    pub refresh_interval_ms: u64,
    pub persistence_path: PathBuf,
    pub error_log_path: PathBuf,
    pub theme: ThemeConfig,
    pub reconnect: ReconnectConfig,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthConfig {
    pub api_key: Option<String>,
    pub jwt: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ThemeConfig {
    pub name: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReconnectConfig {
    pub initial_ms: u64,
    pub max_ms: u64,
    pub multiplier: f64,
    pub jitter_ms: u64,
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("Missing configuration file path (use --config or CALIBER_TUI_CONFIG)")]
    MissingConfigPath,
    #[error("Failed to read config file: {0}")]
    Io(#[from] std::io::Error),
    #[error("Failed to parse config TOML: {0}")]
    Parse(#[from] toml::de::Error),
    #[error("Invalid config value for {field}: {reason}")]
    InvalidValue { field: &'static str, reason: String },
}

impl TuiConfig {
    pub fn load() -> Result<Self, ConfigError> {
        let path = config_path_from_args().or_else(config_path_from_env);
        let path = path.ok_or(ConfigError::MissingConfigPath)?;
        let config = Self::from_path(&path)?;
        config.validate()?;
        Ok(config)
    }

    pub fn from_path(path: &Path) -> Result<Self, ConfigError> {
        let contents = std::fs::read_to_string(path)?;
        let config: TuiConfig = toml::from_str(&contents)?;
        Ok(config)
    }

    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.api_base_url.trim().is_empty() {
            return Err(ConfigError::InvalidValue {
                field: "api_base_url",
                reason: "must not be empty".to_string(),
            });
        }
        if self.grpc_endpoint.trim().is_empty() {
            return Err(ConfigError::InvalidValue {
                field: "grpc_endpoint",
                reason: "must not be empty".to_string(),
            });
        }
        if self.ws_endpoint.trim().is_empty() {
            return Err(ConfigError::InvalidValue {
                field: "ws_endpoint",
                reason: "must not be empty".to_string(),
            });
        }
        if self.auth.api_key.is_none() && self.auth.jwt.is_none() {
            return Err(ConfigError::InvalidValue {
                field: "auth",
                reason: "api_key or jwt must be provided".to_string(),
            });
        }
        if self.request_timeout_ms == 0 {
            return Err(ConfigError::InvalidValue {
                field: "request_timeout_ms",
                reason: "must be > 0".to_string(),
            });
        }
        if self.refresh_interval_ms == 0 {
            return Err(ConfigError::InvalidValue {
                field: "refresh_interval_ms",
                reason: "must be > 0".to_string(),
            });
        }
        if self.theme.name.trim().is_empty() {
            return Err(ConfigError::InvalidValue {
                field: "theme.name",
                reason: "must not be empty".to_string(),
            });
        }
        if self.persistence_path.as_os_str().is_empty() {
            return Err(ConfigError::InvalidValue {
                field: "persistence_path",
                reason: "must not be empty".to_string(),
            });
        }
        if self.error_log_path.as_os_str().is_empty() {
            return Err(ConfigError::InvalidValue {
                field: "error_log_path",
                reason: "must not be empty".to_string(),
            });
        }
        if self.theme.name.to_ascii_lowercase() != "synthbrute" {
            return Err(ConfigError::InvalidValue {
                field: "theme.name",
                reason: "only 'synthbrute' is supported".to_string(),
            });
        }
        if self.reconnect.initial_ms == 0 {
            return Err(ConfigError::InvalidValue {
                field: "reconnect.initial_ms",
                reason: "must be > 0".to_string(),
            });
        }
        if self.reconnect.max_ms < self.reconnect.initial_ms {
            return Err(ConfigError::InvalidValue {
                field: "reconnect.max_ms",
                reason: "must be >= initial_ms".to_string(),
            });
        }
        if self.reconnect.multiplier < 1.0 {
            return Err(ConfigError::InvalidValue {
                field: "reconnect.multiplier",
                reason: "must be >= 1.0".to_string(),
            });
        }
        Ok(())
    }
}

fn config_path_from_env() -> Option<PathBuf> {
    std::env::var("CALIBER_TUI_CONFIG").ok().map(PathBuf::from)
}

fn config_path_from_args() -> Option<PathBuf> {
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--config" {
            return args.next().map(PathBuf::from);
        }
    }
    None
}
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TuiConfig {
    pub api_base_url: String,
    pub grpc_endpoint: String,
    pub ws_endpoint: String,
    pub auth: AuthConfig,
    pub reconnect: ReconnectConfig,
    pub refresh_interval_ms: u64,
    pub state_path: PathBuf,
    pub theme: ThemeName,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    pub api_key: Option<String>,
    pub bearer_token: Option<String>,
    pub tenant_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReconnectConfig {
    pub initial_ms: u64,
    pub max_ms: u64,
    pub multiplier: f64,
    pub jitter_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ThemeName {
    SynthBrute,
}

impl TuiConfig {
    pub fn load() -> Result<Self, String> {
        let path = Self::config_path().ok_or_else(|| {
            "Missing config path. Provide --config <path> or CALIBER_TUI_CONFIG".to_string()
        })?;

        let config = Self::from_path(&path)?;
        config.validate()?;
        Ok(config)
    }

    fn config_path() -> Option<PathBuf> {
        let mut args = env::args().skip(1);
        while let Some(arg) = args.next() {
            if arg == "--config" {
                return args.next().map(PathBuf::from);
            }
        }

        env::var("CALIBER_TUI_CONFIG").ok().map(PathBuf::from)
    }

    fn from_path(path: &Path) -> Result<Self, String> {
        let content = fs::read_to_string(path)
            .map_err(|e| format!("Failed to read config {}: {}", path.display(), e))?;

        serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse config {}: {}", path.display(), e))
    }

    fn validate(&self) -> Result<(), String> {
        if self.api_base_url.trim().is_empty() {
            return Err("api_base_url is required".to_string());
        }
        if self.grpc_endpoint.trim().is_empty() {
            return Err("grpc_endpoint is required".to_string());
        }
        if self.ws_endpoint.trim().is_empty() {
            return Err("ws_endpoint is required".to_string());
        }
        if self.refresh_interval_ms == 0 {
            return Err("refresh_interval_ms must be > 0".to_string());
        }
        if self.reconnect.initial_ms == 0 {
            return Err("reconnect.initial_ms must be > 0".to_string());
        }
        if self.reconnect.max_ms < self.reconnect.initial_ms {
            return Err("reconnect.max_ms must be >= reconnect.initial_ms".to_string());
        }
        if self.reconnect.multiplier < 1.0 {
            return Err("reconnect.multiplier must be >= 1.0".to_string());
        }
        if self.state_path.as_os_str().is_empty() {
            return Err("state_path is required".to_string());
        }
        Ok(())
    }
}
