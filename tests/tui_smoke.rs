use cursive::views::ScrollView;
use unikode::{app, config::Options, editor::Editor, tui::EditorView, EDITOR_VIEW_ID};

#[test]
fn complete_ui_builds_with_the_dummy_backend() {
    let options = Options::default();
    let view = EditorView::new(Editor::open(options.clone()).unwrap());
    let mut siv = cursive::dummy();
    app::configure(&mut siv, view, &options);

    assert!(siv
        .find_name::<ScrollView<EditorView>>(EDITOR_VIEW_ID)
        .is_some());
}
