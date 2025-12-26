//! Module search and highlighting.
//!
//! Provides fuzzy search functionality for finding modules by name,
//! with automatic highlighting and camera focus on matches.

use crate::graph::MessageFlowGraph;
use egui::{Pos2, Rect, Vec2};
use std::collections::HashMap;

/// Find modules whose names match the search query.
///
/// Supports both substring matching and fuzzy matching where query
/// characters appear in order within the module name.
pub fn find_matching_modules(graph: &MessageFlowGraph, query: &str) -> Vec<String> {
    if query.is_empty() {
        return Vec::new();
    }

    let query_lower = query.to_lowercase();

    graph
        .graph
        .node_weights()
        .filter(|node| {
            let name_lower = node.name.to_lowercase();
            // Exact substring match
            if name_lower.contains(&query_lower) {
                return true;
            }
            // Fuzzy match: all query chars appear in order
            fuzzy_match(&name_lower, &query_lower)
        })
        .map(|node| node.name.clone())
        .collect()
}

/// Simple fuzzy matching: all characters in query appear in order in target.
pub fn fuzzy_match(target: &str, query: &str) -> bool {
    let mut target_chars = target.chars();
    for query_char in query.chars() {
        loop {
            match target_chars.next() {
                Some(c) if c == query_char => break,
                Some(_) => continue,
                None => return false,
            }
        }
    }
    true
}

/// Select the best match from a list of candidates.
///
/// Prefers exact prefix matches over substring or fuzzy matches.
pub fn get_best_match(matches: &[String], query: &str) -> Option<String> {
    if matches.is_empty() {
        return None;
    }

    let query_lower = query.to_lowercase();

    // Prefer exact prefix match
    for m in matches {
        if m.to_lowercase().starts_with(&query_lower) {
            return Some(m.clone());
        }
    }

    // Otherwise return first match
    Some(matches[0].clone())
}

/// Pan and zoom the viewport to center on a specific node.
///
/// Used to navigate to search results or bookmarked nodes.
pub fn focus_on_node(
    node_name: &str,
    positions: &HashMap<String, Pos2>,
    viewport_rect: Rect,
    pan: &mut Vec2,
    zoom: &mut f32,
) {
    if let Some(node_pos) = positions.get(node_name) {
        let center = viewport_rect.center();
        // Calculate pan to center the node
        *pan = center - *node_pos;
        // Zoom in slightly if zoomed out
        if *zoom < 1.0 {
            *zoom = 1.0;
        }
    }
}

