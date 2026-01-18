//! Direct heap operations for Note entities.
//!
//! This module provides hot-path operations for notes that bypass SQL
//! parsing entirely by using direct heap access via pgrx.
//!
//! # Operations
//!
//! - `note_create_heap` - Insert a new note
//! - `note_get_heap` - Get a note by ID
//! - `note_query_by_trajectory_heap` - Query notes by source trajectory
//! - `note_update_heap` - Update note fields

use pgrx::prelude::*;
use pgrx::pg_sys;

use caliber_core::{
    AbstractionLevel, CaliberError, CaliberResult, ContentHash, EmbeddingVector, EntityId,
    EntityType, Note, NoteType, StorageError, TTL,
};

use crate::column_maps::note;
use crate::heap_ops::{
    current_timestamp, form_tuple, insert_tuple, open_relation,
    update_tuple, LockMode, HeapRelation, get_active_snapshot,
    timestamp_to_pgrx,
};
use crate::index_ops::{
    init_scan_key, open_index, update_indexes_for_insert,
    BTreeStrategy, IndexScanner, operator_oids,
};
use crate::tuple_extract::{
    extract_uuid, extract_text, extract_timestamp, extract_jsonb,
    extract_float4_array, extract_i32, extract_uuid_array,
    extract_values_and_nulls, uuid_to_datum, string_to_datum,
    json_to_datum, float4_array_to_datum, i32_to_datum,
    uuid_array_to_datum, timestamp_to_chrono, content_hash_to_datum,
    extract_content_hash,
};

/// Note row with tenant ownership metadata.
pub struct NoteRow {
    pub note: Note,
    pub tenant_id: Option<EntityId>,
}

impl From<NoteRow> for Note {
    fn from(row: NoteRow) -> Self {
        row.note
    }
}

/// Create a new note using direct heap operations.
///
/// # Arguments
/// * `note_id` - The pre-generated UUIDv7 for this note
/// * `note_type` - The type of note
/// * `title` - The note title
/// * `content` - The note content
/// * `content_hash` - SHA-256 hash of content
/// * `embedding` - Optional embedding vector
/// * `source_trajectory_ids` - Source trajectory IDs
/// * `source_artifact_ids` - Source artifact IDs
/// * `ttl` - Time-to-live setting
/// * `abstraction_level` - Semantic tier (Raw/Summary/Principle) - Battle Intel Feature 2
/// * `source_note_ids` - Notes this was derived from (for L1/L2) - Battle Intel Feature 2
///
/// # Returns
/// * `Ok(EntityId)` - The note ID on success
/// * `Err(CaliberError)` - On failure
///
/// # Requirements
/// - 4.1: Uses heap_form_tuple and simple_heap_insert instead of SPI
/// - 4.5: Updates btree and hnsw indexes via CatalogIndexInsert
pub struct NoteCreateParams<'a> {
    pub note_id: EntityId,
    pub note_type: NoteType,
    pub title: &'a str,
    pub content: &'a str,
    pub content_hash: ContentHash,
    pub embedding: Option<&'a EmbeddingVector>,
    pub source_trajectory_ids: &'a [EntityId],
    pub source_artifact_ids: &'a [EntityId],
    pub ttl: TTL,
    pub abstraction_level: AbstractionLevel,
    pub source_note_ids: &'a [EntityId],
    pub tenant_id: EntityId,
}

