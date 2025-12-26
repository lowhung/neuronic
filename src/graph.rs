//! Graph data model for message flow visualization.
//!
//! Converts buswatch Snapshots into a petgraph structure suitable
//! for rendering with egui_graphs.

use buswatch_types::Snapshot;
use petgraph::graph::{DiGraph, NodeIndex};
use std::collections::HashMap;
use std::time::Instant;

/// Key for identifying an edge (source module, target module, topic).
type EdgeKey = (String, String, String);

/// Health status of a module or topic connection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthStatus {
    Healthy,
    Warning,
    Critical,
}

/// A module node in the message flow graph.
#[derive(Debug, Clone)]
pub struct ModuleNode {
    /// Module name (e.g., "block_unpacker", "accounts_state").
    pub name: String,
    /// Total messages read by this module.
    pub total_reads: u64,
    /// Total messages written by this module.
    pub total_writes: u64,
    /// Read rate (messages/sec) if available.
    pub read_rate: Option<f64>,
    /// Write rate (messages/sec) if available.
    pub write_rate: Option<f64>,
    /// Current health status.
    pub health: HealthStatus,
    /// Topics this module reads from.
    pub read_topics: Vec<String>,
    /// Topics this module writes to.
    pub write_topics: Vec<String>,
}

impl ModuleNode {
    /// Total throughput (reads + writes).
    pub fn throughput(&self) -> u64 {
        self.total_reads + self.total_writes
    }

    /// Combined rate if available.
    pub fn rate(&self) -> Option<f64> {
        match (self.read_rate, self.write_rate) {
            (Some(r), Some(w)) => Some(r + w),
            (Some(r), None) => Some(r),
            (None, Some(w)) => Some(w),
            (None, None) => None,
        }
    }
}

/// An edge representing a topic connection between modules.
#[derive(Debug, Clone)]
pub struct TopicEdge {
    /// Topic name.
    pub topic: String,
    /// Message count on this topic.
    pub message_count: u64,
    /// Message rate if available.
    pub rate: Option<f64>,
    /// Backlog (unread messages) if available.
    pub backlog: Option<u64>,
    /// Pending time in microseconds if available.
    pub pending_us: Option<u64>,
    /// Health status of this connection.
    pub health: HealthStatus,
}

/// Configuration for health thresholds.
#[derive(Debug, Clone)]
pub struct HealthConfig {
    pub backlog_warning: u64,
    pub backlog_critical: u64,
    pub pending_warning_us: u64,
    pub pending_critical_us: u64,
}

impl Default for HealthConfig {
    fn default() -> Self {
        Self {
            backlog_warning: 100,
            backlog_critical: 1000,
            pending_warning_us: 500_000,    // 500ms
            pending_critical_us: 2_000_000, // 2s
        }
    }
}

impl From<crate::config::GraphConfig> for HealthConfig {
    fn from(config: crate::config::GraphConfig) -> Self {
        Self {
            backlog_warning: config.backlog_warning,
            backlog_critical: config.backlog_critical,
            pending_warning_us: config.pending_warning_ms * 1000,
            pending_critical_us: config.pending_critical_ms * 1000,
        }
    }
}

/// The message flow graph.
pub struct MessageFlowGraph {
    /// The underlying petgraph.
    pub graph: DiGraph<ModuleNode, TopicEdge>,
    /// Map from module name to node index.
    pub module_indices: HashMap<String, NodeIndex>,
    /// Health configuration.
    pub health_config: HealthConfig,
    /// Last snapshot timestamp.
    pub last_update_ms: u64,
    /// Topic patterns to ignore.
    pub ignored_topic_prefixes: Vec<String>,
    /// Previous edge message counts for rate inference.
    prev_edge_counts: HashMap<EdgeKey, u64>,
    /// Timestamp of previous snapshot for rate calculation.
    prev_snapshot_time: Option<Instant>,
}

impl MessageFlowGraph {
    /// Create a new empty graph with default ignored topics.
    pub fn new() -> Self {
        Self {
            graph: DiGraph::new(),
            module_indices: HashMap::new(),
            health_config: HealthConfig::default(),
            last_update_ms: 0,
            ignored_topic_prefixes: vec![
                "cardano.query.".to_string(), // Ignore query topics - too noisy
            ],
            prev_edge_counts: HashMap::new(),
            prev_snapshot_time: None,
        }
    }

    /// Create a graph with custom health config and ignored topics.
    pub fn new_with_config(
        health_config: HealthConfig,
        ignored_topic_prefixes: Vec<String>,
    ) -> Self {
        Self {
            graph: DiGraph::new(),
            module_indices: HashMap::new(),
            health_config,
            last_update_ms: 0,
            ignored_topic_prefixes,
            prev_edge_counts: HashMap::new(),
            prev_snapshot_time: None,
        }
    }

    /// Create a graph with custom health config (for tests).
    #[cfg(test)]
    pub fn with_config(health_config: HealthConfig) -> Self {
        Self::new_with_config(health_config, vec![])
    }

