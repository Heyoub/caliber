use crate::{
    agent_heap::AgentRow,
    artifact_heap::ArtifactRow,
    conflict_heap::ConflictRow,
    delegation_heap::DelegationRow,
    edge_heap::EdgeRow,
    handoff_heap::HandoffRow,
    lock_heap::LockRow,
    message_heap::MessageRow,
    note_heap::NoteRow,
    scope_heap::ScopeRow,
    trajectory_heap::TrajectoryRow,
    turn_heap::TurnRow,
};
use caliber_core::{
    AbstractionLevel, Agent, AgentHandoff, AgentId, AgentMessage, Artifact, ArtifactType,
    Conflict, ConflictType, DelegatedTask, Edge, EdgeParticipant, EdgeType, EntityRef,
    EntityType, ExtractionMethod, HandoffReason, LockData, LockId, LockMode, MessageType,
    Note, NoteType, Provenance, Scope, TTL, TenantId, Trajectory, TrajectoryOutcome,
    TrajectoryStatus, Turn, TurnRole, compute_content_hash, EntityIdType,
    TrajectoryId, ScopeId, ArtifactId, NoteId, TurnId, EdgeId,
};
use chrono::Utc;
use uuid::Uuid;

fn sample_uuid(seed: u128) -> Uuid {
    Uuid::from_u128(seed)
}

fn sample_tenant_id(seed: u128) -> TenantId {
    TenantId::new(sample_uuid(seed))
}

fn sample_trajectory_id(seed: u128) -> TrajectoryId {
    TrajectoryId::new(sample_uuid(seed))
}

fn sample_scope_id(seed: u128) -> ScopeId {
    ScopeId::new(sample_uuid(seed))
}

fn sample_artifact_id(seed: u128) -> ArtifactId {
    ArtifactId::new(sample_uuid(seed))
}

fn sample_note_id(seed: u128) -> NoteId {
    NoteId::new(sample_uuid(seed))
}

fn sample_turn_id(seed: u128) -> TurnId {
    TurnId::new(sample_uuid(seed))
}

fn sample_edge_id(seed: u128) -> EdgeId {
    EdgeId::new(sample_uuid(seed))
}

fn sample_agent_id(seed: u128) -> AgentId {
    AgentId::new(sample_uuid(seed))
}

fn sample_provenance() -> Provenance {
    Provenance {
        source_turn: 1,
        extraction_method: ExtractionMethod::Explicit,
        confidence: Some(0.9),
    }
}

fn sample_trajectory() -> Trajectory {
    let now = Utc::now();
    Trajectory {
        trajectory_id: sample_trajectory_id(1),
        name: "trajectory".to_string(),
        description: Some("test".to_string()),
        status: TrajectoryStatus::Active,
        parent_trajectory_id: None,
        root_trajectory_id: None,
        agent_id: None,
        created_at: now,
        updated_at: now,
        completed_at: None,
        outcome: Some(TrajectoryOutcome {
            status: caliber_core::OutcomeStatus::Success,
            summary: "ok".to_string(),
            produced_artifacts: vec![sample_artifact_id(2)],
            produced_notes: vec![sample_note_id(3)],
            error: None,
        }),
        metadata: None,
    }
}

fn sample_scope() -> Scope {
    let now = Utc::now();
    Scope {
        scope_id: sample_scope_id(10),
        trajectory_id: sample_trajectory_id(11),
        parent_scope_id: None,
        name: "scope".to_string(),
        purpose: Some("purpose".to_string()),
        is_active: true,
        created_at: now,
        closed_at: None,
        checkpoint: None,
        token_budget: 1000,
        tokens_used: 10,
        metadata: None,
    }
}

fn sample_artifact() -> Artifact {
    let now = Utc::now();
    let content = "artifact".to_string();
    Artifact {
        artifact_id: sample_artifact_id(20),
        trajectory_id: sample_trajectory_id(21),
        scope_id: sample_scope_id(22),
        artifact_type: ArtifactType::Code,
        name: "artifact".to_string(),
        content: content.clone(),
        content_hash: compute_content_hash(content.as_bytes()),
        embedding: None,
        provenance: sample_provenance(),
        ttl: TTL::Persistent,
        created_at: now,
        updated_at: now,
        superseded_by: None,
        metadata: None,
    }
}

fn sample_note() -> Note {
    let now = Utc::now();
    let content = "note".to_string();
    Note {
        note_id: sample_note_id(30),
        note_type: NoteType::Fact,
        title: "title".to_string(),
        content: content.clone(),
        content_hash: compute_content_hash(content.as_bytes()),
        embedding: None,
        source_trajectory_ids: vec![sample_trajectory_id(31)],
        source_artifact_ids: vec![sample_artifact_id(32)],
        ttl: TTL::LongTerm,
        created_at: now,
        updated_at: now,
        accessed_at: now,
        access_count: 0,
        superseded_by: None,
        metadata: None,
        abstraction_level: AbstractionLevel::Raw,
        source_note_ids: vec![sample_note_id(33)],
    }
}

fn sample_turn() -> Turn {
    Turn {
        turn_id: sample_turn_id(40),
        scope_id: sample_scope_id(41),
        sequence: 1,
        role: TurnRole::User,
        content: "hello".to_string(),
        token_count: 4,
        created_at: Utc::now(),
        tool_calls: None,
        tool_results: None,
        metadata: None,
    }
}

