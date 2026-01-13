# Multi-Agent Coordination Protocol

**Crate:** `caliber-agents/` (depends on `caliber-core`, `caliber-storage`)

## Problem Statement

Multi-agent systems suffer 40-80% failure rates primarily due to memory coordination issues:

- Agents overwrite each other's state
- No visibility into other agents' context
- Race conditions on shared artifacts
- Contradictory decisions without reconciliation
- No handoff protocol between agents

CALIBER addresses this through **typed context contracts** and **coordination primitives** - all implemented as direct Postgres storage operations via pgrx.

---

## 1. Agent Identity & Registration (Rust)

```rust
use pgx::prelude::*;
use uuid::Uuid;
use chrono::{DateTime, Utc};

// === AGENT IDENTITY ===

#[derive(Debug, Clone, PostgresType)]
pub struct Agent {
    pub agent_id: Uuid,
    pub agent_type: String,           // "coder", "reviewer", "planner"
    pub capabilities: Vec<String>,
    pub memory_access: MemoryAccess,
    
    // Runtime state
    pub status: AgentStatus,
    pub current_trajectory_id: Option<Uuid>,
    pub current_scope_id: Option<Uuid>,
    
    // Coordination
    pub can_delegate_to: Vec<String>,
    pub reports_to: Option<Uuid>,
    
    // Timestamps
    pub created_at: DateTime<Utc>,
    pub last_heartbeat: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PostgresEnum)]
pub enum AgentStatus {
    Idle,
    Active,
    Blocked,
    Failed,
}

#[derive(Debug, Clone, PostgresType)]
pub struct MemoryAccess {
    pub read: Vec<MemoryPermission>,
    pub write: Vec<MemoryPermission>,
}

#[derive(Debug, Clone, PostgresType)]
pub struct MemoryPermission {
    pub memory_type: String,
    pub scope: PermissionScope,
    pub filter: Option<String>,  // Serialized filter expression
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PostgresEnum)]
pub enum PermissionScope {
    Own,
    Team,
    Global,
}

// === DIRECT STORAGE OPERATIONS ===

#[pg_extern]
pub fn caliber_agent_register(
    agent_type: &str,
    capabilities: Vec<String>,
    memory_access: MemoryAccess,
    can_delegate_to: Vec<String>,
    reports_to: Option<Uuid>,
) -> Uuid {
    let agent_id = crate::generate_uuidv7();
    let now = chrono::Utc::now();
    
    let agent = Agent {
        agent_id,
        agent_type: agent_type.to_string(),
        capabilities,
        memory_access,
        status: AgentStatus::Idle,
        current_trajectory_id: None,
        current_scope_id: None,
        can_delegate_to,
        reports_to,
        created_at: now,
        last_heartbeat: now,
    };
    
    // Direct heap insert - no SQL
    caliber_agent_insert(&agent);
    
    agent_id
}

#[pg_extern]
pub fn caliber_agent_heartbeat(agent_id: Uuid) {
    // Direct heap update - no SQL
    caliber_agent_update_heartbeat(agent_id, chrono::Utc::now());
}

#[pg_extern]
pub fn caliber_agent_set_status(agent_id: Uuid, status: AgentStatus) {
    caliber_agent_update_status(agent_id, status);
}

#[pg_extern]
pub fn caliber_agent_assign_trajectory(
    agent_id: Uuid,
    trajectory_id: Uuid,
    scope_id: Uuid,
) {
    caliber_agent_update_assignment(agent_id, Some(trajectory_id), Some(scope_id));
    caliber_agent_update_status(agent_id, AgentStatus::Active);
}

#[pg_extern]
pub fn caliber_agent_release(agent_id: Uuid) {
    caliber_agent_update_assignment(agent_id, None, None);
    caliber_agent_update_status(agent_id, AgentStatus::Idle);
}
```

---

## 2. Memory Regions (Rust)

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, PostgresEnum)]
pub enum MemoryRegion {
    Private,      // Only owning agent can access
    Team,         // Agents in same team can access
    Public,       // Any agent can read, owner can write
    Collaborative, // Any agent can read/write with coordination
}

#[derive(Debug, Clone, PostgresType)]
pub struct MemoryRegionConfig {
    pub region_id: Uuid,
    pub region_type: MemoryRegion,
    pub owner_agent_id: Uuid,
    pub team_id: Option<Uuid>,
    
