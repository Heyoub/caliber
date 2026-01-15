# Design Document: CALIBER TUI

## Overview

CALIBER TUI is a terminal-based interface for the CALIBER memory framework, built with Ratatui and following the SynthBrute visual aesthetic. The system consists of two main components:

1. **caliber-api**: A Rust crate providing REST, gRPC, and WebSocket endpoints that wrap the existing `caliber-pg` PostgreSQL extension
2. **caliber-tui**: A Ratatui-based terminal application that consumes the API

This design ensures a single source of truth for all CALIBER operations, enabling both the TUI and future web dashboard to share the same backend.

## Prerequisites

**✅ PREREQUISITE COMPLETE - READY TO BUILD**

The `caliber-pg` crate has been successfully migrated to production-ready state:
- ✅ Zero compiler warnings - clean build
- ✅ Type alignment complete (ExtractionMethod, CaliberConfig, Checkpoint, RawContent, MemoryAccess, MemoryRegionConfig)
- ✅ Timestamp conversion wired up via chrono_to_timestamp helpers
- ✅ Direct heap operations implemented (heap_form_tuple, simple_heap_insert, index_beginscan)
- ✅ All imports wired up with real functionality - no dead code
- ✅ Helper functions DRY up heap operations (build_optional_* pattern)

**What This Means:**
The caliber-pg extension is now production-ready. All `caliber_*` pg_extern functions are available for the API layer to call. Direct heap operations bypass SQL parsing entirely for maximum performance.

**Dependency Chain:**
1. ~~`caliber-pg-hot-path` spec~~ ✅ **COMPLETE**
2. `caliber-api` crate - REST/gRPC/WebSocket layer ⬅️ **START HERE**
3. `caliber-tui` crate - Terminal UI

## Architecture

### Hot Path Design Principle

**NO SQL IN HOT PATH.** The CALIBER spec mandates that hot-path operations bypass SQL parsing entirely using pgrx direct heap operations. The API layer calls PostgreSQL functions that internally use direct heap tuple manipulation, not SQL queries.

```
┌─────────────────────────────────────────────────────────────────────┐
│                         CALIBER TUI (Ratatui)                       │
├─────────────────────────────────────────────────────────────────────┤
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐  │
│  │Trajectory│ │  Scope   │ │ Artifact │ │   Note   │ │   Turn   │  │
│  │   Tree   │ │ Explorer │ │ Browser  │ │ Library  │ │ History  │  │
│  └──────────┘ └──────────┘ └──────────┘ └──────────┘ └──────────┘  │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐  │
│  │  Agent   │ │   Lock   │ │ Message  │ │   DSL    │ │  Config  │  │
│  │Dashboard │ │ Monitor  │ │  Queue   │ │  Editor  │ │  Viewer  │  │
│  └──────────┘ └──────────┘ └──────────┘ └──────────┘ └──────────┘  │
├─────────────────────────────────────────────────────────────────────┤
│                      State Management Layer                         │
│         (AppState, ViewState, TenantContext, EventQueue)           │
├─────────────────────────────────────────────────────────────────────┤
│                         API Client Layer                            │
│              (REST Client, gRPC Client, WebSocket)                  │
└───────────────────────────┬─────────────────────────────────────────┘
                            │ HTTP/gRPC/WS
                            ▼
┌─────────────────────────────────────────────────────────────────────┐
│                      caliber-api (Axum + Tonic)                     │
├─────────────────────────────────────────────────────────────────────┤
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────────────┐  │
│  │ REST Routes │  │gRPC Services│  │ WebSocket Event Broadcaster │  │
│  │   (Axum)    │  │  (Tonic)    │  │    (tokio broadcast)        │  │
│  └─────────────┘  └─────────────┘  └─────────────────────────────┘  │
├─────────────────────────────────────────────────────────────────────┤
│                    Authentication Middleware                         │
│              (API Key / JWT validation, Tenant extraction)          │
├─────────────────────────────────────────────────────────────────────┤
│                      Database Connection Pool                        │
│                    (deadpool-postgres / tokio-postgres)              │
└───────────────────────────┬─────────────────────────────────────────┘
                            │ Calls caliber_* pg_extern functions
                            │ (NOT raw SQL queries)
                            ▼
┌─────────────────────────────────────────────────────────────────────┐
│                    PostgreSQL + caliber-pg extension                │
├─────────────────────────────────────────────────────────────────────┤
│  pg_extern functions (caliber_trajectory_create, etc.)              │
│                            │                                         │
│                            ▼                                         │
│  ┌─────────────────────────────────────────────────────────────┐    │
│  │              Direct Heap Operations (pgrx)                   │    │
│  │  - heap_form_tuple() for inserts                            │    │
│  │  - index_beginscan() for lookups                            │    │
│  │  - simple_heap_update() for updates                         │    │
│  │  - NO SQL parsing, NO query planning                        │    │
│  └─────────────────────────────────────────────────────────────┘    │
│                            │                                         │
│                            ▼                                         │
│  ┌─────────────────────────────────────────────────────────────┐    │
│  │                    PostgreSQL Heap Storage                   │    │
│  │              (caliber_trajectory, caliber_scope, etc.)       │    │
│  └─────────────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────────────┘
```

