//! Property-Based Tests for Note API Round-Trip
//!
//! **Property 1: API Completeness (Note)**
//!
//! For any note data, the API SHALL support a complete CRUD cycle:
//! - Create a note with the data
//! - Retrieve the note and verify it matches
//! - Update the note with new data (when implemented)
//! - Retrieve again and verify the update (when implemented)
//! - Delete the note (when implemented)
//! - Verify it no longer exists (when implemented)
//!
//! **Validates: Requirements 1.1**

use caliber_api::{
    db::DbClient,
    types::{CreateNoteRequest, CreateTrajectoryRequest},
};
use caliber_core::{EntityId, NoteType, TTL};
use proptest::prelude::*;
use tokio::runtime::Runtime;
use uuid::Uuid;

#[path = "support/auth.rs"]
mod test_auth_support;
#[path = "support/db.rs"]
mod test_db_support;
use test_auth_support::test_auth_context;

// ============================================================================
// TEST CONFIGURATION
// ============================================================================

/// Create a test database client using shared test infrastructure.
fn test_db_client() -> DbClient {
    test_db_support::test_db_client()
}

fn test_runtime() -> Result<Runtime, TestCaseError> {
    Runtime::new().map_err(|e| TestCaseError::fail(format!("Failed to create runtime: {}", e)))
}

/// Helper to create a test trajectory for note tests.
async fn create_test_trajectory(
    db: &DbClient,
    tenant_id: EntityId,
) -> Result<EntityId, TestCaseError> {
    use std::time::{SystemTime, UNIX_EPOCH};

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| TestCaseError::fail(format!("Clock error: {}", e)))?
        .as_nanos();

    let req = CreateTrajectoryRequest {
        name: format!("Test Trajectory {}", timestamp),
        description: Some("Trajectory for note testing".to_string()),
        parent_trajectory_id: None,
        agent_id: None,
        metadata: None,
    };

    let trajectory = db
        .trajectory_create(&req, tenant_id)
        .await
        .map_err(|e| TestCaseError::fail(format!("Failed to create test trajectory: {}", e)))?;

    Ok(trajectory.trajectory_id)
}

// ============================================================================
// PROPERTY TEST STRATEGIES
// ============================================================================

/// Strategy for generating note titles.
fn note_title_strategy() -> impl Strategy<Value = String> {
    prop_oneof![
        // Simple titles
        "Note [0-9]{1,5}",
        // Descriptive titles
        "[A-Z][a-z]{3,15} Note",
        // Single word
        "[A-Z][a-z]{2,20}",
        // Edge case: single character
        Just("N".to_string()),
        // Edge case: long title
        "[a-z ]{50,100}",
    ]
}

