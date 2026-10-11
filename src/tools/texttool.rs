use super::{option, simple_display, Tool};
use cursive::{
    event::{Event, EventResult, Key, MouseButton::Left, MouseEvent::Press},
    Vec2,
};
use std::{cmp::min, fmt};

use crate::config::{Options, Symbols};
use crate::constants::CONSUMED;
use crate::core::Point;
use crate::editor::buffer::*;
use crate::editor::command::EditorCommand;
use crate::tui::controller::EditorCtx;

#[derive(Clone)]
pub(crate) struct TextTool {
    src: Option<Vec2>,
    cursor_active: bool,
    buffer: Vec<Vec<char>>,
    cursor: Vec2,
    symbols: Symbols,
}

impl Default for TextTool {
    fn default() -> Self {
        Self {
            src: None,
            cursor_active: false,
            buffer: vec![],
            cursor: Vec2::new(0, 0),
            symbols: Symbols::default(),
        }
    }
}

simple_display! { TextTool, "Text" }

impl Tool for TextTool {
    fn load_opts(&mut self, opts: &Options) {
        self.symbols = opts.symbols.clone();
    }

    fn on_event(&mut self, ctx: &mut EditorCtx<'_>, event: &Event) -> Option<EventResult> {
        let Vec2 { x, y } = &mut self.cursor;

        match ctx.relativize(event) {
            Event::Mouse {
                event: Press(Left),
                position,
                ..
            } => {
                if self.cursor_active {
                    self.commit(ctx);
                }
                self.src = Some(position);
                self.cursor_active = true;
                self.buffer.clear();
                self.buffer.push(vec![]);
                self.cursor = Vec2::new(0, 0);
                ctx.preview(|buf| self.render(buf));
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
                self.commit(ctx);
                self.reset();
                return None;
            }

            _ => return None,
        }

        CONSUMED
    }
}

impl TextTool {
    pub(crate) fn new(pos: Vec2) -> Self {
        Self {
            src: Some(pos),
            cursor_active: true,
            buffer: vec![vec![]],
            cursor: Vec2::new(0, 0),
            symbols: Symbols::default(),
        }
    }

    fn render(&self, buf: &mut Buffer) {
        let src = option!(self.src);

        for (y, line) in self.buffer.iter().enumerate() {
            for (x, c) in line.iter().enumerate() {
                let pos = Vec2::new(x, y) + src;
                buf.setv(true, pos, *c, &self.symbols);
            }
        }

        buf.set_cursor(self.cursor + src);
    }

    fn commit(&self, ctx: &mut EditorCtx<'_>) {
        let Some(src) = self.src else {
            return;
        };
        let text = self
            .buffer
            .iter()
            .map(|line| line.iter().collect::<String>())
            .collect::<Vec<_>>()
            .join("\n");
        ctx.dispatch(EditorCommand::InsertText {
            at: Point::from(src),
            text,
        });
        ctx.0
            .get_inner_mut()
            .write()
            .buffer
            .set_cursor(self.cursor + src);
    }

    fn reset(&mut self) {
        self.src = None;
        self.cursor_active = false;
        self.buffer.clear();
        self.cursor = Vec2::new(0, 0);
    }
}
