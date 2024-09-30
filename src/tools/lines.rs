pub mod arrowtool;
pub mod boxtool;
pub mod linetool;

use line_drawing::Bresenham;
use cursive::{Vec2, XY};
use pathfinding::directed::astar::astar;
use std::cmp::min;

use crate::editor::buffer::Buffer;
use crate::implementations::ordfloat::OrdFloat;
use crate::constants::{
    PLUS, PIPE, DASH, DIAG, GAID,
    D, D2
};

/// Draw the shortest path from `src` to `dst`. Returns the penultimate point
/// along that path.
fn draw_path(buf: &mut Buffer, src: Vec2, dst: Vec2) -> Vec2 {
    let mut path = astar(
        &src.pair(),
        |&pos| buf.neighbors(pos),
        |&pos| heuristic(pos.into(), dst),
        |&pos| pos == dst.pair(),
    )
    .map(|(points, _)| points)
    .unwrap()
    .into_iter()
    .map(Vec2::from)
    .enumerate()
    .peekable();

    let decide = |i: usize, last: Vec2, pos: Vec2| -> char {
        match line_slope(last, pos).pair() {
            _ if i == 0 => PLUS,
            (0, _) => PIPE,
            (_, 0) => DASH,
            (x, y) if (x > 0) == (y > 0) => GAID,
            _ => DIAG,
        }
    };

    let mut last = src;
    while let Some((i, pos)) = path.next() {
        let mut c = decide(i, last, pos);

        if let Some(next) = path.peek().map(|(i, next)| decide(*i, pos, *next)) {
            if c != PLUS && next != c {
                c = PLUS;
            }
            last = pos;
        }

        buf.setv(false, pos, c);
    }
    buf.setv(false, dst, PLUS);

    last
}

/// Draw a line from `src` to `dst`.
fn draw_line(buf: &mut Buffer, src: Vec2, dst: Vec2) {
    for (i, (s, e)) in Bresenham::new(src.signed().pair(), dst.signed().pair())
        .steps()
        .enumerate()
    {
        let c = match line_slope(s, e).pair() {
            _ if i == 0 => PLUS,
            (0, _) => PIPE,
            (_, 0) => DASH,
            (x, y) if (x > 0) == (y > 0) => GAID,
            _ => DIAG,
        };

        buf.set(false, s.0 as usize, s.1 as usize, c);
    }

    buf.setv(false, dst, PLUS);
}

fn snap45(src: Vec2, dst: Vec2) -> Vec2 {
    let delta = min(diff(src.y, dst.y), diff(src.x, dst.x));

    match line_slope(src, dst).pair() {
        // nw
        (x, y) if x < 0 && y < 0 => dst.map(|v| v + delta),
        // ne
        (x, y) if x > 0 && y < 0 => dst.map_x(|x| x - delta).map_y(|y| y + delta),
        // sw
        (x, y) if x < 0 && y > 0 => dst.map_x(|x| x + delta).map_y(|y| y - delta),
        // se
        (x, y) if x > 0 && y > 0 => dst.map(|v| v - delta),

        _ => src,
    }
}

fn snap90(buf: &mut Buffer, src: Vec2, dst: Vec2) -> Vec2 {
    if let Some(DASH) = buf.getv(dst) {
        Vec2::new(dst.x, src.y)
    } else {
        Vec2::new(src.x, dst.y)
    }
}

/// Returns the slope between points at `src` and `dst`.
///
/// The resulting fraction will be reduced to its simplest terms.
fn line_slope<P: Into<XY<isize>>>(src: P, dst: P) -> XY<isize> {
    let xy = dst.into() - src;

    match gcd(xy.x, xy.y) {
        0 => xy,
        d => xy / d,
    }
}

/// Returns the greatest common denominator between `a` and `b`.
fn gcd(mut a: isize, mut b: isize) -> isize {
    while b != 0 {
        let t = b;
        b = a % b;
        a = t;
    }
    a.abs()
}

/// Returns the absolute difference between `a` and `b`.
fn diff(a: usize, b: usize) -> usize {
    (a as isize - b as isize).abs().unsigned_abs()
}

/// Returns a distance heuristic between `pos` and `dst`.
fn heuristic(pos: Vec2, dst: Vec2) -> OrdFloat {
    // base is diagonal distance:
    // http://theory.stanford.edu/~amitp/GameProgramming/Heuristics.html#diagonal-distance
    let dx = (pos.x as f64 - dst.x as f64).abs();
    let dy = (pos.y as f64 - dst.y as f64).abs();

    let dist = if dx > dy {
        D * (dx - dy) + D2 * dy
    } else {
        D * (dy - dx) + D2 * dx
    };

    // prefer to expand paths close to dst:
    // http://theory.stanford.edu/~amitp/GameProgramming/Heuristics.html#breaking-ties
    const P: f64 = 1.0 + (1.0 / 1000.0);

    OrdFloat(dist * P)
}
