//! UI module for Neuronic.
//!
//! This module contains all the graphical user interface components:
//!
//! - `app` - Main application state and event loop
//! - `theme` - Light/dark color theme definitions
//! - `drawing` - Graph rendering with Bezier curves
//! - `layout` - Force-directed and hierarchical layout algorithms
//! - `animations` - Particle effects and pulse animations
//! - `input` - Mouse and keyboard interaction handling
//! - `panels` - Side panels for details, legend, filters
//! - `search` - Fuzzy search functionality
//! - `export` - SVG export capability
//! - `types` - Shared type definitions
//!
//! The main public export is [`NeuronicApp`], which implements the eframe
//! application interface.

mod animations;
mod app;
mod drawing;
mod export;
mod input;
mod layout;
mod panels;
mod search;
mod theme;
mod types;

pub use app::NeuronicApp;
