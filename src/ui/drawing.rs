//! Graph rendering functions.

use crate::graph::{HealthStatus, MessageFlowGraph, ModuleNode};
use egui::{Color32, Pos2, Rect, Stroke, Vec2};
use std::collections::HashMap;

use super::theme::Theme;
use super::types::{NodeActivity, PulseRing, SynapseParticle};

/// Calculate a point on a quadratic Bezier curve.
pub fn quadratic_bezier(p0: Pos2, p1: Pos2, p2: Pos2, t: f32) -> Pos2 {
    let t = t.clamp(0.0, 1.0);
    let mt = 1.0 - t;
    Pos2::new(
        mt * mt * p0.x + 2.0 * mt * t * p1.x + t * t * p2.x,
        mt * mt * p0.y + 2.0 * mt * t * p1.y + t * t * p2.y,
    )
}

/// Generate points along a quadratic Bezier curve.
pub fn bezier_points(p0: Pos2, p1: Pos2, p2: Pos2, segments: usize) -> Vec<Pos2> {
    (0..=segments)
        .map(|i| {
            let t = i as f32 / segments as f32;
            quadratic_bezier(p0, p1, p2, t)
        })
        .collect()
}

/// Calculate tangent direction at a point on a quadratic Bezier curve.
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
pub fn lerp_color(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    Color32::from_rgba_unmultiplied(
        (a.r() as f32 + (b.r() as f32 - a.r() as f32) * t) as u8,
        (a.g() as f32 + (b.g() as f32 - a.g() as f32) * t) as u8,
        (a.b() as f32 + (b.b() as f32 - a.b() as f32) * t) as u8,
        (a.a() as f32 + (b.a() as f32 - a.a() as f32) * t) as u8,
    )
}

/// Format a rate value nicely.
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

/// Get neuron color based on health and activity.
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
    pub graph: &'a MessageFlowGraph,
    pub positions: &'a HashMap<String, Pos2>,
    pub activity: &'a HashMap<String, NodeActivity>,
    pub particles: &'a HashMap<(String, String, String), Vec<SynapseParticle>>,
    pub pulse_rings: &'a HashMap<String, Vec<PulseRing>>,
    pub selected_node: Option<&'a String>,
    pub highlighted_node: Option<&'a String>,
    pub theme: &'a Theme,
    pub zoom: f32,
    pub pan: Vec2,
    pub show_labels: bool,
    pub show_gradient_edges: bool,
    pub show_pulse_rings: bool,
}

impl<'a> DrawContext<'a> {
    /// Convert world position to screen position.
    pub fn world_to_screen(&self, world_pos: Pos2, rect: Rect) -> Pos2 {
        let center = rect.center();
        let offset = (world_pos - center) * self.zoom;
        center + offset + self.pan
    }

    /// Draw the complete graph.
    pub fn draw_graph(&self, ui: &mut egui::Ui, rect: Rect) {
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 0.0, self.theme.background());

