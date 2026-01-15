# Design Document: CALIBER-PG Hot Path Migration

## Overview

This spec migrates `caliber-pg` from SPI-based SQL operations to direct heap operations using pgrx. The goal is to eliminate SQL parsing overhead in hot-path entity CRUD operations while maintaining correctness and PostgreSQL transaction semantics.

## Architecture

### Before (Current State)

```text
┌─────────────────────────────────────────────────────────────────────┐
│                      caliber-pg pg_extern functions                 │
├─────────────────────────────────────────────────────────────────────┤
│  caliber_trajectory_create()                                        │
│           │                                                         │
│           ▼                                                         │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │                    Spi::connect()                            │   │
│  │  client.update("INSERT INTO caliber_trajectory ...", ...)   │   │
│  └─────────────────────────────────────────────────────────────┘   │
│           │                                                         │
│           ▼                                                         │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │              PostgreSQL SQL Parser (gram.y)                  │   │
│  │                    ~15K lines of C                           │   │
│  └─────────────────────────────────────────────────────────────┘   │
│           │                                                         │
│           ▼                                                         │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │              Query Planner / Optimizer                       │   │
│  └─────────────────────────────────────────────────────────────┘   │
│           │                                                         │
│           ▼                                                         │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │                    Executor                                  │   │
│  └─────────────────────────────────────────────────────────────┘   │
│           │                                                         │
│           ▼                                                         │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │              Heap Storage + Indexes                          │   │
│  └─────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────┘
```

### After (Target State)

```text
┌─────────────────────────────────────────────────────────────────────┐
│                      caliber-pg pg_extern functions                 │
├─────────────────────────────────────────────────────────────────────┤
│  caliber_trajectory_create()                                        │
│           │                                                         │
│           ▼                                                         │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │              Direct Heap Operations (pgrx)                   │   │
│  │  - PgRelation::open_with_name("caliber_trajectory")         │   │
│  │  - heap_form_tuple(tuple_desc, values, nulls)               │   │
│  │  - simple_heap_insert(rel, heap_tuple)                      │   │
│  │  - CatalogIndexInsert(index_info, heap_tuple)               │   │
│  └─────────────────────────────────────────────────────────────┘   │
│           │                                                         │
│           ▼  (DIRECT - NO SQL PARSING)                             │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │              Heap Storage + Indexes                          │   │
│  └─────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────┘
```

## Components and Interfaces

### Core Helper Module: `heap_ops.rs`

A new module providing safe wrappers around pgrx direct heap operations:

```rust
//! Direct heap operation helpers for hot-path performance.
//! These bypass SQL parsing entirely.

use pgrx::prelude::*;
use pgrx::pg_sys::{
    self, Datum, HeapTuple, Relation, TupleDesc,
    heap_form_tuple, simple_heap_insert, simple_heap_update,
    simple_heap_delete, index_insert, GetCurrentTransactionStartTimestamp,
};

/// Open a relation by name with appropriate lock mode.
pub fn open_relation(name: &str, lock_mode: pg_sys::LOCKMODE) -> PgRelation {
    unsafe {
        PgRelation::open_with_name_and_share_lock(name)
    }
}

/// Form a heap tuple from values and nulls arrays.
pub fn form_tuple(
    tuple_desc: TupleDesc,
    values: &[Datum],
    nulls: &[bool],
) -> HeapTuple {
    unsafe {
        heap_form_tuple(
            tuple_desc,
            values.as_ptr() as *mut Datum,
            nulls.as_ptr() as *mut bool,
        )
    }
}

/// Insert a tuple into a relation and update indexes.
pub fn insert_tuple(
    rel: &PgRelation,
    tuple: HeapTuple,
) -> pg_sys::ItemPointerData {
    unsafe {
        simple_heap_insert(rel.as_ptr(), tuple)
    }
}

/// Update a tuple in place.
pub fn update_tuple(
    rel: &PgRelation,
    otid: pg_sys::ItemPointerData,
    tuple: HeapTuple,
) {
    unsafe {
        simple_heap_update(rel.as_ptr(), &otid, tuple);
    }
}

/// Delete a tuple.
pub fn delete_tuple(
    rel: &PgRelation,
    tid: pg_sys::ItemPointerData,
) {
    unsafe {
        simple_heap_delete(rel.as_ptr(), &tid);
    }
}

/// Get current transaction timestamp for created_at/updated_at.
pub fn current_timestamp() -> pg_sys::TimestampTz {
    unsafe {
        GetCurrentTransactionStartTimestamp()
    }
}
```

