use super::{Alternative, Reason, Solution, SolveOptions, Solver, Unplaced};
use crate::candidates::{anchors, shapes, Shape};
use crate::geometry::{in_bounds, Occupancy};
use crate::model::{Item, Layout, MeasurementTable, Paper, Placed, Placement, Problem, ProblemError};
use crate::scoring::Scorer;
use crate::validate::{forbidden_patterns, validate, View};
use std::cmp::Ordering;
use std::collections::HashSet;

type Key = Vec<(usize, u8, u8, u8, u8)>;

#[derive(Clone)]
pub(super) struct State {
    pub(super) occ: Occupancy,
    pub(super) placed: Vec<(usize, Placement)>,
    pub(super) unplaced: Vec<(usize, Reason)>,
    pub(super) cheap: f64,
    pub(super) full: f64,
}

impl State {
    pub(super) fn key(&self) -> Key {
        let mut k: Vec<_> = self
            .placed
            .iter()
            .map(|(i, a)| (*i, a.x, a.w, a.y, a.span))
            .collect();
        k.sort_unstable();
        k
    }
}

pub(super) fn by_score_desc(a: &(f64, Key), b: &(f64, Key)) -> Ordering {
    b.0.total_cmp(&a.0).then_with(|| a.1.cmp(&b.1))
}

/// Places items in the order newspaper makeup does (SPEC 10.8, 押えて流す): fixed things first, then
/// nameplate, ads, headlines, photos and boxes, and the stories last, most important first.
pub(super) fn order(problem: &Problem) -> Vec<usize> {
    let mut idx: Vec<usize> = (0..problem.items.len())
        .filter(|i| problem.items[*i].fixed.is_none())
        .collect();
    idx.sort_by(|a, b| {
        let (x, y) = (&problem.items[*a], &problem.items[*b]);
        x.role
            .cmp(&y.role)
            .then(y.priority.cmp(&x.priority))
            .then_with(|| x.id.cmp(&y.id))
    });
    idx
}

pub(super) fn layout_of(problem: &Problem, st: &State) -> Layout {
    let mut placements: Vec<(usize, Placement)> = st.placed.clone();
    placements.sort_by_key(|(i, _)| *i);
    Layout {
        placements: placements
            .into_iter()
            .map(|(i, at)| Placed {
                id: problem.items[i].id.clone(),
                at,
            })
            .collect(),
    }
}

pub(super) fn full_score(problem: &Problem, scorer: &Scorer, opts: &SolveOptions, st: &State) -> f64 {
    let views: Vec<View<'_>> = st
        .placed
        .iter()
        .map(|(i, at)| View {
            id: &problem.items[*i].id,
            role: problem.items[*i].role,
            headline: problem.items[*i].headline,
            at: *at,
        })
        .collect();
    let forbidden = forbidden_patterns(&problem.paper, &views, &opts.rules).len();
    let unplaced: Vec<&Item> = st.unplaced.iter().map(|(i, _)| &problem.items[*i]).collect();
    let placed = st.placed.iter().map(|(i, at)| (&problem.items[*i], *at));
    scorer.total_of(placed, &unplaced, forbidden).total
}

/// Smallest shapes kept per item for the look-ahead; enough to tell whether any room is left for it.
const LOOKAHEAD_SHAPES: usize = 8;

pub(super) struct Prep {
    pub(super) scorer: Scorer,
    pub(super) order: Vec<usize>,
    /// For each item index: its smallest shapes by area (empty if it cannot fit anywhere)
    smallest: Vec<Vec<Shape>>,
}

fn fits_somewhere(paper: &Paper, occ: &Occupancy, shape: &Shape) -> bool {
    (1..=paper.lines - shape.w + 1).any(|x| {
        (1..=paper.dan - shape.span + 1).any(|y| {
            occ.is_free(&Placement {
                x,
                w: shape.w,
                y,
                span: shape.span,
            })
        })
    })
}

/// Whether every item still to come keeps some room, so a big placement now cannot starve the small ones.
fn leaves_room(problem: &Problem, prep: &Prep, occ: &Occupancy, later: &[usize]) -> bool {
    let free = problem.paper.dan as u32 * problem.paper.lines as u32 - occ.cells();
    let mut need = 0;
    for &i in later {
        let Some(smallest) = prep.smallest[i].first() else {
            continue;
        };
        need += smallest.span as u32 * smallest.w as u32;
        if !prep.smallest[i]
            .iter()
            .any(|s| fits_somewhere(&problem.paper, occ, s))
        {
            return false;
        }
    }
    // Rectangles never tile perfectly, so ask for some slack
    need * 6 <= free * 5
}

