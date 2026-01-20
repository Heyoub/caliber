//! Direct heap operation helpers for hot-path performance.
//!
//! These functions bypass SQL parsing entirely by using pgrx's direct access
//! to PostgreSQL's heap storage layer. This eliminates the overhead of:
//! - SQL parsing (~15K lines of gram.y)
//! - Query planning/optimization
//! - Executor setup
//!
//! # Safety
//!
//! All functions in this module wrap unsafe PostgreSQL C functions with safe
//! Rust interfaces. The safety invariants are:
//! - Relations must be opened with appropriate lock modes
//! - Tuples must be formed with correct tuple descriptors
//! - Index operations must maintain consistency
//!
//! # Usage
//!
//! ```ignore
//! use crate::heap_ops::*;
//!
//! // Open relation with row-exclusive lock for writes
//! let rel = open_relation("caliber_trajectory", PgLockMode::RowExclusive)?;
//!
//! // Form tuple from values
//! let tuple = form_tuple(&rel, &values, &nulls)?;
//!
//! // Insert and get TID
//! let tid = insert_tuple(&rel, tuple)?;
//!
//! // Update indexes
//! update_indexes_for_insert(&rel, tuple, tid)?;
//! ```

// Prelude provides common pgrx types and traits used throughout
#[allow(unused_imports)]
use pgrx::prelude::*;
use pgrx::pg_sys;
use pgrx::PgRelation;
use pgrx::datum::TimestampWithTimeZone;

use caliber_core::{CaliberError, CaliberResult, EntityType, StorageError};

/// Lock modes for relation access.
/// Maps to PostgreSQL's LOCKMODE enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PgLockMode {
    /// For read-only operations (SELECT)
    AccessShare,
    /// For SELECT FOR UPDATE
    RowShare,
    /// For INSERT, UPDATE, DELETE
    RowExclusive,
    /// For VACUUM, ANALYZE
    ShareUpdateExclusive,
    /// For CREATE INDEX
    Share,
    /// For REFRESH MATERIALIZED VIEW CONCURRENTLY
    ShareRowExclusive,
    /// For DROP TABLE, TRUNCATE
    Exclusive,
    /// For ALTER TABLE, DROP TABLE
    AccessExclusive,
}

impl PgLockMode {
    /// Convert to PostgreSQL's LOCKMODE value.
    #[inline]
    pub fn to_pg_lockmode(self) -> pg_sys::LOCKMODE {
        match self {
            PgLockMode::AccessShare => pg_sys::AccessShareLock as pg_sys::LOCKMODE,
            PgLockMode::RowShare => pg_sys::RowShareLock as pg_sys::LOCKMODE,
            PgLockMode::RowExclusive => pg_sys::RowExclusiveLock as pg_sys::LOCKMODE,
            PgLockMode::ShareUpdateExclusive => pg_sys::ShareUpdateExclusiveLock as pg_sys::LOCKMODE,
            PgLockMode::Share => pg_sys::ShareLock as pg_sys::LOCKMODE,
            PgLockMode::ShareRowExclusive => pg_sys::ShareRowExclusiveLock as pg_sys::LOCKMODE,
            PgLockMode::Exclusive => pg_sys::ExclusiveLock as pg_sys::LOCKMODE,
            PgLockMode::AccessExclusive => pg_sys::AccessExclusiveLock as pg_sys::LOCKMODE,
        }
    }
}

/// A wrapper around PgRelation that tracks the lock mode.
/// Ensures proper cleanup when dropped.
pub struct HeapRelation {
    inner: PgRelation,
    #[allow(dead_code)]
    lock_mode: PgLockMode,
}

impl AsRef<PgRelation> for HeapRelation {
    #[inline]
    fn as_ref(&self) -> &PgRelation {
        &self.inner
    }
}

impl HeapRelation {
    /// Get the tuple descriptor for this relation.
    #[inline]
    pub fn tuple_desc(&self) -> pg_sys::TupleDesc {
        unsafe { (*self.inner.as_ptr()).rd_att }
    }

    /// Get the relation OID.
    #[inline]
    pub fn oid(&self) -> pg_sys::Oid {
        unsafe { (*self.inner.as_ptr()).rd_id }
    }

    /// Get the number of attributes (columns) in this relation.
    #[inline]
    pub fn natts(&self) -> i16 {
        unsafe { (*self.tuple_desc()).natts as i16 }
    }
}