pub fn note_create_heap(params: NoteCreateParams<'_>) -> CaliberResult<EntityId> {
    let NoteCreateParams {
        note_id,
        note_type,
        title,
        content,
        content_hash,
        embedding,
        source_trajectory_ids,
        source_artifact_ids,
        ttl,
        abstraction_level,
        source_note_ids,
        tenant_id,
    } = params;
    // Open relation with RowExclusive lock for writes
    let rel = open_relation(note::TABLE_NAME, LockMode::RowExclusive)?;
    validate_note_relation(&rel)?;

    // Get current transaction timestamp
    let now = current_timestamp();
    let now_datum = timestamp_to_pgrx(now).into_datum()
        .ok_or_else(|| CaliberError::Storage(StorageError::InsertFailed {
            entity_type: EntityType::Note,
            reason: "Failed to convert timestamp to datum".to_string(),
        }))?;
    
    // Build datum array - must match column order in caliber_note table
    let mut values: [pg_sys::Datum; note::NUM_COLS] = [pg_sys::Datum::from(0); note::NUM_COLS];
    let mut nulls: [bool; note::NUM_COLS] = [false; note::NUM_COLS];
    
    // Column 1: note_id (UUID, NOT NULL)
    values[note::NOTE_ID as usize - 1] = uuid_to_datum(note_id);
    
    // Column 2: note_type (TEXT, NOT NULL)
    values[note::NOTE_TYPE as usize - 1] = string_to_datum(note_type_to_str(note_type));
    
    // Column 3: title (TEXT, NOT NULL)
    values[note::TITLE as usize - 1] = string_to_datum(title);
    
    // Column 4: content (TEXT, NOT NULL)
    values[note::CONTENT as usize - 1] = string_to_datum(content);
    
    // Column 5: content_hash (BYTEA, NOT NULL)
    values[note::CONTENT_HASH as usize - 1] = content_hash_to_datum(&content_hash);
    
    // Column 6: embedding (VECTOR, nullable)
    if let Some(emb) = embedding {
        values[note::EMBEDDING as usize - 1] = float4_array_to_datum(&emb.data);
    } else {
        nulls[note::EMBEDDING as usize - 1] = true;
    }
    
    // Column 7: source_trajectory_ids (UUID[], nullable)
    if !source_trajectory_ids.is_empty() {
        values[note::SOURCE_TRAJECTORY_IDS as usize - 1] = uuid_array_to_datum(source_trajectory_ids);
    } else {
        nulls[note::SOURCE_TRAJECTORY_IDS as usize - 1] = true;
    }
    
    // Column 8: source_artifact_ids (UUID[], nullable)
    if !source_artifact_ids.is_empty() {
        values[note::SOURCE_ARTIFACT_IDS as usize - 1] = uuid_array_to_datum(source_artifact_ids);
    } else {
        nulls[note::SOURCE_ARTIFACT_IDS as usize - 1] = true;
    }
    
    // Column 9: ttl (TEXT, NOT NULL)
    values[note::TTL as usize - 1] = string_to_datum(ttl_to_str(ttl));
    
    // Column 10: created_at (TIMESTAMPTZ, NOT NULL)
    values[note::CREATED_AT as usize - 1] = now_datum;
    
    // Column 11: updated_at (TIMESTAMPTZ, NOT NULL)
    values[note::UPDATED_AT as usize - 1] = now_datum;
    
    // Column 12: accessed_at (TIMESTAMPTZ, NOT NULL)
    values[note::ACCESSED_AT as usize - 1] = now_datum;
    
    // Column 13: access_count (INTEGER, NOT NULL) - default to 0
    values[note::ACCESS_COUNT as usize - 1] = i32_to_datum(0);
    
    // Column 14: superseded_by (UUID, nullable)
    nulls[note::SUPERSEDED_BY as usize - 1] = true;

    // Column 15: metadata (JSONB, nullable)
    nulls[note::METADATA as usize - 1] = true;

    // Column 16: abstraction_level (TEXT, NOT NULL) - Battle Intel Feature 2
    values[note::ABSTRACTION_LEVEL as usize - 1] = string_to_datum(abstraction_level_to_str(abstraction_level));

    // Column 17: source_note_ids (UUID[], nullable) - Battle Intel Feature 2
    if !source_note_ids.is_empty() {
        values[note::SOURCE_NOTE_IDS as usize - 1] = uuid_array_to_datum(source_note_ids);
    } else {
        nulls[note::SOURCE_NOTE_IDS as usize - 1] = true;
    }

    // Column 18: tenant_id (UUID, NOT NULL)
    values[note::TENANT_ID as usize - 1] = uuid_to_datum(tenant_id);

    // Form the heap tuple
    let tuple = form_tuple(&rel, &values, &nulls)?;
    
    // Insert into heap
    let _tid = unsafe { insert_tuple(&rel, tuple)? };
    
    // Update all indexes via CatalogIndexInsert
    unsafe { update_indexes_for_insert(&rel, tuple, &values, &nulls)? };
    
    Ok(note_id)
}


/// Get a note by ID using direct heap operations.
///
/// # Arguments
/// * `id` - The note ID to look up
///
/// # Returns
/// * `Ok(Some(Note))` - The note if found
/// * `Ok(None)` - If no note with that ID exists
/// * `Err(CaliberError)` - On failure
///
/// # Requirements
/// - 4.2: Uses index_beginscan for O(log n) lookup instead of SPI SELECT
pub fn note_get_heap(id: EntityId, tenant_id: EntityId) -> CaliberResult<Option<NoteRow>> {
    // Open relation with AccessShare lock for reads
    let rel = open_relation(note::TABLE_NAME, LockMode::AccessShare)?;
    
    // Open the primary key index
    let index_rel = open_index(note::PK_INDEX)?;
    
    // Get active snapshot for visibility
    let snapshot = get_active_snapshot();
    
    // Build scan key for primary key lookup
    let mut scan_key = pg_sys::ScanKeyData::default();
    init_scan_key(
        &mut scan_key,
        1, // First column of index (note_id)
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
        let row = unsafe { tuple_to_note(tuple, tuple_desc) }?;
        if row.tenant_id == Some(tenant_id) {
            Ok(Some(row))
        } else {
            Ok(None)
        }
    } else {
        Ok(None)
    }
}

