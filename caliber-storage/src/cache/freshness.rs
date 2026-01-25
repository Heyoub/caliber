//! Freshness contracts for cache reads.
//!
//! This module defines the staleness tolerance that callers must specify
//! when reading from the cache, making cache semantics explicit.

use chrono::{DateTime, Utc};
use std::time::Duration;

use super::watermark::Watermark;

/// Freshness requirement for cache reads.
///
/// Callers must "sign the waiver" by specifying their staleness tolerance.
/// This makes cache semantics explicit rather than hiding them behind
/// a "best effort" abstraction.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Freshness {
    /// Accept potentially stale data up to the specified age.
    ///
    /// The cache will return cached data if available and not older than
    /// `max_staleness`. If the cached data is too old, it will be refreshed
    /// from storage.
    ///
    /// Use this when:
    /// - Performance matters more than perfect consistency
    /// - The use case can tolerate bounded staleness
    /// - You're displaying data that doesn't need to be real-time
    BestEffort {
        /// Maximum acceptable staleness for cached data.
        max_staleness: Duration,
    },

    /// Demand fresh data by checking the change journal watermark.
    ///
    /// The cache will verify against the change journal that no mutations
    /// have occurred since the cached data was fetched. If changes exist,
    /// the cache falls back to storage.
    ///
    /// Use this when:
    /// - Correctness is critical (e.g., locking, conflict detection)
    /// - You need read-after-write consistency
    /// - The data is used for decision-making that must be current
    #[default]
    Consistent,
}

impl Freshness {
    /// Create a BestEffort freshness with the given max staleness.
    pub fn best_effort(max_staleness: Duration) -> Self {
        Self::BestEffort { max_staleness }
    }

    /// Create a Consistent freshness requirement.
    pub fn consistent() -> Self {
        Self::Consistent
    }

    /// Returns true if this is a BestEffort freshness.
    pub fn is_best_effort(&self) -> bool {
        matches!(self, Self::BestEffort { .. })
    }

    /// Returns true if this is a Consistent freshness.
    pub fn is_consistent(&self) -> bool {
        matches!(self, Self::Consistent)
    }

    /// Get the max staleness for BestEffort, or zero for Consistent.
    pub fn max_staleness(&self) -> Duration {
        match self {
            Self::BestEffort { max_staleness } => *max_staleness,
            Self::Consistent => Duration::ZERO,
        }
    }
}

/// Result of a cache read, carrying staleness metadata.
///
/// This wrapper ensures callers are aware of the freshness of the data
/// they're working with. It provides methods to inspect staleness and
/// extract the underlying value.
#[derive(Debug, Clone)]
pub struct CacheRead<T> {
    /// The cached value.
    value: T,
    /// When this value was cached (or fetched from storage).
    cached_at: DateTime<Utc>,
    /// The watermark at the time of caching.
    watermark: Option<Watermark>,
    /// Whether this was a cache hit or miss.
    was_cache_hit: bool,
}

impl<T> CacheRead<T> {
    /// Create a new cache read from a cache hit.
    pub fn from_cache(value: T, cached_at: DateTime<Utc>, watermark: Option<Watermark>) -> Self {
        Self {
            value,
            cached_at,
            watermark,
            was_cache_hit: true,
        }
    }

    /// Create a new cache read from a storage fetch (cache miss).
    pub fn from_storage(value: T, watermark: Option<Watermark>) -> Self {
        Self {
            value,
            cached_at: Utc::now(),
            watermark,
            was_cache_hit: false,
        }
    }

    /// Consume the wrapper and return the underlying value.
    ///
    /// This is the primary way to extract the value after acknowledging
    /// the freshness characteristics.
    pub fn into_value(self) -> T {
        self.value
    }

    /// Get a reference to the underlying value.
    pub fn value(&self) -> &T {
        &self.value
    }

    /// Get a mutable reference to the underlying value.
    pub fn value_mut(&mut self) -> &mut T {
        &mut self.value
    }

