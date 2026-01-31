//! Authentication Module
//!
//! This module provides authentication and authorization for the CALIBER API.
//! It supports two authentication methods:
//! 1. API Key authentication (via X-API-Key header)
//! 2. JWT token authentication (via Authorization: Bearer header)
//!
//! Additionally, it extracts tenant context from the X-Tenant-ID header for
//! multi-tenant isolation.
//!
//! Requirements: 1.5, 1.6

use crate::error::{ApiError, ApiResult};
use caliber_core::{CaliberError, ConfigError, EntityIdType, TenantId};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::Arc;
use uuid::Uuid;

// ============================================================================
// CLOCK ABSTRACTION (FOR DETERMINISTIC TESTS + CI ROBUSTNESS)
// ============================================================================

/// Clock abstraction for JWT time validation.
///
/// This allows us to inject time in tests and handle broken CI environments
/// where `SystemTime::now()` might return pre-epoch times (causing panics).
///
/// By owning time validation ourselves (instead of letting `jsonwebtoken` do it),
/// we avoid the `SystemTime::now().duration_since(UNIX_EPOCH).expect()` panic
/// path and make tests fully deterministic.
pub trait JwtClock: Send + Sync {
    /// Get current time as Unix epoch seconds.
    ///
    /// Returns negative values for pre-1970 times (which should be treated as errors
    /// in production but can be handled gracefully in tests).
    fn now_epoch_secs(&self) -> i64;
}

/// Production clock using system time.
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemClock;

impl JwtClock for SystemClock {
    fn now_epoch_secs(&self) -> i64 {
        chrono::Utc::now().timestamp()
    }
}

/// Fixed clock for deterministic tests.
///
/// Always returns the same timestamp, making tests reproducible and
/// immune to CI environment clock issues.
#[derive(Debug, Clone, Copy)]
pub struct FixedClock(pub i64);

impl JwtClock for FixedClock {
    fn now_epoch_secs(&self) -> i64 {
        self.0
    }
}

/// Test clock helpers for common scenarios.
#[cfg(test)]
pub mod test_clocks {
    use super::FixedClock;

    /// 2024-01-01 00:00:00 UTC - always valid for tests
    pub fn valid() -> FixedClock {
        FixedClock(1704067200)
    }

    /// 2020-01-01 00:00:00 UTC - always in the past
    pub fn expired() -> FixedClock {
        FixedClock(1577836800)
    }

    /// 2030-01-01 00:00:00 UTC - far future for nbf tests
    pub fn future() -> FixedClock {
        FixedClock(1893456000)
    }
}

// ============================================================================
// JWT SECRET (TYPE-SAFE)
// ============================================================================

/// Type-safe JWT secret that prevents accidental logging.
///
/// This wraps the secret in a `secrecy::Secret` to ensure it's never
/// accidentally logged or displayed.
#[derive(Clone)]
pub struct JwtSecret(SecretString);

impl JwtSecret {
    /// Create a new JWT secret with validation.
    ///
    /// # Errors
    /// Returns error if the secret is empty.
    pub fn new(secret: String) -> Result<Self, CaliberError> {
        if secret.is_empty() {
            return Err(CaliberError::Config(ConfigError::MissingRequired {
                field: "jwt_secret".to_string(),
            }));
        }
        Ok(Self(SecretString::new(secret.into())))
    }

    /// Expose the secret value (use sparingly, only for cryptographic operations).
    pub fn expose(&self) -> &str {
        self.0.expose_secret()
    }

    /// Get the length of the secret without exposing it.
    pub fn len(&self) -> usize {
        self.0.expose_secret().len()
    }

    /// Check if the secret is empty without exposing it.
    pub fn is_empty(&self) -> bool {
        self.0.expose_secret().is_empty()
    }

    /// Check if the secret is the insecure default.
    pub fn is_insecure_default(&self) -> bool {
        self.0.expose_secret() == "INSECURE_DEFAULT_SECRET_CHANGE_IN_PRODUCTION"
    }
}

impl std::fmt::Debug for JwtSecret {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "JwtSecret([REDACTED, {} chars])", self.len())
    }
}

// ============================================================================
// CONFIGURATION
// ============================================================================

/// Authentication provider selection.
///
/// Determines which authentication backend to use for validating credentials.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AuthProvider {
    /// Standard JWT authentication (default)
    #[default]
    Jwt,

    /// WorkOS SSO authentication (requires `workos` feature)
    WorkOs,
}

impl std::str::FromStr for AuthProvider {
    type Err = std::convert::Infallible;

    /// Parse auth provider from string (case-insensitive).
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s.to_lowercase().as_str() {
            "workos" => AuthProvider::WorkOs,
            _ => AuthProvider::Jwt,
        })
    }
}

/// Authentication configuration.
#[derive(Clone)]
pub struct AuthConfig {
    /// Valid API keys (in production, load from secure storage)
    pub api_keys: HashSet<String>,

