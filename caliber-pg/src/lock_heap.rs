//! Direct heap operations for Lock entities.
//!
//! This module provides hot-path operations for distributed locks that bypass SQL
//! parsing entirely by using direct heap access via pgrx.

use pgrx::prelude::*;
use pgrx::pg_sys;

use caliber_core::{
    CaliberError, CaliberResult, EntityId, EntityType, StorageError,
};
use caliber_agents::{DistributedLock, LockMode};

use crate::column_maps::lock;
use crate::heap_ops::{
    current_timestamp, delete_tuple, form_tuple, insert_tuple, open_relation,
    LockMode as HeapLockMode, HeapRelation, get_active_snapshot,
    timestamp_to_pgrx,
};
use crate::index_ops::{
    init_scan_key, open_index, update_indexes_for_insert,
    BTreeStrategy, IndexScanner, operator_oids,
};
use crate::tuple_extract::{
    extract_uuid, extract_text, extract_timestamp,
    extract_values_and_nulls, uuid_to_datum, string_to_datum,
    timestamp_to_chrono, chrono_to_timestamp,
};

/// Acquire a lock by inserting a lock record using direct heap operations.
pub fn lock_acquire_heap(
    lock_id: EntityId,
    resource_type: &str,
    resource_id: EntityId,
    holder_agent_id: EntityId,
    expires_at: chrono::DateTime<chrono::Utc>,
    mode: LockMode,
) -> CaliberResult<EntityId> {
    let rel = open_relation(lock::TABLE_NAME, HeapLockMode::RowExclusive)?;
    validate_lock_relation(&rel)?;

    let now = current_timestamp();
    let now_datum = timestamp_to_pgrx(now).into_datum()
        .ok_or_else(|| CaliberError::Storage(StorageError::InsertFailed {
            entity_type: EntityType::Lock,
            reason: "Failed to convert timestamp to datum".to_string(),
        }))?;
    
    let expires_datum = chrono_to_timestamp(expires_at).into_datum()
        .ok_or_else(|| CaliberError::Storage(StorageError::InsertFailed {
            entity_type: EntityType::Lock,
            reason: "Failed to convert expires_at to datum".to_string(),
        }))?;

    let mut values: [pg_sys::Datum; lock::NUM_COLS] = [pg_sys::Datum::from(0); lock::NUM_COLS];
    let nulls: [bool; lock::NUM_COLS] = [false; lock::NUM_COLS];
    
    values[lock::LOCK_ID as usize - 1] = uuid_to_datum(lock_id);
    values[lock::RESOURCE_TYPE as usize - 1] = string_to_datum(resource_type);
    values[lock::RESOURCE_ID as usize - 1] = uuid_to_datum(resource_id);
    values[lock::HOLDER_AGENT_ID as usize - 1] = uuid_to_datum(holder_agent_id);
    values[lock::ACQUIRED_AT as usize - 1] = now_datum;
    values[lock::EXPIRES_AT as usize - 1] = expires_datum;
    
    let mode_str = match mode {
        LockMode::Exclusive => "exclusive",
        LockMode::Shared => "shared",
    };
    values[lock::MODE as usize - 1] = string_to_datum(mode_str);
    
    let tuple = form_tuple(&rel, &values, &nulls)?;
    let _tid = insert_tuple(&rel, tuple)?;
    update_indexes_for_insert(&rel, tuple, &values, &nulls)?;
    
    Ok(lock_id)
}

/// Release a lock by deleting its record using direct heap operations.
pub fn lock_release_heap(lock_id: EntityId) -> CaliberResult<bool> {
    let rel = open_relation(lock::TABLE_NAME, HeapLockMode::RowExclusive)?;
    let index_rel = open_index(lock::PK_INDEX)?;
    let snapshot = get_active_snapshot();
    
    let mut scan_key = pg_sys::ScanKeyData::default();
    init_scan_key(
        &mut scan_key,
        1,
        BTreeStrategy::Equal,
        operator_oids::UUID_EQ,
        uuid_to_datum(lock_id),
    );
    
    let mut scanner = IndexScanner::new(&rel, &index_rel, snapshot, 1, &mut scan_key);
    
    if let Some(_tuple) = scanner.next() {
        let tid = scanner.current_tid()
            .ok_or_else(|| CaliberError::Storage(StorageError::TransactionFailed {
                reason: "Failed to get TID of lock tuple".to_string(),
            }))?;
        
        delete_tuple(&rel, &tid)?;
        Ok(true)
    } else {
        Ok(false)
    }
}