/// Free placements for one item, best first by the cheap score.
pub(super) fn candidates(
    problem: &Problem,
    scorer: &Scorer,
    opts: &SolveOptions,
    st: &State,
    item: &Item,
    all_shapes: &[Shape],
) -> Vec<(f64, Placement)> {
    let placed: Vec<Placement> = st.placed.iter().map(|(_, a)| *a).collect();
    let mut cands: Vec<(f64, Placement)> = Vec::new();
    for shape in all_shapes {
        for (x, y) in anchors(&problem.paper, &placed, shape) {
            let at = Placement {
                x,
                w: shape.w,
                y,
                span: shape.span,
            };
            let sliver = item.role.has_text() && scorer.aspect(&at) > opts.max_aspect;
            if !sliver && in_bounds(&problem.paper, &at) && st.occ.is_free(&at) {
                cands.push((scorer.cheap(item, &at), at));
            }
        }
    }
    cands.sort_by(|a, b| {
        b.0.total_cmp(&a.0)
            .then_with(|| (a.1.x, a.1.y, a.1.span, a.1.w).cmp(&(b.1.x, b.1.y, b.1.span, b.1.w)))
    });
    cands
}

fn expand(
    problem: &Problem,
    table: &MeasurementTable,
    prep: &Prep,
    opts: &SolveOptions,
    st: &State,
    pos: usize,
) -> Vec<State> {
    let idx = prep.order[pos];
    let later = &prep.order[pos + 1..];
    let item = &problem.items[idx];
    let all_shapes = shapes(item, &problem.paper, table, &opts.slack);
    if all_shapes.is_empty() {
        let mut next = st.clone();
        next.unplaced.push((idx, Reason::NoShape));
        return vec![next];
    }
    let cands = candidates(problem, &prep.scorer, opts, st, item, &all_shapes);
    let want = opts.branching.max(1);
    let mut chosen: Vec<(f64, Placement)> = Vec::with_capacity(want);
    for (delta, at) in cands.iter().take(want * 8) {
        let mut occ = st.occ.clone();
        occ.occupy(at);
        if leaves_room(problem, prep, &occ, later) {
            chosen.push((*delta, *at));
            if chosen.len() == want {
                break;
            }
        }
    }
    let cands = if chosen.is_empty() {
        cands.into_iter().take(want).collect()
    } else {
        chosen
    };
    if cands.is_empty() {
        let mut next = st.clone();
        next.unplaced.push((idx, Reason::NoSpace));
        return vec![next];
    }
    cands
        .into_iter()
        .map(|(delta, at)| {
            let mut next = st.clone();
            next.occ.occupy(&at);
            next.placed.push((idx, at));
            next.cheap += delta;
            next
        })
        .collect()
}

pub(super) fn prepare(problem: &Problem, table: &MeasurementTable, opts: &SolveOptions) -> Prep {
    let order = order(problem);
    let mut smallest = vec![Vec::new(); problem.items.len()];
    for &i in &order {
        let mut all = shapes(&problem.items[i], &problem.paper, table, &opts.slack);
        all.sort_by_key(|s| (s.span as u32 * s.w as u32, s.span, s.w));
        all.truncate(LOOKAHEAD_SHAPES);
        smallest[i] = all;
    }
    Prep {
        scorer: Scorer::new(problem, table, opts.weights),
        order,
        smallest,
    }
}

pub(super) fn start_state(problem: &Problem) -> State {
    let mut start = State {
        occ: Occupancy::default(),
        placed: Vec::new(),
        unplaced: Vec::new(),
        cheap: 0.0,
        full: 0.0,
    };
    for (i, item) in problem.items.iter().enumerate() {
        if let Some(at) = item.fixed {
            start.occ.occupy(&at);
            start.placed.push((i, at));
        }
    }
    start
}