    /// JWT secret key for signing and verification
    pub jwt_secret: JwtSecret,

    /// JWT algorithm (default: HS256)
    pub jwt_algorithm: Algorithm,

    /// JWT token expiration in seconds (default: 1 hour)
    pub jwt_expiration_secs: i64,

    /// JWT clock skew tolerance in seconds (default: 60)
    ///
    /// Allows tokens to be slightly in the future/past to handle clock drift
    /// in distributed systems. This is standard practice (AWS, Google, Auth0 all do this).
    pub jwt_clock_skew_secs: i64,

    /// Whether to require tenant header
    pub require_tenant_header: bool,

    /// WorkOS client ID (optional, required when using WorkOS auth)
    pub workos_client_id: Option<String>,

    /// WorkOS API key (optional, required when using WorkOS auth)
    pub workos_api_key: Option<String>,

    /// WorkOS redirect URI for SSO callback
    pub workos_redirect_uri: Option<String>,

    /// Authentication provider to use
    pub auth_provider: AuthProvider,

    /// Clock for JWT time validation (injected for testing)
    #[allow(clippy::type_complexity)]
    pub clock: Arc<dyn JwtClock>,
}

impl std::fmt::Debug for AuthConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AuthConfig")
            .field("api_keys", &format!("[{} keys]", self.api_keys.len()))
            .field("jwt_secret", &self.jwt_secret)
            .field("jwt_algorithm", &self.jwt_algorithm)
            .field("jwt_expiration_secs", &self.jwt_expiration_secs)
            .field("jwt_clock_skew_secs", &self.jwt_clock_skew_secs)
            .field("require_tenant_header", &self.require_tenant_header)
            .field(
                "workos_client_id",
                &self.workos_client_id.as_ref().map(|_| "[REDACTED]"),
            )
            .field(
                "workos_api_key",
                &self.workos_api_key.as_ref().map(|_| "[REDACTED]"),
            )
            .field("workos_redirect_uri", &self.workos_redirect_uri)
            .field("auth_provider", &self.auth_provider)
            .field("clock", &"<JwtClock>")
            .finish()
    }
}

impl Default for AuthConfig {
    fn default() -> Self {
        let secret_str = std::env::var("CALIBER_JWT_SECRET")
            .unwrap_or_else(|_| "INSECURE_DEFAULT_SECRET_CHANGE_IN_PRODUCTION".to_string());

        Self {
            api_keys: HashSet::new(),
            jwt_secret: build_jwt_secret(secret_str),
            jwt_algorithm: Algorithm::HS256,
            jwt_expiration_secs: 3600, // 1 hour
            jwt_clock_skew_secs: 60,   // 60 seconds (industry standard)
            require_tenant_header: true,
            workos_client_id: None,
            workos_api_key: None,
            workos_redirect_uri: None,
            auth_provider: AuthProvider::default(),
            clock: Arc::new(SystemClock),
        }
    }
}

