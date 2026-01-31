//! Saga Cleanup Background Task
//!
//! This module provides a background task that periodically checks for and
//! cleans up stuck delegations and handoffs. These can become stuck when:
//!
//! - An agent crashes without completing or failing a delegation
//! - Network partitions prevent completion notifications
//! - Bugs in agent code prevent proper state transitions
//!
//! The cleanup task uses the SQL functions defined in V3__distributed_correctness.sql:
//!
//! - `caliber_find_stuck_delegations(interval)`: Find delegations past timeout
//! - `caliber_timeout_delegation(id, reason)`: Mark a delegation as failed
//! - `caliber_find_stuck_handoffs(interval)`: Find handoffs past timeout
//! - `caliber_timeout_handoff(id, reason)`: Mark a handoff as rejected
//!
//! # Configuration
//!
//! The cleanup task is configured via `SagaCleanupConfig`:
//!
//! ```rust
//! use caliber_api::jobs::SagaCleanupConfig;
//! use std::time::Duration;
//!
//! let config = SagaCleanupConfig {
//!     check_interval: Duration::from_secs(60),      // Check every minute
//!     delegation_timeout: Duration::from_secs(3600), // 1 hour default
//!     handoff_timeout: Duration::from_secs(1800),    // 30 minutes default
//!     batch_size: 100,                               // Process up to 100 at a time
//!     idempotency_cleanup_interval: Duration::from_secs(3600), // Hourly
//!     log_timeouts: true,                            // Log timeouts
//! };
//! ```

use crate::constants::{
    DEFAULT_SAGA_BATCH_SIZE, DEFAULT_SAGA_CHECK_INTERVAL_SECS,
    DEFAULT_SAGA_DELEGATION_TIMEOUT_SECS, DEFAULT_SAGA_HANDOFF_TIMEOUT_SECS,
    DEFAULT_SAGA_IDEMPOTENCY_CLEANUP_INTERVAL_SECS,
};
use crate::db::DbClient;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::watch;
use tokio::time::{interval, MissedTickBehavior};

// ============================================================================
// CONFIGURATION
// ============================================================================

/// Configuration for the saga cleanup background task.
#[derive(Debug, Clone)]
pub struct SagaCleanupConfig {
    /// How often to check for stuck sagas (default: 60 seconds)
    pub check_interval: Duration,

    /// Timeout threshold for delegations without explicit timeout
    /// Delegations in active states longer than this are considered stuck
    /// (default: 1 hour)
    pub delegation_timeout: Duration,

    /// Timeout threshold for handoffs without explicit timeout
    /// Handoffs in active states longer than this are considered stuck
    /// (default: 30 minutes)
    pub handoff_timeout: Duration,

    /// Maximum number of stuck sagas to process per check cycle
    /// (default: 100)
    pub batch_size: usize,

    /// How often to clean up expired idempotency keys
    /// (default: 1 hour)
    pub idempotency_cleanup_interval: Duration,

    /// Whether to log each timed-out saga (default: true)
    pub log_timeouts: bool,
}

impl Default for SagaCleanupConfig {
    fn default() -> Self {
        Self {
            check_interval: Duration::from_secs(DEFAULT_SAGA_CHECK_INTERVAL_SECS),
            delegation_timeout: Duration::from_secs(DEFAULT_SAGA_DELEGATION_TIMEOUT_SECS),
            handoff_timeout: Duration::from_secs(DEFAULT_SAGA_HANDOFF_TIMEOUT_SECS),
            batch_size: DEFAULT_SAGA_BATCH_SIZE,
            idempotency_cleanup_interval: Duration::from_secs(
                DEFAULT_SAGA_IDEMPOTENCY_CLEANUP_INTERVAL_SECS,
            ),
            log_timeouts: true,
        }
    }
}

