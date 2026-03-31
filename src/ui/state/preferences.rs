//! UI preferences for display settings.

use crate::ui::theme::Theme;

/// User preferences for UI display options.
pub struct UIPreferences {
    /// Show node labels.
    pub show_labels: bool,
    /// Show the legend panel.
    pub show_legend: bool,
    /// Show the filter panel.
    pub show_filter_panel: bool,
    /// Show the group panel.
    pub show_group_panel: bool,
    /// Show the cycle detection panel.
    pub show_cycle_panel: bool,
    /// Use gradient coloring on edges.
    pub show_gradient_edges: bool,
    /// Show expanding pulse rings on active nodes.
    pub show_pulse_rings: bool,
    /// Show the minimap overview.
    pub show_minimap: bool,
    /// Pause snapshot processing.
    pub paused: bool,
    /// Enable audio feedback for graph events.
    pub sound_enabled: bool,
    /// Current color theme.
    pub theme: Theme,
}

impl Default for UIPreferences {
    fn default() -> Self {
        Self {
            show_labels: true,
            show_legend: true,
            show_filter_panel: false,
            show_group_panel: false,
            show_cycle_panel: false,
            show_gradient_edges: false,
            show_pulse_rings: true,
            show_minimap: false,
            paused: false,
            sound_enabled: false,
            theme: Theme::Dark,
        }
    }
}
