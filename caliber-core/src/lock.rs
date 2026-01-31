//! Lock typestate for compile-time safety of lock lifecycle.
//!
//! Uses the typestate pattern to make invalid state transitions uncompilable.
//! A lock can only be released or extended when it's in the Acquired state.
//!
//! # State Transition Diagram
//!
//! ```text
//! (unlocked) ─── acquire() ──→ Acquired ─── release() ──→ (unlocked)
//!                                  │
//!                             extend() ↺
//! ```

use crate::{AgentId, LockId, TenantId, Timestamp};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::marker::PhantomData;
use std::str::FromStr;
use std::time::Duration;
use uuid::Uuid;

// ============================================================================
// LOCK MODE ENUM (replaces String)
// ============================================================================

/// Lock mode determining concurrency behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub enum LockMode {
    /// Only one holder can acquire the lock
    Exclusive,
    /// Multiple holders can acquire the lock
    Shared,
}

impl LockMode {
    /// Convert to database string representation.
    pub fn as_db_str(&self) -> &'static str {
        match self {
            LockMode::Exclusive => "Exclusive",
            LockMode::Shared => "Shared",
        }
    }

    /// Parse from database string representation.
    pub fn from_db_str(s: &str) -> Result<Self, LockModeParseError> {
        match s.to_lowercase().as_str() {
            "exclusive" => Ok(LockMode::Exclusive),
            "shared" => Ok(LockMode::Shared),
            _ => Err(LockModeParseError(s.to_string())),
        }
    }
}

impl fmt::Display for LockMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_db_str())
    }
}

impl FromStr for LockMode {
    type Err = LockModeParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_db_str(s)
    }
}

/// Error when parsing an invalid lock mode string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LockModeParseError(pub String);

impl fmt::Display for LockModeParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Invalid lock mode: {}", self.0)
    }
}

impl std::error::Error for LockModeParseError {}

// ============================================================================
// LOCK DATA (internal storage, state-independent)
// ============================================================================

/// Internal data storage for a lock, independent of typestate.
/// This is what gets persisted to the database.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct LockData {
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub lock_id: LockId,
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub tenant_id: TenantId,
    pub resource_type: String,
    /// The ID of the resource being locked (generic UUID since it can be any entity type)
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub resource_id: Uuid,
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub holder_agent_id: AgentId,
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "date-time"))]
    pub acquired_at: Timestamp,
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "date-time"))]
    pub expires_at: Timestamp,
    pub mode: LockMode,
}

impl LockData {
    /// Check if the lock has expired based on current time.
    pub fn is_expired(&self, now: Timestamp) -> bool {
        now >= self.expires_at
    }

    /// Calculate remaining duration until expiry.
    pub fn remaining_duration(&self, now: Timestamp) -> Option<Duration> {
        if now >= self.expires_at {
            None
        } else {
            let duration = self.expires_at - now;
            duration.to_std().ok()
        }
    }
}

// ============================================================================
// TYPESTATE MARKERS
// ============================================================================

/// Marker trait for lock states.
pub trait LockState: private::Sealed + Send + Sync {}

/// Lock is currently held (acquired).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Acquired;
impl LockState for Acquired {}

/// Lock has been released (for documentation; locks in this state don't exist at runtime).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Released;
impl LockState for Released {}

mod private {
    pub trait Sealed {}
    impl Sealed for super::Acquired {}
    impl Sealed for super::Released {}
}

// ============================================================================
// LOCK TYPESTATE WRAPPER
// ============================================================================

/// A lock with compile-time state tracking.
///
/// The type parameter `S` indicates the current state of the lock.
/// Methods are only available in appropriate states:
/// - `Lock<Acquired>`: Can be extended or released
/// - `Lock<Released>`: Cannot be used (transitions consume the lock)
///
/// # Example
///
/// ```ignore
/// let lock: Lock<Acquired> = acquire_lock(...);
/// let extended_lock = lock.extend(Duration::from_secs(30)); // OK
/// let data = extended_lock.release(); // OK, consumes the lock
/// // Can't use extended_lock anymore - it was consumed!
/// ```
#[derive(Debug, Clone)]
pub struct Lock<S: LockState> {
    data: LockData,
    _state: PhantomData<S>,
}

impl<S: LockState> Lock<S> {
    /// Access the underlying lock data (read-only).
    pub fn data(&self) -> &LockData {
        &self.data
    }

    /// Get the lock ID.
    pub fn lock_id(&self) -> LockId {
        self.data.lock_id
    }

