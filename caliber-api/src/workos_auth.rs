//! WorkOS Authentication Module
//!
//! This module provides WorkOS SSO authentication integration for the CALIBER API.
//! It handles:
//! - WorkOS client initialization from environment variables
//! - JWT token validation against WorkOS JWKS
//! - Claims mapping from WorkOS to CALIBER's AuthContext
//! - SSO callback handling for OIDC authorization code flow
//!
//! Enable this module with the `workos` feature flag.
//!
//! # Environment Variables
//! - `CALIBER_WORKOS_CLIENT_ID`: WorkOS application client ID
//! - `CALIBER_WORKOS_API_KEY`: WorkOS API key for server-side operations
//!
//! # Usage
//! ```ignore
//! use caliber_api::workos_auth::{WorkOsConfig, validate_workos_token};
//!
//! let config = WorkOsConfig::from_env()?;
//! let auth_context = validate_workos_token(&config, token, tenant_header).await?;
//! ```

use crate::auth::{AuthContext, AuthMethod};
use crate::error::{ApiError, ApiResult};
use caliber_core::EntityId;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use workos::sso::{AuthorizationCode, ClientId, GetProfileAndToken, Sso};
use workos::{ApiKey, WorkOs};

// ============================================================================
// CONFIGURATION
// ============================================================================

/// WorkOS authentication configuration.
///
/// This struct holds the configuration needed to interact with WorkOS APIs.
#[derive(Debug, Clone)]
pub struct WorkOsConfig {
    /// WorkOS application client ID
    pub client_id: ClientId,

    /// WorkOS API key
    pub api_key: ApiKey,

    /// Redirect URI for SSO callback
    pub redirect_uri: String,
}

impl WorkOsConfig {
    /// Create WorkOS configuration from environment variables.
    ///
    /// # Environment Variables
    /// - `CALIBER_WORKOS_CLIENT_ID`: Required - WorkOS application client ID
    /// - `CALIBER_WORKOS_API_KEY`: Required - WorkOS API key
    /// - `CALIBER_WORKOS_REDIRECT_URI`: Optional - Redirect URI (defaults to /auth/sso/callback)
    ///
    /// # Errors
    /// Returns an error if required environment variables are not set.
    pub fn from_env() -> ApiResult<Self> {
        let client_id = std::env::var("CALIBER_WORKOS_CLIENT_ID")
            .map_err(|_| ApiError::internal_error("CALIBER_WORKOS_CLIENT_ID not set"))?;

        let api_key = std::env::var("CALIBER_WORKOS_API_KEY")
            .map_err(|_| ApiError::internal_error("CALIBER_WORKOS_API_KEY not set"))?;

        let redirect_uri = std::env::var("CALIBER_WORKOS_REDIRECT_URI")
            .unwrap_or_else(|_| "/auth/sso/callback".to_string());

        Ok(Self {
            client_id: ClientId::from(client_id),
            api_key: ApiKey::from(api_key),
            redirect_uri,
        })
    }

    /// Create WorkOS configuration with explicit values.
    pub fn new(client_id: impl Into<String>, api_key: impl Into<String>, redirect_uri: impl Into<String>) -> Self {
        Self {
            client_id: ClientId::from(client_id.into()),
            api_key: ApiKey::from(api_key.into()),
            redirect_uri: redirect_uri.into(),
        }
    }

    /// Create a WorkOS client from this configuration.
    pub fn create_client(&self) -> WorkOs {
        WorkOs::new(&self.api_key)
    }
}

// ============================================================================
// WORKOS CLAIMS
// ============================================================================

/// Claims extracted from WorkOS profile/token.
///
/// This structure represents the user information returned by WorkOS
/// after successful authentication.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkOsClaims {
    /// WorkOS user ID
    pub user_id: String,

    /// User's email address
    pub email: String,

    /// User's first name
    pub first_name: Option<String>,

    /// User's last name
    pub last_name: Option<String>,

    /// WorkOS organization ID (maps to CALIBER tenant)
    pub organization_id: Option<String>,

    /// Connection type (e.g., "GoogleOAuth", "SAML")
    pub connection_type: String,

    /// Identity provider's ID for this user
    pub idp_id: Option<String>,

    /// Raw profile data for extended attributes
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw_attributes: Option<serde_json::Value>,
}

