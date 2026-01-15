# Requirements Document

## Introduction

CALIBER TUI is a terminal-based user interface for the CALIBER memory framework. It provides developers and operators with a full-featured interface to manage AI agent memory, monitor multi-agent coordination, and configure the system. The TUI connects exclusively through a REST/gRPC API layer (`caliber-api` crate), which will also serve the managed web dashboard. The visual design follows the SynthBrute aesthetic (dark background, neon accents, brutalist boxes) using Ratatui.

## Glossary

- **TUI**: Terminal User Interface - the Ratatui-based application
- **API_Layer**: The `caliber-api` crate providing REST/gRPC endpoints
- **Tenant**: An isolated CALIBER instance/project (like Vercel projects)
- **Trajectory**: Top-level task container in CALIBER's memory hierarchy
- **Scope**: Partitioned context window within a trajectory
- **Artifact**: Typed output preserved across scopes
- **Note**: Long-term cross-trajectory knowledge
- **Turn**: Ephemeral conversation buffer entry
- **Agent**: An AI agent registered in the multi-agent system
- **Lock**: Distributed lock for resource coordination
- **Message**: Inter-agent communication payload
- **DSL**: CALIBER's domain-specific language for configuration
- **SynthBrute**: Visual style combining synthwave aesthetics with neo-brutalist design

## Requirements

### Requirement 1: API Layer Foundation

**User Story:** As a developer, I want a unified API layer, so that both TUI and web dashboard consume the same endpoints without redundancy.

#### Acceptance Criteria

1. THE API_Layer SHALL expose REST endpoints for all CALIBER entity operations (trajectories, scopes, artifacts, notes, turns, agents, locks, messages)
2. THE API_Layer SHALL expose gRPC endpoints mirroring REST functionality for high-performance clients
3. THE API_Layer SHALL support WebSocket connections for real-time NOTIFY event streaming
4. WHEN an entity is created, updated, or deleted, THE API_Layer SHALL broadcast the change via WebSocket to subscribed clients
5. THE API_Layer SHALL authenticate requests using API keys or JWT tokens
6. THE API_Layer SHALL support tenant context headers for multi-tenant isolation
7. IF an unauthenticated request is received, THEN THE API_Layer SHALL return 401 Unauthorized
8. IF a request targets a tenant the user lacks access to, THEN THE API_Layer SHALL return 403 Forbidden
9. THE API_Layer SHALL call caliber_* pg_extern functions via PostgreSQL connection, NOT raw SQL queries
10. THE API_Layer SHALL NOT bypass the caliber-pg extension by writing SQL directly to caliber_* tables

### Requirement 2: Tenant Management

**User Story:** As an infrastructure developer managing multiple agentic projects, I want to switch between tenants, so that I can manage memory for different clients like switching Vercel projects.

#### Acceptance Criteria

1. WHEN the TUI starts, THE TUI SHALL display a tenant selector if multiple tenants are available
2. THE TUI SHALL display the current tenant name prominently in the status bar
3. WHEN a user presses the tenant switch hotkey, THE TUI SHALL open a tenant picker modal
4. THE TUI SHALL list all tenants the user has access to with their status indicators
5. WHEN a user selects a different tenant, THE TUI SHALL switch context and refresh all views
6. THE TUI SHALL persist the last-used tenant preference locally
7. IF only one tenant is available, THEN THE TUI SHALL skip the tenant selector and auto-connect

### Requirement 3: Trajectory Tree View

**User Story:** As a developer, I want to see trajectories in a hierarchical tree, so that I can understand task relationships and navigate the memory structure.

#### Acceptance Criteria

1. THE TUI SHALL display trajectories as a collapsible tree structure showing parent-child relationships
2. WHEN a trajectory is selected, THE TUI SHALL display its details in a side panel (name, description, status, agent, timestamps)
3. THE TUI SHALL color-code trajectory nodes by status (active=cyan, completed=green, failed=red, suspended=yellow)
4. WHEN a user presses Enter on a trajectory, THE TUI SHALL expand/collapse its children
5. WHEN a user presses 'n', THE TUI SHALL open a form to create a new trajectory
6. WHEN a user presses 'e', THE TUI SHALL open a form to edit the selected trajectory
7. WHEN a user presses 'd', THE TUI SHALL prompt for confirmation then delete the trajectory
8. THE TUI SHALL support filtering trajectories by status, agent, or date range
9. WHEN new trajectories are created via API, THE TUI SHALL update the tree in real-time

