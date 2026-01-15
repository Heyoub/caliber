//! Direct heap operations for Turn entities.
//!
//! This module provides hot-path operations for turns that bypass SQL
//! parsing entirely by using direct heap access via pgrx.
//!
//! # Operations
//!
//! - `turn_create_heap` - Insert a new turn
//! - `turn_get_by_scope_heap` - Get turns by scope ID (ordered by sequence)

use pgrx::prelude::*;
use pgrx::pg_sys;

use caliber_core::{
    CaliberError, CaliberResult, EntityId, EntityType, StorageError,
    Turn, TurnRole,
};

use crate::column_maps::turn;
use crate::heap_ops::{
    current_timestamp, form_tuple, insert_tuple, open_relation,
    LockMode, HeapRelation, get_active_snapshot, timestamp_to_pgrx,
};
use crate::index_ops::{
    init_scan_key, open_index, update_indexes_for_insert,
    BTreeStrategy, IndexScanner, operator_oids,
};
use crate::tuple_extract::{
    extract_uuid, extract_text, extract_timestamp, extract_jsonb,
    extract_i32, uuid_to_datum, string_to_datum, i32_to_datum,
    json_to_datum, timestamp_to_chrono,
};

/// Create a new turn using direct heap operations.
///
/// # Arguments
/// * `turn_id` - The pre-generated UUIDv7 for this turn
/// * `scope_id` - The parent scope ID
/// * `sequence` - The sequence number within the scope
/// * `role` - The turn role (user, assistant, system, tool)
/// * `content` - The turn content
/// * `token_count` - Number of tokens in this turn
/// * `tool_calls` - Optional tool calls JSON
/// * `tool_results` - Optional tool results JSON
///
/// # Returns
/// * `Ok(EntityId)` - The turn ID on success
/// * `Err(CaliberError)` - On failure
///
/// # Requirements
/// - 5.1: Uses heap_form_tuple and simple_heap_insert instead of SPI
/// - 5.3: Updates the scope_id index via CatalogIndexInsert
pub fn turn_create_heap(
    turn_id: EntityId,
    scope_id: EntityId,
    sequence: i32,
    role: TurnRole,
    content: &str,
    token_count: i32,
    tool_calls: Option<&serde_json::Value>,
    tool_results: Option<&serde_json::Value>,
) -> CaliberResult<EntityId> {
    // Open relation with RowExclusive lock for writes
    let rel = open_relation(turn::TABLE_NAME, LockMode::RowExclusive)?;

    // Get current transaction timestamp for created_at
    let now = current_timestamp();
    let now_datum = timestamp_to_pgrx(now).into_datum()
        .ok_or_else(|| CaliberError::Storage(StorageError::InsertFailed {
            entity_type: EntityType::Turn,
            reason: "Failed to convert timestamp to datum".to_string(),
        }))?;
    
    // Build datum array - must match column order in caliber_turn table
    let mut values: [pg_sys::Datum; turn::NUM_COLS] = [pg_sys::Datum::from(0); turn::NUM_COLS];
    let mut nulls: [bool; turn::NUM_COLS] = [false; turn::NUM_COLS];
    
    // Column 1: turn_id (UUID, NOT NULL)
    values[turn::TURN_ID as usize - 1] = uuid_to_datum(turn_id);
    
    // Column 2: scope_id (UUID, NOT NULL)
    values[turn::SCOPE_ID as usize - 1] = uuid_to_datum(scope_id);
    
    // Column 3: sequence (INTEGER, NOT NULL)
    values[turn::SEQUENCE as usize - 1] = i32_to_datum(sequence);
    
    // Column 4: role (TEXT, NOT NULL)
    values[turn::ROLE as usize - 1] = string_to_datum(role_to_str(role));
    
    // Column 5: content (TEXT, NOT NULL)
    values[turn::CONTENT as usize - 1] = string_to_datum(content);
    
    // Column 6: token_count (INTEGER, NOT NULL)
    values[turn::TOKEN_COUNT as usize - 1] = i32_to_datum(token_count);
    
    // Column 7: created_at (TIMESTAMPTZ, NOT NULL)
    values[turn::CREATED_AT as usize - 1] = now_datum;
    
    // Column 8: tool_calls (JSONB, nullable)
    if let Some(tc) = tool_calls {
        values[turn::TOOL_CALLS as usize - 1] = json_to_datum(tc);
    } else {
        nulls[turn::TOOL_CALLS as usize - 1] = true;
    }
    
    // Column 9: tool_results (JSONB, nullable)
    if let Some(tr) = tool_results {
        values[turn::TOOL_RESULTS as usize - 1] = json_to_datum(tr);
    } else {
        nulls[turn::TOOL_RESULTS as usize - 1] = true;
    }
    
    // Column 10: metadata (JSONB, nullable)
    nulls[turn::METADATA as usize - 1] = true;
    
    // Form the heap tuple
    let tuple = form_tuple(&rel, &values, &nulls)?;
    
    // Insert into heap
    let _tid = insert_tuple(&rel, tuple)?;
    
    // Update all indexes via CatalogIndexInsert
    update_indexes_for_insert(&rel, tuple, &values, &nulls)?;
    
    Ok(turn_id)
}

