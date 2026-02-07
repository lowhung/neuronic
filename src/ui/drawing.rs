//! Graph rendering functions.
//!
//! This module handles all visual rendering of the message flow graph,
//! including nodes, edges, particles, and the minimap.

use crate::graph::{HealthStatus, MessageFlowGraph, ModuleNode};
use egui::{Color32, Pos2, Rect, Stroke, Vec2};
use std::collections::HashMap;

use super::theme::Theme;
use super::types::{DataRefs, NodeActivity, SelectionRefs, StyleConfig, Transform, VisualRefs};

/// Calculate a point on a quadratic Bezier curve.
///
/// # Arguments
///
/// * `p0` - Start point
/// * `p1` - Control point
/// * `p2` - End point
/// * `t` - Parameter in range [0, 1] where 0 = start, 1 = end
///
/// # Returns
///
/// The interpolated position on the curve.
pub fn quadratic_bezier(p0: Pos2, p1: Pos2, p2: Pos2, t: f32) -> Pos2 {
    let t = t.clamp(0.0, 1.0);
    let mt = 1.0 - t;
    Pos2::new(
        mt * mt * p0.x + 2.0 * mt * t * p1.x + t * t * p2.x,
        mt * mt * p0.y + 2.0 * mt * t * p1.y + t * t * p2.y,
    )
}

/// Generate evenly-spaced points along a quadratic Bezier curve.
///
/// Used for drawing smooth curved edges between nodes.
///
/// # Arguments
///
/// * `p0` - Start point
/// * `p1` - Control point
/// * `p2` - End point
/// * `segments` - Number of line segments to generate
pub fn bezier_points(p0: Pos2, p1: Pos2, p2: Pos2, segments: usize) -> Vec<Pos2> {
    (0..=segments)
        .map(|i| {
            let t = i as f32 / segments as f32;
            quadratic_bezier(p0, p1, p2, t)
        })
        .collect()
}

/// Calculate the tangent direction at a point on a quadratic Bezier curve.
///
/// Used for orienting arrows and particles along edge curves.
///
/// # Arguments
///
/// * `p0` - Start point
/// * `p1` - Control point
/// * `p2` - End point
/// * `t` - Parameter in range [0, 1]
///
/// # Returns
///
/// A normalized direction vector tangent to the curve at parameter `t`.
pub fn bezier_tangent(p0: Pos2, p1: Pos2, p2: Pos2, t: f32) -> Vec2 {
    let t = t.clamp(0.0, 1.0);
    let mt = 1.0 - t;
    Vec2::new(
        2.0 * mt * (p1.x - p0.x) + 2.0 * t * (p2.x - p1.x),
        2.0 * mt * (p1.y - p0.y) + 2.0 * t * (p2.y - p1.y),
    )
    .normalized()
}

/// Linearly interpolate between two colors.
///
/// # Arguments
///
/// * `a` - Start color (at t=0)
/// * `b` - End color (at t=1)
/// * `t` - Interpolation factor, clamped to [0, 1]
pub fn lerp_color(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    Color32::from_rgba_unmultiplied(
        (a.r() as f32 + (b.r() as f32 - a.r() as f32) * t) as u8,
        (a.g() as f32 + (b.g() as f32 - a.g() as f32) * t) as u8,
        (a.b() as f32 + (b.b() as f32 - a.b() as f32) * t) as u8,
        (a.a() as f32 + (b.a() as f32 - a.a() as f32) * t) as u8,
    )
}

/// Format a message rate for display.
///
/// Automatically selects appropriate units (M/s, k/s, /s) based on magnitude.
pub fn format_rate(rate: f64) -> String {
    if rate >= 1_000_000.0 {
        format!("{:.1}M/s", rate / 1_000_000.0)
    } else if rate >= 1_000.0 {
        format!("{:.1}k/s", rate / 1_000.0)
    } else if rate >= 1.0 {
        format!("{:.0}/s", rate)
    } else if rate > 0.0 {
        format!("{:.1}/s", rate)
    } else {
        String::new()
    }
}

