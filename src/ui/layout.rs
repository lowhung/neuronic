//! Graph layout algorithms.
//!
//! This module provides layout algorithms for positioning nodes in the graph:
//!
//! - [`apply_force_directed`] - Physics-based layout with repulsion and attraction forces
//! - [`apply_hierarchical`] - Layer-based layout with sources at top, sinks at bottom
//! - [`apply_layout`] - Dispatcher that applies the appropriate layout based on mode

use crate::graph::MessageFlowGraph;
use egui::{Pos2, Rect, Vec2};
use std::collections::HashMap;

use super::types::{LayoutMode, NodeGroup};

/// Configuration for force-directed layout physics simulation.
///
/// These parameters control the physics simulation used to position nodes in the graph.
/// Tuning these values affects how nodes organize themselves and how quickly the layout stabilizes.
#[derive(Clone, Debug)]
pub struct ForceLayoutConfig {
    /// Repulsion force strength between all nodes (prevents overlap).
    /// Higher values push nodes further apart.
    /// Default: 50,000.0
    pub repulsion: f32,

    /// Attraction force strength along edges (clusters connected modules).
    /// Higher values pull connected nodes closer together.
    /// Default: 0.002
    pub attraction: f32,

    /// Velocity damping factor (stabilizes the simulation).
    /// Values between 0 and 1. Lower values slow down movement more quickly.
    /// Default: 0.85
    pub damping: f32,

    /// Minimum desired distance between nodes.
    /// Nodes closer than this experience stronger repulsion.
    /// Default: 120.0
    pub min_distance: f32,

    /// Center gravity strength (keeps graph from drifting).
    /// Gentle force pulling all nodes toward the center.
    /// Default: 0.0005
    pub center_gravity: f32,

    /// Group attraction strength (clusters nodes in same group).
    /// Additional force for nodes that share a group.
    /// Default: 0.001
    pub group_attraction: f32,
}

impl Default for ForceLayoutConfig {
    fn default() -> Self {
        Self {
            repulsion: 50_000.0,
            attraction: 0.002,
            damping: 0.85,
            min_distance: 120.0,
            center_gravity: 0.0005,
            group_attraction: 0.001,
        }
    }
}