/// Get turns by scope ID using direct heap operations.
///
/// Returns turns ordered by sequence number.
///
/// # Arguments
/// * `scope_id` - The scope ID to filter by
///
/// # Returns
/// * `Ok(Vec<Turn>)` - List of turns ordered by sequence
/// * `Err(CaliberError)` - On failure
///
/// # Requirements
/// - 5.2: Uses index scan on scope_id instead of SPI SELECT
pub fn turn_get_by_scope_heap(scope_id: EntityId) -> CaliberResult<Vec<Turn>> {
    // Open relation with AccessShare lock for reads
    let rel = open_relation(turn::TABLE_NAME, LockMode::AccessShare)?;
    
    // Open the scope index
    let index_rel = open_index(turn::SCOPE_INDEX)?;
    
    // Get active snapshot for visibility
    let snapshot = get_active_snapshot();
    
    // Build scan key for scope_id lookup
    let mut scan_key = pg_sys::ScanKeyData::default();
    init_scan_key(
        &mut scan_key,
        1, // First column of index (scope_id)
        BTreeStrategy::Equal,
        operator_oids::UUID_EQ,
        uuid_to_datum(scope_id),
    );
    
    // Create index scanner
    let mut scanner = IndexScanner::new(
        &rel,
        &index_rel,
        snapshot,
        1,
        &mut scan_key,
    );
    
    let tuple_desc = rel.tuple_desc();
    let mut results = Vec::new();
    
    // Collect all matching tuples
    while let Some(tuple) = scanner.next() {
        let turn = tuple_to_turn(tuple, tuple_desc)?;
        results.push(turn);
    }
    
    // Sort by sequence number
    results.sort_by_key(|t| t.sequence);
    
    Ok(results)
}


// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/// Convert a TurnRole enum to its string representation.
fn role_to_str(role: TurnRole) -> &'static str {
    match role {
        TurnRole::User => "user",
        TurnRole::Assistant => "assistant",
        TurnRole::System => "system",
        TurnRole::Tool => "tool",
    }
}

/// Parse a role string to TurnRole enum.
fn str_to_role(s: &str) -> TurnRole {
    match s {
        "user" => TurnRole::User,
        "assistant" => TurnRole::Assistant,
        "system" => TurnRole::System,
        "tool" => TurnRole::Tool,
        _ => TurnRole::User, // Default fallback
    }
}

