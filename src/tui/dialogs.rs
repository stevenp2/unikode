use clipboard::{ClipboardContext, ClipboardProvider};
use cursive::{
    align::HAlign,
    view::{scroll::Scroller, Nameable, View},
    views::{Dialog, EditView, ScrollView, TextView},
    Cursive,
};
use std::path::PathBuf;
use std::sync::Arc;

use super::session::{Session, ToolChoice};
use crate::application::{self, Action};
use crate::constants::{EDITOR_ID, INPUT_ID, NO_MARGIN, POPUP_ID};
use crate::editor::{command::EditorCommand, Editor};
use crate::tui::EditorView;

pub(crate) fn with_checked_editor<T, F>(siv: &mut Cursive, title: T, f: F)
where
    T: Into<String>,
    F: Fn(&mut Cursive) + 'static + Send + Sync,
{
    with_session_mut(siv, |session| session.finish(false));
    if with_editor(siv, Editor::is_dirty) {
        display_yesno(siv, title, "Discard unsaved changes?", f);
    } else {
        f(siv);
    }
}

pub(crate) fn with_editor_mut<T, F>(siv: &mut Cursive, f: F) -> T
where
    F: FnOnce(&mut Editor) -> T,
{
    siv.find_name::<ScrollView<EditorView>>(EDITOR_ID)
        .map(|mut view| f(&mut view.get_inner_mut().write()))
        .expect("Editor view not found")
}

pub(crate) fn with_editor<T, F>(siv: &mut Cursive, f: F) -> T
where
    F: FnOnce(&Editor) -> T,
{
    siv.find_name::<ScrollView<EditorView>>(EDITOR_ID)
        .map(|view| f(&view.get_inner().read()))
        .expect("Editor view not found")
}

fn with_session_mut<T>(siv: &mut Cursive, apply: impl FnOnce(&mut Session) -> T) -> T {
    siv.find_name::<ScrollView<EditorView>>(EDITOR_ID)
        .map(|view| apply(&mut view.get_inner().session_write()))
        .expect("Editor view not found")
}

pub(crate) fn display_yesno<T, C, F>(siv: &mut Cursive, title: T, content: C, yes: F)
where
    T: Into<String>,
    C: Into<String>,
    F: Fn(&mut Cursive) + 'static + Send + Sync,
{
    if siv.find_name::<Dialog>(POPUP_ID).is_some() {
        return;
    }

    let popup = Dialog::text(content)
        .title(title)
        .padding(NO_MARGIN)
        .h_align(HAlign::Center)
        .dismiss_button("No")
        .button("Yes", move |siv| {
            siv.pop_layer();
            yes(siv);
        })
        .with_name(POPUP_ID);

    siv.add_layer(popup);
}

