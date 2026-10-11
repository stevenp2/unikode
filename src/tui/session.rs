use cursive::{
    event::{
        Event, Key,
        MouseButton::Left,
        MouseEvent::{Hold, Press, Release},
    },
    Rect, Vec2,
};

use crate::{
    application::{self, Action},
    core::Region,
    drawing::{draw, DrawRequest, DrawingStyle, ShapeKind},
    editor::{
        command::{Direction, EditorCommand},
        operations::{move_destination, move_on_buffer},
        Editor, EditorMode,
    },
};

use super::text::TextDraft;

#[derive(Clone, Copy, Eq, PartialEq)]
pub(crate) enum ToolChoice {
    Draw(ShapeKind),
    Select,
    Text,
}

enum Interaction {
    Normal(Option<ShapeKind>),
    Drawing { kind: ShapeKind, start: Vec2 },
    Selecting { start: Vec2 },
    Moving { selection: Rect, anchor: Vec2 },
    Text(TextDraft),
}

pub(crate) enum EventOutcome {
    Ignored,
    Consumed,
    Scroll,
}

pub(crate) struct Session {
    pub(crate) editor: Editor,
    interaction: Interaction,
    pending_count: String,
    pub(crate) last_left_press: Option<Vec2>,
    pub(crate) pan_origin: Option<Vec2>,
}

impl Session {
    pub(crate) fn new(editor: Editor) -> Self {
        Self {
            editor,
            interaction: Interaction::Normal(None),
            pending_count: String::new(),
            last_left_press: None,
            pan_origin: None,
        }
    }

    pub(crate) fn reset(&mut self) {
        self.interaction = Interaction::Normal(None);
        self.pending_count.clear();
        self.last_left_press = None;
        self.pan_origin = None;
        self.editor.buffer.discard_edits();
    }

    pub(crate) fn mode(&self) -> EditorMode {
        match &self.interaction {
            Interaction::Normal(_) => EditorMode::Normal,
            Interaction::Drawing {
                kind: ShapeKind::Box,
                start,
            } => EditorMode::Box(*start),
            Interaction::Drawing {
                kind: ShapeKind::Line,
                start,
            } => EditorMode::Line(*start),
            Interaction::Drawing {
                kind: ShapeKind::Arrow,
                start,
            } => EditorMode::Arrow(*start),
            Interaction::Selecting { start } => EditorMode::Select(*start),
            Interaction::Moving { selection, .. } => EditorMode::Select(selection.top_left()),
            Interaction::Text(_) => EditorMode::Text,
        }
    }

    pub(crate) fn is_moving(&self) -> bool {
        matches!(self.interaction, Interaction::Moving { .. })
    }

    pub(crate) fn selection(&self) -> Option<Rect> {
        match self.interaction {
            Interaction::Selecting { start } => Some(Rect::from_corners(start, self.cursor())),
            Interaction::Moving { selection, anchor } => {
                let delta =
                    move_destination(selection, anchor, self.cursor()).signed() - anchor.signed();
                Some(Rect::from_corners(
                    selection.top_left().saturating_add(delta),
                    selection.bottom_right().saturating_add(delta),
                ))
            }
            _ => None,
        }
    }

    pub(crate) fn tool_label(&self) -> String {
        match &self.interaction {
            Interaction::Drawing {
                kind: ShapeKind::Box,
                ..
            } => "(Box)".into(),
            Interaction::Drawing { kind, .. } => format!(
                "({}: {:?} {:?})",
                if *kind == ShapeKind::Line {
                    "Line"
                } else {
                    "Arrow"
                },
                self.editor.opts.render_mode,
                self.editor.opts.effective_path_mode(),
            ),
            Interaction::Selecting { .. } => "(Select)".into(),
            Interaction::Moving { .. } => "(Move)".into(),
            Interaction::Text(_) => "(Text)".into(),
            Interaction::Normal(_) => String::new(),
        }
    }

    fn cursor(&self) -> Vec2 {
        self.editor.buffer.get_cursor().unwrap_or_default()
    }

    pub(crate) fn activate(&mut self, choice: ToolChoice) {
        self.finish(true);
        let start = self.cursor();
        self.pending_count.clear();
        self.interaction = match choice {
            ToolChoice::Draw(kind) => Interaction::Drawing { kind, start },
            ToolChoice::Select => Interaction::Selecting { start },
            ToolChoice::Text => Interaction::Text(TextDraft::new(start)),
        };
        self.preview();
    }

