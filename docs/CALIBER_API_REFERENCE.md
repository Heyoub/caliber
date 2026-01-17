# CALIBER API Reference

> Quick reference for all CALIBER API endpoints

**Base URL:** `https://api.caliber.run` (production) or `http://localhost:3000` (local)

**Authentication:** Bearer token in `Authorization` header
```
Authorization: Bearer <api_key>
```

**Tenant ID:** Required in `X-Tenant-ID` header for all operations
```
X-Tenant-ID: <tenant_uuid>
```

---

## Trajectories

Trajectories are task containers - the primary unit of work.

### Create Trajectory
```http
POST /trajectories
Content-Type: application/json

{
  "name": "Fix bug #123",
  "description": "Optional description",
  "parent_trajectory_id": "uuid",  // Optional, for sub-tasks
  "agent_id": "uuid",              // Optional
  "metadata": {}                   // Optional JSON
}
```

**Response:** `201 Created`
```json
{
  "trajectory_id": "uuid",
  "name": "Fix bug #123",
  "status": "Active",
  "created_at": "2024-01-15T10:00:00Z",
  ...
}
```

### Get Trajectory
```http
GET /trajectories/:trajectory_id
```

### List Trajectories
```http
GET /trajectories?status=Active&agent_id=uuid&limit=50&offset=0
```

### Update Trajectory
```http
PATCH /trajectories/:trajectory_id
Content-Type: application/json

{
  "name": "New name",
  "status": "Completed",
  "metadata": {}
}
```

### Delete Trajectory
```http
DELETE /trajectories/:trajectory_id
```

---

## Scopes

Scopes are context windows with token budgets.

### Create Scope
```http
POST /scopes
Content-Type: application/json

{
  "trajectory_id": "uuid",
  "name": "Implementation phase",
  "purpose": "Implementing the auth feature",
  "token_budget": 8000,
  "parent_scope_id": "uuid",  // Optional, for nested scopes
  "metadata": {}
}
```

### Get Scope
```http
GET /scopes/:scope_id
```

### Update Scope
```http
PATCH /scopes/:scope_id
Content-Type: application/json

{
  "name": "Updated name",
  "token_budget": 16000
}
```

### Create Checkpoint
```http
POST /scopes/:scope_id/checkpoint
Content-Type: application/json

{
  "context_state": "base64_encoded_state",
  "recoverable": true
}
```

### Close Scope
```http
POST /scopes/:scope_id/close
```

**Note:** Closing a scope deletes all turns within it. Extract artifacts first!

---

## Turns

Turns are individual conversation messages. Ephemeral - deleted when scope closes.

### Create Turn
```http
POST /turns
Content-Type: application/json

{
  "scope_id": "uuid",
  "sequence": 1,
  "role": "User",           // User, Assistant, System, Tool
  "content": "Message text",
  "token_count": 150,
  "tool_calls": {},         // Optional
  "tool_results": {},       // Optional
  "metadata": {}
}
```

### Get Turn
```http
GET /turns/:turn_id
```

---

## Artifacts

Artifacts are extracted value from conversations. They persist after scope closes.

### Create Artifact
```http
POST /artifacts
Content-Type: application/json

{
  "trajectory_id": "uuid",
  "scope_id": "uuid",
  "artifact_type": "Code",  // Code, Document, Decision, Summary, etc.
  "name": "auth-handler.ts",
  "content": "export function authenticate() {...}",
  "source_turn": 5,
  "extraction_method": "Explicit",  // Explicit, Inferred, UserProvided
  "confidence": 0.95,               // Optional, 0.0-1.0
  "ttl": "Persistent",              // Persistent, Session, Scope, etc.
  "metadata": {}
}
```

### Get Artifact
```http
GET /artifacts/:artifact_id
```

### List Artifacts
```http
GET /artifacts?trajectory_id=uuid&scope_id=uuid&artifact_type=Code&limit=50
```

### Update Artifact
```http
PATCH /artifacts/:artifact_id
Content-Type: application/json

{
  "name": "Updated name",
  "content": "New content",
  "ttl": "LongTerm"
}
```

### Delete Artifact
```http
DELETE /artifacts/:artifact_id
```

