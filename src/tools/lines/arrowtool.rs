use cursive::{
    event::{
        Event, EventResult, MouseButton::Left,
        MouseEvent::{Release, Hold, Press},
    },
    Vec2,
};

use std::fmt;

use crate::editor::{buffer::*, scroll::EditorCtx};
use crate::config::{Options, Symbols};
use crate::constants::{
    S_N, S_E, S_S, S_W,
    CONSUMED
};

use super::super::{
    PathMode, Tool, fn_on_event_drag, option, mouse_drag
};
use super::{
    draw_path, draw_line, line_slope, snap45, snap90, fixup
};

#[derive(Clone, Default)]
pub(crate) struct ArrowTool {
    src: Option<Vec2>,
    dst: Option<Vec2>,
    path_mode: PathMode,
    symbols: Symbols,
}


impl fmt::Display for ArrowTool {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Arrow: {:?}", self.path_mode)
    }
}

impl Tool for ArrowTool {
    fn load_opts(&mut self, opts: &Options) {
        self.path_mode = opts.path_mode;
        self.symbols = opts.symbols.clone();
    }

    fn_on_event_drag!(|t: &Self, buf: &mut Buffer| {
        let (src, dst) = option!(t.src, t.dst);
        draw_arrow_on_buffer(buf, src, dst, t.path_mode, &t.symbols);
    });
}

pub fn draw_arrow_on_buffer(buf: &mut Buffer, src: Vec2, dst: Vec2, path_mode: PathMode, symbols: &Symbols) {
    if let PathMode::Routed = path_mode {
        let last = draw_path(buf, src, dst, symbols);
        draw_arrow_tip(buf, last, dst, symbols);
        return;
    }

    let mid = match path_mode {
        PathMode::Snap90 => snap90(buf, src, dst, symbols),
        _ => snap45(src, dst),
    };

    if mid != dst {
        let mut points = draw_line(buf, src, mid, symbols);
        points.extend(draw_line(buf, mid, dst, symbols));
        fixup(buf, &points, true, symbols);
        draw_arrow_tip(buf, mid, dst, symbols);
    } else {
        let points = draw_line(buf, src, dst, symbols);
        fixup(buf, &points, true, symbols);
        draw_arrow_tip(buf, src, dst, symbols);
    }
}

fn draw_arrow_tip(buf: &mut Buffer, src: Vec2, dst: Vec2, symbols: &Symbols) {
    let dec = |v: usize| v - 1;
    let inc = |v: usize| v + 1;

    let north = dst.y > 0 && buf.visible(dst.map_y(dec));
    let east = buf.visible(dst.map_x(inc));
    let south = buf.visible(dst.map_y(inc));
    let west = dst.x > 0 && buf.visible(dst.map_x(dec));

    let tip = match line_slope(src, dst).pair() {
        S_N if north || (west && east) => symbols.n,
        S_N if west => symbols.w,
        S_N if east => symbols.e,
        S_N => symbols.n,

        S_E if east || (north && south) => symbols.e,
        S_E if north => symbols.n,
        S_E if south => symbols.s,
        S_E => symbols.e,

        S_S if south || (east && west) => symbols.s,
        S_S if east => symbols.e,
        S_S if west => symbols.w,
        S_S => symbols.s,

        S_W if west || (south && north) => symbols.w,
        S_W if south => symbols.s,
        S_W if north => symbols.n,
        S_W => symbols.w,

        // SE
        (x, y) if x > 0 && y > 0 && buf.visible(dst.map_x(inc)) => symbols.e,
        (x, y) if x > 0 && y > 0 => symbols.s,

        // NE
        (x, y) if x > 0 && y < 0 && buf.visible(dst.map_x(inc)) => symbols.e,
        (x, y) if x > 0 && y < 0 => symbols.n,

        // SW
        (x, y) if x < 0 && y > 0 && dst.x == 0 => symbols.s,
        (x, y) if x < 0 && y > 0 && buf.visible(dst.map_x(dec)) => symbols.w,
        (x, y) if x < 0 && y > 0 => symbols.s,

        // NW
        (x, y) if x < 0 && y < 0 && dst.x == 0 => symbols.n,
        (x, y) if x < 0 && y < 0 && buf.visible(dst.map_x(dec)) => symbols.w,
        (x, y) if x < 0 && y < 0 => symbols.n,

        (_, _) => symbols.plus,
    };

    buf.setv(true, dst, tip, symbols);
}