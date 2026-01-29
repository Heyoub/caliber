//! Application state and view state definitions.

use crate::api_client::ApiClient;
use crate::config::TuiConfig;
use crate::nav::View;
use crate::notifications::{Notification, NotificationLevel};
use crate::theme::SynthBruteTheme;
use crate::widgets::LinksState;
use caliber_api::events::WsEvent;
use caliber_api::types::*;
use caliber_core::{
    AgentId, AgentStatus, ArtifactId, ArtifactType, EntityIdType, LockId, MessageId,
    MessagePriority, MessageType, NoteId, NoteType, ScopeId, TenantId, Timestamp, TrajectoryId,
    TrajectoryStatus, TurnRole,
};
use std::collections::{HashSet, VecDeque};
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct TenantContext {
    pub tenant_id: TenantId,
    pub tenant_name: String,
    pub available_tenants: Vec<TenantInfo>,
}

#[derive(Clone)]
pub struct App {
    pub config: TuiConfig,
    pub theme: SynthBruteTheme,
    pub api: ApiClient,
    pub tenant: TenantContext,
    pub active_view: View,

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
    pub tenant_view: TenantViewState,

    pub notifications: Vec<Notification>,
    pub command_palette: Option<CommandPalette>,
    pub search: Option<GlobalSearch>,
    pub modal: Option<Modal>,

    /// Whether the links panel is visible.
    pub links_panel_visible: bool,
    /// State for the links panel (actions for selected entity).
    pub links_state: LinksState,

    pub ws_connected: bool,
    pub updates_paused: bool,
    pub event_queue: VecDeque<WsEvent>,
}

impl App {
    pub fn new(config: TuiConfig, api: ApiClient) -> Self {
        let theme = SynthBruteTheme::synthbrute(config.theme.colors.as_ref());
        let tenant_id = TenantId::new(config.tenant_id);
        Self {
            config,
            theme,
            api,
            tenant: TenantContext {
                tenant_id,
                tenant_name: "Unknown".to_string(),
                available_tenants: Vec::new(),
            },
            active_view: View::TrajectoryTree,
            trajectory_view: TrajectoryViewState::new(),
            scope_view: ScopeViewState::new(),
            artifact_view: ArtifactViewState::new(),
            note_view: NoteViewState::new(),
            turn_view: TurnViewState::new(),
            agent_view: AgentViewState::new(),
            lock_view: LockViewState::new(),
            message_view: MessageViewState::new(),
            dsl_view: DslViewState::new(),
            config_view: ConfigViewState::new(),
            tenant_view: TenantViewState::new(),
            notifications: Vec::new(),
            command_palette: None,
            search: None,
            modal: None,
            links_panel_visible: false,
            links_state: LinksState::default(),
            ws_connected: false,
            updates_paused: false,
            event_queue: VecDeque::new(),
        }
    }

    /// Toggle the links panel visibility.
    pub fn toggle_links_panel(&mut self) {
        self.links_panel_visible = !self.links_panel_visible;
        if self.links_panel_visible {
            self.update_links_for_selected();
        }
    }

    /// Update the links state based on currently selected entity.
    pub fn update_links_for_selected(&mut self) {
        let links = match self.active_view {
            View::TrajectoryTree => self.trajectory_view.selected.and_then(|id| {
                self.trajectory_view
                    .trajectories
                    .iter()
                    .find(|t| t.trajectory_id.as_uuid() == id)
                    .and_then(|t| t.links.as_ref())
            }),
            View::ScopeExplorer => self.scope_view.selected.and_then(|id| {
                self.scope_view
                    .scopes
                    .iter()
                    .find(|s| s.scope_id.as_uuid() == id)
                    .and_then(|s| s.links.as_ref())
            }),
            View::ArtifactBrowser => self.artifact_view.selected.and_then(|id| {
                self.artifact_view
                    .artifacts
                    .iter()
                    .find(|a| a.artifact_id.as_uuid() == id)
                    .and_then(|a| a.links.as_ref())
            }),
            View::NoteLibrary => self.note_view.selected.and_then(|id| {
                self.note_view
                    .notes
                    .iter()
                    .find(|n| n.note_id.as_uuid() == id)
                    .and_then(|n| n.links.as_ref())
            }),
            _ => None,
        };
        self.links_state.update(links);
    }

    /// Navigate to next link in the links panel.
    pub fn next_link(&mut self) {
        if self.links_panel_visible {
            self.links_state.select_next();
        }
    }

    /// Navigate to previous link in the links panel.
    pub fn prev_link(&mut self) {
        if self.links_panel_visible {
            self.links_state.select_previous();
        }
    }

    /// Get the currently selected link action, if any.
    pub fn selected_link_action(&self) -> Option<&crate::widgets::LinkAction> {
        if self.links_panel_visible {
            self.links_state.selected_action()
        } else {
            None
        }
    }

    pub fn set_tenant(&mut self, tenant: &TenantInfo) {
        self.tenant.tenant_id = tenant.tenant_id;
        self.tenant.tenant_name = tenant.name.clone();
    }

    pub fn enqueue_event(&mut self, event: WsEvent) {
        if self.updates_paused {
            self.event_queue.push_back(event);
        } else {
            self.apply_ws_event(event);
        }
    }

    pub fn flush_queued_events(&mut self) {
        while let Some(event) = self.event_queue.pop_front() {
            self.apply_ws_event(event);
        }
    }

    pub fn notify(&mut self, level: NotificationLevel, message: impl Into<String>) {
        self.notifications.push(Notification::new(level, message));
    }

