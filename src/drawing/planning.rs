use cursive::Vec2;
use pathfinding::directed::astar::astar;

use crate::{
    config::{PathMode, RenderMode},
    constants::{D, D2},
    editor::buffer::Buffer,
};

use super::{ord_float::OrdFloat, DrawRequest, DrawingPlan, DrawingStyle, ShapeKind};

pub(super) fn plan(buffer: &Buffer, request: DrawRequest, style: &DrawingStyle) -> DrawingPlan {
    let (points, degenerate_box) = match request.kind {
        ShapeKind::Box => rectangle(request.from, request.to),
        ShapeKind::Line | ShapeKind::Arrow => {
            (path(buffer, request.from, request.to, style), false)
        }
    };

    DrawingPlan {
        kind: request.kind,
        points,
        connection_origin: request.from,
        cursor: request.to,
        degenerate_box,
    }
}

fn rectangle(from: Vec2, to: Vec2) -> (Vec<Vec2>, bool) {
    let left = from.x.min(to.x);
    let right = from.x.max(to.x);
    let top = from.y.min(to.y);
    let bottom = from.y.max(to.y);
    let top_left = Vec2::new(left, top);
    let top_right = Vec2::new(right, top);
    let bottom_right = Vec2::new(right, bottom);
    let bottom_left = Vec2::new(left, bottom);
    let degenerate = left == right || top == bottom;

    if degenerate {
        (vec![from, to], true)
    } else {
        (
            vec![top_left, top_right, bottom_right, bottom_left, top_left],
            false,
        )
    }
}

fn path(buffer: &Buffer, from: Vec2, to: Vec2, style: &DrawingStyle) -> Vec<Vec2> {
    match (style.render_mode, style.path_mode) {
        (RenderMode::Braille, PathMode::Routed) => routed(buffer, from, to),
        (RenderMode::Classic, _) => {
            let middle = if buffer.getv(to) == Some(style.symbols.hline) {
                Vec2::new(to.x, from.y)
            } else {
                Vec2::new(from.x, to.y)
            };
            polyline(from, middle, to)
        }
        (RenderMode::Braille, PathMode::Snap90) => polyline(from, Vec2::new(from.x, to.y), to),
    }
}

fn polyline(from: Vec2, middle: Vec2, to: Vec2) -> Vec<Vec2> {
    if middle == from || middle == to {
        vec![from, to]
    } else {
        vec![from, middle, to]
    }
}

fn routed(buffer: &Buffer, from: Vec2, to: Vec2) -> Vec<Vec2> {
    astar(
        &from.pair(),
        |&position| neighbors(buffer, position),
        |&position| heuristic(position.into(), to),
        |&position| position == to.pair(),
    )
    .map(|(points, _)| points.into_iter().map(Vec2::from).collect())
    .unwrap_or_else(|| vec![from, to])
}

fn neighbors(buffer: &Buffer, pos: (usize, usize)) -> Vec<((usize, usize), OrdFloat)> {
    let vis = |pos: (usize, usize)| buffer.visible(pos.into());

    let card = |pos| (pos, OrdFloat((vis(pos) as u8 as f64 * 64.0) + D));

    let diag = |pos, (c1, c2)| {
        let cost = {
            let spot = vis(pos) as u8 as f64 * 64.0;
            let edge = (vis(c1) && vis(c2)) as u8 as f64 * 64.0;
            spot + edge
        };

        (pos, OrdFloat(cost + D2))
    };

    let w = |(x, y)| (x - 1, y);
    let n = |(x, y)| (x, y - 1);
    let e = |(x, y)| (x + 1, y);
    let s = |(x, y)| (x, y + 1);

    let mut succ = Vec::with_capacity(8);
    if pos.0 > 0 && pos.1 > 0 {
        succ.push(diag(n(w(pos)), (n(pos), w(pos))));
    }
    if pos.0 > 0 {
        succ.push(card(w(pos)));
        succ.push(diag(s(w(pos)), (s(pos), w(pos))));
    }
    if pos.1 > 0 {
        succ.push(card(n(pos)));
        succ.push(diag(n(e(pos)), (n(pos), e(pos))));
    }
    succ.push(card(e(pos)));
    succ.push(card(s(pos)));
    succ.push(diag(s(e(pos)), (s(pos), e(pos))));

    succ
}

fn heuristic(position: Vec2, destination: Vec2) -> OrdFloat {
    let dx = position.x.abs_diff(destination.x) as f64;
    let dy = position.y.abs_diff(destination.y) as f64;
    let distance = if dx > dy {
        D * (dx - dy) + D2 * dy
    } else {
        D * (dy - dx) + D2 * dx
    };
    const TIE_BREAKER: f64 = 1.0 + (1.0 / 1000.0);
    OrdFloat(distance * TIE_BREAKER)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Symbols;

    fn style(render_mode: RenderMode, path_mode: PathMode) -> DrawingStyle {
        DrawingStyle {
            render_mode,
            path_mode,
            symbols: Symbols::default(),
        }
    }

    #[test]
    fn classic_plans_only_orthogonal_segments() {
        let buffer = Buffer::default();
        let points = path(
            &buffer,
            Vec2::new(0, 0),
            Vec2::new(3, 2),
            &style(RenderMode::Classic, PathMode::Routed),
        );
        assert!(points
            .windows(2)
            .all(|edge| edge[0].x == edge[1].x || edge[0].y == edge[1].y));
    }

    #[test]
    fn routed_braille_path_can_use_diagonal_steps() {
        let buffer = Buffer::default();
        let points = path(
            &buffer,
            Vec2::new(0, 0),
            Vec2::new(3, 3),
            &style(RenderMode::Braille, PathMode::Routed),
        );
        assert!(points
            .windows(2)
            .any(|edge| edge[0].x != edge[1].x && edge[0].y != edge[1].y));
    }
}
