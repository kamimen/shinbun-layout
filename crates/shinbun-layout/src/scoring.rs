use crate::geometry::area;
use crate::model::{Item, Layout, Lookup, MeasurementTable, Paper, Placement, Problem, Role, Writing};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// How much each part of the score counts.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
#[non_exhaustive]
pub struct Weights {
    /// How close each frame is to the size its importance deserves.
    pub size: f64,
    /// How far each frame is from a sliver.
    pub shape: f64,
    /// How well each item sits where makeup would put it (dead corners, the diagonal).
    pub position: f64,
    /// How much of the paper is filled.
    pub coverage: f64,
    /// Whether more important stories get more room.
    pub hierarchy: f64,
    /// The cost of each style warning.
    pub forbidden: f64,
    /// The cost of leaving an item out, scaled by its importance.
    pub unplaced: f64,
}

impl Default for Weights {
    fn default() -> Self {
        Self {
            size: 3.0,
            shape: 2.0,
            position: 2.0,
            coverage: 6.0,
            hierarchy: 1.5,
            forbidden: 1.5,
            unplaced: 40.0,
        }
    }
}

/// The score of a layout, part by part. Higher is better; costs are negative.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ScoreBreakdown {
    /// See [`Weights::size`].
    pub size: f64,
    /// See [`Weights::shape`].
    pub shape: f64,
    /// See [`Weights::position`].
    pub position: f64,
    /// See [`Weights::coverage`].
    pub coverage: f64,
    /// See [`Weights::hierarchy`].
    pub hierarchy: f64,
    /// See [`Weights::forbidden`].
    pub forbidden: f64,
    /// See [`Weights::unplaced`].
    pub unplaced: f64,
    /// The sum of the parts.
    pub total: f64,
}

#[derive(Debug, Clone, Copy)]
pub struct ItemScore {
    pub size: f64,
    pub shape: f64,
    pub position: f64,
}

/// Dropping an important story must cost more than dropping several minor ones, hence the square.
pub fn unplaced_cost(item: &Item) -> f64 {
    (item.priority as f64 / 10.0).powi(2)
}

/// Line pitch over character size in shinbun-css (`--sb-lh`)
pub const LINE_HEIGHT: f64 = 1.55;

fn role_share(role: Role) -> f64 {
    match role {
        Role::Article => 1.0,
        Role::Photo => 0.7,
        Role::Ad => 0.5,
        Role::Box => 0.35,
        Role::Nameplate => 0.3,
        Role::Yoko => 0.25,
    }
}

fn emphasis(item: &Item) -> f64 {
    if item.lead {
        1.0
    } else {
        item.priority as f64 / 10.0
    }
}

pub struct Scorer {
    paper: Paper,
    pub weights: Weights,
    total_cells: f64,
    movable: f64,
    targets: HashMap<String, f64>,
}

impl Scorer {
    pub fn new(problem: &Problem, table: &MeasurementTable, weights: Weights) -> Self {
        let paper = problem.paper;
        let total_cells = paper.dan as f64 * paper.lines as f64;
        let fixed_cells: f64 = problem
            .items
            .iter()
            .filter_map(|i| i.fixed.as_ref())
            .map(|a| area(a) as f64)
            .sum();
        let free = (total_cells - fixed_cells).max(1.0);
        let movable_items: Vec<&Item> = problem.items.iter().filter(|i| i.fixed.is_none()).collect();
        let weight_sum: f64 = movable_items
            .iter()
            .map(|i| i.priority as f64 * role_share(i.role))
            .sum::<f64>()
            .max(1e-9);

        let mut targets = HashMap::new();
        for it in &movable_items {
            let min_area = (1..=paper.dan)
                .filter_map(|span| match table.get(&it.id, span) {
                    Lookup::Needs(n) => Some(span as f64 * n as f64),
                    _ => None,
                })
                .fold(f64::INFINITY, f64::min);
            let share = free * (it.priority as f64 * role_share(it.role)) / weight_sum;
            let target = match (it.role.has_text(), min_area.is_finite()) {
                // Text should fill its frame: stay close to what it needs, and let the lead have a little more
                (true, true) => share.clamp(min_area, min_area * (1.2 + 0.8 * emphasis(it))),
                (_, true) => share.max(min_area),
                _ => share,
            };
            targets.insert(it.id.clone(), target.min(free).max(1.0));
        }
        Self {
            paper,
            weights,
            total_cells,
            movable: movable_items.len() as f64,
            targets,
        }
    }

    /// Cells that count towards filling the page: area beyond a little over the target earns nothing,
    /// so a photo cannot crowd out the stories just to cover more paper.
    pub fn covered(&self, item: &Item, at: &Placement) -> f64 {
        (area(at) as f64).min(self.target_area(&item.id) * 1.25)
    }

    pub fn target_area(&self, id: &str) -> f64 {
        self.targets.get(id).copied().unwrap_or(1.0)
    }

