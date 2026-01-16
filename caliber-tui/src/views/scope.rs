//! Scope explorer view.

use crate::state::App;
use crate::widgets::{DetailPanel, ProgressBar};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::Style,
    widgets::{Block, Borders, List, ListItem, ListState},
    Frame,
};

pub fn render(f: &mut Frame<'_>, app: &App, area: ratatui::layout::Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(area);

    let items: Vec<ListItem> = app
        .scope_view
        .scopes
        .iter()
        .map(|scope| {
            let status = if scope.is_active { "Active" } else { "Closed" };
            ListItem::new(format!("{} ({})", scope.name, status))
        })
        .collect();

    let mut state = ListState::default();
    if let Some(selected) = app.scope_view.selected {
        if let Some(index) = app
            .scope_view
            .scopes
            .iter()
            .position(|s| s.scope_id == selected)
        {
            state.select(Some(index));
        }
    }

    let list = List::new(items)
        .block(Block::default().title("Scopes").borders(Borders::ALL))
        .highlight_style(Style::default().fg(app.theme.primary));
    f.render_stateful_widget(list, chunks[0], &mut state);

    if let Some(selected) = app.scope_view.selected {
        if let Some(scope) = app.scope_view.scopes.iter().find(|s| s.scope_id == selected) {
            let utilization = if scope.token_budget == 0 {
                0.0
            } else {
                (scope.tokens_used as f32 / scope.token_budget as f32) * 100.0
            };

            let progress = ProgressBar {
                title: "Token Utilization".to_string(),
                value: scope.tokens_used as f32,
                max: scope.token_budget as f32,
                thresholds: (70.0, 90.0),
                low_style: Style::default().fg(app.theme.success),
                mid_style: Style::default().fg(app.theme.warning),
                high_style: Style::default().fg(app.theme.error),
            };
            let right_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(5), Constraint::Min(0)])
                .split(chunks[1]);
            progress.render(f, right_chunks[0]);

            let mut fields = Vec::new();
            fields.push(("Scope ID", scope.scope_id.to_string()));
            fields.push(("Trajectory ID", scope.trajectory_id.to_string()));
            fields.push(("Tokens Used", scope.tokens_used.to_string()));
            fields.push(("Token Budget", scope.token_budget.to_string()));
            fields.push(("Utilization", format!("{:.1}%", utilization)));
            if let Some(purpose) = &scope.purpose {
                fields.push(("Purpose", purpose.clone()));
            }
            let detail = DetailPanel {
                title: "Details",
                fields,
                style: Style::default().fg(app.theme.secondary),
            };
            detail.render(f, right_chunks[1]);
        }
    }
}
