use cursive::{
    event::{Event, Key, MouseButton::Left, MouseEvent},
    Vec2,
};

use crate::{
    application::{update, Action},
    core::Point,
    editor::{command::EditorCommand, Editor},
};

pub(crate) struct TextDraft {
    source: Vec2,
    rows: Vec<Vec<char>>,
    cursor: Vec2,
}

impl TextDraft {
    pub(crate) fn new(source: Vec2) -> Self {
        Self {
            source,
            rows: vec![vec![]],
            cursor: Vec2::zero(),
        }
    }

    pub(crate) fn on_event(&mut self, editor: &mut Editor, event: &Event) -> bool {
        let Vec2 { x, y } = &mut self.cursor;
        match event {
            Event::Mouse {
                event: MouseEvent::Press(Left),
                position,
                ..
            } => {
                self.commit(editor);
                *self = Self::new(*position);
            }
            Event::Mouse {
                event: MouseEvent::Release(Left),
                ..
            } => {}
            Event::Key(Key::Enter) | Event::Char('\n') => {
                let next = self.rows[*y].split_off(*x);
                self.rows.insert(*y + 1, next);
                *x = 0;
                *y += 1;
            }
            Event::Char(character) => {
                self.rows[*y].insert(*x, *character);
                *x += 1;
            }
            Event::Key(Key::Up) => {
                *y = y.saturating_sub(1);
                *x = (*x).min(self.rows[*y].len());
            }
            Event::Key(Key::Down) => {
                *y = (*y + 1).min(self.rows.len() - 1);
                *x = (*x).min(self.rows[*y].len());
            }
            Event::Key(Key::Left) => *x = x.saturating_sub(1),
            Event::Key(Key::Right) => *x = (*x + 1).min(self.rows[*y].len()),
            Event::Key(Key::Backspace) | Event::Key(Key::Del) => {
                if *x > 0 {
                    self.rows[*y].remove(*x - 1);
                    *x -= 1;
                } else if *y > 0 {
                    let mut next = self.rows.remove(*y);
                    *y -= 1;
                    *x = self.rows[*y].len();
                    self.rows[*y].append(&mut next);
                }
            }
            _ => return false,
        }
        true
    }

    pub(crate) fn preview(&self, editor: &mut Editor) {
        editor.buffer.discard_edits();
        for (y, row) in self.rows.iter().enumerate() {
            for (x, character) in row.iter().enumerate() {
                editor.buffer.setv(
                    true,
                    self.source + Vec2::new(x, y),
                    *character,
                    &editor.opts.symbols,
                );
            }
        }
        editor.buffer.set_cursor(self.source + self.cursor);
    }

    pub(crate) fn commit(&self, editor: &mut Editor) {
        let text = self
            .rows
            .iter()
            .map(|row| row.iter().collect::<String>())
            .collect::<Vec<_>>()
            .join("\n");
        update(
            editor,
            Action::Editor(EditorCommand::InsertText {
                at: Point::from(self.source),
                text,
            }),
        );
        editor.buffer.set_cursor(self.source + self.cursor);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Options;

    #[test]
    fn unicode_draft_commits_as_one_undoable_change() {
        let mut editor = Editor::open(Options::default()).expect("open editor");
        let mut draft = TextDraft::new(Vec2::zero());
        for event in [
            Event::Char('界'),
            Event::Char('é'),
            Event::Key(Key::Left),
            Event::Key(Key::Enter),
            Event::Key(Key::Backspace),
            Event::Key(Key::Del),
        ] {
            assert!(draft.on_event(&mut editor, &event));
        }
        draft.preview(&mut editor);
        assert_eq!(editor.buffer.get_char_at(Vec2::zero()), 'é');
        assert_eq!(editor.render_text(""), "");
        draft.commit(&mut editor);
        assert_eq!(editor.render_text(""), "é\n");
        assert_eq!(editor.cursor(), Some(Point::new(0, 0)));
        assert!(editor.undo());
        assert_eq!(editor.render_text(""), "");
        assert!(!editor.undo());
    }

    #[test]
    fn preview_discards_deleted_characters_and_movement_stays_in_rows() {
        let mut editor = Editor::open(Options::default()).expect("open editor");
        let mut draft = TextDraft::new(Vec2::new(2, 3));
        for event in [Event::Char('a'), Event::Char('b')] {
            assert!(draft.on_event(&mut editor, &event));
        }
        draft.preview(&mut editor);
        assert_eq!(editor.buffer.get_char_at(Vec2::new(3, 3)), 'b');
        for event in [
            Event::Key(Key::Backspace),
            Event::Key(Key::Up),
            Event::Key(Key::Down),
            Event::Key(Key::Right),
        ] {
            assert!(draft.on_event(&mut editor, &event));
        }
        draft.preview(&mut editor);
        assert_eq!(editor.buffer.get_char_at(Vec2::new(3, 3)), ' ');
        assert_eq!(editor.cursor(), Some(Point::new(3, 3)));
    }

    #[test]
    fn mouse_press_commits_and_release_keeps_the_new_draft_active() {
        let mut editor = Editor::open(Options::default()).expect("open editor");
        let mut draft = TextDraft::new(Vec2::zero());
        assert!(draft.on_event(&mut editor, &Event::Char('a')));
        for mouse in [MouseEvent::Press(Left), MouseEvent::Release(Left)] {
            assert!(draft.on_event(
                &mut editor,
                &Event::Mouse {
                    offset: Vec2::zero(),
                    position: Vec2::new(2, 0),
                    event: mouse,
                },
            ));
        }
        assert!(draft.on_event(&mut editor, &Event::Char('b')));
        assert!(!draft.on_event(&mut editor, &Event::Key(Key::Esc)));
        draft.commit(&mut editor);
        assert_eq!(editor.render_text(""), "a b\n");
    }

    #[test]
    fn character_newline_matches_enter_in_preview_commit_and_cursor() {
        for newline in [Event::Char('\n'), Event::Key(Key::Enter)] {
            let mut editor = Editor::open(Options::default()).unwrap();
            let mut draft = TextDraft::new(Vec2::zero());
            for event in [Event::Char('a'), newline, Event::Char('b')] {
                assert!(draft.on_event(&mut editor, &event));
            }
            draft.preview(&mut editor);
            assert_eq!(editor.buffer.get_char_at(Vec2::new(0, 1)), 'b');
            assert_eq!(editor.cursor(), Some(Point::new(1, 1)));
            draft.commit(&mut editor);
            assert_eq!(editor.render_text(""), "a\nb\n");
            assert_eq!(editor.cursor(), Some(Point::new(1, 1)));
        }
    }
}
