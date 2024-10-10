use cursive::{
    event::{
        Event, EventResult, MouseButton::Left,
        MouseEvent::{Hold, Press, Release},
    },
    Rect, Vec2
};

use line_drawing::Bresenham;
use std::fmt;

use crate::{
    implementations::rectedges::RectEdges,
    constants::{
        TLCORN, TRCORN, BLCORN, BRCORN, VLINE, HLINE,
        LHINTER, RHINTER, TVINTER, BVINTER, CINTER,
        UBOX, SP, CONSUMED
    },
    editor::{buffer::Buffer, scroll::EditorCtx}
};

use super::super::{Tool, simple_display, fn_on_event_drag, option, mouse_drag};

#[derive(Copy, Clone, Default)]
pub(crate) struct BoxTool {
    src: Option<Vec2>,
    dst: Option<Vec2>,
}

const BOX_DRAWING: [char; 11] = [BRCORN, BLCORN, TRCORN, TLCORN, VLINE, HLINE, LHINTER, RHINTER, BVINTER, TVINTER, CINTER];
const CONTINUE_LEFT_CHAR: [char; 7] = [BLCORN, TLCORN, HLINE, LHINTER, BVINTER, TVINTER, CINTER];
const CONTINUE_RIGHT_CHAR: [char; 7] = [BRCORN, TRCORN, HLINE, RHINTER, BVINTER, TVINTER, CINTER];
const CONTINUE_TOP_CHAR: [char; 7] = [TRCORN, TLCORN, VLINE, LHINTER, RHINTER, TVINTER, CINTER];
const CONTINUE_BOTTOM_CHAR: [char; 7] = [BRCORN, BLCORN, VLINE, LHINTER, RHINTER, BVINTER, CINTER];

simple_display! { BoxTool, "Box" }

impl Tool for BoxTool {
    fn_on_event_drag!(|t: &Self, buf: &mut Buffer| {
        let (src, dst) = option!(t.src, t.dst);

        let rect = Rect::from_corners(src, dst);
        let re = RectEdges::new(rect);

        if rect.top_left() == rect.top_right() || rect.bottom_left() == rect.bottom_right() || rect.top_left() == rect.bottom_left() {
            buf.set(false, src.x, src.y, UBOX);
            return
        }

        draw_line(buf, rect.top_left(), rect.top_right(), rect);
        draw_line(buf, rect.top_right(), rect.bottom_right(), rect);
        draw_line(buf, rect.bottom_right(), rect.bottom_left(), rect);
        draw_line(buf, rect.bottom_left(), rect.top_left(), rect);

        let mut change_set = Vec::new();

        for &(x, y) in &re.coordinate_outline {
            let pos = (x, y);
            let compass = Compass::new(pos, buf);

            let new_char = determine_box_join(compass, &re);

            if BOX_DRAWING.contains(&new_char) {
                change_set.push((pos, new_char));
            }
        }

        for cs in change_set.into_iter() {
            let (pos, c) = cs;

            buf.set(true, pos.0, pos.1, c);
        }
    });
}

