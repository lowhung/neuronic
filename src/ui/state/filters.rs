//! Filter state for topic filtering and node grouping.

use crate::ui::types::NodeGroup;

/// State for topic filtering and node grouping.
#[derive(Default)]
pub struct FilterState {
    /// Topic prefixes to filter out.
    pub topic_filters: Vec<String>,
    /// Text input for new filter.
    pub new_filter: String,
    /// Node groups for visual clustering.
    pub node_groups: Vec<NodeGroup>,
    /// Text input for new group pattern.
    pub new_group_pattern: String,
}

impl FilterState {
    /// Create a new filter state with initial topic filters.
    pub fn with_filters(topic_filters: Vec<String>) -> Self {
        Self {
            topic_filters,
            ..Default::default()
        }
    }
}
