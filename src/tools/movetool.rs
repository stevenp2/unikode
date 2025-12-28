use cursive::{
    event::{
        Event, EventResult, Key, MouseButton::Left,
        MouseEvent::{Hold, Press, Release},
    },
    Rect, Vec2
};
use std::fmt;

use crate::editor::{buffer::Buffer, scroll::EditorCtx, EditorMode};
use crate::constants::{SP, CONSUMED};
use crate::config::{Options, Symbols};
use super::{Tool, visible_cells, simple_display, mouse_drag, selecttool::SelectTool, lines::boxtool::BoxTool};

pub(crate) struct MoveTool {
    pub selection: Rect,
    pub anchor: Vec2,
    symbols: Symbols,
}

impl MoveTool {
    pub fn new(selection: Rect, anchor: Vec2) -> Self {
        Self {
            selection,
            anchor,
            symbols: Symbols::default(),
        }
    }
}

impl Tool for MoveTool {
    fn load_opts(&mut self, opts: &Options) {
        self.symbols = opts.symbols.clone();
    }

    fn on_event(&mut self, ctx: &mut EditorCtx<'_>, event: &Event) -> Option<EventResult> {
        match event {
            Event::Mouse { .. } => {
                let (pos, event) = mouse_drag!(ctx, event);

                match event {
                    Press(Left) | Hold(Left) => {
                        ctx.preview(|buf| move_on_buffer(buf, self.selection, self.anchor, pos, &self.symbols));
                    }

                    Release(Left) => {
                        ctx.clobber(|buf| move_on_buffer(buf, self.selection, self.anchor, pos, &self.symbols));
                        let mut editor = ctx.0.get_inner_mut().write();
                        editor.mode = EditorMode::Select(pos);
                        editor.set_tool(SelectTool::default());
                    }

                    _ => return None,
                }
                return CONSUMED;
            }

            Event::Char(c) if c.is_ascii_digit() => {
                let mut editor = ctx.0.get_inner_mut().write();
                editor.pending_count.push(*c);
                return CONSUMED;
            }

            Event::Char('h') | Event::Char('j') | Event::Char('k') | Event::Char('l') => {
                let count = {
                    let mut editor = ctx.0.get_inner_mut().write();
                    let count = editor.pending_count.parse::<usize>().unwrap_or(1).max(1);
                    editor.pending_count.clear();
                    count
                };

                let mut pos = ctx.0.get_inner_mut().read().buffer.get_cursor().unwrap_or_else(|| Vec2::new(0, 0));
                if let Event::Char(c) = event {
                    match *c {
                        'h' => if pos.x >= count { pos.x -= count } else { pos.x = 0 },
                        'j' => pos.y += count,
                        'k' => if pos.y >= count { pos.y -= count } else { pos.y = 0 },
                        'l' => pos.x += count,
                        _ => unreachable!(),
                    }
                }
                ctx.0.get_inner_mut().write().buffer.set_cursor(pos);
                ctx.preview(|buf| move_on_buffer(buf, self.selection, self.anchor, pos, &self.symbols));
                ctx.scroll_to_cursor();
                return CONSUMED;
            }

            Event::Char('\n') | Event::Key(Key::Enter) => {
                let pos = ctx.0.get_inner_mut().read().buffer.get_cursor().unwrap_or_else(|| Vec2::new(0, 0));
                
                ctx.clobber(|buf| move_on_buffer(buf, self.selection, self.anchor, pos, &self.symbols));
                let mut editor = ctx.0.get_inner_mut().write();
                editor.mode = EditorMode::Select(pos);
                editor.set_tool(SelectTool::default());
                return CONSUMED;
            }

            Event::Key(Key::Esc) => {
                let pos = ctx.0.get_inner_mut().read().buffer.get_cursor().unwrap_or_else(|| Vec2::new(0, 0));
                
                ctx.clobber(|buf| move_on_buffer(buf, self.selection, self.anchor, pos, &self.symbols));
                let mut editor = ctx.0.get_inner_mut().write();
                editor.mode = EditorMode::Normal;
                editor.set_tool(BoxTool::default());
                return CONSUMED;
            }

            _ => return None,
        }
    }

    fn move_info(&self) -> Option<(Rect, Vec2)> {
        Some((self.selection, self.anchor))
    }
}

simple_display! { MoveTool, "Move" }

pub fn move_on_buffer(buf: &mut Buffer, selection: Rect, from: Vec2, to: Vec2, symbols: &Symbols) {
    let state: Vec<_> = visible_cells(buf, (selection.top_left(), selection.bottom_right()), symbols).collect();

    for cell in state.iter() {
        buf.setv(true, cell.pos(), SP, symbols);
    }

    let delta = to.signed() - from.signed();

    for cell in state.into_iter().map(|cell| cell.translate(delta)) {
        buf.setv(true, cell.pos(), cell.c(), symbols);
    }
    
    buf.set_cursor(to);
}