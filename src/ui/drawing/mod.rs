//! Graph rendering functions.
//!
//! This module handles all visual rendering of the message flow graph,
//! including nodes, edges, particles, and the minimap.

mod edges;
mod effects;
pub mod math;
pub mod minimap;
pub mod nodes;

use crate::graph::HealthStatus;
use egui::{Color32, Pos2, Rect, Stroke, Vec2};

use super::types::{DataRefs, SelectionRefs, StyleConfig, Transform, VisualRefs};

// Re-export public API
pub use math::{bezier_points, bezier_tangent, lerp_color, quadratic_bezier};
pub use minimap::draw_minimap;
pub use nodes::{format_rate, get_neuron_color};

/// Drawing context with all state needed for rendering.
pub struct DrawContext<'a> {
    pub data: DataRefs<'a>,
    pub visual: VisualRefs<'a>,
    pub selection: SelectionRefs<'a>,
    pub style: StyleConfig,
    pub transform: Transform,
}

impl<'a> DrawContext<'a> {
    /// Convert world position to screen position.
    pub fn world_to_screen(&self, world_pos: Pos2, rect: Rect) -> Pos2 {
        let center = rect.center();
        let offset = (world_pos - center) * self.transform.zoom;
        center + offset + self.transform.pan
    }

    /// Draw the complete graph.
    pub fn draw_graph(&self, ui: &mut egui::Ui, rect: Rect) {
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 0.0, self.style.theme.background());

