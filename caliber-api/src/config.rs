//! API Configuration Module
//!
//! This module provides configuration for CORS, rate limiting, endpoints,
//! and other production-level API settings. Configuration is loaded from
//! environment variables with sensible defaults for development.

use crate::constants::{
    DEFAULT_CONTEXT_MAX_ARTIFACTS, DEFAULT_CONTEXT_MAX_NOTES, DEFAULT_CONTEXT_MAX_SUMMARIES,
    DEFAULT_CONTEXT_MAX_TURNS, DEFAULT_CORS_MAX_AGE_SECS, DEFAULT_GRAPHQL_TOKEN_BUDGET,
    DEFAULT_IDEMPOTENCY_MAX_BODY_SIZE, DEFAULT_IDEMPOTENCY_TTL_SECS, DEFAULT_RATE_LIMIT_AUTHENTICATED,
    DEFAULT_RATE_LIMIT_BURST, DEFAULT_RATE_LIMIT_UNAUTHENTICATED, DEFAULT_REST_TOKEN_BUDGET,
    DEFAULT_WEBHOOK_SIGNATURE_TOLERANCE_SECS,
};
use std::time::Duration;

// ============================================================================
// ENDPOINTS CONFIGURATION
// ============================================================================

/// Configuration for external service URLs and endpoints.
///
/// This struct centralizes all external URLs used by the API, making them
/// configurable via environment variables with sensible defaults for local
/// development.
#[derive(Debug, Clone)]
pub struct EndpointsConfig {
    // ========================================================================
    // Caliber URLs
    // ========================================================================
    /// Base URL for the Caliber API (e.g., "https://api.caliber.run").
    /// Used for generating callback URLs, webhook endpoints, etc.
    pub api_base_url: String,

    /// Main domain for Caliber (e.g., "https://caliber.run").
    /// Used for redirect URLs, dashboard links, etc.
    pub domain: String,

    /// Documentation URL (e.g., "https://docs.caliber.run").
    /// Used for linking to docs in API responses and error messages.
    pub docs_url: String,

    // ========================================================================
    // Third-Party Service URLs
    // ========================================================================
    /// LemonSqueezy API base URL.
    /// Default: "https://api.lemonsqueezy.com"
    pub lemonsqueezy_api_url: String,

    /// WorkOS API base URL.
    /// Default: "https://api.workos.com"
    pub workos_api_url: String,
}

impl Default for EndpointsConfig {
    fn default() -> Self {
        Self {
            // Development defaults - all point to localhost
            api_base_url: "http://localhost:3000".to_string(),
            domain: "http://localhost:3000".to_string(),
            docs_url: "http://localhost:3000/docs".to_string(),
            // Third-party services always use production URLs by default
            lemonsqueezy_api_url: "https://api.lemonsqueezy.com".to_string(),
            workos_api_url: "https://api.workos.com".to_string(),
        }
    }
}

impl EndpointsConfig {
    /// Create EndpointsConfig from environment variables.
    ///
    /// # Environment Variables
    /// - `CALIBER_API_BASE_URL`: Base URL for the Caliber API (default: http://localhost:3000)
    /// - `CALIBER_DOMAIN`: Main domain for Caliber (default: http://localhost:3000)
    /// - `CALIBER_DOCS_URL`: Documentation URL (default: http://localhost:3000/docs)
    /// - `CALIBER_LEMONSQUEEZY_API_URL`: LemonSqueezy API URL (default: https://api.lemonsqueezy.com)
    /// - `CALIBER_WORKOS_API_URL`: WorkOS API URL (default: https://api.workos.com)
    pub fn from_env() -> Self {
        let api_base_url = std::env::var("CALIBER_API_BASE_URL")
            .unwrap_or_else(|_| "http://localhost:3000".to_string());

        let domain = std::env::var("CALIBER_DOMAIN")
            .unwrap_or_else(|_| "http://localhost:3000".to_string());

        let docs_url = std::env::var("CALIBER_DOCS_URL")
            .unwrap_or_else(|_| "http://localhost:3000/docs".to_string());

        let lemonsqueezy_api_url = std::env::var("CALIBER_LEMONSQUEEZY_API_URL")
            .unwrap_or_else(|_| "https://api.lemonsqueezy.com".to_string());

        let workos_api_url = std::env::var("CALIBER_WORKOS_API_URL")
            .unwrap_or_else(|_| "https://api.workos.com".to_string());

        Self {
            api_base_url,
            domain,
            docs_url,
            lemonsqueezy_api_url,
            workos_api_url,
        }
    }