impl AuthConfig {
    /// Create authentication configuration from environment variables.
    ///
    /// # Environment Variables
    /// - `CALIBER_API_KEYS`: Comma-separated list of valid API keys
    /// - `CALIBER_JWT_SECRET`: JWT signing secret
    /// - `CALIBER_JWT_EXPIRATION_SECS`: JWT token expiration (default: 3600)
    /// - `CALIBER_JWT_CLOCK_SKEW_SECS`: JWT clock skew tolerance (default: 60)
    /// - `CALIBER_REQUIRE_TENANT_HEADER`: Whether X-Tenant-ID is required (default: true)
    /// - `CALIBER_AUTH_PROVIDER`: Authentication provider ("jwt" | "workos", default: "jwt")
    /// - `CALIBER_WORKOS_CLIENT_ID`: WorkOS application client ID
    /// - `CALIBER_WORKOS_API_KEY`: WorkOS API key
    /// - `CALIBER_WORKOS_REDIRECT_URI`: WorkOS SSO callback redirect URI
    pub fn from_env() -> Self {
        let mut api_keys = HashSet::new();

        // Load API keys from environment (comma-separated)
        if let Ok(keys_str) = std::env::var("CALIBER_API_KEYS") {
            for key in keys_str.split(',') {
                let trimmed = key.trim();
                if !trimmed.is_empty() {
                    api_keys.insert(trimmed.to_string());
                }
            }
        }

        // Determine auth provider (parse infallibly - always succeeds)
        let auth_provider: AuthProvider = std::env::var("CALIBER_AUTH_PROVIDER")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or_default();

        let secret_str = std::env::var("CALIBER_JWT_SECRET")
            .unwrap_or_else(|_| "INSECURE_DEFAULT_SECRET_CHANGE_IN_PRODUCTION".to_string());

        Self {
            api_keys,
            jwt_secret: build_jwt_secret(secret_str),
            jwt_algorithm: Algorithm::HS256,
            jwt_expiration_secs: std::env::var("CALIBER_JWT_EXPIRATION_SECS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(3600),
            jwt_clock_skew_secs: std::env::var("CALIBER_JWT_CLOCK_SKEW_SECS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(60),
            require_tenant_header: std::env::var("CALIBER_REQUIRE_TENANT_HEADER")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(true),
            workos_client_id: std::env::var("CALIBER_WORKOS_CLIENT_ID").ok(),
            workos_api_key: std::env::var("CALIBER_WORKOS_API_KEY").ok(),
            workos_redirect_uri: std::env::var("CALIBER_WORKOS_REDIRECT_URI").ok(),
            auth_provider,
            clock: Arc::new(SystemClock),
        }
    }

    /// Validate the authentication configuration for production use.
    ///
    /// This function should be called at server startup to ensure that
    /// insecure defaults are not being used in production environments.
    /// In development mode, warnings are logged but the server continues.
    pub fn validate_for_production(&self) -> ApiResult<()> {
        // Check if running in production environment
        let environment = std::env::var("CALIBER_ENVIRONMENT")
            .unwrap_or_else(|_| "development".to_string())
            .to_lowercase();

        let is_production = environment == "production" || environment == "prod";

        // Check for insecure default secret
        if self.jwt_secret.is_insecure_default() {
            if is_production {
                return Err(ApiError::invalid_input(format!(
                    "Cannot start server in production with insecure JWT secret. \
                     Set CALIBER_JWT_SECRET to a secure value. \
                     CALIBER_ENVIRONMENT={}",
                    environment
                )));
            } else {
                tracing::warn!(
                    "⚠️  SECURITY WARNING: Using insecure default JWT secret. \
                     This is acceptable for local development but MUST be changed \
                     before deploying. Set CALIBER_JWT_SECRET environment variable \
                     to a secure random value (minimum 32 characters)."
                );
            }
        }

        // Check for short secret
        if self.jwt_secret.len() < 32 {
            if is_production {
                return Err(ApiError::invalid_input(format!(
                    "JWT secret is too short for production use ({} chars). \
                     It must be at least 32 characters long.",
                    self.jwt_secret.len()
                )));
            } else if !self.jwt_secret.is_insecure_default() {
                // Only warn if not already warned about insecure default
                tracing::warn!(
                    "⚠️  SECURITY WARNING: JWT secret is short ({} chars). \
                     For production, use at least 32 characters. \
                     Set CALIBER_JWT_SECRET to a longer secure value.",
                    self.jwt_secret.len()
                );
            }
        }

        Ok(())
    }

    /// Add an API key to the valid set.
    pub fn add_api_key(&mut self, key: String) {
        self.api_keys.insert(key);
    }

    /// Check if an API key is valid.
    pub fn is_valid_api_key(&self, key: &str) -> bool {
        self.api_keys.contains(key)
    }
}

fn build_jwt_secret(secret_str: String) -> JwtSecret {
    let normalized = if secret_str.trim().is_empty() {
        "INSECURE_DEFAULT_SECRET_CHANGE_IN_PRODUCTION".to_string()
    } else {
        secret_str
    };

    match JwtSecret::new(normalized) {
        Ok(secret) => secret,
        Err(_) => JwtSecret(SecretString::new(
            "INSECURE_DEFAULT_SECRET_CHANGE_IN_PRODUCTION"
                .to_string()
                .into(),
        )),
    }
}

// ============================================================================
// JWT CLAIMS
// ============================================================================

/// JWT claims structure.
///
/// This contains the standard JWT claims plus custom claims for CALIBER.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Claims {
    /// Subject (user ID)
    pub sub: String,

    /// Issued at (Unix timestamp)
    pub iat: i64,

    /// Expiration time (Unix timestamp)
    pub exp: i64,

    /// Tenant ID the user has access to
    pub tenant_id: Option<String>,

    /// User roles/permissions
    #[serde(default)]
    pub roles: Vec<String>,
}

impl Claims {
    /// Create new claims for a user using a clock.
    pub fn new(
        user_id: String,
        tenant_id: Option<TenantId>,
        expiration_secs: i64,
        clock: &dyn JwtClock,
    ) -> Self {
        let now = clock.now_epoch_secs();

        Self {
            sub: user_id,
            iat: now,
            exp: now + expiration_secs,
            tenant_id: tenant_id.map(|id| id.to_string()),
            roles: Vec::new(),
        }
    }

    /// Add a role to the claims.
    pub fn with_role(mut self, role: String) -> Self {
        self.roles.push(role);
        self
    }

    /// Add multiple roles to the claims.
    pub fn with_roles(mut self, roles: Vec<String>) -> Self {
        self.roles.extend(roles);
        self
    }

    /// Check if the token has expired according to a clock.
    pub fn is_expired(&self, clock: &dyn JwtClock) -> bool {
        let now = clock.now_epoch_secs();
        self.exp < now
    }

