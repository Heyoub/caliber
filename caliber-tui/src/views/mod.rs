//! View rendering dispatch.

pub mod agent;
pub mod artifact;
pub mod config;
pub mod dsl;
pub mod helpers;
pub mod lock;
pub mod message;
pub mod note;
pub mod scope;
pub mod tenant;
pub mod trajectory;
pub mod turn;

pub use helpers::{render_links_panel, split_with_links, two_column_with_links};

use crate::nav::View;
use crate::notifications::NotificationLevel;
use crate::state::App;
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Style},
    text::Span,
    widgets::{Block, Borders, Paragraph},
    Frame,
};

pub fn render_view(f: &mut Frame<'_>, app: &App) {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(2),
        ])
        .split(f.size());

    render_header(f, app, layout[0]);

    match app.active_view {
        View::TenantManagement => tenant::render(f, app, layout[1]),
        View::TrajectoryTree => trajectory::render(f, app, layout[1]),
        View::ScopeExplorer => scope::render(f, app, layout[1]),
        View::ArtifactBrowser => artifact::render(f, app, layout[1]),
        View::NoteLibrary => note::render(f, app, layout[1]),
        View::TurnHistory => turn::render(f, app, layout[1]),
        View::AgentDashboard => agent::render(f, app, layout[1]),
        View::LockMonitor => lock::render(f, app, layout[1]),
        View::MessageQueue => message::render(f, app, layout[1]),
        View::DslEditor => dsl::render(f, app, layout[1]),
        View::ConfigViewer => config::render(f, app, layout[1]),
    }

    render_footer(f, app, layout[2]);
}

fn render_header(f: &mut Frame<'_>, app: &App, area: ratatui::layout::Rect) {
    let ws_status = if app.ws_connected { "WS: Connected" } else { "WS: Disconnected" };
    let title = format!(
        "CALIBER TUI | Tenant: {} | {}",
        app.tenant.tenant_name, ws_status
    );
    let block = Block::default().borders(Borders::ALL).title(Span::styled(
        title,
        Style::default().fg(app.theme.primary),
    ));
    f.render_widget(block, area);
}

fn render_footer(f: &mut Frame<'_>, app: &App, area: ratatui::layout::Rect) {
    let help = if app.links_panel_visible {
        "[ / ] nav links • g execute • a close links • q quit"
    } else {
        "h/j/k/l move • Tab switch view • a actions • n new • e edit • / search • q quit"
    };
    let (text, style) = if let Some(note) = app.notifications.last() {
        let label = match note.level {
            NotificationLevel::Info => "INFO",
            NotificationLevel::Warning => "WARN",
            NotificationLevel::Error => "ERROR",
            NotificationLevel::Success => "SUCCESS",
        };
        let color = match note.level {
            NotificationLevel::Info => app.theme.info,
            NotificationLevel::Warning => app.theme.warning,
            NotificationLevel::Error => app.theme.error,
            NotificationLevel::Success => app.theme.success,
        };
        (format!("{}: {}", label, note.message), Style::default().fg(color))
    } else {
        (help.to_string(), Style::default().fg(app.theme.text_dim))
    };
    let footer = Paragraph::new(text)
        .block(Block::default().borders(Borders::ALL))
        .style(style);
    f.render_widget(footer, area);
}