fn sample_edge() -> Edge {
    Edge {
        edge_id: sample_edge_id(50),
        edge_type: EdgeType::RelatesTo,
        participants: vec![
            EdgeParticipant {
                entity_ref: EntityRef {
                    entity_type: EntityType::Artifact,
                    id: sample_uuid(51),
                },
                role: Some("source".to_string()),
            },
            EdgeParticipant {
                entity_ref: EntityRef {
                    entity_type: EntityType::Note,
                    id: sample_uuid(52),
                },
                role: Some("target".to_string()),
            },
        ],
        weight: Some(0.5),
        trajectory_id: None,
        provenance: Provenance {
            source_turn: 2,
            extraction_method: ExtractionMethod::Inferred,
            confidence: Some(0.7),
        },
        created_at: Utc::now(),
        metadata: None,
    }
}

#[test]
fn trajectory_row_into_trajectory() {
    let trajectory = sample_trajectory();
    let row = TrajectoryRow {
        trajectory: trajectory.clone(),
        tenant_id: Some(sample_tenant_id(99)),
    };
    let converted: Trajectory = row.into();
    assert_eq!(converted, trajectory);
}

#[test]
fn scope_row_into_scope() {
    let scope = sample_scope();
    let row = ScopeRow {
        scope: scope.clone(),
        tenant_id: Some(sample_tenant_id(99)),
    };
    let converted: Scope = row.into();
    assert_eq!(converted, scope);
}

#[test]
fn artifact_row_into_artifact() {
    let artifact = sample_artifact();
    let row = ArtifactRow {
        artifact: artifact.clone(),
        tenant_id: Some(sample_tenant_id(99)),
    };
    let converted: Artifact = row.into();
    assert_eq!(converted, artifact);
}

#[test]
fn note_row_into_note() {
    let note = sample_note();
    let row = NoteRow {
        note: note.clone(),
        tenant_id: Some(sample_tenant_id(99)),
    };
    let converted: Note = row.into();
    assert_eq!(converted, note);
}

#[test]
fn turn_row_into_turn() {
    let turn = sample_turn();
    let row = TurnRow {
        turn: turn.clone(),
        tenant_id: Some(sample_tenant_id(99)),
    };
    let converted: Turn = row.into();
    assert_eq!(converted, turn);
}

#[test]
fn edge_row_into_edge() {
    let edge = sample_edge();
    let row = EdgeRow {
        edge: edge.clone(),
        tenant_id: Some(sample_tenant_id(99)),
    };
    let converted: Edge = row.into();
    assert_eq!(converted, edge);
}

#[test]
fn agent_row_into_agent() {
    let agent = Agent::new("coder", vec!["search".to_string()]);
    let row = AgentRow {
        agent: agent.clone(),
        tenant_id: Some(sample_tenant_id(99)),
    };
    let converted: Agent = row.into();
    assert_eq!(converted, agent);
}

#[test]
fn lock_row_into_lock() {
    let now = Utc::now();
    let lock = LockData {
        lock_id: LockId::new(sample_uuid(59)),
        tenant_id: TenantId::new(sample_uuid(99)),
        resource_type: "resource".to_string(),
        resource_id: sample_uuid(60),
        holder_agent_id: AgentId::new(sample_uuid(61)),
        acquired_at: now,
        expires_at: now + chrono::Duration::seconds(1000),
        mode: LockMode::Exclusive,
    };
    let row = LockRow {
        lock: lock.clone(),
    };
    let converted: LockData = row.into();
    assert_eq!(converted, lock);
}

#[test]
fn message_row_into_message() {
    let message = AgentMessage::to_agent(
        sample_agent_id(70),
        sample_agent_id(71),
        MessageType::Heartbeat,
        "{}",
    );
    let row = MessageRow {
        message: message.clone(),
        tenant_id: Some(sample_tenant_id(99)),
    };
    let converted: AgentMessage = row.into();
    assert_eq!(converted, message);
}

#[test]
fn delegation_row_into_delegated_task() {
    let delegation = DelegatedTask::to_agent(
        sample_agent_id(80),
        sample_agent_id(81),
        sample_trajectory_id(82),
        "task",
    );
    let row = DelegationRow {
        delegation: delegation.clone(),
        tenant_id: Some(sample_tenant_id(99)),
    };
    let converted: DelegatedTask = row.into();
    assert_eq!(converted, delegation);
}

#[test]
fn handoff_row_into_handoff() {
    let handoff = AgentHandoff::to_agent(
        sample_agent_id(90),
        sample_agent_id(91),
        sample_trajectory_id(92),
        sample_scope_id(93),
        HandoffReason::Escalation,
    );
    let row = HandoffRow {
        handoff: handoff.clone(),
        tenant_id: Some(sample_tenant_id(99)),
    };
    let converted: AgentHandoff = row.into();
    assert_eq!(converted, handoff);
}

#[test]
fn conflict_row_into_conflict() {
    let conflict = Conflict::new(
        ConflictType::ContradictingFact,
        "artifact",
        sample_uuid(100),
        "note",
        sample_uuid(101),
    );
    let row = ConflictRow {
        conflict: conflict.clone(),
        tenant_id: Some(sample_tenant_id(99)),
    };
    let converted: Conflict = row.into();
    assert_eq!(converted, conflict);
}
