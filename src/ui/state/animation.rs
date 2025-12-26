//! Animation state for visual effects.

use std::collections::HashMap;
use std::time::Instant;

use crate::ui::types::{NodeActivity, PulseRing, SynapseParticle};

/// State for all animation effects.
pub struct AnimationState {
    /// Activity state per node (for firing glow).
    pub node_activity: HashMap<String, NodeActivity>,
    /// Particles traveling along edges (source, target, topic) -> particles.
    pub synapse_particles: HashMap<(String, String, String), Vec<SynapseParticle>>,
    /// Expanding pulse rings per node.
    pub pulse_rings: HashMap<String, Vec<PulseRing>>,
    /// Timestamp of the last frame for delta time calculation.
    pub last_frame: Instant,
}

impl Default for AnimationState {
    fn default() -> Self {
        Self {
            node_activity: HashMap::new(),
            synapse_particles: HashMap::new(),
            pulse_rings: HashMap::new(),
            last_frame: Instant::now(),
        }
    }
}