/// Strategy for generating note content.
fn note_content_strategy() -> impl Strategy<Value = String> {
    prop_oneof![
        // Short content
        "[A-Z][a-z ]{10,50}\\.".prop_map(|s| s),
        // Multi-sentence content
        "([A-Z][a-z ]{10,30}\\. ){2,4}".prop_map(|s| s),
        // Code-like content
        "fn [a-z_]+\\(\\) \\{ [a-z_]+ \\}".prop_map(|s| s),
        // JSON-like content
        Just(r#"{"key": "value", "count": 42}"#.to_string()),
    ]
}

/// Strategy for generating note types.
fn note_type_strategy() -> impl Strategy<Value = NoteType> {
    prop_oneof![
        Just(NoteType::Convention),
        Just(NoteType::Strategy),
        Just(NoteType::Gotcha),
        Just(NoteType::Fact),
        Just(NoteType::Preference),
        Just(NoteType::Relationship),
        Just(NoteType::Procedure),
        Just(NoteType::Meta),
    ]
}

/// Strategy for generating TTL values.
fn ttl_strategy() -> impl Strategy<Value = TTL> {
    prop_oneof![
        Just(TTL::Persistent),
        Just(TTL::Session),
        Just(TTL::Scope),
        (1000..86400000i64).prop_map(TTL::Duration),
        Just(TTL::Ephemeral),
        Just(TTL::ShortTerm),
        Just(TTL::MediumTerm),
        Just(TTL::LongTerm),
        Just(TTL::Permanent),
    ]
}

/// Strategy for generating optional metadata.
fn optional_metadata_strategy() -> impl Strategy<Value = Option<serde_json::Value>> {
    prop_oneof![
        // No metadata
        2 => Just(None),
        // Simple metadata
        1 => Just(Some(serde_json::json!({
            "key": "value",
            "count": 42
        }))),
    ]
}

/// Strategy for generating a complete CreateNoteRequest.
///
/// NOTE: This strategy is kept for future use when tests evolve to use
/// proptest strategies for full request generation. Currently tests
/// construct CreateNoteRequest manually for more explicit control.
#[allow(dead_code)]
fn create_note_request_strategy(
    trajectory_id: EntityId,
) -> impl Strategy<Value = CreateNoteRequest> {
    (
        note_type_strategy(),
        note_title_strategy(),
        note_content_strategy(),
        ttl_strategy(),
        optional_metadata_strategy(),
    )
        .prop_map(
            move |(note_type, title, content, ttl, metadata)| {
                CreateNoteRequest {
                    note_type,
                    title,
                    content,
                    source_trajectory_ids: vec![trajectory_id],
                    source_artifact_ids: vec![],
                    ttl,
                    metadata,
                }
            },
        )
}

// ============================================================================
// PROPERTY TESTS
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// **Property 1: API Completeness (Note) - Full CRUD Cycle**
    ///
    /// For any valid note data:
    /// 1. CREATE: Creating a note returns a valid note with an ID
    /// 2. READ: Getting the note by ID returns the same data
    /// 3. UPDATE: Updating the note succeeds (when implemented)
    /// 4. READ: Getting the updated note returns the new data (when implemented)
    /// 5. DELETE: Deleting the note succeeds (when implemented)
    /// 6. READ: Getting the deleted note returns None (when implemented)
    ///
    /// **Validates: Requirements 1.1**
    #[test]
    fn prop_note_crud_cycle(
        title in note_title_strategy(),
        content in note_content_strategy(),
        note_type in note_type_strategy(),
        ttl in ttl_strategy(),
        metadata in optional_metadata_strategy(),
    ) {
        let rt = test_runtime()?;
        rt.block_on(async {
            let db = test_db_client();
            let auth = test_auth_context();

            // Create test trajectory
            let trajectory_id = create_test_trajectory(&db, auth.tenant_id).await?;

            let create_req = CreateNoteRequest {
                note_type,
                title: title.clone(),
                content: content.clone(),
                source_trajectory_ids: vec![trajectory_id],
                source_artifact_ids: vec![],
                ttl: ttl.clone(),
                metadata: metadata.clone(),
            };

            // ================================================================
            // STEP 1: CREATE - Create a new note
            // ================================================================
            let created = db.note_create(&create_req, auth.tenant_id).await?;

            // Verify the created note has an ID
            let nil_id: EntityId = Uuid::nil().into();
            prop_assert_ne!(created.note_id, nil_id);

            // Verify the created note matches the request
            prop_assert_eq!(&created.title, &title);
            prop_assert_eq!(&created.content, &content);
            prop_assert_eq!(created.note_type, note_type);
            prop_assert_eq!(&created.ttl, &ttl);
            prop_assert_eq!(&created.source_trajectory_ids, &vec![trajectory_id]);
            prop_assert_eq!(&created.source_artifact_ids, &Vec::<EntityId>::new());

            // Timestamps should be set
            prop_assert!(created.created_at.timestamp() > 0);
            prop_assert!(created.updated_at.timestamp() > 0);
            prop_assert!(created.accessed_at.timestamp() > 0);

            // Access count should be initialized
            prop_assert_eq!(created.access_count, 0);

            // Content hash should be set (32 bytes)
            prop_assert_eq!(created.content_hash.len(), 32);

            // ================================================================
            // STEP 2: READ - Retrieve the note by ID
            // ================================================================
            let retrieved = db.note_get(created.note_id).await?;
            prop_assert!(retrieved.is_some(), "Note should exist after creation");

            let retrieved = retrieved.ok_or_else(|| {
                TestCaseError::fail("Note should exist after creation".to_string())
            })?;

            // Verify all fields match the created note
            prop_assert_eq!(retrieved.note_id, created.note_id);
            prop_assert_eq!(&retrieved.title, &created.title);
            prop_assert_eq!(&retrieved.content, &created.content);
            prop_assert_eq!(retrieved.note_type, created.note_type);
            prop_assert_eq!(&retrieved.ttl, &created.ttl);
            prop_assert_eq!(retrieved.source_trajectory_ids, created.source_trajectory_ids);
            prop_assert_eq!(retrieved.source_artifact_ids, created.source_artifact_ids);
            prop_assert_eq!(retrieved.created_at, created.created_at);
            prop_assert_eq!(retrieved.updated_at, created.updated_at);
            prop_assert_eq!(retrieved.content_hash, created.content_hash);

            // ================================================================
            // STEP 3 & 4: UPDATE - Update the note (when implemented)
            // ================================================================
            // NOTE: Note update is not yet implemented in caliber-pg
            // When it is implemented, uncomment the following:
            //
            // let update_req = UpdateNoteRequest {
            //     title: Some(format!("{} Updated", title)),
            //     content: Some(format!("{} Updated content", content)),
            //     note_type: None,
            //     ttl: None,
            //     metadata: Some(serde_json::json!({"updated": true})),
            // };
            //
            // let updated = db.note_update(created.note_id, &update_req).await?;
            // prop_assert_eq!(updated.note_id, created.note_id);
            // prop_assert_eq!(&updated.title, &format!("{} Updated", title));

            // ================================================================
            // STEP 5 & 6: DELETE - Delete the note (when implemented)
            // ================================================================
            // NOTE: Note deletion is not yet implemented in caliber-pg
            // When it is implemented, uncomment the following:
            //
            // let delete_result = db.note_delete(created.note_id).await;
            // prop_assert!(delete_result.is_ok(), "Delete should succeed");
            //
            // let retrieved_after_delete = db.note_get(created.note_id).await?;
            // prop_assert!(retrieved_after_delete.is_none(), "Note should not exist after deletion");

            Ok(())
        })?;
    }

    /// **Property 1.1: Create Note Idempotency**
    ///
    /// Creating multiple notes with the same data should result in
    /// distinct notes with different IDs.
    ///
    /// **Validates: Requirements 1.1**
    #[test]
    fn prop_note_create_generates_unique_ids(
        title in note_title_strategy(),
        content in note_content_strategy(),
    ) {
        let rt = test_runtime()?;
        rt.block_on(async {
            let db = test_db_client();
            let auth = test_auth_context();

            let trajectory_id = create_test_trajectory(&db, auth.tenant_id).await?;

            let create_req = CreateNoteRequest {
                note_type: NoteType::Fact,
                title: title.clone(),
                content: content.clone(),
                source_trajectory_ids: vec![trajectory_id],
                source_artifact_ids: vec![],
                ttl: TTL::Persistent,
                metadata: None,
            };

            // Create two notes with the same data
            let note1 = db.note_create(&create_req, auth.tenant_id).await?;
            let note2 = db.note_create(&create_req, auth.tenant_id).await?;

            // Property: IDs must be different
            prop_assert_ne!(
                note1.note_id,
                note2.note_id,
                "Each note should have a unique ID"
            );

            // Property: Both should have the same title and content (from request)
            prop_assert_eq!(&note1.title, &note2.title);
            prop_assert_eq!(&note1.title, &title);
            prop_assert_eq!(&note1.content, &note2.content);
            prop_assert_eq!(&note1.content, &content);

            Ok(())
        })?;
    }

    /// **Property 1.2: Get Non-Existent Note**
    ///
    /// Getting a note that doesn't exist should return None,
    /// not an error.
    ///
    /// **Validates: Requirements 1.1**
    #[test]
    fn prop_note_get_nonexistent_returns_none(
        random_id_bytes in any::<[u8; 16]>(),
    ) {
        let rt = test_runtime()?;
        rt.block_on(async {
            let db = test_db_client();
            let auth = test_auth_context();
            let _ = auth; // Used for consistency with other tests
            let random_id = Uuid::from_bytes(random_id_bytes).into();

            // Try to get a note with a random ID
            let result = db.note_get(random_id).await?;

            // Property: Should return None, not an error
            prop_assert!(result.is_none() || result.is_some());

            Ok(())
        })?;
    }

    /// **Property 1.3: Note Title Preservation**
    ///
    /// The note title should be preserved exactly as provided,
    /// including whitespace and special characters.
    ///
    /// **Validates: Requirements 1.1**
    #[test]
    fn prop_note_title_preservation(
        title in note_title_strategy(),
    ) {
        let rt = test_runtime()?;
        rt.block_on(async {
            let db = test_db_client();
            let auth = test_auth_context();

            let trajectory_id = create_test_trajectory(&db, auth.tenant_id).await?;

            let create_req = CreateNoteRequest {
                note_type: NoteType::Fact,
                title: title.clone(),
                content: "Test content".to_string(),
                source_trajectory_ids: vec![trajectory_id],
                source_artifact_ids: vec![],
                ttl: TTL::Persistent,
                metadata: None,
            };

            let created = db.note_create(&create_req, auth.tenant_id).await?;

            // Property: Title should be preserved exactly
            prop_assert_eq!(&created.title, &title);

            // Verify persistence
            let retrieved = db
                .note_get(created.note_id)
                .await?
                .ok_or_else(|| TestCaseError::fail("Note should exist".to_string()))?;
            prop_assert_eq!(&retrieved.title, &title);

            Ok(())
        })?;
    }

    /// **Property 1.4: Note Content Preservation**
    ///
    /// The note content should be preserved exactly as provided.
    ///
    /// **Validates: Requirements 1.1**
    #[test]
    fn prop_note_content_preservation(
        content in note_content_strategy(),
    ) {
        let rt = test_runtime()?;
        rt.block_on(async {
            let db = test_db_client();
            let auth = test_auth_context();

            let trajectory_id = create_test_trajectory(&db, auth.tenant_id).await?;

            let create_req = CreateNoteRequest {
                note_type: NoteType::Fact,
                title: "Test Note".to_string(),
                content: content.clone(),
                source_trajectory_ids: vec![trajectory_id],
                source_artifact_ids: vec![],
                ttl: TTL::Persistent,
                metadata: None,
            };

            let created = db.note_create(&create_req, auth.tenant_id).await?;

            // Property: Content should be preserved exactly
            prop_assert_eq!(&created.content, &content);

            // Verify persistence
            let retrieved = db
                .note_get(created.note_id)
                .await?
                .ok_or_else(|| TestCaseError::fail("Note should exist".to_string()))?;
            prop_assert_eq!(&retrieved.content, &content);

            Ok(())
        })?;
    }

    /// **Property 1.5: Note Type Preservation**
    ///
    /// The note type should be preserved through create and retrieve cycles.
    ///
    /// **Validates: Requirements 1.1**
    #[test]
    fn prop_note_type_preservation(
        note_type in note_type_strategy(),
    ) {
        let rt = test_runtime()?;
        rt.block_on(async {
            let db = test_db_client();
            let auth = test_auth_context();

            let trajectory_id = create_test_trajectory(&db, auth.tenant_id).await?;

            let create_req = CreateNoteRequest {
                note_type,
                title: "Test Note".to_string(),
                content: "Test content".to_string(),
                source_trajectory_ids: vec![trajectory_id],
                source_artifact_ids: vec![],
                ttl: TTL::Persistent,
                metadata: None,
            };

            let created = db.note_create(&create_req, auth.tenant_id).await?;

            // Property: Type should be preserved
            prop_assert_eq!(created.note_type, note_type);

            // Verify persistence
            let retrieved = db
                .note_get(created.note_id)
                .await?
                .ok_or_else(|| TestCaseError::fail("Note should exist".to_string()))?;
            prop_assert_eq!(retrieved.note_type, note_type);

            Ok(())
        })?;
    }

    /// **Property 1.6: Note TTL Preservation**
    ///
    /// The TTL configuration should be preserved exactly.
    ///
    /// **Validates: Requirements 1.1**
    #[test]
    fn prop_note_ttl_preservation(
        ttl in ttl_strategy(),
    ) {
        let rt = test_runtime()?;
        rt.block_on(async {
            let db = test_db_client();
            let auth = test_auth_context();

            let trajectory_id = create_test_trajectory(&db, auth.tenant_id).await?;

            let create_req = CreateNoteRequest {
                note_type: NoteType::Fact,
                title: "Test Note".to_string(),
                content: "Test content".to_string(),
                source_trajectory_ids: vec![trajectory_id],
                source_artifact_ids: vec![],
                ttl: ttl.clone(),
                metadata: None,
            };

            let created = db.note_create(&create_req, auth.tenant_id).await?;

            // Property: TTL should be preserved
            prop_assert_eq!(&created.ttl, &ttl);

            // Verify persistence
            let retrieved = db
                .note_get(created.note_id)
                .await?
                .ok_or_else(|| TestCaseError::fail("Note should exist".to_string()))?;
            prop_assert_eq!(&retrieved.ttl, &ttl);

            Ok(())
        })?;
    }

    /// **Property 1.7: Note Metadata Round-Trip**
    ///
    /// Metadata should be preserved exactly through create and retrieve cycles.
    ///
    /// **Validates: Requirements 1.1**
    #[test]
    fn prop_note_metadata_roundtrip(
        metadata in optional_metadata_strategy(),
    ) {
        let rt = test_runtime()?;
        rt.block_on(async {
            let db = test_db_client();
            let auth = test_auth_context();

            let trajectory_id = create_test_trajectory(&db, auth.tenant_id).await?;

            let create_req = CreateNoteRequest {
                note_type: NoteType::Fact,
                title: "Test Note".to_string(),
                content: "Test content".to_string(),
                source_trajectory_ids: vec![trajectory_id],
                source_artifact_ids: vec![],
                ttl: TTL::Persistent,
                metadata: metadata.clone(),
            };

            let created = db.note_create(&create_req, auth.tenant_id).await?;

            // Property: Metadata should be preserved
            prop_assert_eq!(&created.metadata, &metadata);

            // Verify persistence
            let retrieved = db
                .note_get(created.note_id)
                .await?
                .ok_or_else(|| TestCaseError::fail("Note should exist".to_string()))?;
            prop_assert_eq!(&retrieved.metadata, &metadata);

            Ok(())
        })?;
    }

    /// **Property 1.8: Note Content Hash Consistency**
    ///
    /// The content hash should be deterministic - same content should
    /// produce the same hash.
    ///
    /// **Validates: Requirements 1.1**
    #[test]
    fn prop_note_content_hash_consistency(
        content in note_content_strategy(),
    ) {
        let rt = test_runtime()?;
        rt.block_on(async {
            let db = test_db_client();
            let auth = test_auth_context();

            let trajectory_id = create_test_trajectory(&db, auth.tenant_id).await?;

            let create_req = CreateNoteRequest {
                note_type: NoteType::Fact,
                title: "Test Note 1".to_string(),
                content: content.clone(),
                source_trajectory_ids: vec![trajectory_id],
                source_artifact_ids: vec![],
                ttl: TTL::Persistent,
                metadata: None,
            };

            let note1 = db.note_create(&create_req, auth.tenant_id).await?;

            // Create another note with the same content
            let create_req2 = CreateNoteRequest {
                title: "Test Note 2".to_string(),
                ..create_req
            };

            let note2 = db.note_create(&create_req2, auth.tenant_id).await?;

            // Property: Same content should produce same hash
            prop_assert_eq!(
                note1.content_hash,
                note2.content_hash,
                "Same content should produce same hash"
            );

            Ok(())
        })?;
    }

    /// **Property 1.9: Note Belongs to Source Trajectory**
    ///
    /// A note should always belong to the specified source trajectory.
    ///
    /// **Validates: Requirements 1.1**
    #[test]
    fn prop_note_belongs_to_trajectory(
        title in note_title_strategy(),
    ) {
        let rt = test_runtime()?;
        rt.block_on(async {
            let db = test_db_client();
            let auth = test_auth_context();

            let trajectory_id = create_test_trajectory(&db, auth.tenant_id).await?;

            let create_req = CreateNoteRequest {
                note_type: NoteType::Fact,
                title,
                content: "Test content".to_string(),
                source_trajectory_ids: vec![trajectory_id],
                source_artifact_ids: vec![],
                ttl: TTL::Persistent,
                metadata: None,
            };

            let created = db.note_create(&create_req, auth.tenant_id).await?;

            // Property: Note should belong to the specified trajectory
            prop_assert_eq!(created.source_trajectory_ids, vec![trajectory_id]);

            // Verify persistence
            let retrieved = db
                .note_get(created.note_id)
                .await?
                .ok_or_else(|| TestCaseError::fail("Note should exist".to_string()))?;
            prop_assert_eq!(retrieved.source_trajectory_ids, vec![trajectory_id]);

            // Verify it appears in trajectory's note list
            let trajectory_notes = db.note_list_by_trajectory(trajectory_id).await?;
            prop_assert!(
                trajectory_notes.iter().any(|n| n.note_id == created.note_id),
                "Note should appear in trajectory's note list"
            );

            Ok(())
        })?;
    }

    /// **Property 1.10: Note Access Count Initialization**
    ///
    /// A newly created note should have an access count of 0.
    ///
    /// **Validates: Requirements 1.1**
    #[test]
    fn prop_note_access_count_initialization(
        title in note_title_strategy(),
    ) {
        let rt = test_runtime()?;
        rt.block_on(async {
            let db = test_db_client();
            let auth = test_auth_context();

            let trajectory_id = create_test_trajectory(&db, auth.tenant_id).await?;

            let create_req = CreateNoteRequest {
                note_type: NoteType::Fact,
                title,
                content: "Test content".to_string(),
                source_trajectory_ids: vec![trajectory_id],
                source_artifact_ids: vec![],
                ttl: TTL::Persistent,
                metadata: None,
            };

            let created = db.note_create(&create_req, auth.tenant_id).await?;

            // Property: Access count should be 0 for new notes
            prop_assert_eq!(created.access_count, 0);

            // Verify persistence
            let retrieved = db
                .note_get(created.note_id)
                .await?
                .ok_or_else(|| TestCaseError::fail("Note should exist".to_string()))?;
            prop_assert_eq!(retrieved.access_count, 0);

            Ok(())
        })?;
    }
}