    // Access control
    pub readers: Vec<Uuid>,
    pub writers: Vec<Uuid>,
    
    // Coordination settings
    pub require_lock: bool,
    pub conflict_resolution: ConflictResolution,
    pub version_tracking: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PostgresEnum)]
pub enum ConflictResolution {
    LastWriteWins,
    Merge,
    Manual,
    Reject,
}

// === ACCESS CONTROL ===

#[pg_extern]
pub fn caliber_check_access(
    agent_id: Uuid,
    memory_type: &str,
    memory_id: Uuid,
    access_type: &str,  // "read" or "write"
) -> bool {
    let agent = match caliber_agent_get(agent_id) {
        Some(a) => a,
        None => return false,
    };
    
    let permissions = if access_type == "read" {
        &agent.memory_access.read
    } else {
        &agent.memory_access.write
    };
    
    for perm in permissions {
        if perm.memory_type == memory_type || perm.memory_type == "*" {
            match perm.scope {
                PermissionScope::Global => return true,
                PermissionScope::Team => {
                    // Check if memory belongs to same team
                    if check_same_team(agent_id, memory_id) {
                        return true;
                    }
                }
                PermissionScope::Own => {
                    // Check if memory belongs to this agent
                    if check_ownership(agent_id, memory_type, memory_id) {
                        return true;
                    }
                }
            }
        }
    }
    
    false
}
```

---

## 3. Distributed Locks (Rust)

```rust
#[derive(Debug, Clone, PostgresType)]
pub struct DistributedLock {
    pub lock_id: Uuid,
    pub resource_type: String,
    pub resource_id: Uuid,
    pub holder_agent_id: Uuid,
    pub acquired_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub mode: LockMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PostgresEnum)]
pub enum LockMode {
    Exclusive,
    Shared,
}

/// Acquire a lock using Postgres advisory locks
/// This is atomic and deadlock-safe
#[pg_extern]
pub fn caliber_lock_acquire(
    agent_id: Uuid,
    resource_type: &str,
    resource_id: Uuid,
    mode: LockMode,
    timeout_ms: i64,
) -> Option<Uuid> {
    let lock_key = compute_lock_key(resource_type, resource_id);
    
    // Use Postgres advisory locks for atomicity
    let acquired = unsafe {
        if mode == LockMode::Exclusive {
            // Try exclusive lock with timeout
            pg_sys::pg_advisory_xact_lock(lock_key as i64)
        } else {
            // Try shared lock
            pg_sys::pg_advisory_xact_lock_shared(lock_key as i64)
        }
        true // Advisory locks block until acquired or timeout
    };
    
    if !acquired {
        return None;
    }
    
    let now = chrono::Utc::now();
    let lock = DistributedLock {
        lock_id: crate::generate_uuidv7(),
        resource_type: resource_type.to_string(),
        resource_id,
        holder_agent_id: agent_id,
        acquired_at: now,
        expires_at: now + chrono::Duration::milliseconds(timeout_ms),
        mode,
    };
    
    // Record lock in our table for visibility
    caliber_lock_insert(&lock);
    
    Some(lock.lock_id)
}

#[pg_extern]
pub fn caliber_lock_release(lock_id: Uuid) -> bool {
    let lock = match caliber_lock_get(lock_id) {
        Some(l) => l,
        None => return false,
    };
    
    let lock_key = compute_lock_key(&lock.resource_type, lock.resource_id);
    
    // Release advisory lock
    unsafe {
        if lock.mode == LockMode::Exclusive {
            pg_sys::pg_advisory_unlock(lock_key as i64);
        } else {
            pg_sys::pg_advisory_unlock_shared(lock_key as i64);
        }
    }
    
    // Remove from our tracking table
    caliber_lock_delete(lock_id);
    
    true
}

/// Compute a stable i64 key for advisory locks
fn compute_lock_key(resource_type: &str, resource_id: Uuid) -> i64 {
    use std::hash::{Hash, Hasher};
    use std::collections::hash_map::DefaultHasher;
    
    let mut hasher = DefaultHasher::new();
    resource_type.hash(&mut hasher);
    resource_id.as_bytes().hash(&mut hasher);
    hasher.finish() as i64
}
```

---

## 4. Message Passing (Rust + NOTIFY)

```rust
#[derive(Debug, Clone, PostgresType)]
pub struct AgentMessage {
    pub message_id: Uuid,
    
