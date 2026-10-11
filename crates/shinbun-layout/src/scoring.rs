use crate::geometry::area;
use crate::model::{Item, Layout, Lookup, MeasurementTable, Paper, Placement, Problem, Role};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Weights {
    pub size: f64,
    pub shape: f64,
    pub position: f64,
    pub coverage: f64,
    pub hierarchy: f64,
    pub forbidden: f64,
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
            unplaced: 12.0,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct ScoreBreakdown {
    pub size: f64,
    pub shape: f64,
    pub position: f64,
    pub coverage: f64,
    pub hierarchy: f64,
    pub forbidden: f64,
    pub unplaced: f64,
    pub total: f64,
}

#[derive(Debug, Clone, Copy)]
pub struct ItemScore {
    pub size: f64,
    pub shape: f64,
    pub position: f64,
}

/// Line pitch over character size in shinbun-css (`--sb-lh`)
const LINE_HEIGHT: f64 = 1.55;

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

/// Whether the item holds running text, so its size follows from the text rather than from taste.
pub fn has_text(role: Role) -> bool {
    matches!(role, Role::Article | Role::Box)
}

fn emphasis(item: &Item) -> f64 {
    if item.lead {
        1.0
    } else {
        item.priority as f64 / 10.0
    }
}

/// Everything the scores need that does not change while searching.
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
            let target = match (has_text(it.role), min_area.is_finite()) {
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

    pub fn target_area(&self, id: &str) -> f64 {
        self.targets.get(id).copied().unwrap_or(1.0)
    }

    pub fn item_score(&self, item: &Item, at: &Placement) -> ItemScore {
        let target = self.target_area(&item.id);
        let size = 1.0 - ((area(at) as f64 - target).abs() / target).min(1.0);

        let aspect = self.aspect(at);
        let shape = match item.role {
            Role::Article | Role::Photo | Role::Box => 1.0 - ((aspect - 1.0) / 5.0).clamp(0.0, 1.0),
            _ => 1.0,
        };

        ItemScore {
            size,
            shape,
            position: self.position(item, at),
        }
    }

    /// Long side over short side, measured in character sizes (a line is `--sb-lh` characters wide).
    pub fn aspect(&self, at: &Placement) -> f64 {
        let along_lines = at.w as f64 * LINE_HEIGHT;
        let along_columns = at.span as f64 * self.paper.chars as f64;
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
            Role::Nameplate | Role::Yoko => 1.0 - (0.7 * v + 0.3 * u),
            Role::Ad => {
                if at.y + at.span - 1 == self.paper.dan {
                    1.0
                } else {
                    0.5 * v
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
            + w.coverage * area(at) as f64 / self.total_cells
    }

    pub fn total(
        &self,
        problem: &Problem,
        layout: &Layout,
        unplaced: &[&Item],
        forbidden: usize,
    ) -> ScoreBreakdown {
        let w = &self.weights;
        let mut b = ScoreBreakdown::default();
        let mut cells = 0.0;
        let mut stories: Vec<(u8, f64)> = Vec::new();
        for p in &layout.placements {
            let Some(item) = problem.item(&p.id) else { continue };
            cells += area(&p.at) as f64;
            if item.fixed.is_some() {
                continue;
            }
            let s = self.item_score(item, &p.at);
            let pw = 0.5 + 0.5 * item.priority as f64 / 10.0;
            b.size += w.size * s.size * pw;
            b.shape += w.shape * s.shape;
            b.position += w.position * s.position * pw;
            if item.role == Role::Article {
                stories.push((item.priority, area(&p.at) as f64));
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
        b.unplaced = -w.unplaced * unplaced.iter().map(|i| i.priority as f64 / 10.0).sum::<f64>();
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
