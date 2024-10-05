use cursive::{
    event::{
        Event, EventResult, MouseButton::Left,
        MouseEvent::{Hold, Press, Release},
    }, Rect, Vec2,
    logger
};

use line_drawing::Bresenham;
use log::{debug, log, Level};

use std::{array, collections::{HashMap, HashSet}, fmt};

use crate::{constants::{DIAG2, GAID2}, editor::{buffer::Buffer, scroll::EditorCtx}};
// use crate::constants::{
//     TLCORN, TRCORN, BLCORN, BRCORN, VLINE, HLINE, GAID, DIAG,
//     LHINTER, RHINTER, TVINTER, BVINTER, CINTER,
//     CONSUMED
// };
use crate::constants::*;
use crate::editor::cell::Cell;
use crate::implementations::rectedges::RectEdges;

use super::super::{Tool, simple_display, fn_on_event_drag, option, mouse_drag};

#[derive(Copy, Clone, Default)]
pub(crate) struct BoxTool {
    src: Option<Vec2>,
    dst: Option<Vec2>,
}

const BOX_DRAWING: [char; 11] = [
    BRCORN,
    BLCORN,
    TRCORN,
    TLCORN,
    VLINE,
    HLINE,
    LHINTER,
    RHINTER,
    BVINTER,
    TVINTER,
    CINTER,
];

// TODO consider these cases
const BOX_CORNERS: [char; 4] = [
    BRCORN,
    BLCORN,
    TRCORN,
    TLCORN,
];


simple_display! { BoxTool, "Box" }

impl Tool for BoxTool {
    fn_on_event_drag!(|t: &Self, buf: &mut Buffer| {
        let (src, dst) = option!(t.src, t.dst);

        let r = Rect::from_corners(src, dst);
        let re = RectEdges::new(r);

        draw_line(buf, r.top_left(), r.top_right(), r);
        draw_line(buf, r.top_right(), r.bottom_right(), r);
        draw_line(buf, r.bottom_right(), r.bottom_left(), r);
        draw_line(buf, r.bottom_left(), r.top_left(), r);

        let mut change_set = Vec::new();
        
        for &(x, y) in &re.coordinate_outline {
            let pos = (x, y);

            let n = |(x, y): (usize, usize)| (x, y - 1);
            let e = |(x, y): (usize, usize)| (x + 1, y);
            let s = |(x, y): (usize, usize)| (x, y + 1);
            let w = |(x, y): (usize, usize)| (x - 1, y);

            let (u, r, d, l) = (n(pos), e(pos), s(pos), w(pos));

            let centre = DirMapping{coord: pos, box_char: get_coord_safely(pos, buf)};
            let up = DirMapping{coord: u, box_char: get_coord_safely(u, buf)};
            let right = DirMapping{coord: r, box_char: get_coord_safely(r, buf)};
            let down = DirMapping{coord: d, box_char: get_coord_safely(d, buf)};
            let left = DirMapping{coord: l, box_char: get_coord_safely(l, buf)};

            match (up, right, down, left, centre) {

                // 3 case - // need a fn - "not between the corners"
                (u, r, d, l, _) if l.box_char == HLINE && r.box_char == HLINE && d.box_char == SP && u.box_char == SP && !(re.is_between_top(pos) || re.is_between_bottom(pos)) => change_set.push((pos, CINTER)),
                (u, r, d, l, _) if l.box_char == SP && r.box_char == SP && d.box_char == VLINE && u.box_char == VLINE && !(re.is_between_left(pos) || re.is_between_right(pos))=> change_set.push((pos, CINTER)),

                // (u, r, _, l)
                // (u, _, d, l)
                // (_, r, d, l)

                // 2 case
                // (u, r, _, _)
                // (u, _, d, _)
                // (u, _, _, l)
                // (_, r, d, _)
                // (_, r, _, l)
                // (_, _, d, l)


                // none
                (_, _, _, _, _) => (),
            };
        }


        for (_i, cs) in change_set.into_iter().enumerate() {
            let (pos, c) = cs;

            setv2(buf, true, pos.into(), c);
        }

    });
}

#[derive(Debug, Copy, Clone)]
struct DirMapping {
    coord: (usize, usize),
    box_char: char,
}

// get a coordinate from the buffer safely - return ' ' if unsuccessful or if coordinate
// retrieved is not part of the BOX_DRAWING set
fn get_coord_safely(coord: (usize, usize), buf: &mut Buffer) -> char {

    if !buf.visible(coord.into()) {
        return SP
    } 

    let c = buf.getv(coord.into()).unwrap();

    if !BOX_DRAWING.contains(&c) {
        return SP
    }
    
    c
}

fn draw_line(buf: &mut Buffer, src: Vec2, dst: Vec2, r: Rect) {
    
    for (i, (a, _)) in Bresenham::new(src.signed().pair(), dst.signed().pair())
        .steps()
        .enumerate()
    {
        let c = match (i, src, dst) {
            (0, s, _) if s == r.top_left() => TLCORN,
            (0, s, _) if s == r.top_right() => TRCORN,
            (0, s, _) if s == r.bottom_left() => BLCORN,
            (0, s, _) if s == r.bottom_right() => BRCORN,
            (_, s, d) if i > 0 && i < d.x.abs_diff(s.x) => HLINE,
            (_, s, d) if i > 0 && i < d.y.abs_diff(s.y) => VLINE,
            _ => '+',
        };

        set2(buf, false, a.0 as usize, a.1 as usize, c);
    }
}

/// Set the cell at `(x, y)` to `c`.
fn set2(buf: &mut Buffer, force: bool, x: usize, y: usize, c: char) {
    setv2(buf, force, Vec2::new(x, y), c)
}

/// Set the cell at `pos` to `c`.
fn setv2(buf: &mut Buffer, force: bool, pos: Vec2, c: char) {
    if force {
        buf.edits.push(Cell { pos, c });

        return;
    }

    let max_prec = precedence2(c);
    let overrides = |_c| _c == c || precedence2(_c) > max_prec;

    let mut overridden = false;
    if buf.chars.len() > pos.y && buf.chars[pos.y].len() > pos.x {
        overridden |= overrides(buf.chars[pos.y][pos.x]);
    }

    overridden |= buf
        .edits
        .iter()
        .filter(|cell| cell.pos == pos)
        .any(|cell| overrides(cell.c));

    if !overridden {
        buf.edits.push(Cell { pos, c });
    }
}

fn precedence2(c: char) -> usize {
    match c {
        PLUS => 5,
        VLINE => 4,
        PIPE => 3,
        DIAG => 2,
        GAID => 1,
        _ => 0,
    }
}

/* TODO BREAKDOWN
 * [x] create function draw_line that handles the corners of the box
 * [] update function to handle the left and right intersections of the box
 * [] update function to handle the top and bottom intersections of the box
 * [x] update function to handle centre intersections of the box
 * [] do precedence setting?
 *   is the box that is being moved have full precedence?
 *   should each tool implement a precedence function that the buffer uses?
 */