    /// Get the tenant ID as TenantId.
    pub fn tenant_id(&self) -> Option<TenantId> {
        self.tenant_id
            .as_ref()
            .and_then(|s| Uuid::parse_str(s).ok())
            .map(TenantId::new)
    }
}

// ============================================================================
// AUTHENTICATION CONTEXT
// ============================================================================

/// Authentication context extracted from request.
///
/// This is injected into Axum request extensions after successful authentication.
#[derive(Debug, Clone)]
pub struct AuthContext {
    /// User ID (from JWT sub claim or API key identifier)
    pub user_id: String,

    /// Tenant ID (from X-Tenant-ID header or JWT claim)
    pub tenant_id: TenantId,

    /// User roles/permissions
    pub roles: Vec<String>,

    /// Authentication method used
    pub auth_method: AuthMethod,

    /// User's email address (from JWT or WorkOS)
    pub email: Option<String>,

    /// User's first name (from JWT or WorkOS)
    pub first_name: Option<String>,

    /// User's last name (from JWT or WorkOS)
    pub last_name: Option<String>,
}

impl AuthContext {
    /// Create a new authentication context.
    pub fn new(
        user_id: String,
        tenant_id: TenantId,
        roles: Vec<String>,
        auth_method: AuthMethod,
    ) -> Self {
        Self {
            user_id,
            tenant_id,
            roles,
            auth_method,
            email: None,
            first_name: None,
            last_name: None,
        }
    }

    /// Create a new authentication context with user profile info.
    pub fn with_profile(
        user_id: String,
        tenant_id: TenantId,
        roles: Vec<String>,
        auth_method: AuthMethod,
        email: Option<String>,
        first_name: Option<String>,
        last_name: Option<String>,
    ) -> Self {
        Self {
            user_id,
            tenant_id,
            roles,
            auth_method,
            email,
            first_name,
            last_name,
        }
    }

    /// Check if the user has a specific role.
    pub fn has_role(&self, role: &str) -> bool {
        self.roles.iter().any(|r| r == role)
    }

    /// Check if the user has any of the specified roles.
    pub fn has_any_role(&self, roles: &[&str]) -> bool {
        roles.iter().any(|role| self.has_role(role))
    }

    /// Check if the user has all of the specified roles.
    pub fn has_all_roles(&self, roles: &[&str]) -> bool {
        roles.iter().all(|role| self.has_role(role))
    }
}

/// Authentication method used.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthMethod {
    /// API key authentication
    ApiKey,

    /// JWT token authentication
    Jwt,

    /// WorkOS SSO authentication
    WorkOs,
}

// ============================================================================
// AUTHENTICATION FUNCTIONS
// ============================================================================

/// Validate an API key.
///
/// Returns Ok(()) if the key is valid, Err otherwise.
pub fn validate_api_key(config: &AuthConfig, api_key: &str) -> ApiResult<()> {
    if config.is_valid_api_key(api_key) {
        Ok(())
    } else {
        Err(ApiError::unauthorized("Invalid API key"))
    }
}

/// Validate JWT claim times using our own clock logic.
///
/// This is separated from signature validation so we can:
/// 1. Handle broken CI environments (pre-epoch clocks) gracefully
/// 2. Make tests fully deterministic with injected clocks
/// 3. Apply custom clock skew policies
///
/// # Arguments
/// * `now` - Current time from clock
/// * `exp` - Expiration time from JWT claims
/// * `nbf` - Not-before time from JWT claims (optional)
/// * `leeway_secs` - Clock skew tolerance
fn validate_claim_times(now: i64, exp: i64, nbf: Option<i64>, leeway_secs: i64) -> ApiResult<()> {
    // Check not-before (nbf): allow slightly-in-the-future within leeway
    if let Some(nbf) = nbf {
        if now + leeway_secs < nbf {
            return Err(ApiError::unauthorized("Token not yet valid (nbf)"));
        }
    }

    // Check expiration (exp): allow slightly-in-the-past within leeway
    if exp < now - leeway_secs {
        return Err(ApiError::token_expired());
    }

    Ok(())
}

