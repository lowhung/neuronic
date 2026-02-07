//! Minimap rendering for graph overview navigation.

use crate::graph::MessageFlowGraph;
use egui::{Color32, Pos2, Rect, Stroke, Vec2};
use std::collections::HashMap;

use crate::ui::theme::Theme;

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
