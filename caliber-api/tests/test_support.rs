use std::sync::Arc;

use caliber_api::auth::{AuthContext, AuthMethod};
use caliber_api::db::{DbClient, DbConfig};
use caliber_api::ws::WsState;
use caliber_core::EntityId;
use caliber_pcp::{
    AntiSprawlConfig, ConflictResolution, ContextDagConfig, DosageConfig, GroundingConfig,
    LintingConfig, PCPConfig, PCPRuntime, PruneStrategy, RecoveryConfig, RecoveryFrequency,
    StalenessConfig,
};
use uuid::Uuid;

pub fn test_db_client() -> DbClient {
    let config = DbConfig::from_env();
    DbClient::from_config(&config).expect("Failed to create database client")
}

pub fn test_ws_state(capacity: usize) -> Arc<WsState> {
    Arc::new(WsState::new(capacity))
}

pub fn test_pcp_runtime() -> Arc<PCPRuntime> {
    Arc::new(PCPRuntime::new(make_test_pcp_config()).expect("Failed to create PCP runtime"))
}

/// Create a test AuthContext with a random tenant_id.
/// This is used for testing route handlers that require authentication.
pub fn test_auth_context() -> AuthContext {
    AuthContext {
        user_id: "test-user".to_string(),
        tenant_id: EntityId::from(Uuid::now_v7()),
        roles: vec![],
        auth_method: AuthMethod::Jwt,
        email: Some("test@example.com".to_string()),
        first_name: Some("Test".to_string()),
        last_name: Some("User".to_string()),
    }
}

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

fn make_test_pcp_config() -> PCPConfig {
    PCPConfig {
        context_dag: ContextDagConfig {
            max_depth: 10,
            prune_strategy: PruneStrategy::OldestFirst,
        },
        recovery: RecoveryConfig {
            enabled: true,
            frequency: RecoveryFrequency::OnScopeClose,
            max_checkpoints: 5,
        },
        dosage: DosageConfig {
            max_tokens_per_scope: 8000,
            max_artifacts_per_scope: 100,
            max_notes_per_trajectory: 500,
        },
        anti_sprawl: AntiSprawlConfig {
            max_trajectory_depth: 5,
            max_concurrent_scopes: 10,
        },
        grounding: GroundingConfig {
            require_artifact_backing: false,
            contradiction_threshold: 0.85,
            conflict_resolution: ConflictResolution::LastWriteWins,
        },
        linting: LintingConfig {
            max_artifact_size: 1024 * 1024,
            min_confidence_threshold: 0.3,
        },
        staleness: StalenessConfig { stale_hours: 24 * 30 },
    }
}