    /// Check if the data is fresh as of the given timestamp.
    ///
    /// Returns true if the data was cached at or after the specified time.
    pub fn is_fresh_as_of(&self, timestamp: DateTime<Utc>) -> bool {
        self.cached_at >= timestamp
    }

    /// Calculate how stale the data is.
    ///
    /// Returns the duration since the data was cached. A lower value
    /// indicates fresher data.
    pub fn staleness(&self) -> Duration {
        let now = Utc::now();
        if now > self.cached_at {
            (now - self.cached_at)
                .to_std()
                .unwrap_or(Duration::ZERO)
        } else {
            Duration::ZERO
        }
    }

    /// Get when this value was cached.
    pub fn cached_at(&self) -> DateTime<Utc> {
        self.cached_at
    }

    /// Get the watermark at the time of caching, if available.
    pub fn watermark(&self) -> Option<&Watermark> {
        self.watermark.as_ref()
    }

    /// Check if this was a cache hit.
    pub fn was_cache_hit(&self) -> bool {
        self.was_cache_hit
    }

    /// Check if this was a cache miss (fetched from storage).
    pub fn was_cache_miss(&self) -> bool {
        !self.was_cache_hit
    }

    /// Map the inner value to a new type.
    pub fn map<U, F>(self, f: F) -> CacheRead<U>
    where
        F: FnOnce(T) -> U,
    {
        CacheRead {
            value: f(self.value),
            cached_at: self.cached_at,
            watermark: self.watermark,
            was_cache_hit: self.was_cache_hit,
        }
    }
}

impl<T> AsRef<T> for CacheRead<T> {
    fn as_ref(&self) -> &T {
        &self.value
    }
}

impl<T> AsMut<T> for CacheRead<T> {
    fn as_mut(&mut self) -> &mut T {
        &mut self.value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_freshness_best_effort() {
        let freshness = Freshness::best_effort(Duration::from_secs(60));
        assert!(freshness.is_best_effort());
        assert!(!freshness.is_consistent());
        assert_eq!(freshness.max_staleness(), Duration::from_secs(60));
    }

    #[test]
    fn test_freshness_consistent() {
        let freshness = Freshness::consistent();
        assert!(freshness.is_consistent());
        assert!(!freshness.is_best_effort());
        assert_eq!(freshness.max_staleness(), Duration::ZERO);
    }

    #[test]
    fn test_freshness_default_is_consistent() {
        let freshness = Freshness::default();
        assert!(freshness.is_consistent());
    }

    #[test]
    fn test_cache_read_from_cache() {
        let value = "test_value".to_string();
        let cached_at = Utc::now();
        let read = CacheRead::from_cache(value.clone(), cached_at, None);

        assert!(read.was_cache_hit());
        assert!(!read.was_cache_miss());
        assert_eq!(read.value(), &value);
        assert_eq!(read.cached_at(), cached_at);
    }

    #[test]
    fn test_cache_read_from_storage() {
        let value = 42i32;
        let read = CacheRead::from_storage(value, None);

        assert!(!read.was_cache_hit());
        assert!(read.was_cache_miss());
        assert_eq!(read.into_value(), 42);
    }

    #[test]
    fn test_cache_read_staleness() {
        let past = Utc::now() - chrono::Duration::seconds(5);
        let read = CacheRead::from_cache("test", past, None);

        let staleness = read.staleness();
        assert!(staleness >= Duration::from_secs(4));
        assert!(staleness <= Duration::from_secs(10));
    }

    #[test]
    fn test_cache_read_is_fresh_as_of() {
        let cached_at = Utc::now();
        let read = CacheRead::from_cache("test", cached_at, None);

        let past = cached_at - chrono::Duration::seconds(10);
        let future = cached_at + chrono::Duration::seconds(10);

        assert!(read.is_fresh_as_of(past));
        assert!(read.is_fresh_as_of(cached_at));
        assert!(!read.is_fresh_as_of(future));
    }

    #[test]
    fn test_cache_read_map() {
        let read = CacheRead::from_storage(42i32, None);
        let mapped = read.map(|v| v.to_string());

        assert_eq!(mapped.into_value(), "42");
    }
}