### Index Operations Module: `index_ops.rs`

```rust
//! Index operation helpers for maintaining indexes during direct heap ops.

use pgrx::prelude::*;
use pgrx::pg_sys::{
    self, IndexInfo, Relation, HeapTuple,
    CatalogIndexInsert, index_beginscan, index_endscan,
    index_getnext_tid, index_fetch_heap,
};

/// Insert into all indexes for a relation.
pub fn catalog_index_insert(
    rel: &PgRelation,
    tuple: HeapTuple,
    values: &[pg_sys::Datum],
    nulls: &[bool],
) {
    // Get index info and insert into each index
    unsafe {
        let index_list = pg_sys::RelationGetIndexList(rel.as_ptr());
        // Iterate and insert into each index
        // ...
    }
}

/// Scan an index for a specific key value.
pub struct IndexScanner {
    scan: *mut pg_sys::IndexScanDescData,
    rel: PgRelation,
}

impl IndexScanner {
    pub fn new(
        rel: &PgRelation,
        index_name: &str,
        key: pg_sys::ScanKey,
        nkeys: i32,
    ) -> Self {
        unsafe {
            let index_rel = PgRelation::open_with_name(index_name);
            let scan = index_beginscan(
                rel.as_ptr(),
                index_rel.as_ptr(),
                pg_sys::GetActiveSnapshot(),
                nkeys,
                0,
            );
            Self { scan, rel: index_rel }
        }
    }

    pub fn next(&mut self) -> Option<HeapTuple> {
        unsafe {
            let tid = index_getnext_tid(self.scan, pg_sys::ForwardScanDirection);
            if tid.is_null() {
                return None;
            }
            Some(index_fetch_heap(self.scan))
        }
    }
}

impl Drop for IndexScanner {
    fn drop(&mut self) {
        unsafe {
            index_endscan(self.scan);
        }
    }
}
```

### Entity-Specific Modules

Each entity type gets a dedicated module with direct heap operations:

#### `trajectory_heap.rs`

