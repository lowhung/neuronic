//! Common types used across UI modules.

use crate::graph::MessageFlowGraph;
use egui::{Color32, Pos2, Vec2};
use std::collections::HashMap;
use std::time::Instant;

use super::theme::Theme;

/// Layout mode for the graph.
#[derive(Clone, Copy, PartialEq, Default)]
pub enum LayoutMode {
    #[default]
    ForceDirected,
    Hierarchical,
}

/// Activity state for a node (for firing animation).
#[derive(Clone)]
pub struct NodeActivity {
    pub last_count: u64,
    pub fire_intensity: f32,
    pub last_update: Instant,
}

impl Default for NodeActivity {
    fn default() -> Self {
        Self {
            last_count: 0,
            fire_intensity: 0.0,
            last_update: Instant::now(),
        }
    }
}

/// Particle traveling along an edge (synapse firing).
#[derive(Clone)]
pub struct SynapseParticle {
    pub progress: f32,
    pub speed: f32,
}

/// Pulse ring for heavy firing animation.
#[derive(Clone)]
pub struct PulseRing {
    pub radius: f32,
    pub max_radius: f32,
    pub opacity: f32,
}

/// Node group for clustering.
#[derive(Clone)]
pub struct NodeGroup {
    pub name: String,
    pub nodes: Vec<String>,
    pub collapsed: bool,
}

/// Predefined color palette for node groups.
pub const GROUP_COLORS: [Color32; 8] = [
    Color32::from_rgb(70, 130, 180),  // Steel blue
    Color32::from_rgb(144, 238, 144), // Light green
    Color32::from_rgb(255, 182, 193), // Light pink
    Color32::from_rgb(255, 218, 185), // Peach
    Color32::from_rgb(221, 160, 221), // Plum
    Color32::from_rgb(176, 224, 230), // Powder blue
    Color32::from_rgb(255, 255, 150), // Light yellow
    Color32::from_rgb(230, 230, 250), // Lavender
];

/// Get a color for a group by index.
pub fn get_group_color(index: usize) -> Color32 {
    GROUP_COLORS[index % GROUP_COLORS.len()]
}

/// Stable identifier for a selected edge (survives graph rebuilds).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SelectedEdge {
    pub source_node: String,
    pub target_node: String,
    pub topic: String,
}

/// Grouped graph data references for rendering.
pub struct DataRefs<'a> {
    pub graph: &'a MessageFlowGraph,
    pub positions: &'a HashMap<String, Pos2>,
}

/// Grouped animation/visual effect references for rendering.
pub struct VisualRefs<'a> {
    pub activity: &'a HashMap<String, NodeActivity>,
    pub particles: &'a HashMap<(String, String, String), Vec<SynapseParticle>>,
    pub pulse_rings: &'a HashMap<String, Vec<PulseRing>>,
    pub node_groups: &'a [NodeGroup],
}

/// Grouped selection state references for rendering.
pub struct SelectionRefs<'a> {
    pub node: Option<&'a String>,
    pub edge: Option<&'a SelectedEdge>,
    pub highlighted: Option<&'a String>,
}

/// Style configuration for rendering (all Copy/Clone types, owned).
#[derive(Clone)]
pub struct StyleConfig {
    pub theme: Theme,
    pub show_labels: bool,
    pub show_gradient_edges: bool,
    pub show_pulse_rings: bool,
}

/// Camera transform for world-to-screen conversion.
#[derive(Clone, Copy)]
pub struct Transform {
    pub zoom: f32,
    pub pan: Vec2,
}
