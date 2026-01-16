//! Index operation helpers for maintaining indexes during direct heap operations.
//!
//! When using direct heap operations (bypassing SQL), we must manually maintain
//! indexes. This module provides safe wrappers for:
//! - Inserting into indexes after heap insert
//! - Updating indexes after heap update
//! - Scanning indexes for lookups
//!
//! # Index Maintenance
//!
//! PostgreSQL requires that indexes be updated whenever the heap is modified:
//! - INSERT: Call `CatalogIndexInsert` for all indexes
//! - UPDATE: If indexed columns changed, update those indexes
//! - DELETE: Index entries are cleaned up by VACUUM (no immediate action needed)
//!
//! # Index Scanning
//!
//! For efficient lookups, use index scans instead of heap scans:
//! - Primary key lookups: Use btree index on ID column
//! - Status queries: Use btree index on status column
//! - Vector similarity: Use HNSW index on embedding column

// Prelude provides common pgrx types and traits used throughout
#[allow(unused_imports)]
use pgrx::prelude::*;
use pgrx::pg_sys;
use pgrx::PgRelation;

use caliber_core::{CaliberError, CaliberResult, StorageError};

// HeapRelation is used for type signatures in index operations
#[allow(unused_imports)]
use crate::heap_ops::HeapRelation;

/// Strategy numbers for btree index scans.
/// These correspond to PostgreSQL's BTxxxStrategyNumber constants.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BTreeStrategy {
    /// Less than (<)
    Less = 1,
    /// Less than or equal (<=)
    LessOrEqual = 2,
    /// Equal (=)
    Equal = 3,
    /// Greater than or equal (>=)
    GreaterOrEqual = 4,
    /// Greater than (>)
    Greater = 5,
}

impl BTreeStrategy {
    /// Convert to PostgreSQL strategy number.
    #[inline]
    pub fn to_pg_strategy(self) -> u16 {
        self as u16
    }
}

/// Common operator OIDs for scan key initialization.
/// These are the function OIDs for comparison operators.
pub mod operator_oids {
    use pgrx::pg_sys;

    /// UUID equality operator (uuid = uuid)
    pub const UUID_EQ: pg_sys::Oid = pg_sys::Oid::from_u32(2972);
    
    /// Text equality operator (text = text)
    pub const TEXT_EQ: pg_sys::Oid = pg_sys::Oid::from_u32(98);
    
    /// Int4 equality operator (int4 = int4)
    pub const INT4_EQ: pg_sys::Oid = pg_sys::Oid::from_u32(96);
    
    /// Int8 equality operator (int8 = int8)
    pub const INT8_EQ: pg_sys::Oid = pg_sys::Oid::from_u32(410);
    
    /// Boolean equality operator (bool = bool)
    pub const BOOL_EQ: pg_sys::Oid = pg_sys::Oid::from_u32(91);
    
    /// Timestamp with timezone equality
    pub const TIMESTAMPTZ_EQ: pg_sys::Oid = pg_sys::Oid::from_u32(1320);
}

/// Initialize a scan key for index scanning.
///
/// # Arguments
/// * `key` - Pointer to the ScanKeyData to initialize
/// * `attno` - Attribute number (1-based column index)
/// * `strategy` - Comparison strategy (Equal, Less, etc.)
/// * `operator_oid` - OID of the comparison operator function
/// * `argument` - The datum value to compare against
///
/// # Safety
/// The key pointer must be valid and properly aligned.
pub fn init_scan_key(
    key: &mut pg_sys::ScanKeyData,
    attno: i16,
    strategy: BTreeStrategy,
    operator_oid: pg_sys::Oid,
    argument: pg_sys::Datum,
) {
    unsafe {
        pg_sys::ScanKeyInit(
            key,
            attno,
            strategy.to_pg_strategy(),
            operator_oid,
            argument,
        );
    }
}

/// A wrapper around an index relation.
pub struct IndexRelation {
    inner: PgRelation,
}

impl IndexRelation {
    /// Get the underlying relation pointer.
    #[inline]
    pub fn as_ptr(&self) -> pg_sys::Relation {
        self.inner.as_ptr()
    }

    /// Get the index OID.
    #[inline]
    pub fn oid(&self) -> pg_sys::Oid {
        unsafe { (*self.inner.as_ptr()).rd_id }
    }
}

/// Open an index by name.
///
/// # Arguments
/// * `name` - The index name
///
/// # Returns
/// * `Ok(IndexRelation)` - The opened index
/// * `Err(CaliberError)` - If the index cannot be opened
pub fn open_index(name: &str) -> CaliberResult<IndexRelation> {
    // In pgrx 0.16+, this returns Result<PgRelation, &str>
    let rel = PgRelation::open_with_name_and_share_lock(name)
        .map_err(|e| CaliberError::Storage(StorageError::IndexError {
            index_name: name.to_string(),
            reason: format!("Failed to open index: {}", e),
        }))?;

    if rel.as_ptr().is_null() {
        return Err(CaliberError::Storage(StorageError::IndexError {
            index_name: name.to_string(),
            reason: "Failed to open index".to_string(),
        }));
    }

    Ok(IndexRelation { inner: rel })
}

