use std::{cmp::max, sync::Arc};

use cursive::{theme::ColorStyle, view::View, Printer, Rect, Vec2};
use parking_lot::{RwLock, RwLockReadGuard, RwLockWriteGuard};

use crate::config::LineNumberMode;
use crate::editor::{
    cell::{Cell, Char},
    Editor, EditorMode,
};

/// Cursive rendering and ownership adapter for an editor session.
#[derive(Clone)]
pub struct EditorView {
    inner: Arc<RwLock<Editor>>,
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

impl View for EditorView {
    fn draw(&self, printer: &Printer<'_, '_>) {
        let mut normal = print_styled(ColorStyle::primary());
        let mut change = print_styled(ColorStyle::highlight_inactive());
        let mut cursor = print_styled(ColorStyle::highlight());
        let mut selection_style = print_styled(ColorStyle::highlight_inactive());

        let editor = self.read();
        let cursor_pos = editor.buffer.get_cursor().unwrap_or(Vec2::new(0, 0));
        let selection_rect = match editor.mode {
            EditorMode::Select(start) => {
                if let Some((selection, anchor)) = editor
                    .active_tool
                    .as_ref()
                    .and_then(|tool| tool.move_info())
                {
                    let delta = cursor_pos.signed() - anchor.signed();
                    let top_left = selection.top_left().signed() + delta;
                    let bottom_right = selection.bottom_right().signed() + delta;
                    Some(Rect::from_corners(
                        top_left.map(|value| value.max(0) as usize),
                        bottom_right.map(|value| value.max(0) as usize),
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
            .is_some_and(|tool| tool.move_info().is_some());

        for y in 0..printer.size.y {
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

            if view_position.x < printer.content_offset.x + gutter_width - 1 {
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