    pub(crate) fn cycle_path(&mut self) {
        if self.editor.opts.cycle_path_mode() {
            self.preview();
        }
    }

    fn dispatch(&mut self, command: EditorCommand) {
        application::update(&mut self.editor, Action::Editor(command));
    }

    fn commit(&mut self) {
        let to = self.cursor();
        match &self.interaction {
            Interaction::Drawing { kind, start } => {
                let command = match kind {
                    ShapeKind::Box => EditorCommand::DrawBox {
                        from: (*start).into(),
                        to: to.into(),
                    },
                    ShapeKind::Line => EditorCommand::DrawLine {
                        from: (*start).into(),
                        to: to.into(),
                    },
                    ShapeKind::Arrow => EditorCommand::DrawArrow {
                        from: (*start).into(),
                        to: to.into(),
                    },
                };
                self.dispatch(command);
            }
            Interaction::Moving { selection, anchor } => {
                let command = EditorCommand::MoveSelection {
                    region: Region::new(
                        selection.top_left().into(),
                        selection.bottom_right().into(),
                    ),
                    anchor: (*anchor).into(),
                    to: to.into(),
                };
                self.dispatch(command);
            }
            Interaction::Text(draft) => draft.commit(&mut self.editor),
            _ => {}
        }
        self.editor.buffer.discard_edits();
    }

    pub(crate) fn finish(&mut self, return_to_start: bool) {
        let (pointer, start) = match self.interaction {
            Interaction::Drawing { kind, start } => (Some(kind), Some(start)),
            _ => (None, None),
        };
        self.commit();
        if return_to_start && self.editor.opts.box_cursor_start {
            if let Some(start) = start {
                self.editor.buffer.set_cursor(start);
            }
        }
        self.interaction = Interaction::Normal(pointer);
        self.pending_count.clear();
    }

    fn resume_selection(&mut self) {
        let start = self.cursor();
        self.interaction = Interaction::Selecting { start };
        self.pending_count.clear();
        self.editor.buffer.discard_edits();
    }

    fn preview(&mut self) {
        let to = self.cursor();
        self.editor.buffer.discard_edits();
        match &self.interaction {
            Interaction::Drawing { kind, start } => draw(
                &mut self.editor.buffer,
                DrawRequest::new(*kind, *start, to),
                &DrawingStyle::from_options(&self.editor.opts),
            ),
            Interaction::Moving { selection, anchor } => move_on_buffer(
                &mut self.editor.buffer,
                *selection,
                *anchor,
                to,
                &self.editor.opts.symbols,
            ),
            Interaction::Text(draft) => draft.preview(&mut self.editor),
            _ => {}
        }
    }

    fn navigate(&mut self, c: char) -> Option<EventOutcome> {
        let keys = self.editor.opts.keys.clone();
        if ('1'..='9').contains(&c) || (c == '0' && !self.pending_count.is_empty()) {
            self.pending_count.push(c);
            return Some(EventOutcome::Consumed);
        }
        let direction = if c == keys.move_left {
            Some(Direction::Left)
        } else if c == keys.move_down {
            Some(Direction::Down)
        } else if c == keys.move_up {
            Some(Direction::Up)
        } else if c == keys.move_right {
            Some(Direction::Right)
        } else {
            None
        };
        if let Some(direction) = direction {
            let count = self.pending_count.parse().unwrap_or(1);
            self.pending_count.clear();
            self.dispatch(EditorCommand::MoveCursor { direction, count });
        } else if c == keys.move_line_start
            || c == keys.move_first_non_ws
            || c == keys.move_last_non_ws
        {
            self.pending_count.clear();
            let mut position = self.cursor();
            let row = self.editor.buffer.chars.get(position.y);
            position.x = if c == keys.move_line_start {
                0
            } else if c == keys.move_first_non_ws {
                row.and_then(|row| row.iter().position(|c| !c.is_whitespace()))
                    .unwrap_or(0)
            } else {
                row.and_then(|row| row.iter().rposition(|c| !c.is_whitespace()))
                    .unwrap_or(0)
            };
            self.editor.buffer.set_cursor(position);
        } else {
            return None;
        }
        self.preview();
        Some(EventOutcome::Scroll)
    }

