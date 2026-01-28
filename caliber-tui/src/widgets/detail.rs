//! Detail panel widget for showing field/value pairs.

use ratatui::{
    layout::Rect,
    style::Style,
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

pub struct DetailPanel<'a> {
    pub title: &'a str,
    pub fields: Vec<(&'a str, String)>,
    pub style: Style,
}

impl<'a> DetailPanel<'a> {
    pub fn render(&self, f: &mut Frame<'_>, area: Rect) {
        let lines: Vec<Line> = self
            .fields
            .iter()
            .map(|(label, value)| {
                Line::from(vec![
                    Span::styled(format!("{}: ", label), self.style),
                    Span::raw(value.clone()),
                ])
            })
            .collect();

        let text = Text::from(lines);
        let widget = Paragraph::new(text)
            .block(Block::default().title(self.title).borders(Borders::ALL))
            .wrap(Wrap { trim: true });

        f.render_widget(widget, area);
    }
}