/// Draw the search box UI with autocomplete dropdown.
///
/// Returns the selected module name if the user picks a match.
pub fn draw_search_box(
    ui: &mut egui::Ui,
    query: &mut String,
    focused: &mut bool,
    matches: &[String],
) -> Option<String> {
    let mut selected = None;

    ui.horizontal(|ui| {
        ui.label("Search:");

        let response = ui.add(
            egui::TextEdit::singleline(query)
                .hint_text("Type to search... (Ctrl+F)")
                .desired_width(150.0),
        );

        if *focused {
            response.request_focus();
            *focused = false;
        }

        if response.changed() && !query.is_empty() {
            // Auto-select best match when typing
            if let Some(best) = get_best_match(matches, query) {
                selected = Some(best);
            }
        }

        // Show match count
        if !query.is_empty() {
            ui.label(format!("({} matches)", matches.len()));
        }

        // Clear button
        if !query.is_empty() && ui.button("x").clicked() {
            query.clear();
            selected = None;
        }
    });

    // Show dropdown of matches if there are multiple
    if !query.is_empty() && matches.len() > 1 {
        ui.horizontal(|ui| {
            ui.label("  ");
            for (i, m) in matches.iter().take(5).enumerate() {
                if ui.selectable_label(false, m).clicked() {
                    selected = Some(m.clone());
                }
                if i < matches.len().min(5) - 1 {
                    ui.label("|");
                }
            }
            if matches.len() > 5 {
                ui.label(format!("... +{}", matches.len() - 5));
            }
        });
    }

    selected
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::MessageFlowGraph;
    use buswatch_types::Snapshot;

    #[test]
    fn test_fuzzy_match_exact() {
        assert!(fuzzy_match("block_unpacker", "block_unpacker"));
    }

    #[test]
    fn test_fuzzy_match_subsequence() {
        assert!(fuzzy_match("block_unpacker", "blkunp"));
        assert!(fuzzy_match("accounts_state", "accst"));
        assert!(fuzzy_match("message_handler", "msghnd"));
    }

    #[test]
    fn test_fuzzy_match_single_char() {
        assert!(fuzzy_match("block_unpacker", "b"));
        assert!(fuzzy_match("block_unpacker", "u"));
    }

    #[test]
    fn test_fuzzy_match_empty_query() {
        assert!(fuzzy_match("block_unpacker", ""));
    }

    #[test]
    fn test_fuzzy_match_no_match() {
        assert!(!fuzzy_match("block_unpacker", "xyz"));
        assert!(!fuzzy_match("block_unpacker", "zb")); // z not in target
    }

    #[test]
    fn test_fuzzy_match_wrong_order() {
        assert!(!fuzzy_match("abc", "cba")); // Characters must appear in order
    }

    #[test]
    fn test_fuzzy_match_query_longer_than_target() {
        assert!(!fuzzy_match("ab", "abc"));
    }

    #[test]
    fn test_find_matching_modules_empty_query() {
        let graph = MessageFlowGraph::new();
        let matches = find_matching_modules(&graph, "");
        assert!(matches.is_empty());
    }

    #[test]
    fn test_find_matching_modules_substring() {
        let mut graph = MessageFlowGraph::new();
        let snapshot = Snapshot::builder()
            .module("block_unpacker", |m| m.write("events", |w| w.count(10)))
            .module("block_handler", |m| m.write("events", |w| w.count(20)))
            .module("accounts_state", |m| m.read("events", |r| r.count(5)))
            .build();
        graph.update_from_snapshot(&snapshot);

        let matches = find_matching_modules(&graph, "block");
        assert_eq!(matches.len(), 2);
        assert!(matches.contains(&"block_unpacker".to_string()));
        assert!(matches.contains(&"block_handler".to_string()));
    }

    #[test]
    fn test_find_matching_modules_case_insensitive() {
        let mut graph = MessageFlowGraph::new();
        let snapshot = Snapshot::builder()
            .module("BlockUnpacker", |m| m.write("events", |w| w.count(10)))
            .build();
        graph.update_from_snapshot(&snapshot);

        let matches = find_matching_modules(&graph, "blockunpacker");
        assert_eq!(matches.len(), 1);
        assert!(matches.contains(&"BlockUnpacker".to_string()));

        let matches_upper = find_matching_modules(&graph, "BLOCK");
        assert_eq!(matches_upper.len(), 1);
    }

    #[test]
    fn test_find_matching_modules_fuzzy() {
        let mut graph = MessageFlowGraph::new();
        let snapshot = Snapshot::builder()
            .module("message_processor", |m| m.write("events", |w| w.count(10)))
            .build();
        graph.update_from_snapshot(&snapshot);

        // Fuzzy match: "msgprc" should match "message_processor"
        let matches = find_matching_modules(&graph, "msgprc");
        assert_eq!(matches.len(), 1);
        assert!(matches.contains(&"message_processor".to_string()));
    }

    #[test]
    fn test_get_best_match_empty() {
        let matches: Vec<String> = vec![];
        assert!(get_best_match(&matches, "test").is_none());
    }

    #[test]
    fn test_get_best_match_prefers_prefix() {
        let matches = vec![
            "handler_block".to_string(),
            "block_handler".to_string(),
            "the_block".to_string(),
        ];

        let best = get_best_match(&matches, "block");
        assert_eq!(best, Some("block_handler".to_string()));
    }

    #[test]
    fn test_get_best_match_returns_first_if_no_prefix() {
        let matches = vec!["handler_block".to_string(), "the_block".to_string()];

        let best = get_best_match(&matches, "block");
        // Neither starts with "block", so returns first
        assert_eq!(best, Some("handler_block".to_string()));
    }

    #[test]
    fn test_get_best_match_case_insensitive() {
        let matches = vec!["Handler".to_string(), "BlockHandler".to_string()];

        let best = get_best_match(&matches, "Block");
        assert_eq!(best, Some("BlockHandler".to_string()));
    }

    #[test]
    fn test_focus_on_node_existing() {
        let mut positions = HashMap::new();
        positions.insert("node1".to_string(), Pos2::new(100.0, 200.0));
        positions.insert("node2".to_string(), Pos2::new(300.0, 400.0));

        let viewport = Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 600.0));
        let mut pan = Vec2::ZERO;
        let mut zoom = 0.5;

        focus_on_node("node1", &positions, viewport, &mut pan, &mut zoom);

        // Zoom should be at least 1.0
        assert_eq!(zoom, 1.0);
        // Pan should center on node1
        let center = viewport.center();
        let expected_pan = center - Pos2::new(100.0, 200.0);
        assert_eq!(pan, expected_pan);
    }

    #[test]
    fn test_focus_on_node_nonexistent() {
        let positions = HashMap::new();
        let viewport = Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 600.0));
        let mut pan = Vec2::new(10.0, 20.0);
        let mut zoom = 2.0;

        focus_on_node("nonexistent", &positions, viewport, &mut pan, &mut zoom);

        // Should not change pan or zoom for nonexistent node
        assert_eq!(pan, Vec2::new(10.0, 20.0));
        assert_eq!(zoom, 2.0);
    }

    #[test]
    fn test_focus_on_node_preserves_high_zoom() {
        let mut positions = HashMap::new();
        positions.insert("node1".to_string(), Pos2::new(100.0, 200.0));

        let viewport = Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 600.0));
        let mut pan = Vec2::ZERO;
        let mut zoom = 2.0; // Already zoomed in

        focus_on_node("node1", &positions, viewport, &mut pan, &mut zoom);

        // Should preserve zoom level if already >= 1.0
        assert_eq!(zoom, 2.0);
    }
}
