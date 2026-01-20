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

/// Development server URL
pub const DEV_SERVER_URL: &str = "http://localhost:3000";

/// Production server URL
pub const PROD_SERVER_URL: &str = "https://api.caliber.run";
