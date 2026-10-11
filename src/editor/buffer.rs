use cursive::{Rect, Vec2};
use std::{
    cmp::{max, min},
    io::{self, BufRead, BufReader, Read},
    iter,
};

use crate::config::Symbols;
use crate::constants::SP;
use crate::editor::cell::{Cell, Char};

#[derive(Clone, Default, PartialEq, Eq)]
pub struct Buffer {
    pub(crate) chars: Vec<Vec<char>>,
    pub(crate) edits: Vec<Cell>,
    pub(crate) cursor: Option<Vec2>,
}

impl Buffer {
    pub(crate) fn document_eq(&self, other: &Self) -> bool {
        self.chars == other.chars
    }

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

    /// copy committed rows without pending edits or the cursor
    pub fn snapshot(&self) -> Self {
        Self {
            chars: self.chars.clone(),
            edits: vec![],
            cursor: None,
        }
    }

    pub fn set_cursor(&mut self, pos: Vec2) {
        self.cursor = Some(pos);
    }

    pub fn get_cursor(&self) -> Option<Vec2> {
        self.cursor
    }

    pub fn clear(&mut self) {
        self.chars.clear();
        self.edits.clear();
        self.cursor = None;
    }

    /// bound nonblank committed cells, nonblank previews and the cursor
    pub fn bounds(&self) -> Vec2 {
        let mut bounds = Vec2::new(0, 0);

        for (y, line) in self.chars.iter().enumerate() {
            if let Some(x) = line.iter().rposition(|&c| !c.is_whitespace()) {
                bounds.x = max(bounds.x, x + 1);
                bounds.y = max(bounds.y, y + 1);
            }
        }

        for Cell { pos, c } in &self.edits {
            if !c.is_whitespace() {
                bounds.x = max(bounds.x, pos.x + 1);
                bounds.y = max(bounds.y, pos.y + 1);
            }
        }

        if let Some(Vec2 { x, y }) = self.cursor {
            bounds.x = max(bounds.x, x + 1);
            bounds.y = max(bounds.y, y + 1);
        }

        bounds
    }

    pub(crate) fn iter_within(
        &self,
        offset: Vec2,
        size: Vec2,
        symbols: &Symbols,
    ) -> impl Iterator<Item = Char> + '_ {
        let area = Rect::from_corners(offset, offset + size);

        let cursor_char = self
            .cursor
            .map(|pos| self.get_char_at(pos))
            .unwrap_or(symbols.curs);

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
                    .map(|pos| Cell {
                        pos,
                        c: cursor_char,
                    })
                    .map(Char::Cursor),
            )
    }

    pub fn get_char_at(&self, pos: Vec2) -> char {
        self.edits
            .iter()
            .rev()
            .find(|cell| cell.pos == pos)
            .map(|cell| cell.c)
            .or_else(|| self.getv(pos))
            .unwrap_or(SP)
    }

    pub(crate) fn iter<'a>(&'a self, prefix: &'a str) -> impl Iterator<Item = char> + 'a {
        self.chars.iter().flat_map(move |line| {
            prefix
                .chars()
                .chain(line.iter().copied())
                .chain(iter::once('\n'))
        })
    }

    pub fn strip_margin_whitespace(&mut self) {
        let is_only_ws = |v: &[char]| v.iter().all(|c| c.is_whitespace());

        let top = self.chars.iter().take_while(|row| is_only_ws(row)).count();
        self.chars.drain(..top);
        let bottom = self
            .chars
            .iter()
            .rposition(|row| !is_only_ws(row))
            .map_or(0, |y| y + 1);
        self.chars.truncate(bottom);

        if let Some(left) = self
            .chars
            .iter()
            .filter_map(|row| row.iter().position(|c| !c.is_whitespace()))
            .min()
        {
            for row in &mut self.chars {
                row.drain(..min(left, row.len()));
            }
        }

        self.strip_trailing_whitespace();
    }

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

    /// read a committed cell, ignoring pending edits
    pub fn getv(&self, pos: Vec2) -> Option<char> {
        self.chars.get(pos.y).and_then(|v| v.get(pos.x)).copied()
    }

    /// include pending edits when checking for a nonblank cell
    pub fn visible(&self, pos: Vec2) -> bool {
        !self.get_char_at(pos).is_whitespace()
    }

    pub fn setv(&mut self, force: bool, pos: Vec2, c: char, symbols: &Symbols) {
        if force {
            self.edits.push(Cell { pos, c });
            return;
        }

        let max_prec = precedence(c, symbols);
        let overrides = |_c| _c == c || precedence(_c, symbols) > max_prec;

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

    /// apply pending edits, growing rows as needed
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

    pub fn discard_edits(&mut self) {
        self.edits.clear();
    }
}

fn precedence(c: char, symbols: &Symbols) -> usize {
    if c == symbols.plus {
        5
    } else if c == symbols.dash {
        4
    } else if c == symbols.pipe {
        3
    } else if c == symbols.diag {
        2
    } else if c == symbols.gaid {
        1
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(buffer: &Buffer) -> String {
        buffer.iter("").collect()
    }

    #[test]
    fn unicode_and_ragged_lines_round_trip() {
        let buffer = Buffer::read_from("猫\nabc\n".as_bytes()).unwrap();
        assert_eq!(buffer.getv(Vec2::new(0, 0)), Some('猫'));
        assert_eq!(buffer.getv(Vec2::new(2, 1)), Some('c'));
        assert_eq!(text(&buffer), "猫\nabc\n");
    }

    #[test]
    fn preview_edits_can_be_discarded_or_flushed() {
        let symbols = Symbols::default();
        let mut buffer = Buffer::default();
        buffer.setv(true, Vec2::new(2, 1), 'x', &symbols);
        assert_eq!(buffer.get_char_at(Vec2::new(2, 1)), 'x');
        buffer.discard_edits();
        assert_eq!(buffer.get_char_at(Vec2::new(2, 1)), ' ');

        buffer.setv(true, Vec2::new(2, 1), 'x', &symbols);
        buffer.flush_edits();
        assert_eq!(text(&buffer), "\n  x\n");
    }

    #[test]
    fn trimming_handles_blank_margins_and_trailing_spaces() {
        let mut buffer = Buffer::read_from("   \n  a  \n  b \n   \n".as_bytes()).unwrap();
        buffer.strip_margin_whitespace();
        assert_eq!(text(&buffer), "a\nb\n");
    }

    #[test]
    fn stronger_existing_line_symbols_are_not_overwritten() {
        let symbols = Symbols::default();
        let mut buffer = Buffer::default();
        buffer.setv(false, Vec2::new(0, 0), symbols.plus, &symbols);
        buffer.setv(false, Vec2::new(0, 0), symbols.hline, &symbols);
        assert_eq!(buffer.get_char_at(Vec2::new(0, 0)), symbols.plus);
    }

    #[test]
    fn trimming_removes_the_full_left_margin_from_short_blank_rows() {
        let mut buffer = Buffer::read_from("  a\n \n  b\n".as_bytes()).unwrap();
        buffer.strip_margin_whitespace();
        assert_eq!(text(&buffer), "a\n\nb\n");
    }
}
