use cursive::{
    event::{
        Event, EventResult, Key,
        MouseButton::{Left, Right},
        MouseEvent::{Hold, Press, Release},
    },
    view::scroll::Scroller,
    views::ScrollView,
    Vec2,
};

use std::cmp::{max, min};

use crate::application::{self, Action, Effect, Update};
use crate::constants::CONSUMED;

use crate::core::{Point, Region};
use crate::drawing::{draw, DrawRequest, DrawingStyle, ShapeKind};
use crate::editor::{
    buffer::Buffer,
    command::{Direction, EditorCommand},
    EditorMode,
};
use crate::tools::lines::{ArrowShape, BoxShape, DrawingShape, DrawingTool, LineShape};
use crate::tools::movetool::{move_on_buffer, MoveTool};
use crate::tools::selecttool::SelectTool;
use crate::tools::texttool::TextTool;
use crate::tools::Tool;
use crate::tui::EditorView;
use cursive::Rect;

macro_rules! intercept_scrollbar {
    ($ctx:expr, $event:expr) => {{
        if let Event::Mouse {
            offset,
            position: pos,
            event,
        } = $event
        {
            match event {
                Press(Left) if $ctx.on_scrollbar(*offset, *pos) => {
                    $ctx.0.get_inner_mut().write().last_left_press = Some(*pos);
                    return None;
                }

                Press(Left) => {
                    $ctx.0.get_inner_mut().write().last_left_press = Some(*pos);
                }

                Hold(Left)
                    if $ctx
                        .0
                        .get_inner()
                        .read()
                        .last_left_press
                        .map(|pos| $ctx.on_scrollbar(*offset, pos))
                        .unwrap_or(false) =>
                {
                    return None;
                }

                Release(Left)
                    if $ctx
                        .take_last_left_press()
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
        if let Event::Mouse {
            position: pos,
            event,
            ..
        } = $event
        {
            match event {
                Press(Right) => {
                    $ctx.0.get_inner_mut().write().pan_origin = Some(*pos);
                    return CONSUMED;
                }

                Hold(Right) if $ctx.0.get_inner().read().pan_origin.is_none() => {
                    $ctx.0.get_inner_mut().write().pan_origin = Some(*pos);
                    return CONSUMED;
                }

                Hold(Right) => {
                    let old = $ctx
                        .0
                        .get_inner_mut()
                        .write()
                        .pan_origin
                        .replace(*pos)
                        .expect("pan origin initialized by press or hold");

                    let offset = ($ctx.0.content_viewport())
                        .top_left()
                        .map_x(|x| drag(x, pos.x, old.x))
                        .map_y(|y| drag(y, pos.y, old.y));

                    $ctx.0.set_offset(offset);

                    return CONSUMED;
                }

                Release(Right) => {
                    $ctx.0.get_inner_mut().write().pan_origin = None;
                    return CONSUMED;
                }

                _ => {}
            }
        }
    }};
}

pub(crate) struct EditorCtx<'a>(pub(crate) &'a mut ScrollView<EditorView>);

impl<'a> EditorCtx<'a> {
    /// Returns a new `EditorCtx`.
    pub fn new(view: &'a mut ScrollView<EditorView>) -> Self {
        Self(view)
    }

    /// Apply an editor command through the framework-neutral update boundary.
    pub(crate) fn dispatch(&mut self, command: EditorCommand) -> Update {
        let update = {
            let mut editor = self.0.get_inner_mut().write();
            application::update(&mut editor, Action::Editor(command))
        };
        if update.effects.contains(&Effect::ScrollToCursor) {
            self.scroll_to_cursor();
        }
        update
    }

    fn commit_mode(&mut self, mode: EditorMode, start: Vec2, end: Vec2) {
        let command = match mode {
            EditorMode::Box(_) => EditorCommand::DrawBox {
                from: Point::from(start),
                to: Point::from(end),
            },
            EditorMode::Line(_) => EditorCommand::DrawLine {
                from: Point::from(start),
                to: Point::from(end),
            },
            EditorMode::Arrow(_) => EditorCommand::DrawArrow {
                from: Point::from(start),
                to: Point::from(end),
            },
            _ => return,
        };
        self.dispatch(command);
    }

    fn preview_drawing(&mut self, kind: ShapeKind, from: Vec2, to: Vec2, style: DrawingStyle) {
        self.preview(|buffer| draw(buffer, DrawRequest::new(kind, from, to), &style));
    }

    fn preview_mode(&mut self, mode: EditorMode, from: Vec2, to: Vec2, style: DrawingStyle) {
        if let Some((kind, _)) = mode.drawing() {
            self.preview_drawing(kind, from, to, style);
        }
    }

    fn activate_drawing<S: DrawingShape + 'static>(&mut self, position: Vec2) {
        let mut editor = self.0.get_inner_mut().write();
        editor.pending_count.clear();
        editor.mode = S::mode(position);
        editor.set_tool(DrawingTool::<S>::default());
        let style = DrawingStyle::from_options(&editor.opts);
        drop(editor);
        self.preview_drawing(S::KIND, position, position, style);
    }

    fn take_last_left_press(&mut self) -> Option<Vec2> {
        self.0.get_inner_mut().write().last_left_press.take()
    }

    /// Handles an event using the active tool.
    pub fn on_event(&mut self, event: &Event) -> Option<EventResult> {
        intercept_scrollbar!(self, event);
        intercept_pan!(self, event);

        let mode = self.0.get_inner_mut().read().mode;

        // Tool Delegation (Highest Priority)
        if !matches!(mode, EditorMode::Normal) {
            let tool_opt = self.0.get_inner_mut().write().active_tool.take();
            if let Some(mut tool) = tool_opt {
                let res = tool.on_event(self, event);

                let mut editor = self.0.get_inner_mut().write();
                // Only put the tool back if it wasn't replaced by another tool (e.g. SelectTool)
                if editor.active_tool.is_none() {
                    editor.active_tool = Some(tool);
                }

                if let Event::Mouse {
                    event: Release(Left),
                    ..
                } = event
                {
                    if !matches!(editor.mode, EditorMode::Select(_)) {
                        editor.mode = EditorMode::Normal;
                        editor.pending_count.clear();
                    }
                    return CONSUMED;
                }

                if res.is_some() {
                    return res;
                }
            }
        }

        // 1. Box, Arrow, & Select Mode Handling (Keyboard)
        if let EditorMode::Box(start)
        | EditorMode::Arrow(start)
        | EditorMode::Line(start)
        | EditorMode::Select(start) = mode
        {
            if let Event::Char(c) = event {
                let c = *c;
                let keys = self.0.get_inner_mut().read().opts.keys.clone();

                if ('1'..='9').contains(&c) {
                    let mut editor = self.0.get_inner_mut().write();
                    editor.pending_count.push(c);
                    return CONSUMED;
                } else if c == keys.move_line_start {
                    let mut editor = self.0.get_inner_mut().write();
                    if editor.pending_count.is_empty() {
                        let mut pos = editor
                            .buffer
                            .get_cursor()
                            .unwrap_or_else(|| Vec2::new(0, 0));
                        pos.x = 0;
                        editor.buffer.set_cursor(pos);
                        drop(editor);
                        self.scroll_to_cursor();
                        return CONSUMED;
                    } else {
                        editor.pending_count.push(c);
                        return CONSUMED;
                    }
                } else if c == keys.move_first_non_ws {
                    let mut editor = self.0.get_inner_mut().write();
                    editor.pending_count.clear();
                    let mut pos = editor
                        .buffer
                        .get_cursor()
                        .unwrap_or_else(|| Vec2::new(0, 0));
                    if let Some(line) = editor.buffer.chars.get(pos.y) {
                        pos.x = line.iter().position(|&ch| !ch.is_whitespace()).unwrap_or(0);
                    } else {
                        pos.x = 0;
                    }
                    editor.buffer.set_cursor(pos);
                    drop(editor);
                    self.scroll_to_cursor();
                    return CONSUMED;
                } else if c == keys.move_last_non_ws {
                    let mut editor = self.0.get_inner_mut().write();
                    editor.pending_count.clear();
                    let mut pos = editor
                        .buffer
                        .get_cursor()
                        .unwrap_or_else(|| Vec2::new(0, 0));
                    if let Some(line) = editor.buffer.chars.get(pos.y) {
                        pos.x = line
                            .iter()
                            .rposition(|&ch| !ch.is_whitespace())
                            .unwrap_or(0);
                    } else {
                        pos.x = 0;
                    }
                    editor.buffer.set_cursor(pos);
                    drop(editor);
                    self.scroll_to_cursor();
                    return CONSUMED;
                } else if keys.is_movement(c) {
                    let count = {
                        let mut editor = self.0.get_inner_mut().write();
                        let count = editor.pending_count.parse::<usize>().unwrap_or(1).max(1);
                        editor.pending_count.clear();
                        count
                    };
                    let direction = if c == keys.move_left {
                        Direction::Left
                    } else if c == keys.move_down {
                        Direction::Down
                    } else if c == keys.move_up {
                        Direction::Up
                    } else {
                        Direction::Right
                    };
                    self.dispatch(EditorCommand::MoveCursor { direction, count });
                    let editor = self.0.get_inner_mut().read();
                    let pos = editor.buffer.get_cursor().unwrap_or_default();
                    let style = DrawingStyle::from_options(&editor.opts);
                    drop(editor);

                    if mode.drawing().is_some() {
                        self.preview_mode(mode, start, pos, style);
                    } else if matches!(mode, EditorMode::Select(_)) {
                        self.preview(|_| ());
                    }
                    return CONSUMED;
                } else if c == keys.cycle_path
                    && matches!(mode, EditorMode::Arrow(_) | EditorMode::Line(_))
                {
                    let mut editor = self.0.get_inner_mut().write();
                    let changed = editor.opts.cycle_path_mode();
                    if !changed {
                        return CONSUMED;
                    }
                    let options = editor.opts.clone();
                    if let Some(tool) = editor.active_tool.as_mut() {
                        tool.load_opts(&options);
                    }
                    let pos = editor
                        .buffer
                        .get_cursor()
                        .unwrap_or_else(|| Vec2::new(0, 0));
                    let style = DrawingStyle::from_options(&editor.opts);
                    drop(editor);
                    self.preview_mode(mode, start, pos, style);
                    return CONSUMED;
                } else if c == keys.tool_erase && matches!(mode, EditorMode::Select(_)) {
                    let end = self
                        .0
                        .get_inner_mut()
                        .read()
                        .buffer
                        .get_cursor()
                        .unwrap_or_else(|| Vec2::new(0, 0));
                    self.dispatch(EditorCommand::Erase(Region::new(
                        Point::from(start),
                        Point::from(end),
                    )));
                    let mut editor = self.0.get_inner_mut().write();
                    editor.mode = EditorMode::Select(end);
                    editor.buffer.discard_edits();
                    editor.pending_count.clear();
                    editor.buffer.set_cursor(end);
                    return CONSUMED;
                } else if c == keys.tool_move && matches!(mode, EditorMode::Select(_)) {
                    let end = self
                        .0
                        .get_inner_mut()
                        .read()
                        .buffer
                        .get_cursor()
                        .unwrap_or_else(|| Vec2::new(0, 0));
                    let mut editor = self.0.get_inner_mut().write();
                    let selection = Rect::from_corners(start, end);
                    let anchor = end;
                    let symbols = editor.opts.symbols.clone();
                    let tool = MoveTool::new(selection, anchor);
                    editor.set_tool(tool);
                    editor.pending_count.clear();
                    drop(editor);
                    self.preview(|buf| move_on_buffer(buf, selection, anchor, anchor, &symbols));
                    return CONSUMED;
                } else if c == keys.tool_box
                    || c == keys.tool_arrow
                    || c == keys.tool_line
                    || c == keys.tool_select
                    || c == keys.tool_text
                    || c == '\n'
                {
                    let end = self
                        .0
                        .get_inner_mut()
                        .read()
                        .buffer
                        .get_cursor()
                        .unwrap_or_else(|| Vec2::new(0, 0));
                    self.commit_mode(mode, start, end);

                    let mut editor = self.0.get_inner_mut().write();
                    editor.buffer.discard_edits();
                    editor.pending_count.clear();

                    let cursor_pos = if editor.opts.box_cursor_start {
                        start
                    } else {
                        end
                    };
                    editor.buffer.set_cursor(cursor_pos);

                    if c == keys.tool_box && !matches!(mode, EditorMode::Box(_)) {
                        drop(editor);
                        self.activate_drawing::<BoxShape>(cursor_pos);
                    } else if c == keys.tool_arrow && !matches!(mode, EditorMode::Arrow(_)) {
                        drop(editor);
                        self.activate_drawing::<ArrowShape>(cursor_pos);
                    } else if c == keys.tool_line && !matches!(mode, EditorMode::Line(_)) {
                        drop(editor);
                        self.activate_drawing::<LineShape>(cursor_pos);
                    } else if c == keys.tool_select {
                        editor.mode = EditorMode::Select(cursor_pos);
                        let mut tool = SelectTool::default();
                        tool.load_opts(&editor.opts);
                        editor.set_tool(tool);
                    } else if c == keys.tool_text {
                        editor.mode = EditorMode::Text;
                        let mut tool = TextTool::new(cursor_pos);
                        tool.load_opts(&editor.opts);
                        editor.set_tool(tool);
                    } else {
                        editor.mode = EditorMode::Normal;
                    }
                    return CONSUMED;
                } else if keys.is_global(c) {
                    return None;
                } else {
                    return CONSUMED;
                }
            } else if let Event::CtrlChar('r') = event {
                return None;
            } else if let Event::Key(Key::Esc) = event {
                let end = self
                    .0
                    .get_inner_mut()
                    .read()
                    .buffer
                    .get_cursor()
                    .unwrap_or_else(|| Vec2::new(0, 0));
                self.commit_mode(mode, start, end);

                let mut editor = self.0.get_inner_mut().write();
                editor.mode = EditorMode::Normal;
                editor.buffer.discard_edits();
                editor.pending_count.clear();
                return CONSUMED;
            } else if let Event::Key(Key::Enter) = event {
                let end = self
                    .0
                    .get_inner_mut()
                    .read()
                    .buffer
                    .get_cursor()
                    .unwrap_or_else(|| Vec2::new(0, 0));
                self.commit_mode(mode, start, end);

                let mut editor = self.0.get_inner_mut().write();
                editor.mode = EditorMode::Normal;
                editor.buffer.discard_edits();
                editor.pending_count.clear();
                if editor.opts.box_cursor_start {
                    editor.buffer.set_cursor(start);
                } else {
                    editor.buffer.set_cursor(end);
                }
                return CONSUMED;
            }
            return CONSUMED;
        }

        // 3. Normal Mode Handling
        if mode == EditorMode::Normal {
            if let Event::Char(c) = event {
                let c = *c;
                let keys = self.0.get_inner_mut().read().opts.keys.clone();

                if ('1'..='9').contains(&c) {
                    let mut editor = self.0.get_inner_mut().write();
                    editor.pending_count.push(c);
                    return CONSUMED;
                } else if c == keys.move_line_start {
                    let mut editor = self.0.get_inner_mut().write();
                    if editor.pending_count.is_empty() {
                        let mut pos = editor
                            .buffer
                            .get_cursor()
                            .unwrap_or_else(|| Vec2::new(0, 0));
                        pos.x = 0;
                        editor.buffer.set_cursor(pos);
                        drop(editor);
                        self.scroll_to_cursor();
                        return CONSUMED;
                    } else {
                        editor.pending_count.push(c);
                        return CONSUMED;
                    }
                } else if c == keys.move_first_non_ws {
                    let mut editor = self.0.get_inner_mut().write();
                    editor.pending_count.clear();
                    let mut pos = editor
                        .buffer
                        .get_cursor()
                        .unwrap_or_else(|| Vec2::new(0, 0));
                    if let Some(line) = editor.buffer.chars.get(pos.y) {
                        pos.x = line.iter().position(|&ch| !ch.is_whitespace()).unwrap_or(0);
                    } else {
                        pos.x = 0;
                    }
                    editor.buffer.set_cursor(pos);
                    drop(editor);
                    self.scroll_to_cursor();
                    return CONSUMED;
                } else if c == keys.move_last_non_ws {
                    let mut editor = self.0.get_inner_mut().write();
                    editor.pending_count.clear();
                    let mut pos = editor
                        .buffer
                        .get_cursor()
                        .unwrap_or_else(|| Vec2::new(0, 0));
                    if let Some(line) = editor.buffer.chars.get(pos.y) {
                        pos.x = line
                            .iter()
                            .rposition(|&ch| !ch.is_whitespace())
                            .unwrap_or(0);
                    } else {
                        pos.x = 0;
                    }
                    editor.buffer.set_cursor(pos);
                    drop(editor);
                    self.scroll_to_cursor();
                    return CONSUMED;
                } else if keys.is_movement(c) {
                    let count = {
                        let mut editor = self.0.get_inner_mut().write();
                        let count = editor.pending_count.parse::<usize>().unwrap_or(1).max(1);
                        editor.pending_count.clear();
                        count
                    };
                    let direction = if c == keys.move_left {
                        Direction::Left
                    } else if c == keys.move_down {
                        Direction::Down
                    } else if c == keys.move_up {
                        Direction::Up
                    } else {
                        Direction::Right
                    };
                    self.dispatch(EditorCommand::MoveCursor { direction, count });
                    return CONSUMED;
                } else if c == keys.tool_box {
                    let position = self
                        .0
                        .get_inner_mut()
                        .read()
                        .buffer
                        .get_cursor()
                        .unwrap_or_default();
                    self.activate_drawing::<BoxShape>(position);
                    return CONSUMED;
                } else if c == keys.tool_arrow {
                    let position = self
                        .0
                        .get_inner_mut()
                        .read()
                        .buffer
                        .get_cursor()
                        .unwrap_or_default();
                    self.activate_drawing::<ArrowShape>(position);
                    return CONSUMED;
                } else if c == keys.tool_select {
                    let mut editor = self.0.get_inner_mut().write();
                    editor.pending_count.clear();
                    let pos = editor.buffer.get_cursor().unwrap_or(Vec2::new(0, 0));
                    editor.mode = EditorMode::Select(pos);
                    let mut tool = SelectTool::default();
                    tool.load_opts(&editor.opts);
                    editor.set_tool(tool);
                    return CONSUMED;
                } else if c == keys.tool_text {
                    let mut editor = self.0.get_inner_mut().write();
                    editor.pending_count.clear();
                    let pos = editor.buffer.get_cursor().unwrap_or(Vec2::new(0, 0));
                    editor.mode = EditorMode::Text;
                    let mut tool = TextTool::new(pos);
                    tool.load_opts(&editor.opts);
                    editor.set_tool(tool);
                    return CONSUMED;
                } else if c == keys.tool_line {
                    let position = self
                        .0
                        .get_inner_mut()
                        .read()
                        .buffer
                        .get_cursor()
                        .unwrap_or_default();
                    self.activate_drawing::<LineShape>(position);
                    return CONSUMED;
                } else if keys.is_global(c) {
                    let mut editor = self.0.get_inner_mut().write();
                    editor.pending_count.clear();
                    return None;
                } else {
                    let mut editor = self.0.get_inner_mut().write();
                    editor.pending_count.clear();
                    return CONSUMED;
                }
            } else if let Event::Mouse {
                event: Press(Left) | Hold(Left),
                ..
            } = event
            {
                {
                    let mut editor = self.0.get_inner_mut().write();
                    editor.pending_count.clear();
                }
                let rel_event = self.relativize(event);
                if let Event::Mouse { position, .. } = rel_event {
                    let mut editor = self.0.get_inner_mut().write();
                    editor.buffer.set_cursor(position);
                    drop(editor);
                    self.scroll_to_cursor();
                    return CONSUMED;
                }
            } else if let Event::Key(Key::Esc) = event {
                let mut editor = self.0.get_inner_mut().write();
                editor.pending_count.clear();
            }
        }

        // 4. Tool Delegation (Lower Priority for Normal Mode / Fallback)
        let mut editor_guard = self.0.get_inner_mut().write();
        let tool_exists = editor_guard.active_tool.is_some();
        if tool_exists {
            let mut tool = editor_guard.active_tool.take().unwrap();
            drop(editor_guard);

            let res = tool.on_event(self, event);

            self.0.get_inner_mut().write().active_tool = Some(tool);

            if mode == EditorMode::Text && res.is_none() {
                if let Event::Key(Key::Esc) = event {
                    let mut editor = self.0.get_inner_mut().write();
                    editor.mode = EditorMode::Normal;
                    return CONSUMED;
                }
            }

            if res.is_some() {
                return res;
            }
        } else {
            drop(editor_guard);
        }

        None
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
        let gutter_width = self.0.get_inner().read().opts.gutter_width;
        let mut event = event.clone();
        if let Event::Mouse {
            offset, position, ..
        } = &mut event
        {
            let tl = self.0.content_viewport().top_left();
            *position = position.saturating_sub(*offset) + tl;

            // Adjust for gutter
            if position.x >= gutter_width {
                position.x -= gutter_width;
            } else {
                position.x = 0; // Clamp to 0 if inside gutter
            }
        }
        event
    }

    /// Scroll to `pos`, moving at least `step_x` & `step_y` respectively if the x or y
    /// scroll offset needs to be modified.
    pub(crate) fn scroll_to(&mut self, pos: Vec2, step_x: usize, step_y: usize) {
        let gutter_width = self.0.get_inner().read().opts.gutter_width;
        let port = self.0.content_viewport();
        let mut offset = port.top_left();

        // View X = Buffer_X + gutter_width
        let v_x = pos.x + gutter_width;

        if v_x >= port.right() {
            offset.x += max(step_x, v_x + 1 - port.right());
        } else if v_x < offset.x + gutter_width - 1 {
            offset.x -= max(min(step_x, offset.x), (offset.x + gutter_width - 1) - v_x);
        }

        if pos.y >= port.bottom() {
            offset.y += max(step_y, pos.y + 1 - port.bottom());
        } else if pos.y <= port.top() {
            offset.y -= max(min(step_y, offset.y), port.top() - pos.y);
        }
        self.0.set_offset(offset);
    }

    /// Scroll to the edit buffer's current cursor, if one exists.
    pub(crate) fn scroll_to_cursor(&mut self) {
        let (pos, step) = {
            let editor = self.0.get_inner_mut().read();
            (editor.buffer.get_cursor(), editor.opts.scroll_cursor_step)
        };
        if let Some(pos) = pos {
            self.scroll_to(pos, step, step);
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{config::Options, editor::Editor, tui::dialogs::new_scrollview};

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
