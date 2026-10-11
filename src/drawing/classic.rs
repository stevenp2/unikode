use cursive::Vec2;
use line_drawing::Bresenham;

use crate::{config::Symbols, constants::SP, editor::buffer::Buffer};

use super::{
    braille::BrailleMask,
    junctions::{fixup_point, is_joinable},
    DrawingPlan, ShapeKind,
};

pub(super) fn render(buffer: &mut Buffer, plan: &DrawingPlan, symbols: &Symbols) {
    if plan.kind == ShapeKind::Box && plan.degenerate_box {
        write(buffer, false, plan.connection_origin, symbols.ubox, symbols);
        return;
    }

    let mut touched = Vec::new();
    if let Some(&point) = plan.points.first() {
        touched.push(point);
    }
    for edge in plan.points.windows(2) {
        touched.extend(draw_segment(buffer, edge[0], edge[1], symbols));
    }
    fixup(buffer, &touched, plan.is_arrow(), symbols);
}

fn draw_segment(buffer: &mut Buffer, from: Vec2, to: Vec2, symbols: &Symbols) -> Vec<Vec2> {
    let vertical = from.x == to.x;
    let horizontal = from.y == to.y;
    let mut touched = Vec::new();

    for (index, (position, _)) in Bresenham::new(from.signed().pair(), to.signed().pair())
        .steps()
        .enumerate()
    {
        let position = Vec2::new(position.0 as usize, position.1 as usize);
        let character = if index == 0 {
            symbols.plus
        } else if vertical {
            symbols.vline
        } else if horizontal {
            symbols.hline
        } else {
            SP
        };
        write(buffer, false, position, character, symbols);
        touched.push(position);
    }

    write(buffer, false, to, symbols.plus, symbols);
    touched.push(to);
    touched
}

fn fixup(buffer: &mut Buffer, points: &[Vec2], arrow: bool, symbols: &Symbols) {
    let arrow_destination = arrow.then(|| points.last().copied()).flatten();
    let mut changes = Vec::new();

    for &position in points {
        if Some(position) == arrow_destination {
            continue;
        }
        let character = fixup_point(position, buffer, symbols);
        if is_joinable(character, symbols) {
            changes.push((position, character));
        }
    }

    for (position, character) in changes {
        write(buffer, true, position, character, symbols);
    }
}

fn write(buffer: &mut Buffer, force: bool, position: Vec2, character: char, symbols: &Symbols) {
    if BrailleMask::decode(buffer.get_char_at(position)).is_none() {
        buffer.setv(force, position, character, symbols);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{PathMode, RenderMode};
    use crate::drawing::{DrawRequest, DrawingStyle};

    #[test]
    fn generic_closed_polyline_renders_a_classic_box() {
        let mut buffer = Buffer::default();
        let style = DrawingStyle {
            render_mode: RenderMode::Classic,
            path_mode: PathMode::Snap90,
            symbols: Symbols::default(),
        };
        crate::drawing::draw(
            &mut buffer,
            DrawRequest::box_(Vec2::new(0, 0), Vec2::new(4, 2)),
            &style,
        );
        buffer.flush_edits();

        assert_eq!(buffer.iter("").collect::<String>(), "┌───┐\n│   │\n└───┘\n");
    }
}
