pub mod buffer;
pub mod cell;
pub mod command;

use clipboard::{ClipboardContext, ClipboardProvider};
use cursive::{theme::ColorStyle, view::View, Printer, Rect, Vec2};
use parking_lot::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::{
    cmp::max,
    error::Error,
    fmt,
    fs::{self, File, OpenOptions},
    io::{self, ErrorKind, Write},
    mem,
    path::{Path, PathBuf},
    sync::Arc,
};

use crate::config::{LineNumberMode, Options};
use crate::core::Point;
use crate::editor::command::{Direction, EditorCommand};
use crate::editor::{
    buffer::Buffer,
    cell::{Cell, Char},
};
use crate::tools::{
    erasetool::erase_on_buffer,
    lines::{
        arrowtool::draw_arrow_on_buffer, boxtool::draw_box_on_buffer, linetool::draw_line_on_buffer,
    },
    movetool::move_on_buffer,
};
use crate::tools::{lines::boxtool::BoxTool, Tool};

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

#[derive(Clone)]
pub struct EditorView {
    inner: Arc<RwLock<Editor>>,
}

impl View for EditorView {
    fn draw(&self, p: &Printer<'_, '_>) {
        let mut normal = print_styled(ColorStyle::primary());
        let mut change = print_styled(ColorStyle::highlight_inactive());
        let mut cursor = print_styled(ColorStyle::highlight());
        let mut selection_style = print_styled(ColorStyle::highlight_inactive());

        let editor = self.read();
        let cursor_pos = editor.buffer.get_cursor().unwrap_or(Vec2::new(0, 0));

        let selection_rect = match editor.mode {
            EditorMode::Select(start) => {
                if let Some((selection, anchor)) =
                    editor.active_tool.as_ref().and_then(|t| t.move_info())
                {
                    let delta = cursor_pos.signed() - anchor.signed();
                    let top_left = selection.top_left().signed() + delta;
                    let bottom_right = selection.bottom_right().signed() + delta;
                    Some(Rect::from_corners(
                        top_left.map(|v| v.max(0) as usize),
                        bottom_right.map(|v| v.max(0) as usize),
                    ))
                } else {
                    Some(Rect::from_corners(start, cursor_pos))
                }
            }
            _ => None,
        };

        let is_moving = editor
            .active_tool
            .as_ref()
            .is_some_and(|t| t.move_info().is_some());

        // Draw line numbers (sticky on the left)
        for y in 0..p.size.y {
            let buffer_y = p.content_offset.y + y;
            let rel_num = (buffer_y as isize - cursor_pos.y as isize).unsigned_abs();

            let (num_str, is_active) = match editor.opts.line_mode {
                Some(LineNumberMode::Absolute) => {
                    (format!("{:<4}", buffer_y + 1), buffer_y == cursor_pos.y)
                }
                Some(LineNumberMode::Relative) | None => {
                    if rel_num == 0 {
                        (format!("{:<4}", buffer_y + 1), true)
                    } else {
                        (format!("{rel_num:>4}"), false)
                    }
                }
            };

            let style = if is_active {
                ColorStyle::title_primary()
            } else {
                ColorStyle::title_secondary()
            };
            p.with_color(style, |p| {
                // Print at the visible left edge (compensating for scroll offset)
                p.print((p.content_offset.x, buffer_y), &format!("{num_str} "));
            });
        }

        let gutter_width = editor.opts.gutter_width;

        let content_offset = p.content_offset.map_x(|x| x.saturating_sub(gutter_width));
        let content_size = p.size.map_x(|x| x + gutter_width);

        for c in editor
            .buffer
            .iter_within(content_offset, content_size, &editor.opts.symbols)
        {
            let (pos, char_val, is_cursor, is_dirty) = match c {
                Char::Clean(Cell { pos, c }) => (pos, c, false, false),
                Char::Dirty(Cell { pos, c }) => (pos, c, false, true),
                Char::Cursor(Cell { pos, c }) => (pos, c, true, false),
            };

            let view_pos = pos.map_x(|x| x + gutter_width);
            let in_selection = selection_rect
                .map(|r: Rect| r.contains(pos))
                .unwrap_or(false);

            let should_highlight = if is_moving {
                in_selection && is_dirty
            } else {
                in_selection
            };

            // Skip drawing if it would overlap with the sticky line numbers (excluding the space column)
            if view_pos.x < p.content_offset.x + gutter_width - 1 {
                continue;
            }

            if is_cursor {
                cursor(p, view_pos, char_val);
            } else if should_highlight && char_val != ' ' {
                selection_style(p, view_pos, char_val);
            } else if !is_moving && is_dirty && char_val != ' ' {
                change(p, view_pos, char_val);
            } else {
                normal(p, view_pos, char_val);
            }
        }
    }

    fn required_size(&mut self, size: Vec2) -> Vec2 {
        let editor = self.read();

        let buf_bounds = editor.buffer.bounds();

        Vec2 {
            x: max(size.x, buf_bounds.x + editor.opts.gutter_width),
            y: max(size.y, buf_bounds.y),
        }
    }
}

impl EditorView {
    pub fn new(inner: Editor) -> Self {
        Self {
            inner: Arc::new(RwLock::new(inner)),
        }
    }

    pub fn read(&self) -> RwLockReadGuard<'_, Editor> {
        self.inner.read()
    }

    pub fn write(&self) -> RwLockWriteGuard<'_, Editor> {
        self.inner.write()
    }
}

