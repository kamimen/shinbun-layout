use crate::geometry::{in_bounds, overlaps};
use crate::model::{Headline, Layout, Lookup, MeasurementTable, Paper, Placement, Problem, Role};
use serde::{Deserialize, Serialize};

/// Layouts that newspaper makeup avoids (SPEC §10.9 of shinbun-css). They are warnings, not errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Pattern {
    /// シリモチ: a headline or photo that spans two or more columns in the bottom column
    Shirimochi,
    /// 見出しの並列: side by side headlines of the same size over the same columns
    Heiretsu,
    /// 見出しの直列 (エントツ): a headline directly under another headline
    Entotsu,
    /// ハラキリ: a column boundary that cuts across the whole paper
    Harakiri,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Rules {
    pub shirimochi: bool,
    pub heiretsu: bool,
    pub entotsu: bool,
    pub harakiri: bool,
}

impl Default for Rules {
    fn default() -> Self {
        Self {
            shirimochi: true,
            heiretsu: true,
            entotsu: true,
            harakiri: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Violation {
    UnknownItem { id: String },
    OutOfBounds { id: String },
    Overlap { a: String, b: String },
    FixedMoved { id: String },
    DoesNotFit { id: String, needs: u8, has: u8 },
    CannotFitAtSpan { id: String, span: u8 },
    Unmeasured { id: String, span: u8 },
    Forbidden { pattern: Pattern, ids: Vec<String> },
}

impl Violation {
    /// Hard violations make a layout unusable; the others are style warnings.
    pub fn is_hard(&self) -> bool {
        !matches!(self, Violation::Forbidden { .. })
    }
}

struct Placed<'a> {
    id: &'a str,
    role: Role,
    headline: Option<Headline>,
    at: Placement,
}

fn resolve<'a>(problem: &'a Problem, layout: &'a Layout, out: &mut Vec<Violation>) -> Vec<Placed<'a>> {
    let mut placed = Vec::new();
    for p in &layout.placements {
        match problem.item(&p.id) {
            Some(it) => placed.push(Placed {
                id: &p.id,
                role: it.role,
                headline: it.headline,
                at: p.at,
            }),
            None => out.push(Violation::UnknownItem { id: p.id.clone() }),
        }
    }
    placed
}

/// Checks a layout against the hard constraints and the style rules. Independent of any solver.
pub fn validate(
    problem: &Problem,
    table: &MeasurementTable,
    layout: &Layout,
    rules: &Rules,
) -> Vec<Violation> {
    let mut out = Vec::new();
    let placed = resolve(problem, layout, &mut out);

    for p in &placed {
        if !in_bounds(&problem.paper, &p.at) {
            out.push(Violation::OutOfBounds { id: p.id.to_owned() });
        }
        let item = problem.item(p.id).expect("resolved above");
        if let Some(fixed) = &item.fixed {
            if *fixed != p.at {
                out.push(Violation::FixedMoved { id: p.id.to_owned() });
            }
        } else {
            match table.get(p.id, p.at.span) {
                Lookup::Needs(n) if p.at.w < n => out.push(Violation::DoesNotFit {
                    id: p.id.to_owned(),
                    needs: n,
                    has: p.at.w,
                }),
                Lookup::Needs(_) => {}
                Lookup::CannotFit => out.push(Violation::CannotFitAtSpan {
                    id: p.id.to_owned(),
                    span: p.at.span,
                }),
                Lookup::Missing => out.push(Violation::Unmeasured {
                    id: p.id.to_owned(),
                    span: p.at.span,
                }),
            }
        }
    }
    for (i, a) in placed.iter().enumerate() {
        for b in &placed[i + 1..] {
            if overlaps(&a.at, &b.at) {
                out.push(Violation::Overlap {
                    a: a.id.to_owned(),
                    b: b.id.to_owned(),
                });
            }
        }
    }
    out.extend(forbidden_patterns(&problem.paper, &layout_view(&placed), rules));
    out
}

pub struct View<'a> {
    pub id: &'a str,
    pub role: Role,
    pub headline: Option<Headline>,
    pub at: Placement,
}

fn layout_view<'a>(placed: &[Placed<'a>]) -> Vec<View<'a>> {
    placed
        .iter()
        .map(|p| View {
            id: p.id,
            role: p.role,
            headline: p.headline,
            at: p.at,
        })
        .collect()
}

fn is_story(role: Role) -> bool {
    role == Role::Article
}

