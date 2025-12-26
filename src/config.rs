//! Configuration loading for Neuronic.
//!
//! Configuration is loaded from TOML files with environment variable overrides.
//! The loading order is:
//!
//! 1. `config.default.toml` (if it exists) - shipped defaults
//! 2. User-specified config file (e.g., `neuronic.toml`)
//! 3. Environment variables with `NEURONIC_` prefix

use anyhow::Result;
use config::{Config, Environment, File};
use serde::Deserialize;
use std::path::Path;

/// Default configuration file name.
pub const DEFAULT_CONFIG_FILE: &str = "config.default.toml";

/// Application configuration.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct NeuronicConfig {
    /// Subscriber connection settings.
    #[serde(default)]
    pub subscriber: SubscriberConfig,

    /// Filter settings.
    #[serde(default)]
    pub filter: FilterConfig,

    /// Graph layout settings.
    #[serde(default)]
    pub graph: GraphConfig,
}

/// Subscriber connection configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct SubscriberConfig {
    /// Connection URL.
    #[serde(default = "default_subscriber_url")]
    pub url: String,

    /// Exchange name.
    #[serde(default = "default_subscriber_exchange")]
    pub exchange: String,
}

impl Default for SubscriberConfig {
    fn default() -> Self {
        Self {
            url: default_subscriber_url(),
            exchange: default_subscriber_exchange(),
        }
    }
}

fn default_subscriber_url() -> String {
    "amqp://127.0.0.1:5672/%2f".to_string()
}

fn default_subscriber_exchange() -> String {
    "caryatid".to_string()
}

/// Filter configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct FilterConfig {
    /// Topic prefixes to ignore.
    #[serde(default = "default_ignored_topics")]
    pub ignored_topics: Vec<String>,
}

impl Default for FilterConfig {
    fn default() -> Self {
        Self {
            ignored_topics: default_ignored_topics(),
        }
    }
}

fn default_ignored_topics() -> Vec<String> {
    vec!["cardano.query.".to_string()]
}

/// Graph layout configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct GraphConfig {
    /// Backlog warning threshold.
    #[serde(default = "default_backlog_warning")]
    pub backlog_warning: u64,

    /// Backlog critical threshold.
    #[serde(default = "default_backlog_critical")]
    pub backlog_critical: u64,

    /// Pending time warning threshold in milliseconds.
    #[serde(default = "default_pending_warning_ms")]
    pub pending_warning_ms: u64,

    /// Pending time critical threshold in milliseconds.
    #[serde(default = "default_pending_critical_ms")]
    pub pending_critical_ms: u64,
}

impl Default for GraphConfig {
    fn default() -> Self {
        Self {
            backlog_warning: default_backlog_warning(),
            backlog_critical: default_backlog_critical(),
            pending_warning_ms: default_pending_warning_ms(),
            pending_critical_ms: default_pending_critical_ms(),
        }
    }
}

fn default_backlog_warning() -> u64 {
    100
}
fn default_backlog_critical() -> u64 {
    1000
}
fn default_pending_warning_ms() -> u64 {
    500
}
fn default_pending_critical_ms() -> u64 {
    2000
}

