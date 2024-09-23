use cursive::{
    event::{Event, EventResult, MouseButton::*, MouseEvent::*},
    Rect, Vec2
};
use std::fmt;

use super::super::editor::{Buffer, EditorCtx, CONSUMED};
use super::{Tool, simple_display, fn_on_event_drag, option, mouse_drag};

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

        buf.draw_line(r.top_left(), r.top_right());
        buf.draw_line(r.top_right(), r.bottom_right());
        buf.draw_line(r.bottom_right(), r.bottom_left());
        buf.draw_line(r.bottom_left(), r.top_left());
    });
}
