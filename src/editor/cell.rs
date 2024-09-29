use cursive::{Vec2, XY};

#[derive(Copy, Clone, PartialEq, Eq)]
pub(crate) struct Cell {
    pub pos: Vec2,
    pub c: char,
}

impl Cell {
    pub(crate) fn pos(&self) -> Vec2 {
        self.pos
    }

    pub(crate) fn c(&self) -> char {
        self.c
    }

    pub(crate) fn is_whitespace(&self) -> bool {
        self.c.is_whitespace()
    }

    pub(crate) fn translate(mut self, by: XY<isize>) -> Self {
        self.pos = self.pos.saturating_add(by);
        self
    }
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub(crate) enum Char {
    Clean(Cell),
    Dirty(Cell),
    Cursor(Cell),
}
