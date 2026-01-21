//! Direct heap operations for Edge entities (Battle Intel Feature 1).
//!
//! This module provides hot-path operations for graph edges that bypass SQL
//! parsing entirely by using direct heap access via pgrx.
//!
//! Edges support both binary relationships (2 participants) and hyperedges
//! (N-ary relationships). Inspired by Mem0's graph-based memory.
//!
//! # Operations
//!
//! - `edge_create_heap` - Insert a new edge
//! - `edge_get_heap` - Get an edge by ID
//! - `edge_query_by_type_heap` - Query edges by type
//! - `edge_query_by_trajectory_heap` - Query edges by trajectory

use pgrx::prelude::*;
use pgrx::pg_sys;

use caliber_core::{
    CaliberError, CaliberResult, Edge, EdgeParticipant, EdgeType, EntityId,
    EntityType, ExtractionMethod, Provenance, StorageError,
};

use crate::column_maps::edge;
use crate::heap_ops::{
    current_timestamp, form_tuple, insert_tuple, open_relation,
    PgLockMode as LockMode, HeapRelation, get_active_snapshot, timestamp_to_pgrx,
};
use crate::index_ops::{
    init_scan_key, open_index, update_indexes_for_insert,
    BTreeStrategy, IndexScanner, operator_oids,
};
use crate::tuple_extract::{
    extract_uuid, extract_text, extract_timestamp, extract_jsonb,
    extract_float4, extract_i32, uuid_to_datum, string_to_datum,
    json_to_datum, float4_to_datum, i32_to_datum, timestamp_to_chrono,
};

/// Edge row with tenant ownership metadata.
pub struct EdgeRow {
    pub edge: Edge,
    pub tenant_id: Option<EntityId>,
}

impl From<EdgeRow> for Edge {
    fn from(row: EdgeRow) -> Self {
        row.edge
    }
}

/// Create a new edge using direct heap operations.
///
/// # Arguments
/// * `edge` - The Edge entity to insert
///
/// # Returns
/// * `Ok(EntityId)` - The edge ID on success
/// * `Err(CaliberError)` - On failure
pub fn edge_create_heap(edge: &Edge, tenant_id: EntityId) -> CaliberResult<EntityId> {
    // Open relation with RowExclusive lock for writes
    let rel = open_relation(edge::TABLE_NAME, LockMode::RowExclusive)?;
    validate_edge_relation(&rel)?;

    // Get current transaction timestamp
    let now = current_timestamp();
    let now_datum = timestamp_to_pgrx(now)?.into_datum()
        .ok_or_else(|| CaliberError::Storage(StorageError::InsertFailed {
            entity_type: EntityType::Edge,
            reason: "Failed to convert timestamp to datum".to_string(),
        }))?;

    // Build datum array - must match column order in caliber_edge table
    let mut values: [pg_sys::Datum; edge::NUM_COLS] = [pg_sys::Datum::from(0); edge::NUM_COLS];
    let mut nulls: [bool; edge::NUM_COLS] = [false; edge::NUM_COLS];

    // Column 1: edge_id (UUID, NOT NULL)
    values[edge::EDGE_ID as usize - 1] = uuid_to_datum(edge.edge_id);

    // Column 2: edge_type (TEXT, NOT NULL)
    values[edge::EDGE_TYPE as usize - 1] = string_to_datum(edge_type_to_str(edge.edge_type));

    // Column 3: participants (JSONB, NOT NULL)
    let participants_json = serde_json::to_value(&edge.participants)
        .map_err(|e| CaliberError::Storage(StorageError::InsertFailed {
            entity_type: EntityType::Edge,
            reason: format!("Failed to serialize participants: {}", e),
        }))?;
    values[edge::PARTICIPANTS as usize - 1] = json_to_datum(&participants_json);

    // Column 4: weight (REAL, nullable)
    if let Some(w) = edge.weight {
        values[edge::WEIGHT as usize - 1] = float4_to_datum(w);
    } else {
        nulls[edge::WEIGHT as usize - 1] = true;
    }

    // Column 5: trajectory_id (UUID, nullable)
    if let Some(traj_id) = edge.trajectory_id {
        values[edge::TRAJECTORY_ID as usize - 1] = uuid_to_datum(traj_id);
    } else {
        nulls[edge::TRAJECTORY_ID as usize - 1] = true;
    }

    // Column 6: source_turn (INTEGER, NOT NULL) - from provenance
    values[edge::SOURCE_TURN as usize - 1] = i32_to_datum(edge.provenance.source_turn);

    // Column 7: extraction_method (TEXT, NOT NULL) - from provenance
    values[edge::EXTRACTION_METHOD as usize - 1] = string_to_datum(
        extraction_method_to_str(&edge.provenance.extraction_method)
    );

    // Column 8: confidence (REAL, nullable) - from provenance
    if let Some(conf) = edge.provenance.confidence {
        values[edge::CONFIDENCE as usize - 1] = float4_to_datum(conf);
    } else {
        nulls[edge::CONFIDENCE as usize - 1] = true;
    }

    // Column 9: created_at (TIMESTAMPTZ, NOT NULL)
    values[edge::CREATED_AT as usize - 1] = now_datum;

    // Column 10: metadata (JSONB, nullable)
    if let Some(ref meta) = edge.metadata {
        values[edge::METADATA as usize - 1] = json_to_datum(meta);
    } else {
        nulls[edge::METADATA as usize - 1] = true;
    }

    // Column 11: tenant_id (UUID, NOT NULL)
    values[edge::TENANT_ID as usize - 1] = uuid_to_datum(tenant_id);

    // Form the heap tuple
    let tuple = form_tuple(&rel, &values, &nulls)?;

    // Insert into heap
    let _tid = unsafe { insert_tuple(&rel, tuple)? };

    // Update all indexes via CatalogIndexInsert
    unsafe { update_indexes_for_insert(&rel, tuple, &values, &nulls)? };

    Ok(edge.edge_id)
}

