use crate::core::{Point, Region};

/// A cardinal cursor movement.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Direction {
    Left,
    Right,
    Up,
    Down,
}

/// Headless operations supported by an [`Editor`](super::Editor).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EditorCommand {
    MoveCursor {
        direction: Direction,
        count: usize,
    },
    DrawBox {
        from: Point,
        to: Point,
    },
    DrawLine {
        from: Point,
        to: Point,
    },
    DrawArrow {
        from: Point,
        to: Point,
    },
    InsertText {
        at: Point,
        text: String,
    },
    Erase(Region),
    MoveSelection {
        region: Region,
        anchor: Point,
        to: Point,
    },
    TrimMargins,
    Undo,
    Redo,
}
