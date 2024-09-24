use cursive::{
    event::{Event, EventResult, MouseButton::*, MouseEvent::*},
    Rect, Vec2,
};
use std::fmt;

use super::{Tool, visible_cells, simple_display, option, mouse_drag};

use crate::editor::{buffer::*, scroll::EditorCtx, CONSUMED, SP};

#[derive(Copy, Clone, Default)]
pub(crate) struct MoveTool {
    src: Option<Vec2>,
    dst: Option<Vec2>,
    grab_src: Option<Vec2>,
    grab_dst: Option<Vec2>,
}

simple_display! { MoveTool, "Move" }

impl Tool for MoveTool {
    fn on_event(&mut self, ctx: &mut EditorCtx<'_>, e: &Event) -> Option<EventResult> {
        let (pos, event) = mouse_drag!(ctx, e);

        match event {
            Press(Left) => {
                if let Some(true) = self
                    .src
                    .and_then(|o| Some((o, self.dst?)))
                    .map(|(o, t)| Rect::from_corners(o, t))
                    .map(|r| r.contains(pos))
                {
                    self.grab_src = Some(pos);
                    self.grab_dst = Some(pos);
                } else {
                    self.src = Some(pos);
                    self.dst = Some(pos);
                    self.grab_src = None;
                    self.grab_dst = None;
                }
                ctx.preview(|buf| self.render(buf));
            }

            Hold(Left) => {
                if self.grab_src.is_some() {
                    self.grab_dst = Some(pos);
                } else {
                    self.dst = Some(pos);
                }
                ctx.preview(|buf| self.render(buf));
            }

            Release(Left) => {
                if self.grab_src.is_some() {
                    self.grab_dst = Some(pos);
                    ctx.clobber(|buf| self.render(buf));
                    self.src = None;
                    self.dst = None;
                    self.grab_src = None;
                    self.grab_dst = None;
                } else {
                    self.dst = Some(pos);
                    ctx.preview(|buf| self.render(buf));
                }
            }

            _ => return None,
        }

        CONSUMED
    }
}

impl MoveTool {
    fn render(&self, buf: &mut Buffer) {
        let (src, dst) = option!(self.src, self.dst);

        let state: Vec<_> = visible_cells(buf, (src, dst)).collect();

        if let (Some(grab_src), Some(grab_dst)) = (self.grab_src, self.grab_dst) {
            for cell in state.iter() {
                buf.setv(true, cell.pos(), SP);
            }

            let delta = grab_dst.signed() - grab_src.signed();

            for cell in state.into_iter().map(|cell| cell.translate(delta)) {
                buf.setv(true, cell.pos(), cell.c());
            }
        } else {
            for cell in state {
                buf.setv(true, cell.pos(), cell.c());
            }
        }
    }
}