### Batch Artifacts
```http
POST /artifacts/batch
Content-Type: application/json

{
  "items": [
    { "operation": "create", "create": { ... } },
    { "operation": "update", "artifact_id": "uuid", "update": { ... } },
    { "operation": "delete", "artifact_id": "uuid" }
  ],
  "stop_on_error": false
}
```

---

## Notes

Notes are cross-trajectory knowledge that persists globally.

### Create Note
```http
POST /notes
Content-Type: application/json

{
  "note_type": "Convention",  // Convention, Strategy, Gotcha, Procedure, etc.
  "title": "TypeScript strict mode",
  "content": "Always use strict TypeScript in this project.",
  "source_trajectory_ids": ["uuid", ...],
  "source_artifact_ids": ["uuid", ...],
  "ttl": "Persistent",
  "metadata": {}
}
```

### Get Note
```http
GET /notes/:note_id
```

### List Notes
```http
GET /notes?note_type=Convention&source_trajectory_id=uuid&limit=50
```

### Update Note
```http
PATCH /notes/:note_id
Content-Type: application/json

{
  "title": "Updated title",
  "content": "New content"
}
```

### Delete Note
```http
DELETE /notes/:note_id
```

### Batch Notes
```http
POST /notes/batch
Content-Type: application/json

{
  "items": [
    { "operation": "create", "create": { ... } },
    { "operation": "update", "note_id": "uuid", "update": { ... } }
  ]
}
```

---

## Agents

Agents are registered workers with capabilities and permissions.

### Register Agent
```http
POST /agents/register
Content-Type: application/json

{
  "agent_type": "coder",
  "capabilities": ["write_code", "run_tests"],
  "memory_access": {
    "read": [{ "memory_type": "Note", "scope": "all" }],
    "write": [{ "memory_type": "Artifact", "scope": "own_trajectory" }]
  },
  "can_delegate_to": ["reviewer", "tester"],
  "reports_to": "uuid"  // Optional supervisor
}
```

### Get Agent
```http
GET /agents/:agent_id
```

### Update Agent
```http
PATCH /agents/:agent_id
Content-Type: application/json

{
  "status": "busy",
  "current_trajectory_id": "uuid"
}
```

### Heartbeat
```http
POST /agents/:agent_id/heartbeat
```

### Unregister Agent
```http
DELETE /agents/:agent_id
```

---

## Locks

Locks provide exclusive or shared access to resources.

### Acquire Lock
```http
POST /locks/acquire
Content-Type: application/json

{
  "resource_type": "trajectory",
  "resource_id": "uuid",
  "holder_agent_id": "uuid",
  "timeout_ms": 30000,
  "mode": "Exclusive"  // Exclusive or Shared
}
```

### Release Lock
```http
DELETE /locks/:lock_id/release
```

### Extend Lock
```http
PATCH /locks/:lock_id/extend
Content-Type: application/json

{
  "additional_ms": 30000
}
```

---

## Messages

Async messages between agents.

### Send Message
```http
POST /messages
Content-Type: application/json

{
  "from_agent_id": "uuid",
  "to_agent_id": "uuid",      // Optional, for targeted
  "to_agent_type": "reviewer", // Optional, for broadcast
  "message_type": "task_complete",
  "payload": "{\"result\": \"success\"}",
  "trajectory_id": "uuid",     // Optional context
  "scope_id": "uuid",
  "artifact_ids": ["uuid", ...],
  "priority": "normal",        // low, normal, high, urgent
  "expires_at": "2024-01-15T12:00:00Z"  // Optional
}
```

### Get Message
```http
GET /messages/:message_id
```

### List Messages (for agent)
```http
GET /agents/:agent_id/messages?unacknowledged=true
```

### Acknowledge Message
```http
POST /messages/:message_id/acknowledge
```

---

## Delegations

Delegations are sub-task assignments between agents.

### Create Delegation
```http
POST /delegations
Content-Type: application/json

{
  "from_agent_id": "uuid",
  "to_agent_id": "uuid",
  "trajectory_id": "uuid",
  "scope_id": "uuid",
  "task_description": "Review this code for security issues",
  "expected_completion": "2024-01-15T14:00:00Z",
  "context": {}
}
```

### Accept Delegation
```http
POST /delegations/:delegation_id/accept
```

### Reject Delegation
```http
POST /delegations/:delegation_id/reject
Content-Type: application/json

{
  "reason": "Too busy with other tasks"
}
```

