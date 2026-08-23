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
use structopt::clap::ErrorKind;
use structopt::StructOpt;

use super::dialogs::{
    editor_clip, editor_clip_prefix, editor_help, editor_new, editor_open, editor_quit,
    editor_redo, editor_save, editor_save_as, editor_tool, editor_trim_margins, editor_undo,
    modify_opts, new_scrollview,
};
use super::modeline::ModeLine;
use crate::config::{parse_color, Options};
use crate::constants::EDITOR_ID;
use crate::editor::{Editor, EditorView};
use crate::tools::{
    lines::{arrowtool::ArrowTool, boxtool::BoxTool, linetool::LineTool},
    selecttool::SelectTool,
    texttool::TextTool,
};
use crate::tui::controller::EditorCtx;

/// Parse process arguments and run the terminal application.
pub fn run() -> Result<(), Box<dyn Error>> {
    run_with_args(std::env::args_os())
}

/// Build and run the application with an explicit argument source.
pub fn run_with_args<I, T>(args: I) -> Result<(), Box<dyn Error>>
where
    I: IntoIterator<Item = T>,
    T: Into<std::ffi::OsString> + Clone,
{
    logger::init();
    log::set_max_level(log::LevelFilter::Info);

    let mut opts = match Options::from_iter_safe(args) {
        Ok(options) => options,
        Err(error)
            if matches!(
                error.kind,
                ErrorKind::HelpDisplayed | ErrorKind::VersionDisplayed
            ) =>
        {
            error.write_to(&mut std::io::stdout())?;
            return Ok(());
        }
        Err(error) => return Err(Box::new(error)),
    };
    opts.resolve_config();
    debug!("{:?}", opts);

    let editor = EditorView::new(Editor::open(opts.clone())?);
    let mut siv = build_ui(editor, &opts);
    siv.run();
    Ok(())
}

/// Construct the complete view tree without starting the event loop.
pub fn build_ui(editor: EditorView, opts: &Options) -> cursive::CursiveRunnable {
    let mut siv = cursive::crossterm();
    configure(&mut siv, editor, opts);
    siv
}

/// Populate an arbitrary Cursive root, including the dummy test backend.
pub fn configure(siv: &mut Cursive, editor: EditorView, opts: &Options) {
    let mut theme = siv.current_theme().clone();

    if let Some(bg) = opts
        .background
        .as_deref()
        .and_then(parse_color)
        .or(Some(Color::TerminalDefault))
    {
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
    siv.add_global_callback(keys.tool_select, editor_tool::<SelectTool, _>(|_| ()));
    siv.add_global_callback(keys.tool_box, editor_tool::<BoxTool, _>(|_| ()));
    siv.add_global_callback(keys.tool_line, editor_tool::<LineTool, _>(|_| ()));
    siv.add_global_callback(keys.tool_arrow, editor_tool::<ArrowTool, _>(|_| ()));
    siv.add_global_callback(keys.cycle_path, modify_opts(Options::cycle_path_mode));
    siv.add_global_callback(keys.tool_text, editor_tool::<TextTool, _>(|_| ()));
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
