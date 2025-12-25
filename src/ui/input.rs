//! Input handling for zoom, pan, drag, and click.

use crate::graph::MessageFlowGraph;
use egui::{Pos2, Rect, Vec2};
use std::collections::HashMap;

/// Result of input handling.
pub struct InputResult {
    pub clicked_node: Option<String>,
    pub started_dragging: Option<String>,
    pub stopped_dragging: bool,
}

/// Handle zoom and pan input.
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
                *pan = *pan + mouse_offset * (1.0 - new_zoom / *zoom);
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

    // Handle click for node selection
    if response.clicked() {
        if let Some(pos) = response.interact_pointer_pos() {
            result.clicked_node = find_node_at_position(graph, positions, pos, rect, *zoom, *pan);
        }
    }

    result
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

/// Handle keyboard shortcuts.
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
