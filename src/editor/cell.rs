use cursive::Vec2;

#[derive(Copy, Clone, PartialEq, Eq)]
pub(crate) struct Cell {
    pub pos: Vec2,
    pub c: char,
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub(crate) enum Char {
    Clean(Cell),
    Dirty(Cell),
    Cursor(Cell),
}
