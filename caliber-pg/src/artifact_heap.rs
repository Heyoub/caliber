//! Direct heap operations for Artifact entities.
//!
//! This module provides hot-path operations for artifacts that bypass SQL
//! parsing entirely by using direct heap access via pgrx.
//!
//! # Operations
//!
//! - `artifact_create_heap` - Insert a new artifact
//! - `artifact_get_heap` - Get an artifact by ID
//! - `artifact_query_by_type_heap` - Query artifacts by type
//! - `artifact_query_by_scope_heap` - Query artifacts by scope
//! - `artifact_update_heap` - Update artifact fields

use pgrx::prelude::*;
use pgrx::pg_sys;

use caliber_core::{
    Artifact, ArtifactId, ArtifactType, CaliberError, CaliberResult, ContentHash,
    EmbeddingVector, EntityIdType, EntityType, ExtractionMethod, Provenance,
    ScopeId, StorageError, TenantId, TrajectoryId, TTL,
};

use crate::column_maps::artifact;
use crate::heap_ops::{
    current_timestamp, form_tuple, insert_tuple, open_relation,
    update_tuple, PgLockMode as LockMode, HeapRelation, get_active_snapshot,
    timestamp_to_pgrx,
};
use crate::index_ops::{
    init_scan_key, open_index, update_indexes_for_insert,
    BTreeStrategy, IndexScanner, operator_oids,
};
use crate::tuple_extract::{
    extract_uuid, extract_text, extract_timestamp, extract_jsonb,
    extract_bytea, extract_float4_array, extract_values_and_nulls,
    uuid_to_datum, string_to_datum, json_to_datum, bytea_to_datum,
    float4_array_to_datum, timestamp_to_chrono, content_hash_to_datum,
    extract_content_hash,
};

/// Artifact row with tenant ownership metadata.
pub struct ArtifactRow {
    pub artifact: Artifact,
    pub tenant_id: Option<TenantId>,
}

impl From<ArtifactRow> for Artifact {
    fn from(row: ArtifactRow) -> Self {
        row.artifact
    }
}

/// Create a new artifact using direct heap operations.
///
/// This bypasses SQL parsing entirely for hot-path performance.
///
/// # Arguments
/// * `artifact_id` - The pre-generated UUIDv7 for this artifact
/// * `trajectory_id` - The parent trajectory ID
/// * `scope_id` - The parent scope ID
/// * `artifact_type` - The type of artifact
/// * `name` - The artifact name
/// * `content` - The artifact content
/// * `content_hash` - SHA-256 hash of content
/// * `embedding` - Optional embedding vector
/// * `provenance` - Provenance information
/// * `ttl` - Time-to-live setting
///
/// # Returns
/// * `Ok(EntityId)` - The artifact ID on success
/// * `Err(CaliberError)` - On failure
///
/// # Requirements
/// - 3.1: Uses heap_form_tuple and simple_heap_insert instead of SPI
/// - 3.6: Updates btree and hnsw indexes via CatalogIndexInsert
pub struct ArtifactCreateParams<'a> {
    pub artifact_id: ArtifactId,
    pub trajectory_id: TrajectoryId,
    pub scope_id: ScopeId,
    pub artifact_type: ArtifactType,
    pub name: &'a str,
    pub content: &'a str,
    pub content_hash: ContentHash,
    pub embedding: Option<&'a EmbeddingVector>,
    pub provenance: &'a Provenance,
    pub ttl: TTL,
    pub tenant_id: TenantId,
}

pub fn artifact_create_heap(params: ArtifactCreateParams<'_>) -> CaliberResult<ArtifactId> {
    let ArtifactCreateParams {
        artifact_id,
        trajectory_id,
        scope_id,
        artifact_type,
        name,
        content,
        content_hash,
        embedding,
        provenance,
        ttl,
        tenant_id,
    } = params;
    // Open relation with RowExclusive lock for writes
    let rel = open_relation(artifact::TABLE_NAME, LockMode::RowExclusive)?;
    validate_artifact_relation(&rel)?;

    // Get current transaction timestamp for created_at/updated_at
    let now = current_timestamp();
    let now_datum = timestamp_to_pgrx(now)?.into_datum()
        .ok_or_else(|| CaliberError::Storage(StorageError::InsertFailed {
            entity_type: EntityType::Artifact,
            reason: "Failed to convert timestamp to datum".to_string(),
        }))?;
    
    // Build datum array - must match column order in caliber_artifact table
    let mut values: [pg_sys::Datum; artifact::NUM_COLS] = [pg_sys::Datum::from(0); artifact::NUM_COLS];
    let mut nulls: [bool; artifact::NUM_COLS] = [false; artifact::NUM_COLS];
    
    // Column 1: artifact_id (UUID, NOT NULL)
    values[artifact::ARTIFACT_ID as usize - 1] = uuid_to_datum(artifact_id.as_uuid());

    // Column 2: trajectory_id (UUID, NOT NULL)
    values[artifact::TRAJECTORY_ID as usize - 1] = uuid_to_datum(trajectory_id.as_uuid());

    // Column 3: scope_id (UUID, NOT NULL)
    values[artifact::SCOPE_ID as usize - 1] = uuid_to_datum(scope_id.as_uuid());
    
    // Column 4: artifact_type (TEXT, NOT NULL)
    values[artifact::ARTIFACT_TYPE as usize - 1] = string_to_datum(artifact_type_to_str(artifact_type));
    
    // Column 5: name (TEXT, NOT NULL)
    values[artifact::NAME as usize - 1] = string_to_datum(name);
    
    // Column 6: content (TEXT, NOT NULL)
    values[artifact::CONTENT as usize - 1] = string_to_datum(content);
    
    // Column 7: content_hash (BYTEA, NOT NULL)
    values[artifact::CONTENT_HASH as usize - 1] = content_hash_to_datum(&content_hash);
    
    // Column 8: embedding (VECTOR, nullable)
    if let Some(emb) = embedding {
        values[artifact::EMBEDDING as usize - 1] = float4_array_to_datum(&emb.data);
    } else {
        nulls[artifact::EMBEDDING as usize - 1] = true;
    }
    
    // Column 9: provenance (JSONB, NOT NULL)
    let provenance_json = serde_json::to_value(provenance)
        .map_err(|e| CaliberError::Storage(StorageError::InsertFailed {
            entity_type: EntityType::Artifact,
            reason: format!("Failed to serialize provenance: {}", e),
        }))?;
    values[artifact::PROVENANCE as usize - 1] = json_to_datum(&provenance_json);
    
    // Column 10: ttl (TEXT, NOT NULL)
    values[artifact::TTL as usize - 1] = string_to_datum(ttl_to_str(ttl));
    
    // Column 11: created_at (TIMESTAMPTZ, NOT NULL)
    values[artifact::CREATED_AT as usize - 1] = now_datum;
    
    // Column 12: updated_at (TIMESTAMPTZ, NOT NULL)
    values[artifact::UPDATED_AT as usize - 1] = now_datum;
    
    // Column 13: superseded_by (UUID, nullable)
    nulls[artifact::SUPERSEDED_BY as usize - 1] = true;
    
    // Column 14: metadata (JSONB, nullable)
    nulls[artifact::METADATA as usize - 1] = true;

    // Column 15: tenant_id (UUID, NOT NULL)
    values[artifact::TENANT_ID as usize - 1] = uuid_to_datum(tenant_id.as_uuid());
    
    // Form the heap tuple
    let tuple = form_tuple(&rel, &values, &nulls)?;
    
    // Insert into heap
    let _tid = unsafe { insert_tuple(&rel, tuple)? };
    
    // Update all indexes via CatalogIndexInsert
    unsafe { update_indexes_for_insert(&rel, tuple, &values, &nulls)? };
    
    Ok(artifact_id)
}


