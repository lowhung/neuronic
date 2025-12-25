//! Neuronic - Real-time graphical visualization of Caryatid message bus flow.
//!
//! A GUI application that subscribes to monitor snapshots and renders
//! an interactive force-directed graph showing message flow between modules.

mod config;
mod graph;
mod subscriber;
mod ui;

pub use config::NeuronicConfig;

use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

/// Real-time message bus flow visualization.
#[derive(Parser, Debug)]
#[command(name = "neuronic", version, about)]
struct Args {
    /// Path to configuration file.
    #[arg(short, long, default_value = "neuronic.toml")]
    config: PathBuf,

    /// RabbitMQ topic to subscribe to.
    #[arg(short, long, default_value = "caryatid.monitor.snapshot")]
    topic: String,

    /// Enable debug logging.
    #[arg(short, long)]
    debug: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();

    // Initialize tracing
    let filter = if args.debug { "debug" } else { "info" };
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| filter.into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("Starting Neuronic");
    tracing::info!("Config: {:?}", args.config);
    tracing::info!("Topic: {}", args.topic);

    // Run the GUI application
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 800.0])
            .with_title("Neuronic - Message Flow Visualizer"),
        ..Default::default()
    };

    eframe::run_native(
        "Neuronic",
        options,
        Box::new(move |cc| Ok(Box::new(ui::NeuronicApp::new(cc, args.config, args.topic)))),
    )
    .map_err(|e| anyhow::anyhow!("eframe error: {}", e))
}
