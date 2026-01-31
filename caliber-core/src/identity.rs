//! Identity types for CALIBER entities

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fmt;
use std::hash::Hash;
use std::str::FromStr;
use uuid::Uuid;

// ============================================================================
// ENTITY ID TYPE SYSTEM
// ============================================================================

/// Trait for type-safe entity IDs.
///
/// This trait provides compile-time safety by ensuring entity IDs cannot be
/// accidentally mixed up. Each entity type has its own strongly-typed ID.
pub trait EntityIdType:
    Copy
    + Clone
    + Eq
    + PartialEq
    + Hash
    + fmt::Debug
    + fmt::Display
    + FromStr
    + Serialize
    + serde::de::DeserializeOwned
    + Send
    + Sync
    + 'static
{
    /// The name of the entity type (e.g., "tenant", "trajectory").
    const ENTITY_NAME: &'static str;

    /// Create a new ID from a UUID.
    fn new(uuid: Uuid) -> Self;

    /// Get the underlying UUID.
    fn as_uuid(&self) -> Uuid;

    /// Create a nil (all zeros) ID.
    fn nil() -> Self {
        Self::new(Uuid::nil())
    }

    /// Create a new timestamp-sortable UUIDv7 ID.
    fn now_v7() -> Self {
        Self::new(Uuid::now_v7())
    }

    /// Create a new random UUIDv4 ID.
    fn new_v4() -> Self {
        Self::new(Uuid::new_v4())
    }
}

/// Error type for parsing entity IDs from strings.
#[derive(Debug, Clone)]
pub struct EntityIdParseError {
    pub entity_name: &'static str,
    pub input: String,
    pub source: uuid::Error,
}

impl fmt::Display for EntityIdParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Failed to parse {} ID from '{}': {}",
            self.entity_name, self.input, self.source
        )
    }
}

impl std::error::Error for EntityIdParseError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.source)
    }
}

/// Macro to define a type-safe entity ID newtype.
///
/// This generates a newtype wrapper around UUID with all the necessary trait
/// implementations for compile-time type safety.
macro_rules! define_entity_id {
    ($name:ident, $entity:literal, $doc:literal) => {
        #[doc = $doc]
        #[derive(Clone, Copy, PartialEq, Eq, Hash)]
        #[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
        pub struct $name(Uuid);

        impl EntityIdType for $name {
            const ENTITY_NAME: &'static str = $entity;

            fn new(uuid: Uuid) -> Self {
                Self(uuid)
            }

            fn as_uuid(&self) -> Uuid {
                self.0
            }
        }

        impl fmt::Debug for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}({})", stringify!($name), self.0)
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}", self.0)
            }
        }

        impl FromStr for $name {
            type Err = EntityIdParseError;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                Uuid::from_str(s)
                    .map(Self::new)
                    .map_err(|e| EntityIdParseError {
                        entity_name: Self::ENTITY_NAME,
                        input: s.to_string(),
                        source: e,
                    })
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::nil()
            }
        }

        impl Serialize for $name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: serde::Serializer,
            {
                // Serialize transparently as UUID string
                self.0.serialize(serializer)
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                // Deserialize transparently from UUID
                Uuid::deserialize(deserializer).map(Self::new)
            }
        }
    };
}

// ============================================================================
// ENTITY ID TYPES
// ============================================================================

define_entity_id!(TenantId, "tenant", "Type-safe ID for tenant entities.");
define_entity_id!(
    TrajectoryId,
    "trajectory",
    "Type-safe ID for trajectory entities."
);
define_entity_id!(ScopeId, "scope", "Type-safe ID for scope entities.");
define_entity_id!(
    ArtifactId,
    "artifact",
    "Type-safe ID for artifact entities."
);
define_entity_id!(NoteId, "note", "Type-safe ID for note entities.");
define_entity_id!(TurnId, "turn", "Type-safe ID for turn entities.");
define_entity_id!(AgentId, "agent", "Type-safe ID for agent entities.");
define_entity_id!(EdgeId, "edge", "Type-safe ID for edge entities.");
define_entity_id!(LockId, "lock", "Type-safe ID for lock entities.");
define_entity_id!(MessageId, "message", "Type-safe ID for message entities.");
define_entity_id!(
    DelegationId,
    "delegation",
    "Type-safe ID for delegation entities."
);
define_entity_id!(HandoffId, "handoff", "Type-safe ID for handoff entities.");
define_entity_id!(ApiKeyId, "api_key", "Type-safe ID for API key entities.");
define_entity_id!(WebhookId, "webhook", "Type-safe ID for webhook entities.");
define_entity_id!(
    SummarizationPolicyId,
    "summarization_policy",
    "Type-safe ID for summarization policy entities."
);
define_entity_id!(
    ConflictId,
    "conflict",
    "Type-safe ID for conflict entities."
);
define_entity_id!(
    DslConfigId,
    "dsl_config",
    "Type-safe ID for DSL configuration entities."
);