/// Get a lock by ID using direct heap operations.
pub fn lock_get_heap(lock_id: EntityId) -> CaliberResult<Option<DistributedLock>> {
    let rel = open_relation(lock::TABLE_NAME, HeapLockMode::AccessShare)?;
    let index_rel = open_index(lock::PK_INDEX)?;
    let snapshot = get_active_snapshot();
    
    let mut scan_key = pg_sys::ScanKeyData::default();
    init_scan_key(
        &mut scan_key,
        1,
        BTreeStrategy::Equal,
        operator_oids::UUID_EQ,
        uuid_to_datum(lock_id),
    );
    
    let mut scanner = IndexScanner::new(&rel, &index_rel, snapshot, 1, &mut scan_key);
    
    if let Some(tuple) = scanner.next() {
        let tuple_desc = rel.tuple_desc();
        let lock = tuple_to_lock(tuple, tuple_desc)?;
        Ok(Some(lock))
    } else {
        Ok(None)
    }
}

/// List locks by resource type and ID using direct heap operations.
pub fn lock_list_by_resource_heap(
    resource_type: &str,
    resource_id: EntityId,
) -> CaliberResult<Vec<DistributedLock>> {
    let rel = open_relation(lock::TABLE_NAME, HeapLockMode::AccessShare)?;
    let index_rel = open_index(lock::RESOURCE_INDEX)?;
    let snapshot = get_active_snapshot();
    
    let mut scan_keys: [pg_sys::ScanKeyData; 2] = [
        pg_sys::ScanKeyData::default(),
        pg_sys::ScanKeyData::default(),
    ];
    
    init_scan_key(
        &mut scan_keys[0],
        1,
        BTreeStrategy::Equal,
        operator_oids::TEXT_EQ,
        string_to_datum(resource_type),
    );
    
    init_scan_key(
        &mut scan_keys[1],
        2,
        BTreeStrategy::Equal,
        operator_oids::UUID_EQ,
        uuid_to_datum(resource_id),
    );
    
    let mut scanner = IndexScanner::new(&rel, &index_rel, snapshot, 2, scan_keys.as_mut_ptr());
    
    let tuple_desc = rel.tuple_desc();
    let mut results = Vec::new();
    
    while let Some(tuple) = scanner.next() {
        let lock = tuple_to_lock(tuple, tuple_desc)?;
        results.push(lock);
    }
    
    Ok(results)
}

/// Validate that a HeapRelation is suitable for lock operations.
fn validate_lock_relation(rel: &HeapRelation) -> CaliberResult<()> {
    let natts = rel.natts();
    if natts != lock::NUM_COLS as i16 {
        return Err(CaliberError::Storage(StorageError::TransactionFailed {
            reason: format!(
                "Lock relation has {} columns, expected {}",
                natts,
                lock::NUM_COLS
            ),
        }));
    }
    Ok(())
}