/// Get an artifact by ID using direct heap operations.
///
/// This bypasses SQL parsing entirely for hot-path performance.
///
/// # Arguments
/// * `id` - The artifact ID to look up
///
/// # Returns
/// * `Ok(Some(Artifact))` - The artifact if found
/// * `Ok(None)` - If no artifact with that ID exists
/// * `Err(CaliberError)` - On failure
///
/// # Requirements
/// - 3.2: Uses index_beginscan for O(log n) lookup instead of SPI SELECT
pub fn artifact_get_heap(id: ArtifactId, tenant_id: TenantId) -> CaliberResult<Option<ArtifactRow>> {
    // Open relation with AccessShare lock for reads
    let rel = open_relation(artifact::TABLE_NAME, LockMode::AccessShare)?;
    
    // Open the primary key index
    let index_rel = open_index(artifact::PK_INDEX)?;
    
    // Get active snapshot for visibility
    let snapshot = get_active_snapshot();
    
    // Build scan key for primary key lookup
    let mut scan_key = pg_sys::ScanKeyData::default();
    init_scan_key(
        &mut scan_key,
        1, // First column of index (artifact_id)
        BTreeStrategy::Equal,
        operator_oids::UUID_EQ,
        uuid_to_datum(id.as_uuid()),
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
        let row = unsafe { tuple_to_artifact(tuple, tuple_desc) }?;
        if row.tenant_id.map(|t| t.as_uuid()) == Some(tenant_id.as_uuid()) {
            Ok(Some(row))
        } else {
            Ok(None)
        }
    } else {
        Ok(None)
    }
}

