//! Neuronic - Real-time graphical visualization of Caryatid message bus flow.
//!
//! This library provides components for visualizing message flow in distributed
//! systems that use the Caryatid framework. It can be used to:
//!
//! - Subscribe to RabbitMQ monitoring snapshots
//! - Build and update a graph model from snapshots
//! - Render interactive visualizations with egui
//!
//! # Modules
//!
//! - [`config`] - Configuration loading from TOML and environment variables
//! - [`graph`] - Graph data model for message flow visualization
//! - [`subscriber`] - RabbitMQ subscription for receiving monitor snapshots
//! - [`ui`] - GUI components for rendering the visualization
//!
//! # Example
//!
//! ```no_run
//! use neuronic::{NeuronicConfig, MessageFlowGraph, HealthConfig};
//! use std::path::Path;
//!
//! // Load configuration
//! let config = NeuronicConfig::load(Path::new("neuronic.toml")).unwrap();
//!
//! // Create a graph with custom health thresholds
//! let health_config = HealthConfig {
//!     backlog_warning: config.graph.backlog_warning,
//!     backlog_critical: config.graph.backlog_critical,
//!     pending_warning_us: config.graph.pending_warning_ms * 1000,
//!     pending_critical_us: config.graph.pending_critical_ms * 1000,
//! };
//! let graph = MessageFlowGraph::new_with_config(
//!     health_config,
//!     config.filter.ignored_topics,
//! );
//! ```

pub mod config;
pub mod graph;
pub mod subscriber;
pub mod ui;

// Re-export main types for convenience
pub use config::{FilterConfig, GraphConfig, NeuronicConfig, DEFAULT_CONFIG_FILE};
pub use graph::{HealthConfig, HealthStatus, MessageFlowGraph, ModuleNode, TopicEdge};
pub use ui::NeuronicApp;