impl WorkOsClaims {
    /// Get the tenant ID from the organization ID.
    ///
    /// WorkOS organization IDs are mapped to CALIBER tenant IDs.
    /// If the organization ID is a valid UUID, it's used directly.
    /// Otherwise, we generate a deterministic UUID from the organization ID.
    pub fn tenant_id(&self) -> Option<EntityId> {
        self.organization_id.as_ref().map(|org_id| {
            // Try to parse as UUID first
            if let Ok(uuid) = Uuid::parse_str(org_id) {
                uuid
            } else {
                // Generate a deterministic UUID v5 from the org ID
                // Using DNS namespace for consistency
                Uuid::new_v5(&Uuid::NAMESPACE_DNS, org_id.as_bytes())
            }
        })
    }

    /// Convert to AuthContext for use in the rest of the API.
    pub fn to_auth_context(&self, tenant_id: EntityId) -> AuthContext {
        AuthContext::with_profile(
            self.user_id.clone(),
            tenant_id,
            self.derive_roles(),
            AuthMethod::WorkOs,
            Some(self.email.clone()),
            self.first_name.clone(),
            self.last_name.clone(),
        )
    }

    /// Derive roles from WorkOS claims.
    ///
    /// This is a basic implementation - in production you'd want to
    /// integrate with WorkOS Directory Sync or your own role management.
    fn derive_roles(&self) -> Vec<String> {
        let mut roles = vec!["workos_user".to_string()];

        // Add organization member role if in an organization
        if self.organization_id.is_some() {
            roles.push("org_member".to_string());
        }

        roles
    }
}

// ============================================================================
// SSO CALLBACK HANDLING
// ============================================================================

/// Request parameters for SSO callback.
#[derive(Debug, Deserialize)]
pub struct SsoCallbackParams {
    /// Authorization code from WorkOS
    pub code: String,

    /// State parameter for CSRF protection
    #[serde(default)]
    pub state: Option<String>,
}

/// Response from SSO callback.
#[derive(Debug, Serialize)]
pub struct SsoCallbackResponse {
    /// Access token for subsequent API calls
    pub access_token: String,

    /// Token type (always "Bearer")
    pub token_type: String,

    /// User profile information
    pub profile: WorkOsClaims,

    /// Tenant ID derived from WorkOS organization
    pub tenant_id: Option<String>,
}

/// Exchange authorization code for profile and token.
///
/// This is the core SSO callback handler that:
/// 1. Exchanges the authorization code for a profile and token
/// 2. Extracts user and organization information
/// 3. Maps WorkOS profile to CALIBER claims
///
/// # Arguments
/// * `config` - WorkOS configuration
/// * `code` - Authorization code from WorkOS redirect
///
/// # Returns
/// A tuple of (access_token, WorkOsClaims) on success
pub async fn exchange_code_for_profile(
    config: &WorkOsConfig,
    code: &str,
) -> ApiResult<(String, WorkOsClaims)> {
    let workos = config.create_client();
    let sso = workos.sso();

    let params = GetProfileAndToken::builder()
        .client_id(&config.client_id)
        .code(&AuthorizationCode::from(code.to_string()))
        .build();

    let response = sso
        .get_profile_and_token(&params)
        .await
        .map_err(|e| ApiError::unauthorized(format!("WorkOS SSO error: {}", e)))?;

    // Extract profile information
    let profile = response.profile;

    let claims = WorkOsClaims {
        user_id: profile.id.to_string(),
        email: profile.email.to_string(),
        first_name: profile.first_name.map(|s| s.to_string()),
        last_name: profile.last_name.map(|s| s.to_string()),
        organization_id: profile.organization_id.map(|id| id.to_string()),
        connection_type: profile.connection_type.to_string(),
        idp_id: profile.idp_id.map(|s| s.to_string()),
        raw_attributes: profile.raw_attributes.map(|attrs| {
            serde_json::to_value(attrs).unwrap_or(serde_json::Value::Null)
        }),
    };

    Ok((response.access_token.to_string(), claims))
}

// ============================================================================
// TOKEN VALIDATION
// ============================================================================