/// Validate a JWT token and extract claims.
///
/// This performs signature validation ONLY (no time validation) to avoid
/// the `SystemTime::now().duration_since(UNIX_EPOCH).expect()` panic path
/// in `jsonwebtoken`. We do our own time validation with injected clocks.
///
/// Returns the claims if the token is valid, Err otherwise.
pub fn validate_jwt_token(config: &AuthConfig, token: &str) -> ApiResult<Claims> {
    let decoding_key = DecodingKey::from_secret(config.jwt_secret.expose().as_bytes());

    // Decode with signature validation ONLY (skip exp/nbf validation)
    let mut validation = Validation::new(config.jwt_algorithm);
    validation.validate_exp = false; // We'll do this ourselves with our clock
    validation.validate_nbf = false;
    // Keep required_spec_claims with "exp" to ensure it's present
    validation.required_spec_claims = std::collections::HashSet::from(["exp".to_string()]);

    let token_data =
        decode::<Claims>(token, &decoding_key, &validation).map_err(|e| match e.kind() {
            jsonwebtoken::errors::ErrorKind::InvalidToken => {
                ApiError::invalid_token("Token is invalid")
            }
            jsonwebtoken::errors::ErrorKind::InvalidSignature => {
                ApiError::invalid_token("Token signature is invalid")
            }
            _ => ApiError::invalid_token(format!("Token validation failed: {}", e)),
        })?;

    let claims = token_data.claims;

    // Get current time from config's clock
    let now = config.clock.now_epoch_secs();

    // Fail loud if production clock returns pre-epoch time (server is hallucinating)
    if now < 0 {
        tracing::error!(
            timestamp = now,
            "System clock returned pre-epoch time - server time is broken"
        );
        return Err(ApiError::internal_error(
            "Server time configuration error - please contact support",
        ));
    }

    // Apply our own time validation with clock skew tolerance
    validate_claim_times(now, claims.exp, None, config.jwt_clock_skew_secs)?;

    Ok(claims)
}

/// Generate a JWT token for a user.
///
/// Returns the encoded token string.
pub fn generate_jwt_token(
    config: &AuthConfig,
    user_id: String,
    tenant_id: Option<TenantId>,
    roles: Vec<String>,
) -> ApiResult<String> {
    let claims = Claims::new(
        user_id,
        tenant_id,
        config.jwt_expiration_secs,
        &*config.clock,
    )
    .with_roles(roles);

    let encoding_key = EncodingKey::from_secret(config.jwt_secret.expose().as_bytes());
    let header = Header::new(config.jwt_algorithm);

    encode(&header, &claims, &encoding_key)
        .map_err(|e| ApiError::internal_error(format!("Failed to generate token: {}", e)))
}

/// Extract tenant ID from header value.
///
/// Parses the X-Tenant-ID header as a UUID.
pub fn extract_tenant_id(header_value: &str) -> ApiResult<TenantId> {
    let uuid = Uuid::parse_str(header_value)
        .map_err(|_| ApiError::invalid_format("X-Tenant-ID", "valid UUID"))?;
    Ok(TenantId::new(uuid))
}

/// Authenticate a request using API key.
///
/// Extracts the API key from the X-API-Key header and validates it.
/// Also extracts the tenant ID from the X-Tenant-ID header.
pub fn authenticate_api_key(
    config: &AuthConfig,
    api_key: &str,
    tenant_id_header: Option<&str>,
) -> ApiResult<AuthContext> {
    // Validate API key
    validate_api_key(config, api_key)?;

    // Extract tenant ID
    let tenant_id = if let Some(tenant_header) = tenant_id_header {
        extract_tenant_id(tenant_header)?
    } else if config.require_tenant_header {
        return Err(ApiError::missing_field("X-Tenant-ID"));
    } else {
        // Use a default tenant ID if not required (for single-tenant deployments)
        TenantId::new(Uuid::nil())
    };

    // For API key auth, we use the key itself as a user identifier
    // In production, you'd look up the user associated with the key
    let user_id = format!("api_key_{}", &api_key[..8.min(api_key.len())]);

    Ok(AuthContext::new(
        user_id,
        tenant_id,
        vec!["api_user".to_string()],
        AuthMethod::ApiKey,
    ))
}

/// Authenticate a request using JWT token.
///
/// Extracts the JWT token from the Authorization: Bearer header and validates it.
/// The tenant ID can come from either the JWT claims or the X-Tenant-ID header.
pub fn authenticate_jwt(
    config: &AuthConfig,
    token: &str,
    tenant_id_header: Option<&str>,
) -> ApiResult<AuthContext> {
    // Validate JWT and extract claims
    let claims = validate_jwt_token(config, token)?;

    // Determine tenant ID (header takes precedence over JWT claim)
    let tenant_id = if let Some(tenant_header) = tenant_id_header {
        extract_tenant_id(tenant_header)?
    } else if let Some(jwt_tenant_id) = claims.tenant_id() {
        jwt_tenant_id
    } else if config.require_tenant_header {
        return Err(ApiError::missing_field(
            "X-Tenant-ID or JWT tenant_id claim",
        ));
    } else {
        // Use a default tenant ID if not required
        TenantId::new(Uuid::nil())
    };

    Ok(AuthContext::new(
        claims.sub,
        tenant_id,
        claims.roles,
        AuthMethod::Jwt,
    ))
}

