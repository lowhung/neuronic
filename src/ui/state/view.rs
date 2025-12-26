//! View state for camera and layout.

use egui::{Pos2, Vec2};
use std::collections::HashMap;

use crate::ui::types::LayoutMode;

/// State related to the graph view and camera.
pub struct ViewState {
    /// World positions of each node by name.
    pub node_positions: HashMap<String, Pos2>,
    /// Velocities for force-directed layout simulation.
    pub node_velocities: HashMap<String, Vec2>,
    /// Current layout algorithm mode.
    pub layout_mode: LayoutMode,
    /// Current zoom level (1.0 = 100%).
    pub zoom: f32,
    /// Current pan offset from center.
    pub pan: Vec2,
}

impl Default for ViewState {
    fn default() -> Self {
        Self {
            node_positions: HashMap::new(),
            node_velocities: HashMap::new(),
            layout_mode: LayoutMode::default(),
            zoom: 1.0,
            pan: Vec2::ZERO,
        }
    }
}