    // Routing
    pub from_agent_id: Uuid,
    pub to_agent_id: Option<Uuid>,     // Specific agent
    pub to_agent_type: Option<String>, // Or broadcast to type
    
    // Content
    pub message_type: MessageType,
    pub payload: String,  // JSON serialized
    
    // References
    pub trajectory_id: Option<Uuid>,
    pub scope_id: Option<Uuid>,
    pub artifact_ids: Vec<Uuid>,
    
    // Delivery
    pub created_at: DateTime<Utc>,
    pub delivered_at: Option<DateTime<Utc>>,
    pub acknowledged_at: Option<DateTime<Utc>>,
    
    // Priority & TTL
    pub priority: MessagePriority,
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PostgresEnum)]
pub enum MessageType {
    TaskDelegation,
    TaskResult,
    ContextRequest,
    ContextShare,
    CoordinationSignal,
    Handoff,
    Interrupt,
    Heartbeat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PostgresEnum)]
pub enum MessagePriority {
    Low,
    Normal,
    High,
    Critical,
}

#[pg_extern]
pub fn caliber_message_send(
    from_agent_id: Uuid,
    to_agent_id: Option<Uuid>,
    to_agent_type: Option<String>,
    message_type: MessageType,
    payload: &str,
    trajectory_id: Option<Uuid>,
    priority: MessagePriority,
) -> Uuid {
    let message_id = crate::generate_uuidv7();
    let now = chrono::Utc::now();
    
    let message = AgentMessage {
        message_id,
        from_agent_id,
        to_agent_id,
        to_agent_type: to_agent_type.clone(),
        message_type,
        payload: payload.to_string(),
        trajectory_id,
        scope_id: None,
        artifact_ids: vec![],
        created_at: now,
        delivered_at: None,
        acknowledged_at: None,
        priority,
        expires_at: None,
    };
    
    // Direct heap insert
    caliber_message_insert(&message);
    
    // Notify via Postgres NOTIFY for real-time delivery
    let notify_payload = serde_json::json!({
        "message_id": message_id.to_string(),
        "to_agent_id": to_agent_id.map(|u| u.to_string()),
        "to_agent_type": to_agent_type,
        "message_type": format!("{:?}", message_type),
        "priority": format!("{:?}", priority),
    });
    
    crate::pg_notify("caliber_agent_message", &notify_payload.to_string());
    
    message_id
}

#[pg_extern]
pub fn caliber_message_receive(
    agent_id: Uuid,
    agent_type: &str,
    limit: i32,
) -> Vec<AgentMessage> {
    // Direct index scan for pending messages
    let messages = caliber_message_query_pending(
        Some(agent_id),
        Some(agent_type.to_string()),
        limit,
    );
    
    // Mark as delivered
    let now = chrono::Utc::now();
    for msg in &messages {
        caliber_message_update_delivered(msg.message_id, now);
    }
    
    messages
}

#[pg_extern]
pub fn caliber_message_acknowledge(message_id: Uuid) {
    caliber_message_update_acknowledged(message_id, chrono::Utc::now());
}
```

---

## 5. Task Delegation (Rust)

```rust
#[derive(Debug, Clone, PostgresType)]
pub struct DelegatedTask {
    pub delegation_id: Uuid,
    
    // Who's involved
    pub delegator_agent_id: Uuid,
    pub delegatee_agent_id: Option<Uuid>,
    pub delegatee_agent_type: Option<String>,
    
    // What's being delegated
    pub task_description: String,
    pub parent_trajectory_id: Uuid,
    pub child_trajectory_id: Option<Uuid>,
    
    // Context sharing
    pub shared_artifacts: Vec<Uuid>,
    pub shared_notes: Vec<Uuid>,
    pub additional_context: Option<String>,
    
    // Constraints
    pub constraints: String,  // JSON serialized
    pub deadline: Option<DateTime<Utc>>,
    
    // Status
    pub status: DelegationStatus,
    pub result: Option<DelegationResult>,
    
    pub created_at: DateTime<Utc>,
    pub accepted_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PostgresEnum)]
pub enum DelegationStatus {
    Pending,
    Accepted,
    Rejected,
    InProgress,
    Completed,
    Failed,
}

