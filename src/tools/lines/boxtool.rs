use cursive::{
    event::{
        Event, EventResult, MouseButton::Left,
        MouseEvent::{Release, Hold, Press},
    },
    Rect, Vec2,
};

use std::fmt;

use crate::editor::{buffer::*, scroll::EditorCtx};
use crate::constants::CONSUMED;

use super::super::{Tool, simple_display, fn_on_event_drag, option, mouse_drag};
use super::draw_line;

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

        draw_line(buf, r.top_left(), r.top_right());
        draw_line(buf, r.top_right(), r.bottom_right());
        draw_line(buf, r.bottom_right(), r.bottom_left());
        draw_line(buf, r.bottom_left(), r.top_left());
    });
}