pub(crate) fn display_form<T, F>(siv: &mut Cursive, title: T, form: F)
where
    T: Into<String>,
    F: Fn(&mut Cursive, &'static str, &str) + 'static + Send + Sync,
{
    if siv.find_name::<Dialog>(POPUP_ID).is_some() {
        return;
    }

    let submit = Arc::new(move |siv: &mut Cursive, input: &str| {
        form(siv, POPUP_ID, input);
    });

    let submit_ok = Arc::clone(&submit);

    let input = EditView::new()
        .on_submit(move |siv, input| submit(siv, input))
        .with_name(INPUT_ID);

    let popup = Dialog::around(input)
        .title(title)
        .button("Ok", move |siv| {
            let input = siv
                .call_on_name(INPUT_ID, |view: &mut EditView| view.get_content())
                .expect("Input view not found");
            submit_ok(siv, &input);
        })
        .dismiss_button("Cancel")
        .with_name(POPUP_ID);

    siv.add_layer(popup);
}

pub(crate) fn notify<T, C>(siv: &mut Cursive, title: T, content: C)
where
    T: Into<String>,
    C: Into<String>,
{
    let content = ScrollView::new(TextView::new(content))
        .scroll_x(false)
        .scroll_y(true);

    siv.add_layer(
        Dialog::around(content)
            .title(title)
            .dismiss_button("Ok")
            .h_align(HAlign::Center)
            .padding(NO_MARGIN),
    );
}

pub(crate) fn notify_unique<T, C>(siv: &mut Cursive, unique_id: &'static str, title: T, content: C)
where
    T: Into<String>,
    C: Into<String>,
{
    if siv.find_name::<Dialog>(unique_id).is_some() {
        return;
    }

    let content = ScrollView::new(TextView::new(content))
        .scroll_x(false)
        .scroll_y(true);

    siv.add_layer(
        Dialog::around(content)
            .title(title)
            .dismiss_button("Ok")
            .h_align(HAlign::Center)
            .padding(NO_MARGIN)
            .with_name(unique_id),
    );
}

pub(crate) fn new_scrollview<V: View>(inner: V) -> ScrollView<V> {
    let mut scroll = ScrollView::new(inner).scroll_x(true).scroll_y(true);
    scroll.get_scroller_mut().set_scrollbar_padding((0, 0));
    scroll
}

pub(crate) fn editor_new(siv: &mut Cursive) {
    with_checked_editor(siv, "New", |siv| {
        with_session_mut(siv, |session| {
            session.editor.clear();
            session.reset();
        })
    });
}

pub(crate) fn editor_open(siv: &mut Cursive) {
    with_checked_editor(siv, "Open", |siv| {
        display_form(siv, "Open", |siv, id, raw_path| {
            let mut view = siv.find_name::<Dialog>(id).unwrap();

            if raw_path.is_empty() {
                view.set_title("Open: path is empty!");
                return;
            }

            let path: PathBuf = raw_path.into();
            if !path.exists() {
                view.set_title(format!("Open: {path:?} does not exist!"));
                return;
            }
            if !path.is_file() {
                view.set_title(format!("Open: {path:?} is not a file!"));
                return;
            }
            siv.pop_layer();

            if let Err(e) = with_session_mut(siv, |session| {
                session.editor.open_file(path)?;
                session.reset();
                Ok::<_, std::io::Error>(())
            }) {
                notify(siv, "open failed", format!("{e:?}"));
            }
        })
    });
}

pub(crate) fn editor_save(siv: &mut Cursive) {
    with_session_mut(siv, |session| session.finish(false));
    match with_editor_mut(siv, Editor::save).map_err(|e| format!("{e:?}")) {
        Ok(false) => editor_save_as(siv),
        Ok(true) => notify(siv, "saved", ""),
        Err(e) => notify(siv, "save failed", e),
    }
}

pub(crate) fn editor_save_as(siv: &mut Cursive) {
    with_session_mut(siv, |session| session.finish(false));
    display_form(siv, "Save As", |siv, id, raw_path| {
        let mut view = siv.find_name::<Dialog>(id).unwrap();

        if raw_path.is_empty() {
            view.set_title("Save As: path is empty!");
            return;
        }

        let path: PathBuf = raw_path.into();
        if path.is_dir() {
            view.set_title(format!("Save As: {path:?} is a directory!"));
            return;
        }
        siv.pop_layer();

        match with_editor_mut(siv, |e| e.save_as(path)).map_err(|e| format!("{e:?}")) {
            Ok(()) => notify(siv, "saved", ""),
            Err(e) => notify(siv, "save as failed", e),
        }
    });
}

pub(crate) fn editor_clip(siv: &mut Cursive) {
    with_session_mut(siv, |session| session.finish(false));
    match copy_to_clipboard(with_editor(siv, |editor| editor.export_text(""))) {
        Ok(()) => notify(siv, "clipped", ""),
        Err(e) => notify(siv, "clip failed", e),
    }
}

pub(crate) fn editor_clip_prefix(siv: &mut Cursive) {
    with_session_mut(siv, |session| session.finish(false));
    display_form(siv, "Clip Prefix", |siv, _, prefix| {
        siv.pop_layer();

        match copy_to_clipboard(with_editor(siv, |editor| editor.export_text(prefix))) {
            Ok(()) => notify(siv, "clipped", ""),
            Err(e) => notify(siv, "clip failed", e),
        }
    });
}

fn copy_to_clipboard(content: String) -> Result<(), String> {
    let mut clipboard = ClipboardContext::new().map_err(|error| format!("{error:?}"))?;
    clipboard
        .set_contents(content)
        .map_err(|error| format!("{error:?}"))
}

pub(crate) fn editor_quit(siv: &mut Cursive) {
    with_checked_editor(siv, "Quit", Cursive::quit);
}

pub(crate) fn editor_undo(siv: &mut Cursive) {
    with_editor_mut(siv, |editor| {
        application::update(editor, Action::Editor(EditorCommand::Undo))
    });
}

pub(crate) fn editor_redo(siv: &mut Cursive) {
    with_editor_mut(siv, |editor| {
        application::update(editor, Action::Editor(EditorCommand::Redo))
    });
}

pub(crate) fn editor_trim_margins(siv: &mut Cursive) {
    with_editor_mut(siv, |editor| {
        application::update(editor, Action::Editor(EditorCommand::TrimMargins))
    });
    notify(siv, "trimmed", "");
}

pub(crate) fn editor_tool(choice: ToolChoice) -> impl Fn(&mut Cursive) {
    move |siv| with_session_mut(siv, |session| session.activate(choice))
}

pub(crate) fn editor_cycle_path(siv: &mut Cursive) {
    with_session_mut(siv, Session::cycle_path);
}

const HELP: &str = "DEFAULT KEYBINDS (custom bindings appear in the menu/config):
    Esc Focus the menu bar.
    n   New: Open a new (blank) file.
    o   Open: Open the specified file.
    w   Save: Save buffer to the current path.
    S   Save As: Save buffer to the specified path.
    c   Clip: Export buffer to the clipboard.
    C   Clip Prefix: Export buffer to the clipboard with a prefix before each line.
    `   Debug: Open the debug console.
    q   Quit: Quit without saving.
    u   Undo: Undo the last buffer modification.
    Ctrl+r Redo: Redo the last undo.
    T   Trim Margins: Trim excess whitespace from all margins.
    s   Switch to Select mode.
    b   Switch to the Box tool (enters Box Mode).
    L   Switch to the Line tool.
    a   Switch to the Arrow tool (enters Arrow Mode).
    p   Cycle the type of path that Line and Arrow tools will draw.
    t   Switch to the Text tool (enters Text Mode).
    ?   Help: Display this help message.

MODES:
    Select Mode Actions:
        e   Erase selected content.
        m   Enter Move mode to move selected content.
        s   Finish selection and return to Normal mode.
        Esc Exit Select mode.

    Move Mode Actions:
        m   Commit move and return to Normal mode.
        Enter Commit move and restart selection at the new position.
        Esc Finish move and return to Normal mode.

    Box/Line/Arrow Modes:
        Enter/Esc Commit changes and return to Normal mode.

    Text Mode:
        Enter Insert a newline.
        Esc Commit text and return to Normal mode.

NAVIGATION:
    hjkl to move the cursor outside Text mode.
    Arrow keys move within a text draft in Text mode.
    Scroll with arrow keys or page-up and page-down.
    Pan around by dragging with the right mouse button.
";

pub(crate) fn editor_help(siv: &mut Cursive) {
    let version_str = format!("unikode {}", env!("CARGO_PKG_VERSION"));

    let authors = env!("CARGO_PKG_AUTHORS")
        .split(':')
        .map(|s| format!("* {s}"))
        .collect::<Vec<_>>()
        .join("\n");

    let author_str = format!("Made with love by:\n{authors}");

    let help_str = format!("{version_str}\n\n{author_str}\n\n{HELP}");

    notify_unique(siv, "editor_help", "Help", help_str);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{config::Options, tui::app};
    use cursive::{
        event::{Event, Key},
        Vec2,
    };

    fn text_draft(options: Options) -> (cursive::CursiveRunnable, EditorView) {
        let view = EditorView::new(Editor::open(options.clone()).unwrap());
        let mut siv = cursive::dummy();
        app::configure(&mut siv, view.clone(), &options);
        siv.screen_mut().layout(Vec2::new(80, 24));
        siv.on_event(Event::Char('t'));
        siv.on_event(Event::Char('x'));
        (siv, view)
    }

    #[test]
    fn new_prompts_before_discarding_a_visible_draft() {
        let (mut siv, view) = text_draft(Options::default());
        editor_new(&mut siv);
        assert!(siv.find_name::<Dialog>(POPUP_ID).is_some());
        assert_eq!(view.read().render_text(""), "x\n");
        siv.screen_mut().layout(Vec2::new(80, 24));
        siv.on_event(Event::Key(Key::Enter));
        assert!(siv.find_name::<Dialog>(POPUP_ID).is_none());
        assert_eq!(view.read().render_text(""), "x\n");
    }

    #[test]
    fn save_includes_the_visible_draft_and_keeps_it_undoable() {
        let path = std::env::temp_dir().join(format!("unikode-menu-save-{}", std::process::id()));
        std::fs::write(&path, "").unwrap();
        let mut options = Options::default();
        options.file = Some(path.clone());
        let (mut siv, view) = text_draft(options);
        editor_save(&mut siv);
        let saved = std::fs::read_to_string(&path).unwrap();
        std::fs::remove_file(path).unwrap();
        assert_eq!(saved, "x\n");
        assert!(!view.read().is_dirty());
        assert!(view.write().undo());
        assert_eq!(view.read().render_text(""), "");
        assert!(view.read().is_dirty());
    }
}