        self.draw_edges(&painter, rect);
        self.draw_nodes(&painter, rect);
    }

    fn draw_edges(&self, painter: &egui::Painter, rect: Rect) {
        for edge_idx in self.data.graph.graph.edge_indices() {
            if let Some((source, target)) = self.data.graph.graph.edge_endpoints(edge_idx) {
                let source_node = &self.data.graph.graph[source];
                let target_node = &self.data.graph.graph[target];
                let edge = &self.data.graph.graph[edge_idx];

                // Check if this edge is selected
                let is_selected = self.selection.edge.is_some_and(|sel| {
                    sel.source_node == source_node.name
                        && sel.target_node == target_node.name
                        && sel.topic == edge.topic
                });

                let world_s = self
                    .data
                    .positions
                    .get(&source_node.name)
                    .copied()
                    .unwrap_or(rect.center());
                let world_t = self
                    .data
                    .positions
                    .get(&target_node.name)
                    .copied()
                    .unwrap_or(rect.center());

                let pos_s = self.world_to_screen(world_s, rect);
                let pos_t = self.world_to_screen(world_t, rect);

                // Use white/bright color for selected edge
                let edge_color = if is_selected {
                    Color32::WHITE
                } else {
                    match edge.health {
                        HealthStatus::Healthy => self.style.theme.synapse_base(),
                        HealthStatus::Warning => {
                            self.style.theme.neuron_warning().gamma_multiply(0.7)
                        }
                        HealthStatus::Critical => {
                            self.style.theme.neuron_critical().gamma_multiply(0.7)
                        }
                    }
                };

                // Make selected edge thicker
                let base_width = (1.0 + (edge.rate.unwrap_or(0.0).log10().max(0.0) as f32) * 0.3)
                    * self.transform.zoom;
                let width = if is_selected {
                    base_width * 2.5
                } else {
                    base_width
                };

                // Calculate Bezier control point
                let mid = pos_s + (pos_t - pos_s) * 0.5;
                let dir = (pos_t - pos_s).normalized();
                let perp = Vec2::new(-dir.y, dir.x);
                let distance = (pos_t - pos_s).length();
                let curve_amount = (distance * 0.15).min(40.0 * self.transform.zoom);
                let control = mid + perp * curve_amount;

                // Draw curved edge
                if self.style.show_gradient_edges {
                    edges::draw_gradient_edge(painter, pos_s, control, pos_t, edge_color, width);
                } else {
                    let segments = 20;
                    let points = bezier_points(pos_s, control, pos_t, segments);
                    for i in 0..points.len() - 1 {
                        painter.line_segment(
                            [points[i], points[i + 1]],
                            Stroke::new(width, edge_color),
                        );
                    }
                }

                // Draw arrow
                let node_radius = 12.0 * self.transform.zoom;
                let arrow_t = 1.0 - (node_radius + 8.0 * self.transform.zoom) / distance.max(1.0);
                let arrow_pos = quadratic_bezier(pos_s, control, pos_t, arrow_t.max(0.8));
                let arrow_dir = bezier_tangent(pos_s, control, pos_t, arrow_t.max(0.8));
                let arrow_perp = Vec2::new(-arrow_dir.y, arrow_dir.x);
                let arrow_size = 5.0 * self.transform.zoom;
                let arrow_points = vec![
                    arrow_pos + arrow_dir * arrow_size,
                    arrow_pos + arrow_perp * arrow_size * 0.4,
                    arrow_pos - arrow_perp * arrow_size * 0.4,
                ];
                painter.add(egui::Shape::convex_polygon(
                    arrow_points,
                    edge_color,
                    Stroke::NONE,
                ));

                // Draw synapse particles
                let key = (
                    source_node.name.clone(),
                    target_node.name.clone(),
                    edge.topic.clone(),
                );
                if let Some(particles) = self.visual.particles.get(&key) {
                    effects::draw_particles(
                        painter,
                        particles,
                        pos_s,
                        control,
                        pos_t,
                        &self.style.theme,
                        self.transform.zoom,
                    );
                }

                // Topic label
                if self.style.show_labels && self.transform.zoom > 0.5 {
                    let label_pos = quadratic_bezier(pos_s, control, pos_t, 0.5);
                    painter.text(
                        label_pos,
                        egui::Align2::CENTER_CENTER,
                        &edge.topic,
                        egui::FontId::proportional(9.0 * self.transform.zoom),
                        self.style.theme.text_secondary(),
                    );
                }
            }
        }
    }

    fn draw_nodes(&self, painter: &egui::Painter, rect: Rect) {
        for node in self.data.graph.graph.node_weights() {
            let world_pos = self
                .data
                .positions
                .get(&node.name)
                .copied()
                .unwrap_or(rect.center());

            let pos = self.world_to_screen(world_pos, rect);

            let base_radius = 12.0 * self.transform.zoom;
            let radius = base_radius
                + (node.throughput() as f32).log10().max(0.0) * 2.0 * self.transform.zoom;

            let activity = self.visual.activity.get(&node.name);
            let color = get_neuron_color(node, activity, &self.style.theme);
            let is_selected = self.selection.node == Some(&node.name);
            let is_highlighted = self.selection.highlighted == Some(&node.name);
            let is_edge_endpoint = self
                .selection
                .edge
                .is_some_and(|sel| sel.source_node == node.name || sel.target_node == node.name);

            let fire_intensity = activity.map(|a| a.fire_intensity).unwrap_or(0.0);

            // Draw pulse rings
            if self.style.show_pulse_rings {
                if let Some(rings) = self.visual.pulse_rings.get(&node.name) {
                    effects::draw_pulse_rings(
                        painter,
                        rings,
                        pos,
                        &self.style.theme,
                        self.transform.zoom,
                    );
                }
            }

            // Glow effect for active neurons
            if fire_intensity > 0.1 {
                let glow_radius = radius + (8.0 + fire_intensity * 10.0) * self.transform.zoom;
                let glow_color = self
                    .style
                    .theme
                    .neuron_active()
                    .gamma_multiply(fire_intensity * 0.4);
                painter.circle_filled(pos, glow_radius, glow_color);
            }

            // Main neuron body
            painter.circle_filled(pos, radius, color);

            // Inner highlight (nucleus)
            let nucleus_color = lerp_color(color, Color32::WHITE, 0.3);
            painter.circle_filled(pos, radius * 0.4, nucleus_color);

            // Group color ring (drawn before selection ring)
            for (group_idx, group) in self.visual.node_groups.iter().enumerate() {
                if !group.collapsed && group.nodes.contains(&node.name) {
                    let group_color = super::types::get_group_color(group_idx);
                    painter.circle_stroke(
                        pos,
                        radius + 6.0 * self.transform.zoom,
                        Stroke::new(3.0 * self.transform.zoom, group_color),
                    );
                    break; // Only show first matching group
                }
            }

            // Selection/highlight ring
            if is_selected || is_highlighted || is_edge_endpoint {
                let ring_color = if is_highlighted {
                    Color32::YELLOW
                } else {
                    Color32::WHITE
                };
                painter.circle_stroke(
                    pos,
                    radius + 4.0 * self.transform.zoom,
                    Stroke::new(2.0, ring_color),
                );
            }

            // Node label
            if self.transform.zoom > 0.4 {
                painter.text(
                    pos + Vec2::new(0.0, radius + 10.0 * self.transform.zoom),
                    egui::Align2::CENTER_CENTER,
                    &node.name,
                    egui::FontId::proportional(10.0 * self.transform.zoom),
                    self.style.theme.text_primary(),
                );

                if let Some(rate) = node.rate() {
                    let rate_text = format_rate(rate);
                    painter.text(
                        pos + Vec2::new(0.0, radius + 22.0 * self.transform.zoom),
                        egui::Align2::CENTER_CENTER,
                        &rate_text,
                        egui::FontId::proportional(8.0 * self.transform.zoom),
                        self.style.theme.text_secondary(),
                    );
                }
            }
        }
    }
}