pub(super) fn search(problem: &Problem, table: &MeasurementTable, opts: &SolveOptions) -> Vec<State> {
    let prep = prepare(problem, table, opts);
    let scorer = &prep.scorer;
    let width = opts.beam_width.max(1);
    let mut beam = vec![start_state(problem)];

    for pos in 0..prep.order.len() {
        let mut children: Vec<State> = beam
            .iter()
            .flat_map(|st| expand(problem, table, &prep, opts, st, pos))
            .collect();
        let mut seen = HashSet::new();
        children.retain(|c| seen.insert((c.key(), c.unplaced.iter().map(|(i, _)| *i).collect::<Vec<_>>())));
        children.sort_by(|a, b| by_score_desc(&(a.cheap, a.key()), &(b.cheap, b.key())));
        children.truncate(width * 4);
        for c in &mut children {
            c.full = full_score(problem, scorer, opts, c);
        }
        children.sort_by(|a, b| by_score_desc(&(a.full, a.key()), &(b.full, b.key())));
        children.truncate(width);
        beam = children;
    }
    for st in &mut beam {
        st.full = full_score(problem, scorer, opts, st);
    }
    beam.sort_by(|a, b| by_score_desc(&(a.full, a.key()), &(b.full, b.key())));
    beam
}

pub(super) fn solution(
    problem: &Problem,
    table: &MeasurementTable,
    opts: &SolveOptions,
    states: &[State],
) -> Solution {
    let scorer = Scorer::new(problem, table, opts.weights);
    let describe = |st: &State| {
        let layout = layout_of(problem, st);
        let unplaced: Vec<&Item> = st.unplaced.iter().map(|(i, _)| &problem.items[*i]).collect();
        let warnings = validate(problem, table, &layout, &opts.rules);
        let forbidden = warnings.iter().filter(|w| !w.is_hard()).count();
        let score = scorer.total(problem, &layout, &unplaced, forbidden);
        (layout, score, warnings)
    };
    let best = &states[0];
    let (layout, score, warnings) = describe(best);
    let unplaced = best
        .unplaced
        .iter()
        .map(|(i, reason)| Unplaced {
            id: problem.items[*i].id.clone(),
            reason: *reason,
        })
        .collect();
    let alternatives = states
        .iter()
        .skip(1)
        .filter(|s| s.key() != best.key())
        .take(opts.alternatives)
        .map(|s| {
            let (layout, score, warnings) = describe(s);
            Alternative {
                layout,
                score,
                warnings,
            }
        })
        .collect();
    Solution {
        layout,
        unplaced,
        score,
        warnings,
        alternatives,
    }
}

/// Beam search over the placements of one item after another. Deterministic: ties are broken by position.
#[derive(Debug, Clone, Default)]
pub struct Beam {
    /// Beam width, branching and the rest.
    pub options: SolveOptions,
}

impl Beam {
    /// A beam search with these options.
    pub fn new(options: SolveOptions) -> Self {
        Self { options }
    }
}

impl Solver for Beam {
    fn solve(&self, problem: &Problem, table: &MeasurementTable) -> Result<Solution, ProblemError> {
        problem.check()?;
        let mut states = search(problem, table, &self.options);
        if self.options.beam_width > 1 {
            // The greedy path can fall out of a narrow beam, so keep its result as a floor.
            let greedy = SolveOptions {
                beam_width: 1,
                branching: 1,
                ..self.options.clone()
            };
            let mut seen: HashSet<_> = states.iter().map(State::key).collect();
            states.extend(
                search(problem, table, &greedy)
                    .into_iter()
                    .filter(|s| seen.insert(s.key())),
            );
            states.sort_by(|a, b| by_score_desc(&(a.full, a.key()), &(b.full, b.key())));
        }
        Ok(solution(problem, table, &self.options, &states))
    }
}

/// The greedy method: the best placement for each item in turn, never looking back.
#[derive(Debug, Clone, Default)]
pub struct Constructive {
    /// Only the weights, rules and shapes matter; the width and branching are set to 1.
    pub options: SolveOptions,
}

impl Solver for Constructive {
    fn solve(&self, problem: &Problem, table: &MeasurementTable) -> Result<Solution, ProblemError> {
        problem.check()?;
        let options = SolveOptions {
            beam_width: 1,
            branching: 1,
            alternatives: 0,
            ..self.options.clone()
        };
        let states = search(problem, table, &options);
        Ok(solution(problem, table, &options, &states))
    }
}
