use cursive::{
    align::HAlign,
    view::Nameable,
    views::{Dialog, EditView, ScrollView, TextView},
    Cursive,
};
use std::sync::Arc;

use crate::constants::{EDITOR_ID, NO_MARGIN, POPUP_ID, INPUT_ID};
use crate::editor::{Editor, EditorView};

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
        .unwrap()
}

/// Run `f` with an immutable reference to the editor, returning its result. Shorthand for
/// looking up the view any time it's needed.
pub(crate) fn with_editor<T, F>(siv: &mut Cursive, f: F) -> T
where
    F: FnOnce(&Editor) -> T,
{
    siv.find_name::<ScrollView<EditorView>>(EDITOR_ID)
        .map(|view| f(&view.get_inner().read()))
        .unwrap()
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
                .unwrap();
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
