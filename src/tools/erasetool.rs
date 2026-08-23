use cursive::Vec2;

use super::visible_cells;
use crate::config::Symbols;
use crate::constants::SP;
use crate::editor::buffer::Buffer;

pub fn erase_on_buffer(buf: &mut Buffer, src: Vec2, dst: Vec2, symbols: &Symbols) {
    let state: Vec<_> = visible_cells(buf, (src, dst), symbols).collect();

    for cell in state {
        buf.setv(true, cell.pos(), SP, symbols);
    }

    buf.set_cursor(dst);
}