/// Query notes by source trajectory using direct heap operations.
///
/// Note: This uses a GIN index on the source_trajectory_ids array.
/// For simplicity, we do a sequential scan with filtering since GIN
/// array containment queries are complex to set up via direct heap.
///
/// # Arguments
/// * `trajectory_id` - The trajectory ID to search for in source_trajectory_ids
///
/// # Returns
/// * `Ok(Vec<Note>)` - List of matching notes
/// * `Err(CaliberError)` - On failure
///
/// # Requirements
/// - 4.3: Uses index scan instead of SPI SELECT
pub fn note_query_by_trajectory_heap(
    trajectory_id: EntityId,
    tenant_id: EntityId,
) -> CaliberResult<Vec<NoteRow>> {
    // Open relation with AccessShare lock for reads
    let rel = open_relation(note::TABLE_NAME, LockMode::AccessShare)?;
    
    // Get active snapshot for visibility
    let snapshot = get_active_snapshot();
    
    // For array containment queries, we need to do a heap scan and filter
    // A proper implementation would use the GIN index, but that requires
    // more complex scan key setup with array operators
    let mut scanner = unsafe { crate::heap_ops::HeapScanner::new(
        &rel,
        snapshot,
        0,
        std::ptr::null_mut(),
    ) };
    
    let tuple_desc = rel.tuple_desc();
    let mut results = Vec::new();
    
    // Scan all tuples and filter by source_trajectory_ids containing trajectory_id
    for tuple in &mut scanner {
        let source_ids = unsafe { extract_uuid_array(tuple, tuple_desc, note::SOURCE_TRAJECTORY_IDS) }?;

        if let Some(ids) = source_ids {
            if ids.contains(&trajectory_id) {
                let row = unsafe { tuple_to_note(tuple, tuple_desc) }?;
                // Enforce TTL - skip expired notes
                if row.tenant_id == Some(tenant_id)
                    && !is_note_expired(&row.note.ttl, row.note.created_at)
                {
                    results.push(row);
                }
            }
        }
    }

    Ok(results)
}