#[derive(Debug, Clone, PostgresType)]
pub struct DelegationResult {
    pub status: DelegationResultStatus,
    pub produced_artifacts: Vec<Uuid>,
    pub produced_notes: Vec<Uuid>,
    pub summary: String,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PostgresEnum)]
pub enum DelegationResultStatus {
    Success,
    Partial,
    Failure,
}

#[pg_extern]
pub fn caliber_delegate_task(
    delegator_agent_id: Uuid,
    delegatee_agent_type: &str,
    task_description: &str,
    parent_trajectory_id: Uuid,
    shared_artifacts: Vec<Uuid>,
    deadline: Option<DateTime<Utc>>,
) -> Uuid {
    let delegation_id = crate::generate_uuidv7();
    let now = chrono::Utc::now();
    
    let task = DelegatedTask {
        delegation_id,
        delegator_agent_id,
        delegatee_agent_id: None,
        delegatee_agent_type: Some(delegatee_agent_type.to_string()),
        task_description: task_description.to_string(),
        parent_trajectory_id,
        child_trajectory_id: None,
        shared_artifacts,
        shared_notes: vec![],
        additional_context: None,
        constraints: "[]".to_string(),
        deadline,
        status: DelegationStatus::Pending,
        result: None,
        created_at: now,
        accepted_at: None,
        completed_at: None,
    };
    
    // Direct heap insert
    caliber_delegation_insert(&task);
    
    // Send delegation message
    caliber_message_send(
        delegator_agent_id,
        None,
        Some(delegatee_agent_type.to_string()),
        MessageType::TaskDelegation,
        &serde_json::json!({ "delegation_id": delegation_id.to_string() }).to_string(),
        Some(parent_trajectory_id),
        if deadline.is_some() { MessagePriority::High } else { MessagePriority::Normal },
    );
    
    delegation_id
}

#[pg_extern]
pub fn caliber_accept_delegation(
    delegation_id: Uuid,
    delegatee_agent_id: Uuid,
) -> Uuid {
    let mut task = caliber_delegation_get(delegation_id)
        .expect("Delegation not found");
    
    if task.status != DelegationStatus::Pending {
        panic!("Delegation not available for acceptance");
    }
    
    // Create child trajectory
    let child_trajectory_id = crate::CaliberOrchestrator::start_trajectory(
        &task.task_description,
        EntityType::Agent,
        delegatee_agent_id,
        Some(task.parent_trajectory_id),
    );
    
    // Copy shared artifacts to child trajectory
    let child_scope_id = caliber_scope_get_current(child_trajectory_id)
        .expect("No scope").scope_id;
    
    for artifact_id in &task.shared_artifacts {
        if let Some(artifact) = caliber_artifact_get(*artifact_id) {
            crate::CaliberOrchestrator::create_artifact(
                child_scope_id,
                child_trajectory_id,
                artifact.artifact_type,
                &artifact.content,
                ExtractionMethod::UserProvided,
            );
        }
    }
    
    // Update delegation
    let now = chrono::Utc::now();
    caliber_delegation_update(delegation_id, DelegationUpdate {
        delegatee_agent_id: Some(delegatee_agent_id),
        child_trajectory_id: Some(child_trajectory_id),
        status: Some(DelegationStatus::Accepted),
        accepted_at: Some(now),
        ..Default::default()
    });
    
    // Notify delegator
    caliber_message_send(
        delegatee_agent_id,
        Some(task.delegator_agent_id),
        None,
        MessageType::CoordinationSignal,
        &serde_json::json!({
            "event": "delegation_accepted",
            "delegation_id": delegation_id.to_string(),
            "child_trajectory_id": child_trajectory_id.to_string(),
        }).to_string(),
        Some(task.parent_trajectory_id),
        MessagePriority::Normal,
    );
    
    child_trajectory_id
}

