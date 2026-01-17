//! API Configuration Module
//!
//! This module provides configuration for CORS, rate limiting, and other
//! production-level API settings. Configuration is loaded from environment
//! variables with sensible defaults for development.

use std::time::Duration;

// ============================================================================
// API CONFIGURATION
// ============================================================================

/// API configuration for CORS, rate limiting, and production hardening.
#[derive(Debug, Clone)]
pub struct ApiConfig {
    // ========================================================================
    // CORS Configuration
    // ========================================================================
    /// Allowed CORS origins (comma-separated in env var).
    /// Empty means allow all origins (dev mode).
    /// Example: "https://caliber.run,https://app.caliber.run"
    pub cors_origins: Vec<String>,

    /// Whether to allow credentials in CORS requests.
    pub cors_allow_credentials: bool,

    /// Max age for CORS preflight cache in seconds.
    pub cors_max_age_secs: u64,

    // ========================================================================
    // Rate Limiting Configuration
    // ========================================================================
    /// Whether rate limiting is enabled.
    pub rate_limit_enabled: bool,

    /// Rate limit for unauthenticated requests (per IP, per minute).
    pub rate_limit_unauthenticated: u32,

    /// Rate limit for authenticated requests (per tenant, per minute).
    pub rate_limit_authenticated: u32,

    /// Burst capacity (allow this many requests beyond the limit temporarily).
    pub rate_limit_burst: u32,

    /// Window size for rate limiting.
    pub rate_limit_window: Duration,
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            // CORS defaults: permissive for development
            cors_origins: Vec::new(), // Empty = allow all
            cors_allow_credentials: false,
            cors_max_age_secs: 86400, // 24 hours

            // Rate limiting defaults: enabled with reasonable limits
            rate_limit_enabled: true,
            rate_limit_unauthenticated: 100,  // 100 req/min per IP
            rate_limit_authenticated: 1000,    // 1000 req/min per tenant
            rate_limit_burst: 10,              // Allow 10 burst requests
            rate_limit_window: Duration::from_secs(60), // 1 minute window
        }
    }
}

impl ApiConfig {
    /// Create ApiConfig from environment variables.
    ///
    /// Environment variables:
    /// - `CALIBER_CORS_ORIGINS`: Comma-separated allowed origins (empty = allow all)
    /// - `CALIBER_CORS_ALLOW_CREDENTIALS`: "true" or "false" (default: false)
    /// - `CALIBER_CORS_MAX_AGE_SECS`: Preflight cache duration (default: 86400)
    /// - `CALIBER_RATE_LIMIT_ENABLED`: "true" or "false" (default: true)
    /// - `CALIBER_RATE_LIMIT_UNAUTHENTICATED`: Requests per minute per IP (default: 100)
    /// - `CALIBER_RATE_LIMIT_AUTHENTICATED`: Requests per minute per tenant (default: 1000)
    /// - `CALIBER_RATE_LIMIT_BURST`: Burst capacity (default: 10)
    pub fn from_env() -> Self {
        let cors_origins = std::env::var("CALIBER_CORS_ORIGINS")
            .ok()
            .map(|s| {
                s.split(',')
                    .map(|o| o.trim().to_string())
                    .filter(|o| !o.is_empty())
                    .collect()
            })
            .unwrap_or_default();

        let cors_allow_credentials = std::env::var("CALIBER_CORS_ALLOW_CREDENTIALS")
            .ok()
            .map(|s| s.to_lowercase() == "true")
            .unwrap_or(false);

        let cors_max_age_secs = std::env::var("CALIBER_CORS_MAX_AGE_SECS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(86400);

        let rate_limit_enabled = std::env::var("CALIBER_RATE_LIMIT_ENABLED")
            .ok()
            .map(|s| s.to_lowercase() != "false")
            .unwrap_or(true);

        let rate_limit_unauthenticated = std::env::var("CALIBER_RATE_LIMIT_UNAUTHENTICATED")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(100);

        let rate_limit_authenticated = std::env::var("CALIBER_RATE_LIMIT_AUTHENTICATED")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(1000);

        let rate_limit_burst = std::env::var("CALIBER_RATE_LIMIT_BURST")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(10);

        Self {
            cors_origins,
            cors_allow_credentials,
            cors_max_age_secs,
            rate_limit_enabled,
            rate_limit_unauthenticated,
            rate_limit_authenticated,
            rate_limit_burst,
            rate_limit_window: Duration::from_secs(60),
        }
    }

    /// Check if running in production mode (strict CORS).
    pub fn is_production(&self) -> bool {
        !self.cors_origins.is_empty()
    }

    /// Check if a given origin is allowed.
    pub fn is_origin_allowed(&self, origin: &str) -> bool {
        if self.cors_origins.is_empty() {
            // Dev mode: allow all
            return true;
        }

        self.cors_origins.iter().any(|allowed| {
            // Exact match or wildcard subdomain match
            if allowed == origin {
                return true;
            }
            // Support wildcard subdomains: *.caliber.run
            if let Some(pattern) = allowed.strip_prefix("*.") {
                if let Some(origin_domain) = origin.strip_prefix("https://") {
                    return origin_domain.ends_with(pattern)
                        || origin_domain == pattern.strip_prefix('.').unwrap_or(pattern);
                }
            }
            false
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = ApiConfig::default();
        assert!(config.cors_origins.is_empty());
        assert!(!config.cors_allow_credentials);
        assert_eq!(config.cors_max_age_secs, 86400);
        assert!(config.rate_limit_enabled);
        assert_eq!(config.rate_limit_unauthenticated, 100);
        assert_eq!(config.rate_limit_authenticated, 1000);
        assert_eq!(config.rate_limit_burst, 10);
    }

    #[test]
    fn test_is_production() {
        let mut config = ApiConfig::default();
        assert!(!config.is_production());

        config.cors_origins = vec!["https://caliber.run".to_string()];
        assert!(config.is_production());
    }

    #[test]
    fn test_origin_allowed_dev_mode() {
        let config = ApiConfig::default();
        assert!(config.is_origin_allowed("https://anything.com"));
        assert!(config.is_origin_allowed("http://localhost:3000"));
    }

    #[test]
    fn test_origin_allowed_production() {
        let mut config = ApiConfig::default();
        config.cors_origins = vec![
            "https://caliber.run".to_string(),
            "https://app.caliber.run".to_string(),
        ];

        assert!(config.is_origin_allowed("https://caliber.run"));
        assert!(config.is_origin_allowed("https://app.caliber.run"));
        assert!(!config.is_origin_allowed("https://evil.com"));
        assert!(!config.is_origin_allowed("https://notcaliber.run"));
    }

    #[test]
    fn test_wildcard_subdomain() {
        let mut config = ApiConfig::default();
        config.cors_origins = vec!["*.caliber.run".to_string()];

        assert!(config.is_origin_allowed("https://app.caliber.run"));
        assert!(config.is_origin_allowed("https://api.caliber.run"));
        assert!(!config.is_origin_allowed("https://evil.com"));
    }
}
