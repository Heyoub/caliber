//! HATEOAS Links Widget
//!
//! Displays available actions from API `_links` responses.

use caliber_api::types::{Link, Links};
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState},
    Frame,
};

/// A single action derived from a link.
#[derive(Debug, Clone)]
pub struct LinkAction {
    /// The relation name (e.g., "close", "delete", "children").
    pub rel: String,
    /// The link data.
    pub link: Link,
}

impl LinkAction {
    /// Create a new link action.
    pub fn new(rel: impl Into<String>, link: Link) -> Self {
        Self {
            rel: rel.into(),
            link,
        }
    }

    /// Get the HTTP method (defaults to GET).
    pub fn method(&self) -> &str {
        self.link.method.as_deref().unwrap_or("GET")
    }

    /// Get the display title.
    pub fn title(&self) -> &str {
        self.link.title.as_deref().unwrap_or(&self.rel)
    }

    /// Check if this is a "safe" action (GET/HEAD).
    pub fn is_safe(&self) -> bool {
        matches!(self.method().to_uppercase().as_str(), "GET" | "HEAD")
    }

    /// Check if this is a destructive action (DELETE).
    pub fn is_destructive(&self) -> bool {
        self.method().to_uppercase() == "DELETE"
    }
}

/// Extract available actions from links, excluding "self".
pub fn extract_actions(links: &Links) -> Vec<LinkAction> {
    links
        .iter()
        .filter(|(rel, _)| *rel != "self")
        .map(|(rel, link)| LinkAction::new(rel, link.clone()))
        .collect()
}

/// Style configuration for the links panel.
#[derive(Debug, Clone)]
pub struct LinksStyle {
    /// Normal text style.
    pub normal: Style,
    /// Selected item style.
    pub selected: Style,
    /// GET method color.
    pub get_color: Color,
    /// POST method color.
    pub post_color: Color,
    /// PUT/PATCH method color.
    pub patch_color: Color,
    /// DELETE method color.
    pub delete_color: Color,
}

impl Default for LinksStyle {
    fn default() -> Self {
        Self {
            normal: Style::default(),
            selected: Style::default().add_modifier(Modifier::REVERSED),
            get_color: Color::Cyan,
            post_color: Color::Green,
            patch_color: Color::Yellow,
            delete_color: Color::Red,
        }
    }
}

impl LinksStyle {
    /// Create a new style with theme colors.
    pub fn with_theme(primary: Color, secondary: Color) -> Self {
        Self {
            normal: Style::default().fg(secondary),
            selected: Style::default().fg(primary).add_modifier(Modifier::BOLD),
            ..Default::default()
        }
    }

    /// Get the color for a method.
    pub fn method_color(&self, method: &str) -> Color {
        match method.to_uppercase().as_str() {
            "GET" | "HEAD" => self.get_color,
            "POST" => self.post_color,
            "PUT" | "PATCH" => self.patch_color,
            "DELETE" => self.delete_color,
            _ => self.normal.fg.unwrap_or(Color::White),
        }
    }
}

/// Widget for displaying available link actions.
pub struct LinksPanel<'a> {
    /// Panel title.
    pub title: &'a str,
    /// Available actions.
    pub actions: &'a [LinkAction],
    /// Currently selected index.
    pub selected: Option<usize>,
    /// Style configuration.
    pub style: LinksStyle,
    /// Whether to show method badges.
    pub show_methods: bool,
}

impl<'a> LinksPanel<'a> {
    /// Create a new links panel.
    pub fn new(title: &'a str, actions: &'a [LinkAction]) -> Self {
        Self {
            title,
            actions,
            selected: None,
            style: LinksStyle::default(),
            show_methods: true,
        }
    }

    /// Set the selected index.
    pub fn with_selected(mut self, selected: Option<usize>) -> Self {
        self.selected = selected;
        self
    }

    /// Set the style.
    pub fn with_style(mut self, style: LinksStyle) -> Self {
        self.style = style;
        self
    }

    /// Enable or disable method badges.
    pub fn with_methods(mut self, show: bool) -> Self {
        self.show_methods = show;
        self
    }