// BDI (Belief-Desire-Intention) Agent Primitives (Phase 2)
define_entity_id!(GoalId, "goal", "Type-safe ID for agent goal entities.");
define_entity_id!(PlanId, "plan", "Type-safe ID for agent plan entities.");
define_entity_id!(
    ActionId,
    "action",
    "Type-safe ID for agent action entities."
);
define_entity_id!(StepId, "step", "Type-safe ID for plan step entities.");
define_entity_id!(
    ObservationId,
    "observation",
    "Type-safe ID for agent observation entities."
);
define_entity_id!(
    BeliefId,
    "belief",
    "Type-safe ID for agent belief entities."
);
define_entity_id!(
    LearningId,
    "learning",
    "Type-safe ID for agent learning entities."
);

// ============================================================================
// OTHER IDENTITY TYPES
// ============================================================================

/// Timestamp type using UTC timezone.
pub type Timestamp = DateTime<Utc>;

/// Duration in milliseconds for TTL and timeout values.
pub type DurationMs = i64;

/// SHA-256 content hash for deduplication and integrity verification.
pub type ContentHash = [u8; 32];

/// Raw binary content for BYTEA storage.
pub type RawContent = Vec<u8>;

// ============================================================================
// UTILITY FUNCTIONS
// ============================================================================

/// Compute SHA-256 hash of content.
pub fn compute_content_hash(content: &[u8]) -> ContentHash {
    let mut hasher = Sha256::new();
    hasher.update(content);
    let result = hasher.finalize();
    let mut hash = [0u8; 32];
    hash.copy_from_slice(&result);
    hash
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entity_id_type_safety() {
        // Different ID types cannot be mixed
        let tenant_id = TenantId::now_v7();
        let trajectory_id = TrajectoryId::now_v7();

        // This would not compile if uncommented:
        // let _: TenantId = trajectory_id;

        assert_ne!(tenant_id.as_uuid(), trajectory_id.as_uuid());
    }

    #[test]
    fn test_entity_id_display() {
        let id = TenantId::new(Uuid::nil());
        assert_eq!(
            format!("{:?}", id),
            "TenantId(00000000-0000-0000-0000-000000000000)"
        );
        assert_eq!(format!("{}", id), "00000000-0000-0000-0000-000000000000");
    }

    #[test]
    fn test_entity_id_from_str() {
        let uuid_str = "550e8400-e29b-41d4-a716-446655440000";
        let id: TenantId = uuid_str.parse().expect("valid UUID should parse");
        assert_eq!(id.to_string(), uuid_str);
    }

    #[test]
    fn test_entity_id_parse_error() {
        let result: Result<TenantId, _> = "invalid".parse();
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.entity_name, "tenant");
        assert_eq!(err.input, "invalid");
    }

    #[test]
    fn test_entity_id_serde() {
        let id = TenantId::now_v7();
        let json = serde_json::to_string(&id).expect("serialization should succeed");
        // Should serialize as UUID string (not wrapped in object)
        assert!(json.starts_with('"'));
        assert!(json.ends_with('"'));

        let deserialized: TenantId =
            serde_json::from_str(&json).expect("deserialization should succeed");
        assert_eq!(id, deserialized);
    }

    #[test]
    fn test_entity_id_default() {
        let id = TenantId::default();
        assert_eq!(id, TenantId::nil());
    }

    #[test]
    fn test_all_entity_types() {
        // Ensure all 15 entity types are defined
        let _tenant = TenantId::now_v7();
        let _trajectory = TrajectoryId::now_v7();
        let _scope = ScopeId::now_v7();
        let _artifact = ArtifactId::now_v7();
        let _note = NoteId::now_v7();
        let _turn = TurnId::now_v7();
        let _agent = AgentId::now_v7();
        let _edge = EdgeId::now_v7();
        let _lock = LockId::now_v7();
        let _message = MessageId::now_v7();
        let _delegation = DelegationId::now_v7();
        let _handoff = HandoffId::now_v7();
        let _api_key = ApiKeyId::now_v7();
        let _webhook = WebhookId::now_v7();
        let _summarization_policy = SummarizationPolicyId::now_v7();
    }
}
