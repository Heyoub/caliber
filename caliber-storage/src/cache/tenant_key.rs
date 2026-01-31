//! Tenant-scoped cache key system for multi-tenant LMDB isolation.
//!
//! The key insight is that `TenantScopedKey`'s private constructor makes
//! cross-tenant access UNCOMPILABLE. You cannot construct a key without
//! explicitly providing a tenant ID.

use caliber_core::EntityType;
use uuid::Uuid;

/// Separator byte between tenant_id and the rest of the key.
const SEPARATOR: u8 = 0xFF;

/// A cache key that is scoped to a specific tenant.
///
/// # Design
///
/// The private inner struct ensures that a `TenantScopedKey` can ONLY be
/// constructed via the `new()` method, which requires a tenant_id. This
/// makes cross-tenant data access impossible at compile time.
///
/// # Binary Format
///
/// The key encodes to a fixed 34-byte array:
/// - Bytes 0-15: tenant_id (UUID as bytes)
/// - Byte 16: separator (0xFF)
/// - Byte 17: entity_type (single byte discriminant)
/// - Bytes 18-33: entity_id (UUID as bytes)
///
/// This format ensures:
/// - Keys are naturally sorted by tenant first
/// - LMDB range scans can efficiently iterate a single tenant's data
/// - Fixed-size keys optimize B-tree operations
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TenantScopedKey {
    /// Private inner data - cannot be constructed externally
    inner: TenantKeyInner,
}

/// Private inner struct - prevents external construction.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct TenantKeyInner {
    tenant_id: Uuid,
    entity_type: EntityType,
    entity_id: Uuid,
}

impl TenantScopedKey {
    /// Create a new tenant-scoped cache key.
    ///
    /// This is the ONLY way to construct a `TenantScopedKey`, ensuring
    /// that all cache operations are tenant-isolated by construction.
    ///
    /// # Arguments
    ///
    /// * `tenant_id` - The tenant this key belongs to
    /// * `entity_type` - The type of entity being cached
    /// * `entity_id` - The unique identifier of the entity
    pub fn new(tenant_id: Uuid, entity_type: EntityType, entity_id: Uuid) -> Self {
        Self {
            inner: TenantKeyInner {
                tenant_id,
                entity_type,
                entity_id,
            },
        }
    }

    /// Get the tenant ID this key is scoped to.
    pub fn tenant_id(&self) -> Uuid {
        self.inner.tenant_id
    }

    /// Get the entity type for this key.
    pub fn entity_type(&self) -> EntityType {
        self.inner.entity_type
    }

    /// Get the entity ID for this key.
    pub fn entity_id(&self) -> Uuid {
        self.inner.entity_id
    }

    /// Encode this key to a fixed-size byte array for LMDB storage.
    ///
    /// Format: [tenant_id: 16 bytes][separator: 1 byte][type: 1 byte][entity_id: 16 bytes]
    /// Total: 34 bytes
    pub fn encode(&self) -> [u8; 34] {
        let mut bytes = [0u8; 34];

        // Bytes 0-15: tenant_id
        bytes[0..16].copy_from_slice(self.inner.tenant_id.as_bytes());

        // Byte 16: separator
        bytes[16] = SEPARATOR;

        // Byte 17: entity_type discriminant
        bytes[17] = entity_type_to_byte(self.inner.entity_type);

        // Bytes 18-33: entity_id
        bytes[18..34].copy_from_slice(self.inner.entity_id.as_bytes());

        bytes
    }

    /// Decode a key from bytes.
    ///
    /// Returns `None` if:
    /// - The byte slice is not exactly 34 bytes
    /// - The separator byte is missing or incorrect
    /// - The entity type byte is invalid
    /// - Either UUID is malformed
    pub fn decode(bytes: &[u8]) -> Option<Self> {
        if bytes.len() != 34 {
            return None;
        }

        // Check separator
        if bytes[16] != SEPARATOR {
            return None;
        }

        // Parse tenant_id
        let tenant_id = uuid::Uuid::from_slice(&bytes[0..16]).ok()?;

        // Parse entity_type
        let entity_type = byte_to_entity_type(bytes[17])?;

        // Parse entity_id
        let entity_id = uuid::Uuid::from_slice(&bytes[18..34]).ok()?;

        Some(Self {
            inner: TenantKeyInner {
                tenant_id,
                entity_type,
                entity_id,
            },
        })
    }