#[pg_extern]
pub fn caliber_complete_delegation(
    delegation_id: Uuid,
    result_status: DelegationResultStatus,
    summary: &str,
    produced_artifacts: Vec<Uuid>,
) {
    let task = caliber_delegation_get(delegation_id)
        .expect("Delegation not found");
    
    let result = DelegationResult {
        status: result_status,
        produced_artifacts: produced_artifacts.clone(),
        produced_notes: vec![],
        summary: summary.to_string(),
        error: None,
    };
    
    let now = chrono::Utc::now();
    caliber_delegation_update(delegation_id, DelegationUpdate {
        status: Some(if result_status == DelegationResultStatus::Failure {
            DelegationStatus::Failed
        } else {
            DelegationStatus::Completed
        }),
        result: Some(result.clone()),
        completed_at: Some(now),
        ..Default::default()
    });
    
    // Notify delegator
    if let Some(delegatee_id) = task.delegatee_agent_id {
        caliber_message_send(
            delegatee_id,
            Some(task.delegator_agent_id),
            None,
            MessageType::TaskResult,
            &serde_json::json!({
                "delegation_id": delegation_id.to_string(),
                "result": {
                    "status": format!("{:?}", result_status),
                    "summary": summary,
                },
            }).to_string(),
            Some(task.parent_trajectory_id),
            MessagePriority::High,
        );
    }
}
```

---

## 6. Conflict Resolution (Rust)

```rust
#[derive(Debug, Clone, PostgresType)]
pub struct Conflict {
    pub conflict_id: Uuid,
    pub conflict_type: ConflictType,
    
    // Conflicting items
    pub item_a_type: String,
    pub item_a_id: Uuid,
    pub item_b_type: String,
    pub item_b_id: Uuid,
    
    // Agents involved
    pub agent_a_id: Option<Uuid>,
    pub agent_b_id: Option<Uuid>,
    
    // Context
    pub trajectory_id: Option<Uuid>,
    
    // Resolution
    pub status: ConflictStatus,
    pub resolution: Option<ConflictResolutionRecord>,
    
    pub detected_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PostgresEnum)]
pub enum ConflictType {
    ConcurrentWrite,
    ContradictingFact,
    IncompatibleDecision,
    ResourceContention,
    GoalConflict,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PostgresEnum)]
pub enum ConflictStatus {
    Detected,
    Resolving,
    Resolved,
    Escalated,
}

#[derive(Debug, Clone, PostgresType)]
pub struct ConflictResolutionRecord {
    pub strategy: ResolutionStrategy,
    pub winner: Option<String>,  // "a", "b", or "merged"
    pub merged_result_id: Option<Uuid>,
    pub reason: String,
    pub resolved_by: String,  // "automatic" or agent UUID
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PostgresEnum)]
pub enum ResolutionStrategy {
    LastWriteWins,
    FirstWriteWins,
    HighestConfidence,
    Merge,
    Escalate,
    RejectBoth,
}

#[pg_extern]
pub fn caliber_detect_conflicts(trajectory_id: Uuid) -> Vec<Conflict> {
    let mut conflicts = Vec::new();
    
    // Get all fact artifacts in trajectory
    let facts = caliber_artifact_query_by_type(trajectory_id, ArtifactType::Fact);
    
    // Check for contradictions via embedding similarity
    for i in 0..facts.len() {
        for j in (i + 1)..facts.len() {
            if let (Some(emb_a), Some(emb_b)) = (&facts[i].embedding, &facts[j].embedding) {
                let similarity = emb_a.cosine_similarity(emb_b);
                
                // High similarity but different content = potential contradiction
                if similarity > 0.85 && facts[i].content_hash != facts[j].content_hash {
                    conflicts.push(Conflict {
                        conflict_id: crate::generate_uuidv7(),
                        conflict_type: ConflictType::ContradictingFact,
                        item_a_type: "artifact".to_string(),
                        item_a_id: facts[i].artifact_id,
                        item_b_type: "artifact".to_string(),
                        item_b_id: facts[j].artifact_id,
                        agent_a_id: None,
                        agent_b_id: None,
                        trajectory_id: Some(trajectory_id),
                        status: ConflictStatus::Detected,
                        resolution: None,
                        detected_at: chrono::Utc::now(),
                        resolved_at: None,
                    });
                }
            }
        }
    }
    
    // Store detected conflicts
    for conflict in &conflicts {
        caliber_conflict_insert(conflict);
    }
    
    conflicts
}