    /// Get the default redirect URL for billing success.
    /// Returns the dashboard settings page on the configured domain.
    pub fn billing_success_redirect(&self) -> String {
        format!("{}/dashboard/settings", self.domain)
    }

    /// Get the SSO callback URL for WorkOS.
    pub fn workos_callback_url(&self) -> String {
        format!("{}/auth/sso/callback", self.api_base_url)
    }

    /// Check if running in production mode (non-localhost URLs).
    pub fn is_production(&self) -> bool {
        !self.api_base_url.contains("localhost") && !self.api_base_url.contains("127.0.0.1")
    }
}

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
            cors_max_age_secs: DEFAULT_CORS_MAX_AGE_SECS,

            // Rate limiting defaults: enabled with reasonable limits (from constants.rs)
            rate_limit_enabled: true,
            rate_limit_unauthenticated: DEFAULT_RATE_LIMIT_UNAUTHENTICATED,
            rate_limit_authenticated: DEFAULT_RATE_LIMIT_AUTHENTICATED,
            rate_limit_burst: DEFAULT_RATE_LIMIT_BURST,
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
            .unwrap_or(DEFAULT_CORS_MAX_AGE_SECS);

        let rate_limit_enabled = std::env::var("CALIBER_RATE_LIMIT_ENABLED")
            .ok()
            .map(|s| s.to_lowercase() != "false")
            .unwrap_or(true);

        let rate_limit_unauthenticated = std::env::var("CALIBER_RATE_LIMIT_UNAUTHENTICATED")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(DEFAULT_RATE_LIMIT_UNAUTHENTICATED);

        let rate_limit_authenticated = std::env::var("CALIBER_RATE_LIMIT_AUTHENTICATED")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(DEFAULT_RATE_LIMIT_AUTHENTICATED);

        let rate_limit_burst = std::env::var("CALIBER_RATE_LIMIT_BURST")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(DEFAULT_RATE_LIMIT_BURST);

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

// ============================================================================
// CONTEXT CONFIGURATION
// ============================================================================

/// Configuration for context assembly operations.
///
/// These defaults are used when clients don't specify values in their requests.
#[derive(Debug, Clone)]
pub struct ContextConfig {
    /// Default token budget for REST endpoints
    pub rest_token_budget: i32,
    /// Default token budget for GraphQL endpoints
    pub graphql_token_budget: i32,
    /// Maximum number of notes to include by default
    pub max_notes: usize,
    /// Maximum number of artifacts to include by default
    pub max_artifacts: usize,
    /// Maximum number of conversation turns to include by default
    pub max_turns: usize,
    /// Maximum number of scope summaries to include by default
    pub max_summaries: usize,
}

impl Default for ContextConfig {
    fn default() -> Self {
        Self {
            rest_token_budget: DEFAULT_REST_TOKEN_BUDGET,
            graphql_token_budget: DEFAULT_GRAPHQL_TOKEN_BUDGET,
            max_notes: DEFAULT_CONTEXT_MAX_NOTES,
            max_artifacts: DEFAULT_CONTEXT_MAX_ARTIFACTS,
            max_turns: DEFAULT_CONTEXT_MAX_TURNS,
            max_summaries: DEFAULT_CONTEXT_MAX_SUMMARIES,
        }
    }
}