    /// Check if a topic should be ignored.
    fn should_ignore_topic(&self, topic: &str) -> bool {
        self.ignored_topic_prefixes
            .iter()
            .any(|prefix| topic.starts_with(prefix))
    }

    /// Update the graph from a snapshot.
    pub fn update_from_snapshot(&mut self, snapshot: &Snapshot) {
        self.last_update_ms = snapshot.timestamp_ms;
        let now = Instant::now();
        let elapsed_secs = self
            .prev_snapshot_time
            .map(|t| now.duration_since(t).as_secs_f64());

        // First pass: create/update all module nodes
        for (module_name, metrics) in snapshot.iter() {
            let health = self.compute_module_health(metrics);

            // Filter out ignored topics
            let read_topics: Vec<String> = metrics
                .reads
                .keys()
                .filter(|t| !self.should_ignore_topic(t))
                .cloned()
                .collect();
            let write_topics: Vec<String> = metrics
                .writes
                .keys()
                .filter(|t| !self.should_ignore_topic(t))
                .cloned()
                .collect();

            // Compute rates
            let read_rate: Option<f64> = metrics
                .reads
                .values()
                .filter_map(|r| r.rate)
                .reduce(|a, b| a + b);
            let write_rate: Option<f64> = metrics
                .writes
                .values()
                .filter_map(|w| w.rate)
                .reduce(|a, b| a + b);

            let node = ModuleNode {
                name: module_name.clone(),
                total_reads: metrics.total_reads(),
                total_writes: metrics.total_writes(),
                read_rate,
                write_rate,
                health,
                read_topics,
                write_topics,
            };

            if let Some(&idx) = self.module_indices.get(module_name) {
                // Update existing node
                self.graph[idx] = node;
            } else {
                // Add new node
                let idx = self.graph.add_node(node);
                self.module_indices.insert(module_name.clone(), idx);
            }
        }

        // Clear existing edges (we rebuild them each update)
        self.graph.clear_edges();

        // Second pass: create edges based on topic connections
        // For each topic, connect producers to consumers
        let mut topic_producers: HashMap<String, Vec<(NodeIndex, u64, Option<f64>)>> =
            HashMap::new();
        #[allow(clippy::type_complexity)]
        let mut topic_consumers: HashMap<
            String,
            Vec<(NodeIndex, u64, Option<u64>, Option<u64>, Option<f64>)>,
        > = HashMap::new();

        for (module_name, metrics) in snapshot.iter() {
            let idx = self.module_indices[module_name];

            for (topic, write_metrics) in &metrics.writes {
                if self.should_ignore_topic(topic) {
                    continue;
                }
                topic_producers.entry(topic.clone()).or_default().push((
                    idx,
                    write_metrics.count,
                    write_metrics.rate,
                ));
            }

            for (topic, read_metrics) in &metrics.reads {
                if self.should_ignore_topic(topic) {
                    continue;
                }
                topic_consumers.entry(topic.clone()).or_default().push((
                    idx,
                    read_metrics.count,
                    read_metrics.backlog,
                    read_metrics.pending.map(|p| p.as_micros()),
                    read_metrics.rate,
                ));
            }
        }

        // Create edges from producers to consumers
        let mut new_edge_counts: HashMap<EdgeKey, u64> = HashMap::new();

        for (topic, producers) in &topic_producers {
            if let Some(consumers) = topic_consumers.get(topic) {
                for &(producer_idx, write_count, write_rate) in producers {
                    for &(consumer_idx, read_count, backlog, pending_us, read_rate) in consumers {
                        let health = self.compute_edge_health(backlog, pending_us);
                        let message_count = write_count.max(read_count);

                        // Use provided rate, or infer from count delta
                        let rate = write_rate.or(read_rate).or_else(|| {
                            let source_name = &self.graph[producer_idx].name;
                            let target_name = &self.graph[consumer_idx].name;
                            let key = (source_name.clone(), target_name.clone(), topic.clone());

                            // Store current count for next iteration
                            new_edge_counts.insert(key.clone(), message_count);

                            // Calculate rate from previous count if available
                            if let (Some(prev_count), Some(elapsed)) =
                                (self.prev_edge_counts.get(&key), elapsed_secs)
                            {
                                if elapsed > 0.0 && message_count >= *prev_count {
                                    let delta = message_count - prev_count;
                                    return Some(delta as f64 / elapsed);
                                }
                            }
                            None
                        });

                        // Also store count when rate was provided (for consistency)
                        if write_rate.is_some() || read_rate.is_some() {
                            let source_name = &self.graph[producer_idx].name;
                            let target_name = &self.graph[consumer_idx].name;
                            let key = (source_name.clone(), target_name.clone(), topic.clone());
                            new_edge_counts.insert(key, message_count);
                        }

                        let edge = TopicEdge {
                            topic: topic.clone(),
                            message_count,
                            rate,
                            backlog,
                            pending_us,
                            health,
                        };
                        self.graph.add_edge(producer_idx, consumer_idx, edge);
                    }
                }
            }
        }

        // Update state for next snapshot
        self.prev_edge_counts = new_edge_counts;
        self.prev_snapshot_time = Some(now);
    }

