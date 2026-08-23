use std::collections::HashSet;

use cursive::Rect;

/// Coordinates on the inclusive outline of a rectangle.
pub struct RectEdges {
    pub coordinate_outline: HashSet<(usize, usize)>,
}

impl RectEdges {
    pub fn new(rect: Rect) -> Self {
        let mut coordinate_outline = HashSet::new();
        let left = rect.left();
        let right = rect.right();
        let top = rect.top();
        let bottom = rect.bottom();

        for x in left..=right {
            coordinate_outline.insert((x, top));
            coordinate_outline.insert((x, bottom));
        }
        for y in top..=bottom {
            coordinate_outline.insert((left, y));
            coordinate_outline.insert((right, y));
        }

        Self { coordinate_outline }
    }
}

#[cfg(test)]
mod tests {
    use cursive::{Rect, Vec2};

    use super::RectEdges;

    #[test]
    fn outline_includes_edges_at_the_canvas_origin() {
        let edges = RectEdges::new(Rect::from_corners(Vec2::new(0, 0), Vec2::new(2, 1)));
        let expected = [(0, 0), (1, 0), (2, 0), (0, 1), (1, 1), (2, 1)];
        assert!(expected
            .iter()
            .all(|point| edges.coordinate_outline.contains(point)));
    }
}
