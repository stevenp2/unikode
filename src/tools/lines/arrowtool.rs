use cursive::{
    event::{Event, EventResult, MouseButton::*, MouseEvent::*},
    Vec2
};
use std::fmt;

use crate::editor::{buffer::*, scroll::EditorCtx};
use crate::implementations::options::Options;
use crate::constants::{
    S_N, S_E, S_S, S_W,
    N, E, S, W,
    PLUS,
    CONSUMED
};

use super::super::{
    PathMode, Tool, fn_on_event_drag, option, mouse_drag
};
use super::{
    draw_path, draw_line, line_slope, snap45, snap90,
};

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
            let last = draw_path(buf, src, dst);
            draw_arrow_tip(buf, last, dst);
            return;
        }

        let mid = match t.path_mode {
            PathMode::Snap90 => snap90(buf, src, dst),
            _ => snap45(src, dst),
        };

        if mid != dst {
            draw_line(buf, src, mid);
            draw_line(buf, mid, dst);
            draw_arrow_tip(buf, mid, dst);
        } else {
            draw_line(buf, src, dst);
            draw_arrow_tip(buf, src, dst);
        }
    });
}

fn draw_arrow_tip(buf: &mut Buffer, src: Vec2, dst: Vec2) {
    let dec = |v: usize| v - 1;
    let inc = |v: usize| v + 1;

    let north = dst.y > 0 && buf.visible(dst.map_y(dec));
    let east = buf.visible(dst.map_x(inc));
    let south = buf.visible(dst.map_y(inc));
    let west = dst.x > 0 && buf.visible(dst.map_x(dec));

    let tip = match line_slope(src, dst).pair() {
        S_N if north || (west && east) => N,
        S_N if west => W,
        S_N if east => E,
        S_N => N,

        S_E if east || (north && south) => E,
        S_E if north => N,
        S_E if south => S,
        S_E => E,

        S_S if south || (east && west) => S,
        S_S if east => E,
        S_S if west => W,
        S_S => S,

        S_W if west || (south && north) => W,
        S_W if south => S,
        S_W if north => N,
        S_W => W,

        // SE
        (x, y) if x > 0 && y > 0 && buf.visible(dst.map_x(inc)) => E,
        (x, y) if x > 0 && y > 0 => S,

        // NE
        (x, y) if x > 0 && y < 0 && buf.visible(dst.map_x(inc)) => E,
        (x, y) if x > 0 && y < 0 => N,

        // SW
        (x, y) if x < 0 && y > 0 && dst.x == 0 => S,
        (x, y) if x < 0 && y > 0 && buf.visible(dst.map_x(dec)) => W,
        (x, y) if x < 0 && y > 0 => S,

        // NW
        (x, y) if x < 0 && y < 0 && dst.x == 0 => N,
        (x, y) if x < 0 && y < 0 && buf.visible(dst.map_x(dec)) => W,
        (x, y) if x < 0 && y < 0 => N,

        (_, _) => PLUS,
    };

    buf.setv(true, dst, tip);
}
