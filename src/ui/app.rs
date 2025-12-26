//! Main application window.

use crate::config::NeuronicConfig;
use crate::graph::{MessageFlowGraph, ModuleNode, TopicEdge};
use crate::subscriber;
use buswatch_types::Snapshot;
use egui::{Pos2, Vec2};
use egui_graphs::Graph;
use petgraph::stable_graph::StableGraph;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::mpsc as std_mpsc;
use std::time::Instant;

use super::animations;
use super::drawing::{self, DrawContext};
use super::export;
use super::input::{self, KeyboardAction};
use super::layout;
use super::panels;
use super::search;
use super::theme::Theme;
use super::types::*;

/// Main application state.
pub struct NeuronicApp {
    // Data and connection
    flow_graph: MessageFlowGraph,
    #[allow(dead_code)]
    egui_graph: Graph<ModuleNode, TopicEdge>,
    snapshot_rx: Option<std_mpsc::Receiver<Snapshot>>,
    _runtime: tokio::runtime::Runtime,
    connected: bool,
    connection_error: Option<String>,

    // Layout and positioning
    node_positions: HashMap<String, Pos2>,
    node_velocities: HashMap<String, Vec2>,
    layout_mode: LayoutMode,
    zoom: f32,
    pan: Vec2,

    // Selection and interaction
    selected_node: Option<String>,
    selected_edge: Option<SelectedEdge>,
    dragged_node: Option<String>,
    search_query: String,
    search_focused: bool,
    highlighted_node: Option<String>,
    search_matches: Vec<String>,

    // Animation state
    node_activity: HashMap<String, NodeActivity>,
    synapse_particles: HashMap<(String, String, String), Vec<SynapseParticle>>,
    pulse_rings: HashMap<String, Vec<PulseRing>>,
    last_frame: Instant,

    // Filtering and grouping
    topic_filters: Vec<String>,
    new_filter: String,
    node_groups: Vec<NodeGroup>,
    new_group_pattern: String,

    // UI visibility toggles
    show_labels: bool,
    show_legend: bool,
    show_filter_panel: bool,
    show_group_panel: bool,
    show_gradient_edges: bool,
    show_pulse_rings: bool,
    show_minimap: bool,
    #[allow(dead_code)]
    sound_enabled: bool,
    paused: bool,
    theme: Theme,

    // Stats
    update_count: u64,
}