/// Update a note using direct heap operations.
///
/// # Arguments
/// * `id` - The note ID to update
/// * `content` - Optional new content
/// * `content_hash` - Optional new content hash (required if content changes)
/// * `embedding` - Optional new embedding (Some(None) to clear)
/// * `superseded_by` - Optional superseding note ID
/// * `metadata` - Optional metadata (Some(None) to clear)
///
/// # Returns
/// * `Ok(true)` - If the note was found and updated
/// * `Ok(false)` - If no note with that ID exists
/// * `Err(CaliberError)` - On failure
///
/// # Requirements
/// - 4.4: Uses simple_heap_update instead of SPI UPDATE
pub fn note_update_heap(
    id: EntityId,
    content: Option<&str>,
    content_hash: Option<ContentHash>,
    embedding: Option<Option<&EmbeddingVector>>,
    superseded_by: Option<Option<EntityId>>,
    metadata: Option<Option<&serde_json::Value>>,
    tenant_id: EntityId,
) -> CaliberResult<bool> {
    // Open relation with RowExclusive lock for writes
    let rel = open_relation(note::TABLE_NAME, LockMode::RowExclusive)?;
    
    // Open the primary key index
    let index_rel = open_index(note::PK_INDEX)?;
    
    // Get active snapshot for visibility
    let snapshot = get_active_snapshot();
    
    // Build scan key for primary key lookup
    let mut scan_key = pg_sys::ScanKeyData::default();
    init_scan_key(
        &mut scan_key,
        1,
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
    
    // Find the existing tuple
    let old_tuple = match scanner.next() {
        Some(t) => t,
        None => return Ok(false), // Not found
    };
    
    let tid = scanner.current_tid()
        .ok_or_else(|| CaliberError::Storage(StorageError::UpdateFailed {
            entity_type: EntityType::Note,
            id,
            reason: "Failed to get TID of existing tuple".to_string(),
        }))?;
    
    let tuple_desc = rel.tuple_desc();
    let existing_tenant = unsafe { extract_uuid(old_tuple, tuple_desc, note::TENANT_ID)? };
    if existing_tenant != Some(tenant_id) {
        return Ok(false);
    }
    
    // Extract current values and nulls
    let (mut values, mut nulls) = unsafe { extract_values_and_nulls(old_tuple, tuple_desc) }?;
    
    // Apply updates
    if let Some(new_content) = content {
        values[note::CONTENT as usize - 1] = string_to_datum(new_content);
    }
    
    if let Some(new_hash) = content_hash {
        values[note::CONTENT_HASH as usize - 1] = content_hash_to_datum(&new_hash);
    }
    
    if let Some(new_embedding) = embedding {
        match new_embedding {
            Some(emb) => {
                values[note::EMBEDDING as usize - 1] = float4_array_to_datum(&emb.data);
                nulls[note::EMBEDDING as usize - 1] = false;
            }
            None => {
                nulls[note::EMBEDDING as usize - 1] = true;
            }
        }
    }
    
    if let Some(new_superseded) = superseded_by {
        match new_superseded {
            Some(s) => {
                values[note::SUPERSEDED_BY as usize - 1] = uuid_to_datum(s);
                nulls[note::SUPERSEDED_BY as usize - 1] = false;
            }
            None => {
                nulls[note::SUPERSEDED_BY as usize - 1] = true;
            }
        }
    }
    
    if let Some(new_metadata) = metadata {
        match new_metadata {
            Some(m) => {
                values[note::METADATA as usize - 1] = json_to_datum(m);
                nulls[note::METADATA as usize - 1] = false;
            }
            None => {
                nulls[note::METADATA as usize - 1] = true;
            }
        }
    }
    
    // Always update updated_at
    let now = current_timestamp();
    values[note::UPDATED_AT as usize - 1] = timestamp_to_pgrx(now).into_datum()
        .unwrap_or(pg_sys::Datum::from(0));
    
    // Form new tuple
    let new_tuple = form_tuple(&rel, &values, &nulls)?;
    
    // Update in place
    unsafe { update_tuple(&rel, &tid, new_tuple)? };
    
    // Update indexes
    unsafe { update_indexes_for_insert(&rel, new_tuple, &values, &nulls)? };
    
    Ok(true)
}


// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/// Validate that a HeapRelation is suitable for note operations.
fn validate_note_relation(rel: &HeapRelation) -> CaliberResult<()> {
    let natts = rel.natts();
    if natts != note::NUM_COLS as i16 {
        return Err(CaliberError::Storage(StorageError::TransactionFailed {
            reason: format!(
                "Note relation has {} columns, expected {}",
                natts,
                note::NUM_COLS
            ),
        }));
    }
    Ok(())
}

/// Convert a NoteType enum to its string representation.
fn note_type_to_str(t: NoteType) -> &'static str {
    match t {
        // Core note types
        NoteType::Convention => "convention",
        NoteType::Strategy => "strategy",
        NoteType::Gotcha => "gotcha",
        NoteType::Fact => "fact",
        NoteType::Preference => "preference",
        NoteType::Relationship => "relationship",
        NoteType::Procedure => "procedure",
        NoteType::Meta => "meta",
        // Extended note types
        NoteType::Insight => "insight",
        NoteType::Correction => "correction",
        NoteType::Summary => "summary",
    }
}

/// Parse a note type string to NoteType enum.
fn str_to_note_type(s: &str) -> NoteType {
    match s {
        // Core note types
        "convention" => NoteType::Convention,
        "strategy" => NoteType::Strategy,
        "gotcha" => NoteType::Gotcha,
        "fact" => NoteType::Fact,
        "preference" => NoteType::Preference,
        "relationship" => NoteType::Relationship,
        "procedure" => NoteType::Procedure,
        "meta" => NoteType::Meta,
        // Extended note types
        "insight" => NoteType::Insight,
        "correction" => NoteType::Correction,
        "summary" => NoteType::Summary,
        // Default fallback - use Meta for unknown types
        _ => {
            if s != "meta" {
                pgrx::warning!("CALIBER: Unknown note type '{}', defaulting to Meta", s);
            }
            NoteType::Meta
        }
    }
}

/// Convert a TTL enum to its string representation.
/// Returns a static string for simple variants, or a leaked string for Duration.
fn ttl_to_str(ttl: TTL) -> &'static str {
    match ttl {
        // Canonical variants
        TTL::Persistent => "persistent",
        TTL::Session => "session",
        TTL::Scope => "scope",
        TTL::Duration(ms) => Box::leak(format!("duration:{}", ms).into_boxed_str()),
        // Semantic aliases
        TTL::Ephemeral => "ephemeral",
        TTL::ShortTerm => "short_term",
        TTL::MediumTerm => "medium_term",
        TTL::LongTerm => "long_term",
        TTL::Permanent => "permanent",
    }
}