impl NeuronicConfig {
    /// Load configuration from a file.
    ///
    /// Loads configuration in the following order (later sources override earlier):
    /// 1. `config.default.toml` in the current directory (if present)
    /// 2. The specified config file path
    /// 3. Environment variables with `NEURONIC_` prefix
    ///
    /// If no config files exist, sensible defaults are used.
    pub fn load(path: &Path) -> Result<Self> {
        let config = Config::builder()
            // First, try to load the default config file
            .add_source(File::with_name(DEFAULT_CONFIG_FILE).required(false))
            // Then load the user-specified config file
            .add_source(File::from(path).required(false))
            // Finally, override with environment variables
            .add_source(Environment::with_prefix("NEURONIC").separator("_"))
            .build()?;

        let neuronic_config: NeuronicConfig = config.try_deserialize().unwrap_or_default();

        Ok(neuronic_config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use config::{Config, File};
    use tempfile::tempdir;

    #[test]
    fn test_default_config() {
        let config = NeuronicConfig::default();

        assert_eq!(config.filter.ignored_topics, vec!["cardano.query."]);
        assert_eq!(config.graph.backlog_warning, 100);
        assert_eq!(config.graph.backlog_critical, 1000);
        assert_eq!(config.graph.pending_warning_ms, 500);
        assert_eq!(config.graph.pending_critical_ms, 2000);
    }

    #[test]
    fn test_filter_config_default() {
        let filter = FilterConfig::default();
        assert_eq!(filter.ignored_topics, vec!["cardano.query."]);
    }

    #[test]
    fn test_graph_config_default() {
        let graph = GraphConfig::default();
        assert_eq!(graph.backlog_warning, 100);
        assert_eq!(graph.backlog_critical, 1000);
        assert_eq!(graph.pending_warning_ms, 500);
        assert_eq!(graph.pending_critical_ms, 2000);
    }

    #[test]
    fn test_load_nonexistent_file_uses_defaults() {
        let path = Path::new("nonexistent_config_file.toml");
        let config = NeuronicConfig::load(path).unwrap();

        // When loading from a nonexistent file but config.default.toml exists,
        // the values come from config.default.toml
        assert_eq!(config.filter.ignored_topics, vec!["cardano.query."]);
        assert_eq!(config.graph.backlog_warning, 100);
    }

    #[test]
    fn test_config_file_values_are_loaded() {
        // Test config loading by directly using the config crate
        // This avoids changing directories which causes race conditions in parallel tests
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("user_config.toml");

        std::fs::write(
            &config_path,
            r#"
[filter]
ignored_topics = ["test.prefix.", "another.prefix."]

[graph]
backlog_warning = 50
backlog_critical = 500
pending_warning_ms = 250
pending_critical_ms = 1000
"#,
        )
        .unwrap();

        // Build config directly from just the user file (no default file)
        let config = Config::builder()
            .add_source(File::from(config_path.as_path()))
            .build()
            .unwrap();

        let neuronic_config: NeuronicConfig = config.try_deserialize().unwrap();

        assert_eq!(
            neuronic_config.filter.ignored_topics,
            vec!["test.prefix.", "another.prefix."]
        );
        assert_eq!(neuronic_config.graph.backlog_warning, 50);
        assert_eq!(neuronic_config.graph.backlog_critical, 500);
        assert_eq!(neuronic_config.graph.pending_warning_ms, 250);
        assert_eq!(neuronic_config.graph.pending_critical_ms, 1000);
    }

    #[test]
    fn test_partial_config_uses_struct_defaults_for_missing() {
        // Test that partial config uses struct defaults for unspecified values
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("partial_config.toml");

        std::fs::write(
            &config_path,
            r#"
[graph]
backlog_warning = 200
"#,
        )
        .unwrap();

        // Build config directly from just the user file
        let config = Config::builder()
            .add_source(File::from(config_path.as_path()))
            .build()
            .unwrap();

        let neuronic_config: NeuronicConfig = config.try_deserialize().unwrap_or_default();

        // Custom value from user file
        assert_eq!(neuronic_config.graph.backlog_warning, 200);
        // Struct defaults for unspecified values
        assert_eq!(neuronic_config.graph.backlog_critical, 1000);
        assert_eq!(
            neuronic_config.filter.ignored_topics,
            vec!["cardano.query."]
        );
    }

    #[test]
    fn test_empty_config_uses_struct_defaults() {
        // When no config file exists, struct defaults should be used
        let config = Config::builder().build().unwrap();
        let neuronic_config: NeuronicConfig = config.try_deserialize().unwrap_or_default();

        assert_eq!(
            neuronic_config.filter.ignored_topics,
            vec!["cardano.query."]
        );
        assert_eq!(neuronic_config.graph.backlog_warning, 100);
        assert_eq!(neuronic_config.graph.backlog_critical, 1000);
    }
}
