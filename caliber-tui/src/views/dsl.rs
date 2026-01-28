//! DSL editor view.

use crate::state::App;
use crate::widgets::SyntaxHighlighter;
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::Color,
    widgets::{Block, Borders, List, ListItem},
    Frame,
};

pub fn render(f: &mut Frame<'_>, app: &App, area: ratatui::layout::Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(6)])
        .split(area);

    let highlighter = SyntaxHighlighter {
        keyword_color: app.theme.primary,
        memory_color: app.theme.secondary,
        field_color: app.theme.tertiary,
        string_color: Color::Rgb(255, 165, 0),
        number_color: Color::Rgb(0, 255, 0),
    };
    highlighter.render(f, chunks[0], &app.dsl_view.content);

    let errors: Vec<ListItem> = app
        .dsl_view
        .parse_errors
        .iter()
        .map(|err| {
            ListItem::new(format!(
                "Line {} Col {}: {}",
                err.line, err.column, err.message
            ))
        })
        .collect();
    let list =
        List::new(errors).block(Block::default().title("Parse Errors").borders(Borders::ALL));
    f.render_widget(list, chunks[1]);
}