/// Parse a TTL string to TTL enum.
fn str_to_ttl(s: &str) -> TTL {
    match s {
        // Canonical variants
        "persistent" => TTL::Persistent,
        "session" => TTL::Session,
        "scope" => TTL::Scope,
        // Semantic aliases
        "ephemeral" => TTL::Ephemeral,
        "short_term" => TTL::ShortTerm,
        "medium_term" => TTL::MediumTerm,
        "long_term" => TTL::LongTerm,
        "permanent" => TTL::Permanent,
        // Duration parsing
        s if s.starts_with("duration:") => {
            let ms_str = &s[9..];
            ms_str.parse::<i64>().map(TTL::Duration).unwrap_or(TTL::Session)
        }
        _ => {
            pgrx::warning!("CALIBER: Unknown TTL value '{}', defaulting to Session", s);
            TTL::Session
        }
    }
}

// ============================================================================
// ABSTRACTION LEVEL HELPERS (Battle Intel Feature 2)
// ============================================================================

/// Convert an AbstractionLevel enum to its string representation.
fn abstraction_level_to_str(level: AbstractionLevel) -> &'static str {
    match level {
        AbstractionLevel::Raw => "raw",
        AbstractionLevel::Summary => "summary",
        AbstractionLevel::Principle => "principle",
    }
}

/// Parse an abstraction level string to AbstractionLevel enum.
fn str_to_abstraction_level(s: &str) -> AbstractionLevel {
    match s {
        "raw" => AbstractionLevel::Raw,
        "summary" => AbstractionLevel::Summary,
        "principle" => AbstractionLevel::Principle,
        _ => {
            pgrx::warning!("CALIBER: Unknown abstraction level '{}', defaulting to Raw", s);
            AbstractionLevel::Raw
        }
    }
}

/// Check if a note has expired based on its TTL and creation time.
///
/// Returns true if the note should be considered expired.
///
/// TTL enforcement rules:
/// - Persistent/Permanent: Never expires
/// - Session: Expires when session ends (not enforced here - requires session tracking)
/// - Scope/Ephemeral: Expires when scope closes (not enforced here - requires scope status check)
/// - Duration(ms): Expires if now > created_at + duration
/// - ShortTerm: Expires after 1 hour (3600000 ms)
/// - MediumTerm: Expires after 24 hours (86400000 ms)
/// - LongTerm: Expires after 7 days (604800000 ms)
pub fn is_note_expired(ttl: &TTL, created_at: chrono::DateTime<chrono::Utc>) -> bool {
    let now = chrono::Utc::now();

    match ttl {
        // Never expire
        TTL::Persistent | TTL::Permanent => false,

        // Session-based - can't enforce without session tracking
        TTL::Session => false,

        // Scope-based - can't enforce without checking scope status
        // Caller should filter these separately if needed
        TTL::Scope | TTL::Ephemeral => false,

        // Duration-based - check if expired
        TTL::Duration(ms) => {
            let expires_at = created_at + chrono::Duration::milliseconds(*ms);
            now > expires_at
        }

        // Semantic duration aliases
        TTL::ShortTerm => {
            let expires_at = created_at + chrono::Duration::milliseconds(3_600_000); // 1 hour
            now > expires_at
        }
        TTL::MediumTerm => {
            let expires_at = created_at + chrono::Duration::milliseconds(86_400_000); // 24 hours
            now > expires_at
        }
        TTL::LongTerm => {
            let expires_at = created_at + chrono::Duration::milliseconds(604_800_000); // 7 days
            now > expires_at
        }
    }
}

