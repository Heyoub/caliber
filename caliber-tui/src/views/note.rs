//! Note library view.

use crate::state::App;
use crate::widgets::DetailPanel;
use caliber_core::EntityIdType;
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
        .note_view
        .notes
        .iter()
        .map(|note| ListItem::new(format!("{} [{}]", note.title, note.note_type)))
        .collect();

    let mut state = ListState::default();
    if let Some(selected) = app.note_view.selected {
        if let Some(index) = app
            .note_view
            .notes
            .iter()
            .position(|n| n.note_id.as_uuid() == selected)
        {
            state.select(Some(index));
        }
    }

    let list = List::new(items)
        .block(Block::default().title("Notes").borders(Borders::ALL))
        .highlight_style(Style::default().fg(app.theme.primary));
    f.render_stateful_widget(list, chunks[0], &mut state);

    if let Some(selected) = app.note_view.selected {
        if let Some(note) = app
            .note_view
            .notes
            .iter()
            .find(|n| n.note_id.as_uuid() == selected)
        {
            let right = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(8), Constraint::Min(0)])
                .split(chunks[1]);

            let mut fields = Vec::new();
            fields.push(("Note ID", note.note_id.to_string()));
            fields.push(("Type", note.note_type.to_string()));
            fields.push(("Created", note.created_at.to_rfc3339()));
            if !note.source_trajectory_ids.is_empty() {
                let ids = note
                    .source_trajectory_ids
                    .iter()
                    .map(|id| id.to_string())
                    .collect::<Vec<_>>()
                    .join(", ");
                fields.push(("Trajectories", ids));
            }

            let detail = DetailPanel {
                title: "Details",
                fields,
                style: Style::default().fg(app.theme.secondary),
            };
            detail.render(f, right[0]);

            let content = Paragraph::new(note.content.clone())
                .block(Block::default().title("Content").borders(Borders::ALL))
                .wrap(Wrap { trim: false });
            f.render_widget(content, right[1]);
        }
    }
}
