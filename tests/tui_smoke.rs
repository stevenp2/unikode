use cursive::{
    event::{Event, Key, MouseButton, MouseEvent},
    theme::{Color, PaletteColor},
    view::View,
    views::ScrollView,
    Vec2,
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

fn ui(options: Options) -> (cursive::CursiveRunnable, EditorView) {
    let view = EditorView::new(Editor::open(options.clone()).unwrap());
    let mut siv = cursive::dummy();
    app::configure(&mut siv, view.clone(), &options);
    siv.screen_mut().layout(Vec2::new(80, 24));
    (siv, view)
}

fn mouse(siv: &mut cursive::Cursive, event: MouseEvent, x: usize, y: usize) {
    siv.on_event(Event::Mouse {
        offset: Vec2::zero(),
        position: Vec2::new(x, y),
        event,
    });
}

#[test]
fn mouse_release_keeps_text_input_active() {
    let (mut siv, view) = ui(Options::default());
    siv.on_event(Event::Char('t'));
    mouse(&mut siv, MouseEvent::Press(MouseButton::Left), 5, 1);
    mouse(&mut siv, MouseEvent::Release(MouseButton::Left), 5, 1);
    siv.on_event(Event::Char('x'));
    siv.on_event(Event::Key(Key::Esc));
    assert_eq!(view.read().render_text("").trim(), "x");
    assert!(view.write().undo());
    assert_eq!(view.read().render_text(""), "");
}

#[test]
fn selected_drawing_tool_can_draw_consecutive_mouse_strokes() {
    let (mut siv, view) = ui(Options::default());
    siv.on_event(Event::Char('b'));
    mouse(&mut siv, MouseEvent::Press(MouseButton::Left), 5, 1);
    mouse(&mut siv, MouseEvent::Release(MouseButton::Left), 8, 3);
    let first = view.read().render_text("");
    assert!(first.contains('┌'));
    mouse(&mut siv, MouseEvent::Press(MouseButton::Left), 10, 1);
    mouse(&mut siv, MouseEvent::Release(MouseButton::Left), 12, 2);
    assert_ne!(view.read().render_text(""), first);
    assert!(view.write().undo());
    assert_eq!(view.read().render_text(""), first);
}

#[test]
fn zero_width_gutter_supports_navigation() {
    let mut options = Options::default();
    options.gutter_width = 0;
    let (mut siv, view) = ui(options);
    siv.on_event(Event::Char('l'));
    assert_eq!(view.read().cursor(), Some(unikode::core::Point::new(1, 0)));
}

#[test]
fn remapped_global_commands_share_history_and_reset_interaction() {
    let mut options = Options::default();
    options.keys.undo = 'U';
    options.keys.new = 'N';
    let (mut siv, view) = ui(options);
    for event in [
        Event::Char('b'),
        Event::Char('3'),
        Event::Char('l'),
        Event::Char('2'),
        Event::Char('j'),
        Event::Key(Key::Enter),
    ] {
        siv.on_event(event);
    }
    let document = view.read().render_text("");
    assert!(document.contains('┌'));
    siv.on_event(Event::Char('U'));
    assert_eq!(view.read().render_text(""), "");
    siv.on_event(Event::CtrlChar('r'));
    assert_eq!(view.read().render_text(""), document);
    siv.on_event(Event::Char('U'));
    siv.on_event(Event::Char('s'));
    siv.on_event(Event::Char('N'));
    mouse(&mut siv, MouseEvent::Press(MouseButton::Left), 5, 1);
    mouse(&mut siv, MouseEvent::Release(MouseButton::Left), 8, 3);
    assert_eq!(view.read().render_text(""), "");
}
