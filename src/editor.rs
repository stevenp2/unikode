pub mod buffer;
pub mod cell;
pub mod scroll;

use clipboard::{ClipboardContext, ClipboardProvider};
use cursive::{
    theme::ColorStyle,
    view::View,
    Printer, Vec2,
};
use parking_lot::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::{
    cmp::max,
    error::Error,
    fs::{self, File, OpenOptions},
    io::{self, ErrorKind, Write},
    mem,
    path::{Path, PathBuf},
    sync::Arc,
};

use crate::editor::{
    buffer::Buffer,
    cell::{Cell, Char}
};
use crate::tools::{
    Tool,
    lines::boxtool::BoxTool
};
use crate::config::Options;

#[derive(Clone)]
pub(crate) struct EditorView {
    inner: Arc<RwLock<Editor>>,
}

impl View for EditorView {
    fn draw(&self, p: &Printer<'_, '_>) {
        let mut normal = print_styled(ColorStyle::primary());
        let mut change = print_styled(ColorStyle::highlight_inactive());
        let mut cursor = print_styled(ColorStyle::highlight());

        for c in self.read().buffer.iter_within(p.content_offset, p.size) {
            match c {
                Char::Clean(Cell { pos, c }) => normal(p, pos, c),
                Char::Dirty(Cell { pos, c }) => change(p, pos, c),
                Char::Cursor(Cell { pos, c }) => cursor(p, pos, c),
            }
        }
    }

    fn required_size(&mut self, size: Vec2) -> Vec2 {
        let mut editor = self.write();

        let buf_bounds = editor.buffer.bounds();

        editor.canvas = Vec2 {
            x: max(buf_bounds.x, editor.canvas.x),
            y: max(buf_bounds.y, editor.canvas.y),
        };

        Vec2 {
            x: max(size.x, editor.canvas.x),
            y: max(size.y, editor.canvas.y),
        }
    }
}

impl EditorView {
    pub(crate) fn new(inner: Editor) -> Self {
        Self {
            inner: Arc::new(RwLock::new(inner)),
        }
    }

    pub(crate) fn read(&self) -> RwLockReadGuard<Editor> {
        self.inner.read()
    }

    pub(crate) fn write(&self) -> RwLockWriteGuard<Editor> {
        self.inner.write()
    }

}

pub(crate) struct Editor {
    opts: Options,
    buffer: Buffer,
    lsave: Buffer,
    dirty: bool,
    undo_history: Vec<Buffer>,
    redo_history: Vec<Buffer>,
    active_tool: Option<Box<dyn Tool + Send + Sync>>,
    canvas: Vec2,
    rendered: String,
}

