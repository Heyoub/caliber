//! Configuration viewer.

use crate::state::App;
use ratatui::{
    layout::{Constraint, Direction, Layout},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame,
};

pub fn render(f: &mut Frame<'_>, app: &App, area: ratatui::layout::Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(6)])
        .split(area);

    let config_text = Paragraph::new(app.config_view.content.clone())
        .block(Block::default().title("Config").borders(Borders::ALL))
        .wrap(Wrap { trim: false });
    f.render_widget(config_text, chunks[0]);

    let errors: Vec<ListItem> = app
        .config_view
        .validation_errors
        .iter()
        .map(|err| ListItem::new(err.clone()))
        .collect();
    let list = List::new(errors)
        .block(Block::default().title("Validation Errors").borders(Borders::ALL));
    f.render_widget(list, chunks[1]);
}
