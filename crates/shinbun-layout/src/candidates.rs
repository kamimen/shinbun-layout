use crate::model::{Item, Lookup, MeasureQuery, MeasurementTable, Paper, Placement, Problem, Role, Writing};
use crate::scoring::LINE_HEIGHT;
use std::collections::BTreeSet;
use std::ops::RangeInclusive;

/// Widths tried beyond the smallest one that fits, so a story can be given more room to fill the page.
pub const DEFAULT_SLACK: [u8; 5] = [0, 1, 2, 3, 5];

/// Widths tried for things without text (photos, ads, headlines), whose size is a matter of taste.
const FREE_WIDTHS: [u8; 16] = [1, 2, 3, 4, 5, 6, 8, 10, 12, 14, 16, 20, 24, 30, 40, 50];

pub fn span_range(paper: &Paper, item: &Item) -> RangeInclusive<u8> {
    let lo = item.min_span.unwrap_or(1).max(1);
    let hi = item.max_span.unwrap_or(paper.dan).min(paper.dan);
    lo..=hi
}

/// Step 1: what the host has to measure. Fits do not depend on where an item sits, so one query per column span is enough.
pub fn required_measurements(problem: &Problem) -> Vec<MeasureQuery> {
    problem
        .items
        .iter()
        .filter(|it| it.fixed.is_none())
        .flat_map(|it| {
            span_range(&problem.paper, it).map(|span| MeasureQuery {
                id: it.id.clone(),
                span,
            })
        })
        .collect()
}

/// A size an item can take: `span` columns and `w` lines, where `w` is at least what the text needs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Shape {
    pub span: u8,
    pub w: u8,
    pub needs: u8,
}

/// How far the frame of a picture may differ from its ratio, as a fraction of the ideal length.
const RATIO_TOLERANCE: f64 = 0.12;

/// Widths that keep a picture's ratio at `span` columns. The caption takes one line of the frame.
fn picture_widths(item: &Item, paper: &Paper, ratio: f64, span: u8, needs: u8) -> BTreeSet<u8> {
    let caption = u8::from(item.caption);
    let height_em = paper.length_em(span);
    let ideal = match paper.writing {
        Writing::Vertical => ratio * height_em / LINE_HEIGHT,
        Writing::Horizontal => height_em / (ratio * LINE_HEIGHT),
    };
    let max = paper.lines.saturating_sub(caption);
    let near: BTreeSet<u8> = [ideal.floor(), ideal.ceil()]
        .into_iter()
        .filter(|l| *l >= 1.0 && *l <= max as f64)
        .map(|l| l as u8)
        .filter(|l| ((*l as f64 - ideal) / ideal).abs() <= RATIO_TOLERANCE)
        .map(|l| l + caption)
        .filter(|w| *w >= needs)
        .collect();
    if !near.is_empty() || ideal < 1.0 || ideal > max as f64 {
        return near;
    }
    BTreeSet::from([(ideal.round() as u8).max(1) + caption])
}

pub fn shapes(item: &Item, paper: &Paper, table: &MeasurementTable, slack: &[u8]) -> Vec<Shape> {
    let mut out = Vec::new();
    for span in span_range(paper, item) {
        let Lookup::Needs(needs) = table.get(&item.id, span) else {
            continue;
        };
        let widths: BTreeSet<u8> = if let (Role::Photo, Some(ratio)) = (item.role, item.ratio) {
            picture_widths(item, paper, ratio, span, needs)
        } else if item.role.has_text() {
            slack
                .iter()
                .filter_map(|s| needs.checked_add(*s))
                .filter(|w| *w <= paper.lines)
                .collect()
        } else {
            FREE_WIDTHS
                .iter()
                .copied()
                .filter(|w| *w >= needs && *w <= paper.lines)
                .collect()
        };
        out.extend(widths.into_iter().map(|w| Shape { span, w, needs }));
    }
    out
}