/// Open an index by OID.
///
/// # Arguments
/// * `oid` - The index OID
///
/// # Returns
/// * `Ok(IndexRelation)` - The opened index
/// * `Err(CaliberError)` - If the index cannot be opened
pub fn open_index_by_oid(oid: pg_sys::Oid) -> CaliberResult<IndexRelation> {
    let rel = unsafe {
        PgRelation::with_lock(oid, pg_sys::AccessShareLock as pg_sys::LOCKMODE)
    };

    if rel.as_ptr().is_null() {
        return Err(CaliberError::Storage(StorageError::IndexError {
            index_name: format!("OID {}", oid.to_u32()),
            reason: "Failed to open index".to_string(),
        }));
    }

    Ok(IndexRelation { inner: rel })
}

/// Get the list of index OIDs for a relation.
///
/// # Arguments
/// * `rel` - The heap relation
///
/// # Returns
/// A vector of index OIDs.
pub fn get_index_list(rel: &HeapRelation) -> Vec<pg_sys::Oid> {
    let mut indexes = Vec::new();

    unsafe {
        let list = pg_sys::RelationGetIndexList(rel.as_ref().as_ptr());
        if list.is_null() {
            return indexes;
        }

        // In PostgreSQL 18 / pgrx 0.16, List uses elements array instead of head/next
        let len = (*list).length as usize;
        for i in 0..len {
            // list_nth_oid extracts OID at position i
            let oid = pg_sys::list_nth_oid(list, i as i32);
            indexes.push(oid);
        }

        // Free the list (but not the OIDs, they're just integers)
        pg_sys::list_free(list);
    }

    indexes
}

/// Insert into all indexes for a relation after a heap insert.
///
/// This is the critical function that maintains index consistency.
/// It MUST be called after every `simple_heap_insert`.
///
/// # Arguments
/// * `rel` - The heap relation
/// * `tuple` - The inserted tuple
/// * `values` - The datum values used to form the tuple
/// * `nulls` - The null flags
///
/// # Returns
/// * `Ok(())` - All indexes updated successfully
/// * `Err(CaliberError)` - If any index update fails
///
/// # Safety
/// The tuple pointer must be valid for the target relation. The relation must
/// be opened with an appropriate lock mode for writes.
pub unsafe fn update_indexes_for_insert(
    rel: &HeapRelation,
    tuple: *mut pg_sys::HeapTupleData,
    _values: &[pg_sys::Datum],  // Reserved for manual index updates if needed
    _nulls: &[bool],            // Reserved for manual index updates if needed
) -> CaliberResult<()> {
    if tuple.is_null() {
        return Err(CaliberError::Storage(StorageError::IndexError {
            index_name: "all".to_string(),
            reason: "Cannot update indexes for null tuple".to_string(),
        }));
    }

    // Get the index info structure
    let index_info = pg_sys::CatalogOpenIndexes(rel.as_ref().as_ptr());

    if !index_info.is_null() {
        // Insert into all indexes using the WithInfo variant
        pg_sys::CatalogTupleInsertWithInfo(
            rel.as_ref().as_ptr(),
            tuple,
            index_info,
        );

        // Close the index info
        pg_sys::CatalogCloseIndexes(index_info);
    }

    Ok(())
}

/// A RAII wrapper for index scans.
pub struct IndexScanner {
    scan: *mut pg_sys::IndexScanDescData,
    /// The heap relation - stored for potential future use in cleanup/revalidation.
    #[allow(dead_code)]
    heap_rel: pg_sys::Relation,
    slot: *mut pg_sys::TupleTableSlot,
}

impl IndexScanner {
    /// Create a new index scanner.
    ///
    /// # Arguments
    /// * `heap_rel` - The heap relation being scanned
    /// * `index_rel` - The index to use for scanning
    /// * `snapshot` - The snapshot for visibility
    /// * `nkeys` - Number of scan keys
    /// * `keys` - The scan keys
    ///
    /// # Returns
    /// A new IndexScanner ready for iteration.
    ///
    /// # Safety
    /// The snapshot and keys pointers must be valid for the duration of the scan.
    pub unsafe fn new(
        heap_rel: &HeapRelation,
        index_rel: &IndexRelation,
        snapshot: *mut pg_sys::SnapshotData,
        nkeys: i32,
        keys: *mut pg_sys::ScanKeyData,
    ) -> Self {
        // In PostgreSQL 18 / pgrx 0.16, index_beginscan takes 6 args:
        // (heapRelation, indexRelation, snapshot, instrument, nkeys, norderbys)
        let scan = pg_sys::index_beginscan(
            heap_rel.as_ref().as_ptr(),
            index_rel.as_ptr(),
            snapshot,
            std::ptr::null_mut(), // instrument (IndexScanInstrumentation*)
            nkeys,
            0, // norderbys
        );

        // Set the scan keys
        if nkeys > 0 && !keys.is_null() {
            pg_sys::index_rescan(scan, keys, nkeys, std::ptr::null_mut(), 0);
        }

        // Create a TupleTableSlot for fetching tuples
        // Create a virtual tuple table slot for the heap relation
        let tuple_desc = (*heap_rel.as_ref().as_ptr()).rd_att;
        let slot = pg_sys::MakeSingleTupleTableSlot(tuple_desc, &pg_sys::TTSOpsHeapTuple);

        Self {
            scan,
            heap_rel: heap_rel.as_ref().as_ptr(),
            slot,
        }
    }

