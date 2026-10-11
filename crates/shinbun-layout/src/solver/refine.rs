use super::beam::{candidates, full_score, prepare, search, solution, start_state, Prep, State};
use super::separate::separate;
use super::{Reason, Solution, SolveOptions, Solver};
use crate::candidates::shapes;
use crate::geometry::Occupancy;
use crate::model::{MeasurementTable, Placement, Problem, ProblemError};
use std::collections::HashSet;

/// Small deterministic generator (splitmix64); the solver must give the same answer for the same seed.
pub(super) struct Rng(pub(super) u64);

impl Rng {
    pub(super) fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    pub(super) fn below(&mut self, n: usize) -> usize {
        (self.next() % n.max(1) as u64) as usize
    }

    pub(super) fn unit(&mut self) -> f64 {
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

/// Attempts to fit left-out items by overlap minimisation, each from a fresh random start of the search.
const SEPARATE_TRIES: usize = 3;

/// How hard the search for a best way of putting items back works: it grows with the product of the
/// number of items and their candidate placements, so small pages get a deeper search and large ones a cheaper one.
#[derive(Clone, Copy)]
struct Depth {
    items: usize,
    /// Up to this many items left, the wider branching applies
    wide_until: usize,
    branch_small: usize,
    branch_large: usize,
}

impl Depth {
    fn for_problem(problem: &Problem) -> Self {
        let movable = problem.items.iter().filter(|i| i.fixed.is_none()).count();
        if movable <= 8 {
            Depth {
                items: 4,
                wide_until: 3,
                branch_small: 10,
                branch_large: 6,
            }
        } else {
            Depth {
                items: 3,
                wide_until: 2,
                branch_small: 8,
                branch_large: 4,
            }
        }
    }

    fn branch(&self, items: usize) -> usize {
        if items <= self.wide_until {
            self.branch_small
        } else {
            self.branch_large
        }
    }
}

/// Puts the items back in the best way among their top placements, or leaves some out if that scores higher.
fn best_completion(
    problem: &Problem,
    table: &MeasurementTable,
    prep: &Prep,
    opts: &SolveOptions,
    st: State,
    pending: &[usize],
    depth: Depth,
) -> State {
    let Some((&idx, rest)) = pending.split_first() else {
        let mut done = st;
        done.full = full_score(problem, &prep.scorer, opts, &done);
        return done;
    };
    let item = &problem.items[idx];
    let all = shapes(item, &problem.paper, table, &opts.slack);
    let mut best: Option<State> = None;
    let mut consider = |cand: State| {
        if best.as_ref().map_or(true, |b| cand.full > b.full) {
            best = Some(cand);
        }
    };
    if !all.is_empty() {
        for (_, at) in candidates(problem, &prep.scorer, opts, &st, item, &all)
            .iter()
            .take(depth.branch(rest.len() + 1))
        {
            let mut next = st.clone();
            next.occ.occupy(at);
            next.placed.push((idx, *at));
            consider(best_completion(problem, table, prep, opts, next, rest, depth));
        }
    }
    let mut skipped = st;
    let reason = if all.is_empty() {
        Reason::NoShape
    } else {
        Reason::NoSpace
    };
    skipped.unplaced.push((idx, reason));
    consider(best_completion(problem, table, prep, opts, skipped, rest, depth));
    best.expect("at least the skipped branch exists")
}

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
    if rng.below(2) == 0 {
        // Sometimes the stories go first, so a photo placed early cannot take the room they need
        pending.sort_by_key(|i| {
            let it = &problem.items[*i];
            (
                std::cmp::Reverse(it.priority),
                prep.order.iter().position(|o| o == i),
            )
        });
    }
    st.unplaced.clear();
    let depth = Depth::for_problem(problem);
    if pending.len() <= depth.items {
        return best_completion(problem, table, prep, opts, st, &pending, depth);
    }
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
    /// Beam width, iterations, seed and the rest.
    pub options: SolveOptions,
}

impl Refine {
    /// A refining search with these options.
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
            let room = Depth::for_problem(problem)
                .items
                .saturating_sub(current.unplaced.len())
                .clamp(1, 3);
            let count = 1 + rng.below(room);
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

        if opts.separate_moves > 0 && best.unplaced.iter().any(|(_, r)| *r == Reason::NoSpace) {
            for _ in 0..SEPARATE_TRIES {
                if let Some(found) = separate(
                    problem,
                    table,
                    &prep.scorer,
                    opts,
                    &best,
                    &mut rng,
                    opts.separate_moves,
                ) {
                    if found.full > best.full {
                        best = found;
                    }
                    break;
                }
            }
        }

        let mut seen = HashSet::new();
        states.insert(0, best);
        states.retain(|s| seen.insert(s.key()));
        states.sort_by(|a, b| b.full.total_cmp(&a.full).then_with(|| a.key().cmp(&b.key())));
        Ok(solution(problem, table, opts, &states))
    }
}
