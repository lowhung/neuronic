//! Animation state and updates.

use buswatch_types::Snapshot;
use std::collections::HashMap;
use std::time::Instant;

use super::types::{NodeActivity, PulseRing, SynapseParticle};

/// Update all animations for a frame.
pub fn update_animations(
    dt: f32,
    node_activity: &mut HashMap<String, NodeActivity>,
    synapse_particles: &mut HashMap<(String, String, String), Vec<SynapseParticle>>,
    pulse_rings: &mut HashMap<String, Vec<PulseRing>>,
) {
    // Decay node fire intensity
    for activity in node_activity.values_mut() {
        activity.fire_intensity = (activity.fire_intensity - dt * 2.0).max(0.0);
    }

    // Update synapse particles
    for particles in synapse_particles.values_mut() {
        for particle in particles.iter_mut() {
            particle.progress += particle.speed * dt;
        }
        particles.retain(|p| p.progress < 1.0);
    }
    synapse_particles.retain(|_, v| !v.is_empty());

    // Update pulse rings
    for rings in pulse_rings.values_mut() {
        for ring in rings.iter_mut() {
            ring.radius += dt * 50.0; // Expand at 50 pixels/sec
            ring.opacity = 1.0 - (ring.radius / ring.max_radius);
        }
        rings.retain(|r| r.radius < r.max_radius);
    }
    pulse_rings.retain(|_, v| !v.is_empty());
}

/// Detect activity changes and trigger firing animations.
pub fn detect_activity(
    snapshot: &Snapshot,
    node_activity: &mut HashMap<String, NodeActivity>,
    synapse_particles: &mut HashMap<(String, String, String), Vec<SynapseParticle>>,
    pulse_rings: &mut HashMap<String, Vec<PulseRing>>,
    show_pulse_rings: bool,
) {
    let now = Instant::now();

    for (module_name, metrics) in snapshot.iter() {
        let total = metrics.total_reads() + metrics.total_writes();
        let activity = node_activity.entry(module_name.clone()).or_default();

        if total > activity.last_count {
            let delta = total - activity.last_count;
            activity.fire_intensity =
                (activity.fire_intensity + (delta as f32 / 100.0).min(1.0)).min(1.0);
            activity.last_update = now;

            // Spawn pulse ring for heavy activity
            if show_pulse_rings && delta > 50 {
                let rings = pulse_rings.entry(module_name.clone()).or_default();
                rings.push(PulseRing {
                    radius: 15.0,
                    max_radius: 60.0 + (delta as f32 / 10.0).min(40.0),
                    opacity: 1.0,
                });
            }

            // Spawn particles on outgoing edges
            for topic in metrics.writes.keys() {
                for (other_name, other_metrics) in snapshot.iter() {
                    if other_metrics.reads.contains_key(topic) {
                        let key = (module_name.clone(), other_name.clone(), topic.clone());
                        let particles = synapse_particles.entry(key).or_default();
                        let new_particles = (delta as usize / 50).max(1).min(5);
                        for i in 0..new_particles {
                            particles.push(SynapseParticle {
                                progress: i as f32 * 0.1,
                                speed: 0.8 + (i as f32 * 0.1),
                            });
                        }
                    }
                }
            }
        }

        activity.last_count = total;
    }
}
