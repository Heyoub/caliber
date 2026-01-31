//! Constants for CALIBER API
//!
//! This module contains all constant values used throughout the API.
//! Centralizing constants makes them easy to find, modify, and test.

// ============================================================================
// AUTHENTICATION
// ============================================================================

/// Default JWT token expiration time in seconds (1 hour)
pub const DEFAULT_JWT_EXPIRATION_SECS: i64 = 3600;

/// Minimum required length for JWT secret keys
pub const MIN_JWT_SECRET_LENGTH: usize = 32;

// ============================================================================
// CORS
// ============================================================================

/// Default CORS max age in seconds (24 hours)
pub const DEFAULT_CORS_MAX_AGE_SECS: u64 = 86400;

// ============================================================================
// RATE LIMITING
// ============================================================================

/// Default rate limit for unauthenticated requests (per minute)
pub const DEFAULT_RATE_LIMIT_UNAUTHENTICATED: u32 = 100;

/// Default rate limit for authenticated requests (per minute)
pub const DEFAULT_RATE_LIMIT_AUTHENTICATED: u32 = 1000;

/// Default burst size for rate limiting
pub const DEFAULT_RATE_LIMIT_BURST: u32 = 10;

// ============================================================================
// PAGINATION
// ============================================================================

/// Default page size for list operations
pub const DEFAULT_PAGE_SIZE: i32 = 50;

/// Maximum page size for list operations
pub const MAX_PAGE_SIZE: i32 = 1000;

// ============================================================================
// BATCH OPERATIONS
// ============================================================================

/// Maximum number of items allowed in a single batch request
pub const MAX_BATCH_ITEMS: usize = 100;

// ============================================================================
// SERVER URLs
// ============================================================================

/// Development server URL.
///
/// For runtime configuration, prefer using `EndpointsConfig::from_env()`
/// which reads the `CALIBER_API_BASE_URL` environment variable.
pub const DEV_SERVER_URL: &str = "http://localhost:3000";

/// Default production server URL.
///
/// **NOTE**: For runtime configuration, prefer using `EndpointsConfig::from_env()`
/// which reads the `CALIBER_API_BASE_URL` environment variable. This constant
/// is kept for backward compatibility and for cases where static values are needed
/// (e.g., OpenAPI spec generation).
///
/// Environment variables for production:
/// - `CALIBER_API_BASE_URL`: API base URL (e.g., https://api.caliber.run)
/// - `CALIBER_DOMAIN`: Main domain (e.g., https://caliber.run)
/// - `CALIBER_DOCS_URL`: Documentation URL (e.g., https://docs.caliber.run)
pub const PROD_SERVER_URL: &str = "https://api.caliber.run";

/// Default production domain.
///
/// **NOTE**: For runtime configuration, prefer using `EndpointsConfig::from_env()`
/// which reads the `CALIBER_DOMAIN` environment variable.
pub const PROD_DOMAIN: &str = "https://caliber.run";

/// Default production documentation URL.
///
/// **NOTE**: For runtime configuration, prefer using `EndpointsConfig::from_env()`
/// which reads the `CALIBER_DOCS_URL` environment variable.
pub const PROD_DOCS_URL: &str = "https://docs.caliber.run";

// ============================================================================
// CONTEXT ASSEMBLY
// ============================================================================

/// Default token budget for REST context assembly endpoints
pub const DEFAULT_REST_TOKEN_BUDGET: i32 = 8000;

/// Default token budget for GraphQL context assembly endpoints
pub const DEFAULT_GRAPHQL_TOKEN_BUDGET: i32 = 4096;

/// Default maximum number of notes to include in context assembly
pub const DEFAULT_CONTEXT_MAX_NOTES: usize = 10;

/// Default maximum number of artifacts to include in context assembly
pub const DEFAULT_CONTEXT_MAX_ARTIFACTS: usize = 5;

/// Default maximum number of conversation turns to include in context assembly
pub const DEFAULT_CONTEXT_MAX_TURNS: usize = 20;

/// Default maximum number of scope summaries to include in context assembly
pub const DEFAULT_CONTEXT_MAX_SUMMARIES: usize = 5;

// ============================================================================
// WEBHOOKS
// ============================================================================

/// Default webhook signature tolerance in seconds (5 minutes)
pub const DEFAULT_WEBHOOK_SIGNATURE_TOLERANCE_SECS: i64 = 300;

// ============================================================================
// IDEMPOTENCY
// ============================================================================

/// Default TTL for idempotency keys in seconds (24 hours)
pub const DEFAULT_IDEMPOTENCY_TTL_SECS: u64 = 86400;

/// Default maximum body size for idempotency hashing (1 MB)
pub const DEFAULT_IDEMPOTENCY_MAX_BODY_SIZE: usize = 1024 * 1024;

// ============================================================================
// SAGA CLEANUP CONFIGURATION
// ============================================================================

/// Default check interval for saga cleanup in seconds (1 minute)
pub const DEFAULT_SAGA_CHECK_INTERVAL_SECS: u64 = 60;

/// Default delegation timeout in seconds (1 hour)
pub const DEFAULT_SAGA_DELEGATION_TIMEOUT_SECS: u64 = 3600;

/// Default handoff timeout in seconds (30 minutes)
pub const DEFAULT_SAGA_HANDOFF_TIMEOUT_SECS: u64 = 1800;

/// Default batch size for saga cleanup operations
pub const DEFAULT_SAGA_BATCH_SIZE: usize = 100;

/// Default idempotency key cleanup interval in seconds (1 hour)
pub const DEFAULT_SAGA_IDEMPOTENCY_CLEANUP_INTERVAL_SECS: u64 = 3600;

// ============================================================================
// CIRCUIT BREAKER CONFIGURATION
// ============================================================================

/// Default failure threshold before circuit opens
pub const DEFAULT_CIRCUIT_FAILURE_THRESHOLD: u32 = 5;

/// Default success threshold to close circuit after half-open
pub const DEFAULT_CIRCUIT_SUCCESS_THRESHOLD: u32 = 3;

/// Default circuit breaker timeout in seconds (30 seconds)
pub const DEFAULT_CIRCUIT_TIMEOUT_SECS: u64 = 30;

// ============================================================================
// MCP TOOL EXECUTION
// ============================================================================

/// Default tool execution timeout in milliseconds (30 seconds)
pub const DEFAULT_MCP_TOOL_TIMEOUT_MS: u64 = 30_000;

/// Maximum tool execution timeout in milliseconds (5 minutes)
pub const MAX_MCP_TOOL_TIMEOUT_MS: u64 = 300_000;

/// Minimum tool execution timeout in milliseconds (100ms)
pub const MIN_MCP_TOOL_TIMEOUT_MS: u64 = 100;