/// Convert a heap tuple to a Turn struct.
fn tuple_to_turn(
    tuple: *mut pg_sys::HeapTupleData,
    tuple_desc: pg_sys::TupleDesc,
) -> CaliberResult<Turn> {
    let turn_id = extract_uuid(tuple, tuple_desc, turn::TURN_ID)?
        .ok_or_else(|| CaliberError::Storage(StorageError::TransactionFailed {
            reason: "turn_id is NULL".to_string(),
        }))?;
    
    let scope_id = extract_uuid(tuple, tuple_desc, turn::SCOPE_ID)?
        .ok_or_else(|| CaliberError::Storage(StorageError::TransactionFailed {
            reason: "scope_id is NULL".to_string(),
        }))?;
    
    let sequence = extract_i32(tuple, tuple_desc, turn::SEQUENCE)?
        .ok_or_else(|| CaliberError::Storage(StorageError::TransactionFailed {
            reason: "sequence is NULL".to_string(),
        }))?;
    
    let role_str = extract_text(tuple, tuple_desc, turn::ROLE)?
        .ok_or_else(|| CaliberError::Storage(StorageError::TransactionFailed {
            reason: "role is NULL".to_string(),
        }))?;
    let role = str_to_role(&role_str);
    
    let content = extract_text(tuple, tuple_desc, turn::CONTENT)?
        .ok_or_else(|| CaliberError::Storage(StorageError::TransactionFailed {
            reason: "content is NULL".to_string(),
        }))?;
    
    let token_count = extract_i32(tuple, tuple_desc, turn::TOKEN_COUNT)?
        .ok_or_else(|| CaliberError::Storage(StorageError::TransactionFailed {
            reason: "token_count is NULL".to_string(),
        }))?;
    
    let created_at_ts = extract_timestamp(tuple, tuple_desc, turn::CREATED_AT)?
        .ok_or_else(|| CaliberError::Storage(StorageError::TransactionFailed {
            reason: "created_at is NULL".to_string(),
        }))?;
    let created_at = timestamp_to_chrono(created_at_ts);
    
    let tool_calls = extract_jsonb(tuple, tuple_desc, turn::TOOL_CALLS)?;
    let tool_results = extract_jsonb(tuple, tuple_desc, turn::TOOL_RESULTS)?;
    let metadata = extract_jsonb(tuple, tuple_desc, turn::METADATA)?;
    
    Ok(Turn {
        turn_id,
        scope_id,
        sequence,
        role,
        content,
        token_count,
        created_at,
        tool_calls,
        tool_results,
        metadata,
    })
}

