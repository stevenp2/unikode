mod arrowhead;
mod braille;
mod classic;
mod junctions;
mod ord_float;
mod planning;

use cursive::Vec2;

use crate::{
    config::{Options, PathMode, RenderMode, Symbols},
    editor::buffer::Buffer,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ShapeKind {
    Box,
    Line,
    Arrow,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct DrawRequest {
    pub kind: ShapeKind,
    pub from: Vec2,
    pub to: Vec2,
}

impl DrawRequest {
    pub(crate) fn new(kind: ShapeKind, from: Vec2, to: Vec2) -> Self {
        Self { kind, from, to }
    }

    pub(crate) fn box_(from: Vec2, to: Vec2) -> Self {
        Self::new(ShapeKind::Box, from, to)
    }

    pub(crate) fn line(from: Vec2, to: Vec2) -> Self {
        Self::new(ShapeKind::Line, from, to)
    }

    pub(crate) fn arrow(from: Vec2, to: Vec2) -> Self {
        Self::new(ShapeKind::Arrow, from, to)
    }
}

#[derive(Clone, Debug)]
pub(crate) struct DrawingStyle {
    pub render_mode: RenderMode,
    pub path_mode: PathMode,
    pub symbols: Symbols,
}

impl DrawingStyle {
    pub(crate) fn from_options(options: &Options) -> Self {
        Self {
            render_mode: options.render_mode,
            path_mode: options.effective_path_mode(),
            symbols: options.symbols.clone(),
        }
    }
}

impl Default for DrawingStyle {
    fn default() -> Self {
        Self::from_options(&Options::default())
    }
}

#[derive(Clone, Debug)]
struct DrawingPlan {
    kind: ShapeKind,
    points: Vec<Vec2>,
    connection_origin: Vec2,
    cursor: Vec2,
    degenerate_box: bool,
}

impl DrawingPlan {
    fn is_arrow(&self) -> bool {
        self.kind == ShapeKind::Arrow
    }

    fn arrow_direction(&self) -> Option<(Vec2, Vec2)> {
        let destination = *self.points.last()?;
        let previous = self
            .points
            .iter()
            .rev()
            .skip(1)
            .copied()
            .find(|point| *point != destination)?;
        Some((previous, destination))
    }
}

pub(crate) fn draw(buffer: &mut Buffer, request: DrawRequest, style: &DrawingStyle) {
    let plan = planning::plan(buffer, request, style);

    match style.render_mode {
        RenderMode::Classic => classic::render(buffer, &plan, &style.symbols),
        RenderMode::Braille => braille::render(buffer, &plan, &style.symbols),
    }

    if plan.is_arrow() {
        if let Some((previous, destination)) = plan.arrow_direction() {
            arrowhead::place(buffer, previous, destination, &style.symbols);
        }
    }
    buffer.set_cursor(plan.cursor);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_length_braille_arrow_has_no_classic_arrowhead() {
        let mut buffer = Buffer::default();
        let style = DrawingStyle {
            render_mode: RenderMode::Braille,
            path_mode: PathMode::Routed,
            symbols: Symbols::default(),
        };
        let position = Vec2::new(2, 2);

        draw(&mut buffer, DrawRequest::arrow(position, position), &style);

        assert!(braille::BrailleMask::decode(buffer.get_char_at(position)).is_some());
    }

    #[test]
    fn braille_diagonal_arrows_use_cardinal_arrowheads_in_all_quadrants() {
        let style = DrawingStyle {
            render_mode: RenderMode::Braille,
            path_mode: PathMode::Routed,
            symbols: Symbols::default(),
        };
        for (from, to) in [
            (Vec2::new(2, 2), Vec2::new(5, 5)),
            (Vec2::new(5, 5), Vec2::new(2, 2)),
            (Vec2::new(2, 5), Vec2::new(5, 2)),
            (Vec2::new(5, 2), Vec2::new(2, 5)),
        ] {
            let mut buffer = Buffer::default();
            draw(&mut buffer, DrawRequest::arrow(from, to), &style);
            assert!([
                style.symbols.n,
                style.symbols.s,
                style.symbols.w,
                style.symbols.e,
            ]
            .contains(&buffer.get_char_at(to)));
        }
    }

    #[test]
    fn classic_geometry_does_not_overwrite_braille_cells() {
        let position = Vec2::new(1, 0);
        let symbols = Symbols::default();
        let mut buffer = Buffer::default();
        buffer.setv(true, position, '\u{280f}', &symbols);
        buffer.flush_edits();
        let style = DrawingStyle {
            render_mode: RenderMode::Classic,
            path_mode: PathMode::Snap90,
            symbols,
        };

        draw(
            &mut buffer,
            DrawRequest::line(Vec2::new(0, 0), Vec2::new(3, 0)),
            &style,
        );

        assert_eq!(buffer.get_char_at(position), '\u{280f}');
    }
}