    /// Compute health status for a module based on its metrics.
    fn compute_module_health(&self, metrics: &buswatch_types::ModuleMetrics) -> HealthStatus {
        let mut worst = HealthStatus::Healthy;

        for (topic, read) in &metrics.reads {
            // Skip ignored topics
            if self.should_ignore_topic(topic) {
                continue;
            }
            if let Some(backlog) = read.backlog {
                if backlog >= self.health_config.backlog_critical {
                    return HealthStatus::Critical;
                } else if backlog >= self.health_config.backlog_warning {
                    worst = HealthStatus::Warning;
                }
            }
            if let Some(pending) = read.pending {
                let pending_us = pending.as_micros();
                if pending_us >= self.health_config.pending_critical_us {
                    return HealthStatus::Critical;
                } else if pending_us >= self.health_config.pending_warning_us {
                    worst = HealthStatus::Warning;
                }
            }
        }

        for (topic, write) in &metrics.writes {
            // Skip ignored topics
            if self.should_ignore_topic(topic) {
                continue;
            }
            if let Some(pending) = write.pending {
                let pending_us = pending.as_micros();
                if pending_us >= self.health_config.pending_critical_us {
                    return HealthStatus::Critical;
                } else if pending_us >= self.health_config.pending_warning_us {
                    worst = HealthStatus::Warning;
                }
            }
        }

        worst
    }

    /// Compute health status for an edge.
    fn compute_edge_health(&self, backlog: Option<u64>, pending_us: Option<u64>) -> HealthStatus {
        if let Some(backlog) = backlog {
            if backlog >= self.health_config.backlog_critical {
                return HealthStatus::Critical;
            } else if backlog >= self.health_config.backlog_warning {
                return HealthStatus::Warning;
            }
        }

        if let Some(pending) = pending_us {
            if pending >= self.health_config.pending_critical_us {
                return HealthStatus::Critical;
            } else if pending >= self.health_config.pending_warning_us {
                return HealthStatus::Warning;
            }
        }

        HealthStatus::Healthy
    }

    /// Get the number of modules.
    pub fn module_count(&self) -> usize {
        self.graph.node_count()
    }

    /// Get the number of topic connections.
    pub fn edge_count(&self) -> usize {
        self.graph.edge_count()
    }

    /// Check if a module has critical health.
    #[cfg(test)]
    pub fn has_critical_modules(&self) -> bool {
        self.graph
            .node_weights()
            .any(|n| n.health == HealthStatus::Critical)
    }

    /// Get modules with critical health.
    #[cfg(test)]
    pub fn critical_modules(&self) -> Vec<&ModuleNode> {
        self.graph
            .node_weights()
            .filter(|n| n.health == HealthStatus::Critical)
            .collect()
    }
}

impl Default for MessageFlowGraph {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use buswatch_types::Snapshot;

    #[test]
    fn test_empty_graph() {
        let graph = MessageFlowGraph::new();
        assert_eq!(graph.module_count(), 0);
        assert_eq!(graph.edge_count(), 0);
    }

    #[test]
    fn test_update_from_snapshot() {
        let mut graph = MessageFlowGraph::new();

        let snapshot = Snapshot::builder()
            .timestamp_ms(1000)
            .module("producer", |m| m.write("events", |w| w.count(100)))
            .module("consumer", |m| m.read("events", |r| r.count(95).backlog(5)))
            .build();

        graph.update_from_snapshot(&snapshot);

        assert_eq!(graph.module_count(), 2);
        assert_eq!(graph.edge_count(), 1); // producer -> consumer via "events"
    }

    #[test]
    fn test_health_detection() {
        let mut graph = MessageFlowGraph::with_config(HealthConfig {
            backlog_warning: 10,
            backlog_critical: 50,
            pending_warning_us: 100_000,
            pending_critical_us: 500_000,
        });

        let snapshot = Snapshot::builder()
            .module("healthy", |m| m.read("topic", |r| r.count(100).backlog(5)))
            .module("warning", |m| m.read("topic", |r| r.count(100).backlog(20)))
            .module("critical", |m| {
                m.read("topic", |r| r.count(100).backlog(100))
            })
            .build();

        graph.update_from_snapshot(&snapshot);

        let healthy = graph
            .graph
            .node_weights()
            .find(|n| n.name == "healthy")
            .unwrap();
        let warning = graph
            .graph
            .node_weights()
            .find(|n| n.name == "warning")
            .unwrap();
        let critical = graph
            .graph
            .node_weights()
            .find(|n| n.name == "critical")
            .unwrap();

        assert_eq!(healthy.health, HealthStatus::Healthy);
        assert_eq!(warning.health, HealthStatus::Warning);
        assert_eq!(critical.health, HealthStatus::Critical);

        assert!(graph.has_critical_modules());
        assert_eq!(graph.critical_modules().len(), 1);
    }
}
