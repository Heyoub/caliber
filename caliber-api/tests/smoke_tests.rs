//! End-to-end smoke tests for CALIBER API

use caliber_api::{ApiResult, DbClient, DbConfig};
use caliber_core::*;
use caliber_test_utils::*;

fn test_db() -> ApiResult<DbClient> {
    let config = DbConfig::from_env();
    DbClient::from_config(&config)
}

#[tokio::test]
#[cfg(feature = "db-tests")]
async fn smoke_test_full_crud_chain() -> ApiResult<()> {
    let db = test_db()?;
    let tenant_id = tenant_id_default();

    // Create trajectory
    let trajectory = db
        .trajectory_create(
            &CreateTrajectoryRequest {
                name: "smoke-test-trajectory".to_string(),
                description: Some("End-to-end CRUD test".to_string()),
                agent_id: None,
                parent_trajectory_id: None,
            },
            tenant_id,
        )
        .await?;

    assert_eq!(trajectory.status, TrajectoryStatus::Active);

    // Create scope
    let scope = db
        .scope_create(
            &CreateScopeRequest {
                trajectory_id: trajectory.trajectory_id,
                name: "smoke-test-scope".to_string(),
                purpose: Some("Testing".to_string()),
                parent_scope_id: None,
                token_budget: 8000,
            },
            tenant_id,
        )
        .await?;

    assert!(scope.is_active);
    assert_eq!(scope.trajectory_id, trajectory.trajectory_id);

    // Create artifact
    let artifact = db
        .artifact_create(
            &CreateArtifactRequest {
                trajectory_id: trajectory.trajectory_id,
                scope_id: scope.scope_id,
                artifact_type: ArtifactType::Fact,
                name: "smoke-test-artifact".to_string(),
                content: "Test content".to_string(),
                provenance: Provenance {
                    source_turn: 1,
                    extraction_method: ExtractionMethod::Explicit,
                    confidence: Some(1.0),
                },
                ttl: TTL::Persistent,
                embedding: None,
            },
            tenant_id,
        )
        .await?;

    assert_eq!(artifact.scope_id, scope.scope_id);

    // Create note
    let note = db
        .note_create(
            &CreateNoteRequest {
                note_type: NoteType::Fact,
                title: "Smoke Test Note".to_string(),
                content: "Test note".to_string(),
                source_trajectory_ids: vec![trajectory.trajectory_id],
                source_artifact_ids: vec![artifact.artifact_id],
                ttl: TTL::Persistent,
                embedding: None,
                abstraction_level: AbstractionLevel::Raw,
            },
            tenant_id,
        )
        .await?;

    assert!(note
        .source_trajectory_ids
        .contains(&trajectory.trajectory_id));

    // Verify retrieval
    db.trajectory_get(trajectory.trajectory_id, tenant_id)
        .await?;
    db.scope_get(scope.scope_id, tenant_id).await?;
    db.artifact_get(artifact.artifact_id, tenant_id).await?;
    db.note_get(note.note_id, tenant_id).await?;

    println!("✅ Full CRUD chain passed");
    Ok(())
}

#[tokio::test]
#[cfg(feature = "db-tests")]
async fn smoke_test_extension_validation() -> ApiResult<()> {
    let db = test_db()?;
    let conn = db.get_conn().await?;

    let caliber_pg = conn
        .query_opt(
            "SELECT 1 FROM pg_extension WHERE extname = 'caliber_pg'",
            &[],
        )
        .await?;

    assert!(caliber_pg.is_some(), "caliber_pg must be installed");

    let pgvector = conn
        .query_opt("SELECT 1 FROM pg_extension WHERE extname = 'vector'", &[])
        .await?;

    assert!(pgvector.is_some(), "pgvector must be installed");

    println!("✅ Extensions validated");
    Ok(())
}