    /// Get the tenant ID.
    pub fn tenant_id(&self) -> TenantId {
        self.data.tenant_id
    }

    /// Get the resource type being locked.
    pub fn resource_type(&self) -> &str {
        &self.data.resource_type
    }

    /// Get the resource ID being locked.
    pub fn resource_id(&self) -> Uuid {
        self.data.resource_id
    }

    /// Get the agent holding the lock.
    pub fn holder_agent_id(&self) -> AgentId {
        self.data.holder_agent_id
    }

    /// Get the lock mode.
    pub fn mode(&self) -> LockMode {
        self.data.mode
    }

    /// Get when the lock was acquired.
    pub fn acquired_at(&self) -> Timestamp {
        self.data.acquired_at
    }

    /// Get when the lock expires.
    pub fn expires_at(&self) -> Timestamp {
        self.data.expires_at
    }
}

impl Lock<Acquired> {
    /// Create a new acquired lock from data.
    ///
    /// This should only be called when a lock is successfully acquired.
    pub fn new(data: LockData) -> Self {
        Lock {
            data,
            _state: PhantomData,
        }
    }

    /// Extend the lock duration.
    ///
    /// Returns a new `Lock<Acquired>` with the updated expiry time.
    /// The original lock is consumed.
    pub fn extend(mut self, additional: Duration) -> Self {
        let additional_chrono = chrono::Duration::from_std(additional)
            .unwrap_or_else(|_| chrono::Duration::milliseconds(additional.as_millis() as i64));
        self.data.expires_at += additional_chrono;
        self
    }

    /// Extend the lock by milliseconds.
    ///
    /// Convenience method for extending by a millisecond count.
    pub fn extend_ms(self, additional_ms: i64) -> Self {
        let additional = chrono::Duration::milliseconds(additional_ms);
        let mut lock = self;
        lock.data.expires_at += additional;
        lock
    }

    /// Release the lock and return the underlying data.
    ///
    /// Consumes the lock, preventing further operations.
    /// The returned data can be used to update the database.
    pub fn release(self) -> LockData {
        self.data
    }

    /// Check if the lock has expired.
    pub fn is_expired(&self, now: Timestamp) -> bool {
        self.data.is_expired(now)
    }

    /// Get remaining duration until expiry.
    pub fn remaining_duration(&self, now: Timestamp) -> Option<Duration> {
        self.data.remaining_duration(now)
    }

    /// Consume the lock and return just the data (for serialization).
    pub fn into_data(self) -> LockData {
        self.data
    }
}

// ============================================================================
// DATABASE BOUNDARY: STORED LOCK
// ============================================================================

/// A lock as stored in the database (status-agnostic).
///
/// When loading from the database, we don't know the state at compile time.
/// Use the `into_acquired` method to validate and convert to a typed lock.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StoredLock {
    pub data: LockData,
    /// Whether this lock is currently active (not expired/released)
    pub is_active: bool,
}

impl StoredLock {
    /// Convert to an acquired lock if the lock is active and not expired.
    ///
    /// # Errors
    ///
    /// Returns `Err` if the lock is not active or has expired.
    pub fn into_acquired(self, now: Timestamp) -> Result<Lock<Acquired>, LockStateError> {
        if !self.is_active {
            return Err(LockStateError::NotActive {
                lock_id: self.data.lock_id,
            });
        }
        if self.data.is_expired(now) {
            return Err(LockStateError::Expired {
                lock_id: self.data.lock_id,
                expired_at: self.data.expires_at,
            });
        }
        Ok(Lock::new(self.data))
    }

    /// Get the underlying data without state validation.
    pub fn data(&self) -> &LockData {
        &self.data
    }
}

/// Errors when transitioning lock states.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LockStateError {
    /// Lock is not in the active state.
    NotActive { lock_id: LockId },
    /// Lock has expired.
    Expired {
        lock_id: LockId,
        expired_at: Timestamp,
    },
}

impl fmt::Display for LockStateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LockStateError::NotActive { lock_id } => {
                write!(f, "Lock {} is not active", lock_id)
            }
            LockStateError::Expired {
                lock_id,
                expired_at,
            } => {
                write!(f, "Lock {} expired at {}", lock_id, expired_at)
            }
        }
    }
}

impl std::error::Error for LockStateError {}

// ============================================================================
// LOCK KEY COMPUTATION (for PostgreSQL advisory locks)
// ============================================================================