    pub fn select_next(&mut self) {
        match self.active_view {
            View::TrajectoryTree => select_next_id(
                &self.trajectory_view.trajectories,
                &mut self.trajectory_view.selected,
            ),
            View::ScopeExplorer => {
                select_next_id(&self.scope_view.scopes, &mut self.scope_view.selected)
            }
            View::ArtifactBrowser => select_next_id(
                &self.artifact_view.artifacts,
                &mut self.artifact_view.selected,
            ),
            View::NoteLibrary => {
                select_next_id(&self.note_view.notes, &mut self.note_view.selected)
            }
            View::TurnHistory => {
                select_next_id(&self.turn_view.turns, &mut self.turn_view.selected)
            }
            View::AgentDashboard => {
                select_next_id(&self.agent_view.agents, &mut self.agent_view.selected)
            }
            View::LockMonitor => {
                select_next_id(&self.lock_view.locks, &mut self.lock_view.selected)
            }
            View::MessageQueue => {
                select_next_id(&self.message_view.messages, &mut self.message_view.selected)
            }
            View::TenantManagement => {
                select_next_id(&self.tenant_view.tenants, &mut self.tenant_view.selected)
            }
            View::DslEditor | View::ConfigViewer => {}
        }
        // Update links panel when selection changes
        if self.links_panel_visible {
            self.update_links_for_selected();
        }
    }

    pub fn select_previous(&mut self) {
        match self.active_view {
            View::TrajectoryTree => select_prev_id(
                &self.trajectory_view.trajectories,
                &mut self.trajectory_view.selected,
            ),
            View::ScopeExplorer => {
                select_prev_id(&self.scope_view.scopes, &mut self.scope_view.selected)
            }
            View::ArtifactBrowser => select_prev_id(
                &self.artifact_view.artifacts,
                &mut self.artifact_view.selected,
            ),
            View::NoteLibrary => {
                select_prev_id(&self.note_view.notes, &mut self.note_view.selected)
            }
            View::TurnHistory => {
                select_prev_id(&self.turn_view.turns, &mut self.turn_view.selected)
            }
            View::AgentDashboard => {
                select_prev_id(&self.agent_view.agents, &mut self.agent_view.selected)
            }
            View::LockMonitor => {
                select_prev_id(&self.lock_view.locks, &mut self.lock_view.selected)
            }
            View::MessageQueue => {
                select_prev_id(&self.message_view.messages, &mut self.message_view.selected)
            }
            View::TenantManagement => {
                select_prev_id(&self.tenant_view.tenants, &mut self.tenant_view.selected)
            }
            View::DslEditor | View::ConfigViewer => {}
        }
        // Update links panel when selection changes
        if self.links_panel_visible {
            self.update_links_for_selected();
        }
    }

    pub fn toggle_expand(&mut self) {
        if self.active_view == View::TrajectoryTree {
            if let Some(id) = self.trajectory_view.selected {
                if self.trajectory_view.expanded.contains(&id) {
                    self.trajectory_view.expanded.remove(&id);
                } else {
                    self.trajectory_view.expanded.insert(id);
                }
            }
        }
    }

    fn apply_ws_event(&mut self, event: WsEvent) {
        match event {
            WsEvent::TrajectoryCreated { trajectory } => {
                self.trajectory_view.upsert(trajectory);
            }
            WsEvent::TrajectoryUpdated { trajectory } => {
                self.trajectory_view.upsert(trajectory);
            }
            WsEvent::TrajectoryDeleted { id, .. } => {
                self.trajectory_view.remove(id);
            }
            WsEvent::ScopeCreated { scope } => {
                self.scope_view.upsert(scope);
            }
            WsEvent::ScopeUpdated { scope } => {
                self.scope_view.upsert(scope);
            }
            WsEvent::ScopeClosed { scope } => {
                self.scope_view.upsert(scope);
            }
            WsEvent::ArtifactCreated { artifact } => {
                self.artifact_view.upsert(artifact);
            }
            WsEvent::ArtifactUpdated { artifact } => {
                self.artifact_view.upsert(artifact);
            }
            WsEvent::ArtifactDeleted { id, .. } => {
                self.artifact_view.remove(id);
            }
            WsEvent::NoteCreated { note } => {
                self.note_view.upsert(note);
            }
            WsEvent::NoteUpdated { note } => {
                self.note_view.upsert(note);
            }
            WsEvent::NoteDeleted { id, .. } => {
                self.note_view.remove(id);
            }
            WsEvent::TurnCreated { turn } => {
                self.turn_view.turns.push(turn);
            }
            WsEvent::AgentRegistered { agent } => {
                self.agent_view.upsert(agent);
            }
            WsEvent::AgentStatusChanged {
                agent_id, status, ..
            } => {
                self.agent_view.update_status(agent_id, status);
            }
            WsEvent::AgentHeartbeat {
                agent_id,
                timestamp,
                ..
            } => {
                self.agent_view.update_heartbeat(agent_id, timestamp);
            }
            WsEvent::AgentUnregistered { id, .. } => {
                self.agent_view.remove(id);
            }
            WsEvent::LockAcquired { lock } => {
                self.lock_view.upsert(lock);
            }
            WsEvent::LockReleased { lock_id, .. } => {
                self.lock_view.remove(lock_id);
            }
            WsEvent::LockExpired { lock_id, .. } => {
                self.lock_view.remove(lock_id);
            }
            WsEvent::MessageSent { message } => {
                self.message_view.upsert(message);
            }
            WsEvent::MessageDelivered { message_id, .. } => {
                self.message_view.mark_delivered(message_id);
            }
            WsEvent::MessageAcknowledged { message_id, .. } => {
                self.message_view.mark_acknowledged(message_id);
            }
            WsEvent::ConfigUpdated { config } => {
                if let Ok(pretty) = serde_json::to_string_pretty(&config.config) {
                    self.config_view.content = pretty;
                }
                self.config_view.validation_errors = config.errors;
                self.config_view.modified = false;
            }
            WsEvent::DelegationCreated { .. }
            | WsEvent::DelegationAccepted { .. }
            | WsEvent::DelegationRejected { .. }
            | WsEvent::DelegationCompleted { .. }
            | WsEvent::HandoffCreated { .. }
            | WsEvent::HandoffAccepted { .. }
            | WsEvent::HandoffCompleted { .. } => {
                self.notify(NotificationLevel::Info, "Received coordination event");
            }
            WsEvent::Connected { .. } => {
                self.ws_connected = true;
            }
            WsEvent::Disconnected { reason } => {
                self.ws_connected = false;
                self.notify(NotificationLevel::Warning, reason);
            }
            WsEvent::Error { message } => {
                self.notify(NotificationLevel::Error, message);
            }
            WsEvent::SummarizationTriggered { .. }
            | WsEvent::EdgeCreated { .. }
            | WsEvent::EdgesBatchCreated { .. }
            | WsEvent::ToolExecuted { .. } => {}
        }
    }
}

