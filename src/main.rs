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
    event::{EventTrigger, Key},
    logger,
    menu::Tree,
    view::Nameable,
    views::{LinearLayout, OnEventView, NamedView, ScrollView},
    Cursive,
};
use log::debug;
use std::error::Error;

use crate::constants::{
    EDITOR_ID, S90, RTD
};
use crate::config::Options;
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
    erasetool::EraseTool,
    movetool::MoveTool,
    texttool::TextTool,
    PathMode::{Snap90, Routed}
};

fn main() -> Result<(), Box<dyn Error>> {
    logger::init();
    log::set_max_level(log::LevelFilter::Info);

    let opts = match Options::from_args_safe() {
        Ok(o) => o,
        Err(e) => {
            eprintln!("{}", e);
            std::process::exit(1);
        }
    };
    debug!("{:?}", opts);

    let editor = EditorView::new(Editor::open(opts)?);
    let mut siv = cursive::crossterm();

    siv.menubar()
        .add_subtree(
            "File",
            Tree::new()
                .leaf("(n) New", editor_new)
                .leaf("(o) Open", editor_open)
                .leaf("(s) Save", editor_save)
                .leaf("(S) Save As", editor_save_as)
                .leaf("(c) Clip", editor_clip)
                .leaf("(C) Clip Prefix", editor_clip_prefix)
                .delimiter()
                .leaf("(`) Debug", Cursive::toggle_debug_console)
                .leaf("(q) Quit", editor_quit),
        )
        .add_subtree(
            "Edit",
            Tree::new()
                .leaf("(u) Undo", editor_undo)
                .leaf("(r) Redo", editor_redo)
                .leaf("(T) Trim Margins", editor_trim_margins),
        )
        .add_leaf("Help", editor_help)
        .add_delimiter()
        .add_leaf("Box", editor_tool::<BoxTool, _>(|_| ()))
        .add_subtree(
            "Line",
            Tree::new()
                .leaf(S90, editor_tool::<LineTool, _>(|o| o.path_mode = Snap90))
                .leaf(RTD, editor_tool::<LineTool, _>(|o| o.path_mode = Routed)),
        )
        .add_subtree(
            "Arrow",
            Tree::new()
                .leaf(S90, editor_tool::<ArrowTool, _>(|o| o.path_mode = Snap90))
                .leaf(RTD, editor_tool::<ArrowTool, _>(|o| o.path_mode = Routed)),
        )
        .add_leaf("Text", editor_tool::<TextTool, _>(|_| ()))
        .add_leaf("Erase", editor_tool::<EraseTool, _>(|_| ()))
        .add_leaf("Move", editor_tool::<MoveTool, _>(|_| ()));

    // * * * d * f g * i j k * * * * * * * * * * v w x y z
    // A B * D E F G H I J K L M N O P Q R * * U V W X Y Z

    siv.set_autohide_menu(false);
    siv.add_global_callback(Key::Esc, |s| s.select_menubar());

    // File
    siv.add_global_callback('n', editor_new);
    siv.add_global_callback('o', editor_open);
    siv.add_global_callback('s', editor_save);
    siv.add_global_callback('S', editor_save_as);
    siv.add_global_callback('c', editor_clip);
    siv.add_global_callback('C', editor_clip_prefix);
    siv.add_global_callback('`', Cursive::toggle_debug_console);
    siv.add_global_callback('q', editor_quit);

    // Edit
    siv.add_global_callback('u', editor_undo);
    siv.add_global_callback('r', editor_redo);
    siv.add_global_callback('T', editor_trim_margins);

    // Tools
    siv.add_global_callback('b', editor_tool::<BoxTool, _>(|_| ()));
    siv.add_global_callback('l', editor_tool::<LineTool, _>(|_| ()));
    siv.add_global_callback('a', editor_tool::<ArrowTool, _>(|_| ()));
    siv.add_global_callback('p', modify_opts(Options::cycle_path_mode));
    siv.add_global_callback('t', editor_tool::<TextTool, _>(|_| ()));
    siv.add_global_callback('e', editor_tool::<EraseTool, _>(|_| ()));
    siv.add_global_callback('m', editor_tool::<MoveTool, _>(|_| ()));

    // Help
    siv.add_global_callback('h', editor_help);

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