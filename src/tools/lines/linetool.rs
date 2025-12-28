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
use crate::config::{Options, Symbols};

use super::super::{PathMode, Tool, simple_display, fn_on_event_drag, option, mouse_drag};
use super::{draw_path, draw_line, snap90, snap45, fixup};

#[derive(Clone, Default)]
pub(crate) struct LineTool {
    src: Option<Vec2>,
    dst: Option<Vec2>,
    path_mode: PathMode,
    symbols: Symbols,
}

impl fmt::Display for LineTool {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Line: {:?}", self.path_mode)
    }
}

impl Tool for LineTool {
    fn load_opts(&mut self, opts: &Options) {
        self.path_mode = opts.path_mode;
        self.symbols = opts.symbols.clone();
    }

    fn_on_event_drag!(|t: &Self, buf: &mut Buffer| {
        let (src, dst) = option!(t.src, t.dst);
        draw_line_on_buffer(buf, src, dst, t.path_mode, &t.symbols);
    });
}

pub fn draw_line_on_buffer(buf: &mut Buffer, src: Vec2, dst: Vec2, path_mode: PathMode, symbols: &Symbols) {
    if let PathMode::Routed = path_mode {
        draw_path(buf, src, dst, symbols);
        return;
    }

    let mid = match path_mode {
        PathMode::Snap90 => snap90(buf, src, dst, symbols),
        _ => snap45(src, dst),
    };

    let mut points = draw_line(buf, src, mid, symbols);
    points.extend(draw_line(buf, mid, dst, symbols));
    
    fixup(buf, &points, false, symbols);
}