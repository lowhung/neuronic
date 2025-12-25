//! Snapshot history and replay functionality.

use buswatch_types::Snapshot;
use std::collections::VecDeque;
use std::time::Instant;

use super::types::SnapshotEntry;

/// Add a snapshot to history.
pub fn add_to_history(history: &mut VecDeque<SnapshotEntry>, snapshot: Snapshot, max_size: usize) {
    history.push_back(SnapshotEntry {
        snapshot,
        timestamp: Instant::now(),
    });

    // Trim to max size
    while history.len() > max_size {
        history.pop_front();
    }
}

/// Get snapshot at playback position.
#[allow(dead_code)]
pub fn get_playback_snapshot(
    history: &VecDeque<SnapshotEntry>,
    position: Option<usize>,
) -> Option<&Snapshot> {
    position.and_then(|pos| history.get(pos).map(|e| &e.snapshot))
}

/// Adjust playback position after history changes.
#[allow(dead_code)]
pub fn adjust_playback_position(position: &mut Option<usize>, history_len: usize) {
    if let Some(pos) = position {
        if *pos >= history_len {
            *position = Some(history_len.saturating_sub(1));
        }
    }
}