/// Open a relation by name with the specified lock mode.
///
/// # Arguments
/// * `name` - The relation name (table name)
/// * `lock_mode` - The lock mode to acquire
///
/// # Returns
/// * `Ok(HeapRelation)` - The opened relation
/// * `Err(CaliberError)` - If the relation cannot be opened
///
/// # Example
/// ```ignore
/// let rel = open_relation("caliber_trajectory", PgLockMode::RowExclusive)?;
/// ```
pub fn open_relation(name: &str, lock_mode: PgLockMode) -> CaliberResult<HeapRelation> {
    // Use PgRelation's safe wrapper which handles the lock acquisition
    // In pgrx 0.16+, this returns Result<PgRelation, &str>
    let rel = PgRelation::open_with_name_and_share_lock(name)
        .map_err(|e| CaliberError::Storage(StorageError::TransactionFailed {
            reason: format!("Failed to open relation '{}': {}", name, e),
        }))?;

    // Verify the relation was opened successfully
    if rel.as_ptr().is_null() {
        return Err(CaliberError::Storage(StorageError::TransactionFailed {
            reason: format!("Failed to open relation: {}", name),
        }));
    }

    Ok(HeapRelation {
        inner: rel,
        lock_mode,
    })
}

/// Open a relation by OID with the specified lock mode.
///
/// # Arguments
/// * `oid` - The relation OID
/// * `lock_mode` - The lock mode to acquire
///
/// # Returns
/// * `Ok(HeapRelation)` - The opened relation
/// * `Err(CaliberError)` - If the relation cannot be opened
pub fn open_relation_by_oid(oid: pg_sys::Oid, lock_mode: PgLockMode) -> CaliberResult<HeapRelation> {
    let rel = unsafe {
        PgRelation::with_lock(oid, lock_mode.to_pg_lockmode())
    };

    if rel.as_ptr().is_null() {
        return Err(CaliberError::Storage(StorageError::TransactionFailed {
            reason: format!("Failed to open relation with OID: {}", oid.to_u32()),
        }));
    }

    Ok(HeapRelation {
        inner: rel,
        lock_mode,
    })
}

/// Form a heap tuple from datum values and null flags.
///
/// # Arguments
/// * `rel` - The relation to form the tuple for
/// * `values` - Array of Datum values, one per column
/// * `nulls` - Array of null flags, one per column (true = NULL)
///
/// # Returns
/// * `Ok(HeapTuple)` - The formed tuple (caller must free with pfree)
/// * `Err(CaliberError)` - If tuple formation fails
///
/// # Safety
/// The returned HeapTuple is allocated in the current memory context.
/// It will be automatically freed when the memory context is reset.
pub fn form_tuple(
    rel: &HeapRelation,
    values: &[pg_sys::Datum],
    nulls: &[bool],
) -> CaliberResult<*mut pg_sys::HeapTupleData> {
    let tuple_desc = rel.tuple_desc();
    let natts = rel.natts() as usize;

    // Validate array lengths
    if values.len() != natts {
        return Err(CaliberError::Storage(StorageError::InsertFailed {
            entity_type: EntityType::Trajectory, // Generic, will be overridden by caller
            reason: format!(
                "Values array length {} does not match relation column count {}",
                values.len(),
                natts
            ),
        }));
    }

    if nulls.len() != natts {
        return Err(CaliberError::Storage(StorageError::InsertFailed {
            entity_type: EntityType::Trajectory,
            reason: format!(
                "Nulls array length {} does not match relation column count {}",
                nulls.len(),
                natts
            ),
        }));
    }

    // Convert bool slice to the format heap_form_tuple expects
    let tuple = unsafe {
        pg_sys::heap_form_tuple(
            tuple_desc,
            values.as_ptr() as *mut pg_sys::Datum,
            nulls.as_ptr() as *mut bool,
        )
    };

    if tuple.is_null() {
        return Err(CaliberError::Storage(StorageError::InsertFailed {
            entity_type: EntityType::Trajectory,
            reason: "heap_form_tuple returned null".to_string(),
        }));
    }

    Ok(tuple)
}

