pub mod arrowtool;
pub mod boxtool;
pub mod erasetool;
pub mod linetool;
pub mod movetool;
pub mod texttool;

use cursive::{
    event::{Event, EventResult},
    Rect, Vec2,
};
use std::fmt;

use super::Options;

use crate::editor::{buffer::*, cell::*, scroll::EditorCtx};

macro_rules! option {
    ($a:expr) => {
        match $a {
            Some(a) => a,
            _ => return,
        }
    };

    ($a:expr, $b:expr) => {
        match ($a, $b) {
            (Some(a), Some(b)) => (a, b),
            _ => return,
        }
    };
}
pub(super) use option;

macro_rules! mouse_drag {
    ($ctx:expr, $event:expr) => {{
        let (pos, event) = match $ctx.relativize($event) {
            Event::Mouse {
                position, event, ..
            } => (position, event),

            _ => return None,
        };

        if let Hold(Left) = event {
            $ctx.scroll_to(pos, 2, 2);
        }

        (pos, event)
    }};
}
pub(super) use mouse_drag;

/// Provides an implementation of `Tool::on_event` for tools that contain a `src` and
/// `dst` field of type `Option<Vec2>`. The implementation performs basic left mouse
/// drag handling, calling the argument closure when relevant events occur.
macro_rules! fn_on_event_drag {
    ($render:expr) => {
        fn on_event(&mut self, ctx: &mut EditorCtx<'_>, event: &Event) -> Option<EventResult> {
            let (pos, event) = mouse_drag!(ctx, event);

            match event {
                Press(Left) => {
                    self.src = Some(pos);
                    self.dst = Some(pos);
                    ctx.preview(|buf| $render(self, buf));
                }

                Hold(Left) => {
                    self.dst = Some(pos);
                    ctx.preview(|buf| $render(self, buf));
                }

                Release(Left) => {
                    self.dst = Some(pos);
                    ctx.clobber(|buf| $render(self, buf));
                    self.src = None;
                    self.dst = None;
                }

                _ => return None,
            }

            CONSUMED
        }
    };
}
pub(super) use fn_on_event_drag;

#[macro_export]
 macro_rules! simple_display {
    ($type:ty, $fstr:expr) => {
        impl fmt::Display for $type {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, $fstr)
            }
        }
    };
}
pub(super) use simple_display;

pub(crate) trait Tool: fmt::Display {
    fn load_opts(&mut self, _: &Options) {}

    fn on_event(&mut self, ctx: &mut EditorCtx<'_>, e: &Event) -> Option<EventResult>;
}

#[derive(Copy, Clone, Debug)]
pub(crate) enum PathMode {
    Snap90,
    Snap45,
    Routed,
}

impl Default for PathMode {
    fn default() -> Self {
        Self::Snap90
    }
}

pub fn visible_cells(buf: &Buffer, cs: (Vec2, Vec2)) -> impl Iterator<Item = Cell> + '_ {
    let area = Rect::from_corners(cs.0, cs.1);

    buf.iter_within(area.top_left(), area.size())
        .filter_map(|c| match c {
            Char::Clean(cell) => Some(cell),
            Char::Dirty(cell) => Some(cell),
            _ => None,
        })
        .filter(|cell| !cell.is_whitespace())
}
