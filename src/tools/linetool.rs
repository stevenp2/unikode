use cursive::{
    event::{Event, EventResult, MouseButton::*, MouseEvent::*},
    Vec2,
};
use std::fmt;

use super::Options;
use super::{PathMode, Tool, fn_on_event_drag, option, mouse_drag};

use crate::editor::{buffer::*, scroll::EditorCtx, CONSUMED};

#[derive(Copy, Clone, Default)]
pub(crate) struct LineTool {
    src: Option<Vec2>,
    dst: Option<Vec2>,
    path_mode: PathMode,
}

impl fmt::Display for LineTool {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Line: {:?}", self.path_mode)
    }
}

impl Tool for LineTool {
    fn load_opts(&mut self, opts: &Options) {
        self.path_mode = opts.path_mode;
    }

    fn_on_event_drag!(|t: &Self, buf: &mut Buffer| {
        let (src, dst) = option!(t.src, t.dst);

        if let PathMode::Routed = t.path_mode {
            buf.draw_path(src, dst);
            return;
        }

        let mid = match t.path_mode {
            PathMode::Snap90 => buf.snap90(src, dst),
            _ => buf.snap45(src, dst),
        };

        buf.draw_line(src, mid);
        buf.draw_line(mid, dst);
    });
}