```rust
use crate::heap_ops::*;
use crate::index_ops::*;
use caliber_core::{EntityId, Trajectory, TrajectoryStatus, CaliberResult, StorageError};

const TRAJECTORY_TABLE: &str = "caliber_trajectory";
const TRAJECTORY_PK_INDEX: &str = "caliber_trajectory_pkey";
const TRAJECTORY_STATUS_INDEX: &str = "caliber_trajectory_status_idx";

/// Column offsets for caliber_trajectory table
mod cols {
    pub const TRAJECTORY_ID: usize = 0;
    pub const NAME: usize = 1;
    pub const DESCRIPTION: usize = 2;
    pub const STATUS: usize = 3;
    pub const PARENT_TRAJECTORY_ID: usize = 4;
    pub const ROOT_TRAJECTORY_ID: usize = 5;
    pub const AGENT_ID: usize = 6;
    pub const CREATED_AT: usize = 7;
    pub const UPDATED_AT: usize = 8;
    pub const COMPLETED_AT: usize = 9;
    pub const OUTCOME: usize = 10;
    pub const METADATA: usize = 11;
    pub const NUM_COLS: usize = 12;
}

pub fn trajectory_create_heap(
    trajectory_id: EntityId,
    name: &str,
    description: Option<&str>,
    agent_id: Option<EntityId>,
) -> CaliberResult<EntityId> {
    let rel = open_relation(TRAJECTORY_TABLE, pg_sys::RowExclusiveLock as i32);
    let tuple_desc = rel.tuple_desc();
    
    let mut values: [pg_sys::Datum; cols::NUM_COLS] = [0; cols::NUM_COLS];
    let mut nulls: [bool; cols::NUM_COLS] = [false; cols::NUM_COLS];
    
    // Set values
    values[cols::TRAJECTORY_ID] = trajectory_id.into_datum().unwrap();
    values[cols::NAME] = name.into_datum().unwrap();
    
    if let Some(desc) = description {
        values[cols::DESCRIPTION] = desc.into_datum().unwrap();
    } else {
        nulls[cols::DESCRIPTION] = true;
    }
    
    values[cols::STATUS] = "active".into_datum().unwrap();
    
    if let Some(aid) = agent_id {
        values[cols::AGENT_ID] = aid.into_datum().unwrap();
    } else {
        nulls[cols::AGENT_ID] = true;
    }
    
    let now = current_timestamp();
    values[cols::CREATED_AT] = now.into_datum().unwrap();
    values[cols::UPDATED_AT] = now.into_datum().unwrap();
    
    // Null optional fields
    nulls[cols::PARENT_TRAJECTORY_ID] = true;
    nulls[cols::ROOT_TRAJECTORY_ID] = true;
    nulls[cols::COMPLETED_AT] = true;
    nulls[cols::OUTCOME] = true;
    nulls[cols::METADATA] = true;
    
    // Form and insert tuple
    let tuple = form_tuple(tuple_desc, &values, &nulls);
    let tid = insert_tuple(&rel, tuple);
    
    // Update indexes
    catalog_index_insert(&rel, tuple, &values, &nulls);
    
    Ok(trajectory_id)
}

pub fn trajectory_get_heap(id: EntityId) -> CaliberResult<Option<Trajectory>> {
    let rel = open_relation(TRAJECTORY_TABLE, pg_sys::AccessShareLock as i32);
    
    // Build scan key for primary key lookup
    let mut scan_key = pg_sys::ScanKeyData::default();
    unsafe {
        pg_sys::ScanKeyInit(
            &mut scan_key,
            1, // First column (trajectory_id)
            pg_sys::BTEqualStrategyNumber as u16,
            pg_sys::F_UUID_EQ,
            id.into_datum().unwrap(),
        );
    }
    
    let mut scanner = IndexScanner::new(&rel, TRAJECTORY_PK_INDEX, &scan_key, 1);
    
    if let Some(tuple) = scanner.next() {
        // Extract values from tuple and build Trajectory
        Ok(Some(tuple_to_trajectory(tuple, rel.tuple_desc())))
    } else {
        Ok(None)
    }
}

pub fn trajectory_update_status_heap(
    id: EntityId,
    status: TrajectoryStatus,
) -> CaliberResult<bool> {
    let rel = open_relation(TRAJECTORY_TABLE, pg_sys::RowExclusiveLock as i32);
    
    // Find existing tuple via index scan
    let mut scan_key = pg_sys::ScanKeyData::default();
    unsafe {
        pg_sys::ScanKeyInit(
            &mut scan_key,
            1,
            pg_sys::BTEqualStrategyNumber as u16,
            pg_sys::F_UUID_EQ,
            id.into_datum().unwrap(),
        );
    }
    
    let mut scanner = IndexScanner::new(&rel, TRAJECTORY_PK_INDEX, &scan_key, 1);
    
    if let Some(old_tuple) = scanner.next() {
        // Build new tuple with updated status
        let tuple_desc = rel.tuple_desc();
        let mut values = extract_values(old_tuple, tuple_desc);
        let mut nulls = extract_nulls(old_tuple, tuple_desc);
        
        values[cols::STATUS] = status_to_str(status).into_datum().unwrap();
        values[cols::UPDATED_AT] = current_timestamp().into_datum().unwrap();
        
        let new_tuple = form_tuple(tuple_desc, &values, &nulls);
        let old_tid = unsafe { (*old_tuple).t_self };
        
        update_tuple(&rel, old_tid, new_tuple);
        
        Ok(true)
    } else {
        Ok(false)
    }
}

pub fn trajectory_list_by_status_heap(
    status: TrajectoryStatus,
) -> CaliberResult<Vec<Trajectory>> {
    let rel = open_relation(TRAJECTORY_TABLE, pg_sys::AccessShareLock as i32);
    
    // Build scan key for status index
    let mut scan_key = pg_sys::ScanKeyData::default();
    unsafe {
        pg_sys::ScanKeyInit(
            &mut scan_key,
            1,
            pg_sys::BTEqualStrategyNumber as u16,
            pg_sys::F_TEXTEQ,
            status_to_str(status).into_datum().unwrap(),
        );
    }
    
    let mut scanner = IndexScanner::new(&rel, TRAJECTORY_STATUS_INDEX, &scan_key, 1);
    let mut results = Vec::new();
    
    while let Some(tuple) = scanner.next() {
        results.push(tuple_to_trajectory(tuple, rel.tuple_desc()));
    }
    
    Ok(results)
}

fn status_to_str(status: TrajectoryStatus) -> &'static str {
    match status {
        TrajectoryStatus::Active => "active",
        TrajectoryStatus::Completed => "completed",
        TrajectoryStatus::Failed => "failed",
        TrajectoryStatus::Suspended => "suspended",
    }
}

fn tuple_to_trajectory(tuple: HeapTuple, desc: TupleDesc) -> Trajectory {
    // Extract each field from the heap tuple
    // ... implementation details
    todo!()
}

fn extract_values(tuple: HeapTuple, desc: TupleDesc) -> [pg_sys::Datum; cols::NUM_COLS] {
    // Extract all datum values from tuple
    todo!()
}

fn extract_nulls(tuple: HeapTuple, desc: TupleDesc) -> [bool; cols::NUM_COLS] {
    // Extract null flags from tuple
    todo!()
}
```