    fn on_char(&mut self, c: char) -> EventOutcome {
        if let Some(outcome) = self.navigate(c) {
            return outcome;
        }
        let keys = self.editor.opts.keys.clone();
        if c == keys.tool_erase {
            if let Some(region) = self.selection() {
                let source = match self.interaction {
                    Interaction::Moving { selection, .. } => selection,
                    _ => region,
                };
                let cursor = self.cursor();
                self.dispatch(EditorCommand::Erase(Region::new(
                    source.top_left().into(),
                    source.bottom_right().into(),
                )));
                self.editor.buffer.set_cursor(cursor);
                self.resume_selection();
                return EventOutcome::Consumed;
            }
        }
        if c == keys.tool_move {
            if self.is_moving() {
                self.finish(false);
                return EventOutcome::Consumed;
            }
            if let Some(selection) = self.selection() {
                self.interaction = Interaction::Moving {
                    selection,
                    anchor: self.cursor(),
                };
                self.pending_count.clear();
                self.preview();
                return EventOutcome::Consumed;
            }
        }
        if c == keys.cycle_path
            && matches!(
                self.interaction,
                Interaction::Drawing {
                    kind: ShapeKind::Line | ShapeKind::Arrow,
                    ..
                }
            )
        {
            self.cycle_path();
            return EventOutcome::Consumed;
        }
        let choice = if c == keys.tool_box {
            Some(ToolChoice::Draw(ShapeKind::Box))
        } else if c == keys.tool_arrow {
            Some(ToolChoice::Draw(ShapeKind::Arrow))
        } else if c == keys.tool_line {
            Some(ToolChoice::Draw(ShapeKind::Line))
        } else if c == keys.tool_select {
            Some(ToolChoice::Select)
        } else if c == keys.tool_text {
            Some(ToolChoice::Text)
        } else {
            None
        };
        if let Some(choice) = choice {
            let same = match self.interaction {
                Interaction::Drawing { kind, .. } => choice == ToolChoice::Draw(kind),
                Interaction::Selecting { .. } => choice == ToolChoice::Select,
                _ => false,
            };
            if same {
                self.finish(true);
            } else {
                self.activate(choice);
            }
            return EventOutcome::Consumed;
        }
        if c == '\n' {
            return self.on_event(&Event::Key(Key::Enter));
        }
        self.pending_count.clear();
        if keys.is_global(c) {
            EventOutcome::Ignored
        } else {
            EventOutcome::Consumed
        }
    }

    fn on_mouse(&mut self, event: cursive::event::MouseEvent, position: Vec2) -> EventOutcome {
        if !matches!(event, Press(Left) | Hold(Left) | Release(Left)) {
            return EventOutcome::Ignored;
        }
        self.pending_count.clear();
        if let Interaction::Normal(pointer) = self.interaction {
            if let Some(kind) = pointer {
                if event == Press(Left) {
                    self.editor.buffer.set_cursor(position);
                    self.activate(ToolChoice::Draw(kind));
                }
            } else {
                if event != Release(Left) {
                    self.editor.buffer.set_cursor(position);
                    return EventOutcome::Scroll;
                }
                return EventOutcome::Ignored;
            }
        }
        match &mut self.interaction {
            Interaction::Drawing { start, .. } | Interaction::Selecting { start }
                if event == Press(Left) =>
            {
                *start = position;
            }
            _ => {}
        }
        self.editor.buffer.set_cursor(position);
        self.preview();
        if event == Release(Left) {
            match self.interaction {
                Interaction::Drawing { .. } => self.finish(false),
                Interaction::Moving { .. } => {
                    self.commit();
                    self.resume_selection();
                }
                _ => {}
            }
        }
        EventOutcome::Scroll
    }