        self.draw_edges(&painter, rect);
        self.draw_nodes(&painter, rect);
    }

    fn draw_edges(&self, painter: &egui::Painter, rect: Rect) {
        for edge_idx in self.graph.graph.edge_indices() {
            if let Some((source, target)) = self.graph.graph.edge_endpoints(edge_idx) {
                let source_node = &self.graph.graph[source];
                let target_node = &self.graph.graph[target];
                let edge = &self.graph.graph[edge_idx];

                let world_s = self
                    .positions
                    .get(&source_node.name)
                    .copied()
                    .unwrap_or(rect.center());
                let world_t = self
                    .positions
                    .get(&target_node.name)
                    .copied()
                    .unwrap_or(rect.center());

                let pos_s = self.world_to_screen(world_s, rect);
                let pos_t = self.world_to_screen(world_t, rect);

                let edge_color = match edge.health {
                    HealthStatus::Healthy => self.theme.synapse_base(),
                    HealthStatus::Warning => self.theme.neuron_warning().gamma_multiply(0.7),
                    HealthStatus::Critical => self.theme.neuron_critical().gamma_multiply(0.7),
                };

                let width =
                    (1.0 + (edge.rate.unwrap_or(0.0).log10().max(0.0) as f32) * 0.3) * self.zoom;

                // Calculate Bezier control point
                let mid = pos_s + (pos_t - pos_s) * 0.5;
                let dir = (pos_t - pos_s).normalized();
                let perp = Vec2::new(-dir.y, dir.x);
                let distance = (pos_t - pos_s).length();
                let curve_amount = (distance * 0.15).min(40.0 * self.zoom);
                let control = mid + perp * curve_amount;

                // Draw curved edge
                if self.show_gradient_edges {
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
                let node_radius = 12.0 * self.zoom;
                let arrow_t = 1.0 - (node_radius + 8.0 * self.zoom) / distance.max(1.0);
                let arrow_pos = quadratic_bezier(pos_s, control, pos_t, arrow_t.max(0.8));
                let arrow_dir = bezier_tangent(pos_s, control, pos_t, arrow_t.max(0.8));
                let arrow_perp = Vec2::new(-arrow_dir.y, arrow_dir.x);
                let arrow_size = 5.0 * self.zoom;
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
                if let Some(particles) = self.particles.get(&key) {
                    for particle in particles {
                        let particle_pos =
                            quadratic_bezier(pos_s, control, pos_t, particle.progress);
                        let particle_color = self.theme.synapse_active();
                        let glow_radius = 4.0 * self.zoom;
                        painter.circle_filled(
                            particle_pos,
                            glow_radius * 2.0,
                            particle_color.gamma_multiply(0.3),
                        );
                        painter.circle_filled(particle_pos, glow_radius, particle_color);
                    }
                }

                // Topic label
                if self.show_labels && self.zoom > 0.5 {
                    let label_pos = quadratic_bezier(pos_s, control, pos_t, 0.5);
                    painter.text(
                        label_pos,
                        egui::Align2::CENTER_CENTER,
                        &edge.topic,
                        egui::FontId::proportional(9.0 * self.zoom),
                        self.theme.text_secondary(),
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
        for node in self.graph.graph.node_weights() {
            let world_pos = self
                .positions
                .get(&node.name)
                .copied()
                .unwrap_or(rect.center());

            let pos = self.world_to_screen(world_pos, rect);

            let base_radius = 12.0 * self.zoom;
            let radius =
                base_radius + (node.throughput() as f32).log10().max(0.0) * 2.0 * self.zoom;

            let activity = self.activity.get(&node.name);
            let color = get_neuron_color(node, activity, self.theme);
            let is_selected = self.selected_node == Some(&node.name);
            let is_highlighted = self.highlighted_node == Some(&node.name);

            let fire_intensity = activity.map(|a| a.fire_intensity).unwrap_or(0.0);

            // Draw pulse rings
            if self.show_pulse_rings {
                if let Some(rings) = self.pulse_rings.get(&node.name) {
                    for ring in rings {
                        let ring_color = self
                            .theme
                            .neuron_active()
                            .gamma_multiply(ring.opacity * 0.5);
                        painter.circle_stroke(
                            pos,
                            ring.radius * self.zoom,
                            Stroke::new(2.0 * self.zoom, ring_color),
                        );
                    }
                }
            }

            // Glow effect for active neurons
            if fire_intensity > 0.1 {
                let glow_radius = radius + (8.0 + fire_intensity * 10.0) * self.zoom;
                let glow_color = self
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

            // Selection/highlight ring
            if is_selected || is_highlighted {
                let ring_color = if is_highlighted {
                    Color32::YELLOW
                } else {
                    Color32::WHITE
                };
                painter.circle_stroke(pos, radius + 4.0 * self.zoom, Stroke::new(2.0, ring_color));
            }

            // Node label
            if self.zoom > 0.4 {
                painter.text(
                    pos + Vec2::new(0.0, radius + 10.0 * self.zoom),
                    egui::Align2::CENTER_CENTER,
                    &node.name,
                    egui::FontId::proportional(10.0 * self.zoom),
                    self.theme.text_primary(),
                );

                if let Some(rate) = node.rate() {
                    let rate_text = format_rate(rate);
                    painter.text(
                        pos + Vec2::new(0.0, radius + 22.0 * self.zoom),
                        egui::Align2::CENTER_CENTER,
                        &rate_text,
                        egui::FontId::proportional(8.0 * self.zoom),
                        self.theme.text_secondary(),
                    );
                }
            }
        }
    }
}

/// Draw a minimap in the corner.
pub fn draw_minimap(
    ui: &mut egui::Ui,
    graph: &MessageFlowGraph,
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
    painter.rect_stroke(minimap_rect, 4.0, Stroke::new(1.0, theme.text_secondary()));

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
    for (name, pos) in positions {
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
    );
}
