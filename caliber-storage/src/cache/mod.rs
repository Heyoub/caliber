//! Cache layer with correctness contracts and multi-tenant LMDB isolation.
//!
//! This module provides a read-through cache with explicit freshness guarantees
//! and strict tenant isolation using LMDB.
//!
//! # Design Philosophy
//!
//! Traditional caches hide their staleness, leading to subtle bugs. This module
//! makes staleness explicit: callers must specify their tolerance via [`Freshness`],
//! and reads return [`CacheRead<T>`] which carries staleness metadata.
//!
//! # Tenant Isolation
//!
//! The [`TenantScopedKey`] type ensures that cache keys CANNOT be constructed
//! without providing a `tenant_id`. This makes cross-tenant cache access
//! impossible at compile time - not just a runtime check, but structurally
//! enforced by the type system.
//!
//! # Example
//!
//! ```ignore
//! // Caller explicitly opts into potentially stale data
//! let read = cache.get::<Artifact>(id, tenant_id, Freshness::BestEffort {
//!     max_staleness: Duration::from_secs(60),
//! }).await?;
//!
//! // Or demands fresh data
//! let read = cache.get::<Artifact>(id, tenant_id, Freshness::Consistent).await?;
//!
//! // Caller can inspect staleness
//! if read.staleness() > Duration::from_secs(30) {
//!     log::warn!("Data is getting stale");
//! }
//! ```

pub mod freshness;
pub mod lmdb_backend;
pub mod read_through;
pub mod tenant_key;
pub mod traits;
pub mod watermark;

pub use freshness::{CacheRead, Freshness};
pub use lmdb_backend::{LmdbCacheBackend, LmdbCacheError};
pub use read_through::{CacheConfig, ReadThroughCache, StorageFetcher};
pub use tenant_key::TenantScopedKey;
pub use traits::{CacheBackend, CacheStats, CacheableEntity};
pub use watermark::{ChangeJournal, EventDagChangeJournal, InMemoryChangeJournal, Watermark};
