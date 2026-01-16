//! Application state and view state definitions.

use crate::api_client::ApiClient;
use crate::config::TuiConfig;
use crate::nav::View;
use crate::notifications::{Notification, NotificationLevel};
use crate::theme::SynthBruteTheme;
use caliber_api::events::WsEvent;
use caliber_api::types::*;
use caliber_core::{ArtifactType, EntityId, NoteType, Timestamp, TrajectoryStatus, TurnRole};
use std::collections::{HashSet, VecDeque};
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct TenantContext {
    pub tenant_id: EntityId,
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

    pub ws_connected: bool,
    pub updates_paused: bool,
    pub event_queue: VecDeque<WsEvent>,
}

impl App {
    pub fn new(config: TuiConfig, api: ApiClient) -> Self {
        let theme = SynthBruteTheme::synthbrute();
        let tenant_id = config.tenant_id;
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
            ws_connected: false,
            updates_paused: false,
            event_queue: VecDeque::new(),
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
            View::TrajectoryTree => select_next_id(&self.trajectory_view.trajectories, &mut self.trajectory_view.selected),
            View::ScopeExplorer => select_next_id(&self.scope_view.scopes, &mut self.scope_view.selected),
            View::ArtifactBrowser => select_next_id(&self.artifact_view.artifacts, &mut self.artifact_view.selected),
            View::NoteLibrary => select_next_id(&self.note_view.notes, &mut self.note_view.selected),
            View::TurnHistory => select_next_id(&self.turn_view.turns, &mut self.turn_view.selected),
            View::AgentDashboard => select_next_id(&self.agent_view.agents, &mut self.agent_view.selected),
            View::LockMonitor => select_next_id(&self.lock_view.locks, &mut self.lock_view.selected),
            View::MessageQueue => select_next_id(&self.message_view.messages, &mut self.message_view.selected),
            View::TenantManagement => select_next_id(&self.tenant_view.tenants, &mut self.tenant_view.selected),
            View::DslEditor | View::ConfigViewer => {}
        }
    }

    pub fn select_previous(&mut self) {
        match self.active_view {
            View::TrajectoryTree => select_prev_id(&self.trajectory_view.trajectories, &mut self.trajectory_view.selected),
            View::ScopeExplorer => select_prev_id(&self.scope_view.scopes, &mut self.scope_view.selected),
            View::ArtifactBrowser => select_prev_id(&self.artifact_view.artifacts, &mut self.artifact_view.selected),
            View::NoteLibrary => select_prev_id(&self.note_view.notes, &mut self.note_view.selected),
            View::TurnHistory => select_prev_id(&self.turn_view.turns, &mut self.turn_view.selected),
            View::AgentDashboard => select_prev_id(&self.agent_view.agents, &mut self.agent_view.selected),
            View::LockMonitor => select_prev_id(&self.lock_view.locks, &mut self.lock_view.selected),
            View::MessageQueue => select_prev_id(&self.message_view.messages, &mut self.message_view.selected),
            View::TenantManagement => select_prev_id(&self.tenant_view.tenants, &mut self.tenant_view.selected),
            View::DslEditor | View::ConfigViewer => {}
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
            WsEvent::TrajectoryDeleted { id } => {
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
            WsEvent::ArtifactDeleted { id } => {
                self.artifact_view.remove(id);
            }
            WsEvent::NoteCreated { note } => {
                self.note_view.upsert(note);
            }
            WsEvent::NoteUpdated { note } => {
                self.note_view.upsert(note);
            }
            WsEvent::NoteDeleted { id } => {
                self.note_view.remove(id);
            }
            WsEvent::TurnCreated { turn } => {
                self.turn_view.turns.push(turn);
            }
            WsEvent::AgentRegistered { agent } => {
                self.agent_view.upsert(agent);
            }
            WsEvent::AgentStatusChanged { agent_id, status } => {
                self.agent_view.update_status(agent_id, status);
            }
            WsEvent::AgentHeartbeat { agent_id, timestamp } => {
                self.agent_view.update_heartbeat(agent_id, timestamp);
            }
            WsEvent::AgentUnregistered { id } => {
                self.agent_view.remove(id);
            }
            WsEvent::LockAcquired { lock } => {
                self.lock_view.upsert(lock);
            }
            WsEvent::LockReleased { lock_id } => {
                self.lock_view.remove(lock_id);
            }
            WsEvent::LockExpired { lock_id } => {
                self.lock_view.remove(lock_id);
            }
            WsEvent::MessageSent { message } => {
                self.message_view.upsert(message);
            }
            WsEvent::MessageDelivered { message_id } => {
                self.message_view.mark_delivered(message_id);
            }
            WsEvent::MessageAcknowledged { message_id } => {
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
            | WsEvent::EdgesBatchCreated { .. } => {}
        }
    }
}

fn select_next_id<T: HasEntityId>(items: &[T], selected: &mut Option<EntityId>) {
    if items.is_empty() {
        *selected = None;
        return;
    }
    let index = selected
        .and_then(|id| items.iter().position(|item| item.entity_id() == id))
        .unwrap_or(usize::MAX);
    let next = if index == usize::MAX { 0 } else { (index + 1) % items.len() };
    *selected = Some(items[next].entity_id());
}

fn select_prev_id<T: HasEntityId>(items: &[T], selected: &mut Option<EntityId>) {
    if items.is_empty() {
        *selected = None;
        return;
    }
    let index = selected
        .and_then(|id| items.iter().position(|item| item.entity_id() == id))
        .unwrap_or(0);
    let prev = if index == 0 { items.len() - 1 } else { index - 1 };
    *selected = Some(items[prev].entity_id());
}

trait HasEntityId {
    fn entity_id(&self) -> EntityId;
}

impl HasEntityId for TrajectoryResponse {
    fn entity_id(&self) -> EntityId {
        self.trajectory_id
    }
}

impl HasEntityId for ScopeResponse {
    fn entity_id(&self) -> EntityId {
        self.scope_id
    }
}

impl HasEntityId for ArtifactResponse {
    fn entity_id(&self) -> EntityId {
        self.artifact_id
    }
}

impl HasEntityId for NoteResponse {
    fn entity_id(&self) -> EntityId {
        self.note_id
    }
}

impl HasEntityId for TurnResponse {
    fn entity_id(&self) -> EntityId {
        self.turn_id
    }
}

impl HasEntityId for AgentResponse {
    fn entity_id(&self) -> EntityId {
        self.agent_id
    }
}

impl HasEntityId for LockResponse {
    fn entity_id(&self) -> EntityId {
        self.lock_id
    }
}

impl HasEntityId for MessageResponse {
    fn entity_id(&self) -> EntityId {
        self.message_id
    }
}

impl HasEntityId for TenantInfo {
    fn entity_id(&self) -> EntityId {
        self.tenant_id
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
    pub expanded: HashSet<EntityId>,
    pub selected: Option<EntityId>,
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

    pub fn remove(&mut self, id: EntityId) {
        self.trajectories.retain(|t| t.trajectory_id != id);
    }
}

#[derive(Debug, Clone)]
pub struct TrajectoryFilter {
    pub status: Option<TrajectoryStatus>,
    pub agent_id: Option<EntityId>,
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
    pub selected: Option<EntityId>,
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
    pub trajectory_id: Option<EntityId>,
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
    pub selected: Option<EntityId>,
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

    pub fn remove(&mut self, id: EntityId) {
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
    pub trajectory_id: Option<EntityId>,
    pub scope_id: Option<EntityId>,
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
    pub selected: Option<EntityId>,
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
        if let Some(existing) = self
            .notes
            .iter_mut()
            .find(|n| n.note_id == note.note_id)
        {
            *existing = note;
        } else {
            self.notes.push(note);
        }
    }

    pub fn remove(&mut self, id: EntityId) {
        self.notes.retain(|n| n.note_id != id);
    }
}

#[derive(Debug, Clone)]
pub struct NoteFilter {
    pub note_type: Option<NoteType>,
    pub source_trajectory_id: Option<EntityId>,
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
    pub selected: Option<EntityId>,
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
    pub selected: Option<EntityId>,
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

    pub fn remove(&mut self, id: EntityId) {
        self.agents.retain(|a| a.agent_id != id);
    }

    pub fn update_status(&mut self, id: EntityId, status: String) {
        if let Some(agent) = self.agents.iter_mut().find(|a| a.agent_id == id) {
            agent.status = status;
        }
    }

    pub fn update_heartbeat(&mut self, id: EntityId, timestamp: Timestamp) {
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
    pub selected: Option<EntityId>,
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
        if let Some(existing) = self
            .locks
            .iter_mut()
            .find(|l| l.lock_id == lock.lock_id)
        {
            *existing = lock;
        } else {
            self.locks.push(lock);
        }
    }

    pub fn remove(&mut self, id: EntityId) {
        self.locks.retain(|l| l.lock_id != id);
    }
}

#[derive(Debug, Clone)]
pub struct LockFilter {
    pub resource_type: Option<String>,
    pub holder_agent_id: Option<EntityId>,
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
    pub selected: Option<EntityId>,
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

    pub fn mark_delivered(&mut self, id: EntityId) {
        if let Some(message) = self.messages.iter_mut().find(|m| m.message_id == id) {
            message.delivered_at = Some(chrono::Utc::now());
        }
    }

    pub fn mark_acknowledged(&mut self, id: EntityId) {
        if let Some(message) = self.messages.iter_mut().find(|m| m.message_id == id) {
            message.acknowledged_at = Some(chrono::Utc::now());
        }
    }
}

#[derive(Debug, Clone)]
pub struct MessageFilter {
    pub message_type: Option<String>,
    pub from_agent_id: Option<EntityId>,
    pub to_agent_id: Option<EntityId>,
    pub priority: Option<String>,
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
    pub modified: bool,
}

impl DslViewState {
    pub fn new() -> Self {
        Self {
            content: String::new(),
            cursor_position: (0, 0),
            parse_errors: Vec::new(),
            ast_preview: None,
            file_path: None,
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
    pub selected: Option<EntityId>,
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