    pub fn item_score(&self, item: &Item, at: &Placement) -> ItemScore {
        let target = self.target_area(&item.id);
        let size = 1.0 - ((area(at) as f64 - target).abs() / target).min(1.0);

        let aspect = self.aspect(at);
        let shape = match item.role {
            Role::Article | Role::Photo | Role::Box => 1.0 - ((aspect - 1.0) / 5.0).clamp(0.0, 1.0),
            Role::Yoko => ((self.wide(at) - 1.0) / 3.0).clamp(0.0, 1.0),
            Role::Ad => self.band(at),
            Role::Nameplate => match self.paper.writing {
                Writing::Vertical => {
                    let tall = 1.0 / self.wide(at);
                    if tall < 1.5 {
                        (tall / 1.5).clamp(0.0, 1.0)
                    } else {
                        (3.5 / tall).min(1.0)
                    }
                }
                Writing::Horizontal => {
                    if at.span == self.paper.dan {
                        1.0
                    } else {
                        0.3
                    }
                }
            },
        };

        ItemScore {
            size,
            shape,
            position: self.position(item, at),
        }
    }

    pub fn wide(&self, at: &Placement) -> f64 {
        let along_lines = at.w as f64 * LINE_HEIGHT;
        let along_columns = self.paper.length_em(at.span);
        match self.paper.writing {
            Writing::Vertical => along_lines / along_columns,
            Writing::Horizontal => along_columns / along_lines,
        }
    }

    fn band(&self, at: &Placement) -> f64 {
        let (across, full) = match self.paper.writing {
            Writing::Vertical => (at.w, self.paper.lines),
            Writing::Horizontal => (at.span, self.paper.dan),
        };
        if across == full {
            1.0
        } else if across == full / 2 || across == full.div_ceil(2) {
            0.8
        } else if self.wide(at) > 1.0 {
            0.4
        } else {
            0.0
        }
    }

    fn at_bottom(&self, at: &Placement) -> bool {
        match self.paper.writing {
            Writing::Vertical => at.y + at.span - 1 == self.paper.dan,
            Writing::Horizontal => at.x + at.w - 1 == self.paper.lines,
        }
    }

    /// Long side over short side, measured in character sizes (a line is `--sb-lh` characters wide).
    pub fn aspect(&self, at: &Placement) -> f64 {
        let along_lines = at.w as f64 * LINE_HEIGHT;
        let along_columns = self.paper.length_em(at.span);
        along_lines.max(along_columns) / along_lines.min(along_columns)
    }

    /// 0 at the corner where reading starts (top right in vertical writing, top left in horizontal), 1 at the far corner.
    fn corner_offsets(&self, at: &Placement) -> (f64, f64) {
        let u = (at.x as f64 - 1.0) / (self.paper.lines as f64 - at.w as f64).max(1.0);
        let v = (at.y as f64 - 1.0) / (self.paper.dan as f64 - at.span as f64).max(1.0);
        (u.clamp(0.0, 1.0), v.clamp(0.0, 1.0))
    }

    fn position(&self, item: &Item, at: &Placement) -> f64 {
        let (u, v) = self.corner_offsets(at);
        match item.role {
            Role::Nameplate | Role::Yoko => {
                let near_start = match self.paper.writing {
                    Writing::Vertical => 1.0 - (0.7 * v + 0.3 * u),
                    Writing::Horizontal => 1.0 - (0.7 * u + 0.3 * v),
                };
                if item.role == Role::Nameplate {
                    near_start
                } else {
                    0.5 + 0.5 * near_start
                }
            }
            Role::Ad => {
                if self.at_bottom(at) {
                    1.0
                } else {
                    0.5 * match self.paper.writing {
                        Writing::Vertical => v,
                        Writing::Horizontal => u,
                    }
                }
            }
            // SPEC 10.8: boxes and photos go in the dead corners, away from the main diagonal
            Role::Photo | Role::Box => (u - v).abs(),
            Role::Article => {
                let imp = emphasis(item);
                let d = (u + v) / 2.0;
                0.4 * (1.0 - (u - v).abs()) + 0.6 * (imp * (1.0 - d) + (1.0 - imp) * d)
            }
        }
    }

    /// Part of the score that does not need the whole layout; used to rank candidates cheaply.
    pub fn cheap(&self, item: &Item, at: &Placement) -> f64 {
        let s = self.item_score(item, at);
        let w = &self.weights;
        let pw = 0.5 + 0.5 * item.priority as f64 / 10.0;
        w.size * s.size * pw
            + w.shape * s.shape
            + w.position * s.position * pw
            + w.coverage * self.covered(item, at) / self.total_cells
    }

    pub fn total(
        &self,
        problem: &Problem,
        layout: &Layout,
        unplaced: &[&Item],
        forbidden: usize,
    ) -> ScoreBreakdown {
        let placed = layout
            .placements
            .iter()
            .filter_map(|p| problem.item(&p.id).map(|item| (item, p.at)));
        self.total_of(placed, unplaced, forbidden)
    }

