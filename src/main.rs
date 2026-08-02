#![allow(clippy::many_single_char_names)]
mod editor;
mod modeline;
mod tools;
mod ui;
mod constants;
mod defaults;
mod utils;
mod config;

use structopt::StructOpt;
use cursive::{
    event::{EventTrigger, Event},
    logger,
    menu::Tree,
    view::Nameable,
    views::{LinearLayout, OnEventView, NamedView, ScrollView},
    theme::{PaletteColor, Color},
    Cursive,
};
use log::debug;
use std::error::Error;

use crate::constants::EDITOR_ID;
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
                .leaf(format!("({}) Clip Prefix", keys.clip_prefix), editor_clip_prefix)
                .delimiter()
                .leaf(format!("({}) Debug", keys.debug), Cursive::toggle_debug_console)
                .leaf(format!("({}) Quit", keys.quit), editor_quit),
        )
        .add_subtree(
            "Edit",
            Tree::new()
                .leaf(format!("({}) Undo", keys.undo), editor_undo)
                .leaf("(Ctrl+r) Redo", editor_redo)
                .leaf(format!("({}) Trim Margins", keys.trim_margins), editor_trim_margins),
        )
        .add_leaf("Help", editor_help);

    siv.set_autohide_menu(false);

    // File
    siv.add_global_callback(keys.new, editor_new);
    siv.add_global_callback(keys.open, editor_open);
    siv.add_global_callback(keys.save, editor_save);
    siv.add_global_callback(keys.save_as, editor_save_as);
    siv.add_global_callback(keys.clip, editor_clip);
    siv.add_global_callback(keys.clip_prefix, editor_clip_prefix);
    siv.add_global_callback(keys.debug, Cursive::toggle_debug_console);
    siv.add_global_callback(keys.quit, editor_quit);

    // Edit
    siv.add_global_callback(keys.undo, editor_undo);
    siv.add_global_callback(Event::CtrlChar('r'), editor_redo);
    siv.add_global_callback(keys.trim_margins, editor_trim_margins);

    // Tools
    siv.add_global_callback(keys.tool_select, editor_tool::<SelectTool, _>(|_| ()));
    siv.add_global_callback(keys.tool_box, editor_tool::<BoxTool, _>(|_| ()));
    siv.add_global_callback(keys.tool_line, editor_tool::<LineTool, _>(|_| ()));
    siv.add_global_callback(keys.tool_arrow, editor_tool::<ArrowTool, _>(|_| ()));
    siv.add_global_callback(keys.cycle_path, modify_opts(Options::cycle_path_mode));
    siv.add_global_callback(keys.tool_text, editor_tool::<TextTool, _>(|_| ()));

    // Help
    siv.add_global_callback(keys.help, editor_help);

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
