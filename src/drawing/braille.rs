use std::collections::{BTreeMap, BTreeSet};

use cursive::Vec2;

use crate::{config::Symbols, editor::buffer::Buffer};

use super::{junctions::is_arrow_tip, DrawingPlan};

const BRAILLE_BASE: u32 = 0x2800;
const BRAILLE_END: u32 = 0x28ff;
const DOTS: [[u8; 4]; 2] = [[0, 1, 2, 6], [3, 4, 5, 7]];

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) struct BrailleMask(u8);

impl BrailleMask {
    pub(super) fn decode(character: char) -> Option<Self> {
        let code = character as u32;
        if (BRAILLE_BASE..=BRAILLE_END).contains(&code) {
            Some(Self((code - BRAILLE_BASE) as u8))
        } else {
            None
        }
    }

    fn encode(self) -> char {
        char::from_u32(BRAILLE_BASE + u32::from(self.0))
            .expect("a Braille mask is always a valid Unicode scalar")
    }

    fn with_dot(self, x: usize, y: usize) -> Self {
        Self(self.0 | (1 << DOTS[x][y]))
    }

    fn merged(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    fn has_bit(self, bit: u8) -> bool {
        self.0 & (1 << bit) != 0
    }
}

#[derive(Clone, Default)]
struct BrailleRaster {
    cells: BTreeMap<(usize, usize), BrailleMask>,
}

impl BrailleRaster {
    fn plot(&mut self, x: isize, y: isize) {
        let (Ok(x), Ok(y)) = (usize::try_from(x), usize::try_from(y)) else {
            return;
        };
        let cell = self.cells.entry((x / 2, y / 4)).or_default();
        *cell = cell.with_dot(x % 2, y % 4);
    }

