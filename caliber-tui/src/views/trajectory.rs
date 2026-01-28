//! Trajectory tree view.

use crate::state::App;
use crate::views::two_column_with_links;
use crate::widgets::{DetailPanel, TreeItem, TreeStyle, TreeWidget};
use caliber_api::types::TrajectoryResponse;
use caliber_core::{EntityIdType, TrajectoryId};
use ratatui::{style::Style, Frame};
use std::collections::HashMap;

pub fn render(f: &mut Frame<'_>, app: &App, area: ratatui::layout::Rect) {
    let (tree_area, detail_area) = two_column_with_links(f, app, area, 60);

    let items = build_tree(app);
    let selected_index = app
        .trajectory_view
        .selected
        .and_then(|id| items.iter().position(|item| item.id == id));

    let tree = TreeWidget {
        title: "Trajectories",
        items: &items,
        selected: selected_index,
        style: TreeStyle::new(
            Style::default().fg(app.theme.text),
            Style::default().fg(app.theme.primary),
        ),
    };
    tree.render(f, tree_area);

    render_detail_panel(f, app, detail_area);
}

fn render_detail_panel(f: &mut Frame<'_>, app: &App, area: ratatui::layout::Rect) {
    let mut fields = Vec::new();
    if let Some(selected) = app.trajectory_view.selected {
        if let Some(traj) = app
            .trajectory_view
            .trajectories
            .iter()
            .find(|t| t.trajectory_id.as_uuid() == selected)
        {
            fields.push(("Trajectory ID", traj.trajectory_id.to_string()));
            fields.push(("Name", traj.name.clone()));
            fields.push(("Status", traj.status.to_string()));
            if let Some(desc) = &traj.description {
                fields.push(("Description", desc.clone()));
            }
            fields.push(("Created", traj.created_at.to_rfc3339()));
            fields.push(("Updated", traj.updated_at.to_rfc3339()));
        }
    }

    let detail = DetailPanel {
        title: "Details",
        fields,
        style: Style::default().fg(app.theme.secondary),
    };
    detail.render(f, area);
}

fn build_tree(app: &App) -> Vec<TreeItem> {
    let mut grouped: HashMap<Option<_>, Vec<_>> = HashMap::new();
    for traj in &app.trajectory_view.trajectories {
        grouped
            .entry(traj.parent_trajectory_id)
            .or_default()
            .push(traj);
    }

    for children in grouped.values_mut() {
        children.sort_by_key(|t| t.created_at);
    }

    let mut items = Vec::new();
    walk_tree(None, 0, &grouped, &app.trajectory_view.expanded, &mut items);
    items
}

fn walk_tree(
    parent: Option<TrajectoryId>,
    depth: usize,
    grouped: &HashMap<Option<TrajectoryId>, Vec<&TrajectoryResponse>>,
    expanded: &std::collections::HashSet<uuid::Uuid>,
    output: &mut Vec<TreeItem>,
) {
    if let Some(children) = grouped.get(&parent) {
        for child in children {
            let has_children = grouped.contains_key(&Some(child.trajectory_id));
            let is_expanded = expanded.contains(&child.trajectory_id.as_uuid());
            output.push(TreeItem {
                id: child.trajectory_id.as_uuid(),
                label: format!("{} [{}]", child.name, child.status),
                depth,
                expanded: is_expanded,
                has_children,
            });
            if has_children && is_expanded {
                walk_tree(
                    Some(child.trajectory_id),
                    depth + 1,
                    grouped,
                    expanded,
                    output,
                );
            }
        }
    }
}