/// Query artifacts by type using direct heap operations.
///
/// # Arguments
/// * `artifact_type` - The artifact type to filter by
///
/// # Returns
/// * `Ok(Vec<Artifact>)` - List of matching artifacts
/// * `Err(CaliberError)` - On failure
///
/// # Requirements
/// - 3.3: Uses index scan instead of SPI SELECT
pub fn artifact_query_by_type_heap(
    artifact_type: ArtifactType,
    tenant_id: TenantId,
) -> CaliberResult<Vec<ArtifactRow>> {
    // Open relation with AccessShare lock for reads
    let rel = open_relation(artifact::TABLE_NAME, LockMode::AccessShare)?;
    
    // Open the type index
    let index_rel = open_index(artifact::TYPE_INDEX)?;
    
    // Get active snapshot for visibility
    let snapshot = get_active_snapshot();
    
    // Build scan key for type lookup
    let mut scan_key = pg_sys::ScanKeyData::default();
    init_scan_key(
        &mut scan_key,
        1, // First column of index (artifact_type)
        BTreeStrategy::Equal,
        operator_oids::TEXT_EQ,
        string_to_datum(artifact_type_to_str(artifact_type)),
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

    // Collect all matching tuples, filtering out expired ones
    for tuple in &mut scanner {
        let row = unsafe { tuple_to_artifact(tuple, tuple_desc) }?;
        // Enforce TTL - skip expired artifacts
        if row.tenant_id.map(|t| t.as_uuid()) == Some(tenant_id.as_uuid())
            && !is_artifact_expired(&row.artifact.ttl, row.artifact.created_at)
        {
            results.push(row);
        }
    }

    Ok(results)
}

/// Query artifacts by scope using direct heap operations.
///
/// # Arguments
/// * `scope_id` - The scope ID to filter by
///
/// # Returns
/// * `Ok(Vec<Artifact>)` - List of matching artifacts
/// * `Err(CaliberError)` - On failure
///
/// # Requirements
/// - 3.4: Uses index scan instead of SPI SELECT
pub fn artifact_query_by_scope_heap(
    scope_id: ScopeId,
    tenant_id: TenantId,
) -> CaliberResult<Vec<ArtifactRow>> {
    // Open relation with AccessShare lock for reads
    let rel = open_relation(artifact::TABLE_NAME, LockMode::AccessShare)?;
    
    // Open the scope index
    let index_rel = open_index(artifact::SCOPE_INDEX)?;
    
    // Get active snapshot for visibility
    let snapshot = get_active_snapshot();
    
    // Build scan key for scope_id lookup
    let mut scan_key = pg_sys::ScanKeyData::default();
    init_scan_key(
        &mut scan_key,
        1, // First column of index (scope_id)
        BTreeStrategy::Equal,
        operator_oids::UUID_EQ,
        uuid_to_datum(scope_id.as_uuid()),
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

    // Collect all matching tuples, filtering out expired ones
    for tuple in &mut scanner {
        let row = unsafe { tuple_to_artifact(tuple, tuple_desc) }?;
        // Enforce TTL - skip expired artifacts
        if row.tenant_id.map(|t| t.as_uuid()) == Some(tenant_id.as_uuid())
            && !is_artifact_expired(&row.artifact.ttl, row.artifact.created_at)
        {
            results.push(row);
        }
    }

    Ok(results)
}

/// Query artifacts by trajectory using direct heap operations.
///
/// # Arguments
/// * `trajectory_id` - The trajectory ID to filter by
///
/// # Returns
/// * `Ok(Vec<ArtifactRow>)` - List of matching artifacts
/// * `Err(CaliberError)` - On failure
pub fn artifact_query_by_trajectory_heap(
    trajectory_id: TrajectoryId,
    tenant_id: TenantId,
) -> CaliberResult<Vec<ArtifactRow>> {
    // Open relation with AccessShare lock for reads
    let rel = open_relation(artifact::TABLE_NAME, LockMode::AccessShare)?;

    // Open the trajectory index
    let index_rel = open_index(artifact::TRAJECTORY_INDEX)?;

    // Get active snapshot for visibility
    let snapshot = get_active_snapshot();

    // Build scan key for trajectory_id lookup
    let mut scan_key = pg_sys::ScanKeyData::default();
    init_scan_key(
        &mut scan_key,
        1, // First column of index (trajectory_id)
        BTreeStrategy::Equal,
        operator_oids::UUID_EQ,
        uuid_to_datum(trajectory_id.as_uuid()),
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

    // Collect all matching tuples, filtering out expired ones
    for tuple in &mut scanner {
        let row = unsafe { tuple_to_artifact(tuple, tuple_desc) }?;
        if row.tenant_id.map(|t| t.as_uuid()) == Some(tenant_id.as_uuid())
            && !is_artifact_expired(&row.artifact.ttl, row.artifact.created_at)
        {
            results.push(row);
        }
    }

    Ok(results)
}


/// Update an artifact using direct heap operations.
///
/// # Arguments
/// * `id` - The artifact ID to update
/// * `content` - Optional new content
/// * `content_hash` - Optional new content hash (required if content changes)
/// * `embedding` - Optional new embedding (Some(None) to clear)
/// * `superseded_by` - Optional superseding artifact ID
/// * `metadata` - Optional metadata (Some(None) to clear)
///
/// # Returns
/// * `Ok(true)` - If the artifact was found and updated
/// * `Ok(false)` - If no artifact with that ID exists
/// * `Err(CaliberError)` - On failure
///
/// # Requirements
/// - 3.5: Uses simple_heap_update instead of SPI UPDATE
pub fn artifact_update_heap(
    id: ArtifactId,
    content: Option<&str>,
    content_hash: Option<ContentHash>,
    embedding: Option<Option<&EmbeddingVector>>,
    superseded_by: Option<Option<ArtifactId>>,
    metadata: Option<Option<&serde_json::Value>>,
    tenant_id: TenantId,
) -> CaliberResult<bool> {
    // Open relation with RowExclusive lock for writes
    let rel = open_relation(artifact::TABLE_NAME, LockMode::RowExclusive)?;
    
    // Open the primary key index
    let index_rel = open_index(artifact::PK_INDEX)?;
    
    // Get active snapshot for visibility
    let snapshot = get_active_snapshot();
    
    // Build scan key for primary key lookup
    let mut scan_key = pg_sys::ScanKeyData::default();
    init_scan_key(
        &mut scan_key,
        1,
        BTreeStrategy::Equal,
        operator_oids::UUID_EQ,
        uuid_to_datum(id.as_uuid()),
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
            entity_type: EntityType::Artifact,
            id: id.as_uuid(),
            reason: "Failed to get TID of existing tuple".to_string(),
        }))?;

    let tuple_desc = rel.tuple_desc();
    let existing_tenant = unsafe { extract_uuid(old_tuple, tuple_desc, artifact::TENANT_ID)? };
    if existing_tenant != Some(tenant_id.as_uuid()) {
        return Ok(false);
    }
    
    // Extract current values and nulls
    let (mut values, mut nulls) = unsafe { extract_values_and_nulls(old_tuple, tuple_desc) }?;
    
    // Apply updates
    if let Some(new_content) = content {
        values[artifact::CONTENT as usize - 1] = string_to_datum(new_content);
    }
    
    if let Some(new_hash) = content_hash {
        values[artifact::CONTENT_HASH as usize - 1] = content_hash_to_datum(&new_hash);
    }
    
    if let Some(new_embedding) = embedding {
        match new_embedding {
            Some(emb) => {
                values[artifact::EMBEDDING as usize - 1] = float4_array_to_datum(&emb.data);
                nulls[artifact::EMBEDDING as usize - 1] = false;
            }
            None => {
                nulls[artifact::EMBEDDING as usize - 1] = true;
            }
        }
    }
    
    if let Some(new_superseded) = superseded_by {
        match new_superseded {
            Some(s) => {
                values[artifact::SUPERSEDED_BY as usize - 1] = uuid_to_datum(s.as_uuid());
                nulls[artifact::SUPERSEDED_BY as usize - 1] = false;
            }
            None => {
                nulls[artifact::SUPERSEDED_BY as usize - 1] = true;
            }
        }
    }
    
    if let Some(new_metadata) = metadata {
        match new_metadata {
            Some(m) => {
                values[artifact::METADATA as usize - 1] = json_to_datum(m);
                nulls[artifact::METADATA as usize - 1] = false;
            }
            None => {
                nulls[artifact::METADATA as usize - 1] = true;
            }
        }
    }
    
    // Always update updated_at
    let now = current_timestamp();
    values[artifact::UPDATED_AT as usize - 1] = timestamp_to_pgrx(now)?
        .into_datum()
        .ok_or_else(|| CaliberError::Storage(StorageError::UpdateFailed {
            entity_type: EntityType::Artifact,
            id: id.as_uuid(),
            reason: "Failed to convert timestamp to datum".to_string(),
        }))?;
    
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

/// Validate that a HeapRelation is suitable for artifact operations.
fn validate_artifact_relation(rel: &HeapRelation) -> CaliberResult<()> {
    let natts = rel.natts();
    if natts != artifact::NUM_COLS as i16 {
        return Err(CaliberError::Storage(StorageError::TransactionFailed {
            reason: format!(
                "Artifact relation has {} columns, expected {}",
                natts,
                artifact::NUM_COLS
            ),
        }));
    }
    Ok(())
}

/// Convert an ExtractionMethod to its string representation.
/// This is used for storing the extraction method in the database.
pub fn extraction_method_to_str(method: &ExtractionMethod) -> &'static str {
    match method {
        ExtractionMethod::Explicit => "explicit",
        ExtractionMethod::Inferred => "inferred",
        ExtractionMethod::UserProvided => "user_provided",
        ExtractionMethod::LlmExtraction => "llm_extraction",
        ExtractionMethod::ToolExtraction => "tool_extraction",
        ExtractionMethod::MemoryRecall => "memory_recall",
        ExtractionMethod::ExternalApi => "external_api",
        ExtractionMethod::Unknown => "unknown",
    }
}

/// Parse an extraction method string to ExtractionMethod enum.
pub fn str_to_extraction_method(s: &str) -> ExtractionMethod {
    match s {
        "explicit" | "explicit_save" => ExtractionMethod::Explicit,
        "inferred" | "automatic_extraction" | "system_inferred" => ExtractionMethod::Inferred,
        "user_provided" | "agent_suggestion" => ExtractionMethod::UserProvided,
        "llm_extraction" => ExtractionMethod::LlmExtraction,
        "tool_extraction" => ExtractionMethod::ToolExtraction,
        "memory_recall" => ExtractionMethod::MemoryRecall,
        "external_api" => ExtractionMethod::ExternalApi,
        "unknown" => ExtractionMethod::Unknown,
        _ => {
            pgrx::warning!("CALIBER: Unknown extraction method '{}', defaulting to Unknown", s);
            ExtractionMethod::Unknown
        }
    }
}

/// Convert binary data to a datum for storage.
/// This is useful for storing raw binary artifacts (images, PDFs, etc.).
pub fn binary_data_to_datum(data: &[u8]) -> pg_sys::Datum {
    bytea_to_datum(data)
}

/// Extract binary data from a tuple at the specified column.
/// This is useful for retrieving raw binary artifacts.
///
/// # Safety
/// The tuple and tuple_desc must be valid and correspond to each other.
pub unsafe fn extract_binary_data(
    tuple: *mut pg_sys::HeapTupleData,
    tuple_desc: pg_sys::TupleDesc,
    attno: i16,
) -> CaliberResult<Option<Vec<u8>>> {
    extract_bytea(tuple, tuple_desc, attno)
}

/// Convert an ArtifactType enum to its string representation.
fn artifact_type_to_str(t: ArtifactType) -> &'static str {
    match t {
        // Core types
        ArtifactType::ErrorLog => "error_log",
        ArtifactType::CodePatch => "code_patch",
        ArtifactType::DesignDecision => "design_decision",
        ArtifactType::UserPreference => "user_preference",
        ArtifactType::Fact => "fact",
        ArtifactType::Constraint => "constraint",
        ArtifactType::ToolResult => "tool_result",
        ArtifactType::IntermediateOutput => "intermediate_output",
        ArtifactType::Custom => "custom",
        // Extended types
        ArtifactType::Code => "code",
        ArtifactType::Document => "document",
        ArtifactType::Data => "data",
        ArtifactType::Model => "model",
        ArtifactType::Config => "config",
        ArtifactType::Log => "log",
        ArtifactType::Summary => "summary",
        ArtifactType::Decision => "decision",
        ArtifactType::Plan => "plan",
    }
}

