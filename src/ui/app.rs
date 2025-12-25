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
use std::time::Instant;

// Neural color palette
const NEURON_BASE: Color32 = Color32::from_rgb(60, 80, 120); // Deep blue-gray
const NEURON_ACTIVE: Color32 = Color32::from_rgb(100, 180, 255); // Bright cyan-blue
const NEURON_FIRING: Color32 = Color32::from_rgb(180, 220, 255); // White-blue flash
const SYNAPSE_BASE: Color32 = Color32::from_rgb(80, 60, 120); // Purple-gray
const SYNAPSE_ACTIVE: Color32 = Color32::from_rgb(200, 100, 255); // Magenta
const NEURON_WARNING: Color32 = Color32::from_rgb(255, 180, 80); // Orange
const NEURON_CRITICAL: Color32 = Color32::from_rgb(255, 80, 100); // Red-pink

/// Activity state for a node (for firing animation).
#[derive(Clone)]
struct NodeActivity {
    last_count: u64,
    fire_intensity: f32, // 0.0 to 1.0, decays over time
    last_update: Instant,
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
struct SynapseParticle {
    progress: f32, // 0.0 to 1.0 along the edge
    speed: f32,
}

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
    /// Node positions for layout (in world coordinates).
    node_positions: HashMap<String, Pos2>,
    /// Node velocities for smoother physics.
    node_velocities: HashMap<String, Vec2>,
    /// Node activity state for firing animation.
    node_activity: HashMap<String, NodeActivity>,
    /// Synapse particles for edge animations.
    synapse_particles: HashMap<(String, String, String), Vec<SynapseParticle>>,
    /// Zoom level.
    zoom: f32,
    /// Pan offset (in screen coordinates).
    pan: Vec2,
    /// Whether to show topic labels.
    show_labels: bool,
    /// Whether to show legend.
    show_legend: bool,
    /// Paused state.
    paused: bool,
    /// Stats.
    update_count: u64,
    /// Last frame time for animation.
    last_frame: Instant,
}