    fn segment(&mut self, (mut x0, mut y0): (isize, isize), (x1, y1): (isize, isize)) {
        let dx = (x1 - x0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let dy = -(y1 - y0).abs();
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut error = dx + dy;

        loop {
            self.plot(x0, y0);
            if x0 == x1 && y0 == y1 {
                break;
            }
            let twice_error = error * 2;
            if twice_error >= dy {
                error += dy;
                x0 += sx;
            }
            if twice_error <= dx {
                error += dx;
                y0 += sy;
            }
        }
    }

    fn polyline(&mut self, points: &[Vec2]) {
        if let Some(&point) = points.first() {
            let (x, y) = raster_point(point);
            self.plot(x, y);
        }
        for edge in points.windows(2) {
            self.segment(raster_point(edge[0]), raster_point(edge[1]));
        }
    }

    fn mask_at(&self, position: Vec2) -> Option<BrailleMask> {
        self.cells.get(&(position.x, position.y)).copied()
    }
}

pub(super) fn render(buffer: &mut Buffer, plan: &DrawingPlan, symbols: &Symbols) {
    let mut raster = BrailleRaster::default();
    raster.polyline(&plan.points);

    let mut connectors = BTreeSet::from([plan.connection_origin.pair()]);
    connectors.extend(
        raster
            .cells
            .keys()
            .copied()
            .filter(|&(x, y)| is_arrow_tip(buffer.get_char_at(Vec2::new(x, y)), symbols)),
    );

    let initial_raster = raster.clone();
    for position in connectors.into_iter().map(Vec2::from) {
        for target in surrounding_targets(buffer, &initial_raster, position) {
            raster.segment(raster_point(position), target);
        }
    }

    let composed = raster
        .cells
        .into_iter()
        .filter_map(|((x, y), overlay)| {
            let position = Vec2::new(x, y);
            let current = buffer.get_char_at(position);
            let mask = if current.is_whitespace() || is_arrow_tip(current, symbols) {
                overlay
            } else if let Some(existing) = BrailleMask::decode(current) {
                existing.merged(overlay)
            } else {
                return None;
            };
            Some((position, mask.encode()))
        })
        .collect::<Vec<_>>();

    for (position, character) in composed {
        buffer.setv(true, position, character, symbols);
    }
}

fn surrounding_targets(
    buffer: &Buffer,
    raster: &BrailleRaster,
    position: Vec2,
) -> Vec<(isize, isize)> {
    let origin = raster_point(position);
    let mut targets = Vec::new();
    let mut inspect = |neighbor: Vec2, neighbor_bits: &[u8], local_targets: &[(isize, isize)]| {
        let Some(mask) = combined_mask(buffer, raster, neighbor) else {
            return;
        };
        for (&bit, &target) in neighbor_bits.iter().zip(local_targets) {
            if mask.has_bit(bit) {
                targets.push((origin.0 + target.0, origin.1 + target.1));
            }
        }
    };

    if position.x > 0 {
        inspect(
            Vec2::new(position.x - 1, position.y),
            &[3, 4, 5, 7],
            &[(0, 0), (0, 1), (0, 2), (0, 3)],
        );
    }
    inspect(
        Vec2::new(position.x + 1, position.y),
        &[0, 1, 2, 6],
        &[(1, 0), (1, 1), (1, 2), (1, 3)],
    );
    if position.y > 0 {
        inspect(
            Vec2::new(position.x, position.y - 1),
            &[6, 7],
            &[(0, 0), (1, 0)],
        );
    }
    inspect(
        Vec2::new(position.x, position.y + 1),
        &[0, 3],
        &[(0, 3), (1, 3)],
    );
    if position.x > 0 && position.y > 0 {
        inspect(Vec2::new(position.x - 1, position.y - 1), &[7], &[(0, 0)]);
    }
    if position.y > 0 {
        inspect(Vec2::new(position.x + 1, position.y - 1), &[6], &[(1, 0)]);
    }
    if position.x > 0 {
        inspect(Vec2::new(position.x - 1, position.y + 1), &[3], &[(0, 3)]);
    }
    inspect(Vec2::new(position.x + 1, position.y + 1), &[0], &[(1, 3)]);

    targets
}

fn combined_mask(buffer: &Buffer, raster: &BrailleRaster, position: Vec2) -> Option<BrailleMask> {
    match (
        BrailleMask::decode(buffer.get_char_at(position)),
        raster.mask_at(position),
    ) {
        (Some(existing), Some(overlay)) => Some(existing.merged(overlay)),
        (Some(existing), None) => Some(existing),
        (None, Some(overlay)) => Some(overlay),
        (None, None) => None,
    }
}

fn raster_point(point: Vec2) -> (isize, isize) {
    (point.x as isize * 2, point.y as isize * 4)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mask_with_bit(bit: u8) -> char {
        BrailleMask(1 << bit).encode()
    }

    #[test]
    fn every_mask_round_trips_through_unicode() {
        for value in 0..=u8::MAX {
            let mask = BrailleMask(value);
            assert_eq!(BrailleMask::decode(mask.encode()), Some(mask));
        }
    }

    #[test]
    fn crossing_segments_merge_dots_before_composition() {
        let mut raster = BrailleRaster::default();
        raster.segment((0, 0), (2, 4));
        let before = raster.mask_at(Vec2::new(0, 0)).unwrap();
        raster.segment((0, 4), (2, 0));
        let after = raster.mask_at(Vec2::new(0, 0)).unwrap();
        assert_eq!(after.merged(before), after);
        assert_ne!(after, before);
    }

    #[test]
    fn dot_coordinates_follow_the_unicode_braille_layout() {
        let expected = [
            ((0, 0), '\u{2801}'),
            ((0, 1), '\u{2802}'),
            ((0, 2), '\u{2804}'),
            ((0, 3), '\u{2840}'),
            ((1, 0), '\u{2808}'),
            ((1, 1), '\u{2810}'),
            ((1, 2), '\u{2820}'),
            ((1, 3), '\u{2880}'),
        ];
        for ((x, y), character) in expected {
            assert_eq!(BrailleMask::default().with_dot(x, y).encode(), character);
        }
    }

    #[test]
    fn diagonal_to_horizontal_polyline_shares_a_corner_dot() {
        let mut raster = BrailleRaster::default();
        raster.polyline(&[Vec2::new(0, 0), Vec2::new(1, 1), Vec2::new(3, 1)]);

        let corner = raster.mask_at(Vec2::new(1, 1)).unwrap();
        assert!(corner.has_bit(0));
        assert!(corner.has_bit(3));
    }

    #[test]
    fn connector_reads_cardinal_and_diagonal_neighbor_edges() {
        let symbols = Symbols::default();
        let position = Vec2::new(1, 1);
        let mut buffer = Buffer::default();
        for (neighbor, character) in [
            (Vec2::new(0, 1), mask_with_bit(5)),
            (Vec2::new(2, 1), mask_with_bit(1)),
            (Vec2::new(1, 0), mask_with_bit(7)),
            (Vec2::new(1, 2), mask_with_bit(0)),
            (Vec2::new(2, 2), mask_with_bit(0)),
        ] {
            buffer.setv(true, neighbor, character, &symbols);
        }
        let mut raster = BrailleRaster::default();
        raster.plot(2, 4);

        let snapshot = raster.clone();
        for target in surrounding_targets(&buffer, &snapshot, position) {
            raster.segment(raster_point(position), target);
        }

        let connector = raster.mask_at(position).unwrap();
        for bit in [2, 4, 3, 6, 7] {
            assert!(connector.has_bit(bit), "missing connector dot {bit}");
        }
    }

    #[test]
    fn composition_replaces_arrowheads_but_preserves_text() {
        let symbols = Symbols::default();
        let arrow = Vec2::new(1, 0);
        let text = Vec2::new(2, 0);
        let mut buffer = Buffer::default();
        buffer.setv(true, arrow, symbols.e, &symbols);
        buffer.setv(true, text, 'x', &symbols);
        buffer.flush_edits();
        let plan = DrawingPlan {
            kind: super::super::ShapeKind::Line,
            points: vec![Vec2::new(0, 0), Vec2::new(3, 0)],
            connection_origin: Vec2::new(0, 0),
            cursor: Vec2::new(3, 0),
            degenerate_box: false,
        };

        render(&mut buffer, &plan, &symbols);

        assert!(BrailleMask::decode(buffer.get_char_at(arrow)).is_some());
        assert_eq!(buffer.get_char_at(text), 'x');
    }
}
