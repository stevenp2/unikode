use cursive::{
    theme::{Color, PaletteColor},
    views::ScrollView,
};
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

#[test]
fn background_uses_a_valid_color_or_the_terminal_default() {
    for (background, expected) in [
        (None, Color::TerminalDefault),
        (Some("invalid"), Color::TerminalDefault),
        (Some("#123456"), Color::Rgb(0x12, 0x34, 0x56)),
    ] {
        let mut options = Options::default();
        options.background = background.map(str::to_owned);
        let view = EditorView::new(Editor::open(options.clone()).unwrap());
        let mut siv = cursive::dummy();
        app::configure(&mut siv, view, &options);

        assert_eq!(
            siv.current_theme().palette[PaletteColor::Background],
            expected
        );
        assert_eq!(siv.current_theme().palette[PaletteColor::View], expected);
    }
}
