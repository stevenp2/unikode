use crate::config::Symbols;
use crate::editor::buffer::Buffer;
use cursive::Vec2;

/// Returns true if the character provides a connection point on its BOTTOM edge (pointing South).
pub fn connects_down(c: char, s: &Symbols) -> bool {
    c == s.vline || c == s.tlcorn || c == s.trcorn || c == s.lhinter || c == s.rhinter || c == s.tvinter || c == s.cinter || c == s.plus || c == s.n
}

/// Returns true if the character provides a connection point on its TOP edge (pointing North).
pub fn connects_up(c: char, s: &Symbols) -> bool {
    c == s.vline || c == s.blcorn || c == s.brcorn || c == s.lhinter || c == s.rhinter || c == s.bvinter || c == s.cinter || c == s.plus || c == s.s
}

/// Returns true if the character provides a connection point on its RIGHT edge (pointing East).
pub fn connects_right(c: char, s: &Symbols) -> bool {
    c == s.hline || c == s.tlcorn || c == s.blcorn || c == s.lhinter || c == s.tvinter || c == s.bvinter || c == s.cinter || c == s.plus || c == s.w
}

/// Returns true if the character provides a connection point on its LEFT edge (pointing West).
pub fn connects_left(c: char, s: &Symbols) -> bool {
    c == s.hline || c == s.trcorn || c == s.brcorn || c == s.rhinter || c == s.tvinter || c == s.bvinter || c == s.cinter || c == s.plus || c == s.e
}

pub fn is_joinable(c: char, s: &Symbols) -> bool {
    c == s.vline || c == s.hline || c == s.tlcorn || c == s.trcorn || 
    c == s.blcorn || c == s.brcorn || c == s.lhinter || c == s.rhinter || 
    c == s.tvinter || c == s.bvinter || c == s.cinter || c == s.plus ||
    c == s.n || c == s.s || c == s.w || c == s.e
}

pub fn is_arrow_tip(c: char, s: &Symbols) -> bool {
    c == s.n || c == s.s || c == s.w || c == s.e
}

pub fn get_smart_char(n: bool, s: bool, w: bool, e: bool, symbols: &Symbols, fallback: char) -> char {
    match (n, s, w, e) {
        (true, true, true, true) => symbols.cinter,
        (true, true, true, false) => symbols.rhinter,
        (true, true, false, true) => symbols.lhinter,
        (true, false, true, true) => symbols.bvinter,
        (false, true, true, true) => symbols.tvinter,
        (true, true, false, false) => symbols.vline,
        (false, false, true, true) => symbols.hline,
        (false, true, false, true) => symbols.tlcorn,
        (false, true, true, false) => symbols.trcorn,
        (true, false, false, true) => symbols.blcorn,
        (true, false, true, false) => symbols.brcorn,
        (true, _, _, _) | (_, true, _, _) => symbols.vline,
        (_, _, true, _) | (_, _, _, true) => symbols.hline,
        _ => fallback,
    }
}

pub fn fixup_point(pos: Vec2, buf: &Buffer, symbols: &Symbols) -> char {
    let current = buf.get_char_at(pos);
    
    // Never replace an arrow tip with a box character
    if is_arrow_tip(current, symbols) {
        return current;
    }

    if !is_joinable(current, symbols) && current != ' ' {
        return current;
    }

    let n = buf.get_char_at(pos.saturating_sub(Vec2::new(0, 1)));
    let s = buf.get_char_at(pos.saturating_add(Vec2::new(0, 1)));
    let w = buf.get_char_at(pos.saturating_sub(Vec2::new(1, 0)));
    let e = buf.get_char_at(pos.saturating_add(Vec2::new(1, 0)));

    let nc = connects_down(n, symbols);
    let sc = connects_up(s, symbols);
    let wc = connects_right(w, symbols);
    let ec = connects_left(e, symbols);

    let smart = get_smart_char(nc, sc, wc, ec, symbols, current);
    
    if current == ' ' {
        let count = [nc, sc, wc, ec].iter().filter(|&&b| b).count();
        if count < 2 {
            return ' ';
        }
    }

    smart
}