### Requirement 4: Scope Explorer View

**User Story:** As a developer, I want to explore scopes within trajectories, so that I can understand context partitioning and token usage.

#### Acceptance Criteria

1. THE TUI SHALL display scopes as nested items under their parent trajectory
2. WHEN a scope is selected, THE TUI SHALL show token budget, tokens used, and utilization percentage
3. THE TUI SHALL visualize token utilization as a progress bar with color coding (green <70%, yellow 70-90%, red >90%)
4. WHEN a scope has a checkpoint, THE TUI SHALL display a checkpoint indicator icon
5. WHEN a user presses 'c' on a scope, THE TUI SHALL trigger checkpoint creation via API
6. WHEN a user presses 'x' on an active scope, THE TUI SHALL close the scope via API
7. THE TUI SHALL display scope purpose and metadata in the detail panel
8. WHEN scopes are created or closed, THE TUI SHALL update the view in real-time

### Requirement 5: Artifact Browser View

**User Story:** As a developer, I want to browse and search artifacts, so that I can find preserved outputs and understand what knowledge has been extracted.

#### Acceptance Criteria

1. THE TUI SHALL display artifacts in a filterable, sortable list
2. THE TUI SHALL support filtering artifacts by type (ErrorLog, CodePatch, DesignDecision, UserPreference, Fact, Constraint, ToolResult, IntermediateOutput, Custom)
3. THE TUI SHALL support filtering artifacts by trajectory, scope, or date range
4. THE TUI SHALL support full-text search across artifact names and content
5. WHEN an artifact is selected, THE TUI SHALL display its full content with syntax highlighting where applicable
6. THE TUI SHALL display artifact provenance (source turn, extraction method, confidence)
7. WHEN a user presses 'n', THE TUI SHALL open a form to create a new artifact
8. WHEN a user presses 'e', THE TUI SHALL open a form to edit the selected artifact
9. WHEN a user presses 'd', THE TUI SHALL prompt for confirmation then delete the artifact
10. IF an artifact has an embedding, THEN THE TUI SHALL display a similarity search option
11. WHEN artifacts are created or updated, THE TUI SHALL update the list in real-time

### Requirement 6: Note Library View

**User Story:** As a developer, I want to manage cross-trajectory notes, so that I can build and maintain long-term knowledge.

#### Acceptance Criteria

1. THE TUI SHALL display notes grouped by type (Convention, Strategy, Gotcha, Fact, Preference, Relationship, Procedure, Meta)
2. THE TUI SHALL support filtering notes by type, source trajectory, or date range
3. THE TUI SHALL support full-text search across note titles and content
4. WHEN a note is selected, THE TUI SHALL display its full content and source references
5. THE TUI SHALL display note access statistics (access count, last accessed)
6. WHEN a user presses 'n', THE TUI SHALL open a form to create a new note
7. WHEN a user presses 'e', THE TUI SHALL open a form to edit the selected note
8. WHEN a user presses 'd', THE TUI SHALL prompt for confirmation then delete the note
9. IF a note has been superseded, THEN THE TUI SHALL display a superseded indicator with link to replacement
10. WHEN notes are created or updated, THE TUI SHALL update the library in real-time

### Requirement 7: Turn History View

**User Story:** As a developer, I want to replay conversation history, so that I can debug agent behavior and understand context flow.

#### Acceptance Criteria

1. THE TUI SHALL display turns in chronological order within a selected scope
2. THE TUI SHALL color-code turns by role (User=cyan, Assistant=magenta, System=yellow, Tool=green)
3. WHEN a turn is selected, THE TUI SHALL display its full content with proper formatting
4. THE TUI SHALL display token count for each turn
5. IF a turn has tool calls, THEN THE TUI SHALL display them in a collapsible section
6. IF a turn has tool results, THEN THE TUI SHALL display them in a collapsible section
7. THE TUI SHALL support filtering turns by role
8. THE TUI SHALL support searching turn content
9. WHEN new turns are added, THE TUI SHALL update the history in real-time with auto-scroll option

