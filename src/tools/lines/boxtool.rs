use cursive::{
    event::{
        Event, EventResult, MouseButton::Left,
        MouseEvent::{Hold, Press, Release},
    }, logger::{self, log}, Rect, Vec2
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

        let rect = Rect::from_corners(src, dst);
        let re = RectEdges::new(rect);

        draw_line(buf, rect.top_left(), rect.top_right(), rect);
        draw_line(buf, rect.top_right(), rect.bottom_right(), rect);
        draw_line(buf, rect.bottom_right(), rect.bottom_left(), rect);
        draw_line(buf, rect.bottom_left(), rect.top_left(), rect);

        let mut change_set = Vec::new();
        let mut corners = HashSet::new();
        
        for &(x, y) in &re.coordinate_outline {
            let pos = (x, y);
            let compass = Compass::new(pos, buf);

            // TODO handle case for single line

            let (new_corners, new_char) = determine_box_join(compass, &re, buf);

            if BOX_DRAWING.contains(&new_char) {
                change_set.push((pos, new_char));
            }

            if new_corners.len() != 0 {
                corners.extend(&new_corners);
            }

        }

        for (_i, cs) in change_set.into_iter().enumerate() {
            let (pos, c) = cs;


            setv2(buf, true, pos.into(), c);
        }

        // handle corners
        for (_i, corner) in corners.into_iter().enumerate() {
            let dir_mapping = handle_corners(corner, buf);

            setv2(buf, true, dir_mapping.coord.into(), dir_mapping.box_char);
        }

    });
}

// TODO
fn handle_corners(corner: Compass, buf: &mut Buffer) -> DirMapping {

    DirMapping { coord: (1, 1), box_char: SP }
}

fn determine_box_join(compass: Compass, re: &RectEdges, buf: &mut Buffer) -> (HashSet<Compass>, char) {
    let mut box_char = SP;
    let mut corners = HashSet::new();

    let (u, r, d, l, c) = (compass.top, compass.right, compass.bottom, compass.left, compass.centre);

    // log!(Level::Info, "c:{:?} \t r:{:?}", c, r);

    if BOX_DRAWING.contains(&c.box_char) {

        if re.is_between_left(c.coord) {
            // drawing left edge of rectangle toward a vertical edge
            if c.coord == re.rect.top_left().pair() || c.coord == re.rect.bottom_left().pair() {
                corners.insert(compass);
            } 
            // left edge of rectangle being drawn intersects another rectangle's left corners
            else if [TLCORN, BLCORN, LHINTER].contains(&c.box_char) {
                box_char = LHINTER;
            }
            // left edge of rectangle being drawn intersects another rectangle's right corners
            else if [TRCORN, BRCORN, RHINTER].contains(&c.box_char) {
                box_char = RHINTER;
            }
        }

        else if re.is_between_right(c.coord) {
            // drawing right edge of rectangle toward a vertical edge
            if c.coord == re.rect.top_right().pair() || c.coord == re.rect.bottom_right().pair() {
                corners.insert(compass);
            }
            // right edge of rectangle being drawn intersects another rectangle's right corners
            else if [TRCORN, BRCORN, RHINTER].contains(&c.box_char) {
                box_char = RHINTER;
            }

            // right edge of rectangle being drawn intersects another rectangle's left corners
            else if [TLCORN, BLCORN, LHINTER].contains(&c.box_char) {
                box_char = LHINTER;
            }
        }

        if re.is_between_top(c.coord) {
            // drawing top edge of rectangle toward a horizontal edge
            if c.coord == re.rect.top_left().pair() || c.coord == re.rect.top_right().pair() {
                corners.insert(compass);
            }
            // top edge of rectangle being drawn intersects another rectangle's top corners
            else if [TLCORN, TRCORN, TVINTER].contains(&c.box_char) {
                box_char = TVINTER;
            }

            // top edge of rectangle being drawn intersects another rectangle's bottom corners
            else if [BLCORN, BRCORN, BVINTER].contains(&c.box_char) {
                box_char = BVINTER;
            }
        }

        else if re.is_between_bottom(c.coord) {
            // drawing bottom edge of rectangle toward a horizontal edge
            if c.coord == re.rect.bottom_left().pair() || c.coord == re.rect.bottom_right().pair() {
                corners.insert(compass);
            }
            // bottom edge of rectangle being drawn intersects another rectangle's bottom corners
            else if [BLCORN, BRCORN, BVINTER].contains(&c.box_char) {
                box_char = BVINTER;
            }

            // top edge of rectangle being drawn intersects another rectangle's bottom corners
            else if [TLCORN, TRCORN, TVINTER].contains(&c.box_char) {
                box_char = TVINTER;
            }
        }

    }

    (corners, box_char)
}


#[derive(Hash, PartialEq, Clone, Copy)]
struct DirMapping {
    coord: (usize, usize),
    box_char: char,
}

#[derive(Hash, PartialEq, Clone, Copy)]
struct Compass {
    centre: DirMapping,
    top: DirMapping,
    right: DirMapping,
    bottom: DirMapping,
    left: DirMapping,
}

impl Compass {
    fn new (centre: (usize, usize), buf: &mut Buffer) -> Self {
        let n = |(x, y): (usize, usize)| (x, y - 1);
        let e = |(x, y): (usize, usize)| (x + 1, y);
        let s = |(x, y): (usize, usize)| (x, y + 1);
        let w = |(x, y): (usize, usize)| (x - 1, y);

        let (u, r, d, l) = (n(centre), e(centre), s(centre), w(centre));

        Compass { 
            centre: DirMapping { coord: centre, box_char: (get_coord_safely(centre, buf)) },
            top: DirMapping { coord: u, box_char: (get_coord_safely(u, buf)) },
            right: DirMapping { coord: r, box_char: (get_coord_safely(r, buf)) }, 
            bottom: DirMapping { coord: d, box_char: (get_coord_safely(d, buf)) }, 
            left: DirMapping { coord: l, box_char: (get_coord_safely(l, buf)) }
        }
    }
}

impl Eq for Compass {}

// get a coordinate from the buffer safely - return ' ' if unsuccessful otherwise, return the
// char at the coordinate
fn get_coord_safely(coord: (usize, usize), buf: &mut Buffer) -> char {

    if !buf.visible(coord.into()) {
        return SP
    } 

    let c = buf.getv(coord.into()).unwrap();

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
 * [x] update function to handle the left and right intersections of the box
 * [x] update function to handle the top and bottom intersections of the box
 * [ ] update function to handle centre intersections of the box
 * [] do precedence setting?
 *   is the box that is being moved have full precedence?
 *   should each tool implement a precedence function that the buffer uses?
 */
