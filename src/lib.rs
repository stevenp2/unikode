//! public editor API, not yet semver-stable

pub mod application;
pub mod config;
mod constants;
pub mod core;
mod drawing;
pub mod editor;
pub mod tui;

pub use config::{PathMode, RenderMode};
pub use editor::Canvas;
pub use tui::app;
pub use tui::EDITOR_VIEW_ID;
