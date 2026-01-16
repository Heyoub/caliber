//! Note library view.

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
        .note_view
        .notes
        .iter()
        .map(|note| {
            let title = note.title.clone().unwrap_or_else(|| "Untitled".to_string());
            ListItem::new(format!("{} [{}]", title, note.note_type))
        })
        .collect();

    let mut state = ListState::default();
    if let Some(selected) = app.note_view.selected {
        if let Some(index) = app
            .note_view
            .notes
            .iter()
            .position(|n| n.note_id == selected)
        {
            state.select(Some(index));
        }
    }

    let list = List::new(items)
        .block(Block::default().title("Notes").borders(Borders::ALL))
        .highlight_style(Style::default().fg(app.theme.primary));
    f.render_stateful_widget(list, chunks[0], &mut state);

    if let Some(selected) = app.note_view.selected {
        if let Some(note) = app.note_view.notes.iter().find(|n| n.note_id == selected) {
            let right = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(8), Constraint::Min(0)])
                .split(chunks[1]);

            let mut fields = Vec::new();
            fields.push(("Note ID", note.note_id.to_string()));
            fields.push(("Type", note.note_type.to_string()));
            fields.push(("Scope ID", note.scope_id.to_string()));
            fields.push(("Trajectory ID", note.trajectory_id.to_string()));
            fields.push(("Created", note.created_at.to_rfc3339()));

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
