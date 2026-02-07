//! Edge rendering with gradient support.

use egui::{Color32, Pos2, Stroke};

use super::math::bezier_points;

/// Draw a gradient-colored Bezier edge.
pub fn draw_gradient_edge(
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
