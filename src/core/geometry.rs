/// A zero-based position on the diagram canvas.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct Point {
    pub x: usize,
    pub y: usize,
}

impl Point {
    pub const fn new(x: usize, y: usize) -> Self {
        Self { x, y }
    }

    pub fn saturating_offset(self, x: isize, y: isize) -> Self {
        Self {
            x: self.x.saturating_add_signed(x),
            y: self.y.saturating_add_signed(y),
        }
    }
}

/// An inclusive, normalized rectangular canvas region.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Region {
    pub min: Point,
    pub max: Point,
}

impl Region {
    pub fn new(a: Point, b: Point) -> Self {
        Self {
            min: Point::new(a.x.min(b.x), a.y.min(b.y)),
            max: Point::new(a.x.max(b.x), a.y.max(b.y)),
        }
    }

    pub fn contains(self, point: Point) -> bool {
        (self.min.x..=self.max.x).contains(&point.x) && (self.min.y..=self.max.y).contains(&point.y)
    }

    pub fn width(self) -> usize {
        self.max.x - self.min.x + 1
    }

    pub fn height(self) -> usize {
        self.max.y - self.min.y + 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn region_normalizes_reversed_corners() {
        let region = Region::new(Point::new(4, 7), Point::new(1, 2));
        assert_eq!(region.min, Point::new(1, 2));
        assert_eq!(region.max, Point::new(4, 7));
        assert_eq!((region.width(), region.height()), (4, 6));
    }

    #[test]
    fn offset_saturates_at_canvas_origin() {
        assert_eq!(Point::new(1, 1).saturating_offset(-5, -2), Point::new(0, 0));
    }
}