impl NeuronicApp {
    pub fn new(cc: &eframe::CreationContext<'_>, config_path: PathBuf, topic: String) -> Self {
        let neuronic_config = NeuronicConfig::load(&config_path).unwrap_or_else(|e| {
            tracing::warn!("Failed to load config: {}, using defaults", e);
            NeuronicConfig::default()
        });

        // Set up dark theme
        let theme = Theme::Dark;
        theme.apply_to_egui(&cc.egui_ctx);

        let flow_graph = MessageFlowGraph::new_with_config(
            neuronic_config.graph.clone().into(),
            neuronic_config.filter.ignored_topics.clone(),
        );

        let runtime = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
        let (sync_tx, sync_rx) = std_mpsc::channel();

        let subscriber_config = neuronic_config.subscriber.clone();
        let topic_clone = topic.clone();
        let (connected, connection_error) = {
            let sync_tx = sync_tx.clone();
            match runtime.block_on(async {
                subscriber::create_subscriber(&subscriber_config, &topic_clone).await
            }) {
                Ok((mut async_rx, _handle)) => {
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
            flow_graph,
            egui_graph: Graph::new(StableGraph::new()),
            snapshot_rx: Some(sync_rx),
            _runtime: runtime,
            connected,
            connection_error,
            node_positions: HashMap::new(),
            node_velocities: HashMap::new(),
            layout_mode: LayoutMode::ForceDirected,
            zoom: 1.0,
            pan: Vec2::ZERO,
            selected_node: None,
            selected_edge: None,
            dragged_node: None,
            search_query: String::new(),
            search_focused: false,
            highlighted_node: None,
            search_matches: Vec::new(),
            node_activity: HashMap::new(),
            synapse_particles: HashMap::new(),
            pulse_rings: HashMap::new(),
            last_frame: Instant::now(),
            topic_filters: neuronic_config.filter.ignored_topics.clone(),
            new_filter: String::new(),
            node_groups: Vec::new(),
            new_group_pattern: String::new(),
            show_labels: true,
            show_legend: true,
            show_filter_panel: false,
            show_group_panel: false,
            show_gradient_edges: false,
            show_pulse_rings: true,
            show_minimap: false,
            sound_enabled: false,
            paused: false,
            theme,
            update_count: 0,
        }
    }

    fn process_snapshots(&mut self) {
        if self.paused {
            if let Some(rx) = &self.snapshot_rx {
                while rx.try_recv().is_ok() {}
            }
            return;
        }

        if let Some(rx) = &self.snapshot_rx {
            let mut latest: Option<Snapshot> = None;
            while let Ok(snapshot) = rx.try_recv() {
                latest = Some(snapshot);
            }

            if let Some(snapshot) = latest {
                animations::detect_activity(
                    &snapshot,
                    &mut self.node_activity,
                    &mut self.synapse_particles,
                    &mut self.pulse_rings,
                    self.show_pulse_rings,
                );

                self.flow_graph.update_from_snapshot(&snapshot);
                self.update_count += 1;
            }
        }
    }

    fn draw_controls(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            // Connection status
            if self.connected {
                ui.label(egui::RichText::new("● Connected").color(self.theme.neuron_active()));
            } else {
                ui.label(egui::RichText::new("● Disconnected").color(self.theme.neuron_critical()));
                if let Some(err) = &self.connection_error {
                    ui.label(
                        egui::RichText::new(err)
                            .color(self.theme.neuron_critical())
                            .small(),
                    );
                }
            }

            ui.separator();

            // Search box
            let old_query = self.search_query.clone();
            if let Some(selected) = search::draw_search_box(
                ui,
                &mut self.search_query,
                &mut self.search_focused,
                &self.search_matches,
            ) {
                self.highlighted_node = Some(selected.clone());
                self.selected_node = Some(selected);
            }

            if self.search_query != old_query {
                self.search_matches =
                    search::find_matching_modules(&self.flow_graph, &self.search_query);
                self.highlighted_node =
                    search::get_best_match(&self.search_matches, &self.search_query);
            }

            ui.separator();

            // Layout toggle
            let layout_label = match self.layout_mode {
                LayoutMode::ForceDirected => "Force",
                LayoutMode::Hierarchical => "Hierarchy",
            };
            if ui.button(layout_label).clicked() {
                self.layout_mode = match self.layout_mode {
                    LayoutMode::ForceDirected => LayoutMode::Hierarchical,
                    LayoutMode::Hierarchical => LayoutMode::ForceDirected,
                };
            }

            ui.separator();

            // Theme toggle
            if ui
                .button(if self.theme == Theme::Dark {
                    "Light"
                } else {
                    "Dark"
                })
                .clicked()
            {
                self.theme = self.theme.toggle();
            }

            // Pause button
            if ui
                .button(if self.paused { "Resume" } else { "Pause" })
                .clicked()
            {
                self.paused = !self.paused;
            }

            ui.separator();

            // Toggle buttons
            ui.checkbox(&mut self.show_labels, "Labels");
            ui.checkbox(&mut self.show_minimap, "Minimap");
            ui.checkbox(&mut self.show_gradient_edges, "Gradient");
            ui.checkbox(&mut self.show_pulse_rings, "Pulses");

            ui.separator();

            // Panel toggles
            if ui.button("Filters").clicked() {
                self.show_filter_panel = !self.show_filter_panel;
            }
            if ui.button("Groups").clicked() {
                self.show_group_panel = !self.show_group_panel;
            }

            ui.separator();

            // Export
            export::show_export_dialog(ui, &self.flow_graph, &self.node_positions, &self.theme);

            ui.separator();

            // Zoom controls
            if ui.button("-").clicked() {
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
                "Nodes: {} | Edges: {} | Updates: {}",
                self.flow_graph.module_count(),
                self.flow_graph.edge_count(),
                self.update_count
            ));
        });
    }
}