    pub(crate) fn on_event(&mut self, event: &Event) -> EventOutcome {
        if let Interaction::Text(draft) = &mut self.interaction {
            if *event == Event::Key(Key::Esc) {
                self.finish(false);
                return EventOutcome::Consumed;
            }
            return if draft.on_event(&mut self.editor, event) {
                draft.preview(&mut self.editor);
                EventOutcome::Scroll
            } else {
                EventOutcome::Ignored
            };
        }
        match event {
            Event::Char(c) => self.on_char(*c),
            Event::Key(Key::Enter) => {
                if self.is_moving() {
                    self.commit();
                    self.resume_selection();
                } else {
                    self.finish(true);
                }
                EventOutcome::Consumed
            }
            Event::Key(Key::Esc) => {
                let active = !matches!(self.interaction, Interaction::Normal(_));
                self.finish(false);
                if active {
                    EventOutcome::Consumed
                } else {
                    EventOutcome::Ignored
                }
            }
            Event::Mouse {
                event, position, ..
            } => self.on_mouse(*event, *position),
            _ => EventOutcome::Ignored,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{config::Options, core::Point};

    fn session(options: Options, text: &str) -> Session {
        let mut editor = Editor::open(options).unwrap();
        editor.execute(EditorCommand::InsertText {
            at: Point::new(0, 0),
            text: text.into(),
        });
        Session::new(editor)
    }

    #[test]
    fn tool_switching_restores_the_configured_drawing_origin() {
        for key in ['a', 's', 't'] {
            let mut options = Options::default();
            options.box_cursor_start = true;
            let mut session = session(options, "");
            for key in ['b', '3', 'l', key] {
                session.on_event(&Event::Char(key));
            }
            assert_eq!(session.editor.cursor(), Some(Point::new(0, 0)));
            assert!(!session.editor.render_text("").is_empty());
        }
    }

    #[test]
    fn remapped_move_key_commits_and_returns_to_normal() {
        let mut options = Options::default();
        options.keys.tool_move = 'z';
        let mut session = session(options, "AB");
        for key in ['s', 'l', 'z', 'l', 'z'] {
            session.on_event(&Event::Char(key));
        }
        assert_eq!(session.editor.render_text(""), " AB\n");
        assert_eq!(session.mode(), EditorMode::Normal);
        assert!(session.editor.undo());
        assert_eq!(session.editor.render_text(""), "AB\n");
    }

    #[test]
    fn move_preview_highlight_and_commit_share_the_constrained_destination() {
        for completion in [Key::Enter, Key::Esc] {
            let mut session = session(Options::default(), "AB");
            for key in ['s', 'l', 'm', 'h'] {
                session.on_event(&Event::Char(key));
            }
            assert_eq!(session.editor.cursor(), Some(Point::new(1, 0)));
            assert_eq!(
                session.selection(),
                Some(Rect::from_corners((0, 0), (1, 0)))
            );
            assert_eq!(session.editor.buffer.get_char_at(Vec2::new(0, 0)), 'A');
            assert_eq!(session.editor.buffer.get_char_at(Vec2::new(1, 0)), 'B');
            session.on_event(&Event::Key(completion));
            assert_eq!(session.editor.render_text(""), "AB\n");
            assert_eq!(
                session.mode(),
                if completion == Key::Enter {
                    EditorMode::Select(Vec2::new(1, 0))
                } else {
                    EditorMode::Normal
                }
            );
        }
    }

    #[test]
    fn navigation_uses_counts_and_remapped_bindings_in_all_nontext_modes() {
        for choice in [
            None,
            Some(ToolChoice::Draw(ShapeKind::Box)),
            Some(ToolChoice::Select),
        ] {
            let mut options = Options::default();
            options.keys.move_right = 'f';
            options.keys.move_line_start = 'g';
            let mut session = session(options, "  AB");
            if let Some(choice) = choice {
                session.activate(choice);
            }
            for key in ['1', '0', 'f'] {
                session.on_event(&Event::Char(key));
            }
            assert_eq!(session.editor.cursor(), Some(Point::new(10, 0)));
            session.on_event(&Event::Char('g'));
            assert_eq!(session.editor.cursor(), Some(Point::new(0, 0)));
            session.on_event(&Event::Char('^'));
            assert_eq!(session.editor.cursor(), Some(Point::new(2, 0)));
            session.on_event(&Event::Char('$'));
            assert_eq!(session.editor.cursor(), Some(Point::new(3, 0)));
            assert!(matches!(
                session.on_event(&Event::Char('w')),
                EventOutcome::Ignored
            ));
            assert!(matches!(
                session.on_event(&Event::CtrlChar('r')),
                EventOutcome::Ignored
            ));
        }
    }
}
