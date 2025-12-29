use std::collections::HashSet;

use cursive::{Rect, Vec2};

pub struct RectEdges {
    pub rect: Rect,
    pub coordinate_outline: HashSet<(usize, usize)>
}

impl RectEdges {
    pub fn new(rect: Rect) -> Self {

        let top = collect_edges_x(rect.top_left(), rect.top_right());
        let bottom = collect_edges_x(rect.bottom_left(), rect.bottom_right());
        let left = collect_edges_y(rect.bottom_left(), rect.top_left());
        let right = collect_edges_y(rect.bottom_right(), rect.top_right());

        RectEdges {
            rect,
            coordinate_outline: top.union(&bottom)
                .chain(left.iter())
                .chain(right.iter())
                .cloned()
                .collect()
        }
    }
}

fn collect_edges_x(src: Vec2, dst: Vec2) -> HashSet<(usize, usize)>{
    let mut coords = HashSet::new();
    let is_ngt_direction = (dst.signed().x - src.signed().x) < 0;
    let num_steps = dst.x.abs_diff(src.x);

    if dst.x > 0 {
        for i in 0..num_steps {
            let (x, y) = if is_ngt_direction {
                (dst.x + i, dst.y)
            } else {
                (src.x + i, src.y)
            };

            coords.insert((x, y));
        }
    }

    coords
}

fn collect_edges_y(src: Vec2, dst: Vec2) -> HashSet<(usize, usize)>{

    let mut coords = HashSet::new();
    let is_ngt_direction = (dst.signed().y - src.signed().y) < 0;
    let num_steps = dst.y.abs_diff(src.y);

    if dst.y > 0 {
        for i in 0..num_steps + 1 {
            let (x, y) = if is_ngt_direction {
                (dst.x, dst.y+i)
            } else {
                (src.x, src.y+i)
            };

            coords.insert((x, y));
        }
    }

    coords
}
