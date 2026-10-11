use cursive::{event::EventResult, view::Margins};

use std::f64::consts::SQRT_2;

pub const EDITOR_ID: &str = "editor";
pub(crate) const POPUP_ID: &str = "generic_popup";
pub(crate) const INPUT_ID: &str = "generic_input";

pub(crate) const S_N: (isize, isize) = (0, -1);
pub(crate) const S_E: (isize, isize) = (1, 0);
pub(crate) const S_S: (isize, isize) = (0, 1);
pub(crate) const S_W: (isize, isize) = (-1, 0);

pub(crate) const D: f64 = 1.0;
pub(crate) const D2: f64 = SQRT_2;

pub(crate) const SP: char = ' ';

pub(crate) const CONSUMED: Option<EventResult> = Some(EventResult::Consumed(None));

pub(crate) const NO_MARGIN: Margins = Margins {
    left: 0,
    right: 0,
    top: 0,
    bottom: 0,
};
