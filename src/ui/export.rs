//! Screenshot and SVG export functionality.
//!
//! Provides export capabilities for saving graph visualizations
//! to vector graphics format.

use crate::graph::{HealthStatus, MessageFlowGraph};
use egui::{Pos2, Vec2};
use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::path::Path;

use super::theme::Theme;

/// Export the current graph to an SVG file.
///
/// Renders nodes as circles and edges as quadratic Bezier curves,
/// preserving the current theme colors and node positions.
///
/// # Arguments
///
/// * `path` - Output file path
/// * `graph` - The message flow graph to export
/// * `positions` - Current node positions
/// * `theme` - Color theme to use
/// * `width` - Default viewport width (used if no nodes exist)
/// * `height` - Default viewport height (used if no nodes exist)
pub fn export_to_svg(
    path: &Path,
    graph: &MessageFlowGraph,
    positions: &HashMap<String, Pos2>,
    theme: &Theme,
    width: f32,
    height: f32,
) -> std::io::Result<()> {
    let mut file = File::create(path)?;

    // Find bounds
    let (min_x, max_x, min_y, max_y) = if positions.is_empty() {
        (0.0, width, 0.0, height)
    } else {
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
        (min_x - 50.0, max_x + 50.0, min_y - 50.0, max_y + 50.0)
    };

    let svg_width = max_x - min_x;
    let svg_height = max_y - min_y;

    // SVG header
    writeln!(
        file,
        r#"<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" width="{}" height="{}" viewBox="{} {} {} {}">"#,
        svg_width, svg_height, min_x, min_y, svg_width, svg_height
    )?;

    // Background
    let bg = theme.background();
    writeln!(
        file,
        r#"  <rect x="{}" y="{}" width="{}" height="{}" fill="rgb({},{},{})"/>"#,
        min_x,
        min_y,
        svg_width,
        svg_height,
        bg.r(),
        bg.g(),
        bg.b()
    )?;

    // Draw edges
    for edge_idx in graph.graph.edge_indices() {
        if let Some((source, target)) = graph.graph.edge_endpoints(edge_idx) {
            let source_node = &graph.graph[source];
            let target_node = &graph.graph[target];
            let edge = &graph.graph[edge_idx];

            let pos_s = positions
                .get(&source_node.name)
                .copied()
                .unwrap_or(Pos2::ZERO);
            let pos_t = positions
                .get(&target_node.name)
                .copied()
                .unwrap_or(Pos2::ZERO);

            let edge_color = match edge.health {
                HealthStatus::Healthy => theme.synapse_base(),
                HealthStatus::Warning => theme.neuron_warning(),
                HealthStatus::Critical => theme.neuron_critical(),
            };

            // Bezier curve
            let mid = pos_s + (pos_t - pos_s) * 0.5;
            let dir = (pos_t - pos_s).normalized();
            let perp = Vec2::new(-dir.y, dir.x);
            let distance = (pos_t - pos_s).length();
            let curve_amount = (distance * 0.15).min(40.0);
            let control = mid + perp * curve_amount;

            writeln!(
                file,
                r#"  <path d="M {} {} Q {} {} {} {}" stroke="rgb({},{},{})" stroke-width="2" fill="none"/>"#,
                pos_s.x,
                pos_s.y,
                control.x,
                control.y,
                pos_t.x,
                pos_t.y,
                edge_color.r(),
                edge_color.g(),
                edge_color.b()
            )?;
        }
    }

    // Draw nodes
    for node in graph.graph.node_weights() {
        let pos = positions.get(&node.name).copied().unwrap_or(Pos2::ZERO);
        let radius = 12.0 + (node.throughput() as f32).log10().max(0.0) * 2.0;

        let color = match node.health {
            HealthStatus::Healthy => theme.neuron_base(),
            HealthStatus::Warning => theme.neuron_warning(),
            HealthStatus::Critical => theme.neuron_critical(),
        };

        // Node circle
        writeln!(
            file,
            r#"  <circle cx="{}" cy="{}" r="{}" fill="rgb({},{},{})"/>"#,
            pos.x,
            pos.y,
            radius,
            color.r(),
            color.g(),
            color.b()
        )?;

        // Node label
        let text_color = theme.text_primary();
        writeln!(
            file,
            r#"  <text x="{}" y="{}" text-anchor="middle" fill="rgb({},{},{})" font-size="10">{}</text>"#,
            pos.x,
            pos.y + radius + 12.0,
            text_color.r(),
            text_color.g(),
            text_color.b(),
            node.name
        )?;
    }

    writeln!(file, "</svg>")?;

    Ok(())
}

/// Show the export button and handle file dialog interaction.
///
/// Opens a native file save dialog when clicked, then exports
/// the graph to the selected location.
pub fn show_export_dialog(
    ui: &mut egui::Ui,
    graph: &MessageFlowGraph,
    positions: &HashMap<String, Pos2>,
    theme: &Theme,
) {
    ui.horizontal(|ui| {
        if ui.button("Export SVG").clicked() {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("SVG", &["svg"])
                .set_file_name("neuronic-graph.svg")
                .save_file()
            {
                if let Err(e) = export_to_svg(&path, graph, positions, theme, 1280.0, 800.0) {
                    tracing::error!("Failed to export SVG: {}", e);
                } else {
                    tracing::info!("Exported graph to {:?}", path);
                }
            }
        }
    });
}