/// Authenticate a request using either API key or JWT token.
///
/// This is the main authentication function that tries both methods.
/// It checks for:
/// 1. X-API-Key header for API key authentication
/// 2. Authorization: Bearer header for JWT authentication
///
/// Returns the authentication context if successful.
pub fn authenticate(
    config: &AuthConfig,
    api_key_header: Option<&str>,
    auth_header: Option<&str>,
    tenant_id_header: Option<&str>,
) -> ApiResult<AuthContext> {
    // Try API key authentication first
    if let Some(api_key) = api_key_header {
        return authenticate_api_key(config, api_key, tenant_id_header);
    }

    // Try JWT authentication
    if let Some(auth_value) = auth_header {
        // Extract Bearer token
        if let Some(token) = auth_value.strip_prefix("Bearer ") {
            return authenticate_jwt(config, token, tenant_id_header);
        } else {
            return Err(ApiError::invalid_token(
                "Authorization header must use Bearer scheme",
            ));
        }
    }

    // No authentication provided
    Err(ApiError::unauthorized(
        "Authentication required: provide X-API-Key or Authorization header",
    ))
}

/// Check if a user has access to a specific tenant.
///
/// This validates that the authenticated user's tenant ID matches the requested tenant.
pub fn check_tenant_access(
    auth_context: &AuthContext,
    requested_tenant_id: TenantId,
) -> ApiResult<()> {
    if auth_context.tenant_id == requested_tenant_id {
        Ok(())
    } else {
        Err(ApiError::forbidden(format!(
            "Access denied to tenant {}",
            requested_tenant_id
        )))
    }
}

