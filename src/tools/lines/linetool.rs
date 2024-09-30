use cursive::{
    event::{
        Event, EventResult, MouseButton::Left,
        MouseEvent::{Release, Hold, Press},
    },
    Vec2,
};
use std::fmt;

use crate::constants::CONSUMED;
use crate::editor::{buffer::*, scroll::EditorCtx};
use crate::implementations::options::Options;

use super::super::{PathMode, Tool, fn_on_event_drag, option, mouse_drag};
use super::{draw_path, draw_line, snap90, snap45};

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
            draw_path(buf, src, dst);
            return;
        }

        let mid = match t.path_mode {
            PathMode::Snap90 => snap90(buf, src, dst),
            _ => snap45(src, dst),
        };

        draw_line(buf, src, mid);
        draw_line(buf, mid, dst);
    });
}
