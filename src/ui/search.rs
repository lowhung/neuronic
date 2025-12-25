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
fn fuzzy_match(target: &str, query: &str) -> bool {
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
