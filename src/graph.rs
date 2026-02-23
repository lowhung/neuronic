//! Graph data model for message flow visualization.
//!
//! Converts buswatch Snapshots into a petgraph structure suitable
//! for rendering with egui.

use buswatch_types::Snapshot;
use petgraph::algo::tarjan_scc;
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::visit::EdgeRef;
use std::collections::{HashMap, HashSet};
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

/// Tracks message counts and smoothed rates for rate inference.
#[derive(Default)]
struct RateTracker {
    /// Previous message counts per edge.
    prev_counts: HashMap<EdgeKey, u64>,
    /// Smoothed rates per edge (exponential moving average).
    smoothed_rates: HashMap<EdgeKey, f64>,
    /// Timestamp of previous snapshot.
    prev_time: Option<Instant>,
}

impl RateTracker {
    /// Smoothing factor for exponential moving average (0.0-1.0).
    /// Lower values = smoother (slower to respond), higher = more responsive.
    const SMOOTHING_ALPHA: f64 = 0.3;

    /// Calculate rate for an edge, using smoothing to avoid jumpy values.
    fn calculate_rate(
        &mut self,
        key: EdgeKey,
        message_count: u64,
        elapsed_secs: Option<f64>,
    ) -> Option<f64> {
        let instant_rate = self.calculate_instant_rate(&key, message_count, elapsed_secs);

        // Update stored count for next iteration
        self.prev_counts.insert(key.clone(), message_count);

        // Apply exponential smoothing
        let smoothed = match (instant_rate, self.smoothed_rates.get(&key)) {
            (Some(new_rate), Some(&prev_smoothed)) => {
                Self::SMOOTHING_ALPHA * new_rate + (1.0 - Self::SMOOTHING_ALPHA) * prev_smoothed
            }
            (Some(new_rate), None) => new_rate,
            (None, Some(&prev_smoothed)) => {
                // No new data, decay toward zero
                (1.0 - Self::SMOOTHING_ALPHA) * prev_smoothed
            }
            (None, None) => return None,
        };

        self.smoothed_rates.insert(key, smoothed);
        Some(smoothed)
    }

    /// Calculate instantaneous rate from count delta.
    fn calculate_instant_rate(
        &self,
        key: &EdgeKey,
        message_count: u64,
        elapsed_secs: Option<f64>,
    ) -> Option<f64> {
        let prev_count = self.prev_counts.get(key)?;
        let elapsed = elapsed_secs?;

        if elapsed > 0.0 && message_count >= *prev_count {
            let delta = message_count - prev_count;
            Some(delta as f64 / elapsed)
        } else {
            None
        }
    }

    /// Update the timestamp for the next calculation.
    fn mark_snapshot_time(&mut self, now: Instant) {
        self.prev_time = Some(now);
    }

    /// Get elapsed seconds since last snapshot.
    fn elapsed_secs(&self, now: Instant) -> Option<f64> {
        self.prev_time.map(|t| now.duration_since(t).as_secs_f64())
    }

    /// Remove tracked edge state for modules no longer present in the graph.
    fn prune_stale_modules(&mut self, active_modules: &HashSet<String>) {
        self.prev_counts
            .retain(|(source, target, _), _| active_modules.contains(source) && active_modules.contains(target));
        self.smoothed_rates
            .retain(|(source, target, _), _| active_modules.contains(source) && active_modules.contains(target));
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
    /// Rate inference tracker.
    rate_tracker: RateTracker,
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
            rate_tracker: RateTracker::default(),
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
            rate_tracker: RateTracker::default(),
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
        let elapsed_secs = self.rate_tracker.elapsed_secs(now);
        let mut graph = DiGraph::new();
        let mut module_indices = HashMap::new();
        let active_modules: HashSet<String> = snapshot.iter().map(|(name, _)| name.clone()).collect();

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

            let idx = graph.add_node(node);
            module_indices.insert(module_name.clone(), idx);
        }

