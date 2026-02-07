//! Animation state and updates.
//!
//! Manages visual effects that indicate message activity:
//!
//! - Node firing intensity (glow effect)
//! - Synapse particles traveling along edges
//! - Pulse rings expanding from high-activity nodes

use buswatch_types::Snapshot;
use std::collections::HashMap;
use std::time::Instant;

use super::types::{NodeActivity, PulseRing, SynapseParticle};

/// Maximum number of synapse particles per edge to prevent memory growth.
const MAX_PARTICLES_PER_EDGE: usize = 50;
/// Maximum total synapse particles across all edges.
const MAX_TOTAL_PARTICLES: usize = 5000;
/// Maximum pulse rings per node.
const MAX_PULSE_RINGS_PER_NODE: usize = 3;

/// Update all animation states for a single frame.
///
/// Called each frame to:
/// - Decay node fire intensity over time
/// - Move synapse particles along edges
/// - Expand and fade pulse rings
///
/// Finished animations are automatically removed.
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

/// Detect message activity changes and trigger visual effects.
///
/// Compares current snapshot metrics against previous state to detect
/// new messages. When activity is detected:
/// - Node fire intensity increases proportionally
/// - Synapse particles spawn on active edges
/// - Pulse rings spawn for heavy bursts (50+ messages)
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
                // Limit rings per node to prevent memory growth
                if rings.len() < MAX_PULSE_RINGS_PER_NODE {
                    rings.push(PulseRing {
                        radius: 15.0,
                        max_radius: 60.0 + (delta as f32 / 10.0).min(40.0),
                        opacity: 1.0,
                    });
                }
            }

            // Spawn particles on outgoing edges
            for topic in metrics.writes.keys() {
                for (other_name, other_metrics) in snapshot.iter() {
                    if other_metrics.reads.contains_key(topic) {
                        let key = (module_name.clone(), other_name.clone(), topic.clone());

                        // Check total particle count before spawning
                        let total_particles: usize =
                            synapse_particles.values().map(|v| v.len()).sum();
                        if total_particles >= MAX_TOTAL_PARTICLES {
                            continue;
                        }

                        let particles = synapse_particles.entry(key).or_default();

                        // Check per-edge limit
                        if particles.len() >= MAX_PARTICLES_PER_EDGE {
                            continue;
                        }

                        let new_particles = (delta as usize / 50).clamp(1, 5);
                        for i in 0..new_particles {
                            // Re-check per-edge limit before each particle
                            if particles.len() >= MAX_PARTICLES_PER_EDGE {
                                break;
                            }

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_update_animations_decays_fire_intensity() {
        let mut node_activity = HashMap::new();
        node_activity.insert(
            "test_node".to_string(),
            NodeActivity {
                last_count: 100,
                fire_intensity: 1.0,
                last_update: Instant::now(),
            },
        );
        let mut synapse_particles = HashMap::new();
        let mut pulse_rings = HashMap::new();

        // Run update with dt = 0.1 seconds
        update_animations(
            0.1,
            &mut node_activity,
            &mut synapse_particles,
            &mut pulse_rings,
        );

        // Fire intensity should decay: 1.0 - (0.1 * 2.0) = 0.8
        let activity = node_activity.get("test_node").unwrap();
        assert!((activity.fire_intensity - 0.8).abs() < 0.001);
    }

    #[test]
    fn test_update_animations_fire_intensity_clamps_to_zero() {
        let mut node_activity = HashMap::new();
        node_activity.insert(
            "test_node".to_string(),
            NodeActivity {
                last_count: 100,
                fire_intensity: 0.1,
                last_update: Instant::now(),
            },
        );
        let mut synapse_particles = HashMap::new();
        let mut pulse_rings = HashMap::new();

        // Large dt that would make intensity negative
        update_animations(
            1.0,
            &mut node_activity,
            &mut synapse_particles,
            &mut pulse_rings,
        );

        let activity = node_activity.get("test_node").unwrap();
        assert_eq!(activity.fire_intensity, 0.0);
    }

    #[test]
    fn test_update_animations_moves_particles() {
        let mut node_activity = HashMap::new();
        let mut synapse_particles = HashMap::new();
        synapse_particles.insert(
            ("src".to_string(), "dst".to_string(), "topic".to_string()),
            vec![SynapseParticle {
                progress: 0.0,
                speed: 1.0,
            }],
        );
        let mut pulse_rings = HashMap::new();

        update_animations(
            0.1,
            &mut node_activity,
            &mut synapse_particles,
            &mut pulse_rings,
        );

        let key = ("src".to_string(), "dst".to_string(), "topic".to_string());
        let particles = synapse_particles.get(&key).unwrap();
        // Progress should increase: 0.0 + (1.0 * 0.1) = 0.1
        assert!((particles[0].progress - 0.1).abs() < 0.001);
    }

    #[test]
    fn test_update_animations_removes_finished_particles() {
        let mut node_activity = HashMap::new();
        let mut synapse_particles = HashMap::new();
        synapse_particles.insert(
            ("src".to_string(), "dst".to_string(), "topic".to_string()),
            vec![SynapseParticle {
                progress: 0.95,
                speed: 1.0,
            }],
        );
        let mut pulse_rings = HashMap::new();

        // Progress becomes 0.95 + 0.1 = 1.05, which is >= 1.0
        update_animations(
            0.1,
            &mut node_activity,
            &mut synapse_particles,
            &mut pulse_rings,
        );

        // Particle should be removed, and empty vec should be cleaned up
        assert!(synapse_particles.is_empty());
    }

    #[test]
    fn test_update_animations_expands_pulse_rings() {
        let mut node_activity = HashMap::new();
        let mut synapse_particles = HashMap::new();
        let mut pulse_rings = HashMap::new();
        pulse_rings.insert(
            "test_node".to_string(),
            vec![PulseRing {
                radius: 20.0,
                max_radius: 100.0,
                opacity: 1.0,
            }],
        );

        update_animations(
            0.1,
            &mut node_activity,
            &mut synapse_particles,
            &mut pulse_rings,
        );

        let rings = pulse_rings.get("test_node").unwrap();
        // Radius should increase: 20.0 + (0.1 * 50.0) = 25.0
        assert!((rings[0].radius - 25.0).abs() < 0.001);
        // Opacity should decrease: 1.0 - (25.0 / 100.0) = 0.75
        assert!((rings[0].opacity - 0.75).abs() < 0.001);
    }

    #[test]
    fn test_update_animations_removes_finished_pulse_rings() {
        let mut node_activity = HashMap::new();
        let mut synapse_particles = HashMap::new();
        let mut pulse_rings = HashMap::new();
        pulse_rings.insert(
            "test_node".to_string(),
            vec![PulseRing {
                radius: 95.0,
                max_radius: 100.0,
                opacity: 0.05,
            }],
        );

        // Radius becomes 95.0 + 5.0 = 100.0, which equals max_radius
        update_animations(
            0.1,
            &mut node_activity,
            &mut synapse_particles,
            &mut pulse_rings,
        );

        // Ring should be removed
        assert!(pulse_rings.is_empty());
    }

    #[test]
    fn test_detect_activity_increases_fire_intensity() {
        let snapshot = Snapshot::builder()
            .module("test_module", |m| m.write("topic", |w| w.count(200)))
            .build();

        let mut node_activity = HashMap::new();
        node_activity.insert(
            "test_module".to_string(),
            NodeActivity {
                last_count: 100, // Previous count
                fire_intensity: 0.0,
                last_update: Instant::now(),
            },
        );
        let mut synapse_particles = HashMap::new();
        let mut pulse_rings = HashMap::new();

        detect_activity(
            &snapshot,
            &mut node_activity,
            &mut synapse_particles,
            &mut pulse_rings,
            false,
        );

        let activity = node_activity.get("test_module").unwrap();
        // Delta is 200 - 100 = 100, fire_intensity += min(100/100, 1.0) = 1.0
        assert!(activity.fire_intensity > 0.0);
        // Last count should be updated
        assert_eq!(activity.last_count, 200);
    }

    #[test]
    fn test_detect_activity_spawns_pulse_ring_for_heavy_activity() {
        let snapshot = Snapshot::builder()
            .module("heavy_module", |m| m.write("topic", |w| w.count(200)))
            .build();

        let mut node_activity = HashMap::new();
        node_activity.insert(
            "heavy_module".to_string(),
            NodeActivity {
                last_count: 100, // Delta will be 100 > 50
                fire_intensity: 0.0,
                last_update: Instant::now(),
            },
        );
        let mut synapse_particles = HashMap::new();
        let mut pulse_rings = HashMap::new();

        detect_activity(
            &snapshot,
            &mut node_activity,
            &mut synapse_particles,
            &mut pulse_rings,
            true, // Enable pulse rings
        );

        // Should have spawned a pulse ring
        assert!(pulse_rings.contains_key("heavy_module"));
        let rings = pulse_rings.get("heavy_module").unwrap();
        assert_eq!(rings.len(), 1);
        assert_eq!(rings[0].radius, 15.0); // Initial radius
    }

    #[test]
    fn test_detect_activity_no_pulse_ring_when_disabled() {
        let snapshot = Snapshot::builder()
            .module("heavy_module", |m| m.write("topic", |w| w.count(200)))
            .build();

        let mut node_activity = HashMap::new();
        node_activity.insert(
            "heavy_module".to_string(),
            NodeActivity {
                last_count: 100,
                fire_intensity: 0.0,
                last_update: Instant::now(),
            },
        );
        let mut synapse_particles = HashMap::new();
        let mut pulse_rings = HashMap::new();

        detect_activity(
            &snapshot,
            &mut node_activity,
            &mut synapse_particles,
            &mut pulse_rings,
            false, // Disable pulse rings
        );

        assert!(pulse_rings.is_empty());
    }

    #[test]
    fn test_detect_activity_spawns_synapse_particles() {
        let snapshot = Snapshot::builder()
            .module("producer", |m| m.write("events", |w| w.count(200)))
            .module("consumer", |m| m.read("events", |r| r.count(100)))
            .build();

        let mut node_activity = HashMap::new();
        node_activity.insert(
            "producer".to_string(),
            NodeActivity {
                last_count: 100, // Delta of 100 for producer
                fire_intensity: 0.0,
                last_update: Instant::now(),
            },
        );
        let mut synapse_particles = HashMap::new();
        let mut pulse_rings = HashMap::new();

        detect_activity(
            &snapshot,
            &mut node_activity,
            &mut synapse_particles,
            &mut pulse_rings,
            false,
        );

        // Should have particles on the edge from producer to consumer
        let key = (
            "producer".to_string(),
            "consumer".to_string(),
            "events".to_string(),
        );
        assert!(synapse_particles.contains_key(&key));
        let particles = synapse_particles.get(&key).unwrap();
        assert!(!particles.is_empty());
    }

    #[test]
    fn test_detect_activity_no_change_when_count_same() {
        let snapshot = Snapshot::builder()
            .module("stable_module", |m| m.write("topic", |w| w.count(100)))
            .build();

        let mut node_activity = HashMap::new();
        node_activity.insert(
            "stable_module".to_string(),
            NodeActivity {
                last_count: 100, // Same as snapshot
                fire_intensity: 0.5,
                last_update: Instant::now(),
            },
        );
        let mut synapse_particles = HashMap::new();
        let mut pulse_rings = HashMap::new();

        detect_activity(
            &snapshot,
            &mut node_activity,
            &mut synapse_particles,
            &mut pulse_rings,
            true,
        );

        let activity = node_activity.get("stable_module").unwrap();
        // Fire intensity should not increase (stays at 0.5)
        assert!((activity.fire_intensity - 0.5).abs() < 0.001);
        // No pulse rings
        assert!(pulse_rings.is_empty());
    }

    #[test]
    fn test_detect_activity_creates_new_activity_entry() {
        let snapshot = Snapshot::builder()
            .module("new_module", |m| m.write("topic", |w| w.count(50)))
            .build();

        let mut node_activity = HashMap::new();
        // No entry for new_module yet
        let mut synapse_particles = HashMap::new();
        let mut pulse_rings = HashMap::new();

        detect_activity(
            &snapshot,
            &mut node_activity,
            &mut synapse_particles,
            &mut pulse_rings,
            false,
        );

        // Should have created an entry
        assert!(node_activity.contains_key("new_module"));
        let activity = node_activity.get("new_module").unwrap();
        assert_eq!(activity.last_count, 50);
    }
}
