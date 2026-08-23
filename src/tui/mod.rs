//! Cursive adapters for the headless editor library.

pub mod app;
pub mod controller;
mod conversions;
pub mod dialogs;
pub mod editor_view;
pub mod modeline;
mod theme;

pub use editor_view::EditorView;

/// Stable Cursive view name for integrations that need to locate the editor.
pub const EDITOR_VIEW_ID: &str = crate::constants::EDITOR_ID;
