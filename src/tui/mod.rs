pub mod app;
pub mod controller;
mod conversions;
pub mod dialogs;
pub mod editor_view;
pub mod modeline;
mod session;
mod text;
mod theme;

pub use editor_view::EditorView;

/// editor view name for cursive lookups
pub const EDITOR_VIEW_ID: &str = crate::constants::EDITOR_ID;
