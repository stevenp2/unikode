use cursive::{Rect, Vec2};

use crate::{config::Symbols, constants::SP};

use super::{buffer::Buffer, cell::Cell};

fn cells(buffer: &Buffer, region: Rect) -> impl Iterator<Item = Cell> + '_ {
    buffer
        .chars
        .iter()
        .enumerate()
        .skip(region.top())
        .take(region.height())
        .flat_map(move |(y, row)| {
            row.iter()
                .copied()
                .enumerate()
                .skip(region.left())
                .take(region.width())
                .filter(|(_, c)| !c.is_whitespace())
                .map(move |(x, c)| Cell {
                    pos: Vec2::new(x, y),
                    c,
                })
        })
}

pub(crate) fn erase_on_buffer(buffer: &mut Buffer, from: Vec2, to: Vec2, symbols: &Symbols) {
    let cells: Vec<_> = cells(buffer, Rect::from_corners(from, to)).collect();
    for cell in cells {
        buffer.setv(true, cell.pos, SP, symbols);
    }
    buffer.set_cursor(to);
}

pub(crate) fn move_destination(selection: Rect, anchor: Vec2, to: Vec2) -> Vec2 {
    let delta = (to.signed() - anchor.signed())
        .map_x(|x| x.max(-(selection.left() as isize)))
        .map_y(|y| y.max(-(selection.top() as isize)));
    anchor.saturating_add(delta)
}

pub(crate) fn move_on_buffer(
    buffer: &mut Buffer,
    selection: Rect,
    anchor: Vec2,
    to: Vec2,
    symbols: &Symbols,
) {
    let cells: Vec<_> = cells(buffer, selection).collect();
    let to = move_destination(selection, anchor, to);
    let delta = to.signed() - anchor.signed();
    for cell in &cells {
        buffer.setv(true, cell.pos, SP, symbols);
    }
    for cell in cells {
        buffer.setv(true, cell.pos.saturating_add(delta), cell.c, symbols);
    }
    buffer.set_cursor(to);
}
