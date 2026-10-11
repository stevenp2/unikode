use cursive::Vec2;

use crate::{
    config::Symbols,
    constants::{S_E, S_N, S_S, S_W},
    editor::buffer::Buffer,
};

pub(super) fn place(buffer: &mut Buffer, from: Vec2, to: Vec2, symbols: &Symbols) {
    let north = to.y > 0 && buffer.visible(Vec2::new(to.x, to.y - 1));
    let east = buffer.visible(Vec2::new(to.x + 1, to.y));
    let south = buffer.visible(Vec2::new(to.x, to.y + 1));
    let west = to.x > 0 && buffer.visible(Vec2::new(to.x - 1, to.y));

    let tip = match (to.signed() - from.signed()).map(isize::signum).pair() {
        S_N if north || (west && east) => symbols.n,
        S_N if west => symbols.w,
        S_N if east => symbols.e,
        S_N => symbols.n,
        S_E if east || (north && south) => symbols.e,
        S_E if north => symbols.n,
        S_E if south => symbols.s,
        S_E => symbols.e,
        S_S if south || (east && west) => symbols.s,
        S_S if east => symbols.e,
        S_S if west => symbols.w,
        S_S => symbols.s,
        S_W if west || (south && north) => symbols.w,
        S_W if south => symbols.s,
        S_W if north => symbols.n,
        S_W => symbols.w,
        (x, y) if x > 0 && y > 0 && east => symbols.e,
        (x, y) if x > 0 && y > 0 => symbols.s,
        (x, y) if x > 0 && y < 0 && east => symbols.e,
        (x, y) if x > 0 && y < 0 => symbols.n,
        (x, y) if x < 0 && y > 0 && west => symbols.w,
        (x, y) if x < 0 && y > 0 => symbols.s,
        (x, y) if x < 0 && y < 0 && west => symbols.w,
        (x, y) if x < 0 && y < 0 => symbols.n,
        _ => return,
    };

    buffer.setv(true, to, tip, symbols);
}
