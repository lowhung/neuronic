//! Main application window.

use crate::config::NeuronicConfig;
use crate::graph::MessageFlowGraph;
use crate::subscriber;
use egui::Vec2;
use std::sync::mpsc as std_mpsc;
use std::time::Instant;

use super::animations;
use super::drawing::{self, DrawContext};
use super::export;
use super::input::{self, KeyboardAction};
use super::layout;
use super::panels;
use super::search;
use super::state::{
    AnimationState, ConnectionState, FilterState, InteractionState, UIPreferences, ViewState,
};
use super::theme::Theme;
use super::types::{DataRefs, SelectionRefs, StyleConfig, Transform, VisualRefs};

use std::path::PathBuf;

/// Main application state.
pub struct NeuronicApp {
    /// Connection and data flow state.
    pub connection: ConnectionState,
    /// View and camera state.
    pub view: ViewState,
    /// User interaction state.
    pub interaction: InteractionState,
    /// Animation effects state.
    pub animation: AnimationState,
    /// Filtering and grouping state.
    pub filters: FilterState,
    /// UI display preferences.
    pub preferences: UIPreferences,
    /// Count of processed updates.
    pub update_count: u64,
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

        // Initialize connection state
        let connection = ConnectionState {
            flow_graph,
            snapshot_rx: Some(sync_rx),
            runtime,
            connected,
            connection_error,
        };

        // Initialize preferences with theme
        let preferences = UIPreferences {
            theme,
            ..UIPreferences::default()
        };

        // Initialize filters from config
        let filters = FilterState::with_filters(neuronic_config.filter.ignored_topics.clone());

        Self {
            connection,
            view: ViewState::default(),
            interaction: InteractionState::default(),
            animation: AnimationState::default(),
            filters,
            preferences,
            update_count: 0,
        }
    }

    fn process_snapshots(&mut self) {
        if self.preferences.paused {
            if let Some(rx) = &self.connection.snapshot_rx {
                while rx.try_recv().is_ok() {}
            }
            return;
        }

        if let Some(rx) = &self.connection.snapshot_rx {
            let mut latest = None;
            while let Ok(snapshot) = rx.try_recv() {
                latest = Some(snapshot);
            }

            if let Some(snapshot) = latest {
                animations::detect_activity(
                    &snapshot,
                    &mut self.animation.node_activity,
                    &mut self.animation.synapse_particles,
                    &mut self.animation.pulse_rings,
                    self.preferences.show_pulse_rings,
                );

                self.connection.flow_graph.update_from_snapshot(&snapshot);
                self.update_count += 1;
            }
        }
    }

    fn draw_controls(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            // Connection status
            if self.connection.connected {
                ui.label(
                    egui::RichText::new("● Connected")
                        .color(self.preferences.theme.neuron_active()),
                );
            } else {
                ui.label(
                    egui::RichText::new("● Disconnected")
                        .color(self.preferences.theme.neuron_critical()),
                );
                if let Some(err) = &self.connection.connection_error {
                    ui.label(
                        egui::RichText::new(err)
                            .color(self.preferences.theme.neuron_critical())
                            .small(),
                    );
                }
            }

            ui.separator();

            // Search box
            let old_query = self.interaction.search.query.clone();
            if let Some(selected) = search::draw_search_box(
                ui,
                &mut self.interaction.search.query,
                &mut self.interaction.search.focused,
                &self.interaction.search.matches,
            ) {
                self.interaction.search.highlighted_node = Some(selected.clone());
                self.interaction.selected_node = Some(selected);
            }

            if self.interaction.search.query != old_query {
                self.interaction.search.matches = search::find_matching_modules(
                    &self.connection.flow_graph,
                    &self.interaction.search.query,
                );
                self.interaction.search.highlighted_node = search::get_best_match(
                    &self.interaction.search.matches,
                    &self.interaction.search.query,
                );
            }

            ui.separator();

            // Layout toggle
            let layout_label = match self.view.layout_mode {
                super::types::LayoutMode::ForceDirected => "Force",
                super::types::LayoutMode::Hierarchical => "Hierarchy",
            };
            if ui.button(layout_label).clicked() {
                self.view.layout_mode = match self.view.layout_mode {
                    super::types::LayoutMode::ForceDirected => {
                        super::types::LayoutMode::Hierarchical
                    }
                    super::types::LayoutMode::Hierarchical => {
                        super::types::LayoutMode::ForceDirected
                    }
                };
            }

            ui.separator();

            // Theme toggle
            if ui
                .button(if self.preferences.theme == Theme::Dark {
                    "Light"
                } else {
                    "Dark"
                })
                .clicked()
            {
                self.preferences.theme = self.preferences.theme.toggle();
            }

            // Pause button
            if ui
                .button(if self.preferences.paused {
                    "Resume"
                } else {
                    "Pause"
                })
                .clicked()
            {
                self.preferences.paused = !self.preferences.paused;
            }

            ui.separator();

            // Toggle buttons
            ui.checkbox(&mut self.preferences.show_labels, "Labels");
            ui.checkbox(&mut self.preferences.show_minimap, "Minimap");
            ui.checkbox(&mut self.preferences.show_gradient_edges, "Gradient");
            ui.checkbox(&mut self.preferences.show_pulse_rings, "Pulses");

            ui.separator();

            // Panel toggles
            if ui.button("Filters").clicked() {
                self.preferences.show_filter_panel = !self.preferences.show_filter_panel;
            }
            if ui.button("Groups").clicked() {
                self.preferences.show_group_panel = !self.preferences.show_group_panel;
            }

            ui.separator();

            // Export
            export::show_export_dialog(
                ui,
                &self.connection.flow_graph,
                &self.view.node_positions,
                &self.preferences.theme,
            );

            ui.separator();

            // Zoom controls
            if ui.button("-").clicked() {
                self.view.zoom = (self.view.zoom * 0.8).max(0.1);
            }
            ui.label(format!("{:.0}%", self.view.zoom * 100.0));
            if ui.button("+").clicked() {
                self.view.zoom = (self.view.zoom * 1.25).min(5.0);
            }
            if ui.button("Reset").clicked() {
                self.view.zoom = 1.0;
                self.view.pan = Vec2::ZERO;
            }

            ui.separator();

            // Stats
            ui.label(format!(
                "Nodes: {} | Edges: {} | Updates: {}",
                self.connection.flow_graph.module_count(),
                self.connection.flow_graph.edge_count(),
                self.update_count
            ));
        });
    }
}

