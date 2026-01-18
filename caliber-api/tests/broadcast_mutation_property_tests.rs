//! Property-Based Tests for Mutation Broadcast
//!
//! **Property 3: Mutation Broadcast**
//!
//! For any entity mutation (create), the API SHALL broadcast a corresponding
//! WebSocket event to connected clients.
//!
//! **Validates: Requirements 1.4**

use axum::{extract::State, Json};
use caliber_api::events::WsEvent;
use caliber_api::middleware::AuthExtractor;
use caliber_api::routes::{artifact, note, scope, trajectory, turn};
use caliber_api::types::{
    CreateArtifactRequest, CreateNoteRequest, CreateScopeRequest, CreateTrajectoryRequest,
    CreateTurnRequest,
};
use caliber_core::{ArtifactType, ExtractionMethod, NoteType, TTL, TurnRole};
use proptest::prelude::*;
use proptest::test_runner::TestCaseError;
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio::time::{timeout, Duration};

mod test_support;
use test_support::test_auth_context;

async fn recv_event(rx: &mut broadcast::Receiver<WsEvent>, label: &str) -> WsEvent {
    match timeout(Duration::from_millis(200), rx.recv()).await {
        Ok(Ok(event)) => event,
        Ok(Err(err)) => panic!("Broadcast recv error for {}: {:?}", label, err),
        Err(_) => panic!("Timed out waiting for broadcast event: {}", label),
    }
}

fn name_strategy() -> impl Strategy<Value = String> {
    "[a-zA-Z0-9 _-]{3,32}".prop_map(|s| s.trim().to_string())
}

fn content_strategy() -> impl Strategy<Value = String> {
    "[a-zA-Z0-9 .,;:_-]{10,120}".prop_map(|s| s.trim().to_string())
}

fn token_budget_strategy() -> impl Strategy<Value = i32> {
    1..10_000i32
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(50))]

    /// **Property 3: Mutation Broadcast**
    ///
    /// For any valid create requests, the API SHALL broadcast the corresponding
    /// WebSocket events.
    #[test]
    fn prop_mutation_broadcasts_events(
        trajectory_name in name_strategy(),
        scope_name in name_strategy(),
        artifact_name in name_strategy(),
        note_title in name_strategy(),
        turn_content in content_strategy(),
        token_budget in token_budget_strategy(),
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let db = test_support::test_db_client();
            let auth = test_auth_context();
            let ws = test_support::test_ws_state(100);
            let mut rx = ws.subscribe();
            let pcp = test_support::test_pcp_runtime();

            // ------------------------------------------------------------
            // Trajectory Created
            // ------------------------------------------------------------
            let trajectory_state = Arc::new(trajectory::TrajectoryState::new(db.clone(), ws.clone()));
            let create_traj = CreateTrajectoryRequest {
                name: trajectory_name.clone(),
                description: None,
                parent_trajectory_id: None,
                agent_id: None,
                metadata: None,
            };
            let _ = trajectory::create_trajectory(State(trajectory_state), AuthExtractor(auth.clone()), Json(create_traj)).await?;

            let trajectory = match recv_event(&mut rx, "TrajectoryCreated").await {
                WsEvent::TrajectoryCreated { trajectory } => trajectory,
                other => {
                    prop_assert!(false, "Expected TrajectoryCreated, got {:?}", other);
                    unreachable!()
                }
            };

            // ------------------------------------------------------------
            // Scope Created
            // ------------------------------------------------------------
            let scope_state = Arc::new(scope::ScopeState::new(db.clone(), ws.clone(), pcp.clone()));
            let create_scope = CreateScopeRequest {
                trajectory_id: trajectory.trajectory_id,
                parent_scope_id: None,
                name: scope_name.clone(),
                purpose: None,
                token_budget,
                metadata: None,
            };
            let _ = scope::create_scope(State(scope_state), AuthExtractor(auth.clone()), Json(create_scope)).await?;

            let scope = match recv_event(&mut rx, "ScopeCreated").await {
                WsEvent::ScopeCreated { scope } => scope,
                other => {
                    prop_assert!(false, "Expected ScopeCreated, got {:?}", other);
                    unreachable!()
                }
            };

            // ------------------------------------------------------------
            // Artifact Created
            // ------------------------------------------------------------
            let artifact_state = Arc::new(artifact::ArtifactState::new(db.clone(), ws.clone()));
            let create_artifact = CreateArtifactRequest {
                trajectory_id: trajectory.trajectory_id,
                scope_id: scope.scope_id,
                artifact_type: ArtifactType::Fact,
                name: artifact_name.clone(),
                content: "artifact content".to_string(),
                source_turn: 0,
                extraction_method: ExtractionMethod::Explicit,
                confidence: Some(0.9),
                ttl: TTL::Session,
                metadata: None,
            };
            let _ = artifact::create_artifact(State(artifact_state), AuthExtractor(auth.clone()), Json(create_artifact)).await?;

            match recv_event(&mut rx, "ArtifactCreated").await {
                WsEvent::ArtifactCreated { .. } => {}
                other => prop_assert!(false, "Expected ArtifactCreated, got {:?}", other),
            }

            // ------------------------------------------------------------
            // Note Created
            // ------------------------------------------------------------
            let note_state = Arc::new(note::NoteState::new(db.clone(), ws.clone()));
            let create_note = CreateNoteRequest {
                note_type: NoteType::Fact,
                title: note_title.clone(),
                content: "note content".to_string(),
                source_trajectory_ids: vec![trajectory.trajectory_id],
                source_artifact_ids: Vec::new(),
                ttl: TTL::Session,
                metadata: None,
            };
            let _ = note::create_note(State(note_state), AuthExtractor(auth.clone()), Json(create_note)).await?;

            match recv_event(&mut rx, "NoteCreated").await {
                WsEvent::NoteCreated { .. } => {}
                other => prop_assert!(false, "Expected NoteCreated, got {:?}", other),
            }

            // ------------------------------------------------------------
            // Turn Created
            // ------------------------------------------------------------
            let turn_state = Arc::new(turn::TurnState::new(db.clone(), ws.clone(), pcp));
            let create_turn = CreateTurnRequest {
                scope_id: scope.scope_id,
                sequence: 0,
                role: TurnRole::User,
                content: turn_content.clone(),
                token_count: 1,
                tool_calls: None,
                tool_results: None,
                metadata: None,
            };
            let _ = turn::create_turn(State(turn_state), AuthExtractor(auth.clone()), Json(create_turn)).await?;

            match recv_event(&mut rx, "TurnCreated").await {
                WsEvent::TurnCreated { .. } => {}
                other => prop_assert!(false, "Expected TurnCreated, got {:?}", other),
            }

            Ok::<(), TestCaseError>(())
        })?;
    }
}
