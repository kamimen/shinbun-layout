use crate::model::{Paper, Placement};

pub fn in_bounds(paper: &Paper, at: &Placement) -> bool {
    at.w >= 1
        && at.span >= 1
        && at.x >= 1
        && at.y >= 1
        && at.x as u16 + at.w as u16 - 1 <= paper.lines as u16
        && at.y as u16 + at.span as u16 - 1 <= paper.dan as u16
}

fn line_range(at: &Placement) -> (u16, u16) {
    (at.x as u16, at.x as u16 + at.w as u16)
}

fn col_range(at: &Placement) -> (u16, u16) {
    (at.y as u16, at.y as u16 + at.span as u16)
}

pub fn overlaps(a: &Placement, b: &Placement) -> bool {
    let (al, ar) = line_range(a);
    let (bl, br) = line_range(b);
    let (ac, ad) = col_range(a);
    let (bc, bd) = col_range(b);
    al < br && bl < ar && ac < bd && bc < ad
}

pub fn area(at: &Placement) -> u32 {
    at.w as u32 * at.span as u32
}

/// Which grid cells are taken. One `u64` per column holds its lines, so a paper can have up to 64 lines.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct Occupancy {
    cols: [u64; 16],
}

impl Occupancy {
    fn mask(at: &Placement) -> u64 {
        ((1u64 << at.w) - 1) << (at.x - 1)
    }

    fn columns(at: &Placement) -> std::ops::Range<usize> {
        (at.y as usize - 1)..(at.y as usize - 1 + at.span as usize)
    }

    pub fn is_free(&self, at: &Placement) -> bool {
        let mask = Self::mask(at);
        Self::columns(at).all(|c| self.cols[c] & mask == 0)
    }

    pub fn occupy(&mut self, at: &Placement) {
        let mask = Self::mask(at);
        for c in Self::columns(at) {
            self.cols[c] |= mask;
        }
    }

    pub fn cells(&self) -> u32 {
        self.cols.iter().map(|c| c.count_ones()).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(x: u8, w: u8, y: u8, span: u8) -> Placement {
        Placement { x, w, y, span }
    }

    #[test]
    fn touching_rectangles_do_not_overlap() {
        assert!(!overlaps(&at(1, 5, 1, 3), &at(6, 5, 1, 3)));
        assert!(!overlaps(&at(1, 5, 1, 3), &at(1, 5, 4, 3)));
    }

    #[test]
    fn crossing_rectangles_overlap() {
        assert!(overlaps(&at(1, 5, 1, 3), &at(5, 5, 3, 3)));
        assert!(overlaps(&at(1, 9, 1, 9), &at(3, 1, 4, 1)));
    }

    #[test]
    fn occupancy_agrees_with_overlaps() {
        let a = at(3, 4, 2, 3);
        let mut occ = Occupancy::default();
        occ.occupy(&a);
        for x in 1..=12u8 {
            for y in 1..=8u8 {
                for (w, span) in [(1, 1), (3, 2), (6, 4)] {
                    let b = at(x, w, y, span);
                    assert_eq!(!occ.is_free(&b), overlaps(&a, &b), "{b:?}");
                }
            }
        }
    }

    #[test]
    fn counts_cells() {
        let mut occ = Occupancy::default();
        occ.occupy(&at(1, 5, 1, 3));
        occ.occupy(&at(6, 2, 1, 1));
        assert_eq!(occ.cells(), 17);
    }

    #[test]
    fn bounds() {
        let paper = Paper {
            writing: Default::default(),
            dan: 10,
            lines: 50,
            chars: 10,
        };
        assert!(in_bounds(&paper, &at(1, 50, 1, 10)));
        assert!(!in_bounds(&paper, &at(2, 50, 1, 10)));
        assert!(!in_bounds(&paper, &at(1, 50, 2, 10)));
        assert!(!in_bounds(&paper, &at(0, 5, 1, 1)));
        assert!(!in_bounds(&paper, &at(1, 0, 1, 1)));
    }
}