### Implementation State

**✅ PRODUCTION READY:** `caliber-pg` now uses direct heap operations with zero SQL parsing overhead:
```rust
// IMPLEMENTED - direct heap manipulation (no SQL)
use crate::heap_ops::{open_relation, insert_tuple};
use crate::tuple_extract::{extract_uuid, extract_text};

let rel = open_relation("caliber_trajectory", AccessShareLock)?;
let values: [Datum; N] = [...];
let nulls: [bool; N] = [...];
let tid = insert_tuple(&rel, &values, &nulls)?;
```

**For API Developers:** The API layer calls the existing `caliber_*` pg_extern functions via PostgreSQL connection. These functions internally use direct heap operations - you don't need to worry about the implementation details. Just call the functions and enjoy the performance.

## Components and Interfaces

### caliber-api Crate

#### REST API Routes

```rust
// Trajectory endpoints
POST   /api/v1/trajectories
GET    /api/v1/trajectories
GET    /api/v1/trajectories/{id}
PATCH  /api/v1/trajectories/{id}
DELETE /api/v1/trajectories/{id}
GET    /api/v1/trajectories/{id}/scopes
GET    /api/v1/trajectories/{id}/children

// Scope endpoints
POST   /api/v1/scopes
GET    /api/v1/scopes/{id}
PATCH  /api/v1/scopes/{id}
POST   /api/v1/scopes/{id}/checkpoint
POST   /api/v1/scopes/{id}/close
GET    /api/v1/scopes/{id}/turns
GET    /api/v1/scopes/{id}/artifacts

// Artifact endpoints
POST   /api/v1/artifacts
GET    /api/v1/artifacts
GET    /api/v1/artifacts/{id}
PATCH  /api/v1/artifacts/{id}
DELETE /api/v1/artifacts/{id}
POST   /api/v1/artifacts/search

// Note endpoints
POST   /api/v1/notes
GET    /api/v1/notes
GET    /api/v1/notes/{id}
PATCH  /api/v1/notes/{id}
DELETE /api/v1/notes/{id}
POST   /api/v1/notes/search

// Turn endpoints
POST   /api/v1/turns
GET    /api/v1/turns/{id}

// Agent endpoints
POST   /api/v1/agents
GET    /api/v1/agents
GET    /api/v1/agents/{id}
PATCH  /api/v1/agents/{id}
DELETE /api/v1/agents/{id}
POST   /api/v1/agents/{id}/heartbeat

// Lock endpoints
POST   /api/v1/locks/acquire
POST   /api/v1/locks/{id}/release
POST   /api/v1/locks/{id}/extend
GET    /api/v1/locks

// Message endpoints
POST   /api/v1/messages
GET    /api/v1/messages
POST   /api/v1/messages/{id}/acknowledge

// Delegation endpoints
POST   /api/v1/delegations
GET    /api/v1/delegations/{id}
POST   /api/v1/delegations/{id}/accept
POST   /api/v1/delegations/{id}/reject
POST   /api/v1/delegations/{id}/complete

// Handoff endpoints
POST   /api/v1/handoffs
GET    /api/v1/handoffs/{id}
POST   /api/v1/handoffs/{id}/accept
POST   /api/v1/handoffs/{id}/complete

// DSL endpoints
POST   /api/v1/dsl/validate
POST   /api/v1/dsl/parse

// Config endpoints
GET    /api/v1/config
PATCH  /api/v1/config
POST   /api/v1/config/validate

// Tenant endpoints
GET    /api/v1/tenants
GET    /api/v1/tenants/{id}

// WebSocket
GET    /api/v1/ws
```

