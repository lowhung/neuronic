//! Particle and pulse ring visual effects rendering.

use egui::{Pos2, Stroke};

use super::math::quadratic_bezier;
use crate::ui::theme::Theme;
use crate::ui::types::{PulseRing, SynapseParticle};

/// Draw synapse particles traveling along an edge curve.
pub fn draw_particles(
    painter: &egui::Painter,
    particles: &[SynapseParticle],
    pos_s: Pos2,
    control: Pos2,
    pos_t: Pos2,
    theme: &Theme,
    zoom: f32,
) {
    for particle in particles {
        let particle_pos = quadratic_bezier(pos_s, control, pos_t, particle.progress);
        let particle_color = theme.synapse_active();
        let glow_radius = 4.0 * zoom;
        painter.circle_filled(
            particle_pos,
            glow_radius * 2.0,
            particle_color.gamma_multiply(0.3),
        );
        painter.circle_filled(particle_pos, glow_radius, particle_color);
    }
}

/// Draw expanding pulse rings around a node.
pub fn draw_pulse_rings(
    painter: &egui::Painter,
    rings: &[PulseRing],
    pos: Pos2,
    theme: &Theme,
    zoom: f32,
) {
    for ring in rings {
        let ring_color = theme.neuron_active().gamma_multiply(ring.opacity * 0.5);
        painter.circle_stroke(pos, ring.radius * zoom, Stroke::new(2.0 * zoom, ring_color));
    }
}