/// Convert a heap tuple to a Note struct.
unsafe fn tuple_to_note(
    tuple: *mut pg_sys::HeapTupleData,
    tuple_desc: pg_sys::TupleDesc,
) -> CaliberResult<NoteRow> {
    let note_id = extract_uuid(tuple, tuple_desc, note::NOTE_ID)?
        .ok_or_else(|| CaliberError::Storage(StorageError::TransactionFailed {
            reason: "note_id is NULL".to_string(),
        }))?;
    
    let note_type_str = extract_text(tuple, tuple_desc, note::NOTE_TYPE)?
        .ok_or_else(|| CaliberError::Storage(StorageError::TransactionFailed {
            reason: "note_type is NULL".to_string(),
        }))?;
    let note_type = str_to_note_type(&note_type_str);
    
    let title = extract_text(tuple, tuple_desc, note::TITLE)?
        .ok_or_else(|| CaliberError::Storage(StorageError::TransactionFailed {
            reason: "title is NULL".to_string(),
        }))?;
    
    let content = extract_text(tuple, tuple_desc, note::CONTENT)?
        .ok_or_else(|| CaliberError::Storage(StorageError::TransactionFailed {
            reason: "content is NULL".to_string(),
        }))?;
    
    let content_hash = extract_content_hash(tuple, tuple_desc, note::CONTENT_HASH)?
        .ok_or_else(|| CaliberError::Storage(StorageError::TransactionFailed {
            reason: "content_hash is NULL".to_string(),
        }))?;
    
    // Extract embedding as float array and convert to EmbeddingVector
    let embedding_data = extract_float4_array(tuple, tuple_desc, note::EMBEDDING)?;
    let embedding = embedding_data.map(|data| EmbeddingVector::new(data, "unknown".to_string()));
    
    let source_trajectory_ids = extract_uuid_array(tuple, tuple_desc, note::SOURCE_TRAJECTORY_IDS)?
        .unwrap_or_default();
    
    let source_artifact_ids = extract_uuid_array(tuple, tuple_desc, note::SOURCE_ARTIFACT_IDS)?
        .unwrap_or_default();
    
    let ttl_str = extract_text(tuple, tuple_desc, note::TTL)?
        .ok_or_else(|| CaliberError::Storage(StorageError::TransactionFailed {
            reason: "ttl is NULL".to_string(),
        }))?;
    let ttl = str_to_ttl(&ttl_str);
    
    let created_at_ts = extract_timestamp(tuple, tuple_desc, note::CREATED_AT)?
        .ok_or_else(|| CaliberError::Storage(StorageError::TransactionFailed {
            reason: "created_at is NULL".to_string(),
        }))?;
    let created_at = timestamp_to_chrono(created_at_ts);
    
    let updated_at_ts = extract_timestamp(tuple, tuple_desc, note::UPDATED_AT)?
        .ok_or_else(|| CaliberError::Storage(StorageError::TransactionFailed {
            reason: "updated_at is NULL".to_string(),
        }))?;
    let updated_at = timestamp_to_chrono(updated_at_ts);
    
    let accessed_at_ts = extract_timestamp(tuple, tuple_desc, note::ACCESSED_AT)?
        .ok_or_else(|| CaliberError::Storage(StorageError::TransactionFailed {
            reason: "accessed_at is NULL".to_string(),
        }))?;
    let accessed_at = timestamp_to_chrono(accessed_at_ts);
    
    let access_count = extract_i32(tuple, tuple_desc, note::ACCESS_COUNT)?
        .ok_or_else(|| CaliberError::Storage(StorageError::TransactionFailed {
            reason: "access_count is NULL".to_string(),
        }))?;
    
    let superseded_by = extract_uuid(tuple, tuple_desc, note::SUPERSEDED_BY)?;

    let metadata = extract_jsonb(tuple, tuple_desc, note::METADATA)?;

    // Battle Intel Feature 2: Abstraction level
    let abstraction_level_str = extract_text(tuple, tuple_desc, note::ABSTRACTION_LEVEL)?
        .ok_or_else(|| CaliberError::Storage(StorageError::TransactionFailed {
            reason: "abstraction_level is NULL".to_string(),
        }))?;
    let abstraction_level = str_to_abstraction_level(&abstraction_level_str);

    // Battle Intel Feature 2: Source note IDs (derivation chain)
    let source_note_ids = extract_uuid_array(tuple, tuple_desc, note::SOURCE_NOTE_IDS)?
        .unwrap_or_default();

    let tenant_id = extract_uuid(tuple, tuple_desc, note::TENANT_ID)?;

    Ok(NoteRow {
        note: Note {
            note_id,
            note_type,
            title,
            content,
            content_hash,
            embedding,
            source_trajectory_ids,
            source_artifact_ids,
            ttl,
            created_at,
            updated_at,
            accessed_at,
            access_count,
            superseded_by,
            metadata,
            abstraction_level,
            source_note_ids,
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
    // Test Helpers - Generators for Note data
    // ========================================================================

    fn arb_note_title() -> impl Strategy<Value = String> {
        "[a-zA-Z][a-zA-Z0-9 _-]{0,63}".prop_map(|s| s.trim().to_string())
            .prop_filter("title must not be empty", |s| !s.is_empty())
    }

    fn arb_content() -> impl Strategy<Value = String> {
        "[a-zA-Z0-9 .,!?\\n-]{1,500}".prop_map(|s| s)
    }

    fn arb_note_type() -> impl Strategy<Value = NoteType> {
        prop_oneof![
            Just(NoteType::Insight),
            Just(NoteType::Procedure),
            Just(NoteType::Fact),
            Just(NoteType::Preference),
            Just(NoteType::Correction),
            Just(NoteType::Summary),
        ]
    }

    fn arb_ttl() -> impl Strategy<Value = TTL> {
        prop_oneof![
            Just(TTL::Ephemeral),
            Just(TTL::Session),
            Just(TTL::ShortTerm),
            Just(TTL::MediumTerm),
            Just(TTL::LongTerm),
            Just(TTL::Permanent),
        ]
    }

    // ========================================================================
    // Property 1: Insert-Get Round Trip (Note)
    // Validates: Requirements 4.1, 4.2
    // ========================================================================

    #[cfg(feature = "pg_test")]
    mod pg_tests {
        use super::*;
        use pgrx_tests::pg_test;

        /// Property 1: Insert-Get Round Trip (Note)
        #[pg_test]
        fn prop_note_insert_get_roundtrip() {
            use proptest::test_runner::{TestRunner, Config};

            let config = Config::with_cases(50);
            let mut runner = TestRunner::new(config);

            let strategy = (
                arb_note_title(),
                arb_content(),
                arb_note_type(),
                arb_ttl(),
            );

            runner.run(&strategy, |(title, content, note_type, ttl)| {
                let note_id = caliber_core::new_entity_id();
                let tenant_id = caliber_core::new_entity_id();
                let content_hash = caliber_core::compute_content_hash(content.as_bytes());

                // Insert via heap
                let result = note_create_heap(NoteCreateParams {
                    note_id,
                    note_type,
                    title: &title,
                    content: &content,
                    content_hash,
                    embedding: None,
                    source_trajectory_ids: &[],
                    source_artifact_ids: &[],
                    ttl,
                    abstraction_level: AbstractionLevel::Raw, // Battle Intel Feature 2
                    source_note_ids: &[],                      // Battle Intel Feature 2
                    tenant_id,
                });
                prop_assert!(result.is_ok(), "Insert should succeed");
                prop_assert_eq!(result.unwrap(), note_id);

                // Get via heap
                let get_result = note_get_heap(note_id, tenant_id);
                prop_assert!(get_result.is_ok(), "Get should succeed");
                
                let note = get_result.unwrap();
                prop_assert!(note.is_some(), "Note should be found");
                
                let row = note.unwrap();
                let n = row.note;
                
                // Verify round-trip preserves data
                prop_assert_eq!(n.note_id, note_id);
                prop_assert_eq!(n.note_type, note_type);
                prop_assert_eq!(n.title, title);
                prop_assert_eq!(n.content, content);
                prop_assert_eq!(n.content_hash, content_hash);
                prop_assert_eq!(n.ttl, ttl);
                prop_assert_eq!(n.access_count, 0);
                prop_assert!(n.embedding.is_none());
                prop_assert!(n.source_trajectory_ids.is_empty());
                prop_assert!(n.source_artifact_ids.is_empty());
                prop_assert!(n.superseded_by.is_none());
                prop_assert!(n.metadata.is_none());
                // Battle Intel Feature 2 assertions
                prop_assert_eq!(n.abstraction_level, AbstractionLevel::Raw);
                prop_assert!(n.source_note_ids.is_empty());
                prop_assert_eq!(row.tenant_id, Some(tenant_id));

                Ok(())
            }).unwrap();
        }

        /// Get non-existent note returns None
        #[pg_test]
        fn prop_note_get_nonexistent_returns_none() {
            use proptest::test_runner::{TestRunner, Config};

            let config = Config::with_cases(100);
            let mut runner = TestRunner::new(config);

            runner.run(&any::<[u8; 16]>(), |bytes| {
                let random_id = uuid::Uuid::from_bytes(bytes);
                
                let tenant_id = caliber_core::new_entity_id();
                let result = note_get_heap(random_id, tenant_id);
                prop_assert!(result.is_ok(), "Get should not error");
                prop_assert!(result.unwrap().is_none(), "Non-existent note should return None");

                Ok(())
            }).unwrap();
        }
    }

    // ========================================================================
    // Property 2: Update Persistence (Note)
    // Validates: Requirements 4.4
    // ========================================================================

    #[cfg(feature = "pg_test")]
    mod update_tests {
        use super::*;
        use pgrx_tests::pg_test;

        /// Property 2: Update content persists
        #[pg_test]
        fn prop_note_update_content_persists() {
            use proptest::test_runner::{TestRunner, Config};

            let config = Config::with_cases(50);
            let mut runner = TestRunner::new(config);

            let strategy = (arb_content(), arb_content());

            runner.run(&strategy, |(original_content, new_content)| {
                let note_id = caliber_core::new_entity_id();
                let tenant_id = caliber_core::new_entity_id();
                let original_hash = caliber_core::compute_content_hash(original_content.as_bytes());
                
                let _ = note_create_heap(NoteCreateParams {
                    note_id,
                    note_type: NoteType::Fact,
                    title: "test_note",
                    content: &original_content,
                    content_hash: original_hash,
                    embedding: None,
                    source_trajectory_ids: &[],
                    source_artifact_ids: &[],
                    ttl: TTL::MediumTerm,
                    abstraction_level: AbstractionLevel::Raw,
                    source_note_ids: &[],
                    tenant_id,
                });

                // Update content
                let new_hash = caliber_core::compute_content_hash(new_content.as_bytes());
                let update_result = note_update_heap(
                    note_id,
                    Some(&new_content),
                    Some(new_hash),
                    None,
                    None,
                    None,
                    tenant_id,
                );
                prop_assert!(update_result.is_ok());
                prop_assert!(update_result.unwrap(), "Update should find the note");

                // Verify updated
                let after = note_get_heap(note_id, tenant_id).unwrap().unwrap();
                prop_assert_eq!(after.note.content, new_content);
                prop_assert_eq!(after.note.content_hash, new_hash);
                prop_assert_eq!(after.tenant_id, Some(tenant_id));

                Ok(())
            }).unwrap();
        }

        /// Update non-existent note returns false
        #[pg_test]
        fn prop_note_update_nonexistent_returns_false() {
            use proptest::test_runner::{TestRunner, Config};

            let config = Config::with_cases(100);
            let mut runner = TestRunner::new(config);

            runner.run(&any::<[u8; 16]>(), |bytes| {
                let random_id = uuid::Uuid::from_bytes(bytes);
                
                let result = note_update_heap(
                    random_id,
                    Some("new content"),
                    None,
                    None,
                    None,
                    None,
                    caliber_core::new_entity_id(),
                );
                prop_assert!(result.is_ok(), "Update should not error");
                prop_assert!(!result.unwrap(), "Update of non-existent note should return false");

                Ok(())
            }).unwrap();
        }
    }

    // ========================================================================
    // Property 3: Query by Trajectory (Note)
    // Validates: Requirements 4.3
    // ========================================================================

    #[cfg(feature = "pg_test")]
    mod query_tests {
        use super::*;
        use pgrx_tests::pg_test;

        /// Property 3: Query by trajectory returns notes with that source
        #[pg_test]
        fn prop_note_query_by_trajectory() {
            use proptest::test_runner::{TestRunner, Config};

            let config = Config::with_cases(30);
            let mut runner = TestRunner::new(config);

            let strategy = 1usize..4usize;

            runner.run(&strategy, |num_notes| {
                // Create a trajectory to use as source
                let trajectory_id = caliber_core::new_entity_id();
                let tenant_id = caliber_core::new_entity_id();
                let _ = crate::trajectory_heap::trajectory_create_heap(
                    trajectory_id,
                    "source_trajectory",
                    None,
                    None,
                    tenant_id,
                );

                // Create notes with this trajectory as source
                let mut note_ids = Vec::new();
                for i in 0..num_notes {
                    let note_id = caliber_core::new_entity_id();
                    let content = format!("note_content_{}", i);
                    let content_hash = caliber_core::compute_content_hash(content.as_bytes());
                    
                    let _ = note_create_heap(NoteCreateParams {
                        note_id,
                        note_type: NoteType::Fact,
                        title: &format!("note_{}", i),
                        content: &content,
                        content_hash,
                        embedding: None,
                        source_trajectory_ids: &[trajectory_id],
                        source_artifact_ids: &[],
                        ttl: TTL::MediumTerm,
                        abstraction_level: AbstractionLevel::Raw,
                        source_note_ids: &[],
                        tenant_id,
                    });
                    note_ids.push(note_id);
                }

                // Query by trajectory
                let query_result = note_query_by_trajectory_heap(trajectory_id, tenant_id);
                prop_assert!(query_result.is_ok(), "Query should succeed");
                
                let notes = query_result.unwrap();
                prop_assert_eq!(notes.len(), num_notes, "Should return all notes");

                // All created notes should be in result
                for note_id in &note_ids {
                    prop_assert!(
                        notes.iter().any(|n| n.note.note_id == *note_id),
                        "All created notes should be in result"
                    );
                }

                // All results should have trajectory_id in source_trajectory_ids
                for row in &notes {
                    prop_assert!(
                        row.note.source_trajectory_ids.contains(&trajectory_id),
                        "All notes should have trajectory in source_trajectory_ids"
                    );
                    prop_assert_eq!(row.tenant_id, Some(tenant_id));
                }

                Ok(())
            }).unwrap();
        }

        /// Query by non-existent trajectory returns empty
        #[pg_test]
        fn prop_note_query_by_nonexistent_trajectory_returns_empty() {
            use proptest::test_runner::{TestRunner, Config};

            let config = Config::with_cases(100);
            let mut runner = TestRunner::new(config);

            runner.run(&any::<[u8; 16]>(), |bytes| {
                let random_id = uuid::Uuid::from_bytes(bytes);
                
                let tenant_id = caliber_core::new_entity_id();
                let result = note_query_by_trajectory_heap(random_id, tenant_id);
                prop_assert!(result.is_ok(), "Query should not error");
                prop_assert!(result.unwrap().is_empty(), "Query for non-existent trajectory should be empty");

                Ok(())
            }).unwrap();
        }
    }
}