#### gRPC Service Definitions

```protobuf
service CaliberService {
  // Trajectories
  rpc CreateTrajectory(CreateTrajectoryRequest) returns (Trajectory);
  rpc GetTrajectory(GetTrajectoryRequest) returns (Trajectory);
  rpc ListTrajectories(ListTrajectoriesRequest) returns (ListTrajectoriesResponse);
  rpc UpdateTrajectory(UpdateTrajectoryRequest) returns (Trajectory);
  rpc DeleteTrajectory(DeleteTrajectoryRequest) returns (Empty);
  
  // Scopes
  rpc CreateScope(CreateScopeRequest) returns (Scope);
  rpc GetScope(GetScopeRequest) returns (Scope);
  rpc CloseScope(CloseScopeRequest) returns (Scope);
  rpc CreateCheckpoint(CreateCheckpointRequest) returns (Checkpoint);
  
  // ... similar for other entities
  
  // Streaming
  rpc SubscribeEvents(SubscribeRequest) returns (stream Event);
}
```

#### WebSocket Event Types

```rust
#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum WsEvent {
    // Entity events
    TrajectoryCreated { trajectory: Trajectory },
    TrajectoryUpdated { trajectory: Trajectory },
    TrajectoryDeleted { id: EntityId },
    
    ScopeCreated { scope: Scope },
    ScopeUpdated { scope: Scope },
    ScopeClosed { scope: Scope },
    
    ArtifactCreated { artifact: Artifact },
    ArtifactUpdated { artifact: Artifact },
    ArtifactDeleted { id: EntityId },
    
    NoteCreated { note: Note },
    NoteUpdated { note: Note },
    NoteDeleted { id: EntityId },
    
    TurnCreated { turn: Turn },
    
    // Agent events
    AgentRegistered { agent: Agent },
    AgentStatusChanged { agent_id: EntityId, status: AgentStatus },
    AgentHeartbeat { agent_id: EntityId, timestamp: Timestamp },
    
    // Lock events
    LockAcquired { lock: DistributedLock },
    LockReleased { lock_id: EntityId },
    LockExpired { lock_id: EntityId },
    
    // Message events
    MessageSent { message: AgentMessage },
    MessageDelivered { message_id: EntityId },
    MessageAcknowledged { message_id: EntityId },
    
    // Connection events
    Connected { tenant_id: EntityId },
    Disconnected { reason: String },
    Error { message: String },
}
```

### caliber-tui Crate

#### Application State

```rust
pub struct App {
    /// Current tenant context
    pub tenant: TenantContext,
    
    /// Active view
    pub active_view: View,
    
    /// View-specific state
    pub trajectory_view: TrajectoryViewState,
    pub scope_view: ScopeViewState,
    pub artifact_view: ArtifactViewState,
    pub note_view: NoteViewState,
    pub turn_view: TurnViewState,
    pub agent_view: AgentViewState,
    pub lock_view: LockViewState,
    pub message_view: MessageViewState,
    pub dsl_view: DslViewState,
    pub config_view: ConfigViewState,
    
    /// Global state
    pub notifications: Vec<Notification>,
    pub command_palette: Option<CommandPalette>,
    pub search: Option<GlobalSearch>,
    pub modal: Option<Modal>,
    
    /// Real-time state
    pub ws_connected: bool,
    pub updates_paused: bool,
    pub event_queue: VecDeque<WsEvent>,
    
    /// API client
    pub api: ApiClient,
}

pub enum View {
    TrajectoryTree,
    ScopeExplorer,
    ArtifactBrowser,
    NoteLibrary,
    TurnHistory,
    AgentDashboard,
    LockMonitor,
    MessageQueue,
    DslEditor,
    ConfigViewer,
}

pub struct TenantContext {
    pub tenant_id: EntityId,
    pub tenant_name: String,
    pub available_tenants: Vec<TenantInfo>,
}
```

#### View State Examples