/// Insert a tuple into a relation using simple_heap_insert.
///
/// This is the low-level insert that bypasses triggers and rules.
/// Use for hot-path operations where triggers are not needed.
///
/// # Arguments
/// * `rel` - The relation to insert into (must be opened with RowExclusive lock)
/// * `tuple` - The tuple to insert
///
/// # Returns
/// * `Ok(ItemPointerData)` - The TID of the inserted tuple
/// * `Err(CaliberError)` - If insertion fails
///
/// # Note
/// After calling this, you MUST call `update_indexes_for_insert` to maintain
/// index consistency.
///
/// # Safety
/// The tuple pointer must be valid for the target relation and remain valid
/// for the duration of the insert. The relation must be opened with an
/// appropriate lock mode for writes.
pub unsafe fn insert_tuple(
    rel: &HeapRelation,
    tuple: *mut pg_sys::HeapTupleData,
) -> CaliberResult<pg_sys::ItemPointerData> {
    if tuple.is_null() {
        return Err(CaliberError::Storage(StorageError::InsertFailed {
            entity_type: EntityType::Trajectory,
            reason: "Cannot insert null tuple".to_string(),
        }));
    }

    pg_sys::simple_heap_insert(rel.inner.as_ptr(), tuple);

    // Get the TID from the inserted tuple
    let tid = (*tuple).t_self;

    Ok(tid)
}

/// Update a tuple in place using simple_heap_update.
///
/// # Arguments
/// * `rel` - The relation containing the tuple (must be opened with RowExclusive lock)
/// * `otid` - The TID of the tuple to update
/// * `new_tuple` - The new tuple data
///
/// # Returns
/// * `Ok(())` - Update succeeded
/// * `Err(CaliberError)` - If update fails
///
/// # Note
/// After calling this, you MUST call `update_indexes_for_update` if any
/// indexed columns changed.
///
/// # Safety
/// The new_tuple pointer must be valid for the target relation. The relation
/// must be opened with an appropriate lock mode for writes.
pub unsafe fn update_tuple(
    rel: &HeapRelation,
    otid: &pg_sys::ItemPointerData,
    new_tuple: *mut pg_sys::HeapTupleData,
) -> CaliberResult<()> {
    if new_tuple.is_null() {
        return Err(CaliberError::Storage(StorageError::UpdateFailed {
            entity_type: EntityType::Trajectory,
            id: uuid::Uuid::nil(),
            reason: "Cannot update with null tuple".to_string(),
        }));
    }

    // In PostgreSQL 18 / pgrx 0.16, simple_heap_update takes 4 args
    // The 4th arg is an out parameter indicating which indexes to update
    let mut update_indexes = pg_sys::TU_UpdateIndexes::TU_All;
    pg_sys::simple_heap_update(
        rel.inner.as_ptr(),
        otid as *const pg_sys::ItemPointerData as *mut pg_sys::ItemPointerData,
        new_tuple,
        &mut update_indexes,
    );

    Ok(())
}

/// Delete a tuple using simple_heap_delete.
///
/// # Arguments
/// * `rel` - The relation containing the tuple (must be opened with RowExclusive lock)
/// * `tid` - The TID of the tuple to delete
///
/// # Returns
/// * `Ok(())` - Delete succeeded
/// * `Err(CaliberError)` - If delete fails
///
/// # Note
/// Index entries are NOT automatically removed. The indexes will be cleaned
/// up during VACUUM.
///
/// # Safety
/// The TID pointer must be valid for the target relation. The relation must be
/// opened with an appropriate lock mode for writes.
pub unsafe fn delete_tuple(
    rel: &HeapRelation,
    tid: &pg_sys::ItemPointerData,
) -> CaliberResult<()> {
    pg_sys::simple_heap_delete(
        rel.inner.as_ptr(),
        tid as *const pg_sys::ItemPointerData as *mut pg_sys::ItemPointerData,
    );

    Ok(())
}

/// Get the current transaction start timestamp.
///
/// This should be used for all created_at/updated_at fields to ensure
/// consistency within a transaction - all operations in the same transaction
/// will have the same timestamp.
///
/// # Returns
/// The transaction start timestamp as a PostgreSQL TimestampTz.
#[inline]
pub fn current_timestamp() -> pg_sys::TimestampTz {
    unsafe { pg_sys::GetCurrentTransactionStartTimestamp() }
}

/// Convert a PostgreSQL TimestampTz to a pgrx TimestampWithTimeZone.
///
/// # Arguments
/// * `ts` - The PostgreSQL timestamp
///
/// # Returns
/// A pgrx TimestampWithTimeZone that can be used with IntoDatum.
#[inline]
pub fn timestamp_to_pgrx(ts: pg_sys::TimestampTz) -> CaliberResult<TimestampWithTimeZone> {
    // In pgrx 0.16+, use TryFrom to create from raw TimestampTz
    TimestampWithTimeZone::try_from(ts).map_err(|e| {
        StorageError::TransactionFailed {
            reason: format!("Failed to convert timestamp: {}", e),
        }
        .into()
    })
}