    pub fn total_of<'a>(
        &self,
        placed: impl IntoIterator<Item = (&'a Item, Placement)>,
        unplaced: &[&Item],
        forbidden: usize,
    ) -> ScoreBreakdown {
        let w = &self.weights;
        let mut b = ScoreBreakdown::default();
        let mut cells = 0.0;
        let mut stories: Vec<(u8, f64)> = Vec::new();
        for (item, at) in placed {
            if item.fixed.is_some() {
                cells += area(&at) as f64;
                continue;
            }
            cells += self.covered(item, &at);
            let s = self.item_score(item, &at);
            let pw = 0.5 + 0.5 * item.priority as f64 / 10.0;
            b.size += w.size * s.size * pw;
            b.shape += w.shape * s.shape;
            b.position += w.position * s.position * pw;
            if item.role == Role::Article {
                stories.push((item.priority, area(&at) as f64));
            }
        }
        b.coverage = w.coverage * (cells / self.total_cells) * self.movable.max(1.0);
        for (i, (pi, ai)) in stories.iter().enumerate() {
            for (pj, aj) in &stories[i + 1..] {
                let (hi, lo) = if pi >= pj {
                    ((*pi, *ai), (*pj, *aj))
                } else {
                    ((*pj, *aj), (*pi, *ai))
                };
                if hi.0 > lo.0 && hi.1 < lo.1 {
                    b.hierarchy -= w.hierarchy * (hi.0 - lo.0) as f64 / 10.0;
                }
            }
        }
        b.forbidden = -w.forbidden * forbidden as f64;
        b.unplaced = -w.unplaced * unplaced.iter().map(|i| unplaced_cost(i)).sum::<f64>();
        b.total = b.size + b.shape + b.position + b.coverage + b.hierarchy + b.forbidden + b.unplaced;
        b
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Headline, Writing};

    fn problem(items: Vec<Item>) -> Problem {
        Problem {
            paper: Paper {
                writing: Writing::Vertical,
                dan: 10,
                lines: 50,
                chars: 10,
                column_em: None,
                gap_em: None,
            },
            items,
        }
    }

    fn article(id: &str, priority: u8) -> Item {
        Item {
            id: id.into(),
            role: Role::Article,
            priority,
            chars: 500,
            headline: Some(Headline::M),
            lead: false,
            ratio: None,
            caption: false,
            min_span: None,
            max_span: None,
            fixed: None,
        }
    }

    fn scorer(items: Vec<Item>) -> (Problem, Scorer) {
        let mut t = MeasurementTable::default();
        for it in &items {
            for span in 1..=10u8 {
                t.insert(&it.id, span, Some((60 / span).max(4)));
            }
        }
        let p = problem(items);
        let s = Scorer::new(&p, &t, Weights::default());
        (p, s)
    }

    #[test]
    fn important_stories_get_larger_targets() {
        let (_, s) = scorer(vec![article("big", 9), article("small", 2)]);
        assert!(s.target_area("big") > s.target_area("small"));
    }

    #[test]
    fn the_lead_prefers_the_start_corner_and_minor_stories_the_far_one() {
        let (p, s) = scorer(vec![article("lead", 10), article("minor", 1)]);
        let near = Placement {
            x: 1,
            w: 15,
            y: 1,
            span: 4,
        };
        let far = Placement {
            x: 36,
            w: 15,
            y: 7,
            span: 4,
        };
        let lead = p.item("lead").unwrap();
        let minor = p.item("minor").unwrap();
        assert!(s.item_score(lead, &near).position > s.item_score(lead, &far).position);
        assert!(s.item_score(minor, &far).position > s.item_score(minor, &near).position);
    }

    #[test]
    fn boxes_prefer_the_dead_corners() {
        let mut b = article("b", 3);
        b.role = Role::Box;
        let (_, s) = scorer(vec![b.clone()]);
        let diagonal = Placement {
            x: 18,
            w: 14,
            y: 4,
            span: 3,
        };
        let corner = Placement {
            x: 37,
            w: 14,
            y: 1,
            span: 3,
        };
        assert!(s.item_score(&b, &corner).position > s.item_score(&b, &diagonal).position);
    }

    #[test]
    fn extreme_shapes_score_worse() {
        let (p, s) = scorer(vec![article("a", 5)]);
        let a = p.item("a").unwrap();
        let square = Placement {
            x: 1,
            w: 10,
            y: 1,
            span: 2,
        };
        let sliver = Placement {
            x: 1,
            w: 40,
            y: 1,
            span: 1,
        };
        assert!(s.item_score(a, &square).shape > s.item_score(a, &sliver).shape);
    }

    #[test]
    fn unplaced_important_items_cost_more() {
        let (p, s) = scorer(vec![article("hi", 9), article("lo", 1)]);
        let empty = Layout::default();
        let hi = s.total(&p, &empty, &[p.item("hi").unwrap()], 0);
        let lo = s.total(&p, &empty, &[p.item("lo").unwrap()], 0);
        assert!(hi.total < lo.total);
    }
}