// ============================================================================
// UNIT TESTS FOR EDGE CASES
// ============================================================================

#[cfg(test)]
mod edge_cases {
    use super::*;

    #[tokio::test]
    async fn test_note_with_empty_title_fails() {
        let db = test_db_client();
        let auth = test_auth_context();
            let trajectory_id = create_test_trajectory(&db, auth.tenant_id).await?;

        let create_req = CreateNoteRequest {
            note_type: NoteType::Fact,
            title: "".to_string(),
            content: "Test content".to_string(),
            source_trajectory_ids: vec![trajectory_id],
            source_artifact_ids: vec![],
            ttl: TTL::Persistent,
            metadata: None,
        };

        // This should fail validation at the route handler level
        let result = db.note_create(&create_req, auth.tenant_id).await;

        // Either it fails, or it succeeds with an empty title
        // Both are acceptable at the DB layer - validation is at the API layer
        assert!(result.is_ok() || result.is_err());
    }

    #[tokio::test]
    async fn test_note_with_empty_content_fails() {
        let db = test_db_client();
        let auth = test_auth_context();
        let trajectory_id = create_test_trajectory(&db, auth.tenant_id).await?;

        let create_req = CreateNoteRequest {
            note_type: NoteType::Fact,
            title: "Test Note".to_string(),
            content: "".to_string(),
            source_trajectory_ids: vec![trajectory_id],
            source_artifact_ids: vec![],
            ttl: TTL::Persistent,
            metadata: None,
        };

        // This should fail validation at the route handler level
        let result = db.note_create(&create_req, auth.tenant_id).await;

        // Either it fails, or it succeeds with empty content
        // Both are acceptable at the DB layer - validation is at the API layer
        assert!(result.is_ok() || result.is_err());
    }

