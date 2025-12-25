//! Common types used across UI modules.

use egui::Color32;
use std::time::Instant;

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
    pub pattern: String,
    pub nodes: Vec<String>,
    pub collapsed: bool,
    pub color: Color32,
}

/// Snapshot history entry for replay.
#[derive(Clone)]
pub struct SnapshotEntry {
    pub snapshot: buswatch_types::Snapshot,
    pub timestamp: Instant,
}
