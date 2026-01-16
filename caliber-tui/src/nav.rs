//! Navigation and view switching utilities.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum View {
    TenantManagement,
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

impl View {
    pub fn title(&self) -> &'static str {
        match self {
            View::TenantManagement => "Tenants",
            View::TrajectoryTree => "Trajectories",
            View::ScopeExplorer => "Scopes",
            View::ArtifactBrowser => "Artifacts",
            View::NoteLibrary => "Notes",
            View::TurnHistory => "Turns",
            View::AgentDashboard => "Agents",
            View::LockMonitor => "Locks",
            View::MessageQueue => "Messages",
            View::DslEditor => "DSL",
            View::ConfigViewer => "Config",
        }
    }

    pub fn all() -> &'static [View] {
        &[
            View::TenantManagement,
            View::TrajectoryTree,
            View::ScopeExplorer,
            View::ArtifactBrowser,
            View::NoteLibrary,
            View::TurnHistory,
            View::AgentDashboard,
            View::LockMonitor,
            View::MessageQueue,
            View::DslEditor,
            View::ConfigViewer,
        ]
    }

    pub fn index(&self) -> usize {
        Self::all()
            .iter()
            .position(|v| v == self)
            .unwrap_or(0)
    }

    pub fn from_index(index: usize) -> Option<View> {
        Self::all().get(index).copied()
    }

    pub fn next(&self) -> View {
        let idx = self.index();
        let all = Self::all();
        let next = (idx + 1) % all.len();
        all[next]
    }

    pub fn previous(&self) -> View {
        let idx = self.index();
        let all = Self::all();
        let prev = if idx == 0 { all.len() - 1 } else { idx - 1 };
        all[prev]
    }
}
