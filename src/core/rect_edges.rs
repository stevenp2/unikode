use std::collections::HashSet;

use super::Region;

/// Coordinates on the inclusive outline of a rectangle.
pub struct RectEdges {
    pub coordinate_outline: HashSet<(usize, usize)>,
}

impl RectEdges {
    pub fn new(region: Region) -> Self {
        let mut coordinate_outline = HashSet::new();
        let left = region.min.x;
        let right = region.max.x;
        let top = region.min.y;
        let bottom = region.max.y;

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
    use super::RectEdges;
    use crate::core::{Point, Region};

    #[test]
    fn outline_includes_edges_at_the_canvas_origin() {
        let edges = RectEdges::new(Region::new(Point::new(0, 0), Point::new(2, 1)));
        let expected = [(0, 0), (1, 0), (2, 0), (0, 1), (1, 1), (2, 1)];
        assert!(expected
            .iter()
            .all(|point| edges.coordinate_outline.contains(point)));
    }
}
