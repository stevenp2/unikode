use std::{cmp::max, sync::Arc};

use cursive::{theme::ColorStyle, view::View, Printer, Vec2};
use parking_lot::{
    MappedRwLockReadGuard, MappedRwLockWriteGuard, RwLock, RwLockReadGuard, RwLockWriteGuard,
};

use crate::config::LineNumberMode;
use crate::editor::{
    cell::{Cell, Char},
    Editor,
};

use super::session::Session;

#[derive(Clone)]
pub struct EditorView {
    inner: Arc<RwLock<Session>>,
}

impl EditorView {
    pub fn new(inner: Editor) -> Self {
        Self {
            inner: Arc::new(RwLock::new(Session::new(inner))),
        }
    }

    pub fn read(&self) -> MappedRwLockReadGuard<'_, Editor> {
        RwLockReadGuard::map(self.inner.read(), |session| &session.editor)
    }

    pub fn write(&self) -> MappedRwLockWriteGuard<'_, Editor> {
        RwLockWriteGuard::map(self.inner.write(), |session| &mut session.editor)
    }

    pub(crate) fn session_read(&self) -> RwLockReadGuard<'_, Session> {
        self.inner.read()
    }

    pub(crate) fn session_write(&self) -> RwLockWriteGuard<'_, Session> {
        self.inner.write()
    }
}

impl View for EditorView {
    fn draw(&self, printer: &Printer<'_, '_>) {
        let mut normal = print_styled(ColorStyle::primary());
        let mut change = print_styled(ColorStyle::highlight_inactive());
        let mut cursor = print_styled(ColorStyle::highlight());
        let mut selection_style = print_styled(ColorStyle::highlight_inactive());

        let session = self.session_read();
        let editor = &session.editor;
        let cursor_pos = editor.buffer.get_cursor().unwrap_or(Vec2::new(0, 0));
        let selection_rect = session.selection();
        let is_moving = session.is_moving();

        for y in 0..printer.size.y {
            if editor.opts.gutter_width == 0 {
                break;
            }
            let buffer_y = printer.content_offset.y + y;
            let relative = (buffer_y as isize - cursor_pos.y as isize).unsigned_abs();
            let (number, is_active) = match editor.opts.line_mode {
                Some(LineNumberMode::Absolute) => {
                    (format!("{:<4}", buffer_y + 1), buffer_y == cursor_pos.y)
                }
                Some(LineNumberMode::Relative) | None if relative == 0 => {
                    (format!("{:<4}", buffer_y + 1), true)
                }
                Some(LineNumberMode::Relative) | None => (format!("{relative:>4}"), false),
            };
            let style = if is_active {
                ColorStyle::title_primary()
            } else {
                ColorStyle::title_secondary()
            };
            printer.with_color(style, |printer| {
                printer.print((printer.content_offset.x, buffer_y), &format!("{number} "));
            });
        }

        let gutter_width = editor.opts.gutter_width;
        let content_offset = printer
            .content_offset
            .map_x(|x| x.saturating_sub(gutter_width));
        let content_size = printer.size.map_x(|x| x + gutter_width);

        for character in
            editor
                .buffer
                .iter_within(content_offset, content_size, &editor.opts.symbols)
        {
            let (position, value, is_cursor, is_dirty) = match character {
                Char::Clean(Cell { pos, c }) => (pos, c, false, false),
                Char::Dirty(Cell { pos, c }) => (pos, c, false, true),
                Char::Cursor(Cell { pos, c }) => (pos, c, true, false),
            };
            let view_position = position.map_x(|x| x + gutter_width);
            let selected = selection_rect.is_some_and(|region| region.contains(position));
            let highlighted = if is_moving {
                selected && is_dirty
            } else {
                selected
            };

            if view_position.x
                < printer
                    .content_offset
                    .x
                    .saturating_add(gutter_width)
                    .saturating_sub(1)
            {
                continue;
            }
            if is_cursor {
                cursor(printer, view_position, value);
            } else if highlighted && value != ' ' {
                selection_style(printer, view_position, value);
            } else if !is_moving && is_dirty && value != ' ' {
                change(printer, view_position, value);
            } else {
                normal(printer, view_position, value);
            }
        }
    }

    fn required_size(&mut self, size: Vec2) -> Vec2 {
        let editor = self.read();
        let bounds = editor.buffer.bounds();
        Vec2 {
            x: max(size.x, bounds.x + editor.opts.gutter_width),
            y: max(size.y, bounds.y),
        }
    }
}

fn print_styled(style: ColorStyle) -> impl FnMut(&Printer<'_, '_>, Vec2, char) {
    let mut buffer = [0; 4];
    move |printer, position, character| {
        printer.with_color(style, |printer| {
            printer.print(position, character.encode_utf8(&mut buffer));
        });
    }
}