impl NeuronicApp {
    /// Create a new application.
    pub fn new(cc: &eframe::CreationContext<'_>, config_path: PathBuf, topic: String) -> Self {
        // Set up dark theme with darker background
        let mut visuals = egui::Visuals::dark();
        visuals.panel_fill = Color32::from_rgb(15, 15, 25);
        visuals.window_fill = Color32::from_rgb(20, 20, 35);
        cc.egui_ctx.set_visuals(visuals);

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
            node_velocities: HashMap::new(),
            node_activity: HashMap::new(),
            synapse_particles: HashMap::new(),
            zoom: 1.0,
            pan: Vec2::ZERO,
            show_labels: true,
            show_legend: true,
            paused: false,
            update_count: 0,
            last_frame: Instant::now(),
        }
    }

    /// Convert world position to screen position.
    fn world_to_screen(&self, world_pos: Pos2, rect: Rect) -> Pos2 {
        let center = rect.center();
        let offset = (world_pos - center) * self.zoom;
        center + offset + self.pan
    }

    /// Convert screen position to world position.
    #[allow(dead_code)]
    fn screen_to_world(&self, screen_pos: Pos2, rect: Rect) -> Pos2 {
        let center = rect.center();
        let offset = screen_pos - center - self.pan;
        center + offset / self.zoom
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
                // Detect activity changes before updating
                self.detect_activity(&snapshot);
                self.flow_graph.update_from_snapshot(&snapshot);
                self.update_count += 1;
            }
        }
    }

    /// Detect activity changes and trigger firing animations.
    fn detect_activity(&mut self, snapshot: &Snapshot) {
        let now = Instant::now();

        for (module_name, metrics) in snapshot.iter() {
            let total = metrics.total_reads() + metrics.total_writes();
            let activity = self.node_activity.entry(module_name.clone()).or_default();

            if total > activity.last_count {
                // Activity detected - fire the neuron!
                let delta = total - activity.last_count;
                // Intensity based on message rate
                activity.fire_intensity =
                    (activity.fire_intensity + (delta as f32 / 100.0).min(1.0)).min(1.0);
                activity.last_update = now;

                // Spawn particles on outgoing edges
                for topic in metrics.writes.keys() {
                    // Find consumers of this topic
                    for (other_name, other_metrics) in snapshot.iter() {
                        if other_metrics.reads.contains_key(topic) {
                            let key = (module_name.clone(), other_name.clone(), topic.clone());
                            let particles = self.synapse_particles.entry(key).or_default();
                            // Add new particles based on activity
                            let new_particles = (delta as usize / 50).max(1).min(5);
                            for i in 0..new_particles {
                                particles.push(SynapseParticle {
                                    progress: i as f32 * 0.1,
                                    speed: 0.8 + (i as f32 * 0.1),
                                });
                            }
                        }
                    }
                }
            }

            activity.last_count = total;
        }
    }

    /// Update animations.
    fn update_animations(&mut self, dt: f32) {
        // Decay node fire intensity
        for activity in self.node_activity.values_mut() {
            activity.fire_intensity = (activity.fire_intensity - dt * 2.0).max(0.0);
        }

        // Update synapse particles
        for particles in self.synapse_particles.values_mut() {
            for particle in particles.iter_mut() {
                particle.progress += particle.speed * dt;
            }
            // Remove completed particles
            particles.retain(|p| p.progress < 1.0);
        }

        // Clean up empty particle lists
        self.synapse_particles.retain(|_, v| !v.is_empty());
    }

    /// Apply force-directed layout with better spacing.
    fn apply_layout(&mut self, rect: Rect) {
        let center = rect.center();
        let node_count = self.flow_graph.module_count();

        if node_count == 0 {
            return;
        }

        // Initialize positions if needed - spread out more
        for node in self.flow_graph.graph.node_weights() {
            if !self.node_positions.contains_key(&node.name) {
                let idx = self.node_positions.len();
                let angle = (idx as f32 / node_count as f32) * std::f32::consts::TAU;
                // Much larger initial radius
                let radius = rect.width().min(rect.height()) * 0.4;
                let pos = center + Vec2::new(angle.cos() * radius, angle.sin() * radius);
                self.node_positions.insert(node.name.clone(), pos);
                self.node_velocities.insert(node.name.clone(), Vec2::ZERO);
            }
        }

        // Force-directed layout with much stronger repulsion
        let repulsion = 50000.0; // Much stronger repulsion
        let attraction = 0.002; // Weaker attraction
        let damping = 0.85;
        let min_distance = 120.0; // Minimum distance between nodes

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

                // Strong repulsion, especially at close range
                let repulsion_force = if dist < min_distance {
                    delta.normalized() * (repulsion / (dist * dist)) * 2.0
                } else {
                    delta.normalized() * (repulsion / (dist * dist))
                };

                *forces.get_mut(&nodes[i].name).unwrap() += repulsion_force;
                *forces.get_mut(&nodes[j].name).unwrap() -= repulsion_force;
            }
        }

        // Attraction along edges (weaker)
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
                let dist = delta.length();

                // Only attract if beyond ideal distance
                if dist > min_distance * 1.5 {
                    let force = delta.normalized() * (dist - min_distance) * attraction;
                    *forces.get_mut(&source_node.name).unwrap() += force;
                    *forces.get_mut(&target_node.name).unwrap() -= force;
                }
            }
        }

        // Gentle center gravity
        for node in self.flow_graph.graph.node_weights() {
            let pos = self
                .node_positions
                .get(&node.name)
                .copied()
                .unwrap_or(center);
            let to_center = center - pos;
            *forces.get_mut(&node.name).unwrap() += to_center * 0.0005;
        }

        // Apply forces with velocity
        for node in self.flow_graph.graph.node_weights() {
            let force = forces.get(&node.name).copied().unwrap_or(Vec2::ZERO);
            let vel = self
                .node_velocities
                .entry(node.name.clone())
                .or_insert(Vec2::ZERO);

            *vel = (*vel + force * 0.016) * damping; // Assume ~60fps

            if let Some(pos) = self.node_positions.get_mut(&node.name) {
                *pos += *vel;
            }
        }
    }

    /// Get neuron color based on health and activity.
    fn get_neuron_color(&self, node: &ModuleNode) -> Color32 {
        let activity = self.node_activity.get(&node.name);
        let fire_intensity = activity.map(|a| a.fire_intensity).unwrap_or(0.0);

        // Base color from health
        let base = match node.health {
            HealthStatus::Healthy => NEURON_BASE,
            HealthStatus::Warning => NEURON_WARNING,
            HealthStatus::Critical => NEURON_CRITICAL,
        };

        // Blend with active/firing colors based on intensity
        if fire_intensity > 0.5 {
            lerp_color(NEURON_ACTIVE, NEURON_FIRING, (fire_intensity - 0.5) * 2.0)
        } else if fire_intensity > 0.0 {
            lerp_color(base, NEURON_ACTIVE, fire_intensity * 2.0)
        } else {
            base
        }
    }

    /// Draw the graph.
    fn draw_graph(&mut self, ui: &mut egui::Ui, rect: Rect) {
        let painter = ui.painter_at(rect);

        // Apply layout (in world coordinates)
        self.apply_layout(rect);

        // Draw edges (synapses) first
        for edge_idx in self.flow_graph.graph.edge_indices() {
            if let Some((source, target)) = self.flow_graph.graph.edge_endpoints(edge_idx) {
                let source_node = &self.flow_graph.graph[source];
                let target_node = &self.flow_graph.graph[target];
                let edge = &self.flow_graph.graph[edge_idx];

                let world_s = self
                    .node_positions
                    .get(&source_node.name)
                    .copied()
                    .unwrap_or(rect.center());
                let world_t = self
                    .node_positions
                    .get(&target_node.name)
                    .copied()
                    .unwrap_or(rect.center());

                // Convert to screen coordinates
                let pos_s = self.world_to_screen(world_s, rect);
                let pos_t = self.world_to_screen(world_t, rect);

                // Edge color based on health
                let edge_color = match edge.health {
                    HealthStatus::Healthy => SYNAPSE_BASE,
                    HealthStatus::Warning => NEURON_WARNING.gamma_multiply(0.7),
                    HealthStatus::Critical => NEURON_CRITICAL.gamma_multiply(0.7),
                };

                // Thinner lines, scaled by zoom
                let width =
                    (1.0 + (edge.rate.unwrap_or(0.0).log10().max(0.0) as f32) * 0.3) * self.zoom;

                painter.line_segment([pos_s, pos_t], Stroke::new(width, edge_color));

                // Draw arrow (smaller), scaled by zoom
                let dir = (pos_t - pos_s).normalized();
                let node_radius = 12.0 * self.zoom;
                let arrow_pos = pos_t - dir * (node_radius + 8.0 * self.zoom);
                let arrow_size = 5.0 * self.zoom;
                let perp = Vec2::new(-dir.y, dir.x);
                let arrow_points = vec![
                    arrow_pos + dir * arrow_size,
                    arrow_pos + perp * arrow_size * 0.4,
                    arrow_pos - perp * arrow_size * 0.4,
                ];
                painter.add(egui::Shape::convex_polygon(
                    arrow_points,
                    edge_color,
                    Stroke::NONE,
                ));

                // Draw synapse particles (firing animation)
                let key = (
                    source_node.name.clone(),
                    target_node.name.clone(),
                    edge.topic.clone(),
                );
                if let Some(particles) = self.synapse_particles.get(&key) {
                    for particle in particles {
                        let particle_pos = pos_s + (pos_t - pos_s) * particle.progress;
                        let particle_color = SYNAPSE_ACTIVE;
                        let glow_radius = 4.0 * self.zoom;
                        // Glow effect
                        painter.circle_filled(
                            particle_pos,
                            glow_radius * 2.0,
                            particle_color.gamma_multiply(0.3),
                        );
                        painter.circle_filled(particle_pos, glow_radius, particle_color);
                    }
                }

                // Topic label (smaller, more transparent)
                if self.show_labels && self.zoom > 0.5 {
                    let mid = pos_s + (pos_t - pos_s) * 0.5;
                    painter.text(
                        mid,
                        egui::Align2::CENTER_CENTER,
                        &edge.topic,
                        egui::FontId::proportional(9.0 * self.zoom),
                        Color32::from_rgba_unmultiplied(150, 150, 180, 180),
                    );
                }
            }
        }

        // Draw nodes (neurons)
        for node in self.flow_graph.graph.node_weights() {
            let world_pos = self
                .node_positions
                .get(&node.name)
                .copied()
                .unwrap_or(rect.center());

            let pos = self.world_to_screen(world_pos, rect);

            // Smaller base radius, scaled by zoom
            let base_radius = 12.0 * self.zoom;
            let radius =
                base_radius + (node.throughput() as f32).log10().max(0.0) * 2.0 * self.zoom;

            let color = self.get_neuron_color(node);
            let is_selected = self.selected_node.as_ref() == Some(&node.name);

            // Glow effect for active neurons
            let activity = self.node_activity.get(&node.name);
            let fire_intensity = activity.map(|a| a.fire_intensity).unwrap_or(0.0);

            if fire_intensity > 0.1 {
                // Outer glow
                let glow_radius = radius + (8.0 + fire_intensity * 10.0) * self.zoom;
                let glow_color = NEURON_ACTIVE.gamma_multiply(fire_intensity * 0.4);
                painter.circle_filled(pos, glow_radius, glow_color);
            }

            // Main neuron body
            painter.circle_filled(pos, radius, color);

            // Inner highlight (nucleus)
            let nucleus_color = lerp_color(color, Color32::WHITE, 0.3);
            painter.circle_filled(pos, radius * 0.4, nucleus_color);

            // Selection highlight
            if is_selected {
                painter.circle_stroke(
                    pos,
                    radius + 4.0 * self.zoom,
                    Stroke::new(2.0, Color32::WHITE),
                );
            }

            // Node label (smaller), only show if zoomed in enough
            if self.zoom > 0.4 {
                painter.text(
                    pos + Vec2::new(0.0, radius + 10.0 * self.zoom),
                    egui::Align2::CENTER_CENTER,
                    &node.name,
                    egui::FontId::proportional(10.0 * self.zoom),
                    Color32::from_rgb(200, 200, 220),
                );
            }
        }
    }

    /// Draw the legend panel.
    fn draw_legend(&self, ui: &mut egui::Ui) {
        ui.heading("Legend");
        ui.separator();

        ui.label("Node Status:");
        ui.horizontal(|ui| {
            let (rect, _) = ui.allocate_exact_size(Vec2::new(12.0, 12.0), egui::Sense::hover());
            ui.painter().circle_filled(rect.center(), 6.0, NEURON_BASE);
            ui.label("Healthy");
        });
        ui.horizontal(|ui| {
            let (rect, _) = ui.allocate_exact_size(Vec2::new(12.0, 12.0), egui::Sense::hover());
            ui.painter()
                .circle_filled(rect.center(), 6.0, NEURON_WARNING);
            ui.label("Warning");
        });
        ui.horizontal(|ui| {
            let (rect, _) = ui.allocate_exact_size(Vec2::new(12.0, 12.0), egui::Sense::hover());
            ui.painter()
                .circle_filled(rect.center(), 6.0, NEURON_CRITICAL);
            ui.label("Critical");
        });
        ui.horizontal(|ui| {
            let (rect, _) = ui.allocate_exact_size(Vec2::new(12.0, 12.0), egui::Sense::hover());
            ui.painter()
                .circle_filled(rect.center(), 6.0, NEURON_ACTIVE);
            ui.label("Active (firing)");
        });

        ui.add_space(8.0);
        ui.label("Node Size:");
        ui.label("  Larger = higher throughput");

        ui.add_space(8.0);
        ui.label("Edge Width:");
        ui.label("  Thicker = higher msg rate");

        ui.add_space(8.0);
        ui.label("Particles:");
        ui.horizontal(|ui| {
            let (rect, _) = ui.allocate_exact_size(Vec2::new(12.0, 12.0), egui::Sense::hover());
            ui.painter()
                .circle_filled(rect.center(), 4.0, SYNAPSE_ACTIVE);
            ui.label("Message flow");
        });

        ui.add_space(8.0);
        ui.separator();
        ui.label("Controls:");
        ui.label("  Scroll: Zoom");
        ui.label("  Drag: Pan");
        ui.label("  Click: Select node");
    }

    /// Draw the control panel.
    fn draw_controls(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            // Connection status
            if self.connected {
                ui.label(egui::RichText::new("● Connected").color(NEURON_ACTIVE));
            } else {
                ui.label(egui::RichText::new("● Disconnected").color(NEURON_CRITICAL));
                if let Some(err) = &self.connection_error {
                    ui.label(egui::RichText::new(err).color(NEURON_CRITICAL).small());
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

            // Show legend toggle
            ui.checkbox(&mut self.show_legend, "Legend");

            ui.separator();

            // Zoom controls
            if ui.button("−").clicked() {
                self.zoom = (self.zoom * 0.8).max(0.1);
            }
            ui.label(format!("{:.0}%", self.zoom * 100.0));
            if ui.button("+").clicked() {
                self.zoom = (self.zoom * 1.25).min(5.0);
            }
            if ui.button("Reset").clicked() {
                self.zoom = 1.0;
                self.pan = Vec2::ZERO;
            }

            ui.separator();

            // Stats
            ui.label(format!(
                "Neurons: {} | Synapses: {} | Updates: {}",
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
                    let (health_text, health_color) = match node.health {
                        HealthStatus::Healthy => ("Healthy", NEURON_ACTIVE),
                        HealthStatus::Warning => ("Warning", NEURON_WARNING),
                        HealthStatus::Critical => ("Critical", NEURON_CRITICAL),
                    };
                    ui.label(egui::RichText::new(health_text).color(health_color));
                });

                if let Some(activity) = self.node_activity.get(&node.name) {
                    ui.label(format!("Activity: {:.0}%", activity.fire_intensity * 100.0));
                }

                ui.label(format!("Total reads: {}", node.total_reads));
                ui.label(format!("Total writes: {}", node.total_writes));

                if let Some(rate) = node.rate() {
                    ui.label(format!("Rate: {:.1} msg/s", rate));
                }

                ui.separator();
                ui.label("Inputs:");
                for topic in &node.read_topics {
                    ui.label(format!("  ← {}", topic));
                }

                ui.separator();
                ui.label("Outputs:");
                for topic in &node.write_topics {
                    ui.label(format!("  → {}", topic));
                }
            }
        } else {
            ui.label("Click a neuron to see details");
        }
    }

    /// Handle zoom and pan input.
    fn handle_input(&mut self, ui: &mut egui::Ui, rect: Rect) -> egui::Response {
        let response = ui.allocate_rect(rect, egui::Sense::click_and_drag());

        // Handle scroll for zoom
        let scroll_delta = ui.input(|i| i.raw_scroll_delta.y);
        if scroll_delta != 0.0 {
            let zoom_factor = if scroll_delta > 0.0 { 1.1 } else { 0.9 };
            let new_zoom = (self.zoom * zoom_factor).clamp(0.1, 5.0);

            // Zoom towards mouse position
            if let Some(mouse_pos) = ui.input(|i| i.pointer.hover_pos()) {
                if rect.contains(mouse_pos) {
                    let center = rect.center();
                    let mouse_offset = mouse_pos - center - self.pan;

                    // Adjust pan to keep mouse position stable
                    self.pan = self.pan + mouse_offset * (1.0 - new_zoom / self.zoom);
                    self.zoom = new_zoom;
                }
            }
        }

        // Handle drag for pan
        if response.dragged_by(egui::PointerButton::Primary) {
            // Check if we clicked on a node first
            if let Some(pos) = response.interact_pointer_pos() {
                let mut on_node = false;
                for node in self.flow_graph.graph.node_weights() {
                    if let Some(world_pos) = self.node_positions.get(&node.name) {
                        let screen_pos = self.world_to_screen(*world_pos, rect);
                        let radius =
                            (12.0 + (node.throughput() as f32).log10().max(0.0) * 2.0) * self.zoom;
                        if (screen_pos - pos).length() < radius + 5.0 {
                            on_node = true;
                            break;
                        }
                    }
                }

                if !on_node {
                    self.pan += response.drag_delta();
                }
            }
        }

        // Handle click for node selection
        if response.clicked() {
            if let Some(pos) = response.interact_pointer_pos() {
                let mut clicked_node = None;
                for node in self.flow_graph.graph.node_weights() {
                    if let Some(world_pos) = self.node_positions.get(&node.name) {
                        let screen_pos = self.world_to_screen(*world_pos, rect);
                        let radius =
                            (12.0 + (node.throughput() as f32).log10().max(0.0) * 2.0) * self.zoom;
                        if (screen_pos - pos).length() < radius + 5.0 {
                            clicked_node = Some(node.name.clone());
                            break;
                        }
                    }
                }
                self.selected_node = clicked_node;
            }
        }

        response
    }
}

/// Linearly interpolate between two colors.
fn lerp_color(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    Color32::from_rgba_unmultiplied(
        (a.r() as f32 + (b.r() as f32 - a.r() as f32) * t) as u8,
        (a.g() as f32 + (b.g() as f32 - a.g() as f32) * t) as u8,
        (a.b() as f32 + (b.b() as f32 - a.b() as f32) * t) as u8,
        (a.a() as f32 + (b.a() as f32 - a.a() as f32) * t) as u8,
    )
}

impl eframe::App for NeuronicApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Calculate delta time
        let now = Instant::now();
        let dt = now.duration_since(self.last_frame).as_secs_f32();
        self.last_frame = now;

        // Process incoming data
        self.process_snapshots();

        // Update animations
        self.update_animations(dt);

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

                if self.show_legend {
                    ui.add_space(16.0);
                    self.draw_legend(ui);
                }
            });

        // Central panel with graph
        egui::CentralPanel::default().show(ctx, |ui| {
            let rect = ui.available_rect_before_wrap();

            // Handle input (zoom, pan, click)
            self.handle_input(ui, rect);

            self.draw_graph(ui, rect);
        });
    }
}