pub struct Editor {
    pub(crate) mode: EditorMode,
    pub(crate) pending_count: String,
    pub(crate) opts: Options,
    pub(crate) buffer: Buffer,
    lsave: Buffer,
    dirty: bool,
    undo_history: Vec<Buffer>,
    redo_history: Vec<Buffer>,
    pub(crate) active_tool: Option<Box<dyn Tool + Send + Sync>>,
    pub(crate) last_left_press: Option<Vec2>,
    pub(crate) pan_origin: Option<Vec2>,
    rendered: String,
}

/// Name used by headless consumers to emphasize that this value owns one
/// document's editing state and history.
pub type EditorSession = Editor;

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

    /// Open the file at `path`, discarding any unsaved changes to the current file, if
    /// there are any.
    ///
    /// No modifications have been performed if this returns `Err(_) `.
    pub fn open_file<P: AsRef<Path>>(&mut self, path: P) -> io::Result<()> {
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
    pub fn save(&mut self) -> io::Result<bool> {
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
    pub fn save_as<P: AsRef<Path>>(&mut self, path: P) -> io::Result<()> {
        let old_path = self.opts.file.replace(path.as_ref().into());
        match self.save() {
            Ok(true) => Ok(()),
            Ok(false) => unreachable!("save_as always configures a path"),
            Err(error) => {
                self.opts.file = old_path;
                Err(error)
            }
        }
    }

    /// Render to `file`, performing whitespace cleanup if enabled.
    fn render_to_file(&mut self, mut file: File) -> io::Result<()> {
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
    pub fn trim_margins(&mut self) {
        self.with_snapshot(|ed| {
            ed.buffer.strip_margin_whitespace();
            ed.buffer.set_cursor(Vec2::new(0, 0));
        });
    }

    /// Take a snapshot of the buffer, discard any pending edits, and run `apply`. If
    /// the buffer was modified, mark it as dirty. Otherwise, remove the snapshot.
    ///
    /// Use this function to execute any buffer modification that should be saved in the
    /// undo history.
    pub(crate) fn with_snapshot<F: FnOnce(&mut Self)>(&mut self, apply: F) {
        let snapshot = self.buffer.snapshot();
        self.undo_history.push(snapshot);
        self.buffer.discard_edits();

        apply(self);

        if self.undo_history.last().unwrap() == &self.buffer {
            self.undo_history.pop();
        } else {
            self.redo_history.clear();
            self.dirty = true;
        }
    }

    /// Undo the last buffer modification.
    ///
    /// Returns `false` if there was nothing to undo.
    pub fn undo(&mut self) -> bool {
        let cursor = self.buffer.get_cursor();
        let undone = self
            .undo_history
            .pop()
            .map(|buffer| mem::replace(&mut self.buffer, buffer))
            .map(|buffer| self.redo_history.push(buffer))
            .is_some();

        if undone {
            self.dirty = self.buffer != self.lsave;
            if self.buffer.get_cursor().is_none() {
                if let Some(p) = cursor {
                    self.buffer.set_cursor(p);
                }
            }
        }

        undone
    }

    /// Redo the last undone buffer modification.
    ///
    /// Returns `false` if there was nothing to redo.
    pub fn redo(&mut self) -> bool {
        let cursor = self.buffer.get_cursor();
        let redone = self
            .redo_history
            .pop()
            .map(|buffer| mem::replace(&mut self.buffer, buffer))
            .map(|buffer| self.undo_history.push(buffer))
            .is_some();

        if redone {
            self.dirty = self.buffer != self.lsave;
            if self.buffer.get_cursor().is_none() {
                if let Some(p) = cursor {
                    self.buffer.set_cursor(p);
                }
            }
        }

        redone
    }

    /// Render the committed document as Unicode text without touching the
    /// filesystem or system clipboard.
    pub fn render_text(&self, prefix: &str) -> String {
        self.buffer.iter(prefix).collect()
    }

    /// Execute a headless editor command through the same history boundary used
    /// by the terminal controller. Returns whether the document changed.
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
                draw_box_on_buffer(buffer, from.into(), to.into(), &options.symbols)
            }),
            EditorCommand::DrawLine { from, to } => self.commit_buffer_edit(|buffer, options| {
                draw_line_on_buffer(
                    buffer,
                    from.into(),
                    to.into(),
                    options.path_mode,
                    &options.symbols,
                )
            }),
            EditorCommand::DrawArrow { from, to } => self.commit_buffer_edit(|buffer, options| {
                draw_arrow_on_buffer(
                    buffer,
                    from.into(),
                    to.into(),
                    options.path_mode,
                    &options.symbols,
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
            EditorCommand::MoveSelection { region, to } => {
                self.commit_buffer_edit(|buffer, options| {
                    move_on_buffer(
                        buffer,
                        region.into(),
                        region.max.into(),
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
            EditorCommand::TrimMargins => {
                let before = self.buffer.clone();
                self.trim_margins();
                before != self.buffer
            }
        }
    }

    fn commit_buffer_edit<F>(&mut self, edit: F) -> bool
    where
        F: FnOnce(&mut Buffer, &Options),
    {
        let before = self.buffer.snapshot();
        let options = self.opts.clone();
        self.with_snapshot(|editor| {
            edit(&mut editor.buffer, &options);
            editor.buffer.flush_edits();
        });
        before != self.buffer
    }
}

fn print_styled(style: ColorStyle) -> impl FnMut(&Printer<'_, '_>, Vec2, char) {
    let mut buf = vec![0; 4];
    move |p, pos, c| {
        p.with_color(style, |p| p.print(pos, c.encode_utf8(&mut buf)));
    }
}
