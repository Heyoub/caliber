//! Background Jobs for CALIBER API
//!
//! This module contains background tasks that run periodically:
//!
//! - `saga_cleanup`: Cleans up stuck delegations and handoffs
//! - (future) `idempotency_cleanup`: Removes expired idempotency keys
//!
//! # Usage
//!
//! Background jobs are typically spawned during server startup:
//!
//! ```ignore
//! use caliber_api::jobs::{SagaCleanupConfig, saga_cleanup_task};
//! use tokio::sync::watch;
//!
//! // Create shutdown signal
//! let (shutdown_tx, shutdown_rx) = watch::channel(false);
//!
//! // Spawn cleanup task
//! let db = Arc::clone(&db_client);
//! let config = SagaCleanupConfig::default();
//! tokio::spawn(saga_cleanup_task(db, config, shutdown_rx));
//!
//! // On shutdown
//! let _ = shutdown_tx.send(true);
//! ```

pub mod saga_cleanup;

// Re-export commonly used types
pub use saga_cleanup::{saga_cleanup_task, SagaCleanupConfig, SagaCleanupMetrics};
