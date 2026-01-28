//! Common view rendering helpers.
//!
//! Provides reusable layout functions for views that want to display
//! the links panel alongside their content.

use crate::state::App;
use crate::widgets::{LinksPanel, LinksStyle};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    Frame,
};

/// Split the right panel to include a links panel if visible.
///
/// Returns the areas for detail and links panels.
pub fn split_with_links(app: &App, area: Rect) -> (Rect, Option<Rect>) {
    if app.links_panel_visible && app.links_state.has_actions() {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
            .split(area);
        (chunks[0], Some(chunks[1]))
    } else {
        (area, None)
    }
}

/// Render the links panel with standard styling.
pub fn render_links_panel(f: &mut Frame<'_>, app: &App, area: Rect) {
    let panel = LinksPanel::new("Actions [a]", &app.links_state.actions)
        .with_selected(app.links_state.selected)
        .with_style(LinksStyle::with_theme(
            app.theme.primary,
            app.theme.secondary,
        ));
    panel.render(f, area);
}

/// Standard two-column layout with optional links panel in the right column.
///
/// Returns the left area and the detail area (links panel rendered automatically).
pub fn two_column_with_links(
    f: &mut Frame<'_>,
    app: &App,
    area: Rect,
    left_percent: u16,
) -> (Rect, Rect) {
    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(left_percent),
            Constraint::Percentage(100 - left_percent),
        ])
        .split(area);

    let (detail_area, links_area) = split_with_links(app, main_chunks[1]);

    if let Some(links_area) = links_area {
        render_links_panel(f, app, links_area);
    }

    (main_chunks[0], detail_area)
}
