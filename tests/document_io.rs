mod common;

use std::fs;
use unikode::{
    config::Options,
    core::Point,
    editor::{command::EditorCommand, Editor},
};

use common::TestDir;

#[test]
fn unicode_documents_round_trip_and_track_saved_state() {
    let dir = TestDir::new("round-trip");
    let file = dir.join("diagram.txt");
    let mut editor = Editor::open(Options::default()).unwrap();
    editor.execute(EditorCommand::InsertText {
        at: Point::new(0, 0),
        text: "λ → 猫".into(),
    });
    assert!(editor.is_dirty());

    editor.save_as(&file).unwrap();
    assert!(!editor.is_dirty());
    assert_eq!(fs::read_to_string(&file).unwrap(), "λ → 猫\n");

    let options = Options::default().with_file(file.clone());
    let reopened = Editor::open(options).unwrap();
    assert_eq!(reopened.render_text(""), "λ → 猫\n");
    assert_eq!(reopened.path(), Some(&file));
}

#[test]
fn failed_save_as_preserves_the_previous_path() {
    let dir = TestDir::new("failed-save");
    let original = dir.join("original.txt");
    let mut editor = Editor::open(Options::default()).unwrap();
    editor.save_as(&original).unwrap();

    let error = editor.save_as(dir.path()).unwrap_err();
    assert!(error.kind() != std::io::ErrorKind::NotFound);
    assert_eq!(editor.path(), Some(&original));
}

#[test]
fn restoring_saved_content_clears_dirty_without_undo() {
    let dir = TestDir::new("saved-state");
    let mut editor = Editor::open(Options::default()).unwrap();
    let at = Point::new(0, 0);
    editor.execute(EditorCommand::InsertText {
        at,
        text: "a".into(),
    });
    editor.save_as(dir.join("diagram.txt")).unwrap();
    editor.execute(EditorCommand::InsertText {
        at,
        text: "b".into(),
    });
    assert!(editor.is_dirty());
    editor.execute(EditorCommand::InsertText {
        at,
        text: "a".into(),
    });
    assert!(!editor.is_dirty());
    assert!(editor.undo());
    assert_eq!(editor.render_text(""), "b\n");
    assert!(editor.is_dirty());
}

#[test]
fn successful_save_normalization_is_undoable_and_failed_save_keeps_state() {
    let dir = TestDir::new("save-normalization");
    let mut editor = Editor::open(Options::default()).unwrap();
    editor.execute(EditorCommand::InsertText {
        at: Point::new(0, 0),
        text: "a  ".into(),
    });
    let before = editor.render_text("");
    let cursor = editor.cursor();
    assert!(editor.save_as(dir.path()).is_err());
    assert_eq!(editor.render_text(""), before);
    assert_eq!(editor.cursor(), cursor);
    assert_eq!(editor.path(), None);
    assert!(editor.is_dirty());
    editor.save_as(dir.join("diagram.txt")).unwrap();
    assert_eq!(editor.render_text(""), "a\n");
    assert!(!editor.is_dirty());
    assert!(editor.undo());
    assert_eq!(editor.render_text(""), before);
    assert!(editor.is_dirty());
    assert!(editor.redo());
    assert_eq!(editor.render_text(""), "a\n");
    assert!(!editor.is_dirty());
}
