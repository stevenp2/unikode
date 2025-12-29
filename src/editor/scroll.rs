use cursive::{
    event::{
        Event, EventResult, Key,
        MouseButton::{Left, Right},
        MouseEvent::{Release, Hold, Press}
    },
    view::scroll::Scroller,
    views::ScrollView,
    Vec2,
};

use lazy_static::lazy_static;
use parking_lot::Mutex;
use std::cmp::{max, min};

use crate::constants::{
    CONSUMED,
    KEY_UNDO,
    KEY_SAVE,
    KEY_SAVE_AS,
    KEY_CLIP,
    KEY_CLIP_PREFIX,
    KEY_NEW,
    KEY_OPEN,
    KEY_QUIT,
    KEY_DEBUG,
    KEY_CYCLE_PATH,
    KEY_TRIM_MARGINS,
    KEY_HELP,
    KEY_MOVE_LEFT,
    KEY_MOVE_DOWN,
    KEY_MOVE_UP,
    KEY_MOVE_RIGHT,
    KEY_MOVE_LINE_START,
    KEY_MOVE_FIRST_NON_WS,
    KEY_MOVE_LAST_NON_WS,
    GUTTER_WIDTH,
    KEY_TOOL_BOX,
    KEY_TOOL_ARROW,
    KEY_TOOL_TEXT,
    KEY_TOOL_ERASE,
    KEY_TOOL_MOVE,
    KEY_TOOL_SELECT,
    KEY_TOOL_LINE,
};

use crate::tools::lines::boxtool::{draw_box_on_buffer, BoxTool};
use crate::tools::lines::arrowtool::{draw_arrow_on_buffer, ArrowTool};
use crate::tools::lines::linetool::{draw_line_on_buffer, LineTool};
use crate::tools::erasetool::erase_on_buffer;
use crate::tools::texttool::TextTool;
use crate::tools::selecttool::SelectTool;
use crate::tools::movetool::{MoveTool, move_on_buffer};
use crate::tools::Tool;
use super::{EditorView, Buffer, EditorMode};
use cursive::Rect;

macro_rules! intercept_scrollbar {
    ($ctx:expr, $event:expr) => {{ 
        lazy_static! {
            static ref LAST_LPRESS: Mutex<Option<Vec2>> = Mutex::new(None);
        }

        if let Event::Mouse {
            offset,
            position: pos,
            event,
        } = $event
        {
            match event {
                Press(Left) if $ctx.on_scrollbar(*offset, *pos) => {
                    *LAST_LPRESS.lock() = Some(*pos);
                    return None;
                }

                Press(Left) => {
                    *LAST_LPRESS.lock() = Some(*pos);
                }

                Hold(Left)
                    if LAST_LPRESS
                        .lock()
                        .map(|pos| $ctx.on_scrollbar(*offset, pos))
                        .unwrap_or(false) =>
                {
                    return None;
                }

                Release(Left)
                    if LAST_LPRESS
                        .lock()
                        .take()
                        .map(|pos| $ctx.on_scrollbar(*offset, pos))
                        .unwrap_or(false) =>
                {
                    return None;
                }

                _ => {}
            }
        }
    }};
}

macro_rules! intercept_pan {
    ($ctx:expr, $event:expr) => {{ 
        lazy_static! {
            static ref OLD: Mutex<Option<Vec2>> = Mutex::new(None);
        }

        if let Event::Mouse {
            position: pos,
            event,
            .. 
        } = $event
        {
            match event {
                Press(Right) => {
                    *OLD.lock() = Some(*pos);
                    return CONSUMED;
                }

                Hold(Right) if OLD.lock().is_none() => {
                    *OLD.lock() = Some(*pos);
                    return CONSUMED;
                }

                Hold(Right) => {
                    let old = OLD.lock().replace(*pos).unwrap();

                    let offset = ($ctx.0.content_viewport())
                        .top_left()
                        .map_x(|x| drag(x, pos.x, old.x))
                        .map_y(|y| drag(y, pos.y, old.y));

                    $ctx.0.set_offset(offset);

                    return CONSUMED;
                }

                Release(Right) => {
                    *OLD.lock() = None;
                    return CONSUMED;
                }

                _ => {}
            }
        }
    }};
}

