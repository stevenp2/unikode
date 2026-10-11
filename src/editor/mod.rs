pub(crate) mod buffer;
pub(crate) mod cell;
pub mod command;
mod document;
mod history;
pub(crate) mod operations;
mod persistence;

use cursive::Vec2;
use std::{fmt, io, path::PathBuf};

use self::operations::{erase_on_buffer, move_on_buffer};
use crate::config::Options;
use crate::core::Point;
use crate::drawing::{draw, DrawRequest, DrawingStyle, ShapeKind};
use crate::editor::buffer::Buffer;
use crate::editor::command::{Direction, EditorCommand};
use crate::editor::history::Transaction;
use crate::tools::{lines::BoxTool, Tool};

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

impl EditorMode {
    pub(crate) fn drawing(self) -> Option<(ShapeKind, Vec2)> {
        match self {
            Self::Box(start) => Some((ShapeKind::Box, start)),
            Self::Line(start) => Some((ShapeKind::Line, start)),
            Self::Arrow(start) => Some((ShapeKind::Arrow, start)),
            _ => None,
        }
    }
}

pub struct Editor {
    pub(crate) mode: EditorMode,
    pub(crate) pending_count: String,
    pub(crate) opts: Options,
    pub(crate) buffer: Buffer,
    lsave: Buffer,
    dirty: bool,
    undo_history: Vec<Transaction>,
    redo_history: Vec<Transaction>,
    pub(crate) active_tool: Option<Box<dyn Tool + Send + Sync>>,
    pub(crate) last_left_press: Option<Vec2>,
    pub(crate) pan_origin: Option<Vec2>,
}

/// Name used by headless consumers to emphasize that this value owns one
/// document's editing state and history.
pub type EditorSession = Editor;

pub use buffer::Buffer as Canvas;

impl Editor {
    /// Open an editor instance with the provided options.
    pub fn open(mut opts: Options) -> io::Result<Self> {
        let file = opts.file.take();

        let mut tool = BoxTool::default();
        tool.load_opts(&opts);

        let mut editor = Self {
            mode: EditorMode::Normal,
            pending_count: String::new(),
            opts,
            buffer: Buffer::default(),
            lsave: Buffer::default(),
            dirty: false,
            undo_history: vec![],
            redo_history: vec![],
            active_tool: Some(Box::new(tool)),
            last_left_press: None,
            pan_origin: None,
        };

        if let Some(path) = file {
            editor.open_file(path)?;
        }

        Ok(editor)
    }

    /// Mutate the loaded options with `apply`.
    pub(crate) fn mut_opts<F: FnOnce(&mut Options)>(&mut self, apply: F) {
        apply(&mut self.opts);
        if let Some(tool) = self.active_tool.as_mut() {
            tool.load_opts(&self.opts);
        }
    }

