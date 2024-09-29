use cursive::{
    event::{Event, EventResult, MouseButton::*, MouseEvent::*},
    view::scroll::Scroller,
    views::ScrollView,
    Vec2,
};
use lazy_static::lazy_static;
use parking_lot::Mutex;
use std::cmp::{max, min};

use crate::constants::CONSUMED;

use super::{EditorView, Buffer};

macro_rules! intercept_scrollbar {
    ($ctx:expr, $event:expr) => {{
        lazy_static! {
            static ref LAST_LPRESS: Mutex<Option<Vec2>> = Mutex::new(None);
        }

        if let Event::Mouse {
            offset,
            position: pos,
            event,
        } = $event
        {
            match event {
                Press(Left) if $ctx.on_scrollbar(*offset, *pos) => {
                    *LAST_LPRESS.lock() = Some(*pos);
                    return None;
                }

                Press(Left) => {
                    *LAST_LPRESS.lock() = Some(*pos);
                }

                Hold(Left)
                    if LAST_LPRESS
                        .lock()
                        .map(|pos| $ctx.on_scrollbar(*offset, pos))
                        .unwrap_or(false) =>
                {
                    return None;
                }

                Release(Left)
                    if LAST_LPRESS
                        .lock()
                        .take()
                        .map(|pos| $ctx.on_scrollbar(*offset, pos))
                        .unwrap_or(false) =>
                {
                    return None;
                }

                _ => {}
            }
        }
    }};
}

macro_rules! intercept_pan {
    ($ctx:expr, $event:expr) => {{
        lazy_static! {
            static ref OLD: Mutex<Option<Vec2>> = Mutex::new(None);
        }

        if let Event::Mouse {
            position: pos,
            event,
            ..
        } = $event
        {
            match event {
                Press(Right) => {
                    *OLD.lock() = Some(*pos);
                    return CONSUMED;
                }

                Hold(Right) if OLD.lock().is_none() => {
                    *OLD.lock() = Some(*pos);
                    return CONSUMED;
                }

                Hold(Right) => {
                    let old = OLD.lock().replace(*pos).unwrap();

                    let offset = ($ctx.0.content_viewport())
                        .top_left()
                        .map_x(|x| drag(x, pos.x, old.x))
                        .map_y(|y| drag(y, pos.y, old.y));

                    $ctx.0.set_offset(offset);

                    let p = $ctx.0.content_viewport();
                    let i = $ctx.0.inner_size();

                    let mut editor = $ctx.0.get_inner_mut().write();
                    if pos.x < old.x && within((old.x - pos.x + 1) * 4, p.right(), i.x) {
                        editor.canvas.x += old.x - pos.x;
                    }
                    if pos.y < old.y && within((old.y - pos.y + 1) * 2, p.bottom(), i.y) {
                        editor.canvas.y += old.y - pos.y;
                    }

                    return CONSUMED;
                }

                Release(Right) => {
                    *OLD.lock() = None;
                    return CONSUMED;
                }

                _ => {}
            }
        }
    }};
}

pub(crate) struct EditorCtx<'a>(&'a mut ScrollView<EditorView>);

impl<'a> EditorCtx<'a> {
    /// Returns a new `EditorCtx`.
    pub fn new(view: &'a mut ScrollView<EditorView>) -> Self {
        Self(view)
    }

    /// Handles an event using the active tool.
    pub fn on_event(&mut self, event: &Event) -> Option<EventResult> {
        intercept_scrollbar!(self, event);
        intercept_pan!(self, event);

        let mut tool = self.0.get_inner_mut().write().active_tool.take().unwrap();
        let res = tool.on_event(self, event);
        self.0.get_inner_mut().write().active_tool = Some(tool);

        res
    }

    /// Returns `true` if `pos` is located on a scrollbar.
    fn on_scrollbar(&self, offset: Vec2, pos: Vec2) -> bool {
        let core = self.0.get_scroller();
        let max = core.last_outer_size() + offset;
        let min = max - core.scrollbar_size();

        (min.x..=max.x).contains(&pos.x) || (min.y..=max.y).contains(&pos.y)
    }

    /// If `event` is a mouse event, relativize its position to the canvas plane.
    pub(crate) fn relativize(&self, event: &Event) -> Event {
        let mut event = event.clone();
        if let Event::Mouse {
            offset, position, ..
        } = &mut event
        {
            let tl = self.0.content_viewport().top_left();
            *position = position.saturating_sub(*offset) + tl;
        }
        event
    }

    /// Scroll to `pos`, moving at least `step_x` & `step_y` respectively if the x or y
    /// scroll offset needs to be modified.
    pub(crate) fn scroll_to(&mut self, pos: Vec2, step_x: usize, step_y: usize) {
        let port = self.0.content_viewport();
        let mut offset = port.top_left();

        if pos.x >= port.right() {
            offset.x += max(step_x, pos.x - port.right());
        } else if pos.x <= port.left() {
            offset.x -= max(min(step_x, offset.x), port.left() - pos.x);
        }
        if pos.y >= port.bottom() {
            offset.y += max(step_y, pos.y - port.bottom());
        } else if pos.y <= port.top() {
            offset.y -= max(min(step_y, offset.y), port.top() - pos.y);
        }
        self.0.set_offset(offset);

        // BUG: scrolling lags behind changes to the canvas bounds by 1 render tick. in
        // order to truly fix the issue, we need to implement scrolling as a function of
        // the editor itself.
        let mut editor = self.0.get_inner_mut().write();

        if pos.x + 1 >= editor.canvas.x {
            editor.canvas.x += max(step_x, (pos.x + 1) - editor.canvas.x);
        }
        if pos.y + 1 >= editor.canvas.y {
            editor.canvas.y += max(step_y, (pos.y + 1) - editor.canvas.y);
        }
    }

    /// Scroll to the edit buffer's current cursor, if one exists.
    pub(crate) fn scroll_to_cursor(&mut self) {
        let pos = self.0.get_inner_mut().read().buffer.get_cursor();

        if let Some(pos) = pos {
            self.scroll_to(pos, 1, 1);
        }
    }

    /// Modify the edit buffer using `render`, flushing any changes and saving a snapshot
    /// of the buffer's prior state in the editor's undo history.
    pub(crate) fn clobber<R: FnOnce(&mut Buffer)>(&mut self, render: R) {
        let mut editor = self.0.get_inner_mut().write();

        editor.with_snapshot(|ed| {
            render(&mut ed.buffer);
            ed.buffer.flush_edits();
            ed.buffer.drop_cursor();
        });
    }

    /// Modify the edit buffer using `render`, without flushing any changes.
    pub(crate) fn preview<R: FnOnce(&mut Buffer)>(&mut self, render: R) {
        let mut editor = self.0.get_inner_mut().write();
        editor.buffer.discard_edits();
        render(&mut editor.buffer);
    }
}

fn drag(x: usize, new: usize, old: usize) -> usize {
    if new > old {
        x.saturating_sub(new - old)
    } else {
        x + (old - new)
    }
}

/// Returns `true` if `a` is within `w` of `b` (inclusive).
fn within(w: usize, a: usize, b: usize) -> bool {
    diff(a, b) <= w
}

/// Returns the absolute difference between `a` and `b`.
fn diff(a: usize, b: usize) -> usize {
    (a as isize - b as isize).abs().unsigned_abs()
}
