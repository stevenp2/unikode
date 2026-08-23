//! Unikode's reusable application library.
//!
//! The public surface exists primarily so the binary and repository-level
//! integration tests exercise the same code. It is not yet a semver-stable SDK.

pub mod config;
pub mod constants;
pub mod core;
pub mod editor;
pub mod tools;
pub mod tui;

pub use editor::buffer::Buffer as Canvas;
pub use tui::app;
