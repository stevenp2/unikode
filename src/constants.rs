//! Fixed internal constants.
//!
//! These values are implementation details — they do not appear in
//! `unikode.toml` and cannot be overridden by the user.
//!
//! For user-configurable defaults, see `defaults.rs`.

use cursive::{
    view::Margins,
    event::EventResult,
};

use std::f64::consts::SQRT_2;

// Editor ids
pub(crate) const EDITOR_ID: &str = "editor";
pub(crate) const POPUP_ID: &str = "generic_popup";
pub(crate) const INPUT_ID: &str = "generic_input";

// Pathfinding
pub(crate) const S_N: (isize, isize) = (0, -1);
pub(crate) const S_E: (isize, isize) = (1, 0);
pub(crate) const S_S: (isize, isize) = (0, 1);
pub(crate) const S_W: (isize, isize) = (-1, 0);

// Cost to move one step on the cardinal plane
pub(crate) const D: f64 = 1.0;
// Cost to move one step on the diagonal plane
pub(crate) const D2: f64 = SQRT_2;

// UI Layout
pub(crate) const SP: char = ' ';

pub(crate) const CONSUMED: Option<EventResult> = Some(EventResult::Consumed(None));

pub(crate) const NO_MARGIN: Margins = Margins {
    left: 0,
    right: 0,
    top: 0,
    bottom: 0,
};
