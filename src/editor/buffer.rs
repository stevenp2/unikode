use core::ops::Add;
use cursive::{
    Rect, Vec2, XY,
};
use line_drawing::Bresenham;
use num_traits::Zero;
use pathfinding::directed::astar::astar;
use std::{
    cmp::{max, min},
    io::{self, BufRead, BufReader, Read},
    iter, mem,
};

use crate::editor::cell::*;

use super::{
    CURS, SP, PLUS, PIPE, DASH, DIAG, GAID,
    S_N, S_E, S_S, S_W,
    N, E, S, W,
    D, D2,
};

#[derive(Clone, Default, PartialEq, Eq)]
pub(crate) struct Buffer {
    chars: Vec<Vec<char>>,
    edits: Vec<Cell>,
    cursor: Option<Vec2>,
}

impl Buffer {
    pub fn read_from<R: Read>(r: R) -> io::Result<Self> {
        Ok(Self {
            chars: BufReader::new(r)
                .lines()
                .map(|lr| lr.map(|s| s.chars().collect()))
                .collect::<io::Result<_>>()?,
            edits: vec![],
            cursor: None,
        })
    }

    /// Returns a copy of this buffer without any pending edits.
    pub fn snapshot(&self) -> Self {
        Self {
            chars: self.chars.clone(),
            edits: vec![],
            cursor: None,
        }
    }

    /// Set the cursor position to `pos`.
    pub(crate) fn set_cursor(&mut self, pos: Vec2) {
        self.cursor = Some(pos);
    }

    /// Disable the cursor.
    pub fn drop_cursor(&mut self) {
        self.cursor = None;
    }

    /// Clears all content in the buffer.
    pub fn clear(&mut self) {
        self.chars.clear();
        self.edits.clear();
        self.cursor = None;
    }

    /// Returns the viewport size required to display all content within the buffer.
    pub fn bounds(&self) -> Vec2 {
        let mut bounds = Vec2 {
            x: self.chars.iter().map(Vec::len).max().unwrap_or(0),
            y: self.chars.len(),
        };

        bounds.x = max(
            bounds.x,
            self.edits
                .iter()
                .map(|Cell { pos, .. }| pos.x + 1)
                .max()
                .unwrap_or(0),
        );

        bounds.y = max(
            bounds.y,
            self.edits
                .iter()
                .map(|Cell { pos, .. }| pos.y + 1)
                .max()
                .unwrap_or(0),
        );

        if let Some(Vec2 { x, y }) = self.cursor {
            bounds.x = max(bounds.x, x + 1);
            bounds.y = max(bounds.y, y + 1);
        }

        bounds
    }

    /// Returns an iterator over all characters within the viewport formed by `offset`
    /// and `size`.
    pub(crate) fn iter_within(
        &self,
        offset: Vec2,
        size: Vec2,
    ) -> impl Iterator<Item = Char> + '_ {
        let area = Rect::from_corners(offset, offset + size);

