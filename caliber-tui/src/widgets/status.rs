//! Status indicator widget.

use ratatui::{
    layout::Rect,
    style::Style,
    widgets::{Block, Borders, Paragraph},
    Frame,
};

pub struct StatusIndicator {
    pub title: String,
    pub status: String,
    pub style: Style,
}

impl StatusIndicator {
    pub fn render(&self, f: &mut Frame<'_>, area: Rect) {
        let paragraph = Paragraph::new(self.status.clone()).style(self.style).block(
            Block::default()
                .title(self.title.as_str())
                .borders(Borders::ALL),
        );
        f.render_widget(paragraph, area);
    }
}