/// Get the current snapshot for visibility checks.
///
/// # Returns
/// The active snapshot pointer.
#[inline]
pub fn get_active_snapshot() -> *mut pg_sys::SnapshotData {
    unsafe { pg_sys::GetActiveSnapshot() }
}

/// Check if a snapshot is valid (not null).
#[inline]
pub fn snapshot_is_valid(snapshot: *mut pg_sys::SnapshotData) -> bool {
    !snapshot.is_null()
}

/// Begin a heap scan on a relation.
///
/// # Arguments
/// * `rel` - The relation to scan
/// * `snapshot` - The snapshot for visibility (use get_active_snapshot())
/// * `nkeys` - Number of scan keys (0 for full scan)
/// * `keys` - Scan keys (can be null if nkeys is 0)
///
/// # Returns
/// A heap scan descriptor that must be ended with `end_heap_scan`.
///
/// # Safety
/// The snapshot and keys pointers must be valid for the duration of the scan.
pub unsafe fn begin_heap_scan(
    rel: &HeapRelation,
    snapshot: *mut pg_sys::SnapshotData,
    nkeys: i32,
    keys: *mut pg_sys::ScanKeyData,
) -> pg_sys::TableScanDesc {
    pg_sys::heap_beginscan(
        rel.inner.as_ptr(),
        snapshot,
        nkeys,
        keys,
        std::ptr::null_mut(), // parallel_scan
        0,                     // flags
    )
}

/// Get the next tuple from a heap scan.
///
/// # Arguments
/// * `scan` - The heap scan descriptor
/// * `direction` - Scan direction (ForwardScanDirection or BackwardScanDirection)
///
/// # Returns
/// The next tuple, or null if no more tuples.
///
/// # Safety
/// The scan descriptor must be valid and created by begin_heap_scan.
pub unsafe fn heap_getnext(
    scan: pg_sys::TableScanDesc,
    direction: pg_sys::ScanDirection::Type,
) -> pg_sys::HeapTuple {
    pg_sys::heap_getnext(scan, direction)
}

/// End a heap scan and release resources.
///
/// # Arguments
/// * `scan` - The heap scan descriptor to end
///
/// # Safety
/// The scan descriptor must be valid and created by begin_heap_scan.
pub unsafe fn end_heap_scan(scan: pg_sys::TableScanDesc) {
    if !scan.is_null() {
        pg_sys::heap_endscan(scan);
    }
}

/// A RAII wrapper for heap scans that ensures cleanup.
pub struct HeapScanner {
    scan: pg_sys::TableScanDesc,
}

impl HeapScanner {
    /// Create a new heap scanner.
    ///
    /// # Safety
    /// The snapshot and keys pointers must be valid for the duration of the scan.
    pub unsafe fn new(
        rel: &HeapRelation,
        snapshot: *mut pg_sys::SnapshotData,
        nkeys: i32,
        keys: *mut pg_sys::ScanKeyData,
    ) -> Self {
        let scan = unsafe { begin_heap_scan(rel, snapshot, nkeys, keys) };
        Self { scan }
    }

    /// Get the raw scan descriptor (for advanced use).
    pub fn as_ptr(&self) -> pg_sys::TableScanDesc {
        self.scan
    }
}

impl Iterator for HeapScanner {
    type Item = pg_sys::HeapTuple;

    fn next(&mut self) -> Option<Self::Item> {
        let tuple = unsafe { heap_getnext(self.scan, pg_sys::ScanDirection::ForwardScanDirection) };
        if tuple.is_null() {
            None
        } else {
            Some(tuple)
        }
    }
}

impl Drop for HeapScanner {
    fn drop(&mut self) {
        unsafe {
            end_heap_scan(self.scan);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lock_mode_conversion() {
        assert_eq!(
            PgLockMode::AccessShare.to_pg_lockmode(),
            pg_sys::AccessShareLock as pg_sys::LOCKMODE
        );
        assert_eq!(
            PgLockMode::RowExclusive.to_pg_lockmode(),
            pg_sys::RowExclusiveLock as pg_sys::LOCKMODE
        );
    }
}
