//! Graph layout algorithms.

use crate::graph::MessageFlowGraph;
use egui::{Pos2, Rect, Vec2};
use std::collections::HashMap;

use super::types::LayoutMode;

/// Apply force-directed layout to node positions.
pub fn apply_force_directed(
    graph: &MessageFlowGraph,
    positions: &mut HashMap<String, Pos2>,
    velocities: &mut HashMap<String, Vec2>,
    rect: Rect,
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

    let repulsion = 50000.0;
    let attraction = 0.002;
    let damping = 0.85;
    let min_distance = 120.0;

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

            let repulsion_force = if dist < min_distance {
                delta.normalized() * (repulsion / (dist * dist)) * 2.0
            } else {
                delta.normalized() * (repulsion / (dist * dist))
            };

            *forces.get_mut(&nodes[i].name).unwrap() += repulsion_force;
            *forces.get_mut(&nodes[j].name).unwrap() -= repulsion_force;
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

            if dist > min_distance * 1.5 {
                let force = delta.normalized() * (dist - min_distance) * attraction;
                *forces.get_mut(&source_node.name).unwrap() += force;
                *forces.get_mut(&target_node.name).unwrap() -= force;
            }
        }
    }

    // Gentle center gravity
    for node in graph.graph.node_weights() {
        let pos = positions.get(&node.name).copied().unwrap_or(center);
        let to_center = center - pos;
        *forces.get_mut(&node.name).unwrap() += to_center * 0.0005;
    }

    // Apply forces with velocity
    for node in graph.graph.node_weights() {
        let force = forces.get(&node.name).copied().unwrap_or(Vec2::ZERO);
        let vel = velocities.entry(node.name.clone()).or_insert(Vec2::ZERO);

        *vel = (*vel + force * 0.016) * damping;

        if let Some(pos) = positions.get_mut(&node.name) {
            *pos += *vel;
        }
    }
}

/// Apply hierarchical layout (sources at top, sinks at bottom).
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
    let usable_height = rect.height() - padding * 2.0;

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

/// Apply the appropriate layout based on mode.
pub fn apply_layout(
    mode: LayoutMode,
    graph: &MessageFlowGraph,
    positions: &mut HashMap<String, Pos2>,
    velocities: &mut HashMap<String, Vec2>,
    rect: Rect,
) {
    match mode {
        LayoutMode::ForceDirected => apply_force_directed(graph, positions, velocities, rect),
        LayoutMode::Hierarchical => apply_hierarchical(graph, positions, rect),
    }
}
