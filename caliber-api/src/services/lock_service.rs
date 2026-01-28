//! Lock Service
//!
//! Business logic for lock operations, extracted from LockResponse.

use caliber_core::AgentId;
use std::time::Duration;

use crate::db::DbClient;
use crate::error::{ApiError, ApiResult};
use crate::types::LockResponse;

/// Check if a lock is currently held (not expired).
pub fn is_lock_held(lock: &LockResponse) -> bool {
    chrono::Utc::now() < lock.expires_at
}

/// Release a lock.
///
/// # Arguments
/// - `db`: Database client for persisting the update
/// - `lock`: The lock to release
/// - `releasing_agent_id`: ID of the agent releasing the lock
///
/// # Errors
/// Returns error if the agent is not the lock holder or lock is expired.
pub async fn release_lock(
    db: &DbClient,
    lock: &LockResponse,
    releasing_agent_id: AgentId,
) -> ApiResult<()> {
    // Verify the releasing agent is the lock holder
    if lock.holder_agent_id != releasing_agent_id {
        return Err(ApiError::forbidden(
            "Only the lock holder can release this lock",
        ));
    }

    if !is_lock_held(lock) {
        return Err(ApiError::state_conflict("Lock has already expired"));
    }

    // Delete the lock record (release = delete for locks)
    db.delete::<LockResponse>(lock.lock_id, lock.tenant_id)
        .await?;
    Ok(())
}

/// Extend a lock's expiration time.
///
/// # Arguments
/// - `db`: Database client for persisting the update
/// - `lock`: The lock to extend
/// - `additional`: Additional duration to add to expiration
///
/// # Errors
/// Returns error if the lock has expired.
pub async fn extend_lock(
    db: &DbClient,
    lock: &LockResponse,
    additional: Duration,
) -> ApiResult<LockResponse> {
    if !is_lock_held(lock) {
        return Err(ApiError::state_conflict("Cannot extend an expired lock"));
    }

    let new_expires = lock.expires_at
        + chrono::Duration::from_std(additional)
            .map_err(|e| ApiError::invalid_input(format!("Invalid duration: {}", e)))?;

    let updates = serde_json::json!({
        "expires_at": new_expires.to_rfc3339()
    });

    db.update_raw::<LockResponse>(lock.lock_id, updates, lock.tenant_id)
        .await
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{DbClient, DbConfig};
    use crate::error::ErrorCode;
    use crate::types::{
        AgentResponse, MemoryAccessRequest, MemoryPermissionRequest, RegisterAgentRequest,
    };
    use caliber_core::{AgentId, EntityIdType, LockId, TenantId};
    use chrono::{Duration as ChronoDuration, Utc};
    use uuid::Uuid;

    fn dummy_db() -> DbClient {
        DbClient::from_config(&DbConfig::default()).expect("db client")
    }

    fn sample_lock(expires_at: chrono::DateTime<Utc>, holder: AgentId) -> LockResponse {
        LockResponse {
            tenant_id: TenantId::now_v7(),
            lock_id: LockId::now_v7(),
            resource_type: "resource".to_string(),
            resource_id: Uuid::now_v7(),
            holder_agent_id: holder,
            acquired_at: Utc::now(),
            expires_at,
            mode: "exclusive".to_string(),
        }
    }

    #[test]
    fn test_is_lock_held_checks_expiration() {
        let holder = AgentId::now_v7();
        let future = Utc::now() + ChronoDuration::seconds(30);
        let past = Utc::now() - ChronoDuration::seconds(30);
        assert!(is_lock_held(&sample_lock(future, holder)));
        assert!(!is_lock_held(&sample_lock(past, holder)));
    }

    #[tokio::test]
    async fn test_release_lock_rejects_wrong_holder() {
        let db = dummy_db();
        let holder = AgentId::now_v7();
        let lock = sample_lock(Utc::now() + ChronoDuration::seconds(30), holder);
        let err = release_lock(&db, &lock, AgentId::now_v7())
            .await
            .unwrap_err();
        assert_eq!(err.code, ErrorCode::Forbidden);
    }

    #[tokio::test]
    async fn test_release_lock_rejects_expired_lock() {
        let db = dummy_db();
        let holder = AgentId::now_v7();
        let lock = sample_lock(Utc::now() - ChronoDuration::seconds(5), holder);
        let err = release_lock(&db, &lock, holder).await.unwrap_err();
        assert_eq!(err.code, ErrorCode::StateConflict);
    }

    #[tokio::test]
    async fn test_extend_lock_rejects_expired_lock() {
        let db = dummy_db();
        let holder = AgentId::now_v7();
        let lock = sample_lock(Utc::now() - ChronoDuration::seconds(5), holder);
        let err = extend_lock(&db, &lock, Duration::from_secs(5))
            .await
            .unwrap_err();
        assert_eq!(err.code, ErrorCode::StateConflict);
    }

    struct DbTestContext {
        db: DbClient,
        tenant_id: TenantId,
    }

    async fn db_test_context() -> Option<DbTestContext> {
        if std::env::var("DB_TESTS").ok().as_deref() != Some("1") {
            return None;
        }

        let db = DbClient::from_config(&DbConfig::from_env()).ok()?;
        let conn = db.get_conn().await.ok()?;
        let has_fn = conn
            .query_opt(
                "SELECT 1 FROM pg_proc WHERE proname = 'caliber_tenant_create' LIMIT 1",
                &[],
            )
            .await
            .ok()
            .flatten()
            .is_some();
        if !has_fn {
            return None;
        }

        let tenant_id = db
            .tenant_create("test-lock-service", None, None)
            .await
            .ok()?;

        Some(DbTestContext { db, tenant_id })
    }

    async fn register_agent(db: &DbClient, tenant_id: TenantId, agent_type: &str) -> AgentResponse {
        let req = RegisterAgentRequest {
            agent_type: agent_type.to_string(),
            capabilities: vec!["read".to_string()],
            memory_access: MemoryAccessRequest {
                read: vec![MemoryPermissionRequest {
                    memory_type: "artifact".to_string(),
                    scope: "own".to_string(),
                    filter: None,
                }],
                write: vec![MemoryPermissionRequest {
                    memory_type: "artifact".to_string(),
                    scope: "own".to_string(),
                    filter: None,
                }],
            },
            can_delegate_to: vec![],
            reports_to: None,
        };

        db.agent_register(&req, tenant_id)
            .await
            .expect("register agent")
    }

    #[tokio::test]
    async fn test_extend_and_release_lock_db_backed() {
        let Some(ctx) = db_test_context().await else {
            return;
        };

        let agent = register_agent(&ctx.db, ctx.tenant_id, "lock-service").await;
        let acquire_req = crate::types::AcquireLockRequest {
            resource_type: "resource".to_string(),
            resource_id: Uuid::now_v7(),
            holder_agent_id: agent.agent_id,
            timeout_ms: 5_000,
            mode: "exclusive".to_string(),
        };

        let lock = ctx
            .db
            .lock_acquire(&acquire_req, ctx.tenant_id)
            .await
            .expect("acquire lock");
        let original_expires = lock.expires_at;

        let extended = extend_lock(&ctx.db, &lock, Duration::from_secs(1))
            .await
            .expect("extend lock");
        assert!(extended.expires_at > original_expires);

        release_lock(&ctx.db, &extended, agent.agent_id)
            .await
            .expect("release lock");

        let fetched = ctx.db.lock_get(extended.lock_id).await.expect("lock get");
        assert!(fetched.is_none());

        ctx.db
            .delete::<AgentResponse>(agent.agent_id, ctx.tenant_id)
            .await
            .ok();
    }
}