    /// Returns `true` if the buffer has been modified since the last save.
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Set the active tool.
    pub(crate) fn set_tool<T: Tool + 'static + Send + Sync>(&mut self, mut tool: T) {
        self.buffer.discard_edits();
        tool.load_opts(&self.opts);
        self.active_tool = Some(Box::new(tool));
    }

    /// Returns the active tool as a human readable string.
    pub(crate) fn active_tool(&self) -> String {
        format!("({})", self.active_tool.as_ref().unwrap())
    }

    /// Returns the current mode as a human readable string.
    pub(crate) fn mode_string(&self) -> String {
        format!("-- {} --", self.mode)
    }

    /// Returns the current save path.
    pub fn path(&self) -> Option<&PathBuf> {
        self.opts.file.as_ref()
    }

    /// Read-only access to the current canvas, primarily for rendering and tests.
    pub fn canvas(&self) -> &Buffer {
        &self.buffer
    }

    /// Current cursor position in headless coordinates.
    pub fn cursor(&self) -> Option<Point> {
        self.buffer.get_cursor().map(Point::from)
    }

    /// Clear all buffer state and begin a blank diagram.
    pub fn clear(&mut self) {
        self.opts.file = None;
        self.buffer.clear();
        self.lsave.clear();
        self.dirty = false;
        self.undo_history.clear();
        self.redo_history.clear();
    }

    /// trim margins and reset the cursor, with undo support
    pub fn trim_margins(&mut self) {
        self.execute(EditorCommand::TrimMargins);
    }

    pub(crate) fn with_snapshot<F: FnOnce(&mut Self)>(&mut self, apply: F) -> bool {
        let before = self.buffer.clone();
        self.buffer.discard_edits();

        apply(self);

        if let Some(transaction) = Transaction::between(&before, &self.buffer) {
            self.undo_history.push(transaction);
            self.redo_history.clear();
            self.dirty = !self.buffer.document_eq(&self.lsave);
            true
        } else {
            false
        }
    }

    /// Undo the last buffer modification.
    ///
    /// Returns `false` if there was nothing to undo.
    pub fn undo(&mut self) -> bool {
        if let Some(transaction) = self.undo_history.pop() {
            transaction.undo(&mut self.buffer);
            self.redo_history.push(transaction);
            self.dirty = !self.buffer.document_eq(&self.lsave);
            true
        } else {
            false
        }
    }

    /// Redo the last undone buffer modification.
    ///
    /// Returns `false` if there was nothing to redo.
    pub fn redo(&mut self) -> bool {
        if let Some(transaction) = self.redo_history.pop() {
            transaction.redo(&mut self.buffer);
            self.undo_history.push(transaction);
            self.dirty = !self.buffer.document_eq(&self.lsave);
            true
        } else {
            false
        }
    }

    /// Render the committed document as Unicode text without touching the
    /// filesystem or system clipboard.
    pub fn render_text(&self, prefix: &str) -> String {
        self.buffer.iter(prefix).collect()
    }

    /// execute a command and report whether committed content changed
    pub fn execute(&mut self, command: EditorCommand) -> bool {
        match command {
            EditorCommand::MoveCursor { direction, count } => {
                let count = count.max(1);
                let mut point = Point::from(self.buffer.get_cursor().unwrap_or_default());
                match direction {
                    Direction::Left => point.x = point.x.saturating_sub(count),
                    Direction::Right => point.x = point.x.saturating_add(count),
                    Direction::Up => point.y = point.y.saturating_sub(count),
                    Direction::Down => point.y = point.y.saturating_add(count),
                }
                self.buffer.set_cursor(point.into());
                false
            }
            EditorCommand::DrawBox { from, to } => self.commit_buffer_edit(|buffer, options| {
                draw(
                    buffer,
                    DrawRequest::box_(from.into(), to.into()),
                    &DrawingStyle::from_options(options),
                )
            }),
            EditorCommand::DrawLine { from, to } => self.commit_buffer_edit(|buffer, options| {
                draw(
                    buffer,
                    DrawRequest::line(from.into(), to.into()),
                    &DrawingStyle::from_options(options),
                )
            }),
            EditorCommand::DrawArrow { from, to } => self.commit_buffer_edit(|buffer, options| {
                draw(
                    buffer,
                    DrawRequest::arrow(from.into(), to.into()),
                    &DrawingStyle::from_options(options),
                )
            }),
            EditorCommand::Erase(region) => self.commit_buffer_edit(|buffer, options| {
                erase_on_buffer(
                    buffer,
                    region.min.into(),
                    region.max.into(),
                    &options.symbols,
                )
            }),
            EditorCommand::MoveSelection { region, anchor, to } => {
                self.commit_buffer_edit(|buffer, options| {
                    move_on_buffer(
                        buffer,
                        region.into(),
                        anchor.into(),
                        to.into(),
                        &options.symbols,
                    )
                })
            }
            EditorCommand::InsertText { at, text } => self.commit_buffer_edit(|buffer, options| {
                for (y, line) in text.split('\n').enumerate() {
                    for (x, character) in line.chars().enumerate() {
                        buffer.setv(
                            true,
                            Point::new(at.x + x, at.y + y).into(),
                            character,
                            &options.symbols,
                        );
                    }
                }
            }),
            EditorCommand::Undo => self.undo(),
            EditorCommand::Redo => self.redo(),
            EditorCommand::TrimMargins => self.with_snapshot(|editor| {
                editor.buffer.strip_margin_whitespace();
                editor.buffer.set_cursor(Vec2::zero());
            }),
        }
    }

    fn commit_buffer_edit<F>(&mut self, edit: F) -> bool
    where
        F: FnOnce(&mut Buffer, &Options),
    {
        self.with_snapshot(|editor| {
            edit(&mut editor.buffer, &editor.opts);
            editor.buffer.flush_edits();
        })
    }
}
