//! Progress bar widget for token utilization.

use ratatui::{
    layout::Rect,
    style::{Style},
    widgets::{Block, Borders, Gauge},
    Frame,
};

pub struct ProgressBar {
    pub title: String,
    pub value: f32,
    pub max: f32,
    pub thresholds: (f32, f32),
    pub low_style: Style,
    pub mid_style: Style,
    pub high_style: Style,
}

impl ProgressBar {
    pub fn render(&self, f: &mut Frame<'_>, area: Rect) {
        let ratio = if self.max <= 0.0 { 0.0 } else { (self.value / self.max).clamp(0.0, 1.0) };
        let percent = ratio * 100.0;

        let style = if percent < self.thresholds.0 {
            self.low_style
        } else if percent < self.thresholds.1 {
            self.mid_style
        } else {
            self.high_style
        };

        let gauge = Gauge::default()
            .block(Block::default().title(self.title.as_str()).borders(Borders::ALL))
            .gauge_style(style)
            .ratio(ratio);
        f.render_widget(gauge, area);
    }
}
