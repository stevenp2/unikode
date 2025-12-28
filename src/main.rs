#![allow(clippy::many_single_char_names)]
mod editor;
mod modeline;
mod tools;
mod ui;
mod constants;
mod utils;
mod config;

use structopt::StructOpt;
use cursive::{
    event::{EventTrigger, Event, Key},
    logger,
    menu::Tree,
    view::Nameable,
    views::{LinearLayout, OnEventView, NamedView, ScrollView},
    theme::{PaletteColor, Color},
    Cursive,
};
use log::debug;
use std::error::Error;

use crate::constants::{
    EDITOR_ID,
    KEY_UNDO, KEY_SAVE, KEY_SAVE_AS, KEY_CLIP, KEY_CLIP_PREFIX,
    KEY_NEW, KEY_OPEN, KEY_QUIT, KEY_DEBUG, KEY_CYCLE_PATH, KEY_TRIM_MARGINS,
    KEY_HELP,
    KEY_TOOL_BOX, KEY_TOOL_LINE, KEY_TOOL_ARROW, KEY_TOOL_TEXT, 
    KEY_TOOL_SELECT,
};
use crate::config::{Options, parse_color};
use crate::modeline::ModeLine;
use crate::ui::{
    editor_new, editor_open, editor_save, editor_save_as, editor_clip,
    editor_clip_prefix, editor_quit, editor_undo, editor_redo,
    editor_trim_margins, editor_tool, modify_opts, editor_help,
    new_scrollview
};
use crate::editor::{
    Editor, EditorView,
    scroll::EditorCtx,
};
use crate::tools::{
    lines::{arrowtool::ArrowTool, boxtool::BoxTool, linetool::LineTool},
    texttool::TextTool,
    selecttool::SelectTool,
    PathMode::{Snap90, Routed}
};

fn main() -> Result<(), Box<dyn Error>> {
    logger::init();
    log::set_max_level(log::LevelFilter::Info);

    let mut opts = match Options::from_args_safe() {
        Ok(o) => o,
        Err(e) => {
            eprintln!("{}", e);
            std::process::exit(1);
        }
    };
    opts.resolve_config();
    debug!("{:?}", opts);

    let editor = EditorView::new(Editor::open(opts.clone())?);
    let mut siv = cursive::crossterm();
    let mut theme = siv.current_theme().clone();

    if let Some(bg) = opts.background.as_deref().and_then(parse_color).or(Some(Color::TerminalDefault)) {
        theme.palette[PaletteColor::Background] = bg;
        theme.palette[PaletteColor::View] = bg;
    }

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

    siv.menubar()
        .add_subtree(
            "File",
            Tree::new()
                .leaf(format!("({}) New", KEY_NEW), editor_new)
                .leaf(format!("({}) Open", KEY_OPEN), editor_open)
                .leaf(format!("({}) Save", KEY_SAVE), editor_save)
                .leaf(format!("({}) Save As", KEY_SAVE_AS), editor_save_as)
                .leaf(format!("({}) Clip", KEY_CLIP), editor_clip)
                .leaf(format!("({}) Clip Prefix", KEY_CLIP_PREFIX), editor_clip_prefix)
                .delimiter()
                .leaf(format!("({}) Debug", KEY_DEBUG), Cursive::toggle_debug_console)
                .leaf(format!("({}) Quit", KEY_QUIT), editor_quit),
        )
        .add_subtree(
            "Edit",
            Tree::new()
                .leaf(format!("({}) Undo", KEY_UNDO), editor_undo)
                .leaf("(Ctrl+r) Redo", editor_redo)
                .leaf(format!("({}) Trim Margins", KEY_TRIM_MARGINS), editor_trim_margins),
        )
        .add_leaf("Help", editor_help);

    siv.set_autohide_menu(false);

    // File
    siv.add_global_callback(KEY_NEW, editor_new);
    siv.add_global_callback(KEY_OPEN, editor_open);
    siv.add_global_callback(KEY_SAVE, editor_save);
    siv.add_global_callback(KEY_SAVE_AS, editor_save_as);
    siv.add_global_callback(KEY_CLIP, editor_clip);
    siv.add_global_callback(KEY_CLIP_PREFIX, editor_clip_prefix);
    siv.add_global_callback(KEY_DEBUG, Cursive::toggle_debug_console);
    siv.add_global_callback(KEY_QUIT, editor_quit);

    // Edit
    siv.add_global_callback(KEY_UNDO, editor_undo);
    siv.add_global_callback(Event::CtrlChar('r'), editor_redo);
    siv.add_global_callback(KEY_TRIM_MARGINS, editor_trim_margins);

    // Tools
    siv.add_global_callback(KEY_TOOL_SELECT, editor_tool::<SelectTool, _>(|_| ()));
    siv.add_global_callback(KEY_TOOL_BOX, editor_tool::<BoxTool, _>(|_| ()));
    siv.add_global_callback(KEY_TOOL_LINE, editor_tool::<LineTool, _>(|_| ()));
    siv.add_global_callback(KEY_TOOL_ARROW, editor_tool::<ArrowTool, _>(|_| ()));
    siv.add_global_callback(KEY_CYCLE_PATH, modify_opts(Options::cycle_path_mode));
    siv.add_global_callback(KEY_TOOL_TEXT, editor_tool::<TextTool, _>(|_| ()));

    // Help
    siv.add_global_callback(KEY_HELP, editor_help);

    let edit_view = OnEventView::new(new_scrollview(editor.clone()).with_name(EDITOR_ID))
        .on_pre_event_inner(EventTrigger::any(), |view: &mut NamedView<ScrollView<EditorView>>, event| {
            let mut scroll = view.get_mut();
            let mut ctx = EditorCtx::new(&mut scroll);
            ctx.on_event(event)
        });

    let layout = LinearLayout::vertical()
        .child(edit_view)
        .weight(100)
        .child(ModeLine::new(editor))
        .weight(1);

    siv.add_fullscreen_layer(layout);

    siv.run();

    Ok(())
}