/// Compute a stable i64 key for advisory locks using FNV-1a hash.
///
/// FNV-1a is deterministic across Rust versions and compilations, making it
/// suitable for distributed lock coordination via PostgreSQL advisory locks.
///
/// # Arguments
///
/// * `resource_type` - The type of resource being locked (e.g., "trajectory", "scope")
/// * `resource_id` - The unique identifier of the resource
///
/// # Returns
///
/// A stable i64 hash that can be used with PostgreSQL's `pg_advisory_lock()`.
///
/// # Example
///
/// ```
/// use caliber_core::compute_lock_key;
/// use uuid::Uuid;
///
/// let resource_id = Uuid::now_v7();
/// let lock_key = compute_lock_key("trajectory", resource_id);
/// // Use lock_key with pg_advisory_lock(lock_key)
/// ```
pub fn compute_lock_key(resource_type: &str, resource_id: Uuid) -> i64 {
    const FNV_OFFSET_BASIS: u64 = 0xcbf29ce484222325;
    const FNV_PRIME: u64 = 0x100000001b3;

    let mut hash = FNV_OFFSET_BASIS;

    // Hash resource type
    for byte in resource_type.as_bytes() {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
    }

    // Hash resource ID bytes
    for byte in resource_id.as_bytes() {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
    }

    hash as i64
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::EntityIdType;
    use chrono::Utc;
    use uuid::Uuid;

    fn make_lock_data() -> LockData {
        let now = Utc::now();
        LockData {
            lock_id: LockId::now_v7(),
            tenant_id: TenantId::now_v7(),
            resource_type: "trajectory".to_string(),
            resource_id: Uuid::now_v7(),
            holder_agent_id: AgentId::now_v7(),
            acquired_at: now,
            expires_at: now + chrono::Duration::minutes(5),
            mode: LockMode::Exclusive,
        }
    }

    #[test]
    fn test_lock_mode_roundtrip() {
        for mode in [LockMode::Exclusive, LockMode::Shared] {
            let db_str = mode.as_db_str();
            let parsed = LockMode::from_db_str(db_str).expect("LockMode roundtrip should succeed");
            assert_eq!(mode, parsed);
        }
    }

    #[test]
    fn test_lock_extend() {
        let data = make_lock_data();
        let original_expires = data.expires_at;
        let lock = Lock::<Acquired>::new(data);

        let extended = lock.extend(Duration::from_secs(60));
        assert!(extended.expires_at() > original_expires);
    }

    #[test]
    fn test_lock_release_consumes() {
        let data = make_lock_data();
        let lock = Lock::<Acquired>::new(data.clone());

        let released_data = lock.release();
        assert_eq!(released_data.lock_id, data.lock_id);
        // lock is now consumed and cannot be used
    }

    #[test]
    fn test_stored_lock_conversion() {
        let now = Utc::now();
        let data = make_lock_data();

        let stored = StoredLock {
            data: data.clone(),
            is_active: true,
        };

        let acquired = stored
            .into_acquired(now)
            .expect("active lock should convert to acquired");
        assert_eq!(acquired.lock_id(), data.lock_id);
    }

    #[test]
    fn test_stored_lock_expired() {
        let now = Utc::now();
        let mut data = make_lock_data();
        data.expires_at = now - chrono::Duration::minutes(1); // Already expired

        let stored = StoredLock {
            data,
            is_active: true,
        };

        assert!(matches!(
            stored.into_acquired(now),
            Err(LockStateError::Expired { .. })
        ));
    }

    #[test]
    fn test_compute_lock_key_deterministic() {
        let resource_id = Uuid::now_v7();
        let resource_type = "trajectory";

        let key1 = compute_lock_key(resource_type, resource_id);
        let key2 = compute_lock_key(resource_type, resource_id);

        assert_eq!(key1, key2, "Lock key should be deterministic");
    }

    #[test]
    fn test_compute_lock_key_uniqueness() {
        let resource_id1 = Uuid::now_v7();
        let resource_id2 = Uuid::now_v7();
        let resource_type1 = "trajectory";
        let resource_type2 = "scope";

        // Same type, different IDs
        let key1 = compute_lock_key(resource_type1, resource_id1);
        let key2 = compute_lock_key(resource_type1, resource_id2);
        assert_ne!(
            key1, key2,
            "Different resource IDs should produce different keys"
        );

        // Different type, same ID
        let key3 = compute_lock_key(resource_type1, resource_id1);
        let key4 = compute_lock_key(resource_type2, resource_id1);
        assert_ne!(
            key3, key4,
            "Different resource types should produce different keys"
        );
    }
}