```rust
pub struct TrajectoryViewState {
    /// All trajectories (flat list, tree built on render)
    pub trajectories: Vec<Trajectory>,
    
    /// Expanded node IDs
    pub expanded: HashSet<EntityId>,
    
    /// Currently selected trajectory
    pub selected: Option<EntityId>,
    
    /// Filter state
    pub filter: TrajectoryFilter,
    
    /// Loading state
    pub loading: bool,
}

pub struct TrajectoryFilter {
    pub status: Option<TrajectoryStatus>,
    pub agent_id: Option<EntityId>,
    pub date_from: Option<Timestamp>,
    pub date_to: Option<Timestamp>,
    pub search_query: Option<String>,
}

pub struct ArtifactViewState {
    pub artifacts: Vec<Artifact>,
    pub selected: Option<EntityId>,
    pub filter: ArtifactFilter,
    pub sort: ArtifactSort,
    pub search_query: String,
    pub loading: bool,
}

pub struct DslViewState {
    pub content: String,
    pub cursor_position: (usize, usize),
    pub parse_errors: Vec<ParseError>,
    pub ast_preview: Option<CaliberAst>,
    pub file_path: Option<PathBuf>,
    pub modified: bool,
}
```

#### Widget Components

```rust
// Reusable widget components
pub struct TreeWidget<'a> {
    items: &'a [TreeItem],
    selected: Option<usize>,
    expanded: &'a HashSet<EntityId>,
    style: TreeStyle,
}

pub struct DetailPanel<'a> {
    title: &'a str,
    fields: Vec<(&'a str, String)>,
    style: PanelStyle,
}

pub struct FilterBar<'a> {
    filters: &'a [FilterOption],
    active: &'a [usize],
}

pub struct ProgressBar {
    value: f32,
    max: f32,
    thresholds: (f32, f32), // (warning, critical)
}

pub struct StatusIndicator {
    status: Status,
    label: Option<String>,
}

pub struct SyntaxHighlighter {
    language: Language,
    theme: Theme,
}
```

## Data Models

### API Request/Response Types

```rust
// Trajectory
#[derive(Serialize, Deserialize)]
pub struct CreateTrajectoryRequest {
    pub name: String,
    pub description: Option<String>,
    pub parent_trajectory_id: Option<EntityId>,
    pub agent_id: Option<EntityId>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Serialize, Deserialize)]
pub struct UpdateTrajectoryRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub status: Option<TrajectoryStatus>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Serialize, Deserialize)]
pub struct ListTrajectoriesRequest {
    pub status: Option<TrajectoryStatus>,
    pub agent_id: Option<EntityId>,
    pub parent_id: Option<EntityId>,
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}

// Similar patterns for other entities...

// Search
#[derive(Serialize, Deserialize)]
pub struct SearchRequest {
    pub query: String,
    pub entity_types: Vec<EntityType>,
    pub filters: Vec<FilterExpr>,
    pub limit: Option<i32>,
}

#[derive(Serialize, Deserialize)]
pub struct SearchResult {
    pub entity_type: EntityType,
    pub id: EntityId,
    pub name: String,
    pub snippet: String,
    pub score: f32,
}

// DSL
#[derive(Serialize, Deserialize)]
pub struct ValidateDslRequest {
    pub source: String,
}

#[derive(Serialize, Deserialize)]
pub struct ValidateDslResponse {
    pub valid: bool,
    pub errors: Vec<ParseError>,
    pub ast: Option<CaliberAst>,
}

// Tenant
#[derive(Serialize, Deserialize)]
pub struct TenantInfo {
    pub tenant_id: EntityId,
    pub name: String,
    pub status: TenantStatus,
    pub created_at: Timestamp,
}

#[derive(Serialize, Deserialize)]
pub enum TenantStatus {
    Active,
    Suspended,
    Archived,
}
```

### Color Theme

