use unikode::{
    config::Options,
    core::Point,
    editor::{command::EditorCommand, Editor},
    PathMode, RenderMode,
};

fn editor(render_mode: RenderMode, path_mode: PathMode) -> Editor {
    Editor::open(
        Options::default()
            .with_render_mode(render_mode)
            .with_path_mode(path_mode),
    )
    .expect("create editor")
}

fn character_at(editor: &Editor, point: Point) -> char {
    editor
        .render_text("")
        .lines()
        .nth(point.y)
        .and_then(|line| line.chars().nth(point.x))
        .expect("drawing contains requested position")
}

#[test]
fn line_and_arrow_commands_cover_both_path_modes() {
    let mut classic = editor(RenderMode::Classic, PathMode::Snap90);
    classic.execute(EditorCommand::DrawLine {
        from: Point::new(0, 0),
        to: Point::new(3, 2),
    });
    let classic_text = classic.render_text("");
    assert!(classic_text.contains('└'));
    assert!(classic_text.contains('─'));

    let mut braille = editor(RenderMode::Braille, PathMode::Routed);
    braille.execute(EditorCommand::DrawArrow {
        from: Point::new(0, 0),
        to: Point::new(3, 2),
    });
    let braille_text = braille.render_text("");
    assert!(braille_text
        .chars()
        .filter(|character| !character.is_whitespace())
        .all(|character| {
            ('\u{2800}'..='\u{28ff}').contains(&character)
                || ['▲', '▼', '◀', '▶'].contains(&character)
        }));
    assert!(braille_text
        .chars()
        .any(|character| ['▲', '▼', '◀', '▶'].contains(&character)));
}

#[test]
fn classic_mode_ignores_routed_diagonals() {
    let mut editor = editor(RenderMode::Classic, PathMode::Routed);
    editor.execute(EditorCommand::DrawLine {
        from: Point::new(0, 0),
        to: Point::new(3, 2),
    });

    let text = editor.render_text("");
    assert!(text.contains('─'));
    assert!(text.contains('│'));
    assert!(!text.contains('╱'));
    assert!(!text.contains('╲'));
    assert!(!text
        .chars()
        .any(|character| ('\u{2800}'..='\u{28ff}').contains(&character)));
}

#[test]
fn braille_geometry_round_trips_through_undo_and_redo() {
    let mut editor = editor(RenderMode::Braille, PathMode::Routed);
    editor.execute(EditorCommand::DrawArrow {
        from: Point::new(0, 0),
        to: Point::new(4, 3),
    });
    let drawing = editor.render_text("");

    assert!(editor.execute(EditorCommand::Undo));
    assert_ne!(editor.render_text(""), drawing);
    assert!(editor.execute(EditorCommand::Redo));
    assert_eq!(editor.render_text(""), drawing);
}

#[test]
fn shape_started_on_an_arrow_tip_replaces_it_with_a_braille_connector() {
    let mut editor = editor(RenderMode::Braille, PathMode::Routed);
    let junction = Point::new(4, 3);
    editor.execute(EditorCommand::DrawArrow {
        from: Point::new(1, 1),
        to: junction,
    });
    assert!(['▲', '▼', '◀', '▶'].contains(&character_at(&editor, junction)));

    editor.execute(EditorCommand::DrawLine {
        from: junction,
        to: Point::new(7, 1),
    });
    assert!(('\u{2800}'..='\u{28ff}').contains(&character_at(&editor, junction)));
}

#[test]
fn box_overlapping_an_arrow_tip_replaces_it_with_a_braille_connector() {
    let mut editor = editor(RenderMode::Braille, PathMode::Routed);
    let overlap = Point::new(4, 3);
    editor.execute(EditorCommand::DrawArrow {
        from: Point::new(1, 1),
        to: overlap,
    });
    editor.execute(EditorCommand::DrawBox {
        from: Point::new(2, 1),
        to: Point::new(6, 3),
    });

    assert!(('\u{2800}'..='\u{28ff}').contains(&character_at(&editor, overlap)));
}