impl ContextConfig {
    /// Create from environment variables with fallback to defaults.
    ///
    /// Environment variables:
    /// - `CALIBER_CONTEXT_REST_TOKEN_BUDGET`: Default token budget for REST (default: 8000)
    /// - `CALIBER_CONTEXT_GRAPHQL_TOKEN_BUDGET`: Default token budget for GraphQL (default: 4096)
    /// - `CALIBER_CONTEXT_MAX_NOTES`: Maximum notes to include (default: 10)
    /// - `CALIBER_CONTEXT_MAX_ARTIFACTS`: Maximum artifacts to include (default: 5)
    /// - `CALIBER_CONTEXT_MAX_TURNS`: Maximum turns to include (default: 20)
    /// - `CALIBER_CONTEXT_MAX_SUMMARIES`: Maximum summaries to include (default: 5)
    pub fn from_env() -> Self {
        let defaults = Self::default();

        Self {
            rest_token_budget: std::env::var("CALIBER_CONTEXT_REST_TOKEN_BUDGET")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(defaults.rest_token_budget),
            graphql_token_budget: std::env::var("CALIBER_CONTEXT_GRAPHQL_TOKEN_BUDGET")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(defaults.graphql_token_budget),
            max_notes: std::env::var("CALIBER_CONTEXT_MAX_NOTES")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(defaults.max_notes),
            max_artifacts: std::env::var("CALIBER_CONTEXT_MAX_ARTIFACTS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(defaults.max_artifacts),
            max_turns: std::env::var("CALIBER_CONTEXT_MAX_TURNS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(defaults.max_turns),
            max_summaries: std::env::var("CALIBER_CONTEXT_MAX_SUMMARIES")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(defaults.max_summaries),
        }
    }
}

// ============================================================================
// WEBHOOK CONFIGURATION
// ============================================================================

/// Configuration for webhook signature verification.
#[derive(Debug, Clone)]
pub struct WebhookConfig {
    /// Maximum age tolerance for webhook signatures in seconds.
    /// Signatures older than this are rejected.
    pub signature_tolerance_secs: i64,
}

impl Default for WebhookConfig {
    fn default() -> Self {
        Self {
            signature_tolerance_secs: DEFAULT_WEBHOOK_SIGNATURE_TOLERANCE_SECS,
        }
    }
}

impl WebhookConfig {
    /// Create from environment variables with fallback to defaults.
    ///
    /// Environment variables:
    /// - `CALIBER_WEBHOOK_SIGNATURE_TOLERANCE_SECS`: Max signature age in seconds (default: 300)
    pub fn from_env() -> Self {
        let defaults = Self::default();

        Self {
            signature_tolerance_secs: std::env::var("CALIBER_WEBHOOK_SIGNATURE_TOLERANCE_SECS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(defaults.signature_tolerance_secs),
        }
    }
}

// ============================================================================
// IDEMPOTENCY CONFIGURATION
// ============================================================================

/// Configuration for idempotency middleware.
///
/// Provides environment variable loading for idempotency key settings.
#[derive(Debug, Clone)]
pub struct IdempotencySettings {
    /// Time-to-live for idempotency keys
    pub ttl: Duration,

    /// Maximum body size to consider for hashing (in bytes)
    pub max_body_size: usize,

    /// Whether to require idempotency keys on mutating requests
    pub require_key: bool,
}

impl Default for IdempotencySettings {
    fn default() -> Self {
        Self {
            ttl: Duration::from_secs(DEFAULT_IDEMPOTENCY_TTL_SECS),
            max_body_size: DEFAULT_IDEMPOTENCY_MAX_BODY_SIZE,
            require_key: false,
        }
    }
}

impl IdempotencySettings {
    /// Create from environment variables with fallback to defaults.
    ///
    /// Environment variables:
    /// - `CALIBER_IDEMPOTENCY_TTL_SECS`: Key TTL in seconds (default: 86400)
    /// - `CALIBER_IDEMPOTENCY_MAX_BODY_SIZE`: Max body size in bytes (default: 1048576)
    /// - `CALIBER_IDEMPOTENCY_REQUIRE_KEY`: Whether to require keys (default: false)
    pub fn from_env() -> Self {
        let defaults = Self::default();

        let ttl_secs = std::env::var("CALIBER_IDEMPOTENCY_TTL_SECS")
            .ok()
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(defaults.ttl.as_secs());

        let require_key = std::env::var("CALIBER_IDEMPOTENCY_REQUIRE_KEY")
            .ok()
            .map(|s| s.to_lowercase() == "true")
            .unwrap_or(defaults.require_key);

        Self {
            ttl: Duration::from_secs(ttl_secs),
            max_body_size: std::env::var("CALIBER_IDEMPOTENCY_MAX_BODY_SIZE")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(defaults.max_body_size),
            require_key,
        }
    }

    /// Convert to middleware-level `IdempotencyConfig`.
    pub fn to_middleware_config(&self) -> crate::middleware::idempotency::IdempotencyConfig {
        crate::middleware::idempotency::IdempotencyConfig {
            ttl: self.ttl,
            max_body_size: self.max_body_size,
            require_key: self.require_key,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::{
        DEFAULT_CORS_MAX_AGE_SECS, DEFAULT_RATE_LIMIT_AUTHENTICATED, DEFAULT_RATE_LIMIT_BURST,
        DEFAULT_RATE_LIMIT_UNAUTHENTICATED,
    };

    #[test]
    fn test_default_config() {
        let config = ApiConfig::default();
        assert!(config.cors_origins.is_empty());
        assert!(!config.cors_allow_credentials);
        assert_eq!(config.cors_max_age_secs, DEFAULT_CORS_MAX_AGE_SECS);
        assert!(config.rate_limit_enabled);
        assert_eq!(
            config.rate_limit_unauthenticated,
            DEFAULT_RATE_LIMIT_UNAUTHENTICATED
        );
        assert_eq!(
            config.rate_limit_authenticated,
            DEFAULT_RATE_LIMIT_AUTHENTICATED
        );
        assert_eq!(config.rate_limit_burst, DEFAULT_RATE_LIMIT_BURST);
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
        let config = ApiConfig {
            cors_origins: vec![
                "https://caliber.run".to_string(),
                "https://app.caliber.run".to_string(),
            ],
            ..Default::default()
        };

        assert!(config.is_origin_allowed("https://caliber.run"));
        assert!(config.is_origin_allowed("https://app.caliber.run"));
        assert!(!config.is_origin_allowed("https://evil.com"));
        assert!(!config.is_origin_allowed("https://notcaliber.run"));
    }

    #[test]
    fn test_wildcard_subdomain() {
        let config = ApiConfig {
            cors_origins: vec!["*.caliber.run".to_string()],
            ..Default::default()
        };

        assert!(config.is_origin_allowed("https://app.caliber.run"));
        assert!(config.is_origin_allowed("https://api.caliber.run"));
        assert!(!config.is_origin_allowed("https://evil.com"));
    }

    // ========================================================================
    // EndpointsConfig Tests
    // ========================================================================

    #[test]
    fn test_endpoints_default_config() {
        let config = EndpointsConfig::default();
        assert_eq!(config.api_base_url, "http://localhost:3000");
        assert_eq!(config.domain, "http://localhost:3000");
        assert_eq!(config.docs_url, "http://localhost:3000/docs");
        assert_eq!(config.lemonsqueezy_api_url, "https://api.lemonsqueezy.com");
        assert_eq!(config.workos_api_url, "https://api.workos.com");
    }

    #[test]
    fn test_endpoints_is_production() {
        let mut config = EndpointsConfig::default();
        assert!(!config.is_production());

        config.api_base_url = "https://api.caliber.run".to_string();
        assert!(config.is_production());

        config.api_base_url = "http://127.0.0.1:3000".to_string();
        assert!(!config.is_production());
    }

    #[test]
    fn test_endpoints_billing_success_redirect() {
        let mut config = EndpointsConfig::default();
        assert_eq!(
            config.billing_success_redirect(),
            "http://localhost:3000/dashboard/settings"
        );

        config.domain = "https://caliber.run".to_string();
        assert_eq!(
            config.billing_success_redirect(),
            "https://caliber.run/dashboard/settings"
        );
    }

    #[test]
    fn test_endpoints_workos_callback_url() {
        let mut config = EndpointsConfig::default();
        assert_eq!(
            config.workos_callback_url(),
            "http://localhost:3000/auth/sso/callback"
        );

        config.api_base_url = "https://api.caliber.run".to_string();
        assert_eq!(
            config.workos_callback_url(),
            "https://api.caliber.run/auth/sso/callback"
        );
    }
}