// matching cases to determine which box character to replace for a corner
fn handle_corners(corner: Compass, re: &RectEdges) -> char {
    let mut ret_char = SP;
    let (u, r, d, l, c) = (corner.top, corner.right, corner.bottom, corner.left, corner.centre);
    let coord = c.coord.unwrap();

    let top_continue = CONTINUE_TOP_CHAR.contains(&u.box_char);
    let bottom_continue = CONTINUE_BOTTOM_CHAR.contains(&d.box_char);
    let left_continue = CONTINUE_LEFT_CHAR.contains(&l.box_char);
    let right_continue = CONTINUE_RIGHT_CHAR.contains(&r.box_char);

    let is_top_left = coord == re.rect.top_left().pair();
    let is_top_right = coord == re.rect.top_right().pair();
    let is_bottom_left = coord == re.rect.bottom_left().pair();
    let is_bottom_right = coord == re.rect.bottom_right().pair();

    if top_continue && bottom_continue && left_continue && right_continue {
        return CINTER;
    }

    match () {
        _ if is_top_left => {
            ret_char = match (top_continue, bottom_continue, left_continue, right_continue) {
                (true, true, true, _   ) => CINTER,
                (true, true, _   , true) => LHINTER,
                (true, _   , true, true) => BVINTER,
                (_   , true, true, true) => TVINTER,
                (true, true, _   , _   ) => LHINTER,
                (true, _   , true, _   ) => CINTER,
                (true, _   , _   , true) => LHINTER,
                (_   , true, true, _   ) => TVINTER,
                (_   , true, _   , true) => TLCORN,
                (_   , _   , true, true) => TVINTER,
                _ => ret_char,
            };
        }
        _ if is_bottom_left => {
            ret_char = match (top_continue, bottom_continue, left_continue, right_continue) {
                (true, true, true, _   ) => CINTER,
                (true, true, _   , true) => LHINTER,
                (true, _   , true, true) => BVINTER,
                (_   , true, true, true) => TVINTER,
                (true, true, _   , _   ) => LHINTER,
                (true, _   , true, _   ) => BVINTER,
                (true, _   , _   , true) => BLCORN,
                (_   , true, true, _   ) => CINTER,
                (_   , true, _   , true) => LHINTER,
                (_   , _   , true, true) => BVINTER,
                _ => ret_char,
            };
        }
        _ if is_top_right => {
            ret_char = match (top_continue, bottom_continue, left_continue, right_continue) {
                (true, true, true, _   ) => RHINTER,
                (true, true, _   , true) => CINTER,
                (true, _   , true, true) => BVINTER,
                (_   , true, true, true) => TVINTER,
                (true, true, _   , _   ) => RHINTER,
                (true, _   , true, _   ) => RHINTER,
                (true, _   , _   , true) => CINTER,
                (_   , true, true, _   ) => TRCORN,
                (_   , true, _   , true) => TVINTER,
                (_   , _   , true, true) => TVINTER,
                _ => ret_char,
            };
        }
        _ if is_bottom_right => {
            ret_char = match (top_continue, bottom_continue, left_continue, right_continue) {
                (true, true, true, _   ) => RHINTER,
                (true, true, _   , true) => CINTER,
                (true, _   , true, true) => BVINTER,
                (_   , true, true, true) => CINTER,
                (true, true, _   , _   ) => RHINTER,
                (true, _   , true, _   ) => BRCORN,
                (true, _   , _   , true) => BVINTER,
                (_   , true, true, _   ) => RHINTER,
                (_   , true, _   , true) => CINTER,
                (_   , _   , true, true) => BVINTER,
                _ => ret_char,
            };
        }
        _ => {}
    }

    ret_char
}

// determine the relevant box character to place in based on the corner and edge of the rectangle
fn determine_box_join(compass: Compass, re: &RectEdges) -> char {
    let mut box_char = SP;
    let (u, r, d, l, c) = (compass.top, compass.right, compass.bottom, compass.left, compass.centre);
    let coord = c.coord.unwrap();


    if BOX_DRAWING.contains(&c.box_char) {

        if re.is_corner(coord) {
            return handle_corners(compass, re);
        } 

        let intersects_top = re.is_between_top(coord);
        let intersects_bottom = re.is_between_bottom(coord);
        let intersects_left = re.is_between_left(coord);
        let intersects_right = re.is_between_right(coord);

        let top_corners = [TLCORN, TRCORN, TVINTER];
        let bottom_corners = [BLCORN, BRCORN, BVINTER];
        let left_corners = [TLCORN, BLCORN, LHINTER];
        let right_corners = [TRCORN, BRCORN, RHINTER];


        match () {
            _ if intersects_left =>  {
                box_char = match c.box_char {
                    // left edge of rectangle being drawn intersects another rectangle's left corners
                    box_char if left_corners.contains(&box_char) => LHINTER,
                    // left edge of rectangle being drawn intersects another rectangle's right corners
                    box_char if right_corners.contains(&box_char) => RHINTER,
                    _ if intersect_verticals(l.box_char, r.box_char, c.box_char) => CINTER,
                    _ => box_char,
                };
            },
            _ if intersects_right => {
                box_char = match c.box_char {
                    // right edge of rectangle being drawn intersects another rectangle's right corners
                    box_char if right_corners.contains(&box_char) => RHINTER,
                    // right edge of rectangle being drawn intersects another rectangle's left corners
                    box_char if left_corners.contains(&box_char) => LHINTER,
                    _ if intersect_verticals(l.box_char, r.box_char, c.box_char) => CINTER,
                    _ => box_char,
                };
            },
            _ if intersects_top => {
                box_char = match c.box_char {
                    // top edge of rectangle being drawn intersects another rectangle's top corners
                    box_char if top_corners.contains(&box_char) => TVINTER,
                    // top edge of rectangle being drawn intersects another rectangle's bottom corners
                    box_char if bottom_corners.contains(&box_char) => BVINTER,
                    _ if intersect_horizontals(u.box_char, d.box_char, c.box_char) => CINTER,
                    _ => box_char,
                };
            },
            _ if intersects_bottom => {
                box_char = match c.box_char {
                    // bottom edge of rectangle being drawn intersects another rectangle's bottom corners
                    box_char if bottom_corners.contains(&box_char) => BVINTER,
                    // top edge of rectangle being drawn intersects another rectangle's bottom corners
                    box_char if top_corners.contains(&box_char) => TVINTER,
                    _ if intersect_horizontals(u.box_char, d.box_char, c.box_char) => CINTER,
                    _ => box_char,
                };
            }
            _ => ()
        }
    }

    box_char
}