### Requirement 8: Agent Dashboard View

**User Story:** As an operator, I want to monitor agent status and activity, so that I can ensure the multi-agent system is functioning correctly.

#### Acceptance Criteria

1. THE TUI SHALL display all registered agents with their current status (Idle, Active, Blocked, Failed)
2. THE TUI SHALL color-code agent status indicators (Idle=dim, Active=cyan, Blocked=yellow, Failed=red)
3. WHEN an agent is selected, THE TUI SHALL display its details (type, capabilities, memory access, delegation targets)
4. THE TUI SHALL display agent heartbeat timestamps with staleness warnings
5. THE TUI SHALL display current trajectory and scope for active agents
6. WHEN a user presses 'n', THE TUI SHALL open a form to register a new agent
7. WHEN a user presses 'e', THE TUI SHALL open a form to edit agent configuration
8. WHEN a user presses 'd', THE TUI SHALL prompt for confirmation then unregister the agent
9. THE TUI SHALL display delegation relationships as a visual graph or list
10. WHEN agent status changes, THE TUI SHALL update the dashboard in real-time

### Requirement 9: Lock Monitor View

**User Story:** As an operator, I want to monitor distributed locks, so that I can identify contention and debug coordination issues.

#### Acceptance Criteria

1. THE TUI SHALL display all active locks with holder agent, resource type, and resource ID
2. THE TUI SHALL display lock mode (Exclusive, Shared) with appropriate icons
3. THE TUI SHALL display lock acquisition time and expiration countdown
4. THE TUI SHALL highlight locks that are near expiration (< 10% time remaining)
5. THE TUI SHALL highlight locks held by failed or stale agents
6. WHEN a user presses 'r' on a lock, THE TUI SHALL force-release the lock via API (with confirmation)
7. THE TUI SHALL support filtering locks by resource type or holder agent
8. WHEN locks are acquired or released, THE TUI SHALL update the monitor in real-time

### Requirement 10: Message Queue View

**User Story:** As an operator, I want to monitor inter-agent messages, so that I can debug communication and coordination.

#### Acceptance Criteria

1. THE TUI SHALL display messages in a queue view with sender, recipient, type, and priority
2. THE TUI SHALL color-code messages by priority (Low=dim, Normal=white, High=yellow, Critical=red)
3. THE TUI SHALL display message delivery and acknowledgment status
4. WHEN a message is selected, THE TUI SHALL display its full payload
5. THE TUI SHALL support filtering messages by type (TaskDelegation, TaskResult, ContextRequest, ContextShare, CoordinationSignal, Handoff, Interrupt, Heartbeat)
6. THE TUI SHALL support filtering messages by sender, recipient, or trajectory
7. THE TUI SHALL highlight expired but undelivered messages
8. WHEN new messages arrive, THE TUI SHALL update the queue in real-time

### Requirement 11: DSL Editor View

**User Story:** As a developer, I want to edit CALIBER DSL configurations with syntax highlighting, so that I can configure memory types, policies, and injection rules.

#### Acceptance Criteria

1. THE TUI SHALL provide a text editor with CALIBER DSL syntax highlighting
2. THE TUI SHALL highlight keywords (caliber, memory, policy, adapter, inject) in cyan
3. THE TUI SHALL highlight memory types (ephemeral, working, episodic, semantic, procedural, meta) in magenta
4. THE TUI SHALL highlight field types (uuid, text, int, float, bool, timestamp, json, embedding) in yellow
5. THE TUI SHALL highlight strings in green and numbers in orange
6. WHEN the user saves, THE TUI SHALL validate the DSL via API and display parse errors inline
7. IF validation fails, THEN THE TUI SHALL highlight the error location and display the error message
8. THE TUI SHALL support loading DSL files from the filesystem
9. THE TUI SHALL support saving DSL files to the filesystem
10. THE TUI SHALL display a live AST preview panel (optional toggle)

