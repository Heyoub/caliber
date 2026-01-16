//! Lock monitor view.

use crate::state::App;
use crate::widgets::DetailPanel;
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::Style,
    widgets::{Block, Borders, List, ListItem, ListState},
    Frame,
};

pub fn render(f: &mut Frame<'_>, app: &App, area: ratatui::layout::Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(area);

    let items: Vec<ListItem> = app
        .lock_view
        .locks
        .iter()
        .map(|lock| ListItem::new(format!("{} [{}]", lock.resource_type, lock.mode)))
        .collect();

    let mut state = ListState::default();
    if let Some(selected) = app.lock_view.selected {
        if let Some(index) = app
            .lock_view
            .locks
            .iter()
            .position(|l| l.lock_id == selected)
        {
            state.select(Some(index));
        }
    }

    let list = List::new(items)
        .block(Block::default().title("Locks").borders(Borders::ALL))
        .highlight_style(Style::default().fg(app.theme.primary));
    f.render_stateful_widget(list, chunks[0], &mut state);

    let mut fields = Vec::new();
    if let Some(selected) = app.lock_view.selected {
        if let Some(lock) = app.lock_view.locks.iter().find(|l| l.lock_id == selected) {
            fields.push(("Lock ID", lock.lock_id.to_string()));
            fields.push(("Resource", lock.resource_type.clone()));
            fields.push(("Mode", lock.mode.clone()));
            fields.push(("Held By", lock.holder_agent_id.to_string()));
            fields.push(("Acquired", lock.acquired_at.to_rfc3339()));
            fields.push(("Expires", lock.expires_at.to_rfc3339()));
        }
    }

    let detail = DetailPanel {
        title: "Details",
        fields,
        style: Style::default().fg(app.theme.secondary),
    };
    detail.render(f, chunks[1]);
}