// ============================================================================
// PROPERTY-BASED TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn arb_content() -> impl Strategy<Value = String> {
        "[a-zA-Z0-9 .,!?\\n-]{1,500}".prop_map(|s| s)
    }

    fn arb_role() -> impl Strategy<Value = TurnRole> {
        prop_oneof![
            Just(TurnRole::User),
            Just(TurnRole::Assistant),
            Just(TurnRole::System),
            Just(TurnRole::Tool),
        ]
    }

    fn arb_token_count() -> impl Strategy<Value = i32> {
        1i32..10000i32
    }

    fn arb_sequence() -> impl Strategy<Value = i32> {
        0i32..1000i32
    }

    #[cfg(feature = "pg_test")]
    mod pg_tests {
        use super::*;
        use pgrx_tests::pg_test;

        /// Property 1: Insert-Get Round Trip (Turn)
        /// Validates: Requirements 5.1, 5.2
        #[pg_test]
        fn prop_turn_insert_get_roundtrip() {
            use proptest::test_runner::{TestRunner, Config};

            let config = Config::with_cases(50);
            let mut runner = TestRunner::new(config);

            let strategy = (
                arb_content(),
                arb_role(),
                arb_token_count(),
                arb_sequence(),
            );

            runner.run(&strategy, |(content, role, token_count, sequence)| {
                // Create trajectory and scope first
                let trajectory_id = caliber_core::new_entity_id();
                let _ = crate::trajectory_heap::trajectory_create_heap(
                    trajectory_id,
                    "test_trajectory",
                    None,
                    None,
                );

                let scope_id = caliber_core::new_entity_id();
                let _ = crate::scope_heap::scope_create_heap(
                    scope_id,
                    trajectory_id,
                    "test_scope",
                    None,
                    10000,
                );

                // Create turn
                let turn_id = caliber_core::new_entity_id();
                let result = turn_create_heap(
                    turn_id,
                    scope_id,
                    sequence,
                    role,
                    &content,
                    token_count,
                    None,
                    None,
                );
                prop_assert!(result.is_ok(), "Insert should succeed");
                prop_assert_eq!(result.unwrap(), turn_id);

                // Get by scope
                let get_result = turn_get_by_scope_heap(scope_id);
                prop_assert!(get_result.is_ok(), "Get should succeed");
                
                let turns = get_result.unwrap();
                prop_assert!(!turns.is_empty(), "Should find at least one turn");
                
                // Find our turn
                let t = turns.iter().find(|t| t.turn_id == turn_id);
                prop_assert!(t.is_some(), "Our turn should be in results");
                
                let t = t.unwrap();
                prop_assert_eq!(t.turn_id, turn_id);
                prop_assert_eq!(t.scope_id, scope_id);
                prop_assert_eq!(t.sequence, sequence);
                prop_assert_eq!(t.role, role);
                prop_assert_eq!(t.content, content);
                prop_assert_eq!(t.token_count, token_count);
                prop_assert!(t.tool_calls.is_none());
                prop_assert!(t.tool_results.is_none());
                prop_assert!(t.metadata.is_none());

                Ok(())
            }).unwrap();
        }

        /// Turns are returned ordered by sequence
        #[pg_test]
        fn prop_turn_get_by_scope_ordered() {
            use proptest::test_runner::{TestRunner, Config};

            let config = Config::with_cases(30);
            let mut runner = TestRunner::new(config);

            let strategy = 2usize..6usize;

            runner.run(&strategy, |num_turns| {
                // Create trajectory and scope
                let trajectory_id = caliber_core::new_entity_id();
                let _ = crate::trajectory_heap::trajectory_create_heap(
                    trajectory_id,
                    "test_trajectory",
                    None,
                    None,
                );

                let scope_id = caliber_core::new_entity_id();
                let _ = crate::scope_heap::scope_create_heap(
                    scope_id,
                    trajectory_id,
                    "test_scope",
                    None,
                    10000,
                );

                // Create turns in random order but with sequential sequence numbers
                let mut sequences: Vec<i32> = (0..num_turns as i32).collect();
                // Reverse to insert in non-sequential order
                sequences.reverse();
                
                for seq in &sequences {
                    let turn_id = caliber_core::new_entity_id();
                    let _ = turn_create_heap(
                        turn_id,
                        scope_id,
                        *seq,
                        TurnRole::User,
                        &format!("content_{}", seq),
                        100,
                        None,
                        None,
                    );
                }

                // Get by scope - should be ordered
                let turns = turn_get_by_scope_heap(scope_id).unwrap();
                prop_assert_eq!(turns.len(), num_turns);

                // Verify ordering
                for (i, turn) in turns.iter().enumerate() {
                    prop_assert_eq!(turn.sequence, i as i32, "Turns should be ordered by sequence");
                }

                Ok(())
            }).unwrap();
        }

        /// Get by non-existent scope returns empty
        #[pg_test]
        fn prop_turn_get_by_nonexistent_scope_returns_empty() {
            use proptest::test_runner::{TestRunner, Config};

            let config = Config::with_cases(100);
            let mut runner = TestRunner::new(config);

            runner.run(&any::<[u8; 16]>(), |bytes| {
                let random_id = uuid::Uuid::from_bytes(bytes);
                
                let result = turn_get_by_scope_heap(random_id);
                prop_assert!(result.is_ok(), "Get should not error");
                prop_assert!(result.unwrap().is_empty(), "Get for non-existent scope should be empty");

                Ok(())
            }).unwrap();
        }
    }
}