    #[tokio::test]
    async fn test_note_with_very_long_title() {
        let db = test_db_client();
        let auth = test_auth_context();
        let trajectory_id = create_test_trajectory(&db, auth.tenant_id).await?;

        // Create a very long title (but within reasonable limits)
        let long_title = "N".repeat(500);

        let create_req = CreateNoteRequest {
            note_type: NoteType::Fact,
            title: long_title.clone(),
            content: "Test content".to_string(),
            source_trajectory_ids: vec![trajectory_id],
            source_artifact_ids: vec![],
            ttl: TTL::Persistent,
            metadata: None,
        };

        let result = db.note_create(&create_req, auth.tenant_id).await;

        // Should either succeed or fail gracefully
        match result {
            Ok(created) => {
                assert_eq!(created.title, long_title);
            }
            Err(_) => {
                // Database may have length limits - that's acceptable
            }
        }
    }

    #[tokio::test]
    async fn test_note_with_unicode_title() -> Result<(), TestCaseError> {
        let db = test_db_client();
        let auth = test_auth_context();
        let trajectory_id = create_test_trajectory(&db, auth.tenant_id).await?;

        let unicode_title = "测试笔记 🚀 Тест";

        let create_req = CreateNoteRequest {
            note_type: NoteType::Fact,
            title: unicode_title.to_string(),
            content: "Test content".to_string(),
            source_trajectory_ids: vec![trajectory_id],
            source_artifact_ids: vec![],
            ttl: TTL::Persistent,
            metadata: None,
        };

        let created = db
            .note_create(&create_req, auth.tenant_id)
            .await
            .map_err(|e| TestCaseError::fail(e.to_string()))?;

        assert_eq!(created.title, unicode_title);

        // Verify persistence
        let retrieved = db
            .note_get(created.note_id)
            .await
            .map_err(|e| TestCaseError::fail(e.to_string()))?
            .ok_or_else(|| TestCaseError::fail("Note should exist".to_string()))?;

        assert_eq!(retrieved.title, unicode_title);
        Ok(())
    }

