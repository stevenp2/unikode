use cursive::{
    view::Margins,
    event::EventResult,
};

use std::f64::consts::SQRT_2;

pub(crate) const EDITOR_ID: &str = "editor";
pub(crate) const S90: &str = "Snap90";
pub(crate) const S45: &str = "Snap45";
pub(crate) const RTD: &str = "Routed";

pub(crate) const N: char = '^';
pub(crate) const S: char = 'v';
pub(crate) const W: char = '<';
pub(crate) const E: char = '>';

pub(crate) const S_N: (isize, isize) = (0, -1);
pub(crate) const S_E: (isize, isize) = (1, 0);
pub(crate) const S_S: (isize, isize) = (0, 1);
pub(crate) const S_W: (isize, isize) = (-1, 0);

/// Cost to move one step on the cardinal plane.
pub(crate) const D: f64 = 1.0;
/// Cost to move one step on the diagonal plane.
pub(crate) const D2: f64 = SQRT_2;

pub(crate) const SP: char = ' ';
pub(crate) const DASH: char = '-';
pub(crate) const PIPE: char = '|';
pub(crate) const DIAG: char = '/';
pub(crate) const GAID: char = '\\';
pub(crate) const PLUS: char = '+';
pub(crate) const CURS: char = '_';

pub(crate) const CONSUMED: Option<EventResult> = Some(EventResult::Consumed(None));

pub(crate) const NO_MARGIN: Margins = Margins {
    left: 0,
    right: 0,
    top: 0,
    bottom: 0,
};

pub(crate) const POPUP_ID: &str = "generic_popup";

pub(crate) const INPUT_ID: &str = "generic_input";
