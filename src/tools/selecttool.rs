use cursive::{
    event::{
        Event, EventResult, MouseButton::Left,
        MouseEvent::{Hold, Press, Release},
    },
    Vec2,
};

use std::fmt;

use crate::editor::{buffer::Buffer, scroll::EditorCtx, EditorMode};
use crate::constants::CONSUMED;
use crate::config::Options;
use super::{Tool, simple_display, mouse_drag};

#[derive(Clone, Default)]
pub(crate) struct SelectTool {
    pub anchor: Option<Vec2>,
}

impl Tool for SelectTool {
    fn load_opts(&mut self, _: &Options) {}

    fn on_event(&mut self, ctx: &mut EditorCtx<'_>, event: &Event) -> Option<EventResult> {
        let (pos, event) = mouse_drag!(ctx, event);

        match event {
            Press(Left) => {
                self.anchor = Some(pos);
                let mut editor = ctx.0.get_inner_mut().write();
                editor.mode = EditorMode::Select(pos);
                editor.buffer.set_cursor(pos);
            }

            Hold(Left) => {
                ctx.preview(|buf: &mut Buffer| buf.set_cursor(pos));
            }

            Release(Left) => {
                ctx.preview(|buf: &mut Buffer| buf.set_cursor(pos));
            }

            _ => return None,
        }

        CONSUMED
    }
}

simple_display! { SelectTool, "Select" }