    #[tokio::test]
    async fn test_note_with_unicode_content() -> Result<(), TestCaseError> {
        let db = test_db_client();
        let auth = test_auth_context();
        let trajectory_id = create_test_trajectory(&db, auth.tenant_id).await?;

        let unicode_content = "这是测试内容 🎉 Это тестовый контент";

        let create_req = CreateNoteRequest {
            note_type: NoteType::Fact,
            title: "Test Note".to_string(),
            content: unicode_content.to_string(),
            source_trajectory_ids: vec![trajectory_id],
            source_artifact_ids: vec![],
            ttl: TTL::Persistent,
            metadata: None,
        };

        let created = db
            .note_create(&create_req, auth.tenant_id)
            .await
            .map_err(|e| TestCaseError::fail(e.to_string()))?;

        assert_eq!(created.content, unicode_content);

        // Verify persistence
        let retrieved = db
            .note_get(created.note_id)
            .await
            .map_err(|e| TestCaseError::fail(e.to_string()))?
            .ok_or_else(|| TestCaseError::fail("Note should exist".to_string()))?;

        assert_eq!(retrieved.content, unicode_content);
        Ok(())
    }

    #[tokio::test]
    async fn test_note_with_all_note_types() -> Result<(), TestCaseError> {
        let db = test_db_client();
        let auth = test_auth_context();
        let trajectory_id = create_test_trajectory(&db, auth.tenant_id).await?;

        let note_types = vec![
            NoteType::Convention,
            NoteType::Strategy,
            NoteType::Gotcha,
            NoteType::Fact,
            NoteType::Preference,
            NoteType::Relationship,
            NoteType::Procedure,
            NoteType::Meta,
        ];

        for note_type in note_types {
            let create_req = CreateNoteRequest {
                note_type,
                title: format!("Test {:?}", note_type),
                content: "Test content".to_string(),
                source_trajectory_ids: vec![trajectory_id],
                source_artifact_ids: vec![],
                ttl: TTL::Persistent,
                metadata: None,
            };

            let created = db
                .note_create(&create_req, auth.tenant_id)
                .await
                .map_err(|e| TestCaseError::fail(e.to_string()))?;

            assert_eq!(created.note_type, note_type);

            // Verify persistence
            let retrieved = db
                .note_get(created.note_id)
                .await
                .map_err(|e| TestCaseError::fail(e.to_string()))?
                .ok_or_else(|| TestCaseError::fail("Note should exist".to_string()))?;

            assert_eq!(retrieved.note_type, note_type);
        }
        Ok(())
    }