/// Get an edge by ID using direct heap operations.
///
/// # Arguments
/// * `id` - The edge ID to look up
///
/// # Returns
/// * `Ok(Some(Edge))` - The edge if found
/// * `Ok(None)` - If no edge with that ID exists
/// * `Err(CaliberError)` - On failure
pub fn edge_get_heap(id: EntityId, tenant_id: EntityId) -> CaliberResult<Option<EdgeRow>> {
    // Open relation with AccessShare lock for reads
    let rel = open_relation(edge::TABLE_NAME, LockMode::AccessShare)?;

    // Open the primary key index
    let index_rel = open_index(edge::PK_INDEX)?;

    // Get active snapshot for visibility
    let snapshot = get_active_snapshot();

    // Build scan key for primary key lookup
    let mut scan_key = pg_sys::ScanKeyData::default();
    init_scan_key(
        &mut scan_key,
        1, // First column of index (edge_id)
        BTreeStrategy::Equal,
        operator_oids::UUID_EQ,
        uuid_to_datum(id),
    );

    // Create index scanner
    let mut scanner = unsafe { IndexScanner::new(
        &rel,
        &index_rel,
        snapshot,
        1,
        &mut scan_key,
    ) };

    // Get the first (and should be only) matching tuple
    if let Some(tuple) = scanner.next() {
        let tuple_desc = rel.tuple_desc();
        let row = unsafe { tuple_to_edge(tuple, tuple_desc) }?;
        if row.tenant_id == Some(tenant_id) {
            Ok(Some(row))
        } else {
            Ok(None)
        }
    } else {
        Ok(None)
    }
}

/// Query edges by type using direct heap operations.
///
/// # Arguments
/// * `edge_type` - The edge type to filter by
///
/// # Returns
/// * `Ok(Vec<Edge>)` - List of matching edges
/// * `Err(CaliberError)` - On failure
pub fn edge_query_by_type_heap(edge_type: EdgeType, tenant_id: EntityId) -> CaliberResult<Vec<EdgeRow>> {
    // Open relation with AccessShare lock for reads
    let rel = open_relation(edge::TABLE_NAME, LockMode::AccessShare)?;

    // Open the type index
    let index_rel = open_index(edge::TYPE_INDEX)?;

    // Get active snapshot for visibility
    let snapshot = get_active_snapshot();

    // Build scan key for type lookup
    let mut scan_key = pg_sys::ScanKeyData::default();
    init_scan_key(
        &mut scan_key,
        1, // First column of index (edge_type)
        BTreeStrategy::Equal,
        operator_oids::TEXT_EQ,
        string_to_datum(edge_type_to_str(edge_type)),
    );

    // Create index scanner
    let mut scanner = unsafe { IndexScanner::new(
        &rel,
        &index_rel,
        snapshot,
        1,
        &mut scan_key,
    ) };

    let tuple_desc = rel.tuple_desc();
    let mut results = Vec::new();

    // Collect all matching tuples
    for tuple in &mut scanner {
        let row = unsafe { tuple_to_edge(tuple, tuple_desc) }?;
        if row.tenant_id == Some(tenant_id) {
            results.push(row);
        }
    }

    Ok(results)
}