/// Apply force-directed layout to node positions.
///
/// Uses a physics simulation where:
/// - All nodes repel each other (prevents overlap)
/// - Connected nodes attract each other (clusters related modules)
/// - A gentle center gravity keeps the graph from drifting
///
/// Node positions are updated incrementally each frame, with velocity
/// damping to eventually reach a stable state.
pub fn apply_force_directed(
    config: &ForceLayoutConfig,
    graph: &MessageFlowGraph,
    positions: &mut HashMap<String, Pos2>,
    velocities: &mut HashMap<String, Vec2>,
    rect: Rect,
    groups: &[NodeGroup],
) {
    let center = rect.center();
    let node_count = graph.module_count();

    if node_count == 0 {
        return;
    }

    // Initialize positions if needed
    for node in graph.graph.node_weights() {
        if !positions.contains_key(&node.name) {
            let idx = positions.len();
            let angle = (idx as f32 / node_count as f32) * std::f32::consts::TAU;
            let radius = rect.width().min(rect.height()) * 0.4;
            let pos = center + Vec2::new(angle.cos() * radius, angle.sin() * radius);
            positions.insert(node.name.clone(), pos);
            velocities.insert(node.name.clone(), Vec2::ZERO);
        }
    }

    let mut forces: HashMap<String, Vec2> = HashMap::new();

    for node in graph.graph.node_weights() {
        forces.insert(node.name.clone(), Vec2::ZERO);
    }

    // Repulsion between all nodes
    let nodes: Vec<_> = graph.graph.node_weights().collect();
    for i in 0..nodes.len() {
        for j in (i + 1)..nodes.len() {
            let pos_i = positions.get(&nodes[i].name).copied().unwrap_or(center);
            let pos_j = positions.get(&nodes[j].name).copied().unwrap_or(center);

            let delta = pos_i - pos_j;
            let dist = delta.length().max(1.0);

            let repulsion_force = if dist < config.min_distance {
                delta.normalized() * (config.repulsion / (dist * dist)) * 2.0
            } else {
                delta.normalized() * (config.repulsion / (dist * dist))
            };

            if let Some(force) = forces.get_mut(&nodes[i].name) {
                *force += repulsion_force;
            } else {
                tracing::warn!("Layout: missing force vector for node '{}'", nodes[i].name);
            }
            if let Some(force) = forces.get_mut(&nodes[j].name) {
                *force -= repulsion_force;
            } else {
                tracing::warn!("Layout: missing force vector for node '{}'", nodes[j].name);
            }
        }
    }

    // Attraction along edges
    for edge_idx in graph.graph.edge_indices() {
        if let Some((source, target)) = graph.graph.edge_endpoints(edge_idx) {
            let source_node = &graph.graph[source];
            let target_node = &graph.graph[target];

            let pos_s = positions.get(&source_node.name).copied().unwrap_or(center);
            let pos_t = positions.get(&target_node.name).copied().unwrap_or(center);

            let delta = pos_t - pos_s;
            let dist = delta.length();

            if dist > config.min_distance * 1.5 {
                let force = delta.normalized() * (dist - config.min_distance) * config.attraction;
                if let Some(f) = forces.get_mut(&source_node.name) {
                    *f += force;
                } else {
                    tracing::warn!(
                        "Layout: missing force vector for source node '{}'",
                        source_node.name
                    );
                }
                if let Some(f) = forces.get_mut(&target_node.name) {
                    *f -= force;
                } else {
                    tracing::warn!(
                        "Layout: missing force vector for target node '{}'",
                        target_node.name
                    );
                }
            }
        }
    }

    // Group attraction - nodes in same group attract each other
    for group in groups {
        if group.collapsed {
            continue; // Don't apply forces for collapsed groups
        }
        for i in 0..group.nodes.len() {
            for j in (i + 1)..group.nodes.len() {
                let name_i = &group.nodes[i];
                let name_j = &group.nodes[j];

                if let (Some(&pos_i), Some(&pos_j)) = (positions.get(name_i), positions.get(name_j))
                {
                    let delta = pos_j - pos_i;
                    let dist = delta.length();

                    if dist > config.min_distance * 0.5 {
                        let force = delta.normalized()
                            * (dist - config.min_distance * 0.5)
                            * config.group_attraction;
                        if let Some(f) = forces.get_mut(name_i) {
                            *f += force;
                        }
                        if let Some(f) = forces.get_mut(name_j) {
                            *f -= force;
                        }
                    }
                }
            }
        }
    }

    // Gentle center gravity
    for node in graph.graph.node_weights() {
        let pos = positions.get(&node.name).copied().unwrap_or(center);
        let to_center = center - pos;
        if let Some(force) = forces.get_mut(&node.name) {
            *force += to_center * config.center_gravity;
        } else {
            tracing::warn!("Layout: missing force vector for node '{}'", node.name);
        }
    }

    // Apply forces with velocity
    for node in graph.graph.node_weights() {
        let force = forces.get(&node.name).copied().unwrap_or(Vec2::ZERO);
        let vel = velocities.entry(node.name.clone()).or_insert(Vec2::ZERO);

        *vel = (*vel + force * 0.016) * config.damping;

        if let Some(pos) = positions.get_mut(&node.name) {
            *pos += *vel;
        }
    }
}

