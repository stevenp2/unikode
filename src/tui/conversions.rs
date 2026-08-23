use cursive::{Rect, Vec2};

use crate::core::{Point, Region};

impl From<Vec2> for Point {
    fn from(value: Vec2) -> Self {
        Self::new(value.x, value.y)
    }
}

impl From<Point> for Vec2 {
    fn from(value: Point) -> Self {
        Self::new(value.x, value.y)
    }
}

impl From<Region> for Rect {
    fn from(value: Region) -> Self {
        Self::from_corners(Vec2::from(value.min), Vec2::from(value.max))
    }
}
