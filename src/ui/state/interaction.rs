//! Interaction state for selection and search.

use crate::ui::types::SelectedEdge;

/// State for the search functionality.
#[derive(Default)]
pub struct SearchState {
    /// Current search query text.
    pub query: String,
    /// Whether the search box is focused.
    pub focused: bool,
    /// Currently highlighted node from search.
    pub highlighted_node: Option<String>,
    /// List of nodes matching the current query.
    pub matches: Vec<String>,
}

/// State related to user interaction with the graph.
#[derive(Default)]
pub struct InteractionState {
    /// Currently selected node name.
    pub selected_node: Option<String>,
    /// Currently selected edge.
    pub selected_edge: Option<SelectedEdge>,
    /// Node currently being dragged.
    pub dragged_node: Option<String>,
    /// Search state.
    pub search: SearchState,
}