pub fn forbidden_patterns(paper: &Paper, items: &[View<'_>], rules: &Rules) -> Vec<Violation> {
    let mut out = Vec::new();
    let flag = |pattern, ids: &[&str]| Violation::Forbidden {
        pattern,
        ids: ids.iter().map(|s| (*s).to_owned()).collect(),
    };

    if rules.shirimochi {
        for it in items {
            let headed = matches!(it.role, Role::Article | Role::Photo);
            if headed && it.at.span >= 2 && it.at.y + it.at.span - 1 == paper.dan {
                out.push(flag(Pattern::Shirimochi, &[it.id]));
            }
        }
    }

    for (i, a) in items.iter().enumerate() {
        for b in &items[i + 1..] {
            if !(is_story(a.role) && is_story(b.role)) {
                continue;
            }
            let side_by_side = a.at.x + a.at.w == b.at.x || b.at.x + b.at.w == a.at.x;
            if rules.heiretsu
                && side_by_side
                && a.at.y == b.at.y
                && a.at.span == b.at.span
                && a.at.span >= 2
                && a.headline.is_some()
                && a.headline == b.headline
            {
                out.push(flag(Pattern::Heiretsu, &[a.id, b.id]));
            }
            let stacked = a.at.y + a.at.span == b.at.y || b.at.y + b.at.span == a.at.y;
            if rules.entotsu && stacked && a.at.x == b.at.x && a.at.span >= 2 && b.at.span >= 2 {
                out.push(flag(Pattern::Entotsu, &[a.id, b.id]));
            }
        }
    }

    if rules.harakiri && paper.dan >= 4 && items.len() >= 2 {
        for k in 1..=(paper.dan - 3) {
            let crossed = items
                .iter()
                .any(|it| it.at.y <= k && it.at.y + it.at.span - 1 > k);
            let above = items.iter().any(|it| it.at.y + it.at.span - 1 <= k);
            let below = items.iter().any(|it| it.at.y > k);
            if !crossed && above && below {
                out.push(Violation::Forbidden {
                    pattern: Pattern::Harakiri,
                    ids: Vec::new(),
                });
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Item, Placed as P};

    fn paper() -> Paper {
        Paper {
            writing: Default::default(),
            dan: 10,
            lines: 50,
            chars: 10,
        }
    }

    fn item(id: &str, role: Role) -> Item {
        Item {
            id: id.into(),
            role,
            priority: 5,
            chars: 100,
            headline: Some(Headline::M),
            lead: false,
            min_span: None,
            max_span: None,
            fixed: None,
        }
    }

    fn at(x: u8, w: u8, y: u8, span: u8) -> Placement {
        Placement { x, w, y, span }
    }

    fn layout(list: &[(&str, Placement)]) -> Layout {
        Layout {
            placements: list
                .iter()
                .map(|(id, at)| P {
                    id: (*id).into(),
                    at: *at,
                })
                .collect(),
        }
    }

    fn problem(items: Vec<Item>) -> Problem {
        Problem {
            paper: paper(),
            items,
        }
    }

    fn table(entries: &[(&str, u8, Option<u8>)]) -> MeasurementTable {
        let mut t = MeasurementTable::default();
        for (id, span, lines) in entries {
            t.insert(id, *span, *lines);
        }
        t
    }

    fn hard(v: Vec<Violation>) -> Vec<Violation> {
        v.into_iter().filter(Violation::is_hard).collect()
    }

    #[test]
    fn a_clean_layout_has_no_hard_violations() {
        let p = problem(vec![item("a", Role::Article), item("b", Role::Article)]);
        let t = table(&[("a", 3, Some(10)), ("b", 3, Some(10))]);
        let l = layout(&[("a", at(1, 10, 1, 3)), ("b", at(11, 10, 1, 3))]);
        assert!(hard(validate(&p, &t, &l, &Rules::default())).is_empty());
    }

    #[test]
    fn out_of_bounds_overlap_and_fit_are_reported() {
        let p = problem(vec![item("a", Role::Article), item("b", Role::Article)]);
        let t = table(&[("a", 3, Some(10)), ("b", 3, Some(10))]);
        let l = layout(&[("a", at(45, 10, 1, 3)), ("b", at(1, 6, 1, 3))]);
        let v = hard(validate(&p, &t, &l, &Rules::default()));
        assert!(v.contains(&Violation::OutOfBounds { id: "a".into() }));
        assert!(v.contains(&Violation::DoesNotFit {
            id: "b".into(),
            needs: 10,
            has: 6
        }));
        let l = layout(&[("a", at(1, 10, 1, 3)), ("b", at(5, 10, 2, 3))]);
        assert!(
            hard(validate(&p, &t, &l, &Rules::default())).contains(&Violation::Overlap {
                a: "a".into(),
                b: "b".into()
            })
        );
    }

    #[test]
    fn unmeasured_and_unfittable_shapes_are_reported() {
        let p = problem(vec![item("a", Role::Article)]);
        let l = layout(&[("a", at(1, 10, 1, 3))]);
        assert!(
            hard(validate(&p, &table(&[]), &l, &Rules::default())).contains(&Violation::Unmeasured {
                id: "a".into(),
                span: 3
            })
        );
        assert!(
            hard(validate(&p, &table(&[("a", 3, None)]), &l, &Rules::default())).contains(
                &Violation::CannotFitAtSpan {
                    id: "a".into(),
                    span: 3
                }
            )
        );
    }

    #[test]
    fn a_fixed_item_must_not_move() {
        let mut nameplate = item("n", Role::Nameplate);
        nameplate.fixed = Some(at(1, 8, 1, 3));
        let p = problem(vec![nameplate]);
        let l = layout(&[("n", at(2, 8, 1, 3))]);
        assert!(hard(validate(&p, &table(&[]), &l, &Rules::default()))
            .contains(&Violation::FixedMoved { id: "n".into() }));
    }

    #[test]
    fn unknown_items_are_reported() {
        let p = problem(vec![]);
        let l = layout(&[("ghost", at(1, 1, 1, 1))]);
        assert_eq!(
            validate(&p, &table(&[]), &l, &Rules::default()),
            vec![Violation::UnknownItem { id: "ghost".into() }]
        );
    }

    fn patterns(v: &[Violation]) -> Vec<Pattern> {
        v.iter()
            .filter_map(|x| {
                if let Violation::Forbidden { pattern, .. } = x {
                    Some(*pattern)
                } else {
                    None
                }
            })
            .collect()
    }

    fn two(a: Placement, b: Placement) -> Vec<Violation> {
        let p = problem(vec![item("a", Role::Article), item("b", Role::Article)]);
        let t = table(&[("a", a.span, Some(a.w)), ("b", b.span, Some(b.w))]);
        validate(&p, &t, &layout(&[("a", a), ("b", b)]), &Rules::default())
    }

    #[test]
    fn shirimochi_is_a_multi_column_story_in_the_bottom_column() {
        let v = two(at(1, 10, 1, 3), at(11, 10, 8, 3));
        assert!(patterns(&v).contains(&Pattern::Shirimochi));
        let v = two(at(1, 10, 1, 3), at(11, 10, 9, 1));
        assert!(!patterns(&v).contains(&Pattern::Shirimochi));
    }

    #[test]
    fn heiretsu_needs_same_columns_and_same_headline_size() {
        let v = two(at(1, 10, 1, 3), at(11, 10, 1, 3));
        assert!(patterns(&v).contains(&Pattern::Heiretsu));
        let v = two(at(1, 10, 1, 1), at(11, 10, 1, 1));
        assert!(
            !patterns(&v).contains(&Pattern::Heiretsu),
            "one-column headlines side by side are fine"
        );
        let v = two(at(1, 10, 1, 3), at(11, 10, 4, 3));
        assert!(!patterns(&v).contains(&Pattern::Heiretsu));
    }

    #[test]
    fn entotsu_is_a_headline_directly_below_a_headline() {
        let v = two(at(1, 10, 1, 3), at(1, 10, 4, 3));
        assert!(patterns(&v).contains(&Pattern::Entotsu));
        let v = two(at(1, 10, 1, 3), at(5, 10, 4, 3));
        assert!(!patterns(&v).contains(&Pattern::Entotsu));
    }

    #[test]
    fn harakiri_is_a_cut_across_the_whole_paper_except_near_the_bottom() {
        let v = two(at(1, 50, 1, 3), at(1, 50, 4, 3));
        assert!(patterns(&v).contains(&Pattern::Harakiri));
        let v = two(at(1, 50, 1, 8), at(1, 50, 9, 2));
        assert!(
            !patterns(&v).contains(&Pattern::Harakiri),
            "a cut in the bottom two columns is allowed"
        );
        let v = two(at(1, 30, 1, 3), at(1, 50, 3, 3));
        assert!(!patterns(&v).contains(&Pattern::Harakiri));
    }

    #[test]
    fn rules_can_be_switched_off() {
        let p = problem(vec![item("a", Role::Article), item("b", Role::Article)]);
        let t = table(&[("a", 3, Some(10)), ("b", 3, Some(10))]);
        let l = layout(&[("a", at(1, 10, 1, 3)), ("b", at(11, 10, 1, 3))]);
        let off = Rules {
            heiretsu: false,
            ..Rules::default()
        };
        assert!(!patterns(&validate(&p, &t, &l, &off)).contains(&Pattern::Heiretsu));
    }
}
