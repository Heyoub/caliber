//! Turn history view.

use crate::state::App;
use caliber_core::EntityIdType;
use crate::theme::turn_role_color;
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};

pub fn render(f: &mut Frame<'_>, app: &App, area: ratatui::layout::Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
        .split(area);

    let items: Vec<ListItem> = app
        .turn_view
        .turns
        .iter()
        .map(|turn| {
            let snippet = turn.content.chars().take(60).collect::<String>();
            let style = Style::default().fg(turn_role_color(turn.role, &app.theme));
            ListItem::new(Line::from(Span::styled(
                format!("[{}] {}", turn.role, snippet),
                style,
            )))
        })
        .collect();

    let mut state = ListState::default();
    if let Some(selected) = app.turn_view.selected {
        if let Some(index) = app
            .turn_view
            .turns
            .iter()
            .position(|t| t.turn_id.as_uuid() == selected)
        {
            state.select(Some(index));
        }
    }

    let list = List::new(items)
        .block(Block::default().title("Turns").borders(Borders::ALL))
        .highlight_style(Style::default().fg(app.theme.primary));
    f.render_stateful_widget(list, chunks[0], &mut state);

    if let Some(selected) = app.turn_view.selected {
        if let Some(turn) = app.turn_view.turns.iter().find(|t| t.turn_id.as_uuid() == selected) {
            let right = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(6), Constraint::Min(0)])
                .split(chunks[1]);

            let header = Paragraph::new(format!(
                "Turn ID: {}\nRole: {}\nCreated: {}",
                turn.turn_id,
                turn.role,
                turn.created_at.to_rfc3339()
            ))
            .block(Block::default().title("Metadata").borders(Borders::ALL));
            f.render_widget(header, right[0]);

            let content = Paragraph::new(turn.content.clone())
                .block(Block::default().title("Content").borders(Borders::ALL))
                .wrap(Wrap { trim: false });
            f.render_widget(content, right[1]);
        }
    }
}
