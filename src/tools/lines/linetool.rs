use cursive::{
    event::{
        Event, EventResult,
        MouseButton::Left,
        MouseEvent::{Hold, Press, Release},
    },
    Vec2,
};
use std::fmt;

use crate::config::{Options, Symbols};
use crate::constants::CONSUMED;
use crate::core::Point;
use crate::editor::buffer::*;
use crate::editor::command::EditorCommand;
use crate::tui::controller::EditorCtx;

use super::super::{fn_on_event_drag, mouse_drag, option, PathMode, Tool};
use super::{draw_line, draw_path, fixup, snap45, snap90};

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

    fn_on_event_drag!(
        |t: &Self, buf: &mut Buffer| {
            let (src, dst) = option!(t.src, t.dst);
            draw_line_on_buffer(buf, src, dst, t.path_mode, &t.symbols);
        },
        |src, dst| EditorCommand::DrawLine {
            from: Point::from(src),
            to: Point::from(dst),
        }
    );
}

pub fn draw_line_on_buffer(
    buf: &mut Buffer,
    src: Vec2,
    dst: Vec2,
    path_mode: PathMode,
    symbols: &Symbols,
) {
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
