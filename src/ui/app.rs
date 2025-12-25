//! Main application window.

use crate::graph::{HealthStatus, MessageFlowGraph, ModuleNode, TopicEdge};
use crate::subscriber;
use buswatch_types::Snapshot;
use egui::{Color32, Pos2, Rect, Stroke, Vec2};
use egui_graphs::Graph;
use petgraph::stable_graph::StableGraph;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::mpsc as std_mpsc;

/// Main application state.
pub struct NeuronicApp {
    /// The message flow graph.
    flow_graph: MessageFlowGraph,
    /// egui_graphs graph for rendering (reserved for future use).
    #[allow(dead_code)]
    egui_graph: Graph<ModuleNode, TopicEdge>,
    /// Receiver for snapshots from the subscriber.
    snapshot_rx: Option<std_mpsc::Receiver<Snapshot>>,
    /// Tokio runtime for async subscriber.
    _runtime: tokio::runtime::Runtime,
    /// Whether we're connected to RabbitMQ.
    connected: bool,
    /// Connection error message if any.
    connection_error: Option<String>,
    /// Selected node for details panel.
    selected_node: Option<String>,
    /// Node positions for layout.
    node_positions: HashMap<String, Pos2>,
    /// Zoom level (reserved for future use).
    #[allow(dead_code)]
    zoom: f32,
    /// Pan offset (reserved for future use).
    #[allow(dead_code)]
    pan: Vec2,
    /// Whether to show topic labels.
    show_labels: bool,
    /// Paused state.
    paused: bool,
    /// Stats.
    update_count: u64,
}

impl NeuronicApp {
    /// Create a new application.
    pub fn new(cc: &eframe::CreationContext<'_>, config_path: PathBuf, topic: String) -> Self {
        // Set up dark theme
        cc.egui_ctx.set_visuals(egui::Visuals::dark());

        // Create tokio runtime
        let runtime = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");

        // Channel to bridge async subscriber to sync UI
        let (sync_tx, sync_rx) = std_mpsc::channel();

        // Spawn subscriber in background
        let config_path_clone = config_path.clone();
        let topic_clone = topic.clone();
        let (connected, connection_error) = {
            let sync_tx = sync_tx.clone();
            match runtime.block_on(async {
                subscriber::create_subscriber(&config_path_clone, &topic_clone).await
            }) {
                Ok((mut async_rx, _handle)) => {
                    // Bridge async to sync
                    runtime.spawn(async move {
                        while let Some(snapshot) = async_rx.recv().await {
                            if sync_tx.send(snapshot).is_err() {
                                break;
                            }
                        }
                    });
                    (true, None)
                }
                Err(e) => {
                    tracing::error!("Failed to connect to RabbitMQ: {}", e);
                    (false, Some(e.to_string()))
                }
            }
        };

        Self {
            flow_graph: MessageFlowGraph::new(),
            egui_graph: Graph::new(StableGraph::new()),
            snapshot_rx: Some(sync_rx),
            _runtime: runtime,
            connected,
            connection_error,
            selected_node: None,
            node_positions: HashMap::new(),
            zoom: 1.0,
            pan: Vec2::ZERO,
            show_labels: true,
            paused: false,
            update_count: 0,
        }
    }

    /// Process incoming snapshots.
    fn process_snapshots(&mut self) {
        if self.paused {
            // Drain but don't process
            if let Some(rx) = &self.snapshot_rx {
                while rx.try_recv().is_ok() {}
            }
            return;
        }

        if let Some(rx) = &self.snapshot_rx {
            // Process all available snapshots (use latest)
            let mut latest: Option<Snapshot> = None;
            while let Ok(snapshot) = rx.try_recv() {
                latest = Some(snapshot);
            }

            if let Some(snapshot) = latest {
                self.flow_graph.update_from_snapshot(&snapshot);
                self.update_count += 1;
            }
        }
    }

    /// Apply force-directed layout.
    fn apply_layout(&mut self, rect: Rect) {
        let center = rect.center();
        let node_count = self.flow_graph.module_count();

        if node_count == 0 {
            return;
        }

        // Initialize positions if needed
        for node in self.flow_graph.graph.node_weights() {
            if !self.node_positions.contains_key(&node.name) {
                // Place in a circle initially
                let idx = self.node_positions.len();
                let angle = (idx as f32 / node_count as f32) * std::f32::consts::TAU;
                let radius = rect.width().min(rect.height()) * 0.3;
                let pos = center + Vec2::new(angle.cos() * radius, angle.sin() * radius);
                self.node_positions.insert(node.name.clone(), pos);
            }
        }

        // Simple force-directed step
        let repulsion = 5000.0;
        let attraction = 0.01;
        let damping = 0.9;

        let mut forces: HashMap<String, Vec2> = HashMap::new();

        // Initialize forces
        for node in self.flow_graph.graph.node_weights() {
            forces.insert(node.name.clone(), Vec2::ZERO);
        }

        // Repulsion between all nodes
        let nodes: Vec<_> = self.flow_graph.graph.node_weights().collect();
        for i in 0..nodes.len() {
            for j in (i + 1)..nodes.len() {
                let pos_i = self
                    .node_positions
                    .get(&nodes[i].name)
                    .copied()
                    .unwrap_or(center);
                let pos_j = self
                    .node_positions
                    .get(&nodes[j].name)
                    .copied()
                    .unwrap_or(center);

                let delta = pos_i - pos_j;
                let dist = delta.length().max(1.0);
                let force = delta.normalized() * (repulsion / (dist * dist));

                *forces.get_mut(&nodes[i].name).unwrap() += force;
                *forces.get_mut(&nodes[j].name).unwrap() -= force;
            }
        }

        // Attraction along edges
        for edge_idx in self.flow_graph.graph.edge_indices() {
            if let Some((source, target)) = self.flow_graph.graph.edge_endpoints(edge_idx) {
                let source_node = &self.flow_graph.graph[source];
                let target_node = &self.flow_graph.graph[target];

                let pos_s = self
                    .node_positions
                    .get(&source_node.name)
                    .copied()
                    .unwrap_or(center);
                let pos_t = self
                    .node_positions
                    .get(&target_node.name)
                    .copied()
                    .unwrap_or(center);

                let delta = pos_t - pos_s;
                let force = delta * attraction;

                *forces.get_mut(&source_node.name).unwrap() += force;
                *forces.get_mut(&target_node.name).unwrap() -= force;
            }
        }

        // Center gravity
        for node in self.flow_graph.graph.node_weights() {
            let pos = self
                .node_positions
                .get(&node.name)
                .copied()
                .unwrap_or(center);
            let to_center = center - pos;
            *forces.get_mut(&node.name).unwrap() += to_center * 0.001;
        }

        // Apply forces
        for node in self.flow_graph.graph.node_weights() {
            if let Some(pos) = self.node_positions.get_mut(&node.name) {
                let force = forces.get(&node.name).copied().unwrap_or(Vec2::ZERO);
                *pos += force * damping;

                // Keep in bounds
                pos.x = pos.x.clamp(rect.left() + 50.0, rect.right() - 50.0);
                pos.y = pos.y.clamp(rect.top() + 50.0, rect.bottom() - 50.0);
            }
        }
    }

    /// Draw the graph.
    fn draw_graph(&mut self, ui: &mut egui::Ui, rect: Rect) {
        let painter = ui.painter_at(rect);

        // Apply layout
        self.apply_layout(rect);

        // Draw edges first (behind nodes)
        for edge_idx in self.flow_graph.graph.edge_indices() {
            if let Some((source, target)) = self.flow_graph.graph.edge_endpoints(edge_idx) {
                let source_node = &self.flow_graph.graph[source];
                let target_node = &self.flow_graph.graph[target];
                let edge = &self.flow_graph.graph[edge_idx];

                let pos_s = self
                    .node_positions
                    .get(&source_node.name)
                    .copied()
                    .unwrap_or(rect.center());
                let pos_t = self
                    .node_positions
                    .get(&target_node.name)
                    .copied()
                    .unwrap_or(rect.center());

                // Edge width based on rate/count
                let base_width = 1.0;
                let width = if let Some(rate) = edge.rate {
                    base_width + (rate.log10().max(0.0) as f32) * 0.5
                } else {
                    base_width + (edge.message_count as f32).log10().max(0.0) * 0.3
                };

                let color = edge.health.color().gamma_multiply(0.7);

                painter.line_segment([pos_s, pos_t], Stroke::new(width, color));

                // Draw arrow
                let dir = (pos_t - pos_s).normalized();
                let arrow_pos = pos_t - dir * 25.0;
                let arrow_size = 8.0;
                let perp = Vec2::new(-dir.y, dir.x);
                let arrow_points = vec![
                    arrow_pos + dir * arrow_size,
                    arrow_pos + perp * arrow_size * 0.5,
                    arrow_pos - perp * arrow_size * 0.5,
                ];
                painter.add(egui::Shape::convex_polygon(
                    arrow_points,
                    color,
                    Stroke::NONE,
                ));

                // Topic label
                if self.show_labels {
                    let mid = (pos_s + pos_t.to_vec2()) * 0.5;
                    painter.text(
                        Pos2::new(mid.x, mid.y),
                        egui::Align2::CENTER_CENTER,
                        &edge.topic,
                        egui::FontId::proportional(10.0),
                        Color32::GRAY,
                    );
                }
            }
        }

        // Draw nodes
        for node in self.flow_graph.graph.node_weights() {
            let pos = self
                .node_positions
                .get(&node.name)
                .copied()
                .unwrap_or(rect.center());

            // Node size based on throughput
            let base_radius = 20.0;
            let radius = base_radius + (node.throughput() as f32).log10().max(0.0) * 5.0;

            let color = node.health.color();
            let is_selected = self.selected_node.as_ref() == Some(&node.name);

            // Draw node circle
            painter.circle_filled(pos, radius, color);

            // Selection highlight
            if is_selected {
                painter.circle_stroke(pos, radius + 3.0, Stroke::new(2.0, Color32::WHITE));
            }

            // Node label
            painter.text(
                pos + Vec2::new(0.0, radius + 12.0),
                egui::Align2::CENTER_CENTER,
                &node.name,
                egui::FontId::proportional(12.0),
                Color32::WHITE,
            );
        }
    }