impl eframe::App for NeuronicApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let now = Instant::now();
        let dt = now.duration_since(self.last_frame).as_secs_f32();
        self.last_frame = now;

        self.theme.apply_to_egui(ctx);
        self.process_snapshots();

        animations::update_animations(
            dt,
            &mut self.node_activity,
            &mut self.synapse_particles,
            &mut self.pulse_rings,
        );

        ctx.request_repaint();

        // Handle keyboard shortcuts
        egui::CentralPanel::default().show(ctx, |ui| {
            match input::handle_keyboard(ui, &mut self.search_focused) {
                KeyboardAction::FocusSearch => {}
                KeyboardAction::ClearSearch => {
                    self.search_query.clear();
                    self.highlighted_node = None;
                    self.search_matches.clear();
                }
                KeyboardAction::None => {}
            }
        });

        // Top panel with controls
        egui::TopBottomPanel::top("controls").show(ctx, |ui| {
            self.draw_controls(ui);
        });

        // Right panel with details
        let mut panel_clicked_edge = None;
        egui::SidePanel::right("details")
            .min_width(200.0)
            .show(ctx, |ui| {
                let panel_result = panels::draw_details_panel(
                    ui,
                    &self.flow_graph,
                    &self.selected_node,
                    &self.selected_edge,
                    &self.node_activity,
                    &self.theme,
                );
                if panel_result.clicked_edge.is_some() {
                    panel_clicked_edge = panel_result.clicked_edge;
                }

                if self.show_legend {
                    ui.add_space(16.0);
                    panels::draw_legend(ui, &self.theme);
                }
            });

        // Handle edge selection from panel topic clicks
        if let Some(edge) = panel_clicked_edge {
            self.selected_edge = Some(edge);
        }

        // Left panel for filters/groups if open
        if self.show_filter_panel || self.show_group_panel {
            egui::SidePanel::left("left_panel")
                .min_width(180.0)
                .show(ctx, |ui| {
                    if self.show_filter_panel {
                        if panels::draw_filter_panel(
                            ui,
                            &mut self.topic_filters,
                            &mut self.new_filter,
                        ) {
                            // Filters changed - update graph's ignored topics
                            self.flow_graph.ignored_topic_prefixes = self.topic_filters.clone();
                        }
                        ui.add_space(16.0);
                    }

                    if self.show_group_panel {
                        panels::draw_group_panel(
                            ui,
                            &mut self.node_groups,
                            &mut self.new_group_pattern,
                            &self.flow_graph,
                        );
                    }
                });
        }

        // Central panel with graph
        egui::CentralPanel::default().show(ctx, |ui| {
            let rect = ui.available_rect_before_wrap();

            // Handle input
            let input_result = input::handle_input(
                ui,
                rect,
                &self.flow_graph,
                &mut self.node_positions,
                &mut self.node_velocities,
                &mut self.zoom,
                &mut self.pan,
                &mut self.dragged_node,
            );

            if let Some(clicked) = input_result.clicked_node {
                self.selected_node = Some(clicked);
                self.selected_edge = None; // Clear edge selection when node selected
            }

            // Apply layout
            layout::apply_layout(
                self.layout_mode,
                &self.flow_graph,
                &mut self.node_positions,
                &mut self.node_velocities,
                rect,
            );

            // Focus on highlighted node
            if let Some(ref node_name) = self.highlighted_node {
                if self.search_focused {
                    search::focus_on_node(
                        node_name,
                        &self.node_positions,
                        rect,
                        &mut self.pan,
                        &mut self.zoom,
                    );
                }
            }

            // Draw the graph
            let draw_ctx = DrawContext {
                graph: &self.flow_graph,
                positions: &self.node_positions,
                activity: &self.node_activity,
                particles: &self.synapse_particles,
                pulse_rings: &self.pulse_rings,
                selected_node: self.selected_node.as_ref(),
                highlighted_node: self.highlighted_node.as_ref(),
                theme: &self.theme,
                zoom: self.zoom,
                pan: self.pan,
                show_labels: self.show_labels,
                show_gradient_edges: self.show_gradient_edges,
                show_pulse_rings: self.show_pulse_rings,
            };

            draw_ctx.draw_graph(ui, rect);

            // Draw minimap
            if self.show_minimap {
                drawing::draw_minimap(
                    ui,
                    &self.flow_graph,
                    &self.node_positions,
                    rect,
                    self.zoom,
                    self.pan,
                    &self.theme,
                );
            }
        });
    }
}
