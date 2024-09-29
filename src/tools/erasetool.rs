use cursive::{
    event::{Event, EventResult, MouseButton::*, MouseEvent::*},
    Vec2,
};
use std::fmt;

use crate::editor::{buffer::*, scroll::EditorCtx};
use crate::constants::{
    SP,
    CONSUMED
};

use super::{Tool, visible_cells, simple_display, fn_on_event_drag, mouse_drag, option};

#[derive(Copy, Clone, Default)]
pub(crate) struct EraseTool {
    src: Option<Vec2>,
    dst: Option<Vec2>,
}

simple_display! { EraseTool, "Erase" }

impl Tool for EraseTool {
    fn_on_event_drag!(|t: &Self, buf: &mut Buffer| {
        let state: Vec<_> = visible_cells(buf, option!(t.src, t.dst)).collect();

        for cell in state {
            buf.setv(true, cell.pos(), SP);
        }
    });
}