#[pg_extern]
pub fn caliber_resolve_conflict(
    conflict_id: Uuid,
    strategy: ResolutionStrategy,
) -> ConflictResolutionRecord {
    let conflict = caliber_conflict_get(conflict_id)
        .expect("Conflict not found");
    
    let resolution = match strategy {
        ResolutionStrategy::LastWriteWins => {
            let item_a = caliber_artifact_get(conflict.item_a_id);
            let item_b = caliber_artifact_get(conflict.item_b_id);
            
            let winner = if item_a.map(|a| a.created_at) > item_b.map(|b| b.created_at) {
                "a"
            } else {
                "b"
            };
            
            ConflictResolutionRecord {
                strategy,
                winner: Some(winner.to_string()),
                merged_result_id: None,
                reason: format!("Selected {} as it was written last", winner),
                resolved_by: "automatic".to_string(),
            }
        }
        
        ResolutionStrategy::HighestConfidence => {
            let item_a = caliber_artifact_get(conflict.item_a_id);
            let item_b = caliber_artifact_get(conflict.item_b_id);
            
            let conf_a = item_a.as_ref().and_then(|a| a.provenance.confidence).unwrap_or(0.5);
            let conf_b = item_b.as_ref().and_then(|b| b.provenance.confidence).unwrap_or(0.5);
            
            if (conf_a - conf_b).abs() < 0.1 {
                ConflictResolutionRecord {
                    strategy: ResolutionStrategy::Escalate,
                    winner: None,
                    merged_result_id: None,
                    reason: format!("Confidence too similar ({} vs {})", conf_a, conf_b),
                    resolved_by: "automatic".to_string(),
                }
            } else {
                let winner = if conf_a > conf_b { "a" } else { "b" };
                ConflictResolutionRecord {
                    strategy,
                    winner: Some(winner.to_string()),
                    merged_result_id: None,
                    reason: format!("Selected {} with confidence {}", winner, if winner == "a" { conf_a } else { conf_b }),
                    resolved_by: "automatic".to_string(),
                }
            }
        }
        
        ResolutionStrategy::Escalate => {
            ConflictResolutionRecord {
                strategy,
                winner: None,
                merged_result_id: None,
                reason: "Requires human or supervisor agent decision".to_string(),
                resolved_by: "automatic".to_string(),
            }
        }
        
        _ => {
            ConflictResolutionRecord {
                strategy: ResolutionStrategy::Escalate,
                winner: None,
                merged_result_id: None,
                reason: "Strategy not implemented".to_string(),
                resolved_by: "automatic".to_string(),
            }
        }
    };
    
    // Update conflict
    let now = chrono::Utc::now();
    caliber_conflict_update(conflict_id, ConflictUpdate {
        status: Some(if resolution.strategy == ResolutionStrategy::Escalate {
            ConflictStatus::Escalated
        } else {
            ConflictStatus::Resolved
        }),
        resolution: Some(resolution.clone()),
        resolved_at: Some(now),
    });
    
    resolution
}
```

---

## 7. Handoff Protocol (Rust)

```rust
#[derive(Debug, Clone, PostgresType)]
pub struct AgentHandoff {
    pub handoff_id: Uuid,
    
    // Agents
    pub from_agent_id: Uuid,
    pub to_agent_id: Option<Uuid>,
    pub to_agent_type: Option<String>,
    
    // What's being handed off
    pub trajectory_id: Uuid,
    pub scope_id: Uuid,
    
    // Context transfer
    pub context_snapshot_id: Uuid,
    pub handoff_notes: String,
    
    // Continuation hints
    pub next_steps: Vec<String>,
    pub blockers: Vec<String>,
    pub open_questions: Vec<String>,
    
    // Status
    pub status: HandoffStatus,
    
    pub initiated_at: DateTime<Utc>,
    pub accepted_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    
    // Reason for handoff
    pub reason: HandoffReason,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PostgresEnum)]
pub enum HandoffStatus {
    Initiated,
    Accepted,
    Completed,
    Rejected,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PostgresEnum)]
pub enum HandoffReason {
    CapabilityMismatch,
    LoadBalancing,
    Specialization,
    Escalation,
    Timeout,
    Failure,
    Scheduled,
}