### Complete Delegation
```http
POST /delegations/:delegation_id/complete
Content-Type: application/json

{
  "status": "success",
  "output": "No security issues found",
  "artifacts": ["uuid", ...],
  "error": null
}
```

---

## Handoffs

Handoffs transfer full control of a trajectory between agents.

### Create Handoff
```http
POST /handoffs
Content-Type: application/json

{
  "from_agent_id": "uuid",
  "to_agent_id": "uuid",
  "trajectory_id": "uuid",
  "scope_id": "uuid",
  "reason": "Need human review for security decision",
  "context_snapshot": "base64_encoded_context"
}
```

### Accept Handoff
```http
POST /handoffs/:handoff_id/accept
```

### Complete Handoff
```http
POST /handoffs/:handoff_id/complete
```

---

## Search

Global search across all entities.

### Search
```http
POST /search
Content-Type: application/json

{
  "query": "authentication error handling",
  "entity_types": ["Artifact", "Note"],
  "filters": [
    { "field": "created_at", "operator": "gt", "value": "2024-01-01" }
  ],
  "limit": 20
}
```

**Response:**
```json
{
  "results": [
    {
      "entity_type": "Artifact",
      "id": "uuid",
      "name": "auth-error-handler.ts",
      "snippet": "...handles authentication errors...",
      "score": 0.95
    }
  ],
  "total": 15
}
```

---

## DSL

Validate and parse CALIBER DSL configurations.

### Validate DSL
```http
POST /dsl/validate
Content-Type: application/json

{
  "source": "agent Coder { capabilities: [write_code] }"
}
```

**Response:**
```json
{
  "valid": true,
  "errors": [],
  "ast": { ... }
}
```

### Parse DSL
```http
POST /dsl/parse
Content-Type: application/json

{
  "source": "agent Coder { capabilities: [write_code] }"
}
```

---

## Health

Health check endpoints.

### Liveness
```http
GET /health/live
```

### Readiness
```http
GET /health/ready
```

---

## WebSocket Events

Connect to `/ws` for real-time events.

**Event Types:**
- `trajectory.created`, `trajectory.updated`, `trajectory.completed`
- `scope.created`, `scope.closed`
- `artifact.created`, `artifact.updated`
- `note.created`, `note.updated`
- `agent.registered`, `agent.heartbeat`
- `message.received`
- `delegation.created`, `delegation.accepted`, `delegation.completed`
- `handoff.created`, `handoff.accepted`

**Event Format:**
```json
{
  "event_type": "artifact.created",
  "entity_id": "uuid",
  "tenant_id": "uuid",
  "timestamp": "2024-01-15T10:00:00Z",
  "data": { ... }
}
```

---

## Error Responses

All errors follow this format:

```json
{
  "error": {
    "code": "NOT_FOUND",
    "message": "Trajectory not found",
    "details": {}
  }
}
```

**Common Status Codes:**
- `400` - Bad Request (validation error)
- `401` - Unauthorized (missing/invalid token)
- `403` - Forbidden (insufficient permissions)
- `404` - Not Found
- `409` - Conflict (e.g., lock contention)
- `422` - Unprocessable Entity (semantic error)
- `500` - Internal Server Error

---

## Enums Reference

### TrajectoryStatus
`Active`, `Completed`, `Failed`, `Suspended`

### TurnRole
`User`, `Assistant`, `System`, `Tool`

### ArtifactType
`Code`, `Document`, `Data`, `Config`, `Log`, `Summary`, `Decision`, `Plan`, `ErrorLog`, `CodePatch`, `DesignDecision`, `UserPreference`, `Fact`, `Constraint`, `ToolResult`, `IntermediateOutput`, `Custom`

### NoteType
`Convention`, `Strategy`, `Gotcha`, `Fact`, `Preference`, `Relationship`, `Procedure`, `Meta`, `Insight`, `Correction`, `Summary`

### ExtractionMethod
`Explicit`, `Inferred`, `UserProvided`

### TTL
`Persistent`, `Session`, `Scope`, `Ephemeral`, `ShortTerm`, `MediumTerm`, `LongTerm`, `Permanent`, `Duration(ms)`

### EntityType
`Trajectory`, `Scope`, `Artifact`, `Note`, `Turn`, `Lock`, `Message`, `Agent`, `Delegation`, `Handoff`, `Conflict`, `Edge`
