pub mod erasetool;
pub mod lines;
pub mod movetool;
pub mod selecttool;
pub mod texttool;

use cursive::{
    event::{Event, EventResult},
    Rect, Vec2,
};
use std::fmt;

use crate::config::{Options, Symbols};
use crate::editor::{
    buffer::Buffer,
    cell::{Cell, Char},
};
use crate::tui::controller::EditorCtx;

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
pub(crate) use option;

macro_rules! mouse_drag {
    ($ctx:expr, $event:expr) => {{
        let (pos, event) = match $ctx.relativize($event) {
            Event::Mouse {
                position, event, ..
            } => (position, event),

            _ => return None,
        };

        if let Hold(Left) = event {
            let step = $ctx.0.get_inner_mut().read().opts.scroll_mouse_drag_step;
            $ctx.scroll_to(pos, step, step);
        }

        (pos, event)
    }};
}
pub(crate) use mouse_drag;

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
pub(crate) use simple_display;

pub(crate) trait Tool: fmt::Display {
    fn load_opts(&mut self, _: &Options) {}

    fn on_event(&mut self, ctx: &mut EditorCtx<'_>, e: &Event) -> Option<EventResult>;

    fn move_info(&self) -> Option<(Rect, Vec2)> {
        None
    }
}

pub(crate) fn visible_cells<'a>(
    buf: &'a Buffer,
    cs: (Vec2, Vec2),
    symbols: &'a Symbols,
) -> impl Iterator<Item = Cell> + 'a {
    let area = Rect::from_corners(cs.0, cs.1);

    buf.iter_within(area.top_left(), area.size(), symbols)
        .filter_map(|c| match c {
            Char::Clean(cell) | Char::Dirty(cell) => Some(cell),
            _ => None,
        })
        .filter(|cell| !cell.is_whitespace())
}