/// Query edges by trajectory using direct heap operations.
///
/// # Arguments
/// * `trajectory_id` - The trajectory ID to filter by
///
/// # Returns
/// * `Ok(Vec<Edge>)` - List of matching edges
/// * `Err(CaliberError)` - On failure
pub fn edge_query_by_trajectory_heap(
    trajectory_id: EntityId,
    tenant_id: EntityId,
) -> CaliberResult<Vec<EdgeRow>> {
    // Open relation with AccessShare lock for reads
    let rel = open_relation(edge::TABLE_NAME, LockMode::AccessShare)?;

    // Open the trajectory index
    let index_rel = open_index(edge::TRAJECTORY_INDEX)?;

    // Get active snapshot for visibility
    let snapshot = get_active_snapshot();

    // Build scan key for trajectory_id lookup
    let mut scan_key = pg_sys::ScanKeyData::default();
    init_scan_key(
        &mut scan_key,
        1, // First column of index (trajectory_id)
        BTreeStrategy::Equal,
        operator_oids::UUID_EQ,
        uuid_to_datum(trajectory_id),
    );

    // Create index scanner
    let mut scanner = unsafe { IndexScanner::new(
        &rel,
        &index_rel,
        snapshot,
        1,
        &mut scan_key,
    ) };

    let tuple_desc = rel.tuple_desc();
    let mut results = Vec::new();

    // Collect all matching tuples
    for tuple in &mut scanner {
        let row = unsafe { tuple_to_edge(tuple, tuple_desc) }?;
        if row.tenant_id == Some(tenant_id) {
            results.push(row);
        }
    }

    Ok(results)
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/// Validate that a HeapRelation is suitable for edge operations.
fn validate_edge_relation(rel: &HeapRelation) -> CaliberResult<()> {
    let natts = rel.natts();
    if natts != edge::NUM_COLS as i16 {
        return Err(CaliberError::Storage(StorageError::TransactionFailed {
            reason: format!(
                "Edge relation has {} columns, expected {}",
                natts,
                edge::NUM_COLS
            ),
        }));
    }
    Ok(())
}

/// Convert an EdgeType enum to its string representation (lowercase for SQL).
fn edge_type_to_str(t: EdgeType) -> &'static str {
    match t {
        EdgeType::Supports => "supports",
        EdgeType::Contradicts => "contradicts",
        EdgeType::Supersedes => "supersedes",
        EdgeType::DerivedFrom => "derivedfrom",
        EdgeType::RelatesTo => "relatesto",
        EdgeType::Temporal => "temporal",
        EdgeType::Causal => "causal",
        EdgeType::SynthesizedFrom => "synthesizedfrom",
        EdgeType::Grouped => "grouped",
        EdgeType::Compared => "compared",
    }
}

/// Parse an edge type string to EdgeType enum.
fn str_to_edge_type(s: &str) -> EdgeType {
    match s {
        "supports" => EdgeType::Supports,
        "contradicts" => EdgeType::Contradicts,
        "supersedes" => EdgeType::Supersedes,
        "derivedfrom" => EdgeType::DerivedFrom,
        "relatesto" => EdgeType::RelatesTo,
        "temporal" => EdgeType::Temporal,
        "causal" => EdgeType::Causal,
        "synthesizedfrom" => EdgeType::SynthesizedFrom,
        "grouped" => EdgeType::Grouped,
        "compared" => EdgeType::Compared,
        _ => {
            if s != "relatesto" {
                pgrx::warning!("CALIBER: Unknown edge type '{}', defaulting to RelatesTo", s);
            }
            EdgeType::RelatesTo
        }
    }
}

/// Convert an ExtractionMethod to its string representation.
fn extraction_method_to_str(method: &ExtractionMethod) -> &'static str {
    match method {
        ExtractionMethod::Explicit => "explicit",
        ExtractionMethod::Inferred => "inferred",
        ExtractionMethod::UserProvided => "userprovided",
    }
}

/// Parse an extraction method string to ExtractionMethod enum.
fn str_to_extraction_method(s: &str) -> ExtractionMethod {
    match s {
        "explicit" => ExtractionMethod::Explicit,
        "inferred" => ExtractionMethod::Inferred,
        "userprovided" => ExtractionMethod::UserProvided,
        _ => {
            pgrx::warning!("CALIBER: Unknown extraction method '{}', defaulting to Inferred", s);
            ExtractionMethod::Inferred
        }
    }
}

