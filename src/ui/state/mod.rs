//! Application state sub-structs.
//!
//! This module contains focused state types that together compose the
//! full application state. Each struct handles a specific concern:
//!
//! - [`ConnectionState`] - RabbitMQ connection and data flow
//! - [`ViewState`] - Camera position, zoom, and layout mode
//! - [`InteractionState`] - Selection, dragging, and search
//! - [`AnimationState`] - Visual effects and particle systems
//! - [`FilterState`] - Topic filtering and node grouping
//! - [`UIPreferences`] - Display toggles and theme settings

mod animation;
mod connection;
mod filters;
mod interaction;
mod preferences;
mod view;

pub use animation::AnimationState;
pub use connection::ConnectionState;
pub use filters::FilterState;
pub use interaction::{InteractionState, SearchState};
pub use preferences::UIPreferences;
pub use view::ViewState;