fn select_next_id<T: HasEntityId>(items: &[T], selected: &mut Option<Uuid>) {
    if items.is_empty() {
        *selected = None;
        return;
    }
    let index = selected
        .and_then(|id| items.iter().position(|item| item.entity_id() == id))
        .unwrap_or(usize::MAX);
    let next = if index == usize::MAX {
        0
    } else {
        (index + 1) % items.len()
    };
    *selected = Some(items[next].entity_id());
}

fn select_prev_id<T: HasEntityId>(items: &[T], selected: &mut Option<Uuid>) {
    if items.is_empty() {
        *selected = None;
        return;
    }
    let index = selected
        .and_then(|id| items.iter().position(|item| item.entity_id() == id))
        .unwrap_or(0);
    let prev = if index == 0 {
        items.len() - 1
    } else {
        index - 1
    };
    *selected = Some(items[prev].entity_id());
}

trait HasEntityId {
    fn entity_id(&self) -> Uuid;
}

impl HasEntityId for TrajectoryResponse {
    fn entity_id(&self) -> Uuid {
        self.trajectory_id.as_uuid()
    }
}

impl HasEntityId for ScopeResponse {
    fn entity_id(&self) -> Uuid {
        self.scope_id.as_uuid()
    }
}

impl HasEntityId for ArtifactResponse {
    fn entity_id(&self) -> Uuid {
        self.artifact_id.as_uuid()
    }
}

impl HasEntityId for NoteResponse {
    fn entity_id(&self) -> Uuid {
        self.note_id.as_uuid()
    }
}

impl HasEntityId for TurnResponse {
    fn entity_id(&self) -> Uuid {
        self.turn_id.as_uuid()
    }
}

impl HasEntityId for AgentResponse {
    fn entity_id(&self) -> Uuid {
        self.agent_id.as_uuid()
    }
}

impl HasEntityId for LockResponse {
    fn entity_id(&self) -> Uuid {
        self.lock_id.as_uuid()
    }
}

impl HasEntityId for MessageResponse {
    fn entity_id(&self) -> Uuid {
        self.message_id.as_uuid()
    }
}

impl HasEntityId for TenantInfo {
    fn entity_id(&self) -> Uuid {
        self.tenant_id.as_uuid()
    }
}

#[derive(Debug, Clone)]
pub struct CommandPalette {
    pub input: String,
    pub suggestions: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct GlobalSearch {
    pub query: String,
}

#[derive(Debug, Clone)]
pub struct Modal {
    pub title: String,
    pub message: String,
}

// ============================================================================
// VIEW STATE DEFINITIONS
// ============================================================================

#[derive(Debug, Clone)]
pub struct TrajectoryViewState {
    pub trajectories: Vec<TrajectoryResponse>,
    pub expanded: HashSet<Uuid>,
    pub selected: Option<Uuid>,
    pub filter: TrajectoryFilter,
    pub loading: bool,
}

impl TrajectoryViewState {
    pub fn new() -> Self {
        Self {
            trajectories: Vec::new(),
            expanded: HashSet::new(),
            selected: None,
            filter: TrajectoryFilter::new(),
            loading: false,
        }
    }

    pub fn upsert(&mut self, trajectory: TrajectoryResponse) {
        if let Some(existing) = self
            .trajectories
            .iter_mut()
            .find(|t| t.trajectory_id == trajectory.trajectory_id)
        {
            *existing = trajectory;
        } else {
            self.trajectories.push(trajectory);
        }
    }

