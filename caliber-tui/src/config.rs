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
    pub auth: ClientCredentials,
    pub request_timeout_ms: u64,
    pub refresh_interval_ms: u64,
    pub persistence_path: PathBuf,
    pub error_log_path: PathBuf,
    pub theme: ThemeConfig,
    pub reconnect: ReconnectConfig,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClientCredentials {
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
        if !self.theme.name.eq_ignore_ascii_case("synthbrute") {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    // ========================================================================
    // Valid Config Tests
    // ========================================================================

    fn valid_config_toml() -> &'static str {
        r#"
api_base_url = "http://localhost:8080"
grpc_endpoint = "http://localhost:50051"
ws_endpoint = "ws://localhost:8080/ws"
tenant_id = "00000000-0000-0000-0000-000000000000"
request_timeout_ms = 5000
refresh_interval_ms = 1000
persistence_path = "/tmp/caliber-tui"
error_log_path = "/tmp/caliber-tui/errors.log"

[auth]
api_key = "test-api-key"

[theme]
name = "synthbrute"

[reconnect]
initial_ms = 100
max_ms = 10000
multiplier = 2.0
jitter_ms = 50
"#
    }

    #[test]
    fn test_parse_valid_config() {
        let config: TuiConfig = toml::from_str(valid_config_toml()).unwrap();

        assert_eq!(config.api_base_url, "http://localhost:8080");
        assert_eq!(config.grpc_endpoint, "http://localhost:50051");
        assert_eq!(config.ws_endpoint, "ws://localhost:8080/ws");
        assert_eq!(config.request_timeout_ms, 5000);
        assert_eq!(config.refresh_interval_ms, 1000);
        assert_eq!(config.theme.name, "synthbrute");
        assert_eq!(config.auth.api_key, Some("test-api-key".to_string()));
        assert!(config.auth.jwt.is_none());
    }

    #[test]
    fn test_validate_valid_config() {
        let config: TuiConfig = toml::from_str(valid_config_toml()).unwrap();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_parse_config_with_jwt() {
        let toml = r#"
api_base_url = "http://localhost:8080"
grpc_endpoint = "http://localhost:50051"
ws_endpoint = "ws://localhost:8080/ws"
tenant_id = "00000000-0000-0000-0000-000000000000"
request_timeout_ms = 5000
refresh_interval_ms = 1000
persistence_path = "/tmp/caliber-tui"
error_log_path = "/tmp/caliber-tui/errors.log"

[auth]
jwt = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.test"

[theme]
name = "synthbrute"

[reconnect]
initial_ms = 100
max_ms = 10000
multiplier = 2.0
jitter_ms = 50
"#;
        let config: TuiConfig = toml::from_str(toml).unwrap();
        assert!(config.auth.api_key.is_none());
        assert!(config.auth.jwt.is_some());
        assert!(config.validate().is_ok());
    }

    // ========================================================================
    // Invalid Config Tests
    // ========================================================================

    #[test]
    fn test_validate_empty_api_base_url() {
        let toml = r#"
api_base_url = ""
grpc_endpoint = "http://localhost:50051"
ws_endpoint = "ws://localhost:8080/ws"
tenant_id = "00000000-0000-0000-0000-000000000000"
request_timeout_ms = 5000
refresh_interval_ms = 1000
persistence_path = "/tmp/caliber-tui"
error_log_path = "/tmp/caliber-tui/errors.log"

[auth]
api_key = "test"

[theme]
name = "synthbrute"

[reconnect]
initial_ms = 100
max_ms = 10000
multiplier = 2.0
jitter_ms = 50
"#;
        let config: TuiConfig = toml::from_str(toml).unwrap();
        let result = config.validate();
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, ConfigError::InvalidValue { field: "api_base_url", .. }));
    }

    #[test]
    fn test_validate_empty_grpc_endpoint() {
        let toml = r#"
api_base_url = "http://localhost:8080"
grpc_endpoint = "   "
ws_endpoint = "ws://localhost:8080/ws"
tenant_id = "00000000-0000-0000-0000-000000000000"
request_timeout_ms = 5000
refresh_interval_ms = 1000
persistence_path = "/tmp/caliber-tui"
error_log_path = "/tmp/caliber-tui/errors.log"

[auth]
api_key = "test"

[theme]
name = "synthbrute"

[reconnect]
initial_ms = 100
max_ms = 10000
multiplier = 2.0
jitter_ms = 50
"#;
        let config: TuiConfig = toml::from_str(toml).unwrap();
        let result = config.validate();
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, ConfigError::InvalidValue { field: "grpc_endpoint", .. }));
    }

    #[test]
    fn test_validate_missing_auth() {
        let toml = r#"
api_base_url = "http://localhost:8080"
grpc_endpoint = "http://localhost:50051"
ws_endpoint = "ws://localhost:8080/ws"
tenant_id = "00000000-0000-0000-0000-000000000000"
request_timeout_ms = 5000
refresh_interval_ms = 1000
persistence_path = "/tmp/caliber-tui"
error_log_path = "/tmp/caliber-tui/errors.log"

[auth]
# No api_key or jwt

[theme]
name = "synthbrute"

[reconnect]
initial_ms = 100
max_ms = 10000
multiplier = 2.0
jitter_ms = 50
"#;
        let config: TuiConfig = toml::from_str(toml).unwrap();
        let result = config.validate();
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, ConfigError::InvalidValue { field: "auth", .. }));
    }

    #[test]
    fn test_validate_zero_timeout() {
        let toml = r#"
api_base_url = "http://localhost:8080"
grpc_endpoint = "http://localhost:50051"
ws_endpoint = "ws://localhost:8080/ws"
tenant_id = "00000000-0000-0000-0000-000000000000"
request_timeout_ms = 0
refresh_interval_ms = 1000
persistence_path = "/tmp/caliber-tui"
error_log_path = "/tmp/caliber-tui/errors.log"

[auth]
api_key = "test"

[theme]
name = "synthbrute"

[reconnect]
initial_ms = 100
max_ms = 10000
multiplier = 2.0
jitter_ms = 50
"#;
        let config: TuiConfig = toml::from_str(toml).unwrap();
        let result = config.validate();
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, ConfigError::InvalidValue { field: "request_timeout_ms", .. }));
    }

    #[test]
    fn test_validate_zero_refresh_interval() {
        let toml = r#"
api_base_url = "http://localhost:8080"
grpc_endpoint = "http://localhost:50051"
ws_endpoint = "ws://localhost:8080/ws"
tenant_id = "00000000-0000-0000-0000-000000000000"
request_timeout_ms = 5000
refresh_interval_ms = 0
persistence_path = "/tmp/caliber-tui"
error_log_path = "/tmp/caliber-tui/errors.log"

[auth]
api_key = "test"

[theme]
name = "synthbrute"

[reconnect]
initial_ms = 100
max_ms = 10000
multiplier = 2.0
jitter_ms = 50
"#;
        let config: TuiConfig = toml::from_str(toml).unwrap();
        let result = config.validate();
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, ConfigError::InvalidValue { field: "refresh_interval_ms", .. }));
    }

    #[test]
    fn test_validate_invalid_theme() {
        let toml = r#"
api_base_url = "http://localhost:8080"
grpc_endpoint = "http://localhost:50051"
ws_endpoint = "ws://localhost:8080/ws"
tenant_id = "00000000-0000-0000-0000-000000000000"
request_timeout_ms = 5000
refresh_interval_ms = 1000
persistence_path = "/tmp/caliber-tui"
error_log_path = "/tmp/caliber-tui/errors.log"

[auth]
api_key = "test"

[theme]
name = "unknown_theme"

[reconnect]
initial_ms = 100
max_ms = 10000
multiplier = 2.0
jitter_ms = 50
"#;
        let config: TuiConfig = toml::from_str(toml).unwrap();
        let result = config.validate();
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, ConfigError::InvalidValue { field: "theme.name", .. }));
    }

    #[test]
    fn test_validate_invalid_reconnect_multiplier() {
        let toml = r#"
api_base_url = "http://localhost:8080"
grpc_endpoint = "http://localhost:50051"
ws_endpoint = "ws://localhost:8080/ws"
tenant_id = "00000000-0000-0000-0000-000000000000"
request_timeout_ms = 5000
refresh_interval_ms = 1000
persistence_path = "/tmp/caliber-tui"
error_log_path = "/tmp/caliber-tui/errors.log"

[auth]
api_key = "test"

[theme]
name = "synthbrute"

[reconnect]
initial_ms = 100
max_ms = 10000
multiplier = 0.5
jitter_ms = 50
"#;
        let config: TuiConfig = toml::from_str(toml).unwrap();
        let result = config.validate();
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, ConfigError::InvalidValue { field: "reconnect.multiplier", .. }));
    }

    #[test]
    fn test_validate_max_ms_less_than_initial() {
        let toml = r#"
api_base_url = "http://localhost:8080"
grpc_endpoint = "http://localhost:50051"
ws_endpoint = "ws://localhost:8080/ws"
tenant_id = "00000000-0000-0000-0000-000000000000"
request_timeout_ms = 5000
refresh_interval_ms = 1000
persistence_path = "/tmp/caliber-tui"
error_log_path = "/tmp/caliber-tui/errors.log"

[auth]
api_key = "test"

[theme]
name = "synthbrute"

[reconnect]
initial_ms = 1000
max_ms = 500
multiplier = 2.0
jitter_ms = 50
"#;
        let config: TuiConfig = toml::from_str(toml).unwrap();
        let result = config.validate();
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, ConfigError::InvalidValue { field: "reconnect.max_ms", .. }));
    }

    // ========================================================================
    // File Loading Tests
    // ========================================================================

    #[test]
    fn test_from_path_valid_file() {
        let mut file = NamedTempFile::new().unwrap();
        write!(file, "{}", valid_config_toml()).unwrap();

        let config = TuiConfig::from_path(file.path());
        assert!(config.is_ok());
    }

    #[test]
    fn test_from_path_nonexistent_file() {
        let result = TuiConfig::from_path(Path::new("/nonexistent/config.toml"));
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ConfigError::Io(_)));
    }

    #[test]
    fn test_from_path_invalid_toml() {
        let mut file = NamedTempFile::new().unwrap();
        write!(file, "this is not valid toml {{{{").unwrap();

        let result = TuiConfig::from_path(file.path());
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ConfigError::Parse(_)));
    }

    #[test]
    fn test_from_path_missing_required_field() {
        let mut file = NamedTempFile::new().unwrap();
        // Missing api_base_url
        write!(
            file,
            r#"
grpc_endpoint = "http://localhost:50051"
ws_endpoint = "ws://localhost:8080/ws"
tenant_id = "00000000-0000-0000-0000-000000000000"
request_timeout_ms = 5000
refresh_interval_ms = 1000
persistence_path = "/tmp/caliber-tui"
error_log_path = "/tmp/caliber-tui/errors.log"

[auth]
api_key = "test"

[theme]
name = "synthbrute"

[reconnect]
initial_ms = 100
max_ms = 10000
multiplier = 2.0
jitter_ms = 50
"#
        )
        .unwrap();

        let result = TuiConfig::from_path(file.path());
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ConfigError::Parse(_)));
    }

    // ========================================================================
    // ConfigError Display Tests
    // ========================================================================

    #[test]
    fn test_config_error_display_missing_path() {
        let err = ConfigError::MissingConfigPath;
        let msg = format!("{}", err);
        assert!(msg.contains("Missing configuration file path"));
    }

    #[test]
    fn test_config_error_display_invalid_value() {
        let err = ConfigError::InvalidValue {
            field: "timeout",
            reason: "must be positive".to_string(),
        };
        let msg = format!("{}", err);
        assert!(msg.contains("timeout"));
        assert!(msg.contains("must be positive"));
    }
}
