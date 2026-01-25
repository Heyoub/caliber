//! Message queue view.

use crate::state::App;
use caliber_core::EntityIdType;
use crate::theme::message_priority_color;
use crate::widgets::DetailPanel;
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
        .message_view
        .messages
        .iter()
        .map(|message| {
            let style = Style::default().fg(message_priority_color(message.priority.as_db_str(), &app.theme));
            let to = message
                .recipient_id
                .map(|id| id.to_string())
                .or_else(|| message.to_agent_type.clone())
                .unwrap_or_else(|| "unspecified".to_string());
            let label = format!(
                "{} -> {} [{}]",
                message.sender_id, to, message.priority
            );
            ListItem::new(Line::from(Span::styled(label, style)))
        })
        .collect();

    let mut state = ListState::default();
    if let Some(selected) = app.message_view.selected {
        if let Some(index) = app
            .message_view
            .messages
            .iter()
            .position(|m| m.message_id.as_uuid() == selected)
        {
            state.select(Some(index));
        }
    }

    let list = List::new(items)
        .block(Block::default().title("Messages").borders(Borders::ALL))
        .highlight_style(Style::default().fg(app.theme.primary));
    f.render_stateful_widget(list, chunks[0], &mut state);

    if let Some(selected) = app.message_view.selected {
        if let Some(message) = app
            .message_view
            .messages
            .iter()
            .find(|m| m.message_id.as_uuid() == selected)
        {
            let right = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(9), Constraint::Min(0)])
                .split(chunks[1]);

            let mut fields = vec![
                ("Message ID", message.message_id.to_string()),
                ("Type", message.message_type.to_string()),
                ("From", message.sender_id.to_string()),
                (
                    "To",
                    message
                        .recipient_id
                        .map(|id| id.to_string())
                        .or_else(|| message.to_agent_type.clone())
                        .unwrap_or_else(|| "unspecified".to_string()),
                ),
                ("Priority", message.priority.to_string()),
                ("Created", message.created_at.to_rfc3339()),
            ];
            if let Some(delivered) = message.delivered_at {
                fields.push(("Delivered", delivered.to_rfc3339()));
            }
            if let Some(ack) = message.acknowledged_at {
                fields.push(("Acknowledged", ack.to_rfc3339()));
            }

            let detail = DetailPanel {
                title: "Details",
                fields,
                style: Style::default().fg(app.theme.secondary),
            };
            detail.render(f, right[0]);

            let content = Paragraph::new(message.payload.clone())
                .block(Block::default().title("Content").borders(Borders::ALL))
                .wrap(Wrap { trim: false });
            f.render_widget(content, right[1]);
        }
    }
}