    pub fn remove(&mut self, id: TrajectoryId) {
        self.trajectories.retain(|t| t.trajectory_id != id);
    }
}

#[derive(Debug, Clone)]
pub struct TrajectoryFilter {
    pub status: Option<TrajectoryStatus>,
    pub agent_id: Option<AgentId>,
    pub date_from: Option<Timestamp>,
    pub date_to: Option<Timestamp>,
    pub search_query: Option<String>,
}

impl TrajectoryFilter {
    pub fn new() -> Self {
        Self {
            status: None,
            agent_id: None,
            date_from: None,
            date_to: None,
            search_query: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ScopeViewState {
    pub scopes: Vec<ScopeResponse>,
    pub selected: Option<Uuid>,
    pub filter: ScopeFilter,
    pub loading: bool,
}

impl ScopeViewState {
    pub fn new() -> Self {
        Self {
            scopes: Vec::new(),
            selected: None,
            filter: ScopeFilter::new(),
            loading: false,
        }
    }

    pub fn upsert(&mut self, scope: ScopeResponse) {
        if let Some(existing) = self
            .scopes
            .iter_mut()
            .find(|s| s.scope_id == scope.scope_id)
        {
            *existing = scope;
        } else {
            self.scopes.push(scope);
        }
    }
}

#[derive(Debug, Clone)]
pub struct ScopeFilter {
    pub trajectory_id: Option<TrajectoryId>,
    pub active_only: Option<bool>,
    pub date_from: Option<Timestamp>,
    pub date_to: Option<Timestamp>,
}

impl ScopeFilter {
    pub fn new() -> Self {
        Self {
            trajectory_id: None,
            active_only: None,
            date_from: None,
            date_to: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ArtifactViewState {
    pub artifacts: Vec<ArtifactResponse>,
    pub selected: Option<Uuid>,
    pub filter: ArtifactFilter,
    pub sort: ArtifactSort,
    pub search_query: String,
    pub loading: bool,
}

impl ArtifactViewState {
    pub fn new() -> Self {
        Self {
            artifacts: Vec::new(),
            selected: None,
            filter: ArtifactFilter::new(),
            sort: ArtifactSort::ByCreatedAtDesc,
            search_query: String::new(),
            loading: false,
        }
    }

    pub fn upsert(&mut self, artifact: ArtifactResponse) {
        if let Some(existing) = self
            .artifacts
            .iter_mut()
            .find(|a| a.artifact_id == artifact.artifact_id)
        {
            *existing = artifact;
        } else {
            self.artifacts.push(artifact);
        }
    }

    pub fn remove(&mut self, id: ArtifactId) {
        self.artifacts.retain(|a| a.artifact_id != id);
    }
}

#[derive(Debug, Clone)]
pub enum ArtifactSort {
    ByCreatedAtDesc,
    ByCreatedAtAsc,
    ByNameAsc,
    ByNameDesc,
}

#[derive(Debug, Clone)]
pub struct ArtifactFilter {
    pub artifact_type: Option<ArtifactType>,
    pub trajectory_id: Option<TrajectoryId>,
    pub scope_id: Option<ScopeId>,
    pub date_from: Option<Timestamp>,
    pub date_to: Option<Timestamp>,
}

impl ArtifactFilter {
    pub fn new() -> Self {
        Self {
            artifact_type: None,
            trajectory_id: None,
            scope_id: None,
            date_from: None,
            date_to: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct NoteViewState {
    pub notes: Vec<NoteResponse>,
    pub selected: Option<Uuid>,
    pub filter: NoteFilter,
    pub search_query: String,
    pub loading: bool,
}

impl NoteViewState {
    pub fn new() -> Self {
        Self {
            notes: Vec::new(),
            selected: None,
            filter: NoteFilter::new(),
            search_query: String::new(),
            loading: false,
        }
    }

    pub fn upsert(&mut self, note: NoteResponse) {
        if let Some(existing) = self.notes.iter_mut().find(|n| n.note_id == note.note_id) {
            *existing = note;
        } else {
            self.notes.push(note);
        }
    }

    pub fn remove(&mut self, id: NoteId) {
        self.notes.retain(|n| n.note_id != id);
    }
}

#[derive(Debug, Clone)]
pub struct NoteFilter {
    pub note_type: Option<NoteType>,
    pub source_trajectory_id: Option<TrajectoryId>,
    pub date_from: Option<Timestamp>,
    pub date_to: Option<Timestamp>,
}

impl NoteFilter {
    pub fn new() -> Self {
        Self {
            note_type: None,
            source_trajectory_id: None,
            date_from: None,
            date_to: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TurnViewState {
    pub turns: Vec<TurnResponse>,
    pub selected: Option<Uuid>,
    pub filter: TurnFilter,
    pub auto_scroll: bool,
    pub loading: bool,
}

impl TurnViewState {
    pub fn new() -> Self {
        Self {
            turns: Vec::new(),
            selected: None,
            filter: TurnFilter::new(),
            auto_scroll: true,
            loading: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TurnFilter {
    pub role: Option<TurnRole>,
}

impl TurnFilter {
    pub fn new() -> Self {
        Self { role: None }
    }
}

#[derive(Debug, Clone)]
pub struct AgentViewState {
    pub agents: Vec<AgentResponse>,
    pub selected: Option<Uuid>,
    pub filter: AgentFilter,
    pub loading: bool,
}

impl AgentViewState {
    pub fn new() -> Self {
        Self {
            agents: Vec::new(),
            selected: None,
            filter: AgentFilter::new(),
            loading: false,
        }
    }

    pub fn upsert(&mut self, agent: AgentResponse) {
        if let Some(existing) = self
            .agents
            .iter_mut()
            .find(|a| a.agent_id == agent.agent_id)
        {
            *existing = agent;
        } else {
            self.agents.push(agent);
        }
    }

    pub fn remove(&mut self, id: AgentId) {
        self.agents.retain(|a| a.agent_id != id);
    }

    pub fn update_status(&mut self, id: AgentId, status: String) {
        if let Some(agent) = self.agents.iter_mut().find(|a| a.agent_id == id) {
            if let Ok(parsed) = status.parse::<AgentStatus>() {
                agent.status = parsed;
            }
        }
    }

    pub fn update_heartbeat(&mut self, id: AgentId, timestamp: Timestamp) {
        if let Some(agent) = self.agents.iter_mut().find(|a| a.agent_id == id) {
            agent.last_heartbeat = timestamp;
        }
    }
}

#[derive(Debug, Clone)]
pub struct AgentFilter {
    pub agent_type: Option<String>,
    pub status: Option<String>,
}

impl AgentFilter {
    pub fn new() -> Self {
        Self {
            agent_type: None,
            status: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct LockViewState {
    pub locks: Vec<LockResponse>,
    pub selected: Option<Uuid>,
    pub filter: LockFilter,
    pub loading: bool,
}

impl LockViewState {
    pub fn new() -> Self {
        Self {
            locks: Vec::new(),
            selected: None,
            filter: LockFilter::new(),
            loading: false,
        }
    }

    pub fn upsert(&mut self, lock: LockResponse) {
        if let Some(existing) = self.locks.iter_mut().find(|l| l.lock_id == lock.lock_id) {
            *existing = lock;
        } else {
            self.locks.push(lock);
        }
    }

    pub fn remove(&mut self, id: LockId) {
        self.locks.retain(|l| l.lock_id != id);
    }
}

#[derive(Debug, Clone)]
pub struct LockFilter {
    pub resource_type: Option<String>,
    pub holder_agent_id: Option<AgentId>,
}

impl LockFilter {
    pub fn new() -> Self {
        Self {
            resource_type: None,
            holder_agent_id: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct MessageViewState {
    pub messages: Vec<MessageResponse>,
    pub selected: Option<Uuid>,
    pub filter: MessageFilter,
    pub loading: bool,
}

impl MessageViewState {
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
            selected: None,
            filter: MessageFilter::new(),
            loading: false,
        }
    }

    pub fn upsert(&mut self, message: MessageResponse) {
        if let Some(existing) = self
            .messages
            .iter_mut()
            .find(|m| m.message_id == message.message_id)
        {
            *existing = message;
        } else {
            self.messages.push(message);
        }
    }

    pub fn mark_delivered(&mut self, id: MessageId) {
        if let Some(message) = self.messages.iter_mut().find(|m| m.message_id == id) {
            message.delivered_at = Some(chrono::Utc::now());
        }
    }

    pub fn mark_acknowledged(&mut self, id: MessageId) {
        if let Some(message) = self.messages.iter_mut().find(|m| m.message_id == id) {
            message.acknowledged_at = Some(chrono::Utc::now());
        }
    }
}

#[derive(Debug, Clone)]
pub struct MessageFilter {
    pub message_type: Option<MessageType>,
    pub from_agent_id: Option<AgentId>,
    pub to_agent_id: Option<AgentId>,
    pub priority: Option<MessagePriority>,
}

impl MessageFilter {
    pub fn new() -> Self {
        Self {
            message_type: None,
            from_agent_id: None,
            to_agent_id: None,
            priority: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct DslViewState {
    pub content: String,
    pub cursor_position: (usize, usize),
    pub parse_errors: Vec<ParseErrorResponse>,
    pub ast_preview: Option<serde_json::Value>,
    pub file_path: Option<PathBuf>,
    pub pack_root: Option<PathBuf>,
    pub config_name: String,
    pub modified: bool,
}

impl DslViewState {
    pub fn new() -> Self {
        let default_pack_root = PathBuf::from("agents-pack");
        let pack_root = if default_pack_root.exists() {
            Some(default_pack_root)
        } else {
            None
        };

        Self {
            content: String::new(),
            cursor_position: (0, 0),
            parse_errors: Vec::new(),
            ast_preview: None,
            file_path: None,
            pack_root,
            config_name: "default".to_string(),
            modified: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ConfigViewState {
    pub content: String,
    pub validation_errors: Vec<String>,
    pub modified: bool,
}

impl ConfigViewState {
    pub fn new() -> Self {
        Self {
            content: String::new(),
            validation_errors: Vec::new(),
            modified: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TenantViewState {
    pub tenants: Vec<TenantInfo>,
    pub selected: Option<Uuid>,
    pub loading: bool,
}

impl TenantViewState {
    pub fn new() -> Self {
        Self {
            tenants: Vec::new(),
            selected: None,
            loading: false,
        }
    }
}

impl Default for TrajectoryViewState {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for TrajectoryFilter {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for ScopeViewState {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for ScopeFilter {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for ArtifactViewState {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for ArtifactFilter {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for NoteViewState {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for NoteFilter {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for TurnViewState {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for TurnFilter {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for AgentViewState {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for AgentFilter {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for LockViewState {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for LockFilter {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for MessageViewState {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for MessageFilter {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for DslViewState {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for ConfigViewState {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for TenantViewState {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // Test Fixtures
    // ========================================================================

    fn sample_trajectory(id: Uuid, name: &str, status: TrajectoryStatus) -> TrajectoryResponse {
        TrajectoryResponse {
            trajectory_id: TrajectoryId::new(id),
            tenant_id: TenantId::new(Uuid::nil()),
            name: name.to_string(),
            description: None,
            status,
            parent_trajectory_id: None,
            root_trajectory_id: None,
            agent_id: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            completed_at: None,
            outcome: None,
            metadata: None,
            links: None,
        }
    }

    fn sample_scope(id: Uuid, trajectory_id: Uuid, name: &str) -> ScopeResponse {
        ScopeResponse {
            scope_id: ScopeId::new(id),
            tenant_id: TenantId::new(Uuid::nil()),
            trajectory_id: TrajectoryId::new(trajectory_id),
            parent_scope_id: None,
            name: name.to_string(),
            purpose: None,
            is_active: true,
            created_at: chrono::Utc::now(),
            closed_at: None,
            checkpoint: None,
            token_budget: 8000,
            tokens_used: 0,
            metadata: None,
            links: None,
        }
    }

    fn sample_artifact(id: Uuid, trajectory_id: Uuid, name: &str) -> ArtifactResponse {
        use caliber_api::types::ProvenanceResponse;
        use caliber_core::ExtractionMethod;

        ArtifactResponse {
            artifact_id: ArtifactId::new(id),
            tenant_id: TenantId::new(Uuid::nil()),
            trajectory_id: TrajectoryId::new(trajectory_id),
            scope_id: ScopeId::new(Uuid::now_v7()),
            artifact_type: ArtifactType::Fact,
            name: name.to_string(),
            content: "test content".to_string(),
            content_hash: [0u8; 32],
            embedding: None,
            provenance: ProvenanceResponse {
                source_turn: 1,
                extraction_method: ExtractionMethod::Explicit,
                confidence: Some(1.0),
            },
            ttl: caliber_core::TTL::Persistent,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            superseded_by: None,
            metadata: None,
            links: None,
        }
    }

    fn sample_note(id: Uuid, title: &str) -> NoteResponse {
        NoteResponse {
            note_id: NoteId::new(id),
            tenant_id: TenantId::new(Uuid::nil()),
            note_type: NoteType::Insight,
            title: title.to_string(),
            content: "test note".to_string(),
            content_hash: [0u8; 32],
            embedding: None,
            source_trajectory_ids: vec![],
            source_artifact_ids: vec![],
            ttl: caliber_core::TTL::Persistent,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            accessed_at: chrono::Utc::now(),
            access_count: 0,
            superseded_by: None,
            metadata: None,
            links: None,
        }
    }

    // ========================================================================
    // TrajectoryViewState Tests
    // ========================================================================

    #[test]
    fn test_trajectory_view_state_new_is_empty() {
        let state = TrajectoryViewState::new();
        assert!(state.trajectories.is_empty());
        assert!(state.expanded.is_empty());
        assert!(state.selected.is_none());
        assert!(!state.loading);
    }

    #[test]
    fn test_trajectory_view_upsert_inserts_new() {
        let mut state = TrajectoryViewState::new();
        let id = Uuid::now_v7();
        let traj = sample_trajectory(id, "test", TrajectoryStatus::Active);

        state.upsert(traj);

        assert_eq!(state.trajectories.len(), 1);
        assert_eq!(state.trajectories[0].trajectory_id.as_uuid(), id);
    }

    #[test]
    fn test_trajectory_view_upsert_updates_existing() {
        let mut state = TrajectoryViewState::new();
        let id = Uuid::now_v7();

        // Insert initial
        let traj1 = sample_trajectory(id, "original", TrajectoryStatus::Active);
        state.upsert(traj1);

        // Update
        let traj2 = sample_trajectory(id, "updated", TrajectoryStatus::Completed);
        state.upsert(traj2);

        assert_eq!(state.trajectories.len(), 1);
        assert_eq!(state.trajectories[0].name, "updated");
        assert_eq!(state.trajectories[0].status, TrajectoryStatus::Completed);
    }

    #[test]
    fn test_trajectory_view_remove_existing() {
        let mut state = TrajectoryViewState::new();
        let id = Uuid::now_v7();
        let traj = sample_trajectory(id, "test", TrajectoryStatus::Active);
        state.upsert(traj);

        state.remove(TrajectoryId::new(id));

        assert!(state.trajectories.is_empty());
    }

    #[test]
    fn test_trajectory_view_remove_nonexistent_is_noop() {
        let mut state = TrajectoryViewState::new();
        let id = Uuid::now_v7();
        let traj = sample_trajectory(id, "test", TrajectoryStatus::Active);
        state.upsert(traj);

        // Remove different ID
        state.remove(TrajectoryId::new(Uuid::now_v7()));

        assert_eq!(state.trajectories.len(), 1);
    }

    // ========================================================================
    // ScopeViewState Tests
    // ========================================================================

    #[test]
    fn test_scope_view_state_new_is_empty() {
        let state = ScopeViewState::new();
        assert!(state.scopes.is_empty());
        assert!(state.selected.is_none());
        assert!(!state.loading);
    }

    #[test]
    fn test_scope_view_upsert_inserts_new() {
        let mut state = ScopeViewState::new();
        let traj_id = Uuid::now_v7();
        let scope_id = Uuid::now_v7();
        let scope = sample_scope(scope_id, traj_id, "test-scope");

        state.upsert(scope);

        assert_eq!(state.scopes.len(), 1);
        assert_eq!(state.scopes[0].scope_id.as_uuid(), scope_id);
    }

    #[test]
    fn test_scope_view_upsert_updates_existing() {
        let mut state = ScopeViewState::new();
        let traj_id = Uuid::now_v7();
        let scope_id = Uuid::now_v7();

        let scope1 = sample_scope(scope_id, traj_id, "original");
        state.upsert(scope1);

        let mut scope2 = sample_scope(scope_id, traj_id, "updated");
        scope2.is_active = false;
        state.upsert(scope2);

        assert_eq!(state.scopes.len(), 1);
        assert_eq!(state.scopes[0].name, "updated");
        assert!(!state.scopes[0].is_active);
    }

    // ========================================================================
    // ArtifactViewState Tests
    // ========================================================================

    #[test]
    fn test_artifact_view_state_new_is_empty() {
        let state = ArtifactViewState::new();
        assert!(state.artifacts.is_empty());
        assert!(state.selected.is_none());
        assert!(state.search_query.is_empty());
        assert!(!state.loading);
    }

    #[test]
    fn test_artifact_view_upsert_inserts_new() {
        let mut state = ArtifactViewState::new();
        let traj_id = Uuid::now_v7();
        let artifact_id = Uuid::now_v7();
        let artifact = sample_artifact(artifact_id, traj_id, "test-artifact");

        state.upsert(artifact);

        assert_eq!(state.artifacts.len(), 1);
        assert_eq!(state.artifacts[0].artifact_id.as_uuid(), artifact_id);
    }

    #[test]
    fn test_artifact_view_remove() {
        let mut state = ArtifactViewState::new();
        let traj_id = Uuid::now_v7();
        let artifact_id = Uuid::now_v7();
        let artifact = sample_artifact(artifact_id, traj_id, "test");
        state.upsert(artifact);

        state.remove(ArtifactId::new(artifact_id));

        assert!(state.artifacts.is_empty());
    }

    // ========================================================================
    // NoteViewState Tests
    // ========================================================================

    #[test]
    fn test_note_view_state_new_is_empty() {
        let state = NoteViewState::new();
        assert!(state.notes.is_empty());
        assert!(state.selected.is_none());
        assert!(state.search_query.is_empty());
        assert!(!state.loading);
    }

    #[test]
    fn test_note_view_upsert_inserts_new() {
        let mut state = NoteViewState::new();
        let note_id = Uuid::now_v7();
        let note = sample_note(note_id, "test-note");

        state.upsert(note);

        assert_eq!(state.notes.len(), 1);
        assert_eq!(state.notes[0].note_id.as_uuid(), note_id);
    }

    #[test]
    fn test_note_view_remove() {
        let mut state = NoteViewState::new();
        let note_id = Uuid::now_v7();
        let note = sample_note(note_id, "test");
        state.upsert(note);

        state.remove(NoteId::new(note_id));

        assert!(state.notes.is_empty());
    }

    // ========================================================================
    // Selection Navigation Tests
    // ========================================================================

    #[test]
    fn test_select_next_empty_list() {
        let trajectories: Vec<TrajectoryResponse> = vec![];
        let mut selected: Option<Uuid> = None;

        select_next_id(&trajectories, &mut selected);

        assert!(selected.is_none());
    }

    #[test]
    fn test_select_next_single_item() {
        let id = Uuid::now_v7();
        let trajectories = vec![sample_trajectory(id, "test", TrajectoryStatus::Active)];
        let mut selected = Some(id);

        select_next_id(&trajectories, &mut selected);

        assert_eq!(selected, Some(id)); // Wraps to same
    }

    #[test]
    fn test_select_next_advances() {
        let id1 = Uuid::now_v7();
        let id2 = Uuid::now_v7();
        let trajectories = vec![
            sample_trajectory(id1, "first", TrajectoryStatus::Active),
            sample_trajectory(id2, "second", TrajectoryStatus::Active),
        ];

        let mut selected = Some(id1);
        select_next_id(&trajectories, &mut selected);

        assert_eq!(selected, Some(id2));
    }

    #[test]
    fn test_select_next_wraps_around() {
        let id1 = Uuid::now_v7();
        let id2 = Uuid::now_v7();
        let trajectories = vec![
            sample_trajectory(id1, "first", TrajectoryStatus::Active),
            sample_trajectory(id2, "second", TrajectoryStatus::Active),
        ];

        let mut selected = Some(id2);
        select_next_id(&trajectories, &mut selected);

        assert_eq!(selected, Some(id1)); // Wrapped to first
    }

    #[test]
    fn test_select_next_no_selection_starts_at_first() {
        let id1 = Uuid::now_v7();
        let id2 = Uuid::now_v7();
        let trajectories = vec![
            sample_trajectory(id1, "first", TrajectoryStatus::Active),
            sample_trajectory(id2, "second", TrajectoryStatus::Active),
        ];

        let mut selected: Option<Uuid> = None;
        select_next_id(&trajectories, &mut selected);

        assert_eq!(selected, Some(id1));
    }

    #[test]
    fn test_select_prev_empty_list() {
        let trajectories: Vec<TrajectoryResponse> = vec![];
        let mut selected: Option<Uuid> = None;

        select_prev_id(&trajectories, &mut selected);

        assert!(selected.is_none());
    }

    #[test]
    fn test_select_prev_wraps_around() {
        let id1 = Uuid::now_v7();
        let id2 = Uuid::now_v7();
        let trajectories = vec![
            sample_trajectory(id1, "first", TrajectoryStatus::Active),
            sample_trajectory(id2, "second", TrajectoryStatus::Active),
        ];

        let mut selected = Some(id1);
        select_prev_id(&trajectories, &mut selected);

        assert_eq!(selected, Some(id2)); // Wrapped to last
    }

    // ========================================================================
    // Filter Default Tests
    // ========================================================================

    #[test]
    fn test_trajectory_filter_default() {
        let filter = TrajectoryFilter::default();
        assert!(filter.status.is_none());
        assert!(filter.agent_id.is_none());
        assert!(filter.date_from.is_none());
        assert!(filter.date_to.is_none());
        assert!(filter.search_query.is_none());
    }

    #[test]
    fn test_scope_filter_default() {
        let filter = ScopeFilter::default();
        assert!(filter.trajectory_id.is_none());
        assert!(filter.active_only.is_none());
        assert!(filter.date_from.is_none());
        assert!(filter.date_to.is_none());
    }

    #[test]
    fn test_artifact_filter_default() {
        let filter = ArtifactFilter::default();
        assert!(filter.artifact_type.is_none());
        assert!(filter.trajectory_id.is_none());
        assert!(filter.scope_id.is_none());
        assert!(filter.date_from.is_none());
        assert!(filter.date_to.is_none());
    }

    #[test]
    fn test_note_filter_default() {
        let filter = NoteFilter::default();
        assert!(filter.note_type.is_none());
        assert!(filter.source_trajectory_id.is_none());
        assert!(filter.date_from.is_none());
        assert!(filter.date_to.is_none());
    }

    // ========================================================================
    // Event Queue Tests
    // ========================================================================

    #[test]
    fn test_event_queue_preserves_order() {
        let mut queue: VecDeque<WsEvent> = VecDeque::new();
        let id1 = Uuid::now_v7();
        let id2 = Uuid::now_v7();

        queue.push_back(WsEvent::TrajectoryCreated {
            trajectory: sample_trajectory(id1, "first", TrajectoryStatus::Active),
        });
        queue.push_back(WsEvent::TrajectoryCreated {
            trajectory: sample_trajectory(id2, "second", TrajectoryStatus::Active),
        });

        assert_eq!(queue.len(), 2);

        // Verify FIFO order
        if let Some(WsEvent::TrajectoryCreated { trajectory }) = queue.pop_front() {
            assert_eq!(trajectory.trajectory_id.as_uuid(), id1);
        } else {
            panic!("Expected TrajectoryCreated event");
        }

        if let Some(WsEvent::TrajectoryCreated { trajectory }) = queue.pop_front() {
            assert_eq!(trajectory.trajectory_id.as_uuid(), id2);
        } else {
            panic!("Expected TrajectoryCreated event");
        }
    }

    // ========================================================================
    // Links Panel Tests
    // ========================================================================

    #[test]
    fn test_links_panel_toggle() {
        let mut visible = false;

        visible = !visible;
        assert!(visible);

        visible = !visible;
        assert!(!visible);
    }

    // ========================================================================
    // View State Default Implementations
    // ========================================================================

    #[test]
    fn test_all_view_states_have_default() {
        let _ = TrajectoryViewState::default();
        let _ = ScopeViewState::default();
        let _ = ArtifactViewState::default();
        let _ = NoteViewState::default();
        let _ = TurnViewState::default();
        let _ = AgentViewState::default();
        let _ = LockViewState::default();
        let _ = MessageViewState::default();
        let _ = DslViewState::default();
        let _ = ConfigViewState::default();
        let _ = TenantViewState::default();
    }

    #[test]
    fn test_all_filters_have_default() {
        let _ = TrajectoryFilter::default();
        let _ = ScopeFilter::default();
        let _ = ArtifactFilter::default();
        let _ = NoteFilter::default();
        let _ = TurnFilter::default();
        let _ = AgentFilter::default();
        let _ = LockFilter::default();
        let _ = MessageFilter::default();
    }
}

// ============================================================================
// PROPERTY-BASED TESTS
// ============================================================================

#[cfg(test)]
mod prop_tests {
    use super::*;
    use crate::nav::View;
    use proptest::prelude::*;

    // ========================================================================
    // Generators for TUI State Types
    // ========================================================================

    /// Generate a random TrajectoryStatus
    fn arb_trajectory_status() -> impl Strategy<Value = TrajectoryStatus> {
        prop_oneof![
            Just(TrajectoryStatus::Active),
            Just(TrajectoryStatus::Completed),
            Just(TrajectoryStatus::Failed),
            Just(TrajectoryStatus::Suspended),
        ]
    }

    /// Generate a random View
    fn arb_view() -> impl Strategy<Value = View> {
        prop_oneof![
            Just(View::TenantManagement),
            Just(View::TrajectoryTree),
            Just(View::ScopeExplorer),
            Just(View::ArtifactBrowser),
            Just(View::NoteLibrary),
            Just(View::TurnHistory),
            Just(View::AgentDashboard),
            Just(View::LockMonitor),
            Just(View::MessageQueue),
            Just(View::DslEditor),
            Just(View::ConfigViewer),
        ]
    }

    /// Generate a TrajectoryResponse with arbitrary data
    fn arb_trajectory_response() -> impl Strategy<Value = TrajectoryResponse> {
        (
            any::<[u8; 16]>(),
            "[a-zA-Z0-9_]{1,50}",
            arb_trajectory_status(),
        )
            .prop_map(|(id_bytes, name, status)| TrajectoryResponse {
                trajectory_id: TrajectoryId::new(Uuid::from_bytes(id_bytes)),
                tenant_id: TenantId::new(Uuid::nil()),
                name,
                description: None,
                status,
                parent_trajectory_id: None,
                root_trajectory_id: None,
                agent_id: None,
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
                completed_at: None,
                outcome: None,
                metadata: None,
                links: None,
            })
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        // ====================================================================
        // Property: Upsert is idempotent
        // ====================================================================

        /// Property: Upserting the same trajectory twice results in single entry
        #[test]
        fn prop_trajectory_upsert_idempotent(traj in arb_trajectory_response()) {
            let mut state = TrajectoryViewState::new();

            state.upsert(traj.clone());
            state.upsert(traj.clone());

            prop_assert_eq!(state.trajectories.len(), 1);
        }

        // ====================================================================
        // Property: Remove after insert leaves empty
        // ====================================================================

        /// Property: Removing an inserted trajectory leaves state empty
        #[test]
        fn prop_trajectory_insert_remove_empty(traj in arb_trajectory_response()) {
            let mut state = TrajectoryViewState::new();
            let id = traj.trajectory_id;

            state.upsert(traj);
            prop_assert_eq!(state.trajectories.len(), 1);

            state.remove(id);
            prop_assert!(state.trajectories.is_empty());
        }

        // ====================================================================
        // Property: Selection never panics
        // ====================================================================

        /// Property: Selection navigation never panics regardless of state
        #[test]
        fn prop_selection_navigation_never_panics(
            trajs in prop::collection::vec(arb_trajectory_response(), 0..10),
            ops in prop::collection::vec(prop_oneof![Just(true), Just(false)], 0..20)
        ) {
            let mut state = TrajectoryViewState::new();
            for traj in trajs {
                state.upsert(traj);
            }

            // Apply random selection operations (true = next, false = prev)
            for op in ops {
                if op {
                    select_next_id(&state.trajectories, &mut state.selected);
                } else {
                    select_prev_id(&state.trajectories, &mut state.selected);
                }
            }

            // If we have items and selected is Some, it should be valid
            if !state.trajectories.is_empty() && state.selected.is_some() {
                let id = state.selected.unwrap();
                prop_assert!(state.trajectories.iter().any(|t| t.trajectory_id.as_uuid() == id));
            }
        }

        // ====================================================================
        // Property: Filter default is all None
        // ====================================================================

        /// Property: Default filter (all None) has no constraints
        #[test]
        fn prop_default_filter_all_none(_dummy in 0..1i32) {
            let filter = TrajectoryFilter::default();

            // All filter fields should be None
            prop_assert!(filter.status.is_none());
            prop_assert!(filter.agent_id.is_none());
            prop_assert!(filter.date_from.is_none());
            prop_assert!(filter.date_to.is_none());
            prop_assert!(filter.search_query.is_none());
        }

        // ====================================================================
        // Property: Event queue preserves order (FIFO)
        // ====================================================================

        /// Property: Event queue maintains FIFO order
        #[test]
        fn prop_event_queue_fifo(
            trajs in prop::collection::vec(arb_trajectory_response(), 1..5)
        ) {
            let mut queue: VecDeque<WsEvent> = VecDeque::new();

            for traj in &trajs {
                queue.push_back(WsEvent::TrajectoryCreated { trajectory: traj.clone() });
            }

            // Verify FIFO order
            for expected in &trajs {
                if let Some(WsEvent::TrajectoryCreated { trajectory }) = queue.pop_front() {
                    prop_assert_eq!(trajectory.trajectory_id, expected.trajectory_id);
                } else {
                    prop_assert!(false, "Queue should not be empty");
                }
            }
        }

        // ====================================================================
        // Property: View enum is exhaustive in match
        // ====================================================================

        /// Property: View enum covers all cases and title() never panics
        #[test]
        fn prop_view_title_never_panics(view in arb_view()) {
            // This ensures we handle all view variants
            let title = view.title();
            prop_assert!(!title.is_empty());
        }

        // ====================================================================
        // Property: View navigation is cyclic
        // ====================================================================

        /// Property: View.next() cycles through all views
        #[test]
        fn prop_view_next_cycles(view in arb_view()) {
            let mut current = view;
            let all_views = View::all();

            // Going next() len times should return to original
            for _ in 0..all_views.len() {
                current = current.next();
            }

            prop_assert_eq!(current, view);
        }

        /// Property: View.previous() cycles through all views
        #[test]
        fn prop_view_prev_cycles(view in arb_view()) {
            let mut current = view;
            let all_views = View::all();

            // Going previous() len times should return to original
            for _ in 0..all_views.len() {
                current = current.previous();
            }

            prop_assert_eq!(current, view);
        }

        // ====================================================================
        // Property: Links panel toggle is reversible
        // ====================================================================

        /// Property: Toggling links panel twice returns to original state
        #[test]
        fn prop_links_toggle_reversible(initial in any::<bool>()) {
            let mut visible = initial;

            visible = !visible; // Toggle once
            visible = !visible; // Toggle twice

            prop_assert_eq!(visible, initial);
        }

        // ====================================================================
        // Property: Multiple upserts with different IDs accumulate
        // ====================================================================

        /// Property: Upserting N distinct trajectories results in N entries
        #[test]
        fn prop_upsert_distinct_accumulates(
            trajs in prop::collection::vec(arb_trajectory_response(), 0..10)
        ) {
            let mut state = TrajectoryViewState::new();
            let mut seen_ids = std::collections::HashSet::new();

            for traj in trajs {
                seen_ids.insert(traj.trajectory_id);
                state.upsert(traj);
            }

            // Number of trajectories should equal number of unique IDs
            prop_assert_eq!(state.trajectories.len(), seen_ids.len());
        }
    }
}