    // next() provided by Iterator impl

    /// Get the TID of the current tuple (after calling next()).
    pub fn current_tid(&self) -> Option<pg_sys::ItemPointerData> {
        unsafe {
            if self.scan.is_null() {
                return None;
            }
            Some((*self.scan).xs_heaptid)
        }
    }

    /// Get the raw scan descriptor.
    pub fn as_ptr(&self) -> *mut pg_sys::IndexScanDescData {
        self.scan
    }
}

impl Iterator for IndexScanner {
    type Item = *mut pg_sys::HeapTupleData;

    fn next(&mut self) -> Option<Self::Item> {
        unsafe {
            // Get the next TID from the index
            let tid = pg_sys::index_getnext_tid(
                self.scan,
                pg_sys::ScanDirection::ForwardScanDirection,
            );

            if tid.is_null() {
                return None;
            }

            // In PostgreSQL 18 / pgrx 0.16, index_fetch_heap takes 2 args and returns bool
            let found = pg_sys::index_fetch_heap(self.scan, self.slot);

            if !found {
                None
            } else {
                // Extract the HeapTuple from the slot
                // The slot contains the tuple after a successful fetch
                let tuple = pg_sys::ExecFetchSlotHeapTuple(self.slot, false, std::ptr::null_mut());
                if tuple.is_null() {
                    None
                } else {
                    Some(tuple)
                }
            }
        }
    }
}

impl Drop for IndexScanner {
    fn drop(&mut self) {
        unsafe {
            if !self.scan.is_null() {
                pg_sys::index_endscan(self.scan);
            }
            if !self.slot.is_null() {
                pg_sys::ExecDropSingleTupleTableSlot(self.slot);
            }
        }
    }
}

/// Perform a single-tuple index lookup by primary key.
///
/// This is optimized for the common case of looking up a single row by ID.
///
/// # Arguments
/// * `heap_rel` - The heap relation
/// * `index_rel` - The primary key index
/// * `id_datum` - The ID value to look up
///
/// # Returns
/// * `Some((tuple, tid))` - The found tuple and its TID
/// * `None` - No matching tuple found
pub fn index_lookup_single(
    heap_rel: &HeapRelation,
    index_rel: &IndexRelation,
    id_datum: pg_sys::Datum,
) -> Option<(*mut pg_sys::HeapTupleData, pg_sys::ItemPointerData)> {
    let snapshot = crate::heap_ops::get_active_snapshot();
    
    // Initialize scan key for equality on first column
    let mut scan_key = pg_sys::ScanKeyData::default();
    init_scan_key(
        &mut scan_key,
        1, // First column (the ID)
        BTreeStrategy::Equal,
        operator_oids::UUID_EQ,
        id_datum,
    );

    let mut scanner = unsafe { IndexScanner::new(
        heap_rel,
        index_rel,
        snapshot,
        1,
        &mut scan_key,
    ) };

    if let Some(tuple) = scanner.next() {
        let tid = scanner.current_tid()?;
        Some((tuple, tid))
    } else {
        None
    }
}

/// Collect all tuples matching an index scan into a vector.
///
/// # Arguments
/// * `heap_rel` - The heap relation
/// * `index_rel` - The index to scan
/// * `nkeys` - Number of scan keys
/// * `keys` - The scan keys
///
/// # Returns
/// A vector of (tuple, tid) pairs for all matching rows.
///
/// # Note
/// The returned tuples are only valid within the current transaction.
///
/// # Safety
/// The keys pointer must be valid for the duration of the scan.
pub unsafe fn index_scan_collect(
    heap_rel: &HeapRelation,
    index_rel: &IndexRelation,
    nkeys: i32,
    keys: *mut pg_sys::ScanKeyData,
) -> Vec<(*mut pg_sys::HeapTupleData, pg_sys::ItemPointerData)> {
    let snapshot = crate::heap_ops::get_active_snapshot();
    let mut scanner = unsafe { IndexScanner::new(heap_rel, index_rel, snapshot, nkeys, keys) };
    let mut results = Vec::new();

    while let Some(tuple) = scanner.next() {
        if let Some(tid) = scanner.current_tid() {
            results.push((tuple, tid));
        }
    }

    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_btree_strategy_conversion() {
        assert_eq!(BTreeStrategy::Equal.to_pg_strategy(), 3);
        assert_eq!(BTreeStrategy::Less.to_pg_strategy(), 1);
        assert_eq!(BTreeStrategy::Greater.to_pg_strategy(), 5);
    }
}
