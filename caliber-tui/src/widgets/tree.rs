//! Collapsible tree widget.

use caliber_core::EntityId;
use ratatui::{
    layout::Rect,
    style::{Style},
    widgets::{Block, Borders, List, ListItem, ListState},
    Frame,
};

#[derive(Debug, Clone)]
pub struct TreeItem {
    pub id: EntityId,
    pub label: String,
    pub depth: usize,
    pub expanded: bool,
    pub has_children: bool,
}

#[derive(Debug, Clone)]
pub struct TreeStyle {
    pub normal: Style,
    pub selected: Style,
}

impl TreeStyle {
    pub fn new(normal: Style, selected: Style) -> Self {
        Self { normal, selected }
    }
}

pub struct TreeWidget<'a> {
    pub title: &'a str,
    pub items: &'a [TreeItem],
    pub selected: Option<usize>,
    pub style: TreeStyle,
}

impl<'a> TreeWidget<'a> {
    pub fn render(&self, f: &mut Frame<'_>, area: Rect) {
        let rows: Vec<ListItem> = self
            .items
            .iter()
            .map(|item| {
                let indent = "  ".repeat(item.depth);
                let marker = if item.has_children {
                    if item.expanded { "▾ " } else { "▸ " }
                } else {
                    "  "
                };
                ListItem::new(format!("{}{}{}", indent, marker, item.label))
            })
            .collect();

        let list = List::new(rows)
            .block(Block::default().title(self.title).borders(Borders::ALL))
            .highlight_style(self.style.selected);

        let mut state = ListState::default();
        state.select(self.selected);
        f.render_stateful_widget(list, area, &mut state);
    }
}