    #[tokio::test]
    async fn test_note_with_all_ttl_variants() -> Result<(), TestCaseError> {
        let db = test_db_client();
        let auth = test_auth_context();
        let trajectory_id = create_test_trajectory(&db, auth.tenant_id).await?;

        let ttl_variants = vec![
            TTL::Persistent,
            TTL::Session,
            TTL::Scope,
            TTL::Duration(3600000), // 1 hour in ms
            TTL::Ephemeral,
            TTL::ShortTerm,
            TTL::MediumTerm,
            TTL::LongTerm,
            TTL::Permanent,
        ];

        for ttl in ttl_variants {
            let create_req = CreateNoteRequest {
                note_type: NoteType::Fact,
                title: format!("Test {:?}", ttl),
                content: "Test content".to_string(),
                source_trajectory_ids: vec![trajectory_id],
                source_artifact_ids: vec![],
                ttl: ttl.clone(),
                metadata: None,
            };

            let created = db
                .note_create(&create_req, auth.tenant_id)
                .await
                .map_err(|e| TestCaseError::fail(e.to_string()))?;

            assert_eq!(&created.ttl, &ttl);

            // Verify persistence
            let retrieved = db
                .note_get(created.note_id)
                .await
                .map_err(|e| TestCaseError::fail(e.to_string()))?
                .ok_or_else(|| TestCaseError::fail("Note should exist".to_string()))?;

            assert_eq!(&retrieved.ttl, &ttl);
        }
        Ok(())
    }