impl SagaCleanupConfig {
    /// Create SagaCleanupConfig from environment variables.
    ///
    /// # Environment Variables
    /// - `CALIBER_SAGA_CHECK_INTERVAL_SECS`: How often to check for stuck sagas (default: 60)
    /// - `CALIBER_SAGA_DELEGATION_TIMEOUT_SECS`: Delegation timeout threshold (default: 3600)
    /// - `CALIBER_SAGA_HANDOFF_TIMEOUT_SECS`: Handoff timeout threshold (default: 1800)
    /// - `CALIBER_SAGA_BATCH_SIZE`: Max sagas to process per cycle (default: 100)
    /// - `CALIBER_SAGA_IDEMPOTENCY_CLEANUP_INTERVAL_SECS`: Idempotency cleanup interval (default: 3600)
    /// - `CALIBER_SAGA_LOG_TIMEOUTS`: Whether to log timeouts (default: true)
    pub fn from_env() -> Self {
        let check_interval = Duration::from_secs(
            std::env::var("CALIBER_SAGA_CHECK_INTERVAL_SECS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(DEFAULT_SAGA_CHECK_INTERVAL_SECS),
        );

        let delegation_timeout = Duration::from_secs(
            std::env::var("CALIBER_SAGA_DELEGATION_TIMEOUT_SECS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(DEFAULT_SAGA_DELEGATION_TIMEOUT_SECS),
        );

        let handoff_timeout = Duration::from_secs(
            std::env::var("CALIBER_SAGA_HANDOFF_TIMEOUT_SECS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(DEFAULT_SAGA_HANDOFF_TIMEOUT_SECS),
        );

        let batch_size = std::env::var("CALIBER_SAGA_BATCH_SIZE")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(DEFAULT_SAGA_BATCH_SIZE);

        let idempotency_cleanup_interval = Duration::from_secs(
            std::env::var("CALIBER_SAGA_IDEMPOTENCY_CLEANUP_INTERVAL_SECS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(DEFAULT_SAGA_IDEMPOTENCY_CLEANUP_INTERVAL_SECS),
        );

        let log_timeouts = std::env::var("CALIBER_SAGA_LOG_TIMEOUTS")
            .ok()
            .map(|s| s.to_lowercase() != "false")
            .unwrap_or(true);

        Self {
            check_interval,
            delegation_timeout,
            handoff_timeout,
            batch_size,
            idempotency_cleanup_interval,
            log_timeouts,
        }
    }

    /// Create a configuration for development/testing with shorter timeouts.
    pub fn development() -> Self {
        Self {
            check_interval: Duration::from_secs(10),
            delegation_timeout: Duration::from_secs(60),
            handoff_timeout: Duration::from_secs(30),
            batch_size: 10,
            idempotency_cleanup_interval: Duration::from_secs(60),
            log_timeouts: true,
        }
    }

    /// Create a configuration for production with longer timeouts.
    pub fn production() -> Self {
        Self {
            check_interval: Duration::from_secs(DEFAULT_SAGA_CHECK_INTERVAL_SECS),
            delegation_timeout: Duration::from_secs(7200), // 2 hours
            handoff_timeout: Duration::from_secs(3600),    // 1 hour
            batch_size: DEFAULT_SAGA_BATCH_SIZE,
            idempotency_cleanup_interval: Duration::from_secs(
                DEFAULT_SAGA_IDEMPOTENCY_CLEANUP_INTERVAL_SECS,
            ),
            log_timeouts: true,
        }
    }
}

// ============================================================================
// METRICS
// ============================================================================

/// Metrics for saga cleanup operations.
///
/// These counters track cleanup activity and can be exposed via Prometheus.
#[derive(Debug, Default)]
pub struct SagaCleanupMetrics {
    /// Total delegations timed out since startup
    pub delegations_timed_out: AtomicU64,

    /// Total handoffs timed out since startup
    pub handoffs_timed_out: AtomicU64,

    /// Total idempotency keys cleaned up since startup
    pub idempotency_keys_cleaned: AtomicU64,

    /// Total cleanup cycles completed
    pub cleanup_cycles: AtomicU64,

    /// Total errors encountered during cleanup
    pub cleanup_errors: AtomicU64,
}

impl SagaCleanupMetrics {
    /// Create new metrics instance.
    pub fn new() -> Self {
        Self::default()
    }

    /// Get current snapshot of all metrics.
    pub fn snapshot(&self) -> SagaCleanupSnapshot {
        SagaCleanupSnapshot {
            delegations_timed_out: self.delegations_timed_out.load(Ordering::Relaxed),
            handoffs_timed_out: self.handoffs_timed_out.load(Ordering::Relaxed),
            idempotency_keys_cleaned: self.idempotency_keys_cleaned.load(Ordering::Relaxed),
            cleanup_cycles: self.cleanup_cycles.load(Ordering::Relaxed),
            cleanup_errors: self.cleanup_errors.load(Ordering::Relaxed),
        }
    }
}

/// Snapshot of cleanup metrics at a point in time.
#[derive(Debug, Clone)]
pub struct SagaCleanupSnapshot {
    pub delegations_timed_out: u64,
    pub handoffs_timed_out: u64,
    pub idempotency_keys_cleaned: u64,
    pub cleanup_cycles: u64,
    pub cleanup_errors: u64,
}

// ============================================================================
// BACKGROUND TASK
// ============================================================================

/// Background task that periodically cleans up stuck sagas.
///
/// This task runs until the shutdown signal is received. It:
///
/// 1. Queries for stuck delegations using `caliber_find_stuck_delegations`
/// 2. Times out each stuck delegation using `caliber_timeout_delegation`
/// 3. Queries for stuck handoffs using `caliber_find_stuck_handoffs`
/// 4. Times out each stuck handoff using `caliber_timeout_handoff`
/// 5. Periodically cleans up expired idempotency keys
///
/// # Arguments
///
/// * `db` - Database client for executing cleanup queries
/// * `config` - Cleanup configuration (intervals, timeouts, batch size)
/// * `shutdown_rx` - Watch receiver for shutdown signal
///
/// # Returns
///
/// Metrics collected during the task's lifetime
///
/// # Example
///
/// ```ignore
/// use tokio::sync::watch;
/// use std::sync::Arc;
///
/// let (shutdown_tx, shutdown_rx) = watch::channel(false);
/// let db = Arc::new(db_client);
/// let config = SagaCleanupConfig::default();
///
/// // Spawn the cleanup task
/// let handle = tokio::spawn(async move {
///     saga_cleanup_task(db, config, shutdown_rx).await
/// });
///
/// // Later, trigger shutdown
/// let _ = shutdown_tx.send(true);
/// let metrics = handle.await.unwrap();
/// println!("Cleaned up {} delegations", metrics.delegations_timed_out.load(Ordering::Relaxed));
/// ```
pub async fn saga_cleanup_task(
    db: Arc<DbClient>,
    config: SagaCleanupConfig,
    mut shutdown_rx: watch::Receiver<bool>,
) -> Arc<SagaCleanupMetrics> {
    let metrics = Arc::new(SagaCleanupMetrics::new());

    // Create interval timers
    let mut cleanup_interval = interval(config.check_interval);
    cleanup_interval.set_missed_tick_behavior(MissedTickBehavior::Skip);

    let mut idempotency_interval = interval(config.idempotency_cleanup_interval);
    idempotency_interval.set_missed_tick_behavior(MissedTickBehavior::Skip);

    tracing::info!(
        check_interval_secs = config.check_interval.as_secs(),
        delegation_timeout_secs = config.delegation_timeout.as_secs(),
        handoff_timeout_secs = config.handoff_timeout.as_secs(),
        "Saga cleanup task started"
    );

    loop {
        tokio::select! {
            // Check for shutdown signal
            _ = shutdown_rx.changed() => {
                if *shutdown_rx.borrow() {
                    tracing::info!("Saga cleanup task shutting down");
                    break;
                }
            }

            // Regular saga cleanup
            _ = cleanup_interval.tick() => {
                cleanup_sagas(&db, &config, &metrics).await;
            }

            // Idempotency key cleanup
            _ = idempotency_interval.tick() => {
                cleanup_idempotency_keys(&db, &metrics).await;
            }
        }
    }

    let snapshot = metrics.snapshot();
    tracing::info!(
        delegations_timed_out = snapshot.delegations_timed_out,
        handoffs_timed_out = snapshot.handoffs_timed_out,
        idempotency_keys_cleaned = snapshot.idempotency_keys_cleaned,
        cleanup_cycles = snapshot.cleanup_cycles,
        cleanup_errors = snapshot.cleanup_errors,
        "Saga cleanup task completed"
    );

    metrics
}

/// Perform one cycle of saga cleanup.
async fn cleanup_sagas(db: &DbClient, config: &SagaCleanupConfig, metrics: &SagaCleanupMetrics) {
    metrics.cleanup_cycles.fetch_add(1, Ordering::Relaxed);

    // Clean up stuck delegations
    let delegation_count = cleanup_stuck_delegations(db, config, metrics).await;

    // Clean up stuck handoffs
    let handoff_count = cleanup_stuck_handoffs(db, config, metrics).await;

    if delegation_count > 0 || handoff_count > 0 {
        tracing::info!(
            delegations = delegation_count,
            handoffs = handoff_count,
            "Saga cleanup cycle completed"
        );
    } else {
        tracing::trace!("Saga cleanup cycle completed with no stuck sagas");
    }
}

/// Find and timeout stuck delegations.
async fn cleanup_stuck_delegations(
    db: &DbClient,
    config: &SagaCleanupConfig,
    metrics: &SagaCleanupMetrics,
) -> u64 {
    let timeout_interval = format!("{} seconds", config.delegation_timeout.as_secs());

    // Find stuck delegations
    let stuck = match find_stuck_delegations(db, &timeout_interval, config.batch_size).await {
        Ok(stuck) => stuck,
        Err(e) => {
            tracing::error!(error = %e, "Failed to find stuck delegations");
            metrics.cleanup_errors.fetch_add(1, Ordering::Relaxed);
            return 0;
        }
    };

    let mut timed_out = 0u64;

    for delegation in stuck {
        if config.log_timeouts {
            tracing::warn!(
                delegation_id = %delegation.id,
                status = %delegation.status,
                stuck_duration = ?delegation.stuck_duration,
                "Timing out stuck delegation"
            );
        }

        match timeout_delegation(db, delegation.id, "Cleanup: exceeded timeout threshold").await {
            Ok(true) => {
                timed_out += 1;
                metrics
                    .delegations_timed_out
                    .fetch_add(1, Ordering::Relaxed);
            }
            Ok(false) => {
                // Delegation was already updated (race condition) - not an error
                tracing::debug!(
                    delegation_id = %delegation.id,
                    "Delegation already updated, skipping"
                );
            }
            Err(e) => {
                tracing::error!(
                    error = %e,
                    delegation_id = %delegation.id,
                    "Failed to timeout delegation"
                );
                metrics.cleanup_errors.fetch_add(1, Ordering::Relaxed);
            }
        }
    }

    timed_out
}

/// Find and timeout stuck handoffs.
async fn cleanup_stuck_handoffs(
    db: &DbClient,
    config: &SagaCleanupConfig,
    metrics: &SagaCleanupMetrics,
) -> u64 {
    let timeout_interval = format!("{} seconds", config.handoff_timeout.as_secs());

    // Find stuck handoffs
    let stuck = match find_stuck_handoffs(db, &timeout_interval, config.batch_size).await {
        Ok(stuck) => stuck,
        Err(e) => {
            tracing::error!(error = %e, "Failed to find stuck handoffs");
            metrics.cleanup_errors.fetch_add(1, Ordering::Relaxed);
            return 0;
        }
    };

    let mut timed_out = 0u64;

    for handoff in stuck {
        if config.log_timeouts {
            tracing::warn!(
                handoff_id = %handoff.id,
                status = %handoff.status,
                stuck_duration = ?handoff.stuck_duration,
                "Timing out stuck handoff"
            );
        }

        match timeout_handoff(db, handoff.id, "Cleanup: exceeded timeout threshold").await {
            Ok(true) => {
                timed_out += 1;
                metrics.handoffs_timed_out.fetch_add(1, Ordering::Relaxed);
            }
            Ok(false) => {
                tracing::debug!(
                    handoff_id = %handoff.id,
                    "Handoff already updated, skipping"
                );
            }
            Err(e) => {
                tracing::error!(
                    error = %e,
                    handoff_id = %handoff.id,
                    "Failed to timeout handoff"
                );
                metrics.cleanup_errors.fetch_add(1, Ordering::Relaxed);
            }
        }
    }

    timed_out
}

/// Clean up expired idempotency keys.
async fn cleanup_idempotency_keys(db: &DbClient, metrics: &SagaCleanupMetrics) {
    match delete_expired_idempotency_keys(db).await {
        Ok(count) => {
            if count > 0 {
                tracing::info!(count, "Cleaned up expired idempotency keys");
                metrics
                    .idempotency_keys_cleaned
                    .fetch_add(count as u64, Ordering::Relaxed);
            }
        }
        Err(e) => {
            tracing::error!(error = %e, "Failed to cleanup idempotency keys");
            metrics.cleanup_errors.fetch_add(1, Ordering::Relaxed);
        }
    }
}

// ============================================================================
// DATABASE OPERATIONS
// ============================================================================

/// Information about a stuck delegation.
#[derive(Debug)]
struct StuckDelegation {
    id: uuid::Uuid,
    status: String,
    stuck_duration: chrono::Duration,
}

/// Information about a stuck handoff.
#[derive(Debug)]
struct StuckHandoff {
    id: uuid::Uuid,
    status: String,
    stuck_duration: chrono::Duration,
}

fn seconds_to_duration(seconds: f64) -> chrono::Duration {
    let millis = (seconds * 1000.0) as i64;
    chrono::Duration::milliseconds(millis)
}

/// Find delegations that are stuck (timed out or stale).
async fn find_stuck_delegations(
    db: &DbClient,
    timeout_interval: &str,
    batch_size: usize,
) -> Result<Vec<StuckDelegation>, tokio_postgres::Error> {
    let client = db.get_conn().await.map_err(|e| {
        tracing::error!(error = %e, "Failed to get database connection");
        tokio_postgres::Error::__private_api_timeout()
    })?;

    let params: [&(dyn tokio_postgres::types::ToSql + Sync); 2] =
        [&timeout_interval, &(batch_size as i64)];
    let rows: Vec<tokio_postgres::Row> = client
        .query(
            "SELECT delegation_id, status, EXTRACT(EPOCH FROM stuck_duration) \
             FROM caliber_find_stuck_delegations($1::interval) \
             LIMIT $2",
            &params,
        )
        .await?;

    let stuck = rows
        .iter()
        .map(|row| {
            let id: uuid::Uuid = row.get(0);
            let status: String = row.get(1);
            let stuck_seconds: f64 = row.get(2);
            let stuck_duration = seconds_to_duration(stuck_seconds);

            StuckDelegation {
                id,
                status,
                stuck_duration,
            }
        })
        .collect();

    Ok(stuck)
}

/// Find handoffs that are stuck (timed out or stale).
async fn find_stuck_handoffs(
    db: &DbClient,
    timeout_interval: &str,
    batch_size: usize,
) -> Result<Vec<StuckHandoff>, tokio_postgres::Error> {
    let client = db.get_conn().await.map_err(|e| {
        tracing::error!(error = %e, "Failed to get database connection");
        tokio_postgres::Error::__private_api_timeout()
    })?;

    let params: [&(dyn tokio_postgres::types::ToSql + Sync); 2] =
        [&timeout_interval, &(batch_size as i64)];
    let rows: Vec<tokio_postgres::Row> = client
        .query(
            "SELECT handoff_id, status, EXTRACT(EPOCH FROM stuck_duration) \
             FROM caliber_find_stuck_handoffs($1::interval) \
             LIMIT $2",
            &params,
        )
        .await?;

    let stuck = rows
        .iter()
        .map(|row| {
            let id: uuid::Uuid = row.get(0);
            let status: String = row.get(1);
            let stuck_seconds: f64 = row.get(2);
            let stuck_duration = seconds_to_duration(stuck_seconds);

            StuckHandoff {
                id,
                status,
                stuck_duration,
            }
        })
        .collect();

    Ok(stuck)
}

/// Timeout a stuck delegation.
async fn timeout_delegation(
    db: &DbClient,
    delegation_id: uuid::Uuid,
    reason: &str,
) -> Result<bool, tokio_postgres::Error> {
    let client = db.get_conn().await.map_err(|e| {
        tracing::error!(error = %e, "Failed to get database connection");
        tokio_postgres::Error::__private_api_timeout()
    })?;

    let params: [&(dyn tokio_postgres::types::ToSql + Sync); 2] = [&delegation_id, &reason];
    let row: tokio_postgres::Row = client
        .query_one("SELECT caliber_timeout_delegation($1, $2)", &params)
        .await?;

    Ok(row.get(0))
}

/// Timeout a stuck handoff.
async fn timeout_handoff(
    db: &DbClient,
    handoff_id: uuid::Uuid,
    reason: &str,
) -> Result<bool, tokio_postgres::Error> {
    let client = db.get_conn().await.map_err(|e| {
        tracing::error!(error = %e, "Failed to get database connection");
        tokio_postgres::Error::__private_api_timeout()
    })?;

    let params: [&(dyn tokio_postgres::types::ToSql + Sync); 2] = [&handoff_id, &reason];
    let row: tokio_postgres::Row = client
        .query_one("SELECT caliber_timeout_handoff($1, $2)", &params)
        .await?;

    Ok(row.get(0))
}

/// Delete expired idempotency keys.
async fn delete_expired_idempotency_keys(db: &DbClient) -> Result<i32, tokio_postgres::Error> {
    let client = db.get_conn().await.map_err(|e| {
        tracing::error!(error = %e, "Failed to get database connection");
        tokio_postgres::Error::__private_api_timeout()
    })?;

    let row: tokio_postgres::Row = client
        .query_one("SELECT caliber_cleanup_idempotency_keys()", &[])
        .await?;

    Ok(row.get(0))
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::{
        DEFAULT_SAGA_BATCH_SIZE, DEFAULT_SAGA_CHECK_INTERVAL_SECS,
        DEFAULT_SAGA_DELEGATION_TIMEOUT_SECS, DEFAULT_SAGA_HANDOFF_TIMEOUT_SECS,
    };

    #[test]
    fn test_config_default() {
        let config = SagaCleanupConfig::default();
        assert_eq!(
            config.check_interval,
            Duration::from_secs(DEFAULT_SAGA_CHECK_INTERVAL_SECS)
        );
        assert_eq!(
            config.delegation_timeout,
            Duration::from_secs(DEFAULT_SAGA_DELEGATION_TIMEOUT_SECS)
        );
        assert_eq!(
            config.handoff_timeout,
            Duration::from_secs(DEFAULT_SAGA_HANDOFF_TIMEOUT_SECS)
        );
        assert_eq!(config.batch_size, DEFAULT_SAGA_BATCH_SIZE);
        assert!(config.log_timeouts);
    }

    #[test]
    fn test_config_development() {
        let config = SagaCleanupConfig::development();
        assert_eq!(config.check_interval, Duration::from_secs(10));
        assert_eq!(config.delegation_timeout, Duration::from_secs(60));
        assert_eq!(config.batch_size, 10);
    }

    #[test]
    fn test_config_production() {
        let config = SagaCleanupConfig::production();
        assert_eq!(
            config.check_interval,
            Duration::from_secs(DEFAULT_SAGA_CHECK_INTERVAL_SECS)
        );
        assert_eq!(config.delegation_timeout, Duration::from_secs(7200));
    }

    #[test]
    fn test_config_from_env_defaults() {
        // Without environment variables set, should use defaults
        let config = SagaCleanupConfig::from_env();
        assert_eq!(
            config.check_interval,
            Duration::from_secs(DEFAULT_SAGA_CHECK_INTERVAL_SECS)
        );
        assert_eq!(
            config.delegation_timeout,
            Duration::from_secs(DEFAULT_SAGA_DELEGATION_TIMEOUT_SECS)
        );
        assert_eq!(
            config.handoff_timeout,
            Duration::from_secs(DEFAULT_SAGA_HANDOFF_TIMEOUT_SECS)
        );
        assert_eq!(config.batch_size, DEFAULT_SAGA_BATCH_SIZE);
        assert!(config.log_timeouts);
    }

    #[test]
    fn test_metrics_new() {
        let metrics = SagaCleanupMetrics::new();
        assert_eq!(metrics.delegations_timed_out.load(Ordering::Relaxed), 0);
        assert_eq!(metrics.handoffs_timed_out.load(Ordering::Relaxed), 0);
        assert_eq!(metrics.cleanup_cycles.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn test_metrics_snapshot() {
        let metrics = SagaCleanupMetrics::new();
        metrics.delegations_timed_out.store(5, Ordering::Relaxed);
        metrics.handoffs_timed_out.store(3, Ordering::Relaxed);
        metrics.cleanup_cycles.store(10, Ordering::Relaxed);

        let snapshot = metrics.snapshot();
        assert_eq!(snapshot.delegations_timed_out, 5);
        assert_eq!(snapshot.handoffs_timed_out, 3);
        assert_eq!(snapshot.cleanup_cycles, 10);
    }
}