    /// Draw the control panel.
    fn draw_controls(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            // Connection status
            if self.connected {
                ui.label(egui::RichText::new("● Connected").color(Color32::GREEN));
            } else {
                ui.label(egui::RichText::new("● Disconnected").color(Color32::RED));
                if let Some(err) = &self.connection_error {
                    ui.label(egui::RichText::new(err).color(Color32::RED).small());
                }
            }

            ui.separator();

            // Pause button
            if ui
                .button(if self.paused {
                    "▶ Resume"
                } else {
                    "⏸ Pause"
                })
                .clicked()
            {
                self.paused = !self.paused;
            }

            ui.separator();

            // Show labels toggle
            ui.checkbox(&mut self.show_labels, "Labels");

            ui.separator();

            // Stats
            ui.label(format!(
                "Modules: {} | Connections: {} | Updates: {}",
                self.flow_graph.module_count(),
                self.flow_graph.edge_count(),
                self.update_count
            ));
        });
    }

    /// Draw the details panel.
    fn draw_details(&mut self, ui: &mut egui::Ui) {
        if let Some(name) = &self.selected_node {
            if let Some(node) = self
                .flow_graph
                .graph
                .node_weights()
                .find(|n| &n.name == name)
            {
                ui.heading(&node.name);
                ui.separator();

                ui.horizontal(|ui| {
                    ui.label("Health:");
                    let health_text = match node.health {
                        HealthStatus::Healthy => "Healthy",
                        HealthStatus::Warning => "Warning",
                        HealthStatus::Critical => "Critical",
                    };
                    ui.label(egui::RichText::new(health_text).color(node.health.color()));
                });

                ui.label(format!("Total reads: {}", node.total_reads));
                ui.label(format!("Total writes: {}", node.total_writes));

                if let Some(rate) = node.rate() {
                    ui.label(format!("Rate: {:.1} msg/s", rate));
                }

                ui.separator();
                ui.label("Read topics:");
                for topic in &node.read_topics {
                    ui.label(format!("  ← {}", topic));
                }

                ui.separator();
                ui.label("Write topics:");
                for topic in &node.write_topics {
                    ui.label(format!("  → {}", topic));
                }
            }
        } else {
            ui.label("Click a node to see details");
        }
    }
}

impl eframe::App for NeuronicApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Process incoming data
        self.process_snapshots();

        // Request continuous repaints for animation
        ctx.request_repaint();

        // Top panel with controls
        egui::TopBottomPanel::top("controls").show(ctx, |ui| {
            self.draw_controls(ui);
        });

        // Right panel with details
        egui::SidePanel::right("details")
            .min_width(200.0)
            .show(ctx, |ui| {
                self.draw_details(ui);
            });

        // Central panel with graph
        egui::CentralPanel::default().show(ctx, |ui| {
            let rect = ui.available_rect_before_wrap();

            // Handle clicks on nodes
            let response = ui.allocate_rect(rect, egui::Sense::click());
            if response.clicked() {
                if let Some(pos) = response.interact_pointer_pos() {
                    // Find clicked node
                    let mut clicked_node = None;
                    for node in self.flow_graph.graph.node_weights() {
                        if let Some(node_pos) = self.node_positions.get(&node.name) {
                            let radius = 20.0 + (node.throughput() as f32).log10().max(0.0) * 5.0;
                            if (*node_pos - pos).length() < radius {
                                clicked_node = Some(node.name.clone());
                                break;
                            }
                        }
                    }
                    self.selected_node = clicked_node;
                }
            }

            self.draw_graph(ui, rect);
        });
    }
}