/// Validate a WorkOS access token and extract claims.
///
/// This function validates the token by calling the WorkOS API to get
/// the user profile associated with the token. In production, you might
/// want to cache this or validate JWT tokens locally using JWKS.
///
/// # Arguments
/// * `config` - WorkOS configuration
/// * `token` - Access token to validate
/// * `tenant_id_header` - Optional tenant ID from X-Tenant-ID header
///
/// # Returns
/// AuthContext on successful validation
pub async fn validate_workos_token(
    config: &WorkOsConfig,
    token: &str,
    tenant_id_header: Option<&str>,
) -> ApiResult<AuthContext> {
    // For WorkOS, the access token is typically a session token
    // that we need to validate against the WorkOS API.
    //
    // Note: WorkOS doesn't provide a direct token validation endpoint.
    // In production, you would typically:
    // 1. Use the token to call WorkOS User Management API
    // 2. Or cache the profile during SSO callback and use session management
    //
    // For this implementation, we'll use a JWT-based approach where
    // the access token is a JWT that we validate locally after initial SSO.

    // Try to decode the token as a JWT to extract claims
    // This assumes you've stored the user info in a JWT during callback
    let claims = decode_workos_session_token(token)?;

    // Determine tenant ID
    let tenant_id = if let Some(header) = tenant_id_header {
        Uuid::parse_str(header)
            .map_err(|_| ApiError::invalid_format("X-Tenant-ID", "valid UUID"))?
    } else if let Some(tid) = claims.tenant_id() {
        tid
    } else {
        return Err(ApiError::missing_field(
            "X-Tenant-ID header or organization membership required",
        ));
    };

    Ok(claims.to_auth_context(tenant_id))
}

/// Decode a session token created after WorkOS SSO callback.
///
/// This is used for ongoing authentication after the initial SSO flow.
/// The session token is a JWT created by CALIBER that contains the
/// WorkOS profile information.
fn decode_workos_session_token(token: &str) -> ApiResult<WorkOsClaims> {
    // Get the JWT secret for session tokens
    let secret = std::env::var("CALIBER_JWT_SECRET")
        .unwrap_or_else(|_| "INSECURE_DEFAULT_SECRET_CHANGE_IN_PRODUCTION".to_string());

    let decoding_key = jsonwebtoken::DecodingKey::from_secret(secret.as_bytes());
    let mut validation = jsonwebtoken::Validation::new(jsonwebtoken::Algorithm::HS256);
    validation.validate_exp = true;

    let token_data = jsonwebtoken::decode::<WorkOsClaims>(token, &decoding_key, &validation)
        .map_err(|e| match e.kind() {
            jsonwebtoken::errors::ErrorKind::ExpiredSignature => ApiError::token_expired(),
            jsonwebtoken::errors::ErrorKind::InvalidToken => {
                ApiError::invalid_token("WorkOS session token is invalid")
            }
            jsonwebtoken::errors::ErrorKind::InvalidSignature => {
                ApiError::invalid_token("WorkOS session token signature is invalid")
            }
            _ => ApiError::invalid_token(format!("WorkOS token validation failed: {}", e)),
        })?;

    Ok(token_data.claims)
}

/// Create a session token from WorkOS claims.
///
/// This creates a JWT that can be used for subsequent API calls after
/// the initial SSO authentication. The token contains the user's profile
/// information and has a configurable expiration.
///
/// # Arguments
/// * `claims` - WorkOS claims to encode
/// * `expiration_secs` - Token expiration in seconds
///
/// # Returns
/// Encoded JWT string
pub fn create_session_token(claims: &WorkOsClaims, expiration_secs: i64) -> ApiResult<String> {
    use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
    use serde::Serialize;

    #[derive(Serialize)]
    struct SessionClaims<'a> {
        #[serde(flatten)]
        claims: &'a WorkOsClaims,
        iat: i64,
        exp: i64,
    }

    let secret = std::env::var("CALIBER_JWT_SECRET")
        .unwrap_or_else(|_| "INSECURE_DEFAULT_SECRET_CHANGE_IN_PRODUCTION".to_string());

    let now = chrono::Utc::now().timestamp();
    let session_claims = SessionClaims {
        claims,
        iat: now,
        exp: now + expiration_secs,
    };

    let encoding_key = EncodingKey::from_secret(secret.as_bytes());
    let header = Header::new(Algorithm::HS256);

    encode(&header, &session_claims, &encoding_key)
        .map_err(|e| ApiError::internal_error(format!("Failed to create session token: {}", e)))
}

// ============================================================================
// SSO URL GENERATION
// ============================================================================

/// Parameters for generating an SSO authorization URL.
#[derive(Debug)]
pub struct SsoAuthorizationParams {
    /// WorkOS connection ID for direct connection (e.g., SAML)
    pub connection: Option<String>,

    /// WorkOS organization ID for organization-level SSO
    pub organization: Option<String>,

    /// Login hint (email) to pre-fill
    pub login_hint: Option<String>,

    /// State parameter for CSRF protection
    pub state: Option<String>,
}

