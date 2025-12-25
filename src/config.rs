//! Configuration loading for Neuronic.

use anyhow::Result;
use config::{Config, Environment, File};
use serde::Deserialize;
use std::path::Path;

/// Application configuration.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct NeuronicConfig {
    /// Filter settings.
    #[serde(default)]
    pub filter: FilterConfig,

    /// Graph layout settings.
    #[serde(default)]
    pub graph: GraphConfig,
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
    pub fn load(path: &Path) -> Result<Self> {
        let config = Config::builder()
            .add_source(File::from(path).required(false))
            .add_source(Environment::with_prefix("NEURONIC").separator("_"))
            .build()?;

        let neuronic_config: NeuronicConfig = config.try_deserialize().unwrap_or_default();

        Ok(neuronic_config)
    }
}