/// Convert a heap tuple to an Edge struct.
unsafe fn tuple_to_edge(
    tuple: *mut pg_sys::HeapTupleData,
    tuple_desc: pg_sys::TupleDesc,
) -> CaliberResult<EdgeRow> {
    let edge_id = extract_uuid(tuple, tuple_desc, edge::EDGE_ID)?
        .ok_or_else(|| CaliberError::Storage(StorageError::TransactionFailed {
            reason: "edge_id is NULL".to_string(),
        }))?;

    let edge_type_str = extract_text(tuple, tuple_desc, edge::EDGE_TYPE)?
        .ok_or_else(|| CaliberError::Storage(StorageError::TransactionFailed {
            reason: "edge_type is NULL".to_string(),
        }))?;
    let edge_type = str_to_edge_type(&edge_type_str);

    // Parse participants from JSONB
    let participants_json = extract_jsonb(tuple, tuple_desc, edge::PARTICIPANTS)?
        .ok_or_else(|| CaliberError::Storage(StorageError::TransactionFailed {
            reason: "participants is NULL".to_string(),
        }))?;
    let participants: Vec<EdgeParticipant> = serde_json::from_value(participants_json)
        .map_err(|e| CaliberError::Storage(StorageError::TransactionFailed {
            reason: format!("Failed to deserialize participants: {}", e),
        }))?;

    let weight = extract_float4(tuple, tuple_desc, edge::WEIGHT)?;

    let trajectory_id = extract_uuid(tuple, tuple_desc, edge::TRAJECTORY_ID)?;

    // Build provenance from individual columns
    let source_turn = extract_i32(tuple, tuple_desc, edge::SOURCE_TURN)?
        .ok_or_else(|| CaliberError::Storage(StorageError::TransactionFailed {
            reason: "source_turn is NULL".to_string(),
        }))?;

    let extraction_method_str = extract_text(tuple, tuple_desc, edge::EXTRACTION_METHOD)?
        .ok_or_else(|| CaliberError::Storage(StorageError::TransactionFailed {
            reason: "extraction_method is NULL".to_string(),
        }))?;
    let extraction_method = str_to_extraction_method(&extraction_method_str);

    let confidence = extract_float4(tuple, tuple_desc, edge::CONFIDENCE)?;

    let provenance = Provenance {
        source_turn,
        extraction_method,
        confidence,
    };

    let created_at_ts = extract_timestamp(tuple, tuple_desc, edge::CREATED_AT)?
        .ok_or_else(|| CaliberError::Storage(StorageError::TransactionFailed {
            reason: "created_at is NULL".to_string(),
        }))?;
    let created_at = timestamp_to_chrono(created_at_ts);

    let metadata = extract_jsonb(tuple, tuple_desc, edge::METADATA)?;

    let tenant_id = extract_uuid(tuple, tuple_desc, edge::TENANT_ID)?;

    Ok(EdgeRow {
        edge: Edge {
            edge_id,
            edge_type,
            participants,
            weight,
            trajectory_id,
            provenance,
            created_at,
            metadata,
        },
        tenant_id,
    })
}

