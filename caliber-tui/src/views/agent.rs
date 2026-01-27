//! Agent dashboard view.

use crate::state::App;
use crate::views::two_column_with_links;
use caliber_core::EntityIdType;
use crate::theme::agent_status_color;
use crate::widgets::DetailPanel;
use ratatui::{
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState},
    Frame,
};

pub fn render(f: &mut Frame<'_>, app: &App, area: ratatui::layout::Rect) {
    let (list_area, detail_area) = two_column_with_links(f, app, area, 60);

    let items: Vec<ListItem> = app
        .agent_view
        .agents
        .iter()
        .map(|agent| {
            let style = Style::default().fg(agent_status_color(agent.status.as_db_str(), &app.theme));
            ListItem::new(Line::from(Span::styled(
                format!("{} ({})", agent.agent_type, agent.status),
                style,
            )))
        })
        .collect();

    let mut state = ListState::default();
    if let Some(selected) = app.agent_view.selected {
        if let Some(index) = app
            .agent_view
            .agents
            .iter()
            .position(|a| a.agent_id.as_uuid() == selected)
        {
            state.select(Some(index));
        }
    }

    let list = List::new(items)
        .block(Block::default().title("Agents").borders(Borders::ALL))
        .highlight_style(Style::default().fg(app.theme.primary));
    f.render_stateful_widget(list, list_area, &mut state);

    render_detail_panel(f, app, detail_area);
}

fn render_detail_panel(f: &mut Frame<'_>, app: &App, area: ratatui::layout::Rect) {
    let mut fields = Vec::new();
    if let Some(selected) = app.agent_view.selected {
        if let Some(agent) = app
            .agent_view
            .agents
            .iter()
            .find(|a| a.agent_id.as_uuid() == selected)
        {
            fields.push(("Agent ID", agent.agent_id.to_string()));
            fields.push(("Type", agent.agent_type.clone()));
            fields.push(("Status", agent.status.to_string()));
            if let Some(traj_id) = agent.current_trajectory_id {
                fields.push(("Trajectory", traj_id.to_string()));
            }
            if let Some(scope_id) = agent.current_scope_id {
                fields.push(("Scope", scope_id.to_string()));
            }
            if let Some(reports_to) = agent.reports_to {
                fields.push(("Reports To", reports_to.to_string()));
            }
            fields.push(("Created", agent.created_at.to_rfc3339()));
        }
    }

    let detail = DetailPanel {
        title: "Details",
        fields,
        style: Style::default().fg(app.theme.secondary),
    };
    detail.render(f, area);
}

