//! Node rendering and color logic.

use crate::graph::{HealthStatus, ModuleNode};
use egui::Color32;

use super::math::lerp_color;
use crate::ui::theme::Theme;
use crate::ui::types::NodeActivity;

/// Format a message rate for display.
///
/// Automatically selects appropriate units (M/s, k/s, /s) based on magnitude.
pub fn format_rate(rate: f64) -> String {
    if rate >= 1_000_000.0 {
        format!("{:.1}M/s", rate / 1_000_000.0)
    } else if rate >= 1_000.0 {
        format!("{:.1}k/s", rate / 1_000.0)
    } else if rate >= 1.0 {
        format!("{:.0}/s", rate)
    } else if rate > 0.0 {
        format!("{:.1}/s", rate)
    } else {
        String::new()
    }
}

/// Determine the display color for a node based on health status and activity.
///
/// Colors transition from base -> active -> firing as activity increases,
/// with the base color determined by health status (healthy/warning/critical).
pub fn get_neuron_color(
    node: &ModuleNode,
    activity: Option<&NodeActivity>,
    theme: &Theme,
) -> Color32 {
    let fire_intensity = activity.map(|a| a.fire_intensity).unwrap_or(0.0);

    let base = match node.health {
        HealthStatus::Healthy => theme.neuron_base(),
        HealthStatus::Warning => theme.neuron_warning(),
        HealthStatus::Critical => theme.neuron_critical(),
    };

    if fire_intensity > 0.5 {
        lerp_color(
            theme.neuron_active(),
            theme.neuron_firing(),
            (fire_intensity - 0.5) * 2.0,
        )
    } else if fire_intensity > 0.0 {
        lerp_color(base, theme.neuron_active(), fire_intensity * 2.0)
    } else {
        base
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_rate_millions() {
        assert_eq!(format_rate(1_500_000.0), "1.5M/s");
        assert_eq!(format_rate(2_000_000.0), "2.0M/s");
    }

    #[test]
    fn test_format_rate_thousands() {
        assert_eq!(format_rate(1_500.0), "1.5k/s");
        assert_eq!(format_rate(50_000.0), "50.0k/s");
    }

    #[test]
    fn test_format_rate_ones() {
        assert_eq!(format_rate(50.0), "50/s");
        assert_eq!(format_rate(1.0), "1/s");
    }

    #[test]
    fn test_format_rate_fractional() {
        assert_eq!(format_rate(0.5), "0.5/s");
        assert_eq!(format_rate(0.1), "0.1/s");
    }

    #[test]
    fn test_format_rate_zero() {
        assert_eq!(format_rate(0.0), "");
    }

    #[test]
    fn test_get_neuron_color_healthy_no_activity() {
        let node = ModuleNode {
            name: "test".to_string(),
            total_reads: 0,
            total_writes: 0,
            read_rate: None,
            write_rate: None,
            health: HealthStatus::Healthy,
            read_topics: vec![],
            write_topics: vec![],
        };
        let theme = Theme::Dark;

        let color = get_neuron_color(&node, None, &theme);
        assert_eq!(color, theme.neuron_base());
    }

    #[test]
    fn test_get_neuron_color_warning() {
        let node = ModuleNode {
            name: "test".to_string(),
            total_reads: 0,
            total_writes: 0,
            read_rate: None,
            write_rate: None,
            health: HealthStatus::Warning,
            read_topics: vec![],
            write_topics: vec![],
        };
        let theme = Theme::Dark;

        let color = get_neuron_color(&node, None, &theme);
        assert_eq!(color, theme.neuron_warning());
    }

    #[test]
    fn test_get_neuron_color_critical() {
        let node = ModuleNode {
            name: "test".to_string(),
            total_reads: 0,
            total_writes: 0,
            read_rate: None,
            write_rate: None,
            health: HealthStatus::Critical,
            read_topics: vec![],
            write_topics: vec![],
        };
        let theme = Theme::Dark;

        let color = get_neuron_color(&node, None, &theme);
        assert_eq!(color, theme.neuron_critical());
    }

    #[test]
    fn test_get_neuron_color_with_high_fire_intensity() {
        let node = ModuleNode {
            name: "test".to_string(),
            total_reads: 0,
            total_writes: 0,
            read_rate: None,
            write_rate: None,
            health: HealthStatus::Healthy,
            read_topics: vec![],
            write_topics: vec![],
        };
        let theme = Theme::Dark;

        let activity = NodeActivity {
            last_count: 0,
            fire_intensity: 0.9,
            last_update: std::time::Instant::now(),
        };

        let color = get_neuron_color(&node, Some(&activity), &theme);
        assert_ne!(color, theme.neuron_base());
    }
}