impl Editor {
    /// Open an editor instance with the provided options.
    pub(crate) fn open(mut opts: Options) -> io::Result<Self> {
        let file = opts.file.take();

        let mut tool = BoxTool::default();
        tool.load_opts(&opts);

        let mut editor = Self {
            opts,
            buffer: Buffer::default(),
            lsave: Buffer::default(),
            dirty: false,
            undo_history: vec![],
            redo_history: vec![],
            active_tool: Some(Box::new(tool)),
            canvas: Vec2::new(0, 0),
            rendered: String::default(),
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
    pub(crate) fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Set the active tool.
    pub(crate) fn set_tool<T: Tool + 'static + Send + Sync>(&mut self, mut tool: T) {
        self.buffer.discard_edits();
        self.buffer.drop_cursor();
        tool.load_opts(&self.opts);
        self.active_tool = Some(Box::new(tool));
    }

    /// Returns the active tool as a human readable string.
    pub(crate) fn active_tool(&self) -> String {
        format!("({})", self.active_tool.as_ref().unwrap())
    }

    /// Returns the current save path.
    pub(crate) fn path(&self) -> Option<&PathBuf> {
        self.opts.file.as_ref()
    }

    /// Clear all buffer state and begin a blank diagram.
    pub(crate) fn clear(&mut self) {
        self.opts.file = None;
        self.buffer.clear();
        self.lsave.clear();
        self.dirty = false;
        self.undo_history.clear();
        self.redo_history.clear();
        self.canvas = Vec2::new(0, 0);
    }

    /// Open the file at `path`, discarding any unsaved changes to the current file, if
    /// there are any.
    ///
    /// No modifications have been performed if this returns `Err(_)`.
    pub(crate) fn open_file<P: AsRef<Path>>(&mut self, path: P) -> io::Result<()> {
        let buffer = OpenOptions::new()
            .read(true)
            .open(path.as_ref())
            .and_then(Buffer::read_from);

        let buffer = match buffer {
            Err(e) if e.kind() == ErrorKind::NotFound => None,
            r => Some(r?),
        };

        self.clear();
        self.opts.file = Some(path.as_ref().into());
        if let Some(buf) = buffer {
            self.lsave = buf.clone();
            self.buffer = buf;
        }

        Ok(())
    }

    /// Save the current buffer contents to disk.
    ///
    /// Returns `Ok(true)` if the buffer was saved, and `Ok(false)` if there is no path
    /// configured for saving.
    ///
    /// If the configured save path does not exist, this will recursively create it.
    pub(crate) fn save(&mut self) -> io::Result<bool> {
        if let Some(path) = self.path() {
            path.parent().map(fs::create_dir_all).transpose()?;

            let file = OpenOptions::new()
                .read(false)
                .write(true)
                .create(true)
                .truncate(true)
                .open(path)?;

            self.render_to_file(file)?;
            self.lsave = self.buffer.clone();
            self.dirty = false;
        }

        Ok(self.path().is_some())
    }

    /// Save the current buffer contents to the file at `path`, and setting that as the
    /// new path for future calls to `save`.
    pub(crate) fn save_as<P: AsRef<Path>>(&mut self, path: P) -> io::Result<()> {
        self.opts.file = Some(path.as_ref().into());
        self.save()?;

        Ok(())
    }

    /// Render to `file`, performing whitespace cleanup if enabled.
    fn render_to_file(&mut self, mut file: File) -> io::Result<()> {
        self.canvas = Vec2::new(0, 0);

        self.with_snapshot(|ed| {
            if ed.opts.strip_margin_ws {
                ed.buffer.strip_margin_whitespace();
            } else if !ed.opts.keep_trailing_ws {
                ed.buffer.strip_trailing_whitespace();
            }
        });

        self.rendered.clear();
        self.rendered.extend(self.buffer.iter(""));

        file.write_all(self.rendered.as_bytes())?;
        file.flush()?;
        file.sync_all()?;

        Ok(())
    }

    /// Render to the clipboard, prefixing all lines with `prefix`.
    ///
    /// Trims all margins in the output without changing the buffer's state.
    pub(crate) fn render_to_clipboard(&self, prefix: &str) -> Result<(), Box<dyn Error>> {
        let mut ctx = ClipboardContext::new()?;

        let mut buf = self.buffer.clone();
        buf.strip_margin_whitespace();

        let mut rendered: String = buf.iter(prefix).collect();
        if let Some(c) = rendered.pop() {
            if c != '\n' {
                rendered.push(c);
            }
        }

        ctx.set_contents(rendered)
    }

    /// Trim all whitespace from margins.
    pub(crate) fn trim_margins(&mut self) {
        self.with_snapshot(|ed| {
            ed.canvas = Vec2::new(0, 0);
            ed.buffer.strip_margin_whitespace();
        });
    }

    /// Take a snapshot of the buffer, discard any pending edits, and run `apply`. If
    /// the buffer was modified, mark it as dirty. Otherwise, remove the snapshot.
    ///
    /// Use this function to execute any buffer modification that should be saved in the
    /// undo history.
    fn with_snapshot<F: FnOnce(&mut Self)>(&mut self, apply: F) {
        let snapshot = self.buffer.snapshot();
        self.undo_history.push(snapshot);
        self.buffer.discard_edits();

        apply(self);

        if self.undo_history.last().unwrap() == &self.buffer {
            self.undo_history.pop();
        } else {
            self.dirty = true;
        }
    }

    /// Undo the last buffer modification.
    ///
    /// Returns `false` if there was nothing to undo.
    pub(crate) fn undo(&mut self) -> bool {
        let undone = self
            .undo_history
            .pop()
            .map(|buffer| mem::replace(&mut self.buffer, buffer))
            .map(|buffer| self.redo_history.push(buffer))
            .is_some();

        if undone {
            self.dirty = self.buffer != self.lsave;
        }

        undone
    }

    /// Redo the last undone buffer modification.
    ///
    /// Returns `false` if there was nothing to redo.
    pub(crate) fn redo(&mut self) -> bool {
        let redone = self
            .redo_history
            .pop()
            .map(|buffer| mem::replace(&mut self.buffer, buffer))
            .map(|buffer| self.undo_history.push(buffer))
            .is_some();

        if redone {
            self.dirty = self.buffer != self.lsave;
        }

        redone
    }
}

fn print_styled(style: ColorStyle) -> impl FnMut(&Printer<'_, '_>, Vec2, char) {
    let mut buf = vec![0; 4];
    move |p, pos, c| {
        p.with_color(style, |p| p.print(pos, c.encode_utf8(&mut buf)));
    }
}