## Data Models

### Relation Metadata Cache

To avoid repeated catalog lookups, cache relation metadata:

```rust
use once_cell::sync::Lazy;
use std::sync::RwLock;
use std::collections::HashMap;

/// Cached relation metadata for hot-path operations.
pub struct RelationMeta {
    pub oid: pg_sys::Oid,
    pub tuple_desc: TupleDesc,
    pub index_oids: Vec<pg_sys::Oid>,
}

static RELATION_CACHE: Lazy<RwLock<HashMap<String, RelationMeta>>> = 
    Lazy::new(|| RwLock::new(HashMap::new()));

pub fn get_relation_meta(name: &str) -> RelationMeta {
    // Check cache first
    if let Some(meta) = RELATION_CACHE.read().unwrap().get(name) {
        return meta.clone();
    }
    
    // Load from catalog and cache
    let meta = load_relation_meta(name);
    RELATION_CACHE.write().unwrap().insert(name.to_string(), meta.clone());
    meta
}
```

### Column Mapping Constants

Each entity table has a constants module defining column positions:

```rust
// trajectory_cols.rs
pub mod trajectory {
    pub const TRAJECTORY_ID: i16 = 1;
    pub const NAME: i16 = 2;
    pub const DESCRIPTION: i16 = 3;
    pub const STATUS: i16 = 4;
    pub const PARENT_TRAJECTORY_ID: i16 = 5;
    pub const ROOT_TRAJECTORY_ID: i16 = 6;
    pub const AGENT_ID: i16 = 7;
    pub const CREATED_AT: i16 = 8;
    pub const UPDATED_AT: i16 = 9;
    pub const COMPLETED_AT: i16 = 10;
    pub const OUTCOME: i16 = 11;
    pub const METADATA: i16 = 12;
}

// scope_cols.rs
pub mod scope {
    pub const SCOPE_ID: i16 = 1;
    pub const TRAJECTORY_ID: i16 = 2;
    pub const PARENT_SCOPE_ID: i16 = 3;
    pub const NAME: i16 = 4;
    pub const PURPOSE: i16 = 5;
    pub const IS_ACTIVE: i16 = 6;
    pub const CREATED_AT: i16 = 7;
    pub const CLOSED_AT: i16 = 8;
    pub const CHECKPOINT: i16 = 9;
    pub const TOKEN_BUDGET: i16 = 10;
    pub const TOKENS_USED: i16 = 11;
    pub const METADATA: i16 = 12;
}

// ... similar for artifact, note, turn, lock, message, agent, delegation, handoff, conflict
```

## Correctness Properties

*A property is a characteristic or behavior that should hold true across all valid executions of a system—essentially, a formal statement about what the system should do. Properties serve as the bridge between human-readable specifications and machine-verifiable correctness guarantees.*

### Property 1: Insert-Get Round Trip

*For any* valid entity (Trajectory, Scope, Artifact, Note, Turn, Agent, Lock, Message, Delegation, Handoff, Conflict), inserting via direct heap then getting via direct heap SHALL return an equivalent entity.

