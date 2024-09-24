use cursive::{
    event::{Event, EventResult, MouseButton::*, MouseEvent::*},
    Vec2,
};
use std::fmt;

use super::{
    Options,
    super::tools::{PathMode, Tool, fn_on_event_drag, option, mouse_drag}
};

use crate::editor::{buffer::*, scroll::EditorCtx, CONSUMED};

#[derive(Copy, Clone, Default)]
pub(crate) struct ArrowTool {
    src: Option<Vec2>,
    dst: Option<Vec2>,
    path_mode: PathMode,
}

impl fmt::Display for ArrowTool {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Arrow: {:?}", self.path_mode)
    }
}

impl Tool for ArrowTool {
    fn load_opts(&mut self, opts: &Options) {
        self.path_mode = opts.path_mode;
    }

    fn_on_event_drag!(|t: &Self, buf: &mut Buffer| {
        let (src, dst) = option!(t.src, t.dst);

        if let PathMode::Routed = t.path_mode {
            let last = buf.draw_path(src, dst);
            buf.draw_arrow_tip(last, dst);
            return;
        }

        let mid = match t.path_mode {
            PathMode::Snap90 => buf.snap90(src, dst),
            _ => buf.snap45(src, dst),
        };

        if mid != dst {
            buf.draw_line(src, mid);
            buf.draw_line(mid, dst);
            buf.draw_arrow_tip(mid, dst);
        } else {
            buf.draw_line(src, dst);
            buf.draw_arrow_tip(src, dst);
        }
    });
}