#[pg_extern]
pub fn caliber_initiate_handoff(
    from_agent_id: Uuid,
    to_agent_type: &str,
    trajectory_id: Uuid,
    reason: HandoffReason,
    handoff_notes: &str,
    next_steps: Vec<String>,
) -> Uuid {
    let handoff_id = crate::generate_uuidv7();
    let now = chrono::Utc::now();
    
    // Get current scope
    let current_scope = caliber_scope_get_current(trajectory_id)
        .expect("No active scope");
    
    // Close current scope
    crate::CaliberOrchestrator::close_scope(current_scope.scope_id);
    
    // Create context snapshot
    let context = crate::CaliberOrchestrator::assemble_context(
        trajectory_id,
        16000,
        None,
    );
    
    let handoff = AgentHandoff {
        handoff_id,
        from_agent_id,
        to_agent_id: None,
        to_agent_type: Some(to_agent_type.to_string()),
        trajectory_id,
        scope_id: current_scope.scope_id,
        context_snapshot_id: context.window_id,
        handoff_notes: handoff_notes.to_string(),
        next_steps,
        blockers: vec![],
        open_questions: vec![],
        status: HandoffStatus::Initiated,
        initiated_at: now,
        accepted_at: None,
        completed_at: None,
        reason,
    };
    
    caliber_handoff_insert(&handoff);
    
    // Send handoff message
    caliber_message_send(
        from_agent_id,
        None,
        Some(to_agent_type.to_string()),
        MessageType::Handoff,
        &serde_json::json!({ "handoff_id": handoff_id.to_string() }).to_string(),
        Some(trajectory_id),
        MessagePriority::Critical,
    );
    
    handoff_id
}

#[pg_extern]
pub fn caliber_accept_handoff(
    handoff_id: Uuid,
    accepting_agent_id: Uuid,
) -> Uuid {
    let handoff = caliber_handoff_get(handoff_id)
        .expect("Handoff not found");
    
    if handoff.status != HandoffStatus::Initiated {
        panic!("Handoff not available");
    }
    
    let now = chrono::Utc::now();
    
    // Update handoff status
    caliber_handoff_update(handoff_id, HandoffUpdate {
        to_agent_id: Some(accepting_agent_id),
        status: Some(HandoffStatus::Accepted),
        accepted_at: Some(now),
        ..Default::default()
    });
    
    // Open new scope for accepting agent
    let new_scope_id = crate::CaliberOrchestrator::open_scope(handoff.trajectory_id);
    
    // Create handoff artifact as first item in new scope
    let handoff_content = format!(
        "HANDOFF RECEIVED\nReason: {:?}\n\nNotes: {}\n\nNext Steps:\n{}",
        handoff.reason,
        handoff.handoff_notes,
        handoff.next_steps.iter().map(|s| format!("- {}", s)).collect::<Vec<_>>().join("\n"),
    );
    
    crate::CaliberOrchestrator::create_artifact(
        new_scope_id,
        handoff.trajectory_id,
        ArtifactType::DesignDecision,
        handoff_content.as_bytes(),
        ExtractionMethod::UserProvided,
    );
    
    // Update agent assignments
    caliber_agent_release(handoff.from_agent_id);
    caliber_agent_assign_trajectory(accepting_agent_id, handoff.trajectory_id, new_scope_id);
    
    new_scope_id
}
```

---

## 8. Bootstrap Schema (One-Time SQL)

This SQL runs ONCE at extension install (via `caliber_init()`), NOT in hot path:

```sql
-- bootstrap.sql (loaded by caliber_init())

-- Agent table
CREATE TABLE IF NOT EXISTS caliber_agent (
    agent_id UUID PRIMARY KEY,
    agent_type TEXT NOT NULL,
    capabilities TEXT[] DEFAULT '{}',
    memory_access JSONB NOT NULL,
    status TEXT DEFAULT 'idle',
    current_trajectory_id UUID,
    current_scope_id UUID,
    can_delegate_to TEXT[] DEFAULT '{}',
    reports_to UUID,
    created_at TIMESTAMPTZ NOT NULL,
    last_heartbeat TIMESTAMPTZ NOT NULL
);

-- Locks table (advisory lock metadata)
CREATE TABLE IF NOT EXISTS caliber_lock (
    lock_id UUID PRIMARY KEY,
    resource_type TEXT NOT NULL,
    resource_id UUID NOT NULL,
    holder_agent_id UUID NOT NULL,
    acquired_at TIMESTAMPTZ NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL,
    mode TEXT NOT NULL
);