fn tuple_to_lock(
    tuple: *mut pg_sys::HeapTupleData,
    tuple_desc: pg_sys::TupleDesc,
) -> CaliberResult<DistributedLock> {
    let lock_id = extract_uuid(tuple, tuple_desc, lock::LOCK_ID)?
        .ok_or_else(|| CaliberError::Storage(StorageError::TransactionFailed {
            reason: "lock_id is NULL".to_string(),
        }))?;
    
    let resource_type = extract_text(tuple, tuple_desc, lock::RESOURCE_TYPE)?
        .ok_or_else(|| CaliberError::Storage(StorageError::TransactionFailed {
            reason: "resource_type is NULL".to_string(),
        }))?;
    
    let resource_id = extract_uuid(tuple, tuple_desc, lock::RESOURCE_ID)?
        .ok_or_else(|| CaliberError::Storage(StorageError::TransactionFailed {
            reason: "resource_id is NULL".to_string(),
        }))?;
    
    let holder_agent_id = extract_uuid(tuple, tuple_desc, lock::HOLDER_AGENT_ID)?
        .ok_or_else(|| CaliberError::Storage(StorageError::TransactionFailed {
            reason: "holder_agent_id is NULL".to_string(),
        }))?;
    
    let acquired_at_ts = extract_timestamp(tuple, tuple_desc, lock::ACQUIRED_AT)?
        .ok_or_else(|| CaliberError::Storage(StorageError::TransactionFailed {
            reason: "acquired_at is NULL".to_string(),
        }))?;
    let acquired_at = timestamp_to_chrono(acquired_at_ts);
    
    let expires_at_ts = extract_timestamp(tuple, tuple_desc, lock::EXPIRES_AT)?
        .ok_or_else(|| CaliberError::Storage(StorageError::TransactionFailed {
            reason: "expires_at is NULL".to_string(),
        }))?;
    let expires_at = timestamp_to_chrono(expires_at_ts);
    
    let mode_str = extract_text(tuple, tuple_desc, lock::MODE)?
        .ok_or_else(|| CaliberError::Storage(StorageError::TransactionFailed {
            reason: "mode is NULL".to_string(),
        }))?;
    let mode = match mode_str.as_str() {
        "exclusive" => LockMode::Exclusive,
        "shared" => LockMode::Shared,
        _ => LockMode::Exclusive,
    };
    
    Ok(DistributedLock {
        lock_id,
        resource_type,
        resource_id,
        holder_agent_id,
        acquired_at,
        expires_at,
        mode,
    })
}

/// Extend a lock's expiration time using direct heap operations.
/// Uses extract_values_and_nulls to read existing tuple and update expires_at.
pub fn lock_extend_heap(
    lock_id: EntityId,
    new_expires_at: chrono::DateTime<chrono::Utc>,
) -> CaliberResult<bool> {
    use crate::heap_ops::update_tuple;

    let rel = open_relation(lock::TABLE_NAME, HeapLockMode::RowExclusive)?;
    let index_rel = open_index(lock::PK_INDEX)?;
    let snapshot = get_active_snapshot();
    let tuple_desc = rel.tuple_desc();

    let mut scan_key = pg_sys::ScanKeyData::default();
    init_scan_key(
        &mut scan_key,
        1,
        BTreeStrategy::Equal,
        operator_oids::UUID_EQ,
        uuid_to_datum(lock_id),
    );

    let mut scanner = IndexScanner::new(&rel, &index_rel, snapshot, 1, &mut scan_key);

    if let Some(tuple) = scanner.next() {
        let tid = scanner.current_tid()
            .ok_or_else(|| CaliberError::Storage(StorageError::TransactionFailed {
                reason: "Failed to get TID of lock tuple".to_string(),
            }))?;

        // Extract existing values using extract_values_and_nulls
        let (mut values, mut nulls) = extract_values_and_nulls(tuple, tuple_desc)?;

        // Update expires_at
        values[lock::EXPIRES_AT as usize - 1] = chrono_to_timestamp(new_expires_at).into_datum()
            .ok_or_else(|| CaliberError::Storage(StorageError::UpdateFailed {
                entity_type: EntityType::Lock,
                id: lock_id,
                reason: "Failed to convert expires_at to datum".to_string(),
            }))?;
        nulls[lock::EXPIRES_AT as usize - 1] = false;

        // Form and update tuple
        let new_tuple = form_tuple(&rel, &values, &nulls)?;
        update_tuple(&rel, &tid, new_tuple)?;

        Ok(true)
    } else {
        Ok(false)
    }
}

