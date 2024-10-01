use cursive::{
    event::{
        Event, EventResult, MouseButton::Left,
        MouseEvent::{Release, Hold, Press},
    },
    Rect, Vec2,
};
use line_drawing::Bresenham;

use std::fmt;

use crate::{constants::{DIAG2, GAID2}, editor::{buffer::Buffer, scroll::EditorCtx}};
use crate::constants::{
    TLCORN, TRCORN, BLCORN, BRCORN, VLINE, HLINE, GAID, DIAG,
    LHINTER, RHINTER, TVINTER, BVINTER, CINTER,
    CONSUMED
};

use super::super::{Tool, simple_display, fn_on_event_drag, option, mouse_drag, lines::line_slope};

#[derive(Copy, Clone, Default)]
pub(crate) struct BoxTool {
    src: Option<Vec2>,
    dst: Option<Vec2>,
}

simple_display! { BoxTool, "Box" }

impl Tool for BoxTool {
    fn_on_event_drag!(|t: &Self, buf: &mut Buffer| {
        let (src, dst) = option!(t.src, t.dst);

        let r = Rect::from_corners(src, dst);

        draw_line(buf, r.top_left(), r.top_right(), r);
        draw_line(buf, r.top_right(), r.bottom_right(), r);
        draw_line(buf, r.bottom_right(), r.bottom_left(), r);
        draw_line(buf, r.bottom_left(), r.top_left(), r);
    });
}

fn draw_line(buf: &mut Buffer, src: Vec2, dst: Vec2, r: Rect) {
    
    for (i, (s, e)) in Bresenham::new(src.signed().pair(), dst.signed().pair())
        .steps()
        .enumerate()
    {
        let slope = line_slope(s, e).pair();

        let c = match (i, slope, src) {
            (0, _, s) if s == r.top_left() => TLCORN,
            (0, _, s) if s == r.top_right() => TRCORN,
            (0, _, s) if s == r.bottom_left() => BLCORN,
            (0, _, s) if s == r.bottom_right() => BRCORN,
            (_, (0, _), _) => VLINE,
            (_, (_, 0), _) => HLINE,
            (_, (x, y), _) if (x > 0) == (y > 0) => GAID2,
            _ => DIAG2,
        };

        buf.set(false, s.0 as usize, s.1 as usize, c);
    }
}

/* TODO BREAKDOWN
 * [x] create function draw_line that handles the corners of the box
 * [] update function to handle the left and right intersections of the box
 * [] update function to handle the top and bottom intersections of the box
 * [] update function to handle the top and bottom intersections of the box
 * [] update function to handle centre intersections of the box
 * [] do precedence setting?
 *   is the box that is being moved have full precedence?
 *   should each tool implement a precedence function that the buffer uses?
 */