impl eframe::App for NeuronicApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let now = Instant::now();
        let dt = now.duration_since(self.animation.last_frame).as_secs_f32();
        self.animation.last_frame = now;

        self.preferences.theme.apply_to_egui(ctx);
        self.process_snapshots();

        animations::update_animations(
            dt,
            &mut self.animation.node_activity,
            &mut self.animation.synapse_particles,
            &mut self.animation.pulse_rings,
        );

        ctx.request_repaint();

        // Handle keyboard shortcuts
        egui::CentralPanel::default().show(ctx, |ui| {
            match input::handle_keyboard(ui, &mut self.interaction.search.focused) {
                KeyboardAction::FocusSearch => {}
                KeyboardAction::ClearSearch => {
                    self.interaction.search.query.clear();
                    self.interaction.search.highlighted_node = None;
                    self.interaction.search.matches.clear();
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
                    &self.connection.flow_graph,
                    &self.interaction.selected_node,
                    &self.interaction.selected_edge,
                    &self.animation.node_activity,
                    &self.preferences.theme,
                );
                if panel_result.clicked_edge.is_some() {
                    panel_clicked_edge = panel_result.clicked_edge;
                }

                if self.preferences.show_legend {
                    ui.add_space(16.0);
                    panels::draw_legend(ui, &self.preferences.theme);
                }

                // Show group legend if groups exist
                if !self.filters.node_groups.is_empty() {
                    ui.add_space(16.0);
                    panels::draw_group_legend(ui, &self.filters.node_groups);
                }
            });

        // Handle edge selection from panel topic clicks
        if let Some(edge) = panel_clicked_edge {
            self.interaction.selected_edge = Some(edge);
        }

        // Left panel for filters/groups if open
        if self.preferences.show_filter_panel || self.preferences.show_group_panel {
            egui::SidePanel::left("left_panel")
                .min_width(180.0)
                .show(ctx, |ui| {
                    if self.preferences.show_filter_panel {
                        if panels::draw_filter_panel(
                            ui,
                            &mut self.filters.topic_filters,
                            &mut self.filters.new_filter,
                        ) {
                            // Filters changed - update graph's ignored topics
                            self.connection.flow_graph.ignored_topic_prefixes =
                                self.filters.topic_filters.clone();
                        }
                        ui.add_space(16.0);
                    }

                    if self.preferences.show_group_panel {
                        panels::draw_group_panel(
                            ui,
                            &mut self.filters.node_groups,
                            &mut self.filters.new_group_pattern,
                            &self.connection.flow_graph,
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
                &self.connection.flow_graph,
                &mut self.view.node_positions,
                &mut self.view.node_velocities,
                &mut self.view.zoom,
                &mut self.view.pan,
                &mut self.interaction.dragged_node,
            );

            if let Some(clicked) = input_result.clicked_node {
                self.interaction.selected_node = Some(clicked);
                self.interaction.selected_edge = None; // Clear edge selection when node selected
            }

            // Apply layout
            layout::apply_layout(
                self.view.layout_mode,
                &self.view.layout_config,
                &self.connection.flow_graph,
                &mut self.view.node_positions,
                &mut self.view.node_velocities,
                rect,
                &self.filters.node_groups,
            );

            // Focus on highlighted node
            if let Some(ref node_name) = self.interaction.search.highlighted_node {
                if self.interaction.search.focused {
                    search::focus_on_node(
                        node_name,
                        &self.view.node_positions,
                        rect,
                        &mut self.view.pan,
                        &mut self.view.zoom,
                    );
                }
            }

            // Draw the graph
            let draw_ctx = DrawContext {
                data: DataRefs {
                    graph: &self.connection.flow_graph,
                    positions: &self.view.node_positions,
                },
                visual: VisualRefs {
                    activity: &self.animation.node_activity,
                    particles: &self.animation.synapse_particles,
                    pulse_rings: &self.animation.pulse_rings,
                    node_groups: &self.filters.node_groups,
                },
                selection: SelectionRefs {
                    node: self.interaction.selected_node.as_ref(),
                    edge: self.interaction.selected_edge.as_ref(),
                    highlighted: self.interaction.search.highlighted_node.as_ref(),
                },
                style: StyleConfig {
                    theme: self.preferences.theme,
                    show_labels: self.preferences.show_labels,
                    show_gradient_edges: self.preferences.show_gradient_edges,
                    show_pulse_rings: self.preferences.show_pulse_rings,
                },
                transform: Transform {
                    zoom: self.view.zoom,
                    pan: self.view.pan,
                },
            };

            draw_ctx.draw_graph(ui, rect);

            // Draw minimap
            if self.preferences.show_minimap {
                drawing::draw_minimap(
                    ui,
                    &self.connection.flow_graph,
                    &self.view.node_positions,
                    rect,
                    self.view.zoom,
                    self.view.pan,
                    &self.preferences.theme,
                );
            }
        });
    }
}