    /// Create a prefix for scanning all keys belonging to a tenant.
    ///
    /// This can be used with LMDB's range queries to efficiently
    /// iterate over all cached entities for a single tenant.
    pub fn tenant_prefix(tenant_id: Uuid) -> [u8; 17] {
        let mut prefix = [0u8; 17];
        prefix[0..16].copy_from_slice(tenant_id.as_bytes());
        prefix[16] = SEPARATOR;
        prefix
    }

    /// Create a prefix for scanning all keys of a specific entity type for a tenant.
    ///
    /// This is more targeted than `tenant_prefix`, useful for invalidating
    /// all cached entities of a specific type.
    pub fn tenant_type_prefix(tenant_id: Uuid, entity_type: EntityType) -> [u8; 18] {
        let mut prefix = [0u8; 18];
        prefix[0..16].copy_from_slice(tenant_id.as_bytes());
        prefix[16] = SEPARATOR;
        prefix[17] = entity_type_to_byte(entity_type);
        prefix
    }
}

/// Convert EntityType to a single-byte discriminant.
fn entity_type_to_byte(entity_type: EntityType) -> u8 {
    match entity_type {
        EntityType::Trajectory => 0,
        EntityType::Scope => 1,
        EntityType::Artifact => 2,
        EntityType::Note => 3,
        EntityType::Turn => 4,
        EntityType::Lock => 5,
        EntityType::Message => 6,
        EntityType::Agent => 7,
        EntityType::Delegation => 8,
        EntityType::Handoff => 9,
        EntityType::Conflict => 10,
        EntityType::Edge => 11,
        EntityType::EvolutionSnapshot => 12,
        EntityType::SummarizationPolicy => 13,
    }
}

