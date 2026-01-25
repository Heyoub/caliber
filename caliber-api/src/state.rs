//! Shared application state for Axum routers.

use std::sync::Arc;

use caliber_pcp::PCPRuntime;
use caliber_storage::{InMemoryChangeJournal, InMemoryEventDag, LmdbCacheBackend, ReadThroughCache};

use crate::cached_db::CachedDbClient;
use crate::db::DbClient;
use crate::routes::billing::BillingState;
use crate::routes::graphql::CaliberSchema;
use crate::routes::mcp::McpState;
use crate::routes::webhooks::WebhookState;
use crate::ws::WsState;

/// Type alias for the EventDag implementation used in the API.
/// Currently uses InMemoryEventDag for development/testing.
/// In production, this would be replaced with HybridDag (LMDB + PostgreSQL).
pub type ApiEventDag = InMemoryEventDag<serde_json::Value>;

/// Type alias for the ReadThroughCache implementation used in the API.
///
/// This cache provides read-through semantics with:
/// - LMDB as the cache backend (fast, memory-mapped, persistent across restarts)
/// - InMemoryChangeJournal for cache invalidation (for now; will be replaced with
///   PostgreSQL-backed journal in production for distributed correctness)
///
/// The cache layer (Three Dragons architecture) sits between the API and PostgreSQL,
/// providing sub-millisecond reads for hot data while maintaining correctness
/// guarantees via the change journal.
pub type ApiCache = ReadThroughCache<LmdbCacheBackend, InMemoryChangeJournal>;

/// Application-wide state shared across all routes.
#[derive(Clone)]
pub struct AppState {
    /// Raw database client (for operations that don't need caching).
    pub db: DbClient,
    /// Cached database client (for transparent read-through caching).
    ///
    /// Routes should prefer using `cached_db` for get operations to benefit
    /// from the LMDB cache layer. The cache is transparent: routes call
    /// `cached_db.trajectory_get()` unchanged, and the cache is checked first.
    pub cached_db: CachedDbClient,
    pub ws: Arc<WsState>,
    pub pcp: Arc<PCPRuntime>,
    pub webhook_state: Arc<WebhookState>,
    pub graphql_schema: CaliberSchema,
    pub billing_state: Arc<BillingState>,
    pub mcp_state: Arc<McpState>,
    pub start_time: std::time::Instant,
    /// Event DAG for audit trail and event sourcing.
    /// All mutations emit events here for replay/audit capabilities.
    pub event_dag: Arc<ApiEventDag>,
    /// Read-through cache for hot data.
    /// Uses LMDB backend with InMemoryChangeJournal for invalidation.
    /// Part of the Three Dragons architecture for sub-millisecond reads.
    pub cache: Arc<ApiCache>,
    #[cfg(feature = "workos")]
    pub workos_config: Option<crate::workos_auth::WorkOsConfig>,
}

// Use macro to reduce boilerplate for FromRef implementations
crate::impl_from_ref!(DbClient, db);
crate::impl_from_ref!(CachedDbClient, cached_db);
crate::impl_from_ref!(Arc<WsState>, ws);
crate::impl_from_ref!(Arc<PCPRuntime>, pcp);
crate::impl_from_ref!(Arc<WebhookState>, webhook_state);
crate::impl_from_ref!(CaliberSchema, graphql_schema);
crate::impl_from_ref!(Arc<BillingState>, billing_state);
crate::impl_from_ref!(Arc<McpState>, mcp_state);
crate::impl_from_ref!(std::time::Instant, start_time);
crate::impl_from_ref!(Arc<ApiEventDag>, event_dag);
crate::impl_from_ref!(Arc<ApiCache>, cache);

// WorkOS configuration is accessed directly from AppState in route handlers.
// The workos_config field is an Option<WorkOsConfig> that handlers check at runtime.
