//! Input handling for zoom, pan, drag, and click.
//!
//! Processes mouse and keyboard input for graph interaction:
//!
//! - Scroll wheel: Zoom in/out centered on cursor
//! - Left drag on node: Move the node
//! - Left drag on background: Pan the view
//! - Click on node: Select it
//! - Ctrl+F: Focus search box
//! - Escape: Clear search

use crate::graph::MessageFlowGraph;
use egui::{Pos2, Rect, Vec2};
use petgraph::graph::EdgeIndex;
use std::collections::HashMap;

/// Result of processing user input for a frame.
pub struct InputResult {
    pub clicked_node: Option<String>,
    pub clicked_edge: Option<EdgeIndex>,
    pub started_dragging: Option<String>,
    pub stopped_dragging: bool,
}

/// Handle mouse input for zoom, pan, drag, and node selection.
///
/// Processes scroll wheel for zooming, drag gestures for panning or
/// moving nodes, and clicks for node selection.
#[allow(clippy::too_many_arguments)]
pub fn handle_input(
    ui: &mut egui::Ui,
    rect: Rect,
    graph: &MessageFlowGraph,
    positions: &mut HashMap<String, Pos2>,
    velocities: &mut HashMap<String, Vec2>,
    zoom: &mut f32,
    pan: &mut Vec2,
    dragged_node: &mut Option<String>,
) -> InputResult {
    let response = ui.allocate_rect(rect, egui::Sense::click_and_drag());
    let mut result = InputResult {
        clicked_node: None,
        clicked_edge: None,
        started_dragging: None,
        stopped_dragging: false,
    };

    // Handle scroll for zoom
    let scroll_delta = ui.input(|i| i.raw_scroll_delta.y);
    if scroll_delta != 0.0 {
        let zoom_factor = if scroll_delta > 0.0 { 1.1 } else { 0.9 };
        let new_zoom = (*zoom * zoom_factor).clamp(0.1, 5.0);

        if let Some(mouse_pos) = ui.input(|i| i.pointer.hover_pos()) {
            if rect.contains(mouse_pos) {
                let center = rect.center();
                let mouse_offset = mouse_pos - center - *pan;
                *pan += mouse_offset * (1.0 - new_zoom / *zoom);
                *zoom = new_zoom;
            }
        }
    }

    // Handle drag start
    if response.drag_started_by(egui::PointerButton::Primary) {
        if let Some(pos) = response.interact_pointer_pos() {
            let found = find_node_at_position(graph, positions, pos, rect, *zoom, *pan);
            *dragged_node = found.clone();
            result.started_dragging = found;
        }
    }

    // Handle ongoing drag
    if response.dragged_by(egui::PointerButton::Primary) {
        let drag_delta = response.drag_delta();

        if let Some(ref node_name) = dragged_node {
            let world_delta = drag_delta / *zoom;
            if let Some(pos) = positions.get_mut(node_name) {
                *pos += world_delta;
            }
            if let Some(vel) = velocities.get_mut(node_name) {
                *vel = Vec2::ZERO;
            }
        } else {
            *pan += drag_delta;
        }
    }

    // Handle drag end
    if response.drag_stopped() {
        *dragged_node = None;
        result.stopped_dragging = true;
    }

    // Handle click for node/edge selection
    if response.clicked() {
        if let Some(pos) = response.interact_pointer_pos() {
            result.clicked_node = find_node_at_position(graph, positions, pos, rect, *zoom, *pan);
            // Only check for edge clicks if no node was clicked
            if result.clicked_node.is_none() {
                result.clicked_edge =
                    find_edge_at_position(graph, positions, pos, rect, *zoom, *pan);
            }
        }
    }

    result
}

/// Find an edge at the given screen position.
fn find_edge_at_position(
    graph: &MessageFlowGraph,
    positions: &HashMap<String, Pos2>,
    screen_pos: Pos2,
    rect: Rect,
    zoom: f32,
    pan: Vec2,
) -> Option<EdgeIndex> {
    let center = rect.center();
    let click_threshold = 10.0; // Distance in screen pixels to consider a click on edge

    for edge_idx in graph.graph.edge_indices() {
        if let Some((source_idx, target_idx)) = graph.graph.edge_endpoints(edge_idx) {
            let source_node = &graph.graph[source_idx];
            let target_node = &graph.graph[target_idx];

            let world_s = positions.get(&source_node.name)?;
            let world_t = positions.get(&target_node.name)?;

            // Convert to screen coordinates
            let offset_s = (*world_s - center) * zoom;
            let offset_t = (*world_t - center) * zoom;
            let pos_s = center + offset_s + pan;
            let pos_t = center + offset_t + pan;

            // Calculate bezier control point (same curve as in drawing.rs)
            let mid = pos_s + (pos_t - pos_s) * 0.5;
            let dir = (pos_t - pos_s).normalized();
            let perp = Vec2::new(-dir.y, dir.x);
            let distance = (pos_t - pos_s).length();
            let curve_amount = (distance * 0.15).min(40.0 * zoom);
            let control = mid + perp * curve_amount;

            // Check distance to bezier curve at multiple sample points
            for i in 0..=20 {
                let t = i as f32 / 20.0;
                let point = quadratic_bezier(pos_s, control, pos_t, t);
                if (point - screen_pos).length() < click_threshold {
                    return Some(edge_idx);
                }
            }
        }
    }
    None
}

/// Quadratic bezier interpolation for edge hit testing.
fn quadratic_bezier(p0: Pos2, p1: Pos2, p2: Pos2, t: f32) -> Pos2 {
    let u = 1.0 - t;
    Pos2::new(
        u * u * p0.x + 2.0 * u * t * p1.x + t * t * p2.x,
        u * u * p0.y + 2.0 * u * t * p1.y + t * t * p2.y,
    )
}

/// Find a node at the given screen position.
fn find_node_at_position(
    graph: &MessageFlowGraph,
    positions: &HashMap<String, Pos2>,
    screen_pos: Pos2,
    rect: Rect,
    zoom: f32,
    pan: Vec2,
) -> Option<String> {
    for node in graph.graph.node_weights() {
        if let Some(world_pos) = positions.get(&node.name) {
            let center = rect.center();
            let offset = (*world_pos - center) * zoom;
            let node_screen_pos = center + offset + pan;

            let radius = (12.0 + (node.throughput() as f32).log10().max(0.0) * 2.0) * zoom;
            if (node_screen_pos - screen_pos).length() < radius + 5.0 {
                return Some(node.name.clone());
            }
        }
    }
    None
}

/// Handle keyboard shortcuts for search and navigation.
///
/// - Ctrl+F: Focus the search box
/// - Escape: Clear search and unfocus
pub fn handle_keyboard(ui: &egui::Ui, search_focused: &mut bool) -> KeyboardAction {
    let mut action = KeyboardAction::None;

    ui.input(|i| {
        // Ctrl+F for search
        if i.modifiers.command && i.key_pressed(egui::Key::F) {
            *search_focused = true;
            action = KeyboardAction::FocusSearch;
        }

        // Escape to clear search
        if i.key_pressed(egui::Key::Escape) {
            *search_focused = false;
            action = KeyboardAction::ClearSearch;
        }
    });

    action
}

pub enum KeyboardAction {
    None,
    FocusSearch,
    ClearSearch,
}