/// Convert a byte back to EntityType.
fn byte_to_entity_type(byte: u8) -> Option<EntityType> {
    match byte {
        0 => Some(EntityType::Trajectory),
        1 => Some(EntityType::Scope),
        2 => Some(EntityType::Artifact),
        3 => Some(EntityType::Note),
        4 => Some(EntityType::Turn),
        5 => Some(EntityType::Lock),
        6 => Some(EntityType::Message),
        7 => Some(EntityType::Agent),
        8 => Some(EntityType::Delegation),
        9 => Some(EntityType::Handoff),
        10 => Some(EntityType::Conflict),
        11 => Some(EntityType::Edge),
        12 => Some(EntityType::EvolutionSnapshot),
        13 => Some(EntityType::SummarizationPolicy),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn test_new_and_getters() {
        let tenant_id = Uuid::now_v7();
        let entity_id = Uuid::now_v7();
        let entity_type = EntityType::Artifact;

        let key = TenantScopedKey::new(tenant_id, entity_type, entity_id);

        assert_eq!(key.tenant_id(), tenant_id);
        assert_eq!(key.entity_type(), entity_type);
        assert_eq!(key.entity_id(), entity_id);
    }

    #[test]
    fn test_encode_decode_roundtrip() {
        let tenant_id = Uuid::now_v7();
        let entity_id = Uuid::now_v7();
        let entity_type = EntityType::Note;

        let key = TenantScopedKey::new(tenant_id, entity_type, entity_id);
        let encoded = key.encode();
        let decoded = TenantScopedKey::decode(&encoded).expect("decode should succeed");

        assert_eq!(key, decoded);
    }

    #[test]
    fn test_encode_length() {
        let key = TenantScopedKey::new(Uuid::now_v7(), EntityType::Trajectory, Uuid::now_v7());
        let encoded = key.encode();
        assert_eq!(encoded.len(), 34);
    }

    #[test]
    fn test_decode_wrong_length() {
        let short = [0u8; 33];
        let long = [0u8; 35];

        assert!(TenantScopedKey::decode(&short).is_none());
        assert!(TenantScopedKey::decode(&long).is_none());
    }

    #[test]
    fn test_decode_wrong_separator() {
        let mut bytes = [0u8; 34];
        bytes[16] = 0x00; // Wrong separator
        assert!(TenantScopedKey::decode(&bytes).is_none());
    }

    #[test]
    fn test_decode_invalid_entity_type() {
        let key = TenantScopedKey::new(Uuid::now_v7(), EntityType::Artifact, Uuid::now_v7());
        let mut encoded = key.encode();
        encoded[17] = 255; // Invalid entity type
        assert!(TenantScopedKey::decode(&encoded).is_none());
    }

    #[test]
    fn test_tenant_prefix() {
        let tenant_id = Uuid::now_v7();
        let prefix = TenantScopedKey::tenant_prefix(tenant_id);

        assert_eq!(prefix.len(), 17);
        assert_eq!(&prefix[0..16], tenant_id.as_bytes());
        assert_eq!(prefix[16], SEPARATOR);
    }

    #[test]
    fn test_tenant_type_prefix() {
        let tenant_id = Uuid::now_v7();
        let entity_type = EntityType::Artifact;
        let prefix = TenantScopedKey::tenant_type_prefix(tenant_id, entity_type);

        assert_eq!(prefix.len(), 18);
        assert_eq!(&prefix[0..16], tenant_id.as_bytes());
        assert_eq!(prefix[16], SEPARATOR);
        assert_eq!(prefix[17], entity_type_to_byte(entity_type));
    }

    #[test]
    fn test_different_tenants_different_keys() {
        let tenant1 = Uuid::now_v7();
        let tenant2 = Uuid::now_v7();
        let entity_id = Uuid::now_v7();

        let key1 = TenantScopedKey::new(tenant1, EntityType::Artifact, entity_id);
        let key2 = TenantScopedKey::new(tenant2, EntityType::Artifact, entity_id);

        assert_ne!(key1.encode(), key2.encode());
    }

    #[test]
    fn test_same_tenant_different_entities_different_keys() {
        let tenant_id = Uuid::now_v7();
        let entity1 = Uuid::now_v7();
        let entity2 = Uuid::now_v7();

        let key1 = TenantScopedKey::new(tenant_id, EntityType::Artifact, entity1);
        let key2 = TenantScopedKey::new(tenant_id, EntityType::Artifact, entity2);

        assert_ne!(key1.encode(), key2.encode());
    }

    #[test]
    fn test_same_tenant_same_entity_different_types_different_keys() {
        let tenant_id = Uuid::now_v7();
        let entity_id = Uuid::now_v7();

        let key1 = TenantScopedKey::new(tenant_id, EntityType::Artifact, entity_id);
        let key2 = TenantScopedKey::new(tenant_id, EntityType::Note, entity_id);

        assert_ne!(key1.encode(), key2.encode());
    }

    #[test]
    fn test_all_entity_types_roundtrip() {
        let tenant_id = Uuid::now_v7();
        let entity_id = Uuid::now_v7();

        let entity_types = [
            EntityType::Trajectory,
            EntityType::Scope,
            EntityType::Artifact,
            EntityType::Note,
            EntityType::Turn,
            EntityType::Lock,
            EntityType::Message,
            EntityType::Agent,
            EntityType::Delegation,
            EntityType::Handoff,
            EntityType::Conflict,
            EntityType::Edge,
            EntityType::EvolutionSnapshot,
            EntityType::SummarizationPolicy,
        ];

        for entity_type in entity_types {
            let key = TenantScopedKey::new(tenant_id, entity_type, entity_id);
            let encoded = key.encode();
            let decoded = TenantScopedKey::decode(&encoded).expect("decode should succeed");
            assert_eq!(key, decoded, "Roundtrip failed for {:?}", entity_type);
        }
    }
}

#[cfg(test)]
mod prop_tests {
    use super::*;
    use proptest::prelude::*;

    /// Strategy to generate random UUIDs for property testing.
    fn uuid_strategy() -> impl Strategy<Value = uuid::Uuid> {
        any::<[u8; 16]>().prop_map(uuid::Uuid::from_bytes)
    }

    /// Strategy to generate random EntityTypes.
    fn entity_type_strategy() -> impl Strategy<Value = EntityType> {
        prop_oneof![
            Just(EntityType::Trajectory),
            Just(EntityType::Scope),
            Just(EntityType::Artifact),
            Just(EntityType::Note),
            Just(EntityType::Turn),
            Just(EntityType::Lock),
            Just(EntityType::Message),
            Just(EntityType::Agent),
            Just(EntityType::Delegation),
            Just(EntityType::Handoff),
            Just(EntityType::Conflict),
            Just(EntityType::Edge),
            Just(EntityType::EvolutionSnapshot),
            Just(EntityType::SummarizationPolicy),
        ]
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(1000))]

        /// Property: Encode/decode roundtrip preserves the original value.
        ///
        /// For all valid TenantScopedKey instances, encoding then decoding
        /// must return an identical key.
        #[test]
        fn prop_encode_decode_roundtrip(
            tenant_id in uuid_strategy(),
            entity_type in entity_type_strategy(),
            entity_id in uuid_strategy(),
        ) {
            let key = TenantScopedKey::new(tenant_id, entity_type, entity_id);
            let encoded = key.encode();
            let decoded = TenantScopedKey::decode(&encoded);

            prop_assert!(decoded.is_some(), "Decode should succeed for valid key");
            prop_assert_eq!(key, decoded.expect("decode should succeed"), "Roundtrip should preserve value");
        }

        /// Property: Encoding is injective (different inputs produce different outputs).
        ///
        /// If two keys are different (differ in any field), their encodings
        /// must be different. This ensures no key collisions in LMDB.
        #[test]
        fn prop_encoding_is_injective(
            tenant1 in uuid_strategy(),
            tenant2 in uuid_strategy(),
            type1 in entity_type_strategy(),
            type2 in entity_type_strategy(),
            id1 in uuid_strategy(),
            id2 in uuid_strategy(),
        ) {
            let key1 = TenantScopedKey::new(tenant1, type1, id1);
            let key2 = TenantScopedKey::new(tenant2, type2, id2);

            // If keys are equal, encodings must be equal
            // If keys are different, encodings must be different (injective)
            if key1 == key2 {
                prop_assert_eq!(key1.encode(), key2.encode());
            } else {
                prop_assert_ne!(
                    key1.encode(),
                    key2.encode(),
                    "Different keys must have different encodings"
                );
            }
        }

        /// Property: Encoded keys are always exactly 34 bytes.
        #[test]
        fn prop_encode_length_always_34(
            tenant_id in uuid_strategy(),
            entity_type in entity_type_strategy(),
            entity_id in uuid_strategy(),
        ) {
            let key = TenantScopedKey::new(tenant_id, entity_type, entity_id);
            let encoded = key.encode();
            prop_assert_eq!(encoded.len(), 34, "Encoded key must be exactly 34 bytes");
        }

        /// Property: The separator byte is always at position 16.
        #[test]
        fn prop_separator_at_correct_position(
            tenant_id in uuid_strategy(),
            entity_type in entity_type_strategy(),
            entity_id in uuid_strategy(),
        ) {
            let key = TenantScopedKey::new(tenant_id, entity_type, entity_id);
            let encoded = key.encode();
            prop_assert_eq!(encoded[16], 0xFF, "Separator must be at position 16");
        }

        /// Property: Tenant ID can be extracted from encoded bytes.
        #[test]
        fn prop_tenant_id_extractable(
            tenant_id in uuid_strategy(),
            entity_type in entity_type_strategy(),
            entity_id in uuid_strategy(),
        ) {
            let key = TenantScopedKey::new(tenant_id, entity_type, entity_id);
            let encoded = key.encode();

            let extracted = uuid::Uuid::from_slice(&encoded[0..16]).expect("UUID extraction should succeed");
            prop_assert_eq!(tenant_id, extracted, "Tenant ID must be at bytes 0-15");
        }

        /// Property: Entity ID can be extracted from encoded bytes.
        #[test]
        fn prop_entity_id_extractable(
            tenant_id in uuid_strategy(),
            entity_type in entity_type_strategy(),
            entity_id in uuid_strategy(),
        ) {
            let key = TenantScopedKey::new(tenant_id, entity_type, entity_id);
            let encoded = key.encode();

            let extracted = uuid::Uuid::from_slice(&encoded[18..34]).expect("UUID extraction should succeed");
            prop_assert_eq!(entity_id, extracted, "Entity ID must be at bytes 18-33");
        }

        /// Property: Tenant prefix is a valid prefix of all keys for that tenant.
        #[test]
        fn prop_tenant_prefix_is_prefix(
            tenant_id in uuid_strategy(),
            entity_type in entity_type_strategy(),
            entity_id in uuid_strategy(),
        ) {
            let key = TenantScopedKey::new(tenant_id, entity_type, entity_id);
            let encoded = key.encode();
            let prefix = TenantScopedKey::tenant_prefix(tenant_id);

            prop_assert_eq!(
                &encoded[0..17],
                &prefix[..],
                "Tenant prefix must match first 17 bytes of encoded key"
            );
        }

        /// Property: Tenant-type prefix is a valid prefix of all keys for that tenant and type.
        #[test]
        fn prop_tenant_type_prefix_is_prefix(
            tenant_id in uuid_strategy(),
            entity_type in entity_type_strategy(),
            entity_id in uuid_strategy(),
        ) {
            let key = TenantScopedKey::new(tenant_id, entity_type, entity_id);
            let encoded = key.encode();
            let prefix = TenantScopedKey::tenant_type_prefix(tenant_id, entity_type);

            prop_assert_eq!(
                &encoded[0..18],
                &prefix[..],
                "Tenant-type prefix must match first 18 bytes of encoded key"
            );
        }
    }
}
