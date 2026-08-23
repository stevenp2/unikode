//! Framework-neutral application update boundary.

use crate::editor::{command::EditorCommand, Editor};

/// A user or system intent that can update application state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Action {
    Editor(EditorCommand),
}

/// Work requested after an action has updated state.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Effect {
    Redraw,
    ScrollToCursor,
}

/// Result of applying one action.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Update {
    pub document_changed: bool,
    pub effects: Vec<Effect>,
}

/// Apply an action through the same boundary used by headless tests and the TUI.
pub fn update(editor: &mut Editor, action: Action) -> Update {
    match action {
        Action::Editor(command) => {
            let scroll = matches!(command, EditorCommand::MoveCursor { .. });
            let document_changed = editor.execute(command);
            let mut effects = vec![Effect::Redraw];
            if scroll {
                effects.push(Effect::ScrollToCursor);
            }
            Update {
                document_changed,
                effects,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        config::Options,
        editor::command::{Direction, EditorCommand},
    };

    #[test]
    fn movement_requests_scroll_without_marking_document_changed() {
        let mut editor = Editor::open(Options::default()).expect("open editor");
        let result = update(
            &mut editor,
            Action::Editor(EditorCommand::MoveCursor {
                direction: Direction::Right,
                count: 2,
            }),
        );

        assert!(!result.document_changed);
        assert_eq!(result.effects, vec![Effect::Redraw, Effect::ScrollToCursor]);
        assert_eq!(editor.cursor().map(|point| point.x), Some(2));
    }
}
