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

/// Result from the details panel indicating which edge (if any) was clicked.
pub struct DetailsPanelResult {
    pub clicked_edge: Option<EdgeIndex>,
}

/// Draw the details panel showing information about the selected node or edge.
///
/// Displays health status, message counts, rates, and topic connections
/// for the currently selected module or edge. Topic names in the Inputs/Outputs
/// sections are clickable links that show edge details when clicked.
///
/// Returns information about user interactions (e.g., clicked topic/edge).
pub fn draw_details_panel(
    ui: &mut egui::Ui,
    graph: &MessageFlowGraph,
    selected_node: &Option<String>,
    selected_edge: &Option<EdgeIndex>,
    node_activity: &std::collections::HashMap<String, NodeActivity>,
    theme: &Theme,
) -> DetailsPanelResult {
    let mut result = DetailsPanelResult { clicked_edge: None };

    // Show edge details if an edge is selected
    if let Some(edge_idx) = selected_edge {
        draw_edge_details(ui, graph, *edge_idx, theme);
        return result;
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
                // Find the edge for this input topic (where this node is the target)
                let edge_idx = find_edge_by_topic_to_node(graph, topic, name);
                if edge_idx.is_some() {
                    if ui.link(format!("  <- {}", topic)).clicked() {
                        result.clicked_edge = edge_idx;
                    }
                } else {
                    ui.label(format!("  <- {}", topic));
                }
            }

            ui.separator();
            ui.label("Outputs:");
            for topic in &node.write_topics {
                // Find the edge for this output topic (where this node is the source)
                let edge_idx = find_edge_by_topic_from_node(graph, topic, name);
                if edge_idx.is_some() {
                    if ui.link(format!("  -> {}", topic)).clicked() {
                        result.clicked_edge = edge_idx;
                    }
                } else {
                    ui.label(format!("  -> {}", topic));
                }
            }
        }
    } else {
        ui.label("Click a neuron to see details");
        ui.label("Then click a topic to see synapse metrics");
    }

    result
}

/// Find an edge by topic name where the given node is the target (input).
fn find_edge_by_topic_to_node(
    graph: &MessageFlowGraph,
    topic: &str,
    node_name: &str,
) -> Option<EdgeIndex> {
    for edge_idx in graph.graph.edge_indices() {
        if let Some(edge) = graph.graph.edge_weight(edge_idx) {
            if edge.topic == topic {
                if let Some((_, target_idx)) = graph.graph.edge_endpoints(edge_idx) {
                    if graph.graph[target_idx].name == node_name {
                        return Some(edge_idx);
                    }
                }
            }
        }
    }
    None
}

/// Find an edge by topic name where the given node is the source (output).
fn find_edge_by_topic_from_node(
    graph: &MessageFlowGraph,
    topic: &str,
    node_name: &str,
) -> Option<EdgeIndex> {
    for edge_idx in graph.graph.edge_indices() {
        if let Some(edge) = graph.graph.edge_weight(edge_idx) {
            if edge.topic == topic {
                if let Some((source_idx, _)) = graph.graph.edge_endpoints(edge_idx) {
                    if graph.graph[source_idx].name == node_name {
                        return Some(edge_idx);
                    }
                }
            }
        }
    }
    None
}

/// Draw edge details panel using a stable grid layout to prevent jitter.
fn draw_edge_details(
    ui: &mut egui::Ui,
    graph: &MessageFlowGraph,
    edge_idx: EdgeIndex,
    theme: &Theme,
) {
    if let Some(edge) = graph.graph.edge_weight(edge_idx) {
        ui.heading("Synapse Details");
        ui.separator();

        // Use Grid for stable layout - labels won't jump around
        egui::Grid::new("edge_details_grid")
            .num_columns(2)
            .spacing([8.0, 4.0])
            .show(ui, |ui| {
                ui.label("Topic:");
                ui.label(&edge.topic);
                ui.end_row();

                // Source and target
                if let Some((source, target)) = graph.graph.edge_endpoints(edge_idx) {
                    let source_name = &graph.graph[source].name;
                    let target_name = &graph.graph[target].name;

                    ui.label("From:");
                    ui.label(source_name.as_str());
                    ui.end_row();

                    ui.label("To:");
                    ui.label(target_name.as_str());
                    ui.end_row();
                }
            });

        ui.separator();
        ui.label("Metrics:");

        egui::Grid::new("edge_metrics_grid")
            .num_columns(2)
            .spacing([8.0, 4.0])
            .min_col_width(70.0)
            .show(ui, |ui| {
                // Always show all metrics with consistent layout
                ui.label("Messages:");
                ui.label(format_count(edge.message_count));
                ui.end_row();

                ui.label("Rate:");
                ui.label(match edge.rate {
                    Some(rate) => format!("{:.1} msg/s", rate),
                    None => "-".to_string(),
                });
                ui.end_row();

                ui.label("Backlog:");
                ui.label(match edge.backlog {
                    Some(backlog) => format_count(backlog),
                    None => "-".to_string(),
                });
                ui.end_row();

                ui.label("Pending:");
                ui.label(match edge.pending_us {
                    Some(pending_us) => format_duration_us(pending_us),
                    None => "-".to_string(),
                });
                ui.end_row();

                ui.label("Health:");
                let (health_text, health_color) = match edge.health {
                    HealthStatus::Healthy => ("Healthy", theme.neuron_active()),
                    HealthStatus::Warning => ("Warning", theme.neuron_warning()),
                    HealthStatus::Critical => ("Critical", theme.neuron_critical()),
                };
                ui.label(egui::RichText::new(health_text).color(health_color));
                ui.end_row();
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