// ============================================================================
// PROPERTY-BASED TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    // ========================================================================
    // Test Helpers - Generators for Edge data
    // ========================================================================

    fn arb_edge_type() -> impl Strategy<Value = EdgeType> {
        prop_oneof![
            Just(EdgeType::Supports),
            Just(EdgeType::Contradicts),
            Just(EdgeType::Supersedes),
            Just(EdgeType::DerivedFrom),
            Just(EdgeType::RelatesTo),
            Just(EdgeType::SynthesizedFrom),
            Just(EdgeType::Grouped),
            Just(EdgeType::Compared),
        ]
    }

    fn arb_extraction_method() -> impl Strategy<Value = ExtractionMethod> {
        prop_oneof![
            Just(ExtractionMethod::Explicit),
            Just(ExtractionMethod::Inferred),
            Just(ExtractionMethod::UserProvided),
        ]
    }

    fn arb_provenance() -> impl Strategy<Value = Provenance> {
        (
            0i32..100i32,
            arb_extraction_method(),
            proptest::option::of(0.0f32..1.0f32),
        )
            .prop_map(|(source_turn, extraction_method, confidence)| Provenance {
                source_turn,
                extraction_method,
                confidence,
            })
    }

    // ========================================================================
    // Property 1: EdgeType Round Trip
    // ========================================================================

    #[test]
    fn test_edge_type_roundtrip() {
        let types = vec![
            EdgeType::Supports,
            EdgeType::Contradicts,
            EdgeType::Supersedes,
            EdgeType::DerivedFrom,
            EdgeType::RelatesTo,
            EdgeType::Temporal,
            EdgeType::Causal,
            EdgeType::SynthesizedFrom,
            EdgeType::Grouped,
            EdgeType::Compared,
        ];

        for t in types {
            let s = edge_type_to_str(t);
            let recovered = str_to_edge_type(s);
            assert_eq!(t, recovered, "EdgeType {} failed roundtrip", s);
        }
    }

    #[test]
    fn test_extraction_method_roundtrip() {
        let methods = vec![
            ExtractionMethod::Explicit,
            ExtractionMethod::Inferred,
            ExtractionMethod::UserProvided,
        ];

        for m in methods {
            let s = extraction_method_to_str(&m);
            let recovered = str_to_extraction_method(s);
            assert_eq!(m, recovered, "ExtractionMethod {} failed roundtrip", s);
        }
    }

    // ========================================================================
    // Property-based tests requiring PostgreSQL
    // ========================================================================

    #[cfg(feature = "pg_test")]
    mod pg_tests {
        use super::*;
        use caliber_core::EntityRef;
        use crate::pg_test;

        /// Property 1: Insert-Get Round Trip (Edge)
        #[pg_test]
        fn prop_edge_insert_get_roundtrip() {
            use proptest::test_runner::{TestRunner, Config};

            let config = Config::with_cases(30);
            let mut runner = TestRunner::new(config);

            let strategy = (
                arb_edge_type(),
                arb_provenance(),
                proptest::option::of(0.0f32..1.0f32),
            );

            runner.run(&strategy, |(edge_type, provenance, weight)| {
                let edge_id = caliber_core::new_entity_id();
                let tenant_id = caliber_core::new_entity_id();

                // Create two participants for a binary edge
                let participant_a = EdgeParticipant {
                    entity_ref: EntityRef {
                        entity_type: EntityType::Note,
                        id: caliber_core::new_entity_id(),
                    },
                    role: Some("source".to_string()),
                };
                let participant_b = EdgeParticipant {
                    entity_ref: EntityRef {
                        entity_type: EntityType::Note,
                        id: caliber_core::new_entity_id(),
                    },
                    role: Some("target".to_string()),
                };

                let edge = Edge {
                    edge_id,
                    edge_type,
                    participants: vec![participant_a.clone(), participant_b.clone()],
                    weight,
                    trajectory_id: None,
                    provenance: provenance.clone(),
                    created_at: chrono::Utc::now(),
                    metadata: None,
                };

                // Insert via heap
                let result = edge_create_heap(&edge, tenant_id);
                prop_assert!(result.is_ok(), "Insert should succeed");
                prop_assert_eq!(result.unwrap(), edge_id);

                // Get via heap
                let get_result = edge_get_heap(edge_id, tenant_id);
                prop_assert!(get_result.is_ok(), "Get should succeed");

                let retrieved = get_result.unwrap();
                prop_assert!(retrieved.is_some(), "Edge should be found");

                let e = retrieved.unwrap().edge;

                // Verify round-trip preserves data
                prop_assert_eq!(e.edge_id, edge_id);
                prop_assert_eq!(e.edge_type, edge_type);
                prop_assert_eq!(e.participants.len(), 2);
                prop_assert_eq!(e.weight, weight);
                prop_assert_eq!(e.trajectory_id, None);
                prop_assert_eq!(e.provenance.source_turn, provenance.source_turn);
                prop_assert_eq!(e.provenance.extraction_method, provenance.extraction_method);
                prop_assert!(e.metadata.is_none());

                Ok(())
            }).unwrap();
        }

        /// Get non-existent edge returns None
        #[pg_test]
        fn prop_edge_get_nonexistent_returns_none() {
            use proptest::test_runner::{TestRunner, Config};

            let config = Config::with_cases(50);
            let mut runner = TestRunner::new(config);

            runner.run(&any::<[u8; 16]>(), |bytes| {
                let random_id = uuid::Uuid::from_bytes(bytes);

                let tenant_id = caliber_core::new_entity_id();
                let result = edge_get_heap(random_id, tenant_id);
                prop_assert!(result.is_ok(), "Get should not error");
                prop_assert!(result.unwrap().is_none(), "Non-existent edge should return None");

                Ok(())
            }).unwrap();
        }
    }
}