### Requirement 12: Config Viewer View

**User Story:** As a developer, I want to inspect the current CaliberConfig, so that I can verify system configuration.

#### Acceptance Criteria

1. THE TUI SHALL display the current CaliberConfig as formatted, syntax-highlighted JSON/TOML
2. THE TUI SHALL group config sections (Context Assembly, PCP Settings, Storage, LLM, Multi-Agent)
3. THE TUI SHALL display validation status for each config section
4. IF a config value is invalid, THEN THE TUI SHALL highlight it in red with the validation error
5. THE TUI SHALL support editing config values inline
6. WHEN config is edited, THE TUI SHALL validate before applying
7. THE TUI SHALL display provider configurations (embedding, summarization) with connection status

### Requirement 13: Visual Design - SynthBrute Aesthetic

**User Story:** As a user, I want the TUI to have a distinctive, visually striking design, so that it feels premium and matches the CALIBER brand.

#### Acceptance Criteria

1. THE TUI SHALL use a dark background color (#0a0a0a or terminal default black)
2. THE TUI SHALL use cyan (#00ffff) as the primary accent color for active elements
3. THE TUI SHALL use magenta (#ff00ff) as the secondary accent color for highlights
4. THE TUI SHALL use yellow (#ffff00) for warnings and tertiary accents
5. THE TUI SHALL use thick box-drawing characters for panel borders (brutalist style)
6. THE TUI SHALL use monospace fonts throughout
7. THE TUI SHALL display the CALIBER logo/wordmark in the header
8. THE TUI SHALL use consistent spacing and alignment across all views
9. WHEN elements are focused, THE TUI SHALL display a visible focus indicator (bright border or inverse colors)
10. THE TUI SHALL support 256-color and truecolor terminals with graceful fallback to 16 colors

### Requirement 14: Navigation and Keybindings

**User Story:** As a power user, I want consistent keyboard navigation, so that I can work efficiently without reaching for the mouse.

#### Acceptance Criteria

1. THE TUI SHALL use vim-style navigation (h/j/k/l or arrow keys) for movement
2. THE TUI SHALL use Tab/Shift+Tab to cycle between panels
3. THE TUI SHALL use number keys (1-0) to switch between views
4. THE TUI SHALL display a persistent hotkey legend in the footer
5. THE TUI SHALL support '?' to open a full keybinding reference modal
6. THE TUI SHALL support '/' to open global search
7. THE TUI SHALL support ':' to open a command palette
8. THE TUI SHALL support 'q' to quit (with confirmation if unsaved changes)
9. THE TUI SHALL support Ctrl+C to cancel current operation
10. THE TUI SHALL support Ctrl+R to refresh current view

### Requirement 15: Real-Time Updates

**User Story:** As an operator, I want live updates without manual refresh, so that I can monitor the system in real-time.

#### Acceptance Criteria

1. THE TUI SHALL maintain a persistent WebSocket connection to the API_Layer
2. WHEN the WebSocket disconnects, THE TUI SHALL display a connection status indicator and attempt reconnection
3. THE TUI SHALL process incoming events and update relevant views without full refresh
4. THE TUI SHALL display a subtle animation or indicator when new data arrives
5. THE TUI SHALL support pausing real-time updates (toggle with 'p')
6. WHEN updates are paused, THE TUI SHALL display a "PAUSED" indicator and queue incoming events
7. WHEN updates are resumed, THE TUI SHALL apply queued events

### Requirement 16: Error Handling

**User Story:** As a user, I want clear error feedback, so that I can understand and resolve issues.

#### Acceptance Criteria

1. WHEN an API request fails, THE TUI SHALL display the error message in a notification area
2. THE TUI SHALL color-code errors in red with appropriate icons
3. THE TUI SHALL display actionable error messages (not raw stack traces)
4. IF a network error occurs, THEN THE TUI SHALL offer retry option
5. IF authentication fails, THEN THE TUI SHALL prompt for re-authentication
6. THE TUI SHALL log errors to a local file for debugging
7. THE TUI SHALL support viewing recent errors via a dedicated error log panel

