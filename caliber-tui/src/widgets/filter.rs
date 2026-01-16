//! Filter bar widget.

use ratatui::{
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

#[derive(Debug, Clone)]
pub struct FilterOption {
    pub label: String,
    pub active: bool,
}

pub struct FilterBar<'a> {
    pub title: &'a str,
    pub filters: &'a [FilterOption],
    pub active_style: Style,
    pub inactive_style: Style,
}

impl<'a> FilterBar<'a> {
    pub fn render(&self, f: &mut Frame<'_>, area: Rect) {
        let spans: Vec<Span> = self
            .filters
            .iter()
            .map(|filter| {
                let style = if filter.active {
                    self.active_style
                } else {
                    self.inactive_style
                };
                Span::styled(format!(" {} ", filter.label), style)
            })
            .collect();

        let paragraph = Paragraph::new(Line::from(spans))
            .block(Block::default().title(self.title).borders(Borders::ALL));
        f.render_widget(paragraph, area);
    }
}