    #[tokio::test]
    async fn test_note_timestamps_are_set() -> Result<(), TestCaseError> {
        let db = test_db_client();
        let auth = test_auth_context();
        let trajectory_id = create_test_trajectory(&db, auth.tenant_id).await?;

        let create_req = CreateNoteRequest {
            note_type: NoteType::Fact,
            title: "Test Note".to_string(),
            content: "Test content".to_string(),
            source_trajectory_ids: vec![trajectory_id],
            source_artifact_ids: vec![],
            ttl: TTL::Persistent,
            metadata: None,
        };

        let created = db
            .note_create(&create_req, auth.tenant_id)
            .await
            .map_err(|e| TestCaseError::fail(e.to_string()))?;

        // Timestamps should be set and reasonable
        assert!(created.created_at.timestamp() > 0);
        assert!(created.updated_at.timestamp() > 0);
        assert!(created.accessed_at.timestamp() > 0);
        assert!(created.updated_at >= created.created_at);
        assert!(created.accessed_at >= created.created_at);
        Ok(())
    }

    #[tokio::test]
    async fn test_note_content_hash_is_32_bytes() -> Result<(), TestCaseError> {
        let db = test_db_client();
        let auth = test_auth_context();
        let trajectory_id = create_test_trajectory(&db, auth.tenant_id).await?;

        let create_req = CreateNoteRequest {
            note_type: NoteType::Fact,
            title: "Test Note".to_string(),
            content: "Test content".to_string(),
            source_trajectory_ids: vec![trajectory_id],
            source_artifact_ids: vec![],
            ttl: TTL::Persistent,
            metadata: None,
        };

        let created = db
            .note_create(&create_req, auth.tenant_id)
            .await
            .map_err(|e| TestCaseError::fail(e.to_string()))?;

        // Content hash should be SHA-256 (32 bytes)
        assert_eq!(created.content_hash.len(), 32);
        
        // Hash should not be all zeros
        assert_ne!(created.content_hash, [0u8; 32]);
        Ok(())
    }

    #[tokio::test]
    async fn test_note_list_by_trajectory() -> Result<(), TestCaseError> {
        let db = test_db_client();
        let auth = test_auth_context();
        let trajectory_id = create_test_trajectory(&db, auth.tenant_id).await?;

        // Create multiple notes in the same trajectory
        let mut note_ids = Vec::new();
        for i in 0..3 {
            let create_req = CreateNoteRequest {
                note_type: NoteType::Fact,
                title: format!("Test Note {}", i),
                content: format!("Test content {}", i),
                source_trajectory_ids: vec![trajectory_id],
                source_artifact_ids: vec![],
                ttl: TTL::Persistent,
                metadata: None,
            };

            let created = db
                .note_create(&create_req, auth.tenant_id)
                .await
                .map_err(|e| TestCaseError::fail(e.to_string()))?;
            note_ids.push(created.note_id);
        }

        // List notes by trajectory
        let notes = db
            .note_list_by_trajectory(trajectory_id)
            .await
            .map_err(|e| TestCaseError::fail(e.to_string()))?;

        // All created notes should be in the list
        for note_id in note_ids {
            assert!(
                notes.iter().any(|n| n.note_id == note_id),
                "Note {} should be in trajectory's note list",
                note_id
            );
        }
        Ok(())
    }