```rust
pub struct SynthBruteTheme {
    // Background
    pub bg: Color,           // #0a0a0a
    pub bg_secondary: Color, // #1a1a1a
    pub bg_highlight: Color, // #2a2a2a
    
    // Primary accent (cyan)
    pub primary: Color,      // #00ffff
    pub primary_dim: Color,  // #008888
    
    // Secondary accent (magenta)
    pub secondary: Color,    // #ff00ff
    pub secondary_dim: Color,// #880088
    
    // Tertiary accent (yellow)
    pub tertiary: Color,     // #ffff00
    pub tertiary_dim: Color, // #888800
    
    // Status colors
    pub success: Color,      // #00ff00
    pub warning: Color,      // #ffff00
    pub error: Color,        // #ff0000
    pub info: Color,         // #00ffff
    
    // Text
    pub text: Color,         // #ffffff
    pub text_dim: Color,     // #888888
    pub text_muted: Color,   // #444444
    
    // Borders
    pub border: Color,       // #444444
    pub border_focus: Color, // #00ffff
}

impl Default for SynthBruteTheme {
    fn default() -> Self {
        Self {
            bg: Color::Rgb(10, 10, 10),
            bg_secondary: Color::Rgb(26, 26, 26),
            bg_highlight: Color::Rgb(42, 42, 42),
            primary: Color::Rgb(0, 255, 255),
            primary_dim: Color::Rgb(0, 136, 136),
            secondary: Color::Rgb(255, 0, 255),
            secondary_dim: Color::Rgb(136, 0, 136),
            tertiary: Color::Rgb(255, 255, 0),
            tertiary_dim: Color::Rgb(136, 136, 0),
            success: Color::Rgb(0, 255, 0),
            warning: Color::Rgb(255, 255, 0),
            error: Color::Rgb(255, 0, 0),
            info: Color::Rgb(0, 255, 255),
            text: Color::Rgb(255, 255, 255),
            text_dim: Color::Rgb(136, 136, 136),
            text_muted: Color::Rgb(68, 68, 68),
            border: Color::Rgb(68, 68, 68),
            border_focus: Color::Rgb(0, 255, 255),
        }
    }
}
```

## Correctness Properties

*A property is a characteristic or behavior that should hold true across all valid executions of a system—essentially, a formal statement about what the system should do. Properties serve as the bridge between human-readable specifications and machine-verifiable correctness guarantees.*

### Property 1: API Completeness

*For any* entity type in CALIBER (Trajectory, Scope, Artifact, Note, Turn, Agent, Lock, Message), the API SHALL expose CRUD endpoints that return valid responses matching the entity schema.

**Validates: Requirements 1.1, 1.2**

### Property 2: REST-gRPC Parity

*For any* REST endpoint, there SHALL exist an equivalent gRPC method that accepts equivalent input and returns equivalent output.

**Validates: Requirements 1.1, 1.2**

### Property 3: Mutation Broadcast

*For any* entity mutation (create, update, delete) performed via the API, a corresponding WebSocket event SHALL be broadcast to all subscribed clients within 100ms.

**Validates: Requirements 1.4, 3.9, 4.8, 5.11, 6.10, 7.9, 8.10, 9.8, 10.8**

### Property 4: Authentication Enforcement

*For any* API request, IF the request lacks valid authentication THEN the API SHALL return 401 Unauthorized, AND IF the request targets a tenant the user lacks access to THEN the API SHALL return 403 Forbidden.

**Validates: Requirements 1.5, 1.7, 1.8**

### Property 5: Tenant Isolation

*For any* authenticated request with a tenant context header, the API SHALL return ONLY data belonging to that tenant, AND mutations SHALL only affect that tenant's data.

**Validates: Requirements 1.6, 2.5**

### Property 6: Status-to-Color Mapping