/// Apply hierarchical layout with sources at top, sinks at bottom.
///
/// Classifies nodes into three tiers:
/// - **Sources** (top): Modules that only write messages
/// - **Middle**: Modules that both read and write messages
/// - **Sinks** (bottom): Modules that only read messages
///
/// This layout is useful for understanding data flow direction through
/// the system.
pub fn apply_hierarchical(
    graph: &MessageFlowGraph,
    positions: &mut HashMap<String, Pos2>,
    rect: Rect,
) {
    let node_count = graph.module_count();
    if node_count == 0 {
        return;
    }

    // Classify nodes by their role
    let mut sources: Vec<String> = Vec::new(); // Write-only nodes
    let mut sinks: Vec<String> = Vec::new(); // Read-only nodes
    let mut middle: Vec<String> = Vec::new(); // Both read and write

    for node in graph.graph.node_weights() {
        let has_reads = !node.read_topics.is_empty();
        let has_writes = !node.write_topics.is_empty();

        match (has_reads, has_writes) {
            (false, true) => sources.push(node.name.clone()),
            (true, false) => sinks.push(node.name.clone()),
            _ => middle.push(node.name.clone()),
        }
    }

    let padding = 80.0;
    let usable_width = rect.width() - padding * 2.0;
    let _usable_height = rect.height() - padding * 2.0;

    // Position sources at top
    let source_y = rect.top() + padding;
    for (i, name) in sources.iter().enumerate() {
        let x = if sources.len() == 1 {
            rect.center().x
        } else {
            rect.left() + padding + (i as f32 / (sources.len() - 1).max(1) as f32) * usable_width
        };
        positions.insert(name.clone(), Pos2::new(x, source_y));
    }

    // Position sinks at bottom
    let sink_y = rect.bottom() - padding;
    for (i, name) in sinks.iter().enumerate() {
        let x = if sinks.len() == 1 {
            rect.center().x
        } else {
            rect.left() + padding + (i as f32 / (sinks.len() - 1).max(1) as f32) * usable_width
        };
        positions.insert(name.clone(), Pos2::new(x, sink_y));
    }

    // Position middle nodes in between
    let middle_y = rect.center().y;
    for (i, name) in middle.iter().enumerate() {
        let x = if middle.len() == 1 {
            rect.center().x
        } else {
            rect.left() + padding + (i as f32 / (middle.len() - 1).max(1) as f32) * usable_width
        };
        positions.insert(name.clone(), Pos2::new(x, middle_y));
    }
}