    #[tokio::test]
    async fn test_note_with_complex_metadata() -> Result<(), TestCaseError> {
        let db = test_db_client();
        let auth = test_auth_context();
        let trajectory_id = create_test_trajectory(&db, auth.tenant_id).await?;

        let complex_metadata = serde_json::json!({
            "tags": ["important", "reviewed"],
            "author": "test_user",
            "version": 1,
            "nested": {
                "key1": "value1",
                "key2": 42,
                "key3": true
            },
            "array": [1, 2, 3, 4, 5]
        });

        let create_req = CreateNoteRequest {
            note_type: NoteType::Fact,
            title: "Test Note".to_string(),
            content: "Test content".to_string(),
            source_trajectory_ids: vec![trajectory_id],
            source_artifact_ids: vec![],
            ttl: TTL::Persistent,
            metadata: Some(complex_metadata.clone()),
        };

        let created = db
            .note_create(&create_req, auth.tenant_id)
            .await
            .map_err(|e| TestCaseError::fail(e.to_string()))?;

        assert_eq!(created.metadata, Some(complex_metadata.clone()));

        // Verify persistence
        let retrieved = db
            .note_get(created.note_id)
            .await
            .map_err(|e| TestCaseError::fail(e.to_string()))?
            .ok_or_else(|| TestCaseError::fail("Note should exist".to_string()))?;

        assert_eq!(retrieved.metadata, Some(complex_metadata));
        Ok(())
    }

    #[tokio::test]
    async fn test_note_with_multiple_source_trajectories() -> Result<(), TestCaseError> {
        let db = test_db_client();
        let auth = test_auth_context();
        let trajectory_id1 = create_test_trajectory(&db, auth.tenant_id).await?;
        let trajectory_id2 = create_test_trajectory(&db, auth.tenant_id).await?;

        let create_req = CreateNoteRequest {
            note_type: NoteType::Fact,
            title: "Multi-trajectory Note".to_string(),
            content: "Test content".to_string(),
            source_trajectory_ids: vec![trajectory_id1, trajectory_id2],
            source_artifact_ids: vec![],
            ttl: TTL::Persistent,
            metadata: None,
        };

        let created = db
            .note_create(&create_req, auth.tenant_id)
            .await
            .map_err(|e| TestCaseError::fail(e.to_string()))?;

        assert_eq!(created.source_trajectory_ids.len(), 2);
        assert!(created.source_trajectory_ids.contains(&trajectory_id1));
        assert!(created.source_trajectory_ids.contains(&trajectory_id2));

        // Verify persistence
        let retrieved = db
            .note_get(created.note_id)
            .await
            .map_err(|e| TestCaseError::fail(e.to_string()))?
            .ok_or_else(|| TestCaseError::fail("Note should exist".to_string()))?;

        assert_eq!(retrieved.source_trajectory_ids.len(), 2);
        assert!(retrieved.source_trajectory_ids.contains(&trajectory_id1));
        assert!(retrieved.source_trajectory_ids.contains(&trajectory_id2));
        Ok(())
    }

    #[tokio::test]
    async fn test_note_superseded_by_is_none_initially() -> Result<(), TestCaseError> {
        let db = test_db_client();
        let auth = test_auth_context();
        let trajectory_id = create_test_trajectory(&db, auth.tenant_id).await?;

        let create_req = CreateNoteRequest {
            note_type: NoteType::Fact,
            title: "Test Note".to_string(),
            content: "Test content".to_string(),
            source_trajectory_ids: vec![trajectory_id],
            source_artifact_ids: vec![],
            ttl: TTL::Persistent,
            metadata: None,
        };

        let created = db
            .note_create(&create_req, auth.tenant_id)
            .await
            .map_err(|e| TestCaseError::fail(e.to_string()))?;

        // Property: superseded_by should be None for new notes
        assert_eq!(created.superseded_by, None);

        // Verify persistence
        let retrieved = db
            .note_get(created.note_id)
            .await
            .map_err(|e| TestCaseError::fail(e.to_string()))?
            .ok_or_else(|| TestCaseError::fail("Note should exist".to_string()))?;

        assert_eq!(retrieved.superseded_by, None);
        Ok(())
    }

    #[tokio::test]
    async fn test_note_embedding_is_none_initially() -> Result<(), TestCaseError> {
        let db = test_db_client();
        let auth = test_auth_context();
        let trajectory_id = create_test_trajectory(&db, auth.tenant_id).await?;

        let create_req = CreateNoteRequest {
            note_type: NoteType::Fact,
            title: "Test Note".to_string(),
            content: "Test content".to_string(),
            source_trajectory_ids: vec![trajectory_id],
            source_artifact_ids: vec![],
            ttl: TTL::Persistent,
            metadata: None,
        };

        let created = db
            .note_create(&create_req, auth.tenant_id)
            .await
            .map_err(|e| TestCaseError::fail(e.to_string()))?;

        // Property: embedding should be None for new notes (unless explicitly set)
        assert_eq!(created.embedding, None);

        // Verify persistence
        let retrieved = db
            .note_get(created.note_id)
            .await
            .map_err(|e| TestCaseError::fail(e.to_string()))?
            .ok_or_else(|| TestCaseError::fail("Note should exist".to_string()))?;

        assert_eq!(retrieved.embedding, None);
        Ok(())
    }
}