    /// Render the panel.
    pub fn render(&self, f: &mut Frame<'_>, area: Rect) {
        if self.actions.is_empty() {
            let empty = List::new(vec![ListItem::new("No actions available")])
                .block(Block::default().title(self.title).borders(Borders::ALL))
                .style(Style::default().fg(Color::DarkGray));
            f.render_widget(empty, area);
            return;
        }

        let items: Vec<ListItem> = self
            .actions
            .iter()
            .map(|action| {
                let method = action.method();
                let method_color = self.style.method_color(method);

                let line = if self.show_methods {
                    Line::from(vec![
                        Span::styled(
                            format!("[{}] ", method),
                            Style::default().fg(method_color),
                        ),
                        Span::styled(action.title(), self.style.normal),
                    ])
                } else {
                    Line::from(Span::styled(action.title(), self.style.normal))
                };

                ListItem::new(line)
            })
            .collect();

        let list = List::new(items)
            .block(Block::default().title(self.title).borders(Borders::ALL))
            .highlight_style(self.style.selected);

        let mut state = ListState::default();
        state.select(self.selected);
        f.render_stateful_widget(list, area, &mut state);
    }
}

/// State for managing link selection.
#[derive(Debug, Clone, Default)]
pub struct LinksState {
    /// Available actions.
    pub actions: Vec<LinkAction>,
    /// Currently selected index.
    pub selected: Option<usize>,
}

impl LinksState {
    /// Create a new state from links.
    pub fn from_links(links: Option<&Links>) -> Self {
        let actions = links.map(extract_actions).unwrap_or_default();
        let selected = if actions.is_empty() { None } else { Some(0) };
        Self { actions, selected }
    }

    /// Update the state with new links.
    pub fn update(&mut self, links: Option<&Links>) {
        self.actions = links.map(extract_actions).unwrap_or_default();
        if self.actions.is_empty() {
            self.selected = None;
        } else if let Some(idx) = self.selected {
            if idx >= self.actions.len() {
                self.selected = Some(self.actions.len() - 1);
            }
        } else {
            self.selected = Some(0);
        }
    }

    /// Select the next action.
    pub fn select_next(&mut self) {
        if self.actions.is_empty() {
            return;
        }
        self.selected = Some(match self.selected {
            Some(idx) => (idx + 1) % self.actions.len(),
            None => 0,
        });
    }

    /// Select the previous action.
    pub fn select_previous(&mut self) {
        if self.actions.is_empty() {
            return;
        }
        self.selected = Some(match self.selected {
            Some(idx) => {
                if idx == 0 {
                    self.actions.len() - 1
                } else {
                    idx - 1
                }
            }
            None => 0,
        });
    }

    /// Get the currently selected action.
    pub fn selected_action(&self) -> Option<&LinkAction> {
        self.selected.and_then(|idx| self.actions.get(idx))
    }

    /// Check if there are any actions available.
    pub fn has_actions(&self) -> bool {
        !self.actions.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn sample_links() -> Links {
        let mut links = HashMap::new();
        links.insert("self".to_string(), Link::get("/api/v1/thing/123"));
        links.insert(
            "close".to_string(),
            Link::post("/api/v1/thing/123/close").with_title("Close Thing"),
        );
        links.insert(
            "delete".to_string(),
            Link::delete("/api/v1/thing/123").with_title("Delete"),
        );
        links
    }

    #[test]
    fn test_extract_actions_excludes_self() {
        let links = sample_links();
        let actions = extract_actions(&links);

        assert_eq!(actions.len(), 2);
        assert!(actions.iter().all(|a| a.rel != "self"));
    }

    #[test]
    fn test_link_action_methods() {
        let action = LinkAction::new(
            "delete",
            Link::delete("/api/v1/thing").with_title("Remove"),
        );

        assert_eq!(action.method(), "DELETE");
        assert_eq!(action.title(), "Remove");
        assert!(action.is_destructive());
        assert!(!action.is_safe());
    }

    #[test]
    fn test_links_state_navigation() {
        let links = sample_links();
        let mut state = LinksState::from_links(Some(&links));

        assert_eq!(state.selected, Some(0));
        state.select_next();
        assert_eq!(state.selected, Some(1));
        state.select_next();
        assert_eq!(state.selected, Some(0)); // wraps around
    }
}