// all `l` matches connections and all `r` connections - or if `c` is already CINTER
// don't change it
fn intersect_verticals(l_char: char, r_char: char, c_char: char) -> bool {
    (CONTINUE_LEFT_CHAR.contains(&l_char) && CONTINUE_RIGHT_CHAR.contains(&r_char)) || c_char == CINTER
}

// all `u` matches connections and all `d` connections - or if `c` is already CINTER
// don't change it
fn intersect_horizontals(u_char: char, d_char: char, c_char: char) -> bool {
    (CONTINUE_TOP_CHAR.contains(&u_char) && CONTINUE_BOTTOM_CHAR.contains(&d_char)) || c_char == CINTER
}

#[derive(Hash, PartialEq, Clone, Copy, Debug)]
struct DirMapping {
    coord: Option<(usize, usize)>,
    box_char: char,
}

#[derive(Hash, PartialEq, Clone, Copy, Debug)]
struct Compass {
    centre: DirMapping,
    top: DirMapping,
    right: DirMapping,
    bottom: DirMapping,
    left: DirMapping,
}

impl Compass {
    fn new (centre: (usize, usize), buf: &mut Buffer) -> Self {
        let n = |(x, y): (usize, usize)| if y > 0 { Some((x, y - 1)) } else { None };
        let e = |(x, y): (usize, usize)| Some((x + 1, y)); // assuming x is always within bounds
        let s = |(x, y): (usize, usize)| Some((x, y + 1)); // assuming y is always within bounds
        let w = |(x, y): (usize, usize)| if x > 0 { Some((x - 1, y)) } else { None };

        let (u, r, d, l) = (
            n(centre),
            e(centre),
            s(centre),
            w(centre),
        );

        Compass {
            centre: DirMapping { coord: Some(centre), box_char: get_coord_safely(Some(centre), buf) },
            top: DirMapping { coord: u, box_char: get_coord_safely(u, buf) },
            right: DirMapping { coord: r, box_char: get_coord_safely(r, buf) }, 
            bottom: DirMapping { coord: d, box_char: get_coord_safely(d, buf) }, 
            left: DirMapping { coord: l, box_char: get_coord_safely(l, buf) }
        }
    }
}

impl Eq for Compass {}

// get a coordinate from the buffer safely - return ' ' if unsuccessful otherwise, return the
// char at the coordinate
fn get_coord_safely(coord: Option<(usize, usize)>, buf: &mut Buffer) -> char {
    let pos = match coord {
        Some(pos) if buf.visible(pos.into()) => pos,
        _ => return SP,
    };

    buf.getv(pos.into()).unwrap()
}

fn draw_line(buf: &mut Buffer, src: Vec2, dst: Vec2, r: Rect) {
    let steps = Bresenham::new(src.signed().pair(), dst.signed().pair()).steps();

    for (i, (a, _)) in steps.enumerate() {
        let c = match (i, src, dst) {
            (0, s, _) if s == r.top_left() => TLCORN,
            (0, s, _) if s == r.top_right() => TRCORN,
            (0, s, _) if s == r.bottom_left() => BLCORN,
            (0, s, _) if s == r.bottom_right() => BRCORN,
            (_, s, d) if i > 0 && i < d.x.abs_diff(s.x) => HLINE,
            (_, s, d) if i > 0 && i < d.y.abs_diff(s.y) => VLINE,
            _ => SP,
        };

        buf.set(false, a.0 as usize, a.1 as usize, c)
    }

}
