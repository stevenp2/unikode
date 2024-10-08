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

const CONTINUE_LEFT: [char; 7] = [BLCORN, TLCORN, HLINE, LHINTER, BVINTER, TVINTER, CINTER];
const CONTINUE_RIGHT: [char; 7] = [BRCORN, TRCORN, HLINE, RHINTER, BVINTER, TVINTER, CINTER];
const CONTINUE_TOP: [char; 7] = [TRCORN, TLCORN, VLINE, LHINTER, RHINTER, TVINTER, CINTER];
const CONTINUE_BOTTOM: [char; 7] = [BRCORN, BLCORN, VLINE, LHINTER, RHINTER, BVINTER, CINTER];

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

fn handle_corners(corner: Compass, re: &RectEdges) -> char {
    let mut ret_char = SP;
    let (u, r, d, l, c) = (corner.top, corner.right, corner.bottom, corner.left, corner.centre);

    if CONTINUE_TOP.contains(&u.box_char) && CONTINUE_BOTTOM.contains(&d.box_char) && CONTINUE_LEFT.contains(&l.box_char) && CONTINUE_RIGHT.contains(&r.box_char) {
        return CINTER
    }

    else if c.coord.unwrap() == re.rect.top_left().pair() {
        if CONTINUE_TOP.contains(&u.box_char) && CONTINUE_BOTTOM.contains(&d.box_char) && CONTINUE_RIGHT.contains(&r.box_char) {
            ret_char = LHINTER;
        }
        else if CONTINUE_TOP.contains(&u.box_char) && CONTINUE_BOTTOM.contains(&d.box_char) && CONTINUE_LEFT.contains(&l.box_char) {
            ret_char = CINTER;
        }
        else if CONTINUE_TOP.contains(&u.box_char) && CONTINUE_LEFT.contains(&l.box_char) && CONTINUE_RIGHT.contains(&r.box_char) {
            ret_char = BVINTER;
        }
        else if CONTINUE_BOTTOM.contains(&d.box_char) && CONTINUE_LEFT.contains(&l.box_char) && CONTINUE_RIGHT.contains(&r.box_char) {
            ret_char = TVINTER;
        }
        else if CONTINUE_TOP.contains(&u.box_char) && CONTINUE_BOTTOM.contains(&d.box_char) {
            ret_char = LHINTER;
        }
        else if CONTINUE_TOP.contains(&u.box_char) && CONTINUE_LEFT.contains(&l.box_char) {
            ret_char = CINTER;
        }
        else if CONTINUE_TOP.contains(&u.box_char) && CONTINUE_RIGHT.contains(&r.box_char) {
            ret_char = LHINTER;
        }
        else if CONTINUE_BOTTOM.contains(&d.box_char) && CONTINUE_LEFT.contains(&l.box_char) {
            ret_char = TVINTER;
        }
        else if CONTINUE_BOTTOM.contains(&d.box_char) && CONTINUE_RIGHT.contains(&r.box_char) {
            ret_char = TLCORN;
        }
        else if CONTINUE_LEFT.contains(&l.box_char) && CONTINUE_RIGHT.contains(&r.box_char) {
            ret_char = TVINTER;
        }
    } else if c.coord.unwrap() == re.rect.top_right().pair() {
        if CONTINUE_TOP.contains(&u.box_char) && CONTINUE_BOTTOM.contains(&d.box_char) && CONTINUE_LEFT.contains(&l.box_char) {
            ret_char = RHINTER;
        }
        else if CONTINUE_TOP.contains(&u.box_char) && CONTINUE_BOTTOM.contains(&d.box_char) && CONTINUE_RIGHT.contains(&r.box_char) {
            ret_char = CINTER;
        }
        else if CONTINUE_TOP.contains(&u.box_char) && CONTINUE_LEFT.contains(&l.box_char) && CONTINUE_RIGHT.contains(&r.box_char) {
            ret_char = BVINTER;
        }
        else if CONTINUE_BOTTOM.contains(&d.box_char) && CONTINUE_LEFT.contains(&l.box_char) && CONTINUE_RIGHT.contains(&r.box_char) {
            ret_char = TVINTER;
        }
        else if CONTINUE_TOP.contains(&u.box_char) && CONTINUE_BOTTOM.contains(&d.box_char) {
            ret_char = RHINTER;
        }
        else if CONTINUE_TOP.contains(&u.box_char) && CONTINUE_LEFT.contains(&l.box_char) {
            ret_char = RHINTER;
        }
        else if CONTINUE_TOP.contains(&u.box_char) && CONTINUE_RIGHT.contains(&r.box_char) {
            ret_char = CINTER;
        }
        else if CONTINUE_BOTTOM.contains(&d.box_char) && CONTINUE_RIGHT.contains(&r.box_char) {
            ret_char = TVINTER;
        }
        else if CONTINUE_BOTTOM.contains(&d.box_char) && CONTINUE_LEFT.contains(&l.box_char) {
            ret_char = TRCORN;
        }
        else if CONTINUE_LEFT.contains(&l.box_char) && CONTINUE_RIGHT.contains(&r.box_char) {
            ret_char = TVINTER;
        }
    } 
    else if c.coord.unwrap() == re.rect.bottom_left().pair() {
        if CONTINUE_TOP.contains(&u.box_char) && CONTINUE_BOTTOM.contains(&d.box_char) && CONTINUE_RIGHT.contains(&r.box_char) {
            ret_char = LHINTER;
        }
        else if CONTINUE_TOP.contains(&u.box_char) && CONTINUE_BOTTOM.contains(&d.box_char) && CONTINUE_LEFT.contains(&l.box_char) {
            ret_char = CINTER;
        }
        else if CONTINUE_TOP.contains(&u.box_char) && CONTINUE_LEFT.contains(&l.box_char) && CONTINUE_RIGHT.contains(&r.box_char) {
            ret_char = BVINTER;
        }
        else if CONTINUE_BOTTOM.contains(&u.box_char) && CONTINUE_LEFT.contains(&l.box_char) && CONTINUE_RIGHT.contains(&r.box_char) {
            ret_char = TVINTER;
        }
        else if CONTINUE_TOP.contains(&u.box_char) && CONTINUE_RIGHT.contains(&r.box_char) {
            ret_char = BLCORN;
        }
        else if CONTINUE_TOP.contains(&u.box_char) && CONTINUE_BOTTOM.contains(&d.box_char) {
            ret_char = LHINTER;
        }
        else if CONTINUE_TOP.contains(&u.box_char) && CONTINUE_LEFT.contains(&l.box_char) {
            ret_char = BVINTER;
        }

        else if CONTINUE_LEFT.contains(&l.box_char) && CONTINUE_BOTTOM.contains(&d.box_char) {
            ret_char = CINTER;
        }

        else if CONTINUE_LEFT.contains(&l.box_char) && CONTINUE_RIGHT.contains(&r.box_char) {
            ret_char = BVINTER;
        }

        else if CONTINUE_RIGHT.contains(&r.box_char) && CONTINUE_BOTTOM.contains(&d.box_char) {
            ret_char = LHINTER;
        }
    } 
    else if c.coord.unwrap() == re.rect.bottom_right().pair() {
        if CONTINUE_TOP.contains(&u.box_char) && CONTINUE_BOTTOM.contains(&d.box_char) && CONTINUE_LEFT.contains(&l.box_char) {
            ret_char = RHINTER;
        }
        else if CONTINUE_TOP.contains(&u.box_char) && CONTINUE_BOTTOM.contains(&d.box_char) && CONTINUE_RIGHT.contains(&r.box_char) {
            ret_char = CINTER;
        }
        else if CONTINUE_TOP.contains(&u.box_char) && CONTINUE_RIGHT.contains(&r.box_char) && CONTINUE_LEFT.contains(&l.box_char) {
            ret_char = BVINTER;
        }
        else if CONTINUE_BOTTOM.contains(&u.box_char) && CONTINUE_RIGHT.contains(&r.box_char) && CONTINUE_LEFT.contains(&l.box_char) {
            ret_char = CINTER;
        }
        else if CONTINUE_TOP.contains(&u.box_char) && CONTINUE_RIGHT.contains(&r.box_char) {
            ret_char = BVINTER;
        }
        else if CONTINUE_TOP.contains(&u.box_char) && CONTINUE_BOTTOM.contains(&d.box_char) {
            ret_char = RHINTER;
        }
        else if CONTINUE_TOP.contains(&u.box_char) && CONTINUE_LEFT.contains(&l.box_char) {
            ret_char = BRCORN;
        }

        else if CONTINUE_LEFT.contains(&l.box_char) && CONTINUE_BOTTOM.contains(&d.box_char) {
            ret_char = RHINTER;
        }

        else if CONTINUE_LEFT.contains(&l.box_char) && CONTINUE_RIGHT.contains(&r.box_char) {
            ret_char = BVINTER;
        }

        else if CONTINUE_RIGHT.contains(&r.box_char) && CONTINUE_BOTTOM.contains(&d.box_char) {
            ret_char = CINTER;
        }
    }

    ret_char
}