/// Determine the display color for a node based on health status and activity.
///
/// Colors transition from base → active → firing as activity increases,
/// with the base color determined by health status (healthy/warning/critical).
pub fn get_neuron_color(
    node: &ModuleNode,
    activity: Option<&NodeActivity>,
    theme: &Theme,
) -> Color32 {
    let fire_intensity = activity.map(|a| a.fire_intensity).unwrap_or(0.0);

    let base = match node.health {
        HealthStatus::Healthy => theme.neuron_base(),
        HealthStatus::Warning => theme.neuron_warning(),
        HealthStatus::Critical => theme.neuron_critical(),
    };

    if fire_intensity > 0.5 {
        lerp_color(
            theme.neuron_active(),
            theme.neuron_firing(),
            (fire_intensity - 0.5) * 2.0,
        )
    } else if fire_intensity > 0.0 {
        lerp_color(base, theme.neuron_active(), fire_intensity * 2.0)
    } else {
        base
    }
}

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
                    self.draw_gradient_edge(painter, pos_s, control, pos_t, edge_color, width);
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
                    for particle in particles {
                        let particle_pos =
                            quadratic_bezier(pos_s, control, pos_t, particle.progress);
                        let particle_color = self.style.theme.synapse_active();
                        let glow_radius = 4.0 * self.transform.zoom;
                        painter.circle_filled(
                            particle_pos,
                            glow_radius * 2.0,
                            particle_color.gamma_multiply(0.3),
                        );
                        painter.circle_filled(particle_pos, glow_radius, particle_color);
                    }
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

    fn draw_gradient_edge(
        &self,
        painter: &egui::Painter,
        p0: Pos2,
        p1: Pos2,
        p2: Pos2,
        base_color: Color32,
        width: f32,
    ) {
        let segments = 20;
        let points = bezier_points(p0, p1, p2, segments);

        for i in 0..points.len() - 1 {
            let t = i as f32 / segments as f32;
            // Gradient from darker to brighter along the edge
            let brightness = 0.5 + t * 0.5;
            let color = Color32::from_rgba_unmultiplied(
                (base_color.r() as f32 * brightness) as u8,
                (base_color.g() as f32 * brightness) as u8,
                (base_color.b() as f32 * brightness) as u8,
                base_color.a(),
            );
            painter.line_segment([points[i], points[i + 1]], Stroke::new(width, color));
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
                    for ring in rings {
                        let ring_color = self
                            .style
                            .theme
                            .neuron_active()
                            .gamma_multiply(ring.opacity * 0.5);
                        painter.circle_stroke(
                            pos,
                            ring.radius * self.transform.zoom,
                            Stroke::new(2.0 * self.transform.zoom, ring_color),
                        );
                    }
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

/// Draw a minimap overview in the bottom-right corner.
///
/// Shows all nodes as dots with a rectangle indicating the current viewport.
/// Helps with navigation when zoomed in on large graphs.
pub fn draw_minimap(
    ui: &mut egui::Ui,
    _graph: &MessageFlowGraph,
    positions: &HashMap<String, Pos2>,
    viewport_rect: Rect,
    zoom: f32,
    pan: Vec2,
    theme: &Theme,
) {
    let minimap_size = Vec2::new(150.0, 100.0);
    let minimap_rect = Rect::from_min_size(
        Pos2::new(
            viewport_rect.right() - minimap_size.x - 10.0,
            viewport_rect.bottom() - minimap_size.y - 10.0,
        ),
        minimap_size,
    );

    let painter = ui.painter_at(minimap_rect);

    // Background
    painter.rect_filled(minimap_rect, 4.0, theme.panel_fill().gamma_multiply(0.9));
    painter.rect_stroke(
        minimap_rect,
        4.0,
        Stroke::new(1.0, theme.text_secondary()),
        egui::StrokeKind::Outside,
    );

    if positions.is_empty() {
        return;
    }

    // Find bounds of all nodes
    let mut min_x = f32::MAX;
    let mut max_x = f32::MIN;
    let mut min_y = f32::MAX;
    let mut max_y = f32::MIN;

    for pos in positions.values() {
        min_x = min_x.min(pos.x);
        max_x = max_x.max(pos.x);
        min_y = min_y.min(pos.y);
        max_y = max_y.max(pos.y);
    }

    let world_width = (max_x - min_x).max(100.0);
    let world_height = (max_y - min_y).max(100.0);
    let world_center = Pos2::new((min_x + max_x) / 2.0, (min_y + max_y) / 2.0);

    let scale_x = (minimap_size.x - 20.0) / world_width;
    let scale_y = (minimap_size.y - 20.0) / world_height;
    let scale = scale_x.min(scale_y);

    // Draw nodes as dots
    for pos in positions.values() {
        let minimap_pos = Pos2::new(
            minimap_rect.center().x + (pos.x - world_center.x) * scale,
            minimap_rect.center().y + (pos.y - world_center.y) * scale,
        );

        if minimap_rect.contains(minimap_pos) {
            painter.circle_filled(minimap_pos, 2.0, theme.neuron_base());
        }
    }

    // Draw viewport rectangle
    let viewport_center = viewport_rect.center();
    let viewport_world_center = Pos2::new(viewport_center.x - pan.x, viewport_center.y - pan.y);

    let viewport_world_size =
        Vec2::new(viewport_rect.width() / zoom, viewport_rect.height() / zoom);

    let minimap_viewport_center = Pos2::new(
        minimap_rect.center().x + (viewport_world_center.x - world_center.x) * scale,
        minimap_rect.center().y + (viewport_world_center.y - world_center.y) * scale,
    );

    let minimap_viewport_size =
        Vec2::new(viewport_world_size.x * scale, viewport_world_size.y * scale);

    let minimap_viewport_rect =
        Rect::from_center_size(minimap_viewport_center, minimap_viewport_size);

    painter.rect_stroke(
        minimap_viewport_rect,
        2.0,
        Stroke::new(1.0, Color32::WHITE.gamma_multiply(0.5)),
        egui::StrokeKind::Outside,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    // Bezier curve tests
    #[test]
    fn test_quadratic_bezier_start_point() {
        let p0 = Pos2::new(0.0, 0.0);
        let p1 = Pos2::new(50.0, 100.0);
        let p2 = Pos2::new(100.0, 0.0);

        let result = quadratic_bezier(p0, p1, p2, 0.0);
        assert!((result.x - p0.x).abs() < 0.001);
        assert!((result.y - p0.y).abs() < 0.001);
    }

    #[test]
    fn test_quadratic_bezier_end_point() {
        let p0 = Pos2::new(0.0, 0.0);
        let p1 = Pos2::new(50.0, 100.0);
        let p2 = Pos2::new(100.0, 0.0);

        let result = quadratic_bezier(p0, p1, p2, 1.0);
        assert!((result.x - p2.x).abs() < 0.001);
        assert!((result.y - p2.y).abs() < 0.001);
    }

    #[test]
    fn test_quadratic_bezier_midpoint() {
        let p0 = Pos2::new(0.0, 0.0);
        let p1 = Pos2::new(50.0, 100.0);
        let p2 = Pos2::new(100.0, 0.0);

        let result = quadratic_bezier(p0, p1, p2, 0.5);
        // At t=0.5: (0.25*0 + 0.5*50 + 0.25*100, 0.25*0 + 0.5*100 + 0.25*0) = (50, 50)
        assert!((result.x - 50.0).abs() < 0.001);
        assert!((result.y - 50.0).abs() < 0.001);
    }

    #[test]
    fn test_quadratic_bezier_clamps_t() {
        let p0 = Pos2::new(0.0, 0.0);
        let p1 = Pos2::new(50.0, 100.0);
        let p2 = Pos2::new(100.0, 0.0);

        // t < 0 should clamp to 0
        let result_neg = quadratic_bezier(p0, p1, p2, -0.5);
        assert!((result_neg.x - p0.x).abs() < 0.001);

        // t > 1 should clamp to 1
        let result_over = quadratic_bezier(p0, p1, p2, 1.5);
        assert!((result_over.x - p2.x).abs() < 0.001);
    }

    #[test]
    fn test_bezier_points_count() {
        let p0 = Pos2::new(0.0, 0.0);
        let p1 = Pos2::new(50.0, 100.0);
        let p2 = Pos2::new(100.0, 0.0);

        let points = bezier_points(p0, p1, p2, 10);
        assert_eq!(points.len(), 11); // segments + 1
    }

    #[test]
    fn test_bezier_points_endpoints() {
        let p0 = Pos2::new(0.0, 0.0);
        let p1 = Pos2::new(50.0, 100.0);
        let p2 = Pos2::new(100.0, 0.0);

        let points = bezier_points(p0, p1, p2, 10);
        assert!((points[0].x - p0.x).abs() < 0.001);
        assert!((points[0].y - p0.y).abs() < 0.001);
        assert!((points[10].x - p2.x).abs() < 0.001);
        assert!((points[10].y - p2.y).abs() < 0.001);
    }

    #[test]
    fn test_bezier_tangent_start() {
        let p0 = Pos2::new(0.0, 0.0);
        let p1 = Pos2::new(50.0, 50.0);
        let p2 = Pos2::new(100.0, 0.0);

        let tangent = bezier_tangent(p0, p1, p2, 0.0);
        // At t=0, tangent should point toward control point from start
        // Direction: p1 - p0 = (50, 50), normalized = (0.707, 0.707)
        assert!((tangent.x - 0.707).abs() < 0.01);
        assert!((tangent.y - 0.707).abs() < 0.01);
    }

    #[test]
    fn test_bezier_tangent_end() {
        let p0 = Pos2::new(0.0, 0.0);
        let p1 = Pos2::new(50.0, 50.0);
        let p2 = Pos2::new(100.0, 0.0);

        let tangent = bezier_tangent(p0, p1, p2, 1.0);
        // At t=1, tangent should point from control to end
        // Direction: p2 - p1 = (50, -50), normalized = (0.707, -0.707)
        assert!((tangent.x - 0.707).abs() < 0.01);
        assert!((tangent.y - (-0.707)).abs() < 0.01);
    }

    #[test]
    fn test_bezier_tangent_is_normalized() {
        let p0 = Pos2::new(0.0, 0.0);
        let p1 = Pos2::new(100.0, 200.0);
        let p2 = Pos2::new(300.0, 50.0);

        for t in [0.0, 0.25, 0.5, 0.75, 1.0] {
            let tangent = bezier_tangent(p0, p1, p2, t);
            let length = (tangent.x * tangent.x + tangent.y * tangent.y).sqrt();
            assert!(
                (length - 1.0).abs() < 0.001,
                "Tangent at t={} not normalized",
                t
            );
        }
    }

    // Color interpolation tests
    #[test]
    fn test_lerp_color_start() {
        let a = Color32::from_rgb(0, 0, 0);
        let b = Color32::from_rgb(255, 255, 255);

        let result = lerp_color(a, b, 0.0);
        assert_eq!(result.r(), 0);
        assert_eq!(result.g(), 0);
        assert_eq!(result.b(), 0);
    }

    #[test]
    fn test_lerp_color_end() {
        let a = Color32::from_rgb(0, 0, 0);
        let b = Color32::from_rgb(255, 255, 255);

        let result = lerp_color(a, b, 1.0);
        assert_eq!(result.r(), 255);
        assert_eq!(result.g(), 255);
        assert_eq!(result.b(), 255);
    }

    #[test]
    fn test_lerp_color_midpoint() {
        let a = Color32::from_rgb(0, 0, 0);
        let b = Color32::from_rgb(200, 100, 50);

        let result = lerp_color(a, b, 0.5);
        assert_eq!(result.r(), 100);
        assert_eq!(result.g(), 50);
        assert_eq!(result.b(), 25);
    }

    #[test]
    fn test_lerp_color_with_alpha() {
        let a = Color32::from_rgba_unmultiplied(100, 100, 100, 0);
        let b = Color32::from_rgba_unmultiplied(100, 100, 100, 200);

        let result = lerp_color(a, b, 0.5);
        assert_eq!(result.a(), 100);
    }

    #[test]
    fn test_lerp_color_clamps() {
        let a = Color32::from_rgb(50, 50, 50);
        let b = Color32::from_rgb(100, 100, 100);

        let result_neg = lerp_color(a, b, -0.5);
        assert_eq!(result_neg.r(), 50); // Clamped to t=0

        let result_over = lerp_color(a, b, 1.5);
        assert_eq!(result_over.r(), 100); // Clamped to t=1
    }

    // Format rate tests
    #[test]
    fn test_format_rate_millions() {
        assert_eq!(format_rate(1_500_000.0), "1.5M/s");
        assert_eq!(format_rate(2_000_000.0), "2.0M/s");
    }

    #[test]
    fn test_format_rate_thousands() {
        assert_eq!(format_rate(1_500.0), "1.5k/s");
        assert_eq!(format_rate(50_000.0), "50.0k/s");
    }

    #[test]
    fn test_format_rate_ones() {
        assert_eq!(format_rate(50.0), "50/s");
        assert_eq!(format_rate(1.0), "1/s");
    }

    #[test]
    fn test_format_rate_fractional() {
        assert_eq!(format_rate(0.5), "0.5/s");
        assert_eq!(format_rate(0.1), "0.1/s");
    }

    #[test]
    fn test_format_rate_zero() {
        assert_eq!(format_rate(0.0), "");
    }

    // get_neuron_color tests
    #[test]
    fn test_get_neuron_color_healthy_no_activity() {
        let node = ModuleNode {
            name: "test".to_string(),
            total_reads: 0,
            total_writes: 0,
            read_rate: None,
            write_rate: None,
            health: HealthStatus::Healthy,
            read_topics: vec![],
            write_topics: vec![],
        };
        let theme = Theme::Dark;

        let color = get_neuron_color(&node, None, &theme);
        assert_eq!(color, theme.neuron_base());
    }

    #[test]
    fn test_get_neuron_color_warning() {
        let node = ModuleNode {
            name: "test".to_string(),
            total_reads: 0,
            total_writes: 0,
            read_rate: None,
            write_rate: None,
            health: HealthStatus::Warning,
            read_topics: vec![],
            write_topics: vec![],
        };
        let theme = Theme::Dark;

        let color = get_neuron_color(&node, None, &theme);
        assert_eq!(color, theme.neuron_warning());
    }

    #[test]
    fn test_get_neuron_color_critical() {
        let node = ModuleNode {
            name: "test".to_string(),
            total_reads: 0,
            total_writes: 0,
            read_rate: None,
            write_rate: None,
            health: HealthStatus::Critical,
            read_topics: vec![],
            write_topics: vec![],
        };
        let theme = Theme::Dark;

        let color = get_neuron_color(&node, None, &theme);
        assert_eq!(color, theme.neuron_critical());
    }

    #[test]
    fn test_get_neuron_color_with_high_fire_intensity() {
        let node = ModuleNode {
            name: "test".to_string(),
            total_reads: 0,
            total_writes: 0,
            read_rate: None,
            write_rate: None,
            health: HealthStatus::Healthy,
            read_topics: vec![],
            write_topics: vec![],
        };
        let theme = Theme::Dark;

        let activity = NodeActivity {
            last_count: 0,
            fire_intensity: 0.9, // High intensity
            last_update: std::time::Instant::now(),
        };

        let color = get_neuron_color(&node, Some(&activity), &theme);
        // With high fire intensity, should be closer to firing color
        // Just verify it's not the base color
        assert_ne!(color, theme.neuron_base());
    }
}
