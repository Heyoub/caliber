//! Tenant management view.

use crate::state::App;
use crate::widgets::DetailPanel;
use caliber_core::EntityIdType;
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
        .tenant_view
        .tenants
        .iter()
        .map(|tenant| {
            let line = format!("{} ({})", tenant.name, tenant.status);
            ListItem::new(line)
        })
        .collect();

    let mut state = ListState::default();
    if let Some(selected) = app.tenant_view.selected {
        if let Some(index) = app
            .tenant_view
            .tenants
            .iter()
            .position(|t| t.tenant_id.as_uuid() == selected)
        {
            state.select(Some(index));
        }
    }

    let list = List::new(items)
        .block(Block::default().title("Tenants").borders(Borders::ALL))
        .highlight_style(Style::default().fg(app.theme.primary));

    f.render_stateful_widget(list, chunks[0], &mut state);

    let mut fields = Vec::new();
    if let Some(selected) = app.tenant_view.selected {
        if let Some(tenant) = app
            .tenant_view
            .tenants
            .iter()
            .find(|t| t.tenant_id.as_uuid() == selected)
        {
            fields.push(("Tenant ID", tenant.tenant_id.to_string()));
            fields.push(("Name", tenant.name.clone()));
            fields.push(("Status", tenant.status.to_string()));
            fields.push(("Created At", tenant.created_at.to_rfc3339()));
        }
    }

    let detail = DetailPanel {
        title: "Details",
        fields,
        style: Style::default().fg(app.theme.secondary),
    };
    detail.render(f, chunks[1]);
}