        // Second pass: create edges based on topic connections
        // For each topic, connect producers to consumers
        let mut topic_producers: HashMap<String, Vec<(String, u64, Option<f64>)>> =
            HashMap::new();
        #[allow(clippy::type_complexity)]
        let mut topic_consumers: HashMap<
            String,
            Vec<(String, u64, Option<u64>, Option<u64>, Option<f64>)>,
        > = HashMap::new();

        for (module_name, metrics) in snapshot.iter() {
            for (topic, write_metrics) in &metrics.writes {
                if self.should_ignore_topic(topic) {
                    continue;
                }
                topic_producers.entry(topic.clone()).or_default().push((
                    module_name.clone(),
                    write_metrics.count,
                    write_metrics.rate,
                ));
            }

            for (topic, read_metrics) in &metrics.reads {
                if self.should_ignore_topic(topic) {
                    continue;
                }
                topic_consumers.entry(topic.clone()).or_default().push((
                    module_name.clone(),
                    read_metrics.count,
                    read_metrics.backlog,
                    read_metrics.pending.map(|p| p.as_micros()),
                    read_metrics.rate,
                ));
            }
        }

        // Create edges from producers to consumers
        for (topic, producers) in &topic_producers {
            if let Some(consumers) = topic_consumers.get(topic) {
                for (producer_name, write_count, write_rate) in producers {
                    for (consumer_name, read_count, backlog, pending_us, read_rate) in consumers {
                        let Some(&producer_idx) = module_indices.get(producer_name) else {
                            continue;
                        };
                        let Some(&consumer_idx) = module_indices.get(consumer_name) else {
                            continue;
                        };

                        let health = self.compute_edge_health(*backlog, *pending_us);
                        let message_count = (*write_count).max(*read_count);

                        // Use provided rate, or infer from count delta with smoothing
                        let rate = (*write_rate).or(*read_rate).or_else(|| {
                            let key = (
                                producer_name.clone(),
                                consumer_name.clone(),
                                topic.clone(),
                            );
                            self.rate_tracker
                                .calculate_rate(key, message_count, elapsed_secs)
                        });

                        let edge = TopicEdge {
                            topic: topic.clone(),
                            message_count,
                            rate,
                            backlog: *backlog,
                            pending_us: *pending_us,
                            health,
                        };
                        graph.add_edge(producer_idx, consumer_idx, edge);
                    }
                }
            }
        }

        self.rate_tracker.prune_stale_modules(&active_modules);
        self.graph = graph;
        self.module_indices = module_indices;
        self.rate_tracker.mark_snapshot_time(now);
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

    /// Detect all cycles in the message flow graph.
    /// Returns a list of cycles, where each cycle is a list of module names.
    /// Only returns strongly connected components with more than one node.
    pub fn detect_cycles(&self) -> Vec<Vec<String>> {
        let sccs = tarjan_scc(&self.graph);
        sccs.into_iter()
            .filter(|scc| scc.len() > 1)
            .map(|scc| {
                scc.into_iter()
                    .map(|idx| self.graph[idx].name.clone())
                    .collect()
            })
            .collect()
    }

    /// Check if a node participates in any cycle.
    pub fn is_in_cycle(&self, module_name: &str) -> bool {
        let cycles = self.detect_cycles();
        cycles
            .iter()
            .any(|cycle| cycle.contains(&module_name.to_string()))
    }

