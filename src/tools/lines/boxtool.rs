use cursive::{
    event::{
        Event, EventResult, MouseButton::Left,
        MouseEvent::{Hold, Press, Release},
    },
    Rect, Vec2
};

use line_drawing::Bresenham;
use std::fmt;

use crate::{
    utils::rectedges::RectEdges,
    utils::junctions::*,
    constants::{
        SP, CONSUMED
    },
    editor::{buffer::Buffer, scroll::EditorCtx},
    config::{Options, Symbols}
};

use super::super::{Tool, simple_display, fn_on_event_drag, option, mouse_drag};

#[derive(Clone, Default)]
pub(crate) struct BoxTool {
    src: Option<Vec2>,
    dst: Option<Vec2>,
    symbols: Symbols,
}

impl Tool for BoxTool {
    fn load_opts(&mut self, opts: &Options) {
        self.symbols = opts.symbols.clone();
    }

    fn on_event(&mut self, ctx: &mut EditorCtx<'_>, event: &Event) -> Option<EventResult> {
        let (pos, event) = mouse_drag!(ctx, event);

        match event {
            Press(Left) => {
                self.src = Some(pos);
                self.dst = Some(pos);
                ctx.preview(|buf| draw_box_on_buffer(buf, pos, pos, &self.symbols));
            }

            Hold(Left) => {
                self.dst = Some(pos);
                ctx.preview(|buf| {
                    if let Some(src) = self.src {
                        draw_box_on_buffer(buf, src, pos, &self.symbols);
                    }
                });
            }

            Release(Left) => {
                self.dst = Some(pos);
                ctx.clobber(|buf| {
                    if let Some(src) = self.src {
                        draw_box_on_buffer(buf, src, pos, &self.symbols);
                    }
                });
                self.src = None;
                self.dst = None;
            }

            _ => return None,
        }

        CONSUMED
    }
}

simple_display! { BoxTool, "Box" }

pub fn draw_box_on_buffer(buf: &mut Buffer, src: Vec2, dst: Vec2, symbols: &Symbols) {
    let rect = Rect::from_corners(src, dst);
    let re = RectEdges::new(rect);

    if rect.top_left() == rect.top_right() || rect.bottom_left() == rect.bottom_right() || rect.top_left() == rect.bottom_left() {
        buf.set(false, src.x, src.y, symbols.ubox, symbols);
        return
    }

    draw_line_rect(buf, rect.top_left(), rect.top_right(), rect, symbols);
    draw_line_rect(buf, rect.top_right(), rect.bottom_right(), rect, symbols);
    draw_line_rect(buf, rect.bottom_right(), rect.bottom_left(), rect, symbols);
    draw_line_rect(buf, rect.bottom_left(), rect.top_left(), rect, symbols);

    let mut change_set = Vec::new();

    for &(x, y) in &re.coordinate_outline {
        let pos = Vec2::new(x, y);
        let new_char = fixup_point(pos, buf, symbols);

        if is_joinable(new_char, symbols) {
            change_set.push((pos, new_char));
        }
    }

    for (pos, c) in change_set {
        buf.setv(true, pos, c, symbols);
    }
    
    buf.set_cursor(dst);
}

fn draw_line_rect(buf: &mut Buffer, src: Vec2, dst: Vec2, r: Rect, symbols: &Symbols) {
    let steps = Bresenham::new(src.signed().pair(), dst.signed().pair()).steps();

    for (i, (a, _)) in steps.enumerate() {
        let c = match (i, src, dst) {
            (0, s, _) if s == r.top_left() => symbols.tlcorn,
            (0, s, _) if s == r.top_right() => symbols.trcorn,
            (0, s, _) if s == r.bottom_left() => symbols.blcorn,
            (0, s, _) if s == r.bottom_right() => symbols.brcorn,
            (_, s, d) if i > 0 && i < d.x.abs_diff(s.x) => symbols.hline,
            (_, s, d) if i > 0 && i < d.y.abs_diff(s.y) => symbols.vline,
            _ => SP,
        };

        buf.set(false, a.0 as usize, a.1 as usize, c, symbols)
    }

}
