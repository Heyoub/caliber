use caliber_api::auth::{AuthContext, AuthMethod};
use caliber_core::TenantId;

/// Create a test AuthContext with a random tenant_id.
/// This is used for testing route handlers that require authentication.
pub fn test_auth_context() -> AuthContext {
    AuthContext {
        user_id: "test-user".to_string(),
        tenant_id: TenantId::now_v7().as_uuid(),
        roles: vec![],
        auth_method: AuthMethod::Jwt,
        email: Some("test@example.com".to_string()),
        first_name: Some("Test".to_string()),
        last_name: Some("User".to_string()),
    }
}

