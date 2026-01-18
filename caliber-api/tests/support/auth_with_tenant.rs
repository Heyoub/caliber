use caliber_api::auth::{AuthContext, AuthMethod};
use caliber_core::EntityId;

/// Create a test AuthContext with a specific tenant_id.
pub fn test_auth_context_with_tenant(tenant_id: EntityId) -> AuthContext {
    AuthContext {
        user_id: "test-user".to_string(),
        tenant_id,
        roles: vec![],
        auth_method: AuthMethod::Jwt,
        email: Some("test@example.com".to_string()),
        first_name: Some("Test".to_string()),
        last_name: Some("User".to_string()),
    }
}
