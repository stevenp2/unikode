use cursive::{
    event::{
        Event, EventResult,
        MouseButton::{Left, Right},
        MouseEvent::{Hold, Press, Release},
    },
    view::scroll::Scroller,
    views::ScrollView,
    Vec2,
};
use std::cmp::{max, min};

use super::{session::EventOutcome, EditorView};
use crate::constants::CONSUMED;

pub(crate) struct EditorCtx<'a>(pub(crate) &'a mut ScrollView<EditorView>);

impl<'a> EditorCtx<'a> {
    pub fn new(view: &'a mut ScrollView<EditorView>) -> Self {
        Self(view)
    }

    pub fn on_event(&mut self, event: &Event) -> Option<EventResult> {
        if self.scrollbar_event(event) {
            return None;
        }
        if self.pan(event) {
            return CONSUMED;
        }
        let relative = self.relativize(event);
        let outcome = self.0.get_inner().session_write().on_event(&relative);
        match outcome {
            EventOutcome::Ignored => None,
            EventOutcome::Consumed => CONSUMED,
            EventOutcome::Scroll => {
                let editor = self.0.get_inner().read();
                let step = if matches!(
                    event,
                    Event::Mouse {
                        event: Hold(Left),
                        ..
                    }
                ) {
                    editor.opts.scroll_mouse_drag_step
                } else {
                    editor.opts.scroll_cursor_step
                };
                let cursor = editor.buffer.get_cursor();
                drop(editor);
                if let Some(cursor) = cursor {
                    self.scroll_to(cursor, step, step);
                }
                CONSUMED
            }
        }
    }

    fn scrollbar_event(&mut self, event: &Event) -> bool {
        let Event::Mouse {
            offset,
            position,
            event,
        } = event
        else {
            return false;
        };
        let origin = match event {
            Press(Left) => {
                self.0.get_inner().session_write().last_left_press = Some(*position);
                Some(*position)
            }
            Hold(Left) => self.0.get_inner().session_read().last_left_press,
            Release(Left) => self.0.get_inner().session_write().last_left_press.take(),
            _ => return false,
        };
        origin.is_some_and(|origin| self.on_scrollbar(*offset, origin))
    }

    fn on_scrollbar(&self, offset: Vec2, position: Vec2) -> bool {
        let scroller = self.0.get_scroller();
        let max = scroller.last_outer_size() + offset;
        let min = max - scroller.scrollbar_size();
        (min.x..=max.x).contains(&position.x) || (min.y..=max.y).contains(&position.y)
    }

    fn pan(&mut self, event: &Event) -> bool {
        let Event::Mouse {
            position, event, ..
        } = event
        else {
            return false;
        };
        match event {
            Press(Right) => self.0.get_inner().session_write().pan_origin = Some(*position),
            Hold(Right) => {
                let old = self
                    .0
                    .get_inner()
                    .session_write()
                    .pan_origin
                    .replace(*position);
                if let Some(old) = old {
                    let offset = self
                        .0
                        .content_viewport()
                        .top_left()
                        .map_x(|x| drag(x, position.x, old.x))
                        .map_y(|y| drag(y, position.y, old.y));
                    self.0.set_offset(offset);
                }
            }
            Release(Right) => self.0.get_inner().session_write().pan_origin = None,
            _ => return false,
        }
        true
    }

    fn relativize(&self, event: &Event) -> Event {
        let gutter = self.0.get_inner().read().opts.gutter_width;
        let mut event = event.clone();
        if let Event::Mouse {
            offset, position, ..
        } = &mut event
        {
            *position = position.saturating_sub(*offset) + self.0.content_viewport().top_left();
            position.x = position.x.saturating_sub(gutter);
        }
        event
    }

    fn scroll_to(&mut self, position: Vec2, step_x: usize, step_y: usize) {
        let gutter = self.0.get_inner().read().opts.gutter_width;
        let port = self.0.content_viewport();
        let mut offset = port.top_left();
        let x = position.x.saturating_add(gutter);
        let left = offset.x.saturating_add(gutter).saturating_sub(1);
        if x >= port.right() {
            offset.x = offset.x.saturating_add(max(
                step_x,
                x.saturating_add(1).saturating_sub(port.right()),
            ));
        } else if x < left {
            offset.x = offset
                .x
                .saturating_sub(max(min(step_x, offset.x), left - x));
        }
        if position.y >= port.bottom() {
            offset.y = offset.y.saturating_add(max(
                step_y,
                position.y.saturating_add(1).saturating_sub(port.bottom()),
            ));
        } else if position.y <= port.top() {
            offset.y = offset
                .y
                .saturating_sub(max(min(step_y, offset.y), port.top() - position.y));
        }
        self.0.set_offset(offset);
    }
}

fn drag(offset: usize, new: usize, old: usize) -> usize {
    if new > old {
        offset.saturating_sub(new - old)
    } else {
        offset.saturating_add(old - new)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{config::Options, editor::Editor, tui::dialogs::new_scrollview};
    use crate::{core::Point, editor::command::EditorCommand};
    use cursive::event::Key;

    fn view() -> ScrollView<EditorView> {
        new_scrollview(EditorView::new(
            Editor::open(Options::default()).expect("open editor"),
        ))
    }

    #[test]
    fn keyboard_movement_uses_the_command_update_path() {
        let mut view = view();
        let mut context = EditorCtx::new(&mut view);

        assert!(context.on_event(&Event::Char('3')).is_some());
        assert!(context.on_event(&Event::Char('l')).is_some());

        assert_eq!(
            context.0.get_inner().read().cursor(),
            Some(Point::new(3, 0))
        );
    }

    #[test]
    fn keyboard_box_commit_is_undoable_by_the_headless_command() {
        let mut view = view();
        let mut context = EditorCtx::new(&mut view);

        context.on_event(&Event::Char('b'));
        context.on_event(&Event::Char('3'));
        context.on_event(&Event::Char('l'));
        context.on_event(&Event::Char('2'));
        context.on_event(&Event::Char('j'));
        context.on_event(&Event::Key(Key::Enter));

        let mut editor = context.0.get_inner_mut().write();
        assert_eq!(editor.render_text(""), "┌──┐\n│  │\n└──┘\n");
        assert!(editor.execute(EditorCommand::Undo));
        assert_eq!(editor.render_text(""), "");
    }
}