-- Messages table
CREATE TABLE IF NOT EXISTS caliber_message (
    message_id UUID PRIMARY KEY,
    from_agent_id UUID NOT NULL,
    to_agent_id UUID,
    to_agent_type TEXT,
    message_type TEXT NOT NULL,
    payload JSONB NOT NULL,
    trajectory_id UUID,
    scope_id UUID,
    artifact_ids UUID[] DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL,
    delivered_at TIMESTAMPTZ,
    acknowledged_at TIMESTAMPTZ,
    priority TEXT DEFAULT 'normal',
    expires_at TIMESTAMPTZ
);

-- Delegations table
CREATE TABLE IF NOT EXISTS caliber_delegation (
    delegation_id UUID PRIMARY KEY,
    delegator_agent_id UUID NOT NULL,
    delegatee_agent_id UUID,
    delegatee_agent_type TEXT,
    task_description TEXT NOT NULL,
    parent_trajectory_id UUID NOT NULL,
    child_trajectory_id UUID,
    shared_artifacts UUID[] DEFAULT '{}',
    shared_notes UUID[] DEFAULT '{}',
    additional_context TEXT,
    constraints JSONB DEFAULT '[]',
    deadline TIMESTAMPTZ,
    status TEXT DEFAULT 'pending',
    result JSONB,
    created_at TIMESTAMPTZ NOT NULL,
    accepted_at TIMESTAMPTZ,
    completed_at TIMESTAMPTZ
);

-- Conflicts table
CREATE TABLE IF NOT EXISTS caliber_conflict (
    conflict_id UUID PRIMARY KEY,
    conflict_type TEXT NOT NULL,
    item_a_type TEXT NOT NULL,
    item_a_id UUID NOT NULL,
    item_b_type TEXT NOT NULL,
    item_b_id UUID NOT NULL,
    agent_a_id UUID,
    agent_b_id UUID,
    trajectory_id UUID,
    status TEXT DEFAULT 'detected',
    resolution JSONB,
    detected_at TIMESTAMPTZ NOT NULL,
    resolved_at TIMESTAMPTZ
);

-- Handoffs table
CREATE TABLE IF NOT EXISTS caliber_handoff (
    handoff_id UUID PRIMARY KEY,
    from_agent_id UUID NOT NULL,
    to_agent_id UUID,
    to_agent_type TEXT,
    trajectory_id UUID NOT NULL,
    scope_id UUID NOT NULL,
    context_snapshot_id UUID NOT NULL,
    handoff_notes TEXT NOT NULL,
    next_steps TEXT[] DEFAULT '{}',
    blockers TEXT[] DEFAULT '{}',
    open_questions TEXT[] DEFAULT '{}',
    status TEXT DEFAULT 'initiated',
    initiated_at TIMESTAMPTZ NOT NULL,
    accepted_at TIMESTAMPTZ,
    completed_at TIMESTAMPTZ,
    reason TEXT NOT NULL
);

-- Indexes (created once, used by direct heap scans)
CREATE INDEX IF NOT EXISTS idx_agent_type ON caliber_agent(agent_type);
CREATE INDEX IF NOT EXISTS idx_agent_status ON caliber_agent(status);
CREATE INDEX IF NOT EXISTS idx_message_recipient ON caliber_message(to_agent_id, to_agent_type);
CREATE INDEX IF NOT EXISTS idx_message_pending ON caliber_message(delivered_at) WHERE delivered_at IS NULL;
CREATE INDEX IF NOT EXISTS idx_delegation_status ON caliber_delegation(status);
CREATE INDEX IF NOT EXISTS idx_conflict_status ON caliber_conflict(status);
CREATE INDEX IF NOT EXISTS idx_handoff_status ON caliber_handoff(status);
```

---

## Architecture Summary

**NO SQL in hot path:**

- All runtime operations use direct pgrx heap/index access
- `caliber_agent_insert()`, `caliber_message_send()`, etc. are Rust functions
- Postgres advisory locks for distributed coordination
- NOTIFY for real-time message delivery

**SQL only for:**

- One-time bootstrap (`caliber_init()`)
- Human debugging (ad-hoc queries)

**Multi-agent coordination via:**

- Agent registry with typed capabilities
- Memory regions with access control
- Distributed locks (Postgres advisory locks)
- Message passing (heap + NOTIFY)
- Task delegation with context sharing
- Conflict detection and resolution
- Handoff protocol for agent transitions
