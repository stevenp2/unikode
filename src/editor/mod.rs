pub(crate) mod buffer;
pub(crate) mod cell;
pub mod command;
mod document;
mod history;
mod mode;
pub(crate) mod operations;
mod persistence;

use cursive::Vec2;
use std::{io, path::PathBuf};

use self::operations::{erase_on_buffer, move_on_buffer};
use crate::config::Options;
use crate::core::Point;
use crate::drawing::{draw, DrawRequest, DrawingStyle};
use crate::editor::buffer::Buffer;
use crate::editor::command::{Direction, EditorCommand};
use crate::editor::history::Transaction;

pub use mode::EditorMode;

pub struct Editor {
    pub(crate) opts: Options,
    pub(crate) buffer: Buffer,
    lsave: Buffer,
    dirty: bool,
    undo_history: Vec<Transaction>,
    redo_history: Vec<Transaction>,
}

pub type EditorSession = Editor;

pub use buffer::Buffer as Canvas;

impl Editor {
    /// open the configured file or start a blank document
    pub fn open(mut opts: Options) -> io::Result<Self> {
        let file = opts.file.take();

        let mut editor = Self {
            opts,
            buffer: Buffer::default(),
            lsave: Buffer::default(),
            dirty: false,
            undo_history: vec![],
            redo_history: vec![],
        };

        if let Some(path) = file {
            editor.open_file(path)?;
        }

        Ok(editor)
    }

    /// report whether committed content differs from the last save
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    pub fn path(&self) -> Option<&PathBuf> {
        self.opts.file.as_ref()
    }

    /// read the canvas, including pending preview edits
    pub fn canvas(&self) -> &Buffer {
        &self.buffer
    }

    pub fn cursor(&self) -> Option<Point> {
        self.buffer.get_cursor().map(Point::from)
    }

    /// clear the document, save path and history
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

    /// undo the last edit, returning false when history is empty
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

    /// redo the last undo, returning false when history is empty
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

    /// render committed rows with a prefix and newline per row
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