**Validates: Requirements 1.1, 1.2, 2.1, 2.2, 3.1, 3.2, 4.1, 4.2, 5.1, 6.1, 6.3, 7.1, 7.2, 8.1, 8.2, 9.1, 9.2, 10.1, 10.2, 11.1, 11.2**

### Property 2: Update Persistence

*For any* entity and valid update, applying the update via direct heap then getting the entity SHALL return the updated values.

**Validates: Requirements 1.3, 1.4, 2.3, 2.5, 3.5, 4.4, 6.2, 7.4, 8.3, 8.4, 9.3, 9.4, 10.3, 10.4, 11.3**

### Property 3: Index Consistency

*For any* entity inserted or updated via direct heap, querying via the corresponding index SHALL return that entity.

**Validates: Requirements 1.5, 1.6, 2.4, 3.3, 3.4, 4.3, 5.2, 6.4, 7.3, 8.5, 9.5, 13.1, 13.2, 13.4, 13.5**

### Property 4: Delete Removes from Index

*For any* entity deleted via direct heap, querying via any index SHALL NOT return that entity.

**Validates: Requirements 6.2, 13.3**

### Property 5: Transaction Visibility

*For any* sequence of operations within a transaction, subsequent operations SHALL see the results of prior operations in the same transaction.

**Validates: Requirements 15.4**

### Property 6: Timestamp Consistency

*For any* entity created or updated via direct heap, the created_at/updated_at timestamps SHALL be set to the current transaction timestamp.

**Validates: Requirements 15.1**

### Property 7: Error Propagation

*For any* storage operation that fails, the function SHALL return a CaliberResult::Err with appropriate StorageError variant, NOT panic.

**Validates: Requirements 14.1, 14.2, 14.3, 14.4, 14.5**

### Property 8: Not Found Returns None

*For any* get operation on a non-existent entity ID, the function SHALL return Ok(None), NOT an error.

**Validates: Requirements 1.7, 14.2**

## Error Handling

### Error Types

All direct heap operations use the existing `CaliberError` and `StorageError` types:

```rust
// From caliber-core
pub enum StorageError {
    NotFound { entity_type: EntityType, id: EntityId },
    InsertFailed { entity_type: EntityType, reason: String },
    UpdateFailed { entity_type: EntityType, id: EntityId, reason: String },
    TransactionFailed { reason: String },
    IndexError { index_name: String, reason: String },
}
```

### Error Conversion

```rust
impl From<pgrx::PgError> for CaliberError {
    fn from(e: pgrx::PgError) -> Self {
        CaliberError::Storage(StorageError::TransactionFailed {
            reason: e.to_string(),
        })
    }
}

/// Wrap unsafe heap operations with error handling
macro_rules! heap_try {
    ($expr:expr, $entity_type:expr, $reason:expr) => {
        match std::panic::catch_unwind(|| $expr) {
            Ok(result) => result,
            Err(_) => {
                return Err(CaliberError::Storage(StorageError::InsertFailed {
                    entity_type: $entity_type,
                    reason: $reason.to_string(),
                }));
            }
        }
    };
}
```

## Testing Strategy

### Unit Tests

Unit tests focus on specific examples and edge cases:

- Insert with all fields populated
- Insert with optional fields null
- Get existing entity
- Get non-existent entity (returns None)
- Update single field
- Update multiple fields
- Delete existing entity
- Delete non-existent entity
- Index scan with matching results
- Index scan with no results

### Property-Based Tests

Property-based tests verify universal properties across generated inputs:

- **Round Trip**: Generate random entities, insert, get, compare
- **Update Persistence**: Generate random updates, apply, verify
- **Index Consistency**: Insert entities, query via each index, verify found
- **Transaction Visibility**: Sequence of ops in transaction, verify visibility

### Integration Tests

- Full CRUD cycle for each entity type
- Concurrent operations (multiple connections)
- Transaction rollback behavior
- Index maintenance under load

### Test Configuration

- Property tests: minimum 100 iterations per property
- Use `proptest` for Rust property-based testing
- Tag format: `Feature: caliber-pg-hot-path, Property {N}: {description}`