        self.chars
            .iter()
            .enumerate()
            .skip(offset.y)
            .take(size.y)
            .flat_map(move |(y, xs)| {
                xs.iter()
                    .copied()
                    .enumerate()
                    .skip(offset.x)
                    .take(size.x)
                    .map(move |(x, c)| (Vec2::new(x, y), c))
                    .map(|(pos, c)| Cell { pos, c })
                    .map(Char::Clean)
            })
            .chain(
                self.edits
                    .iter()
                    .copied()
                    .filter(move |Cell { pos, .. }| area.contains(*pos))
                    .map(Char::Dirty),
            )
            .chain(
                self.cursor
                    .map(|pos| Cell { pos, c: CURS })
                    .map(Char::Cursor),
            )
    }

    /// Returns an iterator over all characters in the buffer, injecting newlines
    /// where appropriate, with `prefix` before each line.
    pub fn iter<'a>(&'a self, prefix: &'a str) -> impl Iterator<Item = char> + 'a {
        self.chars.iter().flat_map(move |line| {
            prefix
                .chars()
                .chain(line.iter().copied())
                .chain(iter::once('\n'))
        })
    }

    /// Strip margin whitespace from the buffer.
    pub fn strip_margin_whitespace(&mut self) {
        let is_only_ws = |v: &[char]| v.iter().all(|c| c.is_whitespace());

        // upper margin
        for _ in 0..self
            .chars
            .iter()
            .take_while(|line| is_only_ws(line))
            .count()
        {
            self.chars.remove(0);
        }

        // lower margin
        for _ in 0..self
            .chars
            .iter()
            .rev()
            .take_while(|line| is_only_ws(line))
            .count()
        {
            self.chars.pop();
        }

        // left margin
        if let Some(min_ws) = self
            .chars
            .iter()
            .filter(|line| !is_only_ws(line))
            .map(|line| line.iter().position(|c| !c.is_whitespace()))
            .min()
            .flatten()
        {
            for line in self.chars.iter_mut() {
                if line.is_empty() {
                    continue;
                }
                let idx = min(line.len() - 1, min_ws);
                let new = line.split_off(idx);
                let _ = mem::replace(line, new);
            }
        }

        // right margin
        self.strip_trailing_whitespace();
    }

    /// Strip trailing whitespace from the buffer.
    pub fn strip_trailing_whitespace(&mut self) {
        for line in self.chars.iter_mut() {
            let idx = line
                .iter()
                .enumerate()
                .rfind(|p| !p.1.is_whitespace())
                .map(|p| p.0 + 1)
                .unwrap_or(0);

            line.truncate(idx);
        }
    }

    /// Get the cell at `pos`, if it exists.
    ///
    /// Does not consider any pending edits.
    pub(crate) fn getv(&self, pos: Vec2) -> Option<char> {
        self.chars.get(pos.y).and_then(|v| v.get(pos.x)).copied()
    }

    /// Returns `true` iff the cell at `pos` exists and contains a non-whitespace
    /// character.
    ///
    /// Does not consider any pending edits.
    pub(crate) fn visible(&self, pos: Vec2) -> bool {
        self.getv(pos).map(|c| !c.is_whitespace()).unwrap_or(false)
    }

    /// Set the cell at `pos` to `c`.
    pub(crate) fn setv(&mut self, force: bool, pos: Vec2, c: char) {
        if force {
            self.edits.push(Cell { pos, c });
            return;
        }

        let max_prec = precedence(c);
        let overrides = |_c| _c == c || precedence(_c) > max_prec;

        let mut overridden = false;
        if self.chars.len() > pos.y && self.chars[pos.y].len() > pos.x {
            overridden |= overrides(self.chars[pos.y][pos.x]);
        }

        overridden |= self
            .edits
            .iter()
            .filter(|cell| cell.pos == pos)
            .any(|cell| overrides(cell.c));

        if !overridden {
            self.edits.push(Cell { pos, c });
        }
    }

    /// Set the cell at `(x, y)` to `c`.
    pub(crate) fn set(&mut self, force: bool, x: usize, y: usize, c: char) {
        self.setv(force, Vec2::new(x, y), c)
    }

    /// Flush any pending edits to the primary buffer, allocating as necessary.
    pub fn flush_edits(&mut self) {
        for Cell {
            pos: Vec2 { x, y },
            c,
            ..
        } in self.edits.drain(..)
        {
            if self.chars.len() <= y {
                self.chars.resize_with(y + 1, Vec::default);
            }
            if self.chars[y].len() <= x {
                self.chars[y].resize(x + 1, SP);
            }
            self.chars[y][x] = c;
        }
    }

    /// Discard any pending edits.
    pub fn discard_edits(&mut self) {
        self.edits.clear();
    }

    /// Draw a line from `src` to `dst`.
    pub(crate) fn draw_line(&mut self, src: Vec2, dst: Vec2) {
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

            self.set(false, s.0 as usize, s.1 as usize, c);
        }

        self.setv(false, dst, PLUS);
    }

    /// Draw an arrow tip for an arrow from `src` to `dst`.
    pub(crate) fn draw_arrow_tip(&mut self, src: Vec2, dst: Vec2) {
        let dec = |v: usize| v - 1;
        let inc = |v: usize| v + 1;

        let north = dst.y > 0 && self.visible(dst.map_y(dec));
        let east = self.visible(dst.map_x(inc));
        let south = self.visible(dst.map_y(inc));
        let west = dst.x > 0 && self.visible(dst.map_x(dec));

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
            (x, y) if x > 0 && y > 0 && self.visible(dst.map_x(inc)) => E,
            (x, y) if x > 0 && y > 0 => S,

            // NE
            (x, y) if x > 0 && y < 0 && self.visible(dst.map_x(inc)) => E,
            (x, y) if x > 0 && y < 0 => N,

            // SW
            (x, y) if x < 0 && y > 0 && dst.x == 0 => S,
            (x, y) if x < 0 && y > 0 && self.visible(dst.map_x(dec)) => W,
            (x, y) if x < 0 && y > 0 => S,

            // NW
            (x, y) if x < 0 && y < 0 && dst.x == 0 => N,
            (x, y) if x < 0 && y < 0 && self.visible(dst.map_x(dec)) => W,
            (x, y) if x < 0 && y < 0 => N,

            (_, _) => PLUS,
        };

        self.setv(true, dst, tip);
    }

    pub(crate) fn snap45(&self, src: Vec2, dst: Vec2) -> Vec2 {
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

    pub(crate) fn snap90(&self, src: Vec2, dst: Vec2) -> Vec2 {
        if let Some(DASH) = self.getv(dst) {
            Vec2::new(dst.x, src.y)
        } else {
            Vec2::new(src.x, dst.y)
        }
    }

    /// Draw the shortest path from `src` to `dst`. Returns the penultimate point
    /// along that path.
    pub(crate) fn draw_path(&mut self, src: Vec2, dst: Vec2) -> Vec2 {
        let mut path = astar(
            &src.pair(),
            |&pos| self.neighbors(pos),
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

            self.setv(false, pos, c);
        }
        self.setv(false, dst, PLUS);

        last
    }

    /// Returns the coordinates neighboring `pos`, along with the cost to reach each one.
    fn neighbors(&self, pos: (usize, usize)) -> Vec<((usize, usize), OrdFloat)> {
        let vis = |pos: (usize, usize)| self.visible(pos.into());

        let card = |pos| (pos, OrdFloat((vis(pos) as u8 as f64 * 64.0) + D));

        let diag = |pos, (c1, c2)| {
            let cost = {
                let spot = vis(pos) as u8 as f64 * 64.0;
                let edge = (vis(c1) && vis(c2)) as u8 as f64 * 64.0;
                spot + edge
            };

            (pos, OrdFloat(cost + D2))
        };

        let w = |(x, y)| (x - 1, y);
        let n = |(x, y)| (x, y - 1);
        let e = |(x, y)| (x + 1, y);
        let s = |(x, y)| (x, y + 1);

        let mut succ = Vec::with_capacity(8);
        if pos.0 > 0 && pos.1 > 0 {
            succ.push(diag(n(w(pos)), (n(pos), w(pos))));
        }
        if pos.0 > 0 {
            succ.push(card(w(pos)));
            succ.push(diag(s(w(pos)), (s(pos), w(pos))));
        }
        if pos.1 > 0 {
            succ.push(card(n(pos)));
            succ.push(diag(n(e(pos)), (n(pos), e(pos))));
        }
        succ.push(card(e(pos)));
        succ.push(card(s(pos)));
        succ.push(diag(s(e(pos)), (s(pos), e(pos))));

        succ
    }

    // Returns cursor
    pub fn get_cursor(&self) -> Option<Vec2> {
        self.cursor
    }
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

/// Returns the overlap precedence for `c`.
fn precedence(c: char) -> usize {
    match c {
        PLUS => 5,
        DASH => 4,
        PIPE => 3,
        DIAG => 2,
        GAID => 1,
        _ => 0,
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

#[derive(PartialEq, Copy, Clone)]
struct OrdFloat(f64);

impl Eq for OrdFloat {}

impl Ord for OrdFloat {
    #[inline]
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.partial_cmp(other).unwrap_or(std::cmp::Ordering::Equal)
    }

    #[inline]
    fn max(self, other: Self) -> Self
        where
            Self: Sized, {
        if self > other {
            self
        } else {
            other
        }
    }

    fn min(self, other: Self) -> Self
        where
            Self: Sized, {
        if self < other {
            self
        } else {
            other
        }
    }

    fn clamp(self, min: Self, max: Self) -> Self
        where
            Self: Sized, {
        self.max(min).min(max)
    }
}

/// Returns the absolute difference between `a` and `b`.
fn diff(a: usize, b: usize) -> usize {
    (a as isize - b as isize).abs().unsigned_abs()
}

impl PartialOrd for OrdFloat {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Add<Self> for OrdFloat {
    type Output = Self;

    #[inline]
    fn add(self, other: Self) -> Self {
        Self(self.0 + other.0)
    }
}

impl Zero for OrdFloat {
    #[inline]
    fn zero() -> Self {
        Self(0.0)
    }

    #[inline]
    fn is_zero(&self) -> bool {
        self.0 == 0.0
    }
}