pub(crate) struct EditorCtx<'a>(pub(crate) &'a mut ScrollView<EditorView>);

impl<'a> EditorCtx<'a> {
    /// Returns a new `EditorCtx`.
    pub fn new(view: &'a mut ScrollView<EditorView>) -> Self {
        Self(view)
    }

    /// Handles an event using the active tool.
    pub fn on_event(&mut self, event: &Event) -> Option<EventResult> {
        intercept_scrollbar!(self, event);
        intercept_pan!(self, event);

        let mode = self.0.get_inner_mut().read().mode;

        // Tool Delegation (Highest Priority)
        if !matches!(mode, EditorMode::Normal) {
            let mut tool_opt = self.0.get_inner_mut().write().active_tool.take();
            if let Some(mut tool) = tool_opt {
                let res = tool.on_event(self, event);
                
                let mut editor = self.0.get_inner_mut().write();
                // Only put the tool back if it wasn't replaced by another tool (e.g. SelectTool)
                if editor.active_tool.is_none() {
                    editor.active_tool = Some(tool);
                }

                if let Event::Mouse { event: Release(Left), .. } = event {
                    if !matches!(editor.mode, EditorMode::Select(_)) {
                        editor.mode = EditorMode::Normal;
                        editor.pending_count.clear();
                    }
                    return CONSUMED;
                }
                
                if res.is_some() { return res; }
            }
        }

        // 1. Box, Arrow, & Select Mode Handling (Keyboard)
        if let EditorMode::Box(start) | EditorMode::Arrow(start) | EditorMode::Line(start) | EditorMode::Select(start) = mode {
             if let Event::Char(c) = event {
                 match *c {
                     '1'..='9' => {
                        let mut editor = self.0.get_inner_mut().write();
                        editor.pending_count.push(*c);
                        return CONSUMED;
                    }
                    c if c == KEY_MOVE_LINE_START => {
                        let mut editor = self.0.get_inner_mut().write();
                        if editor.pending_count.is_empty() {
                            let mut pos = editor.buffer.get_cursor().unwrap_or_else(|| Vec2::new(0, 0));
                            pos.x = 0;
                            editor.buffer.set_cursor(pos);
                            drop(editor);
                            self.scroll_to_cursor();
                            return CONSUMED;
                        } else {
                            editor.pending_count.push(c);
                            return CONSUMED;
                        }
                    }
                    c if c == KEY_MOVE_FIRST_NON_WS => {
                        let mut editor = self.0.get_inner_mut().write();
                        editor.pending_count.clear();
                        let mut pos = editor.buffer.get_cursor().unwrap_or_else(|| Vec2::new(0, 0));
                        if let Some(line) = editor.buffer.chars.get(pos.y) {
                            pos.x = line.iter().position(|&c| !c.is_whitespace()).unwrap_or(0);
                        } else {
                            pos.x = 0;
                        }
                        editor.buffer.set_cursor(pos);
                        drop(editor);
                        self.scroll_to_cursor();
                        return CONSUMED;
                    }
                    c if c == KEY_MOVE_LAST_NON_WS => {
                        let mut editor = self.0.get_inner_mut().write();
                        editor.pending_count.clear();
                        let mut pos = editor.buffer.get_cursor().unwrap_or_else(|| Vec2::new(0, 0));
                        if let Some(line) = editor.buffer.chars.get(pos.y) {
                            pos.x = line.iter().rposition(|&c| !c.is_whitespace()).map(|p| p).unwrap_or(0);
                        } else {
                            pos.x = 0;
                        }
                        editor.buffer.set_cursor(pos);
                        drop(editor);
                        self.scroll_to_cursor();
                        return CONSUMED;
                    }
                    KEY_UNDO | KEY_SAVE | KEY_SAVE_AS | KEY_CLIP | KEY_CLIP_PREFIX | KEY_NEW
                    | KEY_OPEN | KEY_QUIT | KEY_DEBUG | KEY_CYCLE_PATH | KEY_TRIM_MARGINS | KEY_HELP => {
                        return None;
                    }
                    KEY_MOVE_LEFT | KEY_MOVE_DOWN | KEY_MOVE_UP | KEY_MOVE_RIGHT => {
                         let mut editor = self.0.get_inner_mut().write();
                         let count = editor.pending_count.parse::<usize>().unwrap_or(0);
                         let count = if count == 0 { 1 } else { count };
                         editor.pending_count.clear();

                         let mut pos = editor.buffer.get_cursor().unwrap_or_else(|| Vec2::new(0, 0));
                         match *c {
                             KEY_MOVE_LEFT => if pos.x >= count { pos.x -= count } else { pos.x = 0 },
                             KEY_MOVE_DOWN => pos.y += count,
                             KEY_MOVE_UP => if pos.y >= count { pos.y -= count } else { pos.y = 0 },
                             KEY_MOVE_RIGHT => pos.x += count,
                             _ => {}
                         }
                         editor.buffer.set_cursor(pos);
                         let opts_path_mode = editor.opts.path_mode;
                         let symbols = editor.opts.symbols.clone();
                         drop(editor);
                         self.scroll_to_cursor();
                         
                         match mode {
                             EditorMode::Box(_) => self.preview(|buf| draw_box_on_buffer(buf, start, pos, &symbols)),
                             EditorMode::Arrow(_) => self.preview(|buf| draw_arrow_on_buffer(buf, start, pos, opts_path_mode, &symbols)),
                             EditorMode::Line(_) => self.preview(|buf| draw_line_on_buffer(buf, start, pos, opts_path_mode, &symbols)),
                             EditorMode::Select(_) => self.preview(|_| ()), 
                             _ => {}
                         }
                         return CONSUMED;
                    }
                    'r' if matches!(mode, EditorMode::Arrow(_) | EditorMode::Line(_)) => {
                        let mut editor = self.0.get_inner_mut().write();
                        editor.mut_opts(|o| o.cycle_path_mode());
                        let pos = editor.buffer.get_cursor().unwrap_or_else(|| Vec2::new(0, 0));
                        let opts_path_mode = editor.opts.path_mode;
                        let symbols = editor.opts.symbols.clone();
                        drop(editor);
                        match mode {
                            EditorMode::Arrow(_) => self.preview(|buf| draw_arrow_on_buffer(buf, start, pos, opts_path_mode, &symbols)),
                            EditorMode::Line(_) => self.preview(|buf| draw_line_on_buffer(buf, start, pos, opts_path_mode, &symbols)),
                            _ => {}
                        }
                        return CONSUMED;
                    }
                    KEY_TOOL_ERASE if matches!(mode, EditorMode::Select(_)) => {
                        let end = self.0.get_inner_mut().read().buffer.get_cursor().unwrap_or_else(|| Vec2::new(0, 0));
                        let symbols = self.0.get_inner_mut().read().opts.symbols.clone();
                        self.clobber(|buf| erase_on_buffer(buf, start, end, &symbols));
                        let mut editor = self.0.get_inner_mut().write();
                        editor.mode = EditorMode::Select(end);
                        editor.buffer.discard_edits();
                        editor.pending_count.clear();
                        editor.buffer.set_cursor(end);
                        return CONSUMED;
                    }
                    KEY_TOOL_MOVE if matches!(mode, EditorMode::Select(_)) => {
                        let end = self.0.get_inner_mut().read().buffer.get_cursor().unwrap_or_else(|| Vec2::new(0, 0));
                        let mut editor = self.0.get_inner_mut().write();
                        let selection = Rect::from_corners(start, end);
                        let anchor = end;
                        let symbols = editor.opts.symbols.clone();
                        let tool = MoveTool::new(selection, anchor);
                        editor.set_tool(tool);
                        editor.pending_count.clear();
                        drop(editor);
                        self.preview(|buf| move_on_buffer(buf, selection, anchor, anchor, &symbols));
                        return CONSUMED;
                    }
                    KEY_TOOL_BOX | KEY_TOOL_ARROW | KEY_TOOL_LINE | KEY_TOOL_SELECT | KEY_TOOL_TEXT | '\n' => {
                        let end = self.0.get_inner_mut().read().buffer.get_cursor().unwrap_or_else(|| Vec2::new(0, 0));
                        let mut editor = self.0.get_inner_mut().write();
                        let opts_path_mode = editor.opts.path_mode;
                        let symbols = editor.opts.symbols.clone();
                        drop(editor);

                        match mode {
                            EditorMode::Box(_) => self.clobber(|buf| draw_box_on_buffer(buf, start, end, &symbols)),
                            EditorMode::Arrow(_) => self.clobber(|buf| draw_arrow_on_buffer(buf, start, end, opts_path_mode, &symbols)),
                            EditorMode::Line(_) => self.clobber(|buf| draw_line_on_buffer(buf, start, end, opts_path_mode, &symbols)),
                            _ => {}
                        }
                        
                        let mut editor = self.0.get_inner_mut().write();
                        editor.buffer.discard_edits();
                        editor.pending_count.clear();
                        
                        let cursor_pos = if editor.opts.box_cursor_start { start } else { end };
                        editor.buffer.set_cursor(cursor_pos);

                        match *c {
                            KEY_TOOL_BOX if !matches!(mode, EditorMode::Box(_)) => {
                                editor.mode = EditorMode::Box(cursor_pos);
                                let mut tool = BoxTool::default();
                                tool.load_opts(&editor.opts);
                                editor.set_tool(tool);
                                drop(editor);
                                self.preview(|buf| draw_box_on_buffer(buf, cursor_pos, cursor_pos, &symbols));
                            }
                            KEY_TOOL_ARROW if !matches!(mode, EditorMode::Arrow(_)) => {
                                editor.mode = EditorMode::Arrow(cursor_pos);
                                let mut tool = ArrowTool::default();
                                tool.load_opts(&editor.opts);
                                editor.set_tool(tool);
                                drop(editor);
                                self.preview(|buf| draw_arrow_on_buffer(buf, cursor_pos, cursor_pos, opts_path_mode, &symbols));
                            }
                            KEY_TOOL_LINE if !matches!(mode, EditorMode::Line(_)) => {
                                editor.mode = EditorMode::Line(cursor_pos);
                                let mut tool = LineTool::default();
                                tool.load_opts(&editor.opts);
                                editor.set_tool(tool);
                                drop(editor);
                                self.preview(|buf| draw_line_on_buffer(buf, cursor_pos, cursor_pos, opts_path_mode, &symbols));
                            }
                            KEY_TOOL_SELECT => {
                                editor.mode = EditorMode::Select(cursor_pos);
                                let mut tool = SelectTool::default();
                                tool.load_opts(&editor.opts);
                                editor.set_tool(tool);
                            }
                            KEY_TOOL_TEXT => {
                                editor.mode = EditorMode::Text;
                                let mut tool = TextTool::new(cursor_pos);
                                tool.load_opts(&editor.opts);
                                editor.set_tool(tool);
                            }
                            _ => {
                                editor.mode = EditorMode::Normal;
                            }
                        }
                        return CONSUMED;
                    }
                    _ => {}
                 }
             } else if let Event::CtrlChar('r') = event {
                 return None;
             } else if let Event::Char(KEY_UNDO) = event {
                 let mut editor = self.0.get_inner_mut().write();
                 editor.undo();
                 if matches!(mode, EditorMode::Select(_)) {
                     editor.mode = EditorMode::Normal;
                 }
                 editor.buffer.discard_edits();
                 editor.pending_count.clear();
                 return CONSUMED;
             } else if let Event::Key(Key::Esc) = event {
                 let end = self.0.get_inner_mut().read().buffer.get_cursor().unwrap_or_else(|| Vec2::new(0, 0));
                 let editor = self.0.get_inner_mut().write();
                 let opts_path_mode = editor.opts.path_mode;
                 let symbols = editor.opts.symbols.clone();
                 drop(editor);

                 match mode {
                     EditorMode::Box(_) => self.clobber(|buf| draw_box_on_buffer(buf, start, end, &symbols)),
                     EditorMode::Arrow(_) => self.clobber(|buf| draw_arrow_on_buffer(buf, start, end, opts_path_mode, &symbols)),
                     EditorMode::Line(_) => self.clobber(|buf| draw_line_on_buffer(buf, start, end, opts_path_mode, &symbols)),
                     _ => {} 
                 }

                 let mut editor = self.0.get_inner_mut().write();
                 editor.mode = EditorMode::Normal;
                 editor.buffer.discard_edits();
                 editor.pending_count.clear();
                 return CONSUMED;
             } else if let Event::Key(Key::Enter) = event {
                 let end = self.0.get_inner_mut().read().buffer.get_cursor().unwrap_or_else(|| Vec2::new(0, 0));
                 let editor = self.0.get_inner_mut().write();
                 let opts_path_mode = editor.opts.path_mode;
                 let symbols = editor.opts.symbols.clone();
                 drop(editor);

                 match mode {
                     EditorMode::Box(_) => self.clobber(|buf| draw_box_on_buffer(buf, start, end, &symbols)),
                     EditorMode::Arrow(_) => self.clobber(|buf| draw_arrow_on_buffer(buf, start, end, opts_path_mode, &symbols)),
                     EditorMode::Line(_) => self.clobber(|buf| draw_line_on_buffer(buf, start, end, opts_path_mode, &symbols)),
                     _ => {}
                 }
                 
                 let mut editor = self.0.get_inner_mut().write();
                 editor.mode = EditorMode::Normal;
                 editor.buffer.discard_edits();
                 editor.pending_count.clear();
                 if editor.opts.box_cursor_start {
                     editor.buffer.set_cursor(start);
                 } else {
                     editor.buffer.set_cursor(end);
                 }
                 return CONSUMED;
             }
             return CONSUMED;
        }
        
        // 3. Normal Mode Handling
        if mode == EditorMode::Normal {
            if let Event::Char(c) = event {
                let c = *c;
                match c {
                    '1'..='9' => {
                        let mut editor = self.0.get_inner_mut().write();
                        editor.pending_count.push(c);
                        return CONSUMED;
                    }
                    c if c == KEY_MOVE_LINE_START => {
                        let mut editor = self.0.get_inner_mut().write();
                        if editor.pending_count.is_empty() {
                            let mut pos = editor.buffer.get_cursor().unwrap_or_else(|| Vec2::new(0, 0));
                            pos.x = 0;
                            editor.buffer.set_cursor(pos);
                            drop(editor);
                            self.scroll_to_cursor();
                            return CONSUMED;
                        } else {
                            editor.pending_count.push(c);
                            return CONSUMED;
                        }
                    }
                    c if c == KEY_MOVE_FIRST_NON_WS => {
                        let mut editor = self.0.get_inner_mut().write();
                        editor.pending_count.clear();
                        let mut pos = editor.buffer.get_cursor().unwrap_or_else(|| Vec2::new(0, 0));
                        if let Some(line) = editor.buffer.chars.get(pos.y) {
                            pos.x = line.iter().position(|&c| !c.is_whitespace()).unwrap_or(0);
                        } else {
                            pos.x = 0;
                        }
                        editor.buffer.set_cursor(pos);
                        drop(editor);
                        self.scroll_to_cursor();
                        return CONSUMED;
                    }
                    c if c == KEY_MOVE_LAST_NON_WS => {
                        let mut editor = self.0.get_inner_mut().write();
                        editor.pending_count.clear();
                        let mut pos = editor.buffer.get_cursor().unwrap_or_else(|| Vec2::new(0, 0));
                        if let Some(line) = editor.buffer.chars.get(pos.y) {
                            pos.x = line.iter().rposition(|&c| !c.is_whitespace()).map(|p| p).unwrap_or(0);
                        } else {
                            pos.x = 0;
                        }
                        editor.buffer.set_cursor(pos);
                        drop(editor);
                        self.scroll_to_cursor();
                        return CONSUMED;
                    }
                    KEY_MOVE_LEFT | KEY_MOVE_DOWN | KEY_MOVE_UP | KEY_MOVE_RIGHT => {
                        let mut editor = self.0.get_inner_mut().write();
                        let count = editor.pending_count.parse::<usize>().unwrap_or(0);
                        let count = if count == 0 { 1 } else { count };
                        editor.pending_count.clear();

                        let mut pos = editor.buffer.get_cursor().unwrap_or_else(|| Vec2::new(0, 0));
                        match c {
                            KEY_MOVE_LEFT => if pos.x >= count { pos.x -= count } else { pos.x = 0 },
                            KEY_MOVE_DOWN => pos.y += count,
                            KEY_MOVE_UP => if pos.y >= count { pos.y -= count } else { pos.y = 0 },
                            KEY_MOVE_RIGHT => pos.x += count,
                            _ => {}
                        }
                        editor.buffer.set_cursor(pos);
                        drop(editor);
                        self.scroll_to_cursor();
                        return CONSUMED;
                    }
                    KEY_TOOL_BOX => {
                        let mut editor = self.0.get_inner_mut().write();
                        editor.pending_count.clear();
                        let pos = editor.buffer.get_cursor().unwrap_or(Vec2::new(0, 0));
                        editor.mode = EditorMode::Box(pos);
                        let mut tool = BoxTool::default();
                        tool.load_opts(&editor.opts);
                        editor.set_tool(tool);
                        let symbols = editor.opts.symbols.clone();
                        drop(editor);
                        self.preview(|buf| draw_box_on_buffer(buf, pos, pos, &symbols));
                        return CONSUMED;
                    }
                    KEY_TOOL_ARROW => {
                        let mut editor = self.0.get_inner_mut().write();
                        editor.pending_count.clear();
                        let pos = editor.buffer.get_cursor().unwrap_or(Vec2::new(0, 0));
                        editor.mode = EditorMode::Arrow(pos);
                        let mut tool = ArrowTool::default();
                        tool.load_opts(&editor.opts);
                        editor.set_tool(tool);
                        let opts_path_mode = editor.opts.path_mode;
                        let symbols = editor.opts.symbols.clone();
                        drop(editor);
                        self.preview(|buf| draw_arrow_on_buffer(buf, pos, pos, opts_path_mode, &symbols));
                        return CONSUMED;
                    }
                    KEY_TOOL_SELECT => {
                        let mut editor = self.0.get_inner_mut().write();
                        editor.pending_count.clear();
                        let pos = editor.buffer.get_cursor().unwrap_or(Vec2::new(0, 0));
                        editor.mode = EditorMode::Select(pos);
                        let mut tool = SelectTool::default();
                        tool.load_opts(&editor.opts);
                        editor.set_tool(tool);
                        return CONSUMED;
                    }
                    KEY_TOOL_TEXT => {
                        let mut editor = self.0.get_inner_mut().write();
                        editor.pending_count.clear();
                        let pos = editor.buffer.get_cursor().unwrap_or(Vec2::new(0, 0));
                        editor.mode = EditorMode::Text;
                        let mut tool = TextTool::new(pos);
                        tool.load_opts(&editor.opts);
                        editor.set_tool(tool);
                        return CONSUMED;
                    }
                    KEY_TOOL_LINE => {
                        let mut editor = self.0.get_inner_mut().write();
                        editor.pending_count.clear();
                        let pos = editor.buffer.get_cursor().unwrap_or(Vec2::new(0, 0));
                        editor.mode = EditorMode::Line(pos);
                        let mut tool = LineTool::default();
                        tool.load_opts(&editor.opts);
                        editor.set_tool(tool);
                        let opts_path_mode = editor.opts.path_mode;
                        let symbols = editor.opts.symbols.clone();
                        drop(editor);
                        self.preview(|buf| draw_line_on_buffer(buf, pos, pos, opts_path_mode, &symbols));
                        return CONSUMED;
                    }
                    KEY_UNDO | KEY_SAVE | KEY_SAVE_AS | KEY_CLIP | KEY_CLIP_PREFIX | KEY_NEW
                    | KEY_OPEN | KEY_QUIT | KEY_DEBUG | KEY_CYCLE_PATH | KEY_TRIM_MARGINS | KEY_HELP => {
                        let mut editor = self.0.get_inner_mut().write();
                        editor.pending_count.clear();
                        return None;
                    }
                    _ => {
                        let mut editor = self.0.get_inner_mut().write();
                        editor.pending_count.clear();
                        return CONSUMED;
                    }
                }
            } else if let Event::Mouse { event: Press(Left) | Hold(Left), .. } = event {
                {
                    let mut editor = self.0.get_inner_mut().write();
                    editor.pending_count.clear();
                }
                let rel_event = self.relativize(event);
                if let Event::Mouse { position, .. } = rel_event {
                    let mut editor = self.0.get_inner_mut().write();
                    editor.buffer.set_cursor(position);
                    drop(editor);
                    self.scroll_to_cursor();
                    return CONSUMED;
                }
            } else if let Event::Key(Key::Esc) = event {
                 let mut editor = self.0.get_inner_mut().write();
                 editor.pending_count.clear();
            }
        }
        
        // 4. Tool Delegation (Lower Priority for Normal Mode / Fallback)
        let mut editor_guard = self.0.get_inner_mut().write();
        let tool_exists = editor_guard.active_tool.is_some();
        if tool_exists {
            let mut tool = editor_guard.active_tool.take().unwrap();
            drop(editor_guard);
            
            let res = tool.on_event(self, event);
            
            self.0.get_inner_mut().write().active_tool = Some(tool);

            if mode == EditorMode::Text && res.is_none() {
                 if let Event::Key(Key::Esc) = event {
                     let mut editor = self.0.get_inner_mut().write();
                     editor.mode = EditorMode::Normal;
                     return CONSUMED;
                 }
            }

            if res.is_some() { return res; }
        } else {
            drop(editor_guard);
        }

        None
    }