*For any* entity with a status field (Trajectory, Agent, Lock, Message), the TUI SHALL render the status using the correct color from the SynthBrute theme:
- Active/Idle → cyan (#00ffff)
- Completed/Success → green (#00ff00)
- Failed/Error → red (#ff0000)
- Suspended/Blocked/Warning → yellow (#ffff00)
- Dim states → corresponding dim variant

**Validates: Requirements 3.3, 4.3, 8.2, 10.2, 13.2, 13.3, 13.4**

### Property 7: Filter Correctness

*For any* filter applied to a list view (trajectories, artifacts, notes, turns, messages, locks), the displayed items SHALL be exactly the set of items that match ALL active filter criteria.

**Validates: Requirements 3.8, 5.2, 5.3, 5.4, 6.2, 6.3, 7.7, 7.8, 9.7, 10.5, 10.6**

### Property 8: Hierarchy Rendering

*For any* parent-child relationship (Trajectory→Trajectory, Trajectory→Scope, Scope→Turn), the TUI SHALL render children nested under their parent in the tree/list view.

**Validates: Requirements 3.1, 4.1**

### Property 9: Detail Panel Completeness

*For any* selected entity, the detail panel SHALL display ALL non-null fields defined in the entity's schema.

**Validates: Requirements 3.2, 4.2, 4.7, 5.6, 6.4, 6.5, 7.3, 7.4, 8.3, 8.4, 8.5**

### Property 10: Token Utilization Calculation

*For any* scope with token_budget > 0, the utilization percentage SHALL equal (tokens_used / token_budget) * 100, AND the progress bar color SHALL be:
- Green if utilization < 70%
- Yellow if 70% ≤ utilization < 90%
- Red if utilization ≥ 90%

**Validates: Requirements 4.2, 4.3**

### Property 11: DSL Syntax Highlighting

*For any* CALIBER DSL source text, the editor SHALL apply syntax highlighting where:
- Keywords (caliber, memory, policy, adapter, inject) → cyan
- Memory types (ephemeral, working, episodic, semantic, procedural, meta) → magenta
- Field types (uuid, text, int, float, bool, timestamp, json, embedding) → yellow
- Strings → green
- Numbers → orange

**Validates: Requirements 11.1, 11.2, 11.3, 11.4, 11.5**

### Property 12: DSL Validation Round-Trip

*For any* valid CALIBER DSL source, parsing then pretty-printing then parsing again SHALL produce an equivalent AST.

**Validates: Requirements 11.6, 11.7**

### Property 13: Keybinding Consistency

*For any* view in the TUI, the standard navigation keys (h/j/k/l, Tab, number keys, q, ?, /, :) SHALL perform their documented actions consistently.

**Validates: Requirements 14.1, 14.2, 14.3, 14.5, 14.6, 14.7, 14.8**

### Property 14: WebSocket Reconnection

*For any* WebSocket disconnection, the TUI SHALL attempt reconnection with exponential backoff, AND display connection status to the user.

**Validates: Requirements 15.1, 15.2**

### Property 15: Error Display

*For any* API error response, the TUI SHALL display the error message in the notification area with appropriate color coding (red for errors).

**Validates: Requirements 16.1, 16.2, 16.3**

## Error Handling

### API Layer Errors

```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct ApiError {
    pub code: ErrorCode,
    pub message: String,
    pub details: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ErrorCode {
    // Authentication
    Unauthorized,
    Forbidden,
    InvalidToken,
    TokenExpired,
    
    // Validation
    ValidationFailed,
    InvalidInput,
    MissingField,
    
    // Not Found
    EntityNotFound,
    TenantNotFound,
    
    // Conflict
    EntityAlreadyExists,
    ConcurrentModification,
    LockConflict,
    
    // Server
    InternalError,
    DatabaseError,
    ServiceUnavailable,
}
```

### TUI Error Handling

```rust
pub enum TuiError {
    /// API request failed
    Api(ApiError),
    
    /// WebSocket connection error
    WebSocket(String),
    
    /// File I/O error
    FileIo(std::io::Error),
    
    /// Terminal rendering error
    Render(std::io::Error),
    
    /// Configuration error
    Config(String),
}

impl App {
    fn handle_error(&mut self, error: TuiError) {
        let notification = match error {
            TuiError::Api(e) => Notification {
                level: NotificationLevel::Error,
                message: e.message,
                action: Some(NotificationAction::Retry),
            },
            TuiError::WebSocket(msg) => Notification {
                level: NotificationLevel::Warning,
                message: format!("Connection lost: {}", msg),
                action: Some(NotificationAction::Reconnect),
            },
            // ...
        };
        
        self.notifications.push(notification);
        self.log_error(&error);
    }
}
```

## Testing Strategy

### Unit Tests

Unit tests focus on specific examples and edge cases:

- API route handlers with mock database
- Request/response serialization
- Filter logic with specific inputs
- Color mapping for each status value
- DSL syntax highlighting for specific tokens
- Keybinding dispatch

### Property-Based Tests

Property-based tests verify universal properties across generated inputs:

- **API Completeness**: Generate random entity types, verify endpoints exist
- **Mutation Broadcast**: Generate random mutations, verify WebSocket events
- **Tenant Isolation**: Generate cross-tenant requests, verify isolation
- **Filter Correctness**: Generate random filters and data, verify results
- **DSL Round-Trip**: Generate random valid ASTs, verify parse→print→parse equivalence
- **Color Mapping**: Generate all status values, verify correct colors

### Integration Tests

- Full API→Database round-trips
- WebSocket event streaming
- TUI rendering with mock API
- Multi-tenant scenarios

### Test Configuration

- Property tests: minimum 100 iterations per property
- Use `proptest` for Rust property-based testing
- Tag format: `Feature: caliber-tui, Property {N}: {description}`

