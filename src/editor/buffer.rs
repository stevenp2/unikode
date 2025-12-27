use cursive::{Rect, Vec2,};
use std::{
    cmp::{max, min},
    io::{self, BufRead, BufReader, Read},
    iter, mem,
};

use crate::constants::{
    D, D2,
    CURS, SP, PLUS, PIPE, DASH, DIAG, GAID,
};
use crate::editor::cell::{Cell, Char};
use crate::utils::ordfloat::OrdFloat;

#[derive(Clone, Default, PartialEq, Eq)]
pub(crate) struct Buffer {
    pub chars: Vec<Vec<char>>,
    pub edits: Vec<Cell>,
    pub cursor: Option<Vec2>,
}

impl Buffer {
    pub(crate) fn read_from<R: Read>(r: R) -> io::Result<Self> {
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
    pub(crate) fn snapshot(&self) -> Self {
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
    pub(crate) fn drop_cursor(&mut self) {
        self.cursor = None;
    }

    /// Clears all content in the buffer.
    pub(crate) fn clear(&mut self) {
        self.chars.clear();
        self.edits.clear();
        self.cursor = None;
    }

    /// Returns the viewport size required to display all content within the buffer.
    pub(crate) fn bounds(&self) -> Vec2 {
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
    pub(crate) fn iter<'a>(&'a self, prefix: &'a str) -> impl Iterator<Item = char> + 'a {
        self.chars.iter().flat_map(move |line| {
            prefix
                .chars()
                .chain(line.iter().copied())
                .chain(iter::once('\n'))
        })
    }

    /// Strip margin whitespace from the buffer.
    pub(crate) fn strip_margin_whitespace(&mut self) {
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
    pub(crate) fn strip_trailing_whitespace(&mut self) {
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
    pub(crate) fn flush_edits(&mut self) {
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
    pub(crate) fn discard_edits(&mut self) {
        self.edits.clear();
    }

    // Returns cursor
    pub(crate) fn get_cursor(&self) -> Option<Vec2> {
        self.cursor
    }

    /// Returns the coordinates neighboring `pos`, along with the cost to reach each one.
    pub(crate) fn neighbors(&mut self, pos: (usize, usize)) -> Vec<((usize, usize), OrdFloat)> {
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
}

/// Returns the overlap precedence for `c`.
/// Higher values mean the character is "stronger" and less likely to be overwritten
/// by other characters during non-forced updates.
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

