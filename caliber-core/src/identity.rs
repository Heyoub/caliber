//! Identity types for CALIBER entities

use chrono::{DateTime, Utc};
use sha2::{Digest, Sha256};
use uuid::Uuid;

/// Entity identifier using UUIDv7 for timestamp-sortable IDs.
/// UUIDv7 embeds a Unix timestamp, making IDs naturally sortable by creation time.
pub type EntityId = Uuid;

/// Timestamp type using UTC timezone.
pub type Timestamp = DateTime<Utc>;

/// Duration in milliseconds for TTL and timeout values.
pub type DurationMs = i64;

/// SHA-256 content hash for deduplication and integrity verification.
pub type ContentHash = [u8; 32];

/// Raw binary content for BYTEA storage.
pub type RawContent = Vec<u8>;

/// Generate a new UUIDv7 EntityId (timestamp-sortable).
pub fn new_entity_id() -> EntityId {
    Uuid::now_v7()
}

/// Compute SHA-256 hash of content.
pub fn compute_content_hash(content: &[u8]) -> ContentHash {
    let mut hasher = Sha256::new();
    hasher.update(content);
    let result = hasher.finalize();
    let mut hash = [0u8; 32];
    hash.copy_from_slice(&result);
    hash
}
