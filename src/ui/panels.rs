//! Side panels: details, legend, filters, groups.
//!
//! UI panels that provide additional information and controls:
//!
//! - Details panel: Shows metrics for the selected node
//! - Legend: Explains visual indicators
//! - Filter panel: Configure topic filtering
//! - Group panel: Organize nodes by patterns

use crate::graph::{HealthStatus, MessageFlowGraph};
use egui::{Color32, Vec2};
use petgraph::graph::EdgeIndex;

use super::theme::Theme;
use super::types::{NodeActivity, NodeGroup};

/// Draw the details panel showing information about the selected node or edge.
///
/// Displays health status, message counts, rates, and topic connections
/// for the currently selected module or edge.
pub fn draw_details_panel(
    ui: &mut egui::Ui,
    graph: &MessageFlowGraph,
    selected_node: &Option<String>,
    selected_edge: &Option<EdgeIndex>,
    node_activity: &std::collections::HashMap<String, NodeActivity>,
    theme: &Theme,
) {
    // Show edge details if an edge is selected
    if let Some(edge_idx) = selected_edge {
        draw_edge_details(ui, graph, *edge_idx, theme);
        return;
    }

    if let Some(name) = selected_node {
        if let Some(node) = graph.graph.node_weights().find(|n| &n.name == name) {
            ui.heading(&node.name);
            ui.separator();

            ui.horizontal(|ui| {
                ui.label("Health:");
                let (health_text, health_color) = match node.health {
                    HealthStatus::Healthy => ("Healthy", theme.neuron_active()),
                    HealthStatus::Warning => ("Warning", theme.neuron_warning()),
                    HealthStatus::Critical => ("Critical", theme.neuron_critical()),
                };
                ui.label(egui::RichText::new(health_text).color(health_color));
            });

            if let Some(activity) = node_activity.get(&node.name) {
                ui.label(format!("Activity: {:.0}%", activity.fire_intensity * 100.0));
            }

            ui.label(format!("Total reads: {}", node.total_reads));
            ui.label(format!("Total writes: {}", node.total_writes));

            if let Some(rate) = node.rate() {
                ui.label(format!("Rate: {:.1} msg/s", rate));
            }

            ui.separator();
            ui.label("Inputs:");
            for topic in &node.read_topics {
                ui.label(format!("  <- {}", topic));
            }

            ui.separator();
            ui.label("Outputs:");
            for topic in &node.write_topics {
                ui.label(format!("  -> {}", topic));
            }
        }
    } else {
        ui.label("Click a neuron or synapse to see details");
    }
}

/// Draw edge details panel.
fn draw_edge_details(
    ui: &mut egui::Ui,
    graph: &MessageFlowGraph,
    edge_idx: EdgeIndex,
    theme: &Theme,
) {
    if let Some(edge) = graph.graph.edge_weight(edge_idx) {
        ui.heading("Synapse Details");
        ui.separator();

        ui.label(format!("Topic: {}", edge.topic));
        ui.add_space(4.0);

        // Source and target
        if let Some((source, target)) = graph.graph.edge_endpoints(edge_idx) {
            let source_name = &graph.graph[source].name;
            let target_name = &graph.graph[target].name;
            ui.label(format!("From: {}", source_name));
            ui.label(format!("To: {}", target_name));
        }

        ui.separator();
        ui.label("Metrics:");

        // Format message count nicely (1.2k for 1200)
        let msg_count = format_count(edge.message_count);
        ui.label(format!("Messages: {}", msg_count));

        // Rate
        if let Some(rate) = edge.rate {
            ui.label(format!("Rate: {:.1} msg/s", rate));
        }

        // Backlog
        if let Some(backlog) = edge.backlog {
            let backlog_str = format_count(backlog);
            ui.label(format!("Backlog: {}", backlog_str));
        }

        // Pending time (format as ms/s)
        if let Some(pending_us) = edge.pending_us {
            let pending_str = format_duration_us(pending_us);
            ui.label(format!("Pending: {}", pending_str));
        }

        ui.separator();

        // Health status
        ui.horizontal(|ui| {
            ui.label("Health:");
            let (health_text, health_color) = match edge.health {
                HealthStatus::Healthy => ("Healthy", theme.neuron_active()),
                HealthStatus::Warning => ("Warning", theme.neuron_warning()),
                HealthStatus::Critical => ("Critical", theme.neuron_critical()),
            };
            ui.label(egui::RichText::new(health_text).color(health_color));
        });
    } else {
        ui.label("Edge not found");
    }
}

/// Format a count nicely (e.g., 1200 -> "1.2k", 1500000 -> "1.5M").
fn format_count(count: u64) -> String {
    if count >= 1_000_000 {
        format!("{:.1}M", count as f64 / 1_000_000.0)
    } else if count >= 1_000 {
        format!("{:.1}k", count as f64 / 1_000.0)
    } else {
        count.to_string()
    }
}

