//! Unikode's reusable application library.
//!
//! The public surface exists primarily so the binary and repository-level
//! integration tests exercise the same code. It is not yet a semver-stable SDK.

pub mod application;
pub mod config;
mod constants;
pub mod core;
mod drawing;
pub mod editor;
mod tools;
pub mod tui;

pub use config::{PathMode, RenderMode};
pub use editor::Canvas;
pub use tui::app;
pub use tui::EDITOR_VIEW_ID;
