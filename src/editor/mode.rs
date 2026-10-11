use cursive::Vec2;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorMode {
    Normal,
    Text,
    Box(Vec2),
    Line(Vec2),
    Arrow(Vec2),
    Select(Vec2),
}

impl fmt::Display for EditorMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EditorMode::Normal => write!(f, "NORMAL"),
            EditorMode::Text => write!(f, "TEXT"),
            EditorMode::Box(_) => write!(f, "BOX"),
            EditorMode::Line(_) => write!(f, "LINE"),
            EditorMode::Arrow(_) => write!(f, "ARROW"),
            EditorMode::Select(_) => write!(f, "SELECT"),
        }
    }
}
