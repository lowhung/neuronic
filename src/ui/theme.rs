//! Color theme definitions for the visualization.

use egui::Color32;

/// Color theme for the visualization.
#[derive(Clone, Copy, PartialEq, Default)]
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
