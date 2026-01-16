//! Artifact browser view.

use crate::state::App;
use crate::widgets::DetailPanel;
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::Style,
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};

pub fn render(f: &mut Frame<'_>, app: &App, area: ratatui::layout::Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
        .split(area);

    let items: Vec<ListItem> = app
        .artifact_view
        .artifacts
        .iter()
        .map(|artifact| {
            ListItem::new(format!("{} [{}]", artifact.name, artifact.artifact_type))
        })
        .collect();

    let mut state = ListState::default();
    if let Some(selected) = app.artifact_view.selected {
        if let Some(index) = app
            .artifact_view
            .artifacts
            .iter()
            .position(|a| a.artifact_id == selected)
        {
            state.select(Some(index));
        }
    }

    let list = List::new(items)
        .block(Block::default().title("Artifacts").borders(Borders::ALL))
        .highlight_style(Style::default().fg(app.theme.primary));
    f.render_stateful_widget(list, chunks[0], &mut state);

    if let Some(selected) = app.artifact_view.selected {
        if let Some(artifact) = app
            .artifact_view
            .artifacts
            .iter()
            .find(|a| a.artifact_id == selected)
        {
            let right = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(8), Constraint::Min(0)])
                .split(chunks[1]);

            let mut fields = Vec::new();
            fields.push(("Artifact ID", artifact.artifact_id.to_string()));
            fields.push(("Type", artifact.artifact_type.to_string()));
            fields.push(("Scope ID", artifact.scope_id.to_string()));
            fields.push(("Trajectory ID", artifact.trajectory_id.to_string()));
            fields.push(("TTL", format!("{:?}", artifact.ttl)));

            let detail = DetailPanel {
                title: "Details",
                fields,
                style: Style::default().fg(app.theme.secondary),
            };
            detail.render(f, right[0]);

            let content = Paragraph::new(artifact.content.clone())
                .block(Block::default().title("Content").borders(Borders::ALL))
                .wrap(Wrap { trim: false });
            f.render_widget(content, right[1]);
        }
    }
}