    /// Get edges that form part of a cycle.
    /// Returns edge keys (source, target, topic) for edges where both endpoints are in the same SCC.
    pub fn cycle_edges(&self) -> Vec<(String, String, String)> {
        let sccs = tarjan_scc(&self.graph);
        let mut cycle_edge_list = Vec::new();

        for scc in sccs.iter().filter(|s| s.len() > 1) {
            let scc_set: std::collections::HashSet<_> = scc.iter().copied().collect();
            for &node_idx in scc {
                for edge_idx in self.graph.edges(node_idx) {
                    if scc_set.contains(&edge_idx.target()) {
                        cycle_edge_list.push((
                            self.graph[edge_idx.source()].name.clone(),
                            self.graph[edge_idx.target()].name.clone(),
                            edge_idx.weight().topic.clone(),
                        ));
                    }
                }
            }
        }

        cycle_edge_list
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
    fn test_modules_removed_when_missing_from_new_snapshot() {
        let mut graph = MessageFlowGraph::new();

        let initial = Snapshot::builder()
            .timestamp_ms(1000)
            .module("producer", |m| m.write("events", |w| w.count(100)))
            .module("consumer", |m| m.read("events", |r| r.count(95).backlog(5)))
            .build();
        graph.update_from_snapshot(&initial);

        let updated = Snapshot::builder()
            .timestamp_ms(2000)
            .module("consumer", |m| m.read("events", |r| r.count(96).backlog(4)))
            .build();
        graph.update_from_snapshot(&updated);

        assert_eq!(graph.module_count(), 1);
        assert!(graph.module_indices.contains_key("consumer"));
        assert!(!graph.module_indices.contains_key("producer"));
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

    #[test]
    fn test_no_cycles_in_linear_graph() {
        let mut graph = MessageFlowGraph::new();
        let snapshot = Snapshot::builder()
            .module("a", |m| m.write("topic1", |w| w.count(10)))
            .module("b", |m| {
                m.read("topic1", |r| r.count(10))
                    .write("topic2", |w| w.count(5))
            })
            .module("c", |m| m.read("topic2", |r| r.count(5)))
            .build();
        graph.update_from_snapshot(&snapshot);

        assert!(graph.detect_cycles().is_empty());
    }

    #[test]
    fn test_detect_simple_cycle() {
        let mut graph = MessageFlowGraph::new();
        // Create cycle: a -> b -> a
        let snapshot = Snapshot::builder()
            .module("a", |m| {
                m.write("topic1", |w| w.count(10))
                    .read("topic2", |r| r.count(5))
            })
            .module("b", |m| {
                m.read("topic1", |r| r.count(10))
                    .write("topic2", |w| w.count(5))
            })
            .build();
        graph.update_from_snapshot(&snapshot);

        let cycles = graph.detect_cycles();
        assert_eq!(cycles.len(), 1);
        assert_eq!(cycles[0].len(), 2);
        assert!(cycles[0].contains(&"a".to_string()));
        assert!(cycles[0].contains(&"b".to_string()));
    }

    #[test]
    fn test_detect_three_node_cycle() {
        let mut graph = MessageFlowGraph::new();
        // Create cycle: a -> b -> c -> a
        let snapshot = Snapshot::builder()
            .module("a", |m| {
                m.write("t1", |w| w.count(10)).read("t3", |r| r.count(5))
            })
            .module("b", |m| {
                m.read("t1", |r| r.count(10)).write("t2", |w| w.count(5))
            })
            .module("c", |m| {
                m.read("t2", |r| r.count(5)).write("t3", |w| w.count(5))
            })
            .build();
        graph.update_from_snapshot(&snapshot);

        let cycles = graph.detect_cycles();
        assert_eq!(cycles.len(), 1);
        assert_eq!(cycles[0].len(), 3);
    }

    #[test]
    fn test_is_in_cycle() {
        let mut graph = MessageFlowGraph::new();
        let snapshot = Snapshot::builder()
            .module("a", |m| {
                m.write("t1", |w| w.count(10)).read("t2", |r| r.count(5))
            })
            .module("b", |m| {
                m.read("t1", |r| r.count(10)).write("t2", |w| w.count(5))
            })
            .module("c", |m| m.read("t1", |r| r.count(10)))
            .build();
        graph.update_from_snapshot(&snapshot);

        assert!(graph.is_in_cycle("a"));
        assert!(graph.is_in_cycle("b"));
        assert!(!graph.is_in_cycle("c"));
    }

    #[test]
    fn test_cycle_edges() {
        let mut graph = MessageFlowGraph::new();
        let snapshot = Snapshot::builder()
            .module("a", |m| {
                m.write("t1", |w| w.count(10)).read("t2", |r| r.count(5))
            })
            .module("b", |m| {
                m.read("t1", |r| r.count(10)).write("t2", |w| w.count(5))
            })
            .build();
        graph.update_from_snapshot(&snapshot);

        let edges = graph.cycle_edges();
        assert_eq!(edges.len(), 2); // a->b via t1 and b->a via t2
    }
}
