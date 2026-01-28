//! Common traits for TUI components

use crate::state::App;
use ratatui::{layout::Rect, Frame};

/// A component that can be rendered to a terminal frame with application state.
///
/// This trait is implemented by all major view components that need access
/// to the full application state for rendering.
pub trait Renderable {
    /// Render this component to the given frame within the specified area.
    ///
    /// # Arguments
    /// * `f` - The frame to render to
    /// * `app` - The application state
    /// * `area` - The rectangular area to render within
    fn render(&self, f: &mut Frame<'_>, app: &App, area: Rect);
}

/// A widget that renders self-contained content without application state.
///
/// This trait is for simpler widgets that don't need access to the full
/// application state and can render independently.
pub trait Widget {
    /// Render this widget to the given frame within the specified area.
    ///
    /// # Arguments
    /// * `f` - The frame to render to
    /// * `area` - The rectangular area to render within
    fn render(&self, f: &mut Frame<'_>, area: Rect);
}