/// Parse an artifact type string to ArtifactType enum.
fn str_to_artifact_type(s: &str) -> ArtifactType {
    match s {
        // Core types
        "error_log" => ArtifactType::ErrorLog,
        "code_patch" => ArtifactType::CodePatch,
        "design_decision" => ArtifactType::DesignDecision,
        "user_preference" => ArtifactType::UserPreference,
        "fact" => ArtifactType::Fact,
        "constraint" => ArtifactType::Constraint,
        "tool_result" => ArtifactType::ToolResult,
        "intermediate_output" => ArtifactType::IntermediateOutput,
        // Extended types
        "code" => ArtifactType::Code,
        "document" => ArtifactType::Document,
        "data" => ArtifactType::Data,
        "model" => ArtifactType::Model,
        "config" => ArtifactType::Config,
        "log" => ArtifactType::Log,
        "summary" => ArtifactType::Summary,
        "decision" => ArtifactType::Decision,
        "plan" => ArtifactType::Plan,
        // Default
        _ => {
            if s != "custom" {
                pgrx::warning!("CALIBER: Unknown artifact type '{}', defaulting to Custom", s);
            }
            ArtifactType::Custom
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

/// Check if an artifact has expired based on its TTL and creation time.
///
/// Returns true if the artifact should be considered expired.
///
/// TTL enforcement rules:
/// - Persistent/Permanent: Never expires
/// - Session: Expires when session ends (not enforced here - requires session tracking)
/// - Scope/Ephemeral: Expires when scope closes (not enforced here - requires scope status check)
/// - Duration(ms): Expires if now > created_at + duration
/// - ShortTerm: Expires after 1 hour (3600000 ms)
/// - MediumTerm: Expires after 24 hours (86400000 ms)
/// - LongTerm: Expires after 7 days (604800000 ms)
pub fn is_artifact_expired(ttl: &TTL, created_at: chrono::DateTime<chrono::Utc>) -> bool {
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

/// Convert a heap tuple to an Artifact struct.
unsafe fn tuple_to_artifact(
    tuple: *mut pg_sys::HeapTupleData,
    tuple_desc: pg_sys::TupleDesc,
) -> CaliberResult<ArtifactRow> {
    // Extract all fields from the tuple
    let artifact_id = extract_uuid(tuple, tuple_desc, artifact::ARTIFACT_ID)?
        .ok_or_else(|| CaliberError::Storage(StorageError::TransactionFailed {
            reason: "artifact_id is NULL".to_string(),
        }))?;
    
    let trajectory_id = extract_uuid(tuple, tuple_desc, artifact::TRAJECTORY_ID)?
        .ok_or_else(|| CaliberError::Storage(StorageError::TransactionFailed {
            reason: "trajectory_id is NULL".to_string(),
        }))?;
    
    let scope_id = extract_uuid(tuple, tuple_desc, artifact::SCOPE_ID)?
        .ok_or_else(|| CaliberError::Storage(StorageError::TransactionFailed {
            reason: "scope_id is NULL".to_string(),
        }))?;
    
    let artifact_type_str = extract_text(tuple, tuple_desc, artifact::ARTIFACT_TYPE)?
        .ok_or_else(|| CaliberError::Storage(StorageError::TransactionFailed {
            reason: "artifact_type is NULL".to_string(),
        }))?;
    let artifact_type = str_to_artifact_type(&artifact_type_str);
    
    let name = extract_text(tuple, tuple_desc, artifact::NAME)?
        .ok_or_else(|| CaliberError::Storage(StorageError::TransactionFailed {
            reason: "name is NULL".to_string(),
        }))?;
    
    let content = extract_text(tuple, tuple_desc, artifact::CONTENT)?
        .ok_or_else(|| CaliberError::Storage(StorageError::TransactionFailed {
            reason: "content is NULL".to_string(),
        }))?;
    
    let content_hash = extract_content_hash(tuple, tuple_desc, artifact::CONTENT_HASH)?
        .ok_or_else(|| CaliberError::Storage(StorageError::TransactionFailed {
            reason: "content_hash is NULL".to_string(),
        }))?;
    
    // Extract embedding as float array and convert to EmbeddingVector
    let embedding_data = extract_float4_array(tuple, tuple_desc, artifact::EMBEDDING)?;
    let embedding = embedding_data.map(|data| EmbeddingVector::new(data, "unknown".to_string()));
    
    let provenance_json = extract_jsonb(tuple, tuple_desc, artifact::PROVENANCE)?
        .ok_or_else(|| CaliberError::Storage(StorageError::TransactionFailed {
            reason: "provenance is NULL".to_string(),
        }))?;
    let provenance: Provenance = serde_json::from_value(provenance_json)
        .map_err(|e| CaliberError::Storage(StorageError::TransactionFailed {
            reason: format!("Failed to deserialize provenance: {}", e),
        }))?;
    
    let ttl_str = extract_text(tuple, tuple_desc, artifact::TTL)?
        .ok_or_else(|| CaliberError::Storage(StorageError::TransactionFailed {
            reason: "ttl is NULL".to_string(),
        }))?;
    let ttl = str_to_ttl(&ttl_str);
    
    let created_at_ts = extract_timestamp(tuple, tuple_desc, artifact::CREATED_AT)?
        .ok_or_else(|| CaliberError::Storage(StorageError::TransactionFailed {
            reason: "created_at is NULL".to_string(),
        }))?;
    let created_at = timestamp_to_chrono(created_at_ts);
    
    let updated_at_ts = extract_timestamp(tuple, tuple_desc, artifact::UPDATED_AT)?
        .ok_or_else(|| CaliberError::Storage(StorageError::TransactionFailed {
            reason: "updated_at is NULL".to_string(),
        }))?;
    let updated_at = timestamp_to_chrono(updated_at_ts);
    
    let superseded_by = extract_uuid(tuple, tuple_desc, artifact::SUPERSEDED_BY)?.map(ArtifactId::new);

    let metadata = extract_jsonb(tuple, tuple_desc, artifact::METADATA)?;
    let tenant_id = extract_uuid(tuple, tuple_desc, artifact::TENANT_ID)?.map(TenantId::new);

    Ok(ArtifactRow {
        artifact: Artifact {
            artifact_id: ArtifactId::new(artifact_id),
            trajectory_id: TrajectoryId::new(trajectory_id),
            scope_id: ScopeId::new(scope_id),
            artifact_type,
            name,
            content,
            content_hash,
            embedding,
            provenance,
            ttl,
            created_at,
            updated_at,
            superseded_by,
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
    // Test Helpers - Generators for Artifact data
    // ========================================================================

    /// Generate a valid artifact name
    fn arb_artifact_name() -> impl Strategy<Value = String> {
        "[a-zA-Z][a-zA-Z0-9_-]{0,63}".prop_map(|s| s.trim().to_string())
            .prop_filter("name must not be empty", |s| !s.is_empty())
    }

    /// Generate artifact content
    fn arb_content() -> impl Strategy<Value = String> {
        "[a-zA-Z0-9 .,!?\\n-]{1,500}".prop_map(|s| s)
    }

    /// Generate an artifact type
    fn arb_artifact_type() -> impl Strategy<Value = ArtifactType> {
        prop_oneof![
            Just(ArtifactType::Code),
            Just(ArtifactType::Document),
            Just(ArtifactType::Data),
            Just(ArtifactType::Config),
            Just(ArtifactType::Log),
            Just(ArtifactType::Summary),
            Just(ArtifactType::Decision),
            Just(ArtifactType::Plan),
        ]
    }

    /// Generate a TTL value
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

    /// Generate provenance
    fn arb_provenance() -> impl Strategy<Value = Provenance> {
        (0i32..100i32, any::<Option<f32>>().prop_map(|o| o.map(|f| f.abs() % 1.0)))
            .prop_map(|(source_turn, confidence)| Provenance {
                source_turn,
                extraction_method: ExtractionMethod::Explicit,
                confidence,
            })
    }

    /// Generate optional embedding
    fn arb_embedding() -> impl Strategy<Value = Option<EmbeddingVector>> {
        prop_oneof![
            Just(None),
            proptest::collection::vec(any::<f32>().prop_map(|f| f % 1.0), 8..33)
                .prop_map(|data| Some(EmbeddingVector::new(data, "test".to_string()))),
        ]
    }

    // ========================================================================
    // Property 1: Insert-Get Round Trip (Artifact)
    // Feature: caliber-pg-hot-path, Property 1: Insert-Get Round Trip
    // Validates: Requirements 3.1, 3.2
    // ========================================================================

    #[cfg(feature = "pg_test")]
    mod pg_tests {
        use super::*;
        use crate::pg_test;

        /// Property 1: Insert-Get Round Trip (Artifact)
        /// 
        /// *For any* valid artifact data, inserting via direct heap then getting
        /// via direct heap SHALL return an equivalent artifact.
        ///
        /// **Validates: Requirements 3.1, 3.2**
    #[pg_test]
    fn prop_artifact_insert_get_roundtrip() {
        use proptest::test_runner::{TestRunner, Config};

            let config = Config::with_cases(50);
            let mut runner = TestRunner::new(config);

            let strategy = (
                arb_artifact_name(),
                arb_content(),
                arb_artifact_type(),
                arb_ttl(),
                arb_provenance(),
            );

            runner.run(&strategy, |(name, content, artifact_type, ttl, provenance)| {
                // Create trajectory and scope first
                let trajectory_id = TrajectoryId::now_v7();
                let tenant_id = TenantId::now_v7();
                let _ = crate::trajectory_heap::trajectory_create_heap(
                    trajectory_id,
                    "test_trajectory",
                    None,
                    None,
                    tenant_id,
                );

                let scope_id = ScopeId::now_v7();
                let _ = crate::scope_heap::scope_create_heap(
                    scope_id,
                    trajectory_id,
                    "test_scope",
                    None,
                    10000,
                    tenant_id,
                );

                // Generate artifact ID and content hash
                let artifact_id = ArtifactId::now_v7();
                let content_hash = caliber_core::compute_content_hash(content.as_bytes());

                // Insert via heap
                let result = artifact_create_heap(ArtifactCreateParams {
                    artifact_id,
                    trajectory_id,
                    scope_id,
                    artifact_type,
                    name: &name,
                    content: &content,
                    content_hash,
                    embedding: None, // No embedding for basic test
                    provenance: &provenance,
                    ttl: ttl.clone(),
                    tenant_id,
                });
                prop_assert!(result.is_ok(), "Insert should succeed");
                prop_assert_eq!(result.unwrap(), artifact_id);

                // Get via heap
                let get_result = artifact_get_heap(artifact_id, tenant_id);
                prop_assert!(get_result.is_ok(), "Get should succeed");
                
                let artifact = get_result.unwrap();
                prop_assert!(artifact.is_some(), "Artifact should be found");
                
                let row = artifact.unwrap();
                let a = row.artifact;
                
                // Verify round-trip preserves data
                prop_assert_eq!(a.artifact_id.as_uuid(), artifact_id.as_uuid());
                prop_assert_eq!(a.trajectory_id.as_uuid(), trajectory_id.as_uuid());
                prop_assert_eq!(a.scope_id.as_uuid(), scope_id.as_uuid());
                prop_assert_eq!(a.artifact_type, artifact_type);
                prop_assert_eq!(a.name, name);
                prop_assert_eq!(a.content, content);
                prop_assert_eq!(a.content_hash, content_hash);
                prop_assert_eq!(a.ttl, ttl);
                prop_assert!(a.embedding.is_none());
                prop_assert!(a.superseded_by.is_none());
                prop_assert!(a.metadata.is_none());
                prop_assert_eq!(row.tenant_id.map(|t| t.as_uuid()), Some(tenant_id.as_uuid()));

                Ok(())
            }).unwrap();
        }

        /// Property 1 (edge case): Get non-existent artifact returns None
        #[pg_test]
        fn prop_artifact_get_nonexistent_returns_none() {
            use proptest::test_runner::{TestRunner, Config};

            let config = Config::with_cases(100);
            let mut runner = TestRunner::new(config);

            runner.run(&any::<[u8; 16]>(), |bytes| {
                let random_id = ArtifactId::new(uuid::Uuid::from_bytes(bytes));

                let tenant_id = TenantId::now_v7();
                let result = artifact_get_heap(random_id, tenant_id);
                prop_assert!(result.is_ok(), "Get should not error");
                prop_assert!(result.unwrap().is_none(), "Non-existent artifact should return None");

                Ok(())
            }).unwrap();
        }

        /// Property: Insert-Get Round Trip with Embedding + Vector Search
        #[pg_test]
        fn prop_artifact_insert_get_roundtrip_with_embedding_and_search() {
            use proptest::test_runner::{TestRunner, Config};

            let config = Config::with_cases(20);
            let mut runner = TestRunner::new(config);

            let strategy = (
                arb_artifact_name(),
                arb_content(),
                arb_artifact_type(),
                arb_ttl(),
                arb_provenance(),
                arb_embedding().prop_filter_map("embedding required", |e| e),
            );

            runner.run(&strategy, |(name, content, artifact_type, ttl, provenance, embedding)| {
                crate::caliber_debug_clear();

                // Create trajectory and scope first
                let trajectory_id = TrajectoryId::now_v7();
                let tenant_id = TenantId::now_v7();
                let _ = crate::trajectory_heap::trajectory_create_heap(
                    trajectory_id,
                    "test_trajectory",
                    None,
                    None,
                    tenant_id,
                );

                let scope_id = ScopeId::now_v7();
                let _ = crate::scope_heap::scope_create_heap(
                    scope_id,
                    trajectory_id,
                    "test_scope",
                    None,
                    10000,
                    tenant_id,
                );

                // Generate artifact ID and content hash
                let artifact_id = ArtifactId::now_v7();
                let content_hash = caliber_core::compute_content_hash(content.as_bytes());

                // Insert via heap with embedding
                let result = artifact_create_heap(ArtifactCreateParams {
                    artifact_id,
                    trajectory_id,
                    scope_id,
                    artifact_type,
                    name: &name,
                    content: &content,
                    content_hash,
                    embedding: Some(&embedding),
                    provenance: &provenance,
                    ttl: ttl.clone(),
                    tenant_id,
                });
                prop_assert!(result.is_ok(), "Insert should succeed");
                prop_assert_eq!(result.unwrap(), artifact_id);

                // Get via heap
                let get_result = artifact_get_heap(artifact_id, tenant_id);
                prop_assert!(get_result.is_ok(), "Get should succeed");

                let artifact = get_result.unwrap();
                prop_assert!(artifact.is_some(), "Artifact should be found");

                let row = artifact.unwrap();
                let a = row.artifact;

                prop_assert!(a.embedding.is_some());
                let stored_embedding = a.embedding.unwrap();
                prop_assert_eq!(stored_embedding.data.len(), embedding.data.len());
                prop_assert!(stored_embedding.is_valid());

                // Vector search should return the artifact
                let query = serde_json::json!(embedding.data);
                let search = crate::caliber_vector_search(pgrx::JsonB(query), 10);
                let results: Vec<serde_json::Value> = serde_json::from_value(search.0)
                    .unwrap_or_default();
                let contains_artifact = results.iter().any(|row| {
                    row.get("entity_type") == Some(&serde_json::Value::String("artifact".to_string()))
                        && row.get("entity_id") == Some(&serde_json::Value::String(artifact_id.as_uuid().to_string()))
                });
                prop_assert!(contains_artifact);

                Ok(())
            }).unwrap();
        }
    }


    // ========================================================================
    // Property 2: Query by Type and Scope (Artifact)
    // Feature: caliber-pg-hot-path, Property 3: Index Consistency
    // Validates: Requirements 3.3, 3.4
    // ========================================================================

    #[cfg(feature = "pg_test")]
    mod query_tests {
        use super::*;
        use caliber_core::{ArtifactId, EntityIdType, ScopeId, TenantId, TrajectoryId};
        use crate::pg_test;

        /// Property 3: Query by type returns all artifacts of that type
        ///
        /// **Validates: Requirements 3.3**
        #[pg_test]
        fn prop_artifact_query_by_type() {
            use proptest::test_runner::{TestRunner, Config};

            let config = Config::with_cases(30);
            let mut runner = TestRunner::new(config);

            let strategy = (1usize..4usize, arb_artifact_type());

            runner.run(&strategy, |(num_artifacts, artifact_type)| {
                // Create trajectory and scope
                let trajectory_id = TrajectoryId::now_v7();
                let tenant_id = TenantId::now_v7();
                let _ = crate::trajectory_heap::trajectory_create_heap(
                    trajectory_id,
                    "test_trajectory",
                    None,
                    None,
                    tenant_id,
                );

                let scope_id = ScopeId::now_v7();
                let _ = crate::scope_heap::scope_create_heap(
                    scope_id,
                    trajectory_id,
                    "test_scope",
                    None,
                    10000,
                    tenant_id,
                );

                // Create multiple artifacts of the same type
                let mut artifact_ids = Vec::new();
                for i in 0..num_artifacts {
                    let artifact_id = ArtifactId::now_v7();
                    let content = format!("content_{}", i);
                    let content_hash = caliber_core::compute_content_hash(content.as_bytes());
                    let provenance = Provenance {
                        source_turn: i as i32,
                        extraction_method: ExtractionMethod::Explicit,
                        confidence: None,
                    };
                    
                    let _ = artifact_create_heap(ArtifactCreateParams {
                        artifact_id,
                        trajectory_id,
                        scope_id,
                        artifact_type,
                        name: &format!("artifact_{}", i),
                        content: &content,
                        content_hash,
                        embedding: None,
                        provenance: &provenance,
                        ttl: TTL::MediumTerm,
                        tenant_id,
                    });
                    artifact_ids.push(artifact_id);
                }

                // Query by type
                let query_result = artifact_query_by_type_heap(artifact_type, tenant_id);
                prop_assert!(query_result.is_ok(), "Query should succeed");
                
                let artifacts = query_result.unwrap();
                
                // All created artifacts should be in result
                for artifact_id in &artifact_ids {
                    prop_assert!(
                        artifacts.iter().any(|a| a.artifact.artifact_id.as_uuid() == artifact_id.as_uuid()),
                        "All created artifacts should be in result"
                    );
                }

                // All results should have correct type
                for row in &artifacts {
                    prop_assert_eq!(row.artifact.artifact_type, artifact_type);
                    prop_assert_eq!(row.tenant_id.map(|t| t.as_uuid()), Some(tenant_id.as_uuid()));
                }

                Ok(())
            }).unwrap();
        }

        /// Property 3: Query by scope returns all artifacts in that scope
        ///
        /// **Validates: Requirements 3.4**
        #[pg_test]
        fn prop_artifact_query_by_scope() {
            use proptest::test_runner::{TestRunner, Config};

            let config = Config::with_cases(30);
            let mut runner = TestRunner::new(config);

            let strategy = 1usize..4usize;

            runner.run(&strategy, |num_artifacts| {
                // Create trajectory and scope
                let trajectory_id = TrajectoryId::now_v7();
                let tenant_id = TenantId::now_v7();
                let _ = crate::trajectory_heap::trajectory_create_heap(
                    trajectory_id,
                    "test_trajectory",
                    None,
                    None,
                    tenant_id,
                );

                let scope_id = ScopeId::now_v7();
                let _ = crate::scope_heap::scope_create_heap(
                    scope_id,
                    trajectory_id,
                    "test_scope",
                    None,
                    10000,
                    tenant_id,
                );

                // Create multiple artifacts in the scope
                let mut artifact_ids = Vec::new();
                for i in 0..num_artifacts {
                    let artifact_id = ArtifactId::now_v7();
                    let content = format!("content_{}", i);
                    let content_hash = caliber_core::compute_content_hash(content.as_bytes());
                    let provenance = Provenance {
                        source_turn: i as i32,
                        extraction_method: ExtractionMethod::Explicit,
                        confidence: None,
                    };
                    
                    let _ = artifact_create_heap(ArtifactCreateParams {
                        artifact_id,
                        trajectory_id,
                        scope_id,
                        artifact_type: ArtifactType::Document,
                        name: &format!("artifact_{}", i),
                        content: &content,
                        content_hash,
                        embedding: None,
                        provenance: &provenance,
                        ttl: TTL::MediumTerm,
                        tenant_id,
                    });
                    artifact_ids.push(artifact_id);
                }

                // Query by scope
                let query_result = artifact_query_by_scope_heap(scope_id, tenant_id);
                prop_assert!(query_result.is_ok(), "Query should succeed");
                
                let artifacts = query_result.unwrap();
                prop_assert_eq!(artifacts.len(), num_artifacts, "Should return all artifacts");

                // All results should have correct scope_id
                for row in &artifacts {
                    prop_assert_eq!(row.artifact.scope_id.as_uuid(), scope_id.as_uuid());
                    prop_assert_eq!(row.tenant_id.map(|t| t.as_uuid()), Some(tenant_id.as_uuid()));
                }

                // All created artifacts should be in result
                for artifact_id in &artifact_ids {
                    prop_assert!(
                        artifacts.iter().any(|a| a.artifact.artifact_id == *artifact_id),
                        "All created artifacts should be in result"
                    );
                }

                Ok(())
            }).unwrap();
        }

        /// Query by non-existent scope returns empty
        #[pg_test]
        fn prop_artifact_query_by_nonexistent_scope_returns_empty() {
            use proptest::test_runner::{TestRunner, Config};

            let config = Config::with_cases(100);
            let mut runner = TestRunner::new(config);

            runner.run(&any::<[u8; 16]>(), |bytes| {
                let random_id = ScopeId::new(uuid::Uuid::from_bytes(bytes));

                let tenant_id = TenantId::now_v7();
                let result = artifact_query_by_scope_heap(random_id, tenant_id);
                prop_assert!(result.is_ok(), "Query should not error");
                prop_assert!(result.unwrap().is_empty(), "Query for non-existent scope should be empty");

                Ok(())
            }).unwrap();
        }
    }

    // ========================================================================
    // Property 2: Update Persistence (Artifact)
    // Feature: caliber-pg-hot-path, Property 2: Update Persistence
    // Validates: Requirements 3.5
    // ========================================================================

    #[cfg(feature = "pg_test")]
    mod update_tests {
        use super::*;
        use crate::pg_test;

        /// Property 2: Update content persists
        ///
        /// **Validates: Requirements 3.5**
        #[pg_test]
        fn prop_artifact_update_content_persists() {
            use proptest::test_runner::{TestRunner, Config};

            let config = Config::with_cases(50);
            let mut runner = TestRunner::new(config);

            let strategy = (arb_content(), arb_content());

            runner.run(&strategy, |(original_content, new_content)| {
                // Create trajectory and scope
                let trajectory_id = TrajectoryId::now_v7();
                let tenant_id = TenantId::now_v7();
                let _ = crate::trajectory_heap::trajectory_create_heap(
                    trajectory_id,
                    "test_trajectory",
                    None,
                    None,
                    tenant_id,
                );

                let scope_id = ScopeId::now_v7();
                let _ = crate::scope_heap::scope_create_heap(
                    scope_id,
                    trajectory_id,
                    "test_scope",
                    None,
                    10000,
                    tenant_id,
                );

                // Create artifact
                let artifact_id = ArtifactId::now_v7();
                let original_hash = caliber_core::compute_content_hash(original_content.as_bytes());
                let provenance = Provenance {
                    source_turn: 0,
                    extraction_method: ExtractionMethod::Explicit,
                    confidence: None,
                };
                
                let _ = artifact_create_heap(ArtifactCreateParams {
                    artifact_id,
                    trajectory_id,
                    scope_id,
                    artifact_type: ArtifactType::Document,
                    name: "test_artifact",
                    content: &original_content,
                    content_hash: original_hash,
                    embedding: None,
                    provenance: &provenance,
                    ttl: TTL::MediumTerm,
                    tenant_id,
                });

                // Update content
                let new_hash = caliber_core::compute_content_hash(new_content.as_bytes());
                let update_result = artifact_update_heap(
                    artifact_id,
                    Some(&new_content),
                    Some(new_hash),
                    None,
                    None,
                    None,
                    tenant_id,
                );
                prop_assert!(update_result.is_ok());
                prop_assert!(update_result.unwrap(), "Update should find the artifact");

                // Verify updated
                let after = artifact_get_heap(artifact_id, tenant_id).unwrap().unwrap();
                prop_assert_eq!(after.artifact.content, new_content, "Content should be updated");
                prop_assert_eq!(after.artifact.content_hash, new_hash, "Content hash should be updated");
                prop_assert_eq!(after.tenant_id.map(|t| t.as_uuid()), Some(tenant_id.as_uuid()));

                Ok(())
            }).unwrap();
        }

        /// Update non-existent artifact returns false
        #[pg_test]
        fn prop_artifact_update_nonexistent_returns_false() {
            use proptest::test_runner::{TestRunner, Config};

            let config = Config::with_cases(100);
            let mut runner = TestRunner::new(config);

            runner.run(&any::<[u8; 16]>(), |bytes| {
                let random_id = ArtifactId::new(uuid::Uuid::from_bytes(bytes));

                let result = artifact_update_heap(
                    random_id,
                    Some("new content"),
                    None,
                    None,
                    None,
                    None,
                    TenantId::now_v7(),
                );
                prop_assert!(result.is_ok(), "Update should not error");
                prop_assert!(!result.unwrap(), "Update of non-existent artifact should return false");

                Ok(())
            }).unwrap();
        }
    }
}
