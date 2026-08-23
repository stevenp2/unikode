use unikode::{
    config::Options,
    core::{Point, Region},
    editor::{command::EditorCommand, Editor},
};

fn editor() -> Editor {
    Editor::open(Options::default()).expect("create editor")
}

#[test]
fn headless_commands_build_and_modify_a_unicode_diagram() {
    let mut editor = editor();

    assert!(editor.execute(EditorCommand::DrawBox {
        from: Point::new(0, 0),
        to: Point::new(4, 2),
    }));
    assert!(editor.execute(EditorCommand::InsertText {
        at: Point::new(1, 1),
        text: "猫".into(),
    }));

    assert_eq!(editor.render_text(""), "┌───┐\n│猫  │\n└───┘\n");

    assert!(editor.execute(EditorCommand::MoveSelection {
        region: Region::new(Point::new(1, 1), Point::new(1, 1)),
        to: Point::new(2, 1),
    }));
    assert_eq!(editor.render_text(""), "┌───┐\n│ 猫 │\n└───┘\n");

    assert!(editor.execute(EditorCommand::Erase(Region::new(
        Point::new(2, 1),
        Point::new(2, 1),
    ))));
    assert_eq!(editor.render_text(""), "┌───┐\n│   │\n└───┘\n");
}

#[test]
fn a_new_edit_after_undo_invalidates_redo() {
    let mut editor = editor();
    editor.execute(EditorCommand::InsertText {
        at: Point::new(0, 0),
        text: "first".into(),
    });
    assert!(editor.execute(EditorCommand::Undo));

    editor.execute(EditorCommand::InsertText {
        at: Point::new(0, 0),
        text: "second".into(),
    });

    assert!(!editor.execute(EditorCommand::Redo));
    assert_eq!(editor.render_text(""), "second\n");
}

#[test]
fn cursor_movement_is_counted_and_saturates_at_zero() {
    use unikode::editor::command::Direction;

    let mut editor = editor();
    editor.execute(EditorCommand::MoveCursor {
        direction: Direction::Right,
        count: 4,
    });
    editor.execute(EditorCommand::MoveCursor {
        direction: Direction::Down,
        count: 3,
    });
    editor.execute(EditorCommand::MoveCursor {
        direction: Direction::Left,
        count: 20,
    });
    assert_eq!(editor.cursor(), Some(Point::new(0, 3)));
}

#[test]
fn line_and_arrow_commands_cover_both_path_modes() {
    use unikode::tools::PathMode;

    let mut snapped = editor();
    snapped.execute(EditorCommand::DrawLine {
        from: Point::new(0, 0),
        to: Point::new(3, 2),
    });
    let snapped_text = snapped.render_text("");
    assert!(snapped_text.contains('└'));
    assert!(snapped_text.contains('─'));

    let options = Options {
        path_mode: PathMode::Routed,
        ..Options::default()
    };
    let mut routed = Editor::open(options).unwrap();
    routed.execute(EditorCommand::DrawArrow {
        from: Point::new(0, 0),
        to: Point::new(3, 2),
    });
    assert!(routed
        .render_text("")
        .chars()
        .any(|character| ['▲', '▼', '◀', '▶'].contains(&character)));
}