/// Positions worth trying: against the paper edges or flush with another item, so no sliver is left behind.
pub fn anchors(paper: &Paper, placed: &[Placement], shape: &Shape) -> Vec<(u8, u8)> {
    let lines = paper.lines as i32;
    let dan = paper.dan as i32;
    let (w, span) = (shape.w as i32, shape.span as i32);

    let mut xs = BTreeSet::from([1, lines - w + 1]);
    let mut ys = BTreeSet::from([1, dan - span + 1]);
    for p in placed {
        xs.insert(p.x as i32 + p.w as i32);
        xs.insert(p.x as i32 - w);
        ys.insert(p.y as i32 + p.span as i32);
        ys.insert(p.y as i32 - span);
    }
    let xs: Vec<i32> = xs.into_iter().filter(|x| *x >= 1 && x + w - 1 <= lines).collect();
    let ys: Vec<i32> = ys
        .into_iter()
        .filter(|y| *y >= 1 && y + span - 1 <= dan)
        .collect();
    xs.iter()
        .flat_map(|x| ys.iter().map(move |y| (*x as u8, *y as u8)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn paper() -> Paper {
        Paper {
            writing: Writing::Vertical,
            dan: 10,
            lines: 50,
            chars: 10,
            column_em: None,
            gap_em: None,
        }
    }

    fn item(id: &str) -> Item {
        Item {
            id: id.into(),
            role: Role::Article,
            priority: 5,
            chars: 500,
            headline: None,
            lead: false,
            ratio: None,
            caption: false,
            min_span: None,
            max_span: None,
            fixed: None,
        }
    }

    #[test]
    fn fixed_items_need_no_measurement() {
        let mut fixed = item("f");
        fixed.fixed = Some(Placement {
            x: 1,
            w: 8,
            y: 1,
            span: 3,
        });
        let mut limited = item("a");
        limited.min_span = Some(2);
        limited.max_span = Some(4);
        let p = Problem {
            paper: paper(),
            items: vec![fixed, limited],
        };
        let q = required_measurements(&p);
        assert_eq!(q.iter().map(|m| m.span).collect::<Vec<_>>(), vec![2, 3, 4]);
        assert!(q.iter().all(|m| m.id == "a"));
    }

    #[test]
    fn shapes_start_at_the_measured_minimum_and_stay_on_the_paper() {
        let mut t = MeasurementTable::default();
        t.insert("a", 3, Some(18));
        t.insert("a", 4, Some(14));
        t.insert("a", 5, None);
        let s = shapes(&item("a"), &paper(), &t, &[0, 2]);
        assert_eq!(
            s,
            vec![
                Shape {
                    span: 3,
                    w: 18,
                    needs: 18
                },
                Shape {
                    span: 3,
                    w: 20,
                    needs: 18
                },
                Shape {
                    span: 4,
                    w: 14,
                    needs: 14
                },
                Shape {
                    span: 4,
                    w: 16,
                    needs: 14
                }
            ]
        );
        t.insert("a", 6, Some(49));
        assert!(shapes(&item("a"), &paper(), &t, &[0, 5])
            .iter()
            .all(|s| s.w <= 50));
    }

    #[test]
    fn anchors_include_the_edges_and_neighbours() {
        let placed = [Placement {
            x: 1,
            w: 8,
            y: 1,
            span: 3,
        }];
        let a = anchors(
            &paper(),
            &placed,
            &Shape {
                span: 2,
                w: 10,
                needs: 10,
            },
        );
        assert!(a.contains(&(1, 1)));
        assert!(a.contains(&(9, 1)), "flush with the first item along the lines");
        assert!(a.contains(&(1, 4)), "flush below it along the columns");
        assert!(a.contains(&(41, 9)), "the far corner");
        assert!(a
            .iter()
            .all(|(x, y)| *x >= 1 && *x as u16 + 9 <= 50 && *y >= 1 && *y < 10));
    }

    fn picture(ratio: f64, caption: bool, writing: Writing) -> (Item, Paper) {
        let mut it = item("p");
        it.role = Role::Photo;
        it.ratio = Some(ratio);
        it.caption = caption;
        (it, Paper { writing, ..paper() })
    }

    fn lookup_all() -> MeasurementTable {
        let mut t = MeasurementTable::default();
        for span in 1..=10 {
            t.insert("p", span, Some(1));
        }
        t
    }

    #[test]
    fn a_wide_picture_gets_wide_frames_in_vertical_writing() {
        let (it, p) = picture(1.73, false, Writing::Vertical);
        for s in shapes(&it, &p, &lookup_all(), &DEFAULT_SLACK) {
            let width_em = s.w as f64 * LINE_HEIGHT;
            let height_em = p.length_em(s.span);
            let r = width_em / height_em;
            assert!(
                (r / 1.73 - 1.0).abs() < 0.2,
                "span {} w {} gives ratio {r}",
                s.span,
                s.w
            );
        }
    }

    #[test]
    fn horizontal_writing_swaps_the_axes() {
        let (it, p) = picture(1.73, false, Writing::Horizontal);
        let shapes = shapes(&it, &p, &lookup_all(), &DEFAULT_SLACK);
        assert!(!shapes.is_empty());
        for s in shapes {
            let r = p.length_em(s.span) / (s.w as f64 * LINE_HEIGHT);
            assert!(
                (r / 1.73 - 1.0).abs() < 0.2,
                "span {} w {} gives ratio {r}",
                s.span,
                s.w
            );
        }
    }

    #[test]
    fn the_caption_adds_one_line() {
        let (a, p) = picture(1.0, false, Writing::Vertical);
        let (b, _) = picture(1.0, true, Writing::Vertical);
        let t = lookup_all();
        let with: Vec<_> = shapes(&b, &p, &t, &DEFAULT_SLACK);
        let without: Vec<_> = shapes(&a, &p, &t, &DEFAULT_SLACK);
        assert!(!with.is_empty());
        for s in &with {
            assert!(
                without.iter().any(|o| o.span == s.span && o.w + 1 == s.w),
                "span {}",
                s.span
            );
        }
    }
}
