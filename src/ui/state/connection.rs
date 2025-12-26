//! Connection state for RabbitMQ data flow.

use crate::graph::MessageFlowGraph;
use buswatch_types::Snapshot;
use std::sync::mpsc as std_mpsc;

/// State related to the data connection and message flow.
pub struct ConnectionState {
    /// The message flow graph built from snapshots.
    pub flow_graph: MessageFlowGraph,
    /// Receiver for snapshot updates from the subscriber.
    pub snapshot_rx: Option<std_mpsc::Receiver<Snapshot>>,
    /// Tokio runtime for async operations.
    pub runtime: tokio::runtime::Runtime,
    /// Whether we're currently connected to the message broker.
    pub connected: bool,
    /// Error message if connection failed.
    pub connection_error: Option<String>,
}
