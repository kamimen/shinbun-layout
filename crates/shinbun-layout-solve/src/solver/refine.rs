use super::beam::{candidates, full_score, prepare, search, solution, start_state, Prep, State};
use super::{Reason, Solution, SolveOptions, Solver};
use crate::candidates::shapes;
use shinbun_layout_core::geometry::Occupancy;
use shinbun_layout_core::model::{MeasurementTable, Placement, Problem, ProblemError};
use std::collections::HashSet;

/// Small deterministic generator (splitmix64); the solver must give the same answer for the same seed.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    fn below(&mut self, n: usize) -> usize {
        (self.next() % n.max(1) as u64) as usize
    }

    fn unit(&mut self) -> f64 {
        (self.next() >> 11) as f64 / (1u64 << 53) as f64
    }
}

fn center(at: &Placement) -> (f64, f64) {
    (
        at.x as f64 + at.w as f64 / 2.0,
        at.y as f64 + at.span as f64 / 2.0,
    )
}

/// Removes one placed item and its nearest neighbours, so the freed room is contiguous.
fn ruin(problem: &Problem, st: &State, rng: &mut Rng, count: usize) -> State {
    let movable: Vec<usize> = (0..st.placed.len())
        .filter(|k| problem.items[st.placed[*k].0].fixed.is_none())
        .collect();
    let mut next = st.clone();
    if movable.is_empty() {
        return next;
    }
    let seed = movable[rng.below(movable.len())];
    let (cx, cy) = center(&st.placed[seed].1);
    let mut by_distance = movable;
    by_distance.sort_by(|a, b| {
        let d = |k: &usize| {
            let (x, y) = center(&st.placed[*k].1);
            // A column is much longer than a line is wide in cells, so scale the axes alike
            (x - cx).powi(2) + (3.0 * (y - cy)).powi(2)
        };
        d(a).total_cmp(&d(b)).then(a.cmp(b))
    });
    let mut removed: Vec<usize> = by_distance.into_iter().take(count).collect();
    removed.sort_unstable_by(|a, b| b.cmp(a));
    for k in removed {
        let (idx, _) = next.placed.remove(k);
        next.unplaced.push((idx, Reason::NoSpace));
    }
    next.occ = Occupancy::default();
    for (_, at) in &next.placed {
        next.occ.occupy(at);
    }
    next
}

/// Puts the removed and unplaced items back one by one, each at the placement that scores best overall.
fn recreate(
    problem: &Problem,
    table: &MeasurementTable,
    prep: &Prep,
    opts: &SolveOptions,
    mut st: State,
    rng: &mut Rng,
) -> State {
    let mut pending: Vec<usize> = st.unplaced.iter().map(|(i, _)| *i).collect();
    pending.sort_by_key(|i| prep.order.iter().position(|o| o == i));
    st.unplaced.clear();
    for idx in pending {
        let item = &problem.items[idx];
        let all = shapes(item, &problem.paper, table, &opts.slack);
        let cands = if all.is_empty() {
            Vec::new()
        } else {
            candidates(problem, &prep.scorer, opts, &st, item, &all)
        };
        let mut best: Option<(f64, State)> = None;
        for (_, at) in cands.iter().take(opts.branching.max(1) * 2) {
            let mut next = st.clone();
            next.occ.occupy(at);
            next.placed.push((idx, *at));
            let score = full_score(problem, &prep.scorer, opts, &next) + 0.05 * rng.unit();
            if best.as_ref().map_or(true, |(b, _)| score > *b) {
                best = Some((score, next));
            }
        }
        st = match best {
            Some((_, next)) => next,
            None => {
                let reason = if all.is_empty() {
                    Reason::NoShape
                } else {
                    Reason::NoSpace
                };
                st.unplaced.push((idx, reason));
                st
            }
        };
    }
    st
}

/// Beam search followed by ruin-and-recreate under simulated annealing.
/// The best layout seen is returned, so the result is never worse than the beam's.
#[derive(Debug, Clone, Default)]
pub struct Refine {
    pub options: SolveOptions,
}

impl Refine {
    pub fn new(options: SolveOptions) -> Self {
        Self { options }
    }
}

impl Solver for Refine {
    fn solve(&self, problem: &Problem, table: &MeasurementTable) -> Result<Solution, ProblemError> {
        problem.check()?;
        let opts = &self.options;
        let mut states = search(problem, table, opts);
        let Some(first) = states.first().cloned() else {
            return Ok(solution(problem, table, opts, &[start_state(problem)]));
        };
        let prep = prepare(problem, table, opts);
        let mut rng = Rng(opts.seed);
        let iterations = opts.refine_iterations;
        let (t0, t1) = (1.0_f64, 0.02_f64);

        let mut current = first.clone();
        let mut best = first;
        for step in 0..iterations {
            let t = t0 * (t1 / t0).powf(step as f64 / iterations.max(1) as f64);
            let count = 1 + rng.below(3);
            let ruined = ruin(problem, &current, &mut rng, count);
            let mut next = recreate(problem, table, &prep, opts, ruined, &mut rng);
            next.full = full_score(problem, &prep.scorer, opts, &next);
            let accept = next.full >= current.full || rng.unit() < ((next.full - current.full) / t).exp();
            if next.full > best.full {
                best = next.clone();
            }
            if accept {
                current = next;
            }
        }

        let mut seen = HashSet::new();
        states.insert(0, best);
        states.retain(|s| seen.insert(s.key()));
        states.sort_by(|a, b| b.full.total_cmp(&a.full).then_with(|| a.key().cmp(&b.key())));
        Ok(solution(problem, table, opts, &states))
    }
}