    /// Returns `true` if `pos` is located on a scrollbar.
    fn on_scrollbar(&self, offset: Vec2, pos: Vec2) -> bool {
        let core = self.0.get_scroller();
        let max = core.last_outer_size() + offset;
        let min = max - core.scrollbar_size();

        (min.x..=max.x).contains(&pos.x) || (min.y..=max.y).contains(&pos.y)
    }

    /// If `event` is a mouse event, relativize its position to the canvas plane.
    pub(crate) fn relativize(&self, event: &Event) -> Event {
        let mut event = event.clone();
        if let Event::Mouse {
            offset, position, ..
        } = &mut event {
            let tl = self.0.content_viewport().top_left();
            *position = position.saturating_sub(*offset) + tl;
            
            // Adjust for gutter
            if position.x >= GUTTER_WIDTH {
                position.x -= GUTTER_WIDTH;
            } else {
                position.x = 0; // Clamp to 0 if inside gutter
            }
        }
        event
    }

    /// Scroll to `pos`, moving at least `step_x` & `step_y` respectively if the x or y
    /// scroll offset needs to be modified.
    pub(crate) fn scroll_to(&mut self, pos: Vec2, step_x: usize, step_y: usize) {
        let port = self.0.content_viewport();
        let mut offset = port.top_left();

        // View X = Buffer_X + GUTTER_WIDTH
        let v_x = pos.x + GUTTER_WIDTH;

        if v_x >= port.right() {
            offset.x += max(step_x, v_x + 1 - port.right());
        } else if v_x < offset.x + GUTTER_WIDTH - 1 {
            offset.x -= max(min(step_x, offset.x), (offset.x + GUTTER_WIDTH - 1) - v_x);
        }

        if pos.y >= port.bottom() {
            offset.y += max(step_y, pos.y + 1 - port.bottom());
        } else if pos.y <= port.top() {
            offset.y -= max(min(step_y, offset.y), port.top() - pos.y);
        }
        self.0.set_offset(offset);
    }

    /// Scroll to the edit buffer's current cursor, if one exists.
    pub(crate) fn scroll_to_cursor(&mut self) {
        let pos = self.0.get_inner_mut().read().buffer.get_cursor();

        if let Some(pos) = pos {
            self.scroll_to(pos, 1, 1);
        }
    }

    /// Modify the edit buffer using `render`, flushing any changes and saving a snapshot
    /// of the buffer's prior state in the editor's undo history.
    pub(crate) fn clobber<R: FnOnce(&mut Buffer)>(&mut self, render: R) {
        let mut editor = self.0.get_inner_mut().write();

        editor.with_snapshot(|ed| {
            render(&mut ed.buffer);
            ed.buffer.flush_edits();
        });
    }

    /// Modify the edit buffer using `render`, without flushing any changes.
    pub(crate) fn preview<R: FnOnce(&mut Buffer)>(&mut self, render: R) {
        let mut editor = self.0.get_inner_mut().write();
        editor.buffer.discard_edits();
        render(&mut editor.buffer);
    }
}

fn drag(x: usize, new: usize, old: usize) -> usize {
    if new > old {
        x.saturating_sub(new - old)
    } else {
        x + (old - new)
    }
}