fn determine_box_join(compass: Compass, re: &RectEdges) -> char {
    let mut box_char = SP;

    let (u, r, d, l, c) = (compass.top, compass.right, compass.bottom, compass.left, compass.centre);

    if BOX_DRAWING.contains(&c.box_char) {

        if re.is_corner(c.coord.unwrap()) {
            box_char = handle_corners(compass, re);
        } else {
            if re.is_between_left(c.coord.unwrap()) {
                // left edge of rectangle being drawn intersects another rectangle's left corners
                if [TLCORN, BLCORN, LHINTER].contains(&c.box_char) {
                    box_char = LHINTER;
                }
                // left edge of rectangle being drawn intersects another rectangle's right corners
                else if [TRCORN, BRCORN, RHINTER].contains(&c.box_char) {
                    box_char = RHINTER;
                }

                else if intersect_verticals(l.box_char, r.box_char, c.box_char) {
                    box_char = CINTER;
                }
            }

            else if re.is_between_right(c.coord.unwrap()) {
                // right edge of rectangle being drawn intersects another rectangle's right corners
                if [TRCORN, BRCORN, RHINTER].contains(&c.box_char) {
                    box_char = RHINTER;
                }

                // right edge of rectangle being drawn intersects another rectangle's left corners
                else if [TLCORN, BLCORN, LHINTER].contains(&c.box_char) {
                    box_char = LHINTER;
                }

                else if intersect_verticals(l.box_char, r.box_char, c.box_char) {
                    box_char = CINTER;
                }
            }

            else if re.is_between_top(c.coord.unwrap()) {
                // top edge of rectangle being drawn intersects another rectangle's top corners
                if [TLCORN, TRCORN, TVINTER].contains(&c.box_char) {
                    box_char = TVINTER;
                }

                // top edge of rectangle being drawn intersects another rectangle's bottom corners
                else if [BLCORN, BRCORN, BVINTER].contains(&c.box_char) {
                    box_char = BVINTER;
                }

                else if intersect_horizontals(u.box_char, d.box_char, c.box_char) {
                    box_char = CINTER;
                }
            }

            else if re.is_between_bottom(c.coord.unwrap()) {
                // bottom edge of rectangle being drawn intersects another rectangle's bottom corners
                if [BLCORN, BRCORN, BVINTER].contains(&c.box_char) {
                    box_char = BVINTER;
                }

                // top edge of rectangle being drawn intersects another rectangle's bottom corners
                else if [TLCORN, TRCORN, TVINTER].contains(&c.box_char) {
                    box_char = TVINTER;
                }

                else if intersect_horizontals(u.box_char, d.box_char, c.box_char) {
                    box_char = CINTER;
                }
            }
        }

    }

    box_char
}

// all `l` matches connections and all `r` connections - or if `c` is already CINTER
// don't change it
fn intersect_verticals(l_char: char, r_char: char, c_char: char) -> bool {
    (CONTINUE_LEFT.contains(&l_char) && CONTINUE_RIGHT.contains(&r_char)) || c_char == CINTER
}

// all `u` matches connections and all `d` connections - or if `c` is already CINTER
// don't change it
fn intersect_horizontals(u_char: char, d_char: char, c_char: char) -> bool {
    (CONTINUE_TOP.contains(&u_char) && CONTINUE_BOTTOM.contains(&d_char)) || c_char == CINTER
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
    if coord.is_none() {
        return SP;
    }

    let pos = coord.unwrap();

    if !buf.visible(pos.into()) {
        return SP
    } 

    buf.getv(pos.into()).unwrap()
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
            _ => SP,
        };

        buf.set(false, a.0 as usize, a.1 as usize, c)
    }

}