/// Format duration in microseconds to a human-readable string.
fn format_duration_us(us: u64) -> String {
    if us >= 1_000_000 {
        format!("{:.1}s", us as f64 / 1_000_000.0)
    } else if us >= 1_000 {
        format!("{:.1}ms", us as f64 / 1_000.0)
    } else {
        format!("{}µs", us)
    }
}

/// Draw the legend panel explaining visual indicators.
///
/// Shows color meanings for health status, and explains controls
/// for interacting with the graph.
pub fn draw_legend(ui: &mut egui::Ui, theme: &Theme) {
    ui.heading("Legend");
    ui.separator();

    ui.label("Node Status:");
    draw_legend_item(ui, theme.neuron_base(), "Healthy");
    draw_legend_item(ui, theme.neuron_warning(), "Warning");
    draw_legend_item(ui, theme.neuron_critical(), "Critical");
    draw_legend_item(ui, theme.neuron_active(), "Active (firing)");

    ui.add_space(8.0);
    ui.label("Node Size:");
    ui.label("  Larger = higher throughput");

    ui.add_space(8.0);
    ui.label("Edge Width:");
    ui.label("  Thicker = higher msg rate");

    ui.add_space(8.0);
    ui.label("Particles:");
    draw_legend_item(ui, theme.synapse_active(), "Message flow");

    ui.add_space(8.0);
    ui.separator();
    ui.label("Controls:");
    ui.label("  Scroll: Zoom");
    ui.label("  Drag: Pan");
    ui.label("  Click: Select node");
    ui.label("  Ctrl+F: Search");
}

fn draw_legend_item(ui: &mut egui::Ui, color: Color32, label: &str) {
    ui.horizontal(|ui| {
        let (rect, _) = ui.allocate_exact_size(Vec2::new(12.0, 12.0), egui::Sense::hover());
        ui.painter().circle_filled(rect.center(), 6.0, color);
        ui.label(label);
    });
}

/// Draw the topic filter panel for hiding noisy topics.
///
/// Returns `true` if filters were modified, indicating the graph
/// should be updated.
pub fn draw_filter_panel(
    ui: &mut egui::Ui,
    filters: &mut Vec<String>,
    new_filter: &mut String,
) -> bool {
    let mut changed = false;

    ui.heading("Topic Filters");
    ui.separator();
    ui.label("Hide topics matching:");

    // List current filters
    let mut to_remove = None;
    for (i, filter) in filters.iter().enumerate() {
        ui.horizontal(|ui| {
            ui.label(format!("  {}", filter));
            if ui.small_button("x").clicked() {
                to_remove = Some(i);
            }
        });
    }

    if let Some(idx) = to_remove {
        filters.remove(idx);
        changed = true;
    }

    // Add new filter
    ui.horizontal(|ui| {
        let response = ui.text_edit_singleline(new_filter);
        let should_add = ui.button("+").clicked()
            || (response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)));
        if should_add && !new_filter.is_empty() {
            filters.push(new_filter.clone());
            new_filter.clear();
            changed = true;
        }
    });

    ui.add_space(4.0);
    ui.label("Quick filters:");
    ui.horizontal(|ui| {
        if ui.button("query.*").clicked() && !filters.contains(&"cardano.query.".to_string()) {
            filters.push("cardano.query.".to_string());
            changed = true;
        }
    });

    changed
}

/// Draw the node grouping panel for organizing nodes by patterns.
///
/// Allows creating groups based on name suffixes (e.g., "_state", "_validator")
/// to visually cluster related modules.
pub fn draw_group_panel(
    ui: &mut egui::Ui,
    groups: &mut Vec<NodeGroup>,
    new_pattern: &mut String,
    graph: &MessageFlowGraph,
) {
    ui.heading("Node Groups");
    ui.separator();

    // List current groups
    for group in groups.iter_mut() {
        ui.horizontal(|ui| {
            let icon = if group.collapsed { "+" } else { "-" };
            if ui.button(icon).clicked() {
                group.collapsed = !group.collapsed;
            }
            ui.label(&group.name);
            ui.label(format!("({})", group.nodes.len()));
        });
    }

    ui.separator();
    ui.label("Add group by suffix:");
    ui.horizontal(|ui| {
        ui.text_edit_singleline(new_pattern);
        if ui.button("Add").clicked() && !new_pattern.is_empty() {
            let pattern = new_pattern.clone();
            let nodes: Vec<String> = graph
                .graph
                .node_weights()
                .filter(|n| n.name.ends_with(&pattern))
                .map(|n| n.name.clone())
                .collect();

            if !nodes.is_empty() {
                groups.push(NodeGroup {
                    name: format!("*{}", pattern),
                    pattern: pattern.clone(),
                    nodes,
                    collapsed: false,
                    color: Color32::from_rgb(100, 150, 200),
                });
            }
            new_pattern.clear();
        }
    });

    ui.add_space(4.0);
    ui.label("Common patterns:");
    ui.horizontal(|ui| {
        if ui.button("_state").clicked() {
            *new_pattern = "_state".to_string();
        }
        if ui.button("_validator").clicked() {
            *new_pattern = "_validator".to_string();
        }
    });
}