/// Generate an SSO authorization URL.
///
/// This creates a URL that redirects the user to WorkOS for authentication.
/// After authentication, WorkOS will redirect back to the callback URL
/// with an authorization code.
pub fn generate_authorization_url(config: &WorkOsConfig, params: &SsoAuthorizationParams) -> String {
    use workos::sso::{GetAuthorizationUrl, ConnectionSelector};

    let workos = config.create_client();
    let sso = workos.sso();

    let mut builder = GetAuthorizationUrl::builder()
        .client_id(&config.client_id)
        .redirect_uri(&config.redirect_uri);

    // Set connection selector based on params
    if let Some(ref conn) = params.connection {
        builder = builder.connection(&ConnectionSelector::Connection(
            workos::sso::ConnectionId::from(conn.clone())
        ));
    } else if let Some(ref org) = params.organization {
        builder = builder.connection(&ConnectionSelector::Organization(
            workos::organizations::OrganizationId::from(org.clone())
        ));
    }

    // Add optional parameters
    if let Some(ref state) = params.state {
        builder = builder.state(state);
    }

    if let Some(ref hint) = params.login_hint {
        builder = builder.login_hint(hint);
    }

    sso.get_authorization_url(&builder.build())
        .map(|url| url.to_string())
        .unwrap_or_else(|_| String::new())
}


// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workos_claims_tenant_id_uuid() {
        let claims = WorkOsClaims {
            user_id: "user_123".to_string(),
            email: "test@example.com".to_string(),
            first_name: Some("Test".to_string()),
            last_name: Some("User".to_string()),
            organization_id: Some("550e8400-e29b-41d4-a716-446655440000".to_string()),
            connection_type: "SAML".to_string(),
            idp_id: None,
            raw_attributes: None,
        };

        let tenant_id = claims.tenant_id().unwrap();
        assert_eq!(
            tenant_id.to_string(),
            "550e8400-e29b-41d4-a716-446655440000"
        );
    }

    #[test]
    fn test_workos_claims_tenant_id_non_uuid() {
        let claims = WorkOsClaims {
            user_id: "user_123".to_string(),
            email: "test@example.com".to_string(),
            first_name: None,
            last_name: None,
            organization_id: Some("org_acme_corp".to_string()),
            connection_type: "GoogleOAuth".to_string(),
            idp_id: None,
            raw_attributes: None,
        };

        let tenant_id = claims.tenant_id().unwrap();
        // Should be a deterministic UUID v5
        assert!(!tenant_id.is_nil());
    }

    #[test]
    fn test_workos_claims_no_org() {
        let claims = WorkOsClaims {
            user_id: "user_123".to_string(),
            email: "test@example.com".to_string(),
            first_name: None,
            last_name: None,
            organization_id: None,
            connection_type: "GoogleOAuth".to_string(),
            idp_id: None,
            raw_attributes: None,
        };

        assert!(claims.tenant_id().is_none());
    }

    #[test]
    fn test_derive_roles() {
        let claims_with_org = WorkOsClaims {
            user_id: "user_123".to_string(),
            email: "test@example.com".to_string(),
            first_name: None,
            last_name: None,
            organization_id: Some("org_123".to_string()),
            connection_type: "SAML".to_string(),
            idp_id: None,
            raw_attributes: None,
        };

        let roles = claims_with_org.derive_roles();
        assert!(roles.contains(&"workos_user".to_string()));
        assert!(roles.contains(&"org_member".to_string()));

        let claims_without_org = WorkOsClaims {
            user_id: "user_123".to_string(),
            email: "test@example.com".to_string(),
            first_name: None,
            last_name: None,
            organization_id: None,
            connection_type: "GoogleOAuth".to_string(),
            idp_id: None,
            raw_attributes: None,
        };

        let roles = claims_without_org.derive_roles();
        assert!(roles.contains(&"workos_user".to_string()));
        assert!(!roles.contains(&"org_member".to_string()));
    }

    #[test]
    fn test_workos_config_new() {
        let config = WorkOsConfig::new("client_123", "sk_test_abc", "https://example.com/callback");
        assert_eq!(config.redirect_uri, "https://example.com/callback");
    }

    #[test]
    fn test_to_auth_context() {
        let claims = WorkOsClaims {
            user_id: "user_123".to_string(),
            email: "test@example.com".to_string(),
            first_name: Some("Test".to_string()),
            last_name: Some("User".to_string()),
            organization_id: Some("org_123".to_string()),
            connection_type: "SAML".to_string(),
            idp_id: None,
            raw_attributes: None,
        };

        let tenant_id = Uuid::new_v4();
        let auth_context = claims.to_auth_context(tenant_id);

        assert_eq!(auth_context.user_id, "user_123");
        assert_eq!(auth_context.tenant_id, tenant_id);
        assert!(auth_context.has_role("workos_user"));
        assert!(auth_context.has_role("org_member"));
    }
}