/// Apply the appropriate layout algorithm based on the current mode.
///
/// This is the main entry point for layout updates, called each frame
/// to position nodes according to the selected [`LayoutMode`].
pub fn apply_layout(
    mode: LayoutMode,
    config: &ForceLayoutConfig,
    graph: &MessageFlowGraph,
    positions: &mut HashMap<String, Pos2>,
    velocities: &mut HashMap<String, Vec2>,
    rect: Rect,
    groups: &[NodeGroup],
) {
    match mode {
        LayoutMode::ForceDirected => {
            apply_force_directed(config, graph, positions, velocities, rect, groups)
        }
        LayoutMode::Hierarchical => apply_hierarchical(graph, positions, rect),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use buswatch_types::Snapshot;

    fn create_test_rect() -> Rect {
        Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 600.0))
    }

    #[test]
    fn test_force_directed_empty_graph() {
        let config = ForceLayoutConfig::default();
        let graph = MessageFlowGraph::new();
        let mut positions = HashMap::new();
        let mut velocities = HashMap::new();
        let rect = create_test_rect();

        apply_force_directed(&config, &graph, &mut positions, &mut velocities, rect, &[]);

        assert!(positions.is_empty());
        assert!(velocities.is_empty());
    }

    #[test]
    fn test_force_directed_initializes_positions() {
        let config = ForceLayoutConfig::default();
        let mut graph = MessageFlowGraph::new();
        let snapshot = Snapshot::builder()
            .module("node1", |m| m.write("topic", |w| w.count(10)))
            .module("node2", |m| m.read("topic", |r| r.count(5)))
            .build();
        graph.update_from_snapshot(&snapshot);

        let mut positions = HashMap::new();
        let mut velocities = HashMap::new();
        let rect = create_test_rect();

        apply_force_directed(&config, &graph, &mut positions, &mut velocities, rect, &[]);

        // Should have positions for both nodes
        assert_eq!(positions.len(), 2);
        assert!(positions.contains_key("node1"));
        assert!(positions.contains_key("node2"));

        // Positions should be within the rect
        for pos in positions.values() {
            // Allow some margin for initial placement
            assert!(pos.x >= -100.0 && pos.x <= rect.width() + 100.0);
            assert!(pos.y >= -100.0 && pos.y <= rect.height() + 100.0);
        }
    }

    #[test]
    fn test_force_directed_preserves_existing_positions() {
        let config = ForceLayoutConfig::default();
        let mut graph = MessageFlowGraph::new();
        let snapshot = Snapshot::builder()
            .module("node1", |m| m.write("topic", |w| w.count(10)))
            .build();
        graph.update_from_snapshot(&snapshot);

        let mut positions = HashMap::new();
        positions.insert("node1".to_string(), Pos2::new(100.0, 200.0));
        let mut velocities = HashMap::new();
        velocities.insert("node1".to_string(), Vec2::ZERO);
        let rect = create_test_rect();

        let original_pos = positions["node1"];

        apply_force_directed(&config, &graph, &mut positions, &mut velocities, rect, &[]);

        // Position should have changed due to center gravity, but not be reinitialized
        // (it should still exist and be near the original)
        assert!(positions.contains_key("node1"));
        let new_pos = positions["node1"];
        // With center gravity, it should move toward center but not teleport
        let distance =
            ((new_pos.x - original_pos.x).powi(2) + (new_pos.y - original_pos.y).powi(2)).sqrt();
        assert!(distance < 50.0, "Position changed too much");
    }

    #[test]
    fn test_force_directed_nodes_repel() {
        let config = ForceLayoutConfig::default();
        let mut graph = MessageFlowGraph::new();
        // Create two unconnected nodes
        let snapshot = Snapshot::builder()
            .module("node1", |m| m.write("topic_a", |w| w.count(10)))
            .module("node2", |m| m.write("topic_b", |w| w.count(10)))
            .build();
        graph.update_from_snapshot(&snapshot);

        let mut positions = HashMap::new();
        // Place nodes very close together
        positions.insert("node1".to_string(), Pos2::new(400.0, 300.0));
        positions.insert("node2".to_string(), Pos2::new(401.0, 300.0));
        let mut velocities = HashMap::new();
        velocities.insert("node1".to_string(), Vec2::ZERO);
        velocities.insert("node2".to_string(), Vec2::ZERO);
        let rect = create_test_rect();

        // Run several iterations
        for _ in 0..10 {
            apply_force_directed(&config, &graph, &mut positions, &mut velocities, rect, &[]);
        }

        // Nodes should have moved apart
        let pos1 = positions["node1"];
        let pos2 = positions["node2"];
        let distance = ((pos2.x - pos1.x).powi(2) + (pos2.y - pos1.y).powi(2)).sqrt();
        assert!(distance > 5.0, "Nodes should repel each other");
    }

    #[test]
    fn test_hierarchical_empty_graph() {
        let graph = MessageFlowGraph::new();
        let mut positions = HashMap::new();
        let rect = create_test_rect();

        apply_hierarchical(&graph, &mut positions, rect);

        assert!(positions.is_empty());
    }

    #[test]
    fn test_hierarchical_places_sources_at_top() {
        let mut graph = MessageFlowGraph::new();
        // Create a source-only node (writes only)
        let snapshot = Snapshot::builder()
            .module("source", |m| m.write("events", |w| w.count(100)))
            .module("sink", |m| m.read("events", |r| r.count(50)))
            .build();
        graph.update_from_snapshot(&snapshot);

        let mut positions = HashMap::new();
        let rect = create_test_rect();

        apply_hierarchical(&graph, &mut positions, rect);

        let source_pos = positions.get("source").unwrap();
        let sink_pos = positions.get("sink").unwrap();

        // Source should be at top (lower y value)
        assert!(
            source_pos.y < sink_pos.y,
            "Source should be above sink: source_y={}, sink_y={}",
            source_pos.y,
            sink_pos.y
        );
    }

    #[test]
    fn test_hierarchical_places_sinks_at_bottom() {
        let mut graph = MessageFlowGraph::new();
        let snapshot = Snapshot::builder()
            .module("source", |m| m.write("events", |w| w.count(100)))
            .module("sink", |m| m.read("events", |r| r.count(50)))
            .build();
        graph.update_from_snapshot(&snapshot);

        let mut positions = HashMap::new();
        let rect = create_test_rect();
        let padding = 80.0;

        apply_hierarchical(&graph, &mut positions, rect);

        let sink_pos = positions.get("sink").unwrap();

        // Sink should be near bottom
        assert!(
            (sink_pos.y - (rect.bottom() - padding)).abs() < 1.0,
            "Sink should be at bottom"
        );
    }

    #[test]
    fn test_hierarchical_places_middle_nodes_in_center() {
        let mut graph = MessageFlowGraph::new();
        // Middle node both reads and writes
        let snapshot = Snapshot::builder()
            .module("source", |m| m.write("input", |w| w.count(100)))
            .module("processor", |m| {
                m.read("input", |r| r.count(50))
                    .write("output", |w| w.count(50))
            })
            .module("sink", |m| m.read("output", |r| r.count(25)))
            .build();
        graph.update_from_snapshot(&snapshot);

        let mut positions = HashMap::new();
        let rect = create_test_rect();

        apply_hierarchical(&graph, &mut positions, rect);

        let source_pos = positions.get("source").unwrap();
        let processor_pos = positions.get("processor").unwrap();
        let sink_pos = positions.get("sink").unwrap();

        // Processor should be between source and sink
        assert!(
            processor_pos.y > source_pos.y,
            "Processor should be below source"
        );
        assert!(
            processor_pos.y < sink_pos.y,
            "Processor should be above sink"
        );
    }

    #[test]
    fn test_hierarchical_distributes_nodes_horizontally() {
        let mut graph = MessageFlowGraph::new();
        // Multiple sources
        let snapshot = Snapshot::builder()
            .module("source1", |m| m.write("a", |w| w.count(10)))
            .module("source2", |m| m.write("b", |w| w.count(10)))
            .module("source3", |m| m.write("c", |w| w.count(10)))
            .build();
        graph.update_from_snapshot(&snapshot);

        let mut positions = HashMap::new();
        let rect = create_test_rect();

        apply_hierarchical(&graph, &mut positions, rect);

        let pos1 = positions.get("source1").unwrap();
        let pos2 = positions.get("source2").unwrap();
        let pos3 = positions.get("source3").unwrap();

        // All should be at same y level
        assert!((pos1.y - pos2.y).abs() < 1.0);
        assert!((pos2.y - pos3.y).abs() < 1.0);

        // Should be spread horizontally (different x values)
        let mut x_values = [pos1.x, pos2.x, pos3.x];
        x_values.sort_by(|a, b| a.partial_cmp(b).unwrap());
        assert!(x_values[1] - x_values[0] > 10.0);
        assert!(x_values[2] - x_values[1] > 10.0);
    }

    #[test]
    fn test_apply_layout_dispatches_correctly() {
        let config = ForceLayoutConfig::default();
        let mut graph = MessageFlowGraph::new();
        let snapshot = Snapshot::builder()
            .module("node1", |m| m.write("topic", |w| w.count(10)))
            .build();
        graph.update_from_snapshot(&snapshot);

        let mut positions = HashMap::new();
        let mut velocities = HashMap::new();
        let rect = create_test_rect();

        // Test ForceDirected mode
        apply_layout(
            LayoutMode::ForceDirected,
            &config,
            &graph,
            &mut positions,
            &mut velocities,
            rect,
            &[],
        );
        assert!(positions.contains_key("node1"));
        assert!(velocities.contains_key("node1"));

        // Clear and test Hierarchical mode
        positions.clear();
        velocities.clear();
        apply_layout(
            LayoutMode::Hierarchical,
            &config,
            &graph,
            &mut positions,
            &mut velocities,
            rect,
            &[],
        );
        assert!(positions.contains_key("node1"));
    }
}
