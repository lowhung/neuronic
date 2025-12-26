//! Color theme definitions for the visualization.

use egui::Color32;

/// Color theme for the visualization.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum Theme {
    #[default]
    Dark,
    Light,
}

impl Theme {
    pub fn neuron_base(&self) -> Color32 {
        match self {
            Theme::Dark => Color32::from_rgb(60, 80, 120),
            Theme::Light => Color32::from_rgb(70, 130, 180),
        }
    }

    pub fn neuron_active(&self) -> Color32 {
        match self {
            Theme::Dark => Color32::from_rgb(100, 180, 255),
            Theme::Light => Color32::from_rgb(30, 144, 255),
        }
    }

    pub fn neuron_firing(&self) -> Color32 {
        match self {
            Theme::Dark => Color32::from_rgb(180, 220, 255),
            Theme::Light => Color32::from_rgb(135, 206, 250),
        }
    }

    pub fn synapse_base(&self) -> Color32 {
        match self {
            Theme::Dark => Color32::from_rgb(80, 60, 120),
            Theme::Light => Color32::from_rgb(147, 112, 219),
        }
    }

    pub fn synapse_active(&self) -> Color32 {
        match self {
            Theme::Dark => Color32::from_rgb(200, 100, 255),
            Theme::Light => Color32::from_rgb(186, 85, 211),
        }
    }

    pub fn neuron_warning(&self) -> Color32 {
        match self {
            Theme::Dark => Color32::from_rgb(255, 180, 80),
            Theme::Light => Color32::from_rgb(255, 140, 0),
        }
    }

    pub fn neuron_critical(&self) -> Color32 {
        match self {
            Theme::Dark => Color32::from_rgb(255, 80, 100),
            Theme::Light => Color32::from_rgb(220, 20, 60),
        }
    }

    pub fn text_primary(&self) -> Color32 {
        match self {
            Theme::Dark => Color32::from_rgb(200, 200, 220),
            Theme::Light => Color32::from_rgb(30, 30, 50),
        }
    }

    pub fn text_secondary(&self) -> Color32 {
        match self {
            Theme::Dark => Color32::from_rgba_unmultiplied(150, 150, 180, 180),
            Theme::Light => Color32::from_rgba_unmultiplied(80, 80, 100, 200),
        }
    }

    pub fn background(&self) -> Color32 {
        match self {
            Theme::Dark => Color32::from_rgb(15, 15, 25),
            Theme::Light => Color32::from_rgb(245, 245, 250),
        }
    }

    pub fn panel_fill(&self) -> Color32 {
        match self {
            Theme::Dark => Color32::from_rgb(20, 20, 35),
            Theme::Light => Color32::from_rgb(255, 255, 255),
        }
    }

    pub fn toggle(&self) -> Theme {
        match self {
            Theme::Dark => Theme::Light,
            Theme::Light => Theme::Dark,
        }
    }

    #[allow(dead_code)]
    pub fn label(&self) -> &'static str {
        match self {
            Theme::Dark => "Dark",
            Theme::Light => "Light",
        }
    }

    pub fn apply_to_egui(&self, ctx: &egui::Context) {
        let mut visuals = match self {
            Theme::Dark => egui::Visuals::dark(),
            Theme::Light => egui::Visuals::light(),
        };
        visuals.panel_fill = self.panel_fill();
        visuals.window_fill = self.panel_fill();
        ctx.set_visuals(visuals);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_theme_is_dark() {
        let theme = Theme::default();
        assert_eq!(theme, Theme::Dark);
    }

    #[test]
    fn test_toggle_dark_to_light() {
        let theme = Theme::Dark;
        assert_eq!(theme.toggle(), Theme::Light);
    }

    #[test]
    fn test_toggle_light_to_dark() {
        let theme = Theme::Light;
        assert_eq!(theme.toggle(), Theme::Dark);
    }

    #[test]
    fn test_dark_theme_colors() {
        let theme = Theme::Dark;

        // Verify dark theme has darker background
        let bg = theme.background();
        assert!(bg.r() < 50 && bg.g() < 50 && bg.b() < 50);

        // Verify text is light on dark background
        let text = theme.text_primary();
        assert!(text.r() > 150 && text.g() > 150 && text.b() > 150);
    }

    #[test]
    fn test_light_theme_colors() {
        let theme = Theme::Light;

        // Verify light theme has lighter background
        let bg = theme.background();
        assert!(bg.r() > 200 && bg.g() > 200 && bg.b() > 200);

        // Verify text is dark on light background
        let text = theme.text_primary();
        assert!(text.r() < 100 && text.g() < 100 && text.b() < 100);
    }

    #[test]
    fn test_neuron_colors_different_between_themes() {
        let dark = Theme::Dark;
        let light = Theme::Light;

        assert_ne!(dark.neuron_base(), light.neuron_base());
        assert_ne!(dark.neuron_active(), light.neuron_active());
        assert_ne!(dark.neuron_firing(), light.neuron_firing());
    }

    #[test]
    fn test_warning_and_critical_colors() {
        let theme = Theme::Dark;

        let warning = theme.neuron_warning();
        let critical = theme.neuron_critical();

        // Warning should be orange-ish (high red, medium green, low blue)
        assert!(warning.r() > warning.b());
        assert!(warning.g() > warning.b());

        // Critical should be red-ish (high red, low green and blue)
        assert!(critical.r() > critical.g());
        assert!(critical.r() > critical.b());
    }

    #[test]
    fn test_synapse_colors() {
        let dark = Theme::Dark;
        let light = Theme::Light;

        // Synapse colors should exist and differ between themes
        assert_ne!(dark.synapse_base(), light.synapse_base());
        assert_ne!(dark.synapse_active(), light.synapse_active());

        // Active synapse should be more vibrant than base
        let dark_base = dark.synapse_base();
        let dark_active = dark.synapse_active();
        let base_intensity = dark_base.r() as u32 + dark_base.g() as u32 + dark_base.b() as u32;
        let active_intensity =
            dark_active.r() as u32 + dark_active.g() as u32 + dark_active.b() as u32;
        assert!(active_intensity > base_intensity);
    }

    #[test]
    fn test_panel_fill() {
        let dark = Theme::Dark;
        let light = Theme::Light;

        // Panel fill should be slightly different from background
        assert_ne!(dark.panel_fill(), dark.background());
        assert_ne!(light.panel_fill(), light.background());
    }

    #[test]
    fn test_label() {
        assert_eq!(Theme::Dark.label(), "Dark");
        assert_eq!(Theme::Light.label(), "Light");
    }
}