/// Validate that a resource belongs to the authenticated user's tenant.
///
/// This is used by handlers to enforce tenant isolation on read/update/delete operations.
///
/// # Arguments
/// * `auth` - The authentication context from the request
/// * `resource_tenant_id` - The tenant_id of the resource being accessed (may be None for legacy data)
///
/// # Returns
/// * `Ok(())` if the resource belongs to the user's tenant
/// * `Err(ApiError::forbidden)` if tenant mismatch or resource has no tenant (legacy data)
pub fn validate_tenant_ownership(
    auth: &AuthContext,
    resource_tenant_id: Option<TenantId>,
) -> ApiResult<()> {
    match resource_tenant_id {
        Some(tenant_id) if tenant_id == auth.tenant_id => Ok(()),
        Some(tenant_id) => Err(ApiError::forbidden(format!(
            "Access denied: resource belongs to different tenant (expected {}, got {})",
            auth.tenant_id, tenant_id
        ))),
        None => {
            // Resource has no tenant_id - this is legacy data from before tenant isolation
            // In strict mode, we deny access; in permissive mode, we allow access
            // Default to strict mode for security
            tracing::warn!(
                user_id = %auth.user_id,
                tenant_id = %auth.tenant_id,
                "Attempted access to resource without tenant_id (legacy data)"
            );
            Err(ApiError::forbidden(
                "Access denied: resource has no tenant association (legacy data)",
            ))
        }
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static ENV_MUTEX: Mutex<()> = Mutex::new(());

    struct EnvVarGuard {
        key: &'static str,
        previous: Option<String>,
    }

    impl EnvVarGuard {
        fn set(key: &'static str, value: Option<&str>) -> Self {
            let previous = std::env::var(key).ok();
            match value {
                Some(value) => std::env::set_var(key, value),
                None => std::env::remove_var(key),
            }
            Self { key, previous }
        }
    }

    impl Drop for EnvVarGuard {
        fn drop(&mut self) {
            match self.previous.as_deref() {
                Some(value) => std::env::set_var(self.key, value),
                None => std::env::remove_var(self.key),
            }
        }
    }

    fn test_config() -> AuthConfig {
        let mut config = AuthConfig::default();
        config.add_api_key("test_key_123".to_string());
        config.jwt_secret =
            JwtSecret::new("test_secret".to_string()).expect("Test secret should be valid");
        config.require_tenant_header = false;
        config.clock = Arc::new(test_clocks::valid()); // Use deterministic clock
        config
    }

    #[test]
    fn test_api_key_validation() {
        let config = test_config();

        // Valid key
        assert!(validate_api_key(&config, "test_key_123").is_ok());

        // Invalid key
        assert!(validate_api_key(&config, "invalid_key").is_err());
    }

    #[test]
    fn test_jwt_generation_and_validation() -> ApiResult<()> {
        let config = test_config();
        let user_id = "user123".to_string();
        let tenant_id = Some(TenantId::new(Uuid::now_v7()));
        let roles = vec!["admin".to_string()];

        // Generate token
        let token = generate_jwt_token(&config, user_id.clone(), tenant_id, roles.clone())?;

        // Validate token
        let claims = validate_jwt_token(&config, &token)?;

        assert_eq!(claims.sub, user_id);
        assert_eq!(claims.roles, roles);
        assert!(!claims.is_expired(&test_clocks::valid())); // Use same fixed clock as token generation
        Ok(())
    }

    #[test]
    fn test_expired_token() -> ApiResult<()> {
        let mut config = test_config();
        config.jwt_expiration_secs = -1; // Already expired

        let token = generate_jwt_token(&config, "user123".to_string(), None, vec![])?;

        // Move clock forward (or use expired clock) for validation
        config.clock = Arc::new(test_clocks::future()); // Far in the future, token will be expired

        let result = validate_jwt_token(&config, &token);
        assert!(result.is_err());

        if let Err(e) = result {
            assert_eq!(e.code, crate::error::ErrorCode::TokenExpired);
        }
        Ok(())
    }

    #[test]
    fn test_tenant_id_extraction() -> ApiResult<()> {
        let tenant_id = Uuid::now_v7();
        let tenant_id_str = tenant_id.to_string();

        let extracted = extract_tenant_id(&tenant_id_str)?;
        let expected = TenantId::new(tenant_id);
        assert_eq!(extracted, expected);

        // Invalid UUID
        assert!(extract_tenant_id("not-a-uuid").is_err());
        Ok(())
    }

    #[test]
    fn test_authenticate_api_key() -> ApiResult<()> {
        let config = test_config();
        let tenant_id = Uuid::now_v7();

        let auth_context =
            authenticate_api_key(&config, "test_key_123", Some(&tenant_id.to_string()))?;

        let expected_tenant = TenantId::new(tenant_id);
        assert_eq!(auth_context.tenant_id, expected_tenant);
        assert_eq!(auth_context.auth_method, AuthMethod::ApiKey);
        assert!(auth_context.has_role("api_user"));
        Ok(())
    }

    #[test]
    fn test_authenticate_jwt() -> ApiResult<()> {
        let config = test_config();
        let user_id = "user123".to_string();
        let tenant_id = Uuid::now_v7();
        let roles = vec!["admin".to_string()];

        let token = generate_jwt_token(
            &config,
            user_id.clone(),
            Some(TenantId::new(tenant_id)),
            roles.clone(),
        )?;

        let auth_context = authenticate_jwt(&config, &token, None)?;

        let expected_tenant = TenantId::new(tenant_id);
        assert_eq!(auth_context.user_id, user_id);
        assert_eq!(auth_context.tenant_id, expected_tenant);
        assert_eq!(auth_context.roles, roles);
        assert_eq!(auth_context.auth_method, AuthMethod::Jwt);
        Ok(())
    }

    #[test]
    fn test_authenticate_with_api_key() -> ApiResult<()> {
        let config = test_config();
        let tenant_id = Uuid::now_v7();

        let auth_context = authenticate(
            &config,
            Some("test_key_123"),
            None,
            Some(&tenant_id.to_string()),
        )?;

        assert_eq!(auth_context.auth_method, AuthMethod::ApiKey);
        Ok(())
    }

    #[test]
    fn test_authenticate_with_jwt() -> ApiResult<()> {
        let config = test_config();
        let user_id = "user123".to_string();
        let tenant_id = Uuid::now_v7();

        let token = generate_jwt_token(
            &config,
            user_id.clone(),
            Some(TenantId::new(tenant_id)),
            vec![],
        )?;

        let auth_header = format!("Bearer {}", token);

        let auth_context = authenticate(&config, None, Some(&auth_header), None)?;

        assert_eq!(auth_context.auth_method, AuthMethod::Jwt);
        assert_eq!(auth_context.user_id, user_id);
        Ok(())
    }

    #[test]
    fn test_authenticate_no_credentials() {
        let config = test_config();

        let result = authenticate(&config, None, None, None);
        assert!(result.is_err());

        if let Err(e) = result {
            assert_eq!(e.code, crate::error::ErrorCode::Unauthorized);
        }
    }

    #[test]
    fn test_check_tenant_access() {
        let tenant_id = TenantId::new(Uuid::now_v7());
        let auth_context =
            AuthContext::new("user123".to_string(), tenant_id, vec![], AuthMethod::ApiKey);

        // Same tenant - should succeed
        assert!(check_tenant_access(&auth_context, tenant_id).is_ok());

        // Different tenant - should fail
        let other_tenant_id = TenantId::new(Uuid::now_v7());
        let result = check_tenant_access(&auth_context, other_tenant_id);
        assert!(result.is_err());

        if let Err(e) = result {
            assert_eq!(e.code, crate::error::ErrorCode::Forbidden);
        }
    }

    #[test]
    fn test_auth_context_role_checks() {
        let auth_context = AuthContext::new(
            "user123".to_string(),
            TenantId::new(Uuid::now_v7()),
            vec!["admin".to_string(), "editor".to_string()],
            AuthMethod::Jwt,
        );

        assert!(auth_context.has_role("admin"));
        assert!(auth_context.has_role("editor"));
        assert!(!auth_context.has_role("viewer"));

        assert!(auth_context.has_any_role(&["admin", "viewer"]));
        assert!(!auth_context.has_any_role(&["viewer", "guest"]));

        assert!(auth_context.has_all_roles(&["admin", "editor"]));
        assert!(!auth_context.has_all_roles(&["admin", "viewer"]));
    }

    #[test]
    fn test_claims_creation() {
        let user_id = "user123".to_string();
        let tenant_id = TenantId::new(Uuid::now_v7());
        let expiration_secs = 3600;
        let clock = test_clocks::valid();

        let claims = Claims::new(user_id.clone(), Some(tenant_id), expiration_secs, &clock)
            .with_role("admin".to_string())
            .with_roles(vec!["editor".to_string(), "viewer".to_string()]);

        assert_eq!(claims.sub, user_id);
        assert_eq!(claims.tenant_id(), Some(tenant_id));
        assert_eq!(claims.roles.len(), 3);
        assert!(claims.roles.contains(&"admin".to_string()));
        assert!(!claims.is_expired(&clock));
    }

    #[test]
    fn test_production_validation_allows_secure_secret() {
        let _env_lock = ENV_MUTEX.lock().expect("env mutex should not be poisoned");
        let _env_guard = EnvVarGuard::set("CALIBER_ENVIRONMENT", Some("production"));
        let config = AuthConfig {
            jwt_secret: JwtSecret::new(
                "this-is-a-very-secure-secret-that-is-at-least-32-characters-long".to_string(),
            )
            .expect("test secret should be valid"),
            ..Default::default()
        };

        // Should succeed
        assert!(config.validate_for_production().is_ok());
    }

    #[test]
    fn test_production_validation_rejects_insecure_default() {
        let _env_lock = ENV_MUTEX.lock().expect("env mutex should not be poisoned");
        let _env_guard = EnvVarGuard::set("CALIBER_ENVIRONMENT", Some("production"));
        let _secret_guard = EnvVarGuard::set("CALIBER_JWT_SECRET", None);
        let config = AuthConfig::default(); // Uses insecure default

        // Should fail
        assert!(config.validate_for_production().is_err());
    }

    #[test]
    fn test_production_validation_rejects_short_secret() {
        let _env_lock = ENV_MUTEX.lock().expect("env mutex should not be poisoned");
        let _env_guard = EnvVarGuard::set("CALIBER_ENVIRONMENT", Some("production"));
        let config = AuthConfig {
            jwt_secret: JwtSecret::new("short".to_string()).expect("test secret should be valid"),
            ..Default::default()
        };

        // Should fail
        assert!(config.validate_for_production().is_err());
    }

    #[test]
    fn test_production_validation_allows_development() {
        let _env_lock = ENV_MUTEX.lock().expect("env mutex should not be poisoned");
        let _env_guard = EnvVarGuard::set("CALIBER_ENVIRONMENT", Some("development"));
        let config = AuthConfig::default(); // Uses insecure default

        // Should not fail in development
        assert!(config.validate_for_production().is_ok());
    }

    #[test]
    fn test_production_validation_without_env_var() {
        let _env_lock = ENV_MUTEX.lock().expect("env mutex should not be poisoned");
        let _env_guard = EnvVarGuard::set("CALIBER_ENVIRONMENT", None);
        let config = AuthConfig::default(); // Uses insecure default

        // Should not fail when no environment is set (defaults to development)
        assert!(config.validate_for_production().is_ok());
    }

    #[test]
    fn test_clock_skew_tolerance() -> ApiResult<()> {
        let mut config = test_config();
        config.jwt_clock_skew_secs = 60; // 60 seconds leeway

        // Generate token with current clock
        let token = generate_jwt_token(&config, "user123".to_string(), None, vec![])?;

        // Move clock 30 seconds forward (within leeway)
        let future_clock = FixedClock(config.clock.now_epoch_secs() + 30);
        config.clock = Arc::new(future_clock);

        // Should still be valid
        let result = validate_jwt_token(&config, &token);
        assert!(result.is_ok());

        Ok(())
    }

    #[test]
    fn test_clock_skew_beyond_tolerance() -> ApiResult<()> {
        let mut config = test_config();
        config.jwt_clock_skew_secs = 60;
        config.jwt_expiration_secs = 100; // Short-lived token

        // Generate token
        let token = generate_jwt_token(&config, "user123".to_string(), None, vec![])?;

        // Move clock way beyond expiration + leeway
        let far_future_clock = FixedClock(config.clock.now_epoch_secs() + 200);
        config.clock = Arc::new(far_future_clock);

        // Should fail - expired beyond leeway
        let result = validate_jwt_token(&config, &token);
        assert!(result.is_err());

        if let Err(e) = result {
            assert_eq!(e.code, crate::error::ErrorCode::TokenExpired);
        }

        Ok(())
    }

    #[test]
    fn test_pre_epoch_clock_fails_loud() -> ApiResult<()> {
        let mut config = test_config();

        // Generate valid token with normal clock
        let token = generate_jwt_token(&config, "user123".to_string(), None, vec![])?;

        // Now use a broken clock (pre-1970)
        config.clock = Arc::new(FixedClock(-1000));

        // Should fail with internal error, not panic
        let result = validate_jwt_token(&config, &token);
        assert!(result.is_err());

        if let Err(e) = result {
            assert_eq!(e.code, crate::error::ErrorCode::InternalError);
            assert!(e.message.contains("time configuration error"));
        }

        Ok(())
    }
}
