use cursive::{
    event::{
        Event, EventResult, Key, MouseButton::Left, MouseEvent::Press,
    },
    Vec2,
};
use std::{cmp::min, fmt};
use super::{Tool, simple_display, option};

use crate::editor::{buffer::*, scroll::EditorCtx};
use crate::constants::CONSUMED;

#[derive(Clone)]
pub(crate) struct TextTool {
    src: Option<Vec2>,
    cursor_active: bool,
    buffer: Vec<Vec<char>>,
    cursor: Vec2,
}

impl Default for TextTool {
    fn default() -> Self {
        Self {
            src: None,
            cursor_active: false,
            buffer: vec![],
            cursor: Vec2::new(0, 0),
        }
    }
}

simple_display! { TextTool, "Text" }

impl Tool for TextTool {
    fn on_event(&mut self, ctx: &mut EditorCtx<'_>, event: &Event) -> Option<EventResult> {
        let Vec2 { x, y } = &mut self.cursor;

        match ctx.relativize(event) {
            Event::Mouse {
                event: Press(Left),
                position,
                ..
            } => {
                if !self.cursor_active {
                    self.src = Some(position);
                    self.cursor_active = true;
                    self.buffer.clear();
                    self.buffer.push(vec![]);
                    self.cursor = Vec2::new(0, 0);
                    ctx.preview(|buf| self.render(buf));
                } else {
                    ctx.clobber(|buf| self.render(buf));
                    self.reset();
                }
            }

            _ if !self.cursor_active => return None,

            Event::Char(c) => {
                self.buffer[*y].insert(*x, c);
                *x += 1;
                ctx.preview(|buf| self.render(buf));
                ctx.scroll_to_cursor();
            }

            Event::Key(Key::Up) => {
                *y = y.saturating_sub(1);
                *x = min(self.buffer[*y].len(), *x);
                ctx.preview(|buf| self.render(buf));
                ctx.scroll_to_cursor();
            }

            Event::Key(Key::Down) => {
                *y = min(self.buffer.len() - 1, *y + 1);
                *x = min(self.buffer[*y].len(), *x);
                ctx.preview(|buf| self.render(buf));
                ctx.scroll_to_cursor();
            }

            Event::Key(Key::Left) => {
                *x = x.saturating_sub(1);
                ctx.preview(|buf| self.render(buf));
                ctx.scroll_to_cursor();
            }

            Event::Key(Key::Right) => {
                *x = min(self.buffer[*y].len(), *x + 1);
                ctx.preview(|buf| self.render(buf));
                ctx.scroll_to_cursor();
            }

            Event::Key(Key::Enter) => {
                let next = self.buffer[*y].split_off(*x);
                self.buffer.insert(*y + 1, next);
                *x = 0;
                *y += 1;
                ctx.preview(|buf| self.render(buf));
                ctx.scroll_to_cursor();
            }

            Event::Key(Key::Backspace) | Event::Key(Key::Del) => {
                if *x > 0 {
                    self.buffer[*y].remove(*x - 1);
                    *x -= 1;
                } else if *y > 0 {
                    let mut next = self.buffer.remove(*y);
                    *y -= 1;
                    *x = self.buffer[*y].len();
                    self.buffer[*y].append(&mut next);
                }
                ctx.preview(|buf| self.render(buf));
                ctx.scroll_to_cursor();
            }

            Event::Key(Key::Esc) => {
                self.reset();
                ctx.clobber(|buf| self.render(buf));
            }

            _ => return None,
        }

        CONSUMED
    }
}

impl TextTool {
    fn render(&self, buf: &mut Buffer) {
        let src = option!(self.src);

        for (y, line) in self.buffer.iter().enumerate() {
            for (x, c) in line.iter().enumerate() {
                let pos = Vec2::new(x, y) + src;
                buf.setv(true, pos, *c);
            }
        }

        buf.set_cursor(self.cursor + src);
    }

    fn reset(&mut self) {
        self.src = None;
        self.cursor_active = false;
        self.buffer.clear();
        self.cursor = Vec2::new(0, 0);
    }
}
