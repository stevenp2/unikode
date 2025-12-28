use cursive::{
    align::HAlign,
    view::{scroll::Scroller, Nameable, View},
    views::{Dialog, EditView, ScrollView, TextView},
    Cursive,
};
use std::sync::Arc;
use std::path::PathBuf;

use crate::constants::{EDITOR_ID, NO_MARGIN, POPUP_ID, INPUT_ID};
use crate::editor::{Editor, EditorView};
use crate::config::Options;
use crate::tools::Tool;

/// Run `f` if the editor's buffer has not been modified since the last save, or if user
/// has confirmed that they're ok with discarding unsaved changes.
pub(crate) fn with_checked_editor<T, F>(siv: &mut Cursive, title: T, f: F)
where
    T: Into<String>,
    F: Fn(&mut Cursive) + 'static + Send + Sync,
{
    if with_editor(siv, Editor::is_dirty) {
        display_yesno(siv, title, "Discard unsaved changes?", f);
    } else {
        f(siv);
    }
}

/// Run `f` with a mutable reference to the editor, returning its result. Shorthand for
/// looking up the view any time it's needed.
pub(crate) fn with_editor_mut<T, F>(siv: &mut Cursive, f: F) -> T
where
    F: FnOnce(&mut Editor) -> T,
{
    siv.find_name::<ScrollView<EditorView>>(EDITOR_ID)
        .map(|mut view| f(&mut view.get_inner_mut().write()))
        .expect("Editor view not found")
}

/// Run `f` with an immutable reference to the editor, returning its result. Shorthand for
/// looking up the view any time it's needed.
pub(crate) fn with_editor<T, F>(siv: &mut Cursive, f: F) -> T
where
    F: FnOnce(&Editor) -> T,
{
    siv.find_name::<ScrollView<EditorView>>(EDITOR_ID)
        .map(|view| f(&view.get_inner().read()))
        .expect("Editor view not found")
}

/// Display a "Yes / No" prompt with the provided `title`, running `yes` iff "Yes" is
/// pressed. Defaults to "No".
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

/// Display a single line input form, passing the submitted content into the provided
/// callback `form`.
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

/// Display a notification dialog.
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

/// Display a unique notification dialog. No two dialogs with the same `unique_id` will
/// ever be shown at the same time.
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
    with_checked_editor(siv, "New", |siv| with_editor_mut(siv, Editor::clear));
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
                view.set_title(format!("Open: {:?} does not exist!", path));
                return;
            }
            if !path.is_file() {
                view.set_title(format!("Open: {:?} is not a file!", path));
                return;
            }
            siv.pop_layer();

            if let Err(e) = with_editor_mut(siv, |e| e.open_file(path)) {
                notify(siv, "open failed", format!("{:?}", e));
            }
        })
    });
}

pub(crate) fn editor_save(siv: &mut Cursive) {
    match with_editor_mut(siv, Editor::save).map_err(|e| format!("{:?}", e)) {
        Ok(false) => editor_save_as(siv),
        Ok(true) => notify(siv, "saved", ""),
        Err(e) => notify(siv, "save failed", e),
    }
}

pub(crate) fn editor_save_as(siv: &mut Cursive) {
    display_form(siv, "Save As", |siv, id, raw_path| {
        let mut view = siv.find_name::<Dialog>(id).unwrap();

        if raw_path.is_empty() {
            view.set_title("Save As: path is empty!");
            return;
        }

        let path: PathBuf = raw_path.into();
        if path.is_dir() {
            view.set_title(format!("Save As: {:?} is a directory!", path));
            return;
        }
        siv.pop_layer();

        match with_editor_mut(siv, |e| e.save_as(path)).map_err(|e| format!("{:?}", e)) {
            Ok(()) => notify(siv, "saved", ""),
            Err(e) => notify(siv, "save as failed", e),
        }
    });
}

pub(crate) fn editor_clip(siv: &mut Cursive) {
    match with_editor(siv, |e| e.render_to_clipboard("")).map_err(|e| format!("{:?}", e)) {
        Ok(()) => notify(siv, "clipped", ""),
        Err(e) => notify(siv, "clip failed", e),
    }
}

pub(crate) fn editor_clip_prefix(siv: &mut Cursive) {
    display_form(siv, "Clip Prefix", |siv, _, prefix| {
        siv.pop_layer();

        match with_editor(siv, |e| e.render_to_clipboard(prefix)).map_err(|e| format!("{:?}", e)) {
            Ok(()) => notify(siv, "clipped", ""),
            Err(e) => notify(siv, "clip failed", e),
        }
    });
}

pub(crate) fn editor_quit(siv: &mut Cursive) {
    with_checked_editor(siv, "Quit", Cursive::quit);
}

pub(crate) fn editor_undo(siv: &mut Cursive) {
    with_editor_mut(siv, Editor::undo);
}

pub(crate) fn editor_redo(siv: &mut Cursive) {
    with_editor_mut(siv, Editor::redo);
}

pub(crate) fn editor_trim_margins(siv: &mut Cursive) {
    with_editor_mut(siv, Editor::trim_margins);
    notify(siv, "trimmed", "");
}

pub(crate) fn editor_tool<T: 'static + Tool + Default + Send + Sync, S>(apply: S) -> impl Fn(&mut Cursive)
where
    S: Fn(&mut Options),
{
    move |siv| {
        with_editor_mut(siv, |editor| {
            editor.mut_opts(|o| apply(o));
            editor.set_tool(T::default());
        });
    }
}

pub(crate) fn modify_opts<S>(apply: S) -> impl Fn(&mut Cursive)
where
    S: Fn(&mut Options),
{
    move |siv| with_editor_mut(siv, |editor| editor.mut_opts(|o| apply(o)))
}

const HELP: &str = "KEYBINDS:
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
        Enter Commit move and return to Normal mode.
        Esc Finish move and return to Normal mode.

    Box/Arrow/Text Modes:
        Enter/Esc Commit changes and return to Normal mode.

NAVIGATION:
    hjkl or arrow keys to move the cursor.
    Scroll with arrow keys or page-up and page-down.
    Pan around by dragging with the right mouse button.
";

pub(crate) fn editor_help(siv: &mut Cursive) {
    let version_str = format!("askii {}", env!("CARGO_PKG_VERSION"));

    let authors = env!("CARGO_PKG_AUTHORS")
        .split(':')
        .map(|s| format!("* {}", s))
        .collect::<Vec<_>>()
        .join("\n");

    let author_str = format!("Made with love by:\n{}", authors);

    let help_str = format!("{}\n\n{}\n\n{}", version_str, author_str, HELP);

    notify_unique(siv, "editor_help", "Help", help_str);
}
