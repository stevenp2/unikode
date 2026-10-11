use clap::{error::ErrorKind, Parser};
use cursive::{
    event::{Event, EventTrigger},
    logger,
    menu::Tree,
    theme::{Color, PaletteColor},
    view::Nameable,
    views::{LinearLayout, NamedView, OnEventView, ScrollView},
    Cursive,
};
use log::debug;
use std::error::Error;

use super::dialogs::{
    editor_clip, editor_clip_prefix, editor_cycle_path, editor_help, editor_new, editor_open,
    editor_quit, editor_redo, editor_save, editor_save_as, editor_tool, editor_trim_margins,
    editor_undo, new_scrollview,
};
use super::modeline::ModeLine;
use super::session::ToolChoice;
use crate::config::Options;
use crate::constants::EDITOR_ID;
use crate::drawing::ShapeKind;
use crate::editor::Editor;
use crate::tui::controller::EditorCtx;
use crate::tui::theme::parse_color;
use crate::tui::EditorView;

pub fn run() -> Result<(), Box<dyn Error>> {
    run_with_args(std::env::args_os())
}

pub fn run_with_args<I, T>(args: I) -> Result<(), Box<dyn Error>>
where
    I: IntoIterator<Item = T>,
    T: Into<std::ffi::OsString> + Clone,
{
    logger::init();
    log::set_max_level(log::LevelFilter::Info);

    let mut opts = match Options::try_parse_from(args) {
        Ok(options) => options,
        Err(error)
            if matches!(
                error.kind(),
                ErrorKind::DisplayHelp | ErrorKind::DisplayVersion
            ) =>
        {
            error.print()?;
            return Ok(());
        }
        Err(error) => return Err(Box::new(error)),
    };
    opts.resolve_config();
    debug!("{:?}", opts);

    let editor = EditorView::new(Editor::open(opts.clone())?);
    let mut siv = build_ui(editor, &opts);
    siv.try_run()
}

/// build the view tree without starting the event loop
pub fn build_ui(editor: EditorView, opts: &Options) -> cursive::CursiveRunnable {
    let mut siv = cursive::crossterm();
    configure(&mut siv, editor, opts);
    siv
}

/// add the editor UI and callbacks to a supplied root
pub fn configure(siv: &mut Cursive, editor: EditorView, opts: &Options) {
    let mut theme = siv.current_theme().clone();

    let bg = opts
        .background
        .as_deref()
        .and_then(parse_color)
        .unwrap_or(Color::TerminalDefault);
    theme.palette[PaletteColor::Background] = bg;
    theme.palette[PaletteColor::View] = bg;
    if let Some(c) = opts.color_normal.as_deref().and_then(parse_color) {
        theme.palette[PaletteColor::Primary] = c;
    }
    if let Some(c) = opts.color_ui_active.as_deref().and_then(parse_color) {
        theme.palette[PaletteColor::TitlePrimary] = c;
    }
    if let Some(c) = opts.color_ui.as_deref().and_then(parse_color) {
        theme.palette[PaletteColor::TitleSecondary] = c;
    }
    if let Some(c) = opts.color_cursor_fg.as_deref().and_then(parse_color) {
        theme.palette[PaletteColor::HighlightText] = c;
    }
    if let Some(c) = opts.color_cursor_bg.as_deref().and_then(parse_color) {
        theme.palette[PaletteColor::Highlight] = c;
    }
    if let Some(c) = opts.color_selection_bg.as_deref().and_then(parse_color) {
        theme.palette[PaletteColor::HighlightInactive] = c;
    }
    siv.set_theme(theme);

    let keys = &opts.keys;
    siv.menubar()
        .add_subtree(
            "File",
            Tree::new()
                .leaf(format!("({}) New", keys.new), editor_new)
                .leaf(format!("({}) Open", keys.open), editor_open)
                .leaf(format!("({}) Save", keys.save), editor_save)
                .leaf(format!("({}) Save As", keys.save_as), editor_save_as)
                .leaf(format!("({}) Clip", keys.clip), editor_clip)
                .leaf(
                    format!("({}) Clip Prefix", keys.clip_prefix),
                    editor_clip_prefix,
                )
                .delimiter()
                .leaf(
                    format!("({}) Debug", keys.debug),
                    Cursive::toggle_debug_console,
                )
                .leaf(format!("({}) Quit", keys.quit), editor_quit),
        )
        .add_subtree(
            "Edit",
            Tree::new()
                .leaf(format!("({}) Undo", keys.undo), editor_undo)
                .leaf("(Ctrl+r) Redo", editor_redo)
                .leaf(
                    format!("({}) Trim Margins", keys.trim_margins),
                    editor_trim_margins,
                ),
        )
        .add_leaf("Help", editor_help);
    siv.set_autohide_menu(false);

    siv.add_global_callback(keys.new, editor_new);
    siv.add_global_callback(keys.open, editor_open);
    siv.add_global_callback(keys.save, editor_save);
    siv.add_global_callback(keys.save_as, editor_save_as);
    siv.add_global_callback(keys.clip, editor_clip);
    siv.add_global_callback(keys.clip_prefix, editor_clip_prefix);
    siv.add_global_callback(keys.debug, Cursive::toggle_debug_console);
    siv.add_global_callback(keys.quit, editor_quit);
    siv.add_global_callback(keys.undo, editor_undo);
    siv.add_global_callback(Event::CtrlChar('r'), editor_redo);
    siv.add_global_callback(keys.trim_margins, editor_trim_margins);
    siv.add_global_callback(keys.tool_select, editor_tool(ToolChoice::Select));
    siv.add_global_callback(keys.tool_box, editor_tool(ToolChoice::Draw(ShapeKind::Box)));
    siv.add_global_callback(
        keys.tool_line,
        editor_tool(ToolChoice::Draw(ShapeKind::Line)),
    );
    siv.add_global_callback(
        keys.tool_arrow,
        editor_tool(ToolChoice::Draw(ShapeKind::Arrow)),
    );
    siv.add_global_callback(keys.cycle_path, editor_cycle_path);
    siv.add_global_callback(keys.tool_text, editor_tool(ToolChoice::Text));
    siv.add_global_callback(keys.help, editor_help);

    let edit_view = OnEventView::new(new_scrollview(editor.clone()).with_name(EDITOR_ID))
        .on_pre_event_inner(
            EventTrigger::any(),
            |view: &mut NamedView<ScrollView<EditorView>>, event| {
                let mut scroll = view.get_mut();
                let mut ctx = EditorCtx::new(&mut scroll);
                ctx.on_event(event)
            },
        );
    let layout = LinearLayout::vertical()
        .child(edit_view)
        .weight(100)
        .child(ModeLine::new(editor))
        .weight(1);
    siv.add_fullscreen_layer(layout);
}