// ============================================================================
// PROPERTY-BASED TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use chrono::Duration;

    // ========================================================================
    // Test Helpers - Generators for Lock data
    // ========================================================================

    /// Generate a valid resource type
    fn arb_resource_type() -> impl Strategy<Value = String> {
        prop_oneof![
            Just("trajectory".to_string()),
            Just("scope".to_string()),
            Just("artifact".to_string()),
            Just("note".to_string()),
        ]
    }

    /// Generate a random EntityId
    fn arb_entity_id() -> impl Strategy<Value = EntityId> {
        any::<[u8; 16]>().prop_map(|bytes| uuid::Uuid::from_bytes(bytes))
    }

    /// Generate a lock mode
    fn arb_lock_mode() -> impl Strategy<Value = LockMode> {
        prop_oneof![
            Just(LockMode::Exclusive),
            Just(LockMode::Shared),
        ]
    }

    /// Generate a future expiration time (1-60 seconds from now)
    fn arb_expires_at() -> impl Strategy<Value = chrono::DateTime<chrono::Utc>> {
        (1i64..60i64).prop_map(|seconds| {
            chrono::Utc::now() + Duration::seconds(seconds)
        })
    }

    // ========================================================================
    // Property 1: Insert-Get Round Trip (Lock)
    // Feature: caliber-pg-hot-path, Property 1: Insert-Get Round Trip
    // Validates: Requirements 6.1, 6.2, 6.3
    // ========================================================================

    #[cfg(feature = "pg_test")]
    mod pg_tests {
        use super::*;
        use pgrx_tests::pg_test;

        /// Property 1: Insert-Get Round Trip (Lock)
        /// 
        /// *For any* valid lock data (resource_type, resource_id, holder_agent_id, mode),
        /// inserting via direct heap then getting via direct heap SHALL
        /// return an equivalent lock.
        ///
        /// **Validates: Requirements 6.1, 6.3**
        #[pg_test]
        fn prop_lock_insert_get_roundtrip() {
            use proptest::test_runner::{TestRunner, Config};

            let config = Config::with_cases(100);
            let mut runner = TestRunner::new(config);

            let strategy = (
                arb_resource_type(),
                arb_entity_id(),
                arb_entity_id(),
                arb_expires_at(),
                arb_lock_mode(),
            );

            runner.run(&strategy, |(resource_type, resource_id, holder_agent_id, expires_at, mode)| {
                // Generate a new lock ID
                let lock_id = caliber_core::new_entity_id();

                // Insert via heap
                let result = lock_acquire_heap(
                    lock_id,
                    &resource_type,
                    resource_id,
                    holder_agent_id,
                    expires_at,
                    mode,
                );
                prop_assert!(result.is_ok(), "Insert should succeed: {:?}", result.err());
                prop_assert_eq!(result.unwrap(), lock_id);

                // Get via heap
                let get_result = lock_get_heap(lock_id);
                prop_assert!(get_result.is_ok(), "Get should succeed: {:?}", get_result.err());
                
                let lock = get_result.unwrap();
                prop_assert!(lock.is_some(), "Lock should be found");
                
                let l = lock.unwrap();
                
                // Verify round-trip preserves data
                prop_assert_eq!(l.lock_id, lock_id);
                prop_assert_eq!(l.resource_type, resource_type);
                prop_assert_eq!(l.resource_id, resource_id);
                prop_assert_eq!(l.holder_agent_id, holder_agent_id);
                prop_assert_eq!(l.mode, mode);
                
                // Timestamps should be set
                prop_assert!(l.acquired_at <= chrono::Utc::now());

                Ok(())
            }).unwrap();
        }

        /// Property 1 (edge case): Get non-existent lock returns None
        ///
        /// *For any* random UUID that was never inserted, getting it SHALL
        /// return Ok(None), not an error.
        ///
        /// **Validates: Requirements 6.3**
        #[pg_test]
        fn prop_lock_get_nonexistent_returns_none() {
            use proptest::test_runner::{TestRunner, Config};

            let config = Config::with_cases(100);
            let mut runner = TestRunner::new(config);

            runner.run(&any::<[u8; 16]>(), |bytes| {
                let random_id = uuid::Uuid::from_bytes(bytes);
                
                let result = lock_get_heap(random_id);
                prop_assert!(result.is_ok(), "Get should not error: {:?}", result.err());
                prop_assert!(result.unwrap().is_none(), "Non-existent lock should return None");

                Ok(())
            }).unwrap();
        }

        /// Property 4: Delete Removes from Index
        ///
        /// *For any* lock that has been inserted, deleting it via direct heap
        /// then querying via index SHALL NOT return that lock.
        ///
        /// **Validates: Requirements 6.2**
        #[pg_test]
        fn prop_lock_delete_removes_from_index() {
            use proptest::test_runner::{TestRunner, Config};

            let config = Config::with_cases(100);
            let mut runner = TestRunner::new(config);

            let strategy = (
                arb_resource_type(),
                arb_entity_id(),
                arb_entity_id(),
                arb_expires_at(),
                arb_lock_mode(),
            );

            runner.run(&strategy, |(resource_type, resource_id, holder_agent_id, expires_at, mode)| {
                // Generate a new lock ID
                let lock_id = caliber_core::new_entity_id();

                // Insert via heap
                let insert_result = lock_acquire_heap(
                    lock_id,
                    &resource_type,
                    resource_id,
                    holder_agent_id,
                    expires_at,
                    mode,
                );
                prop_assert!(insert_result.is_ok(), "Insert should succeed");

                // Verify it exists
                let get_result = lock_get_heap(lock_id);
                prop_assert!(get_result.is_ok(), "Get should succeed");
                prop_assert!(get_result.unwrap().is_some(), "Lock should exist before delete");

                // Delete via heap
                let delete_result = lock_release_heap(lock_id);
                prop_assert!(delete_result.is_ok(), "Delete should succeed: {:?}", delete_result.err());
                prop_assert!(delete_result.unwrap(), "Delete should return true for existing lock");

                // Verify it no longer exists via primary key index
                let get_after_delete = lock_get_heap(lock_id);
                prop_assert!(get_after_delete.is_ok(), "Get after delete should not error");
                prop_assert!(get_after_delete.unwrap().is_none(), "Lock should not be found after delete");

                // Verify it no longer exists via resource index
                let list_result = lock_list_by_resource_heap(&resource_type, resource_id);
                prop_assert!(list_result.is_ok(), "List by resource should succeed");
                let locks = list_result.unwrap();
                prop_assert!(
                    !locks.iter().any(|l| l.lock_id == lock_id),
                    "Deleted lock should not appear in resource index query"
                );

                Ok(())
            }).unwrap();
        }

        /// Property 4 (edge case): Delete non-existent lock returns false
        ///
        /// *For any* random UUID that was never inserted, deleting it SHALL
        /// return Ok(false), not an error.
        ///
        /// **Validates: Requirements 6.2**
        #[pg_test]
        fn prop_lock_delete_nonexistent_returns_false() {
            use proptest::test_runner::{TestRunner, Config};

            let config = Config::with_cases(100);
            let mut runner = TestRunner::new(config);

            runner.run(&any::<[u8; 16]>(), |bytes| {
                let random_id = uuid::Uuid::from_bytes(bytes);
                
                let result = lock_release_heap(random_id);
                prop_assert!(result.is_ok(), "Delete should not error: {:?}", result.err());
                prop_assert!(!result.unwrap(), "Delete of non-existent lock should return false");

                Ok(())
            }).unwrap();
        }

        /// Property 3: Index Consistency - Resource Index
        ///
        /// *For any* lock inserted via direct heap, querying via the resource
        /// index SHALL return that lock.
        ///
        /// **Validates: Requirements 6.4, 13.1, 13.2, 13.4, 13.5**
        #[pg_test]
        fn prop_lock_resource_index_consistency() {
            use proptest::test_runner::{TestRunner, Config};

            let config = Config::with_cases(100);
            let mut runner = TestRunner::new(config);

            let strategy = (
                arb_resource_type(),
                arb_entity_id(),
                arb_entity_id(),
                arb_expires_at(),
                arb_lock_mode(),
            );

            runner.run(&strategy, |(resource_type, resource_id, holder_agent_id, expires_at, mode)| {
                // Generate a new lock ID
                let lock_id = caliber_core::new_entity_id();

                // Insert via heap
                let insert_result = lock_acquire_heap(
                    lock_id,
                    &resource_type,
                    resource_id,
                    holder_agent_id,
                    expires_at,
                    mode,
                );
                prop_assert!(insert_result.is_ok(), "Insert should succeed");

                // Query via resource index
                let list_result = lock_list_by_resource_heap(&resource_type, resource_id);
                prop_assert!(list_result.is_ok(), "List by resource should succeed: {:?}", list_result.err());
                
                let locks = list_result.unwrap();
                prop_assert!(
                    locks.iter().any(|l| l.lock_id == lock_id),
                    "Inserted lock should be found via resource index"
                );

                // Verify the found lock has correct data
                let found_lock = locks.iter().find(|l| l.lock_id == lock_id).unwrap();
                prop_assert_eq!(found_lock.resource_type, resource_type);
                prop_assert_eq!(found_lock.resource_id, resource_id);
                prop_assert_eq!(found_lock.holder_agent_id, holder_agent_id);
                prop_assert_eq!(found_lock.mode, mode);

                Ok(())
            }).unwrap();
        }
    }
}
