//! Guided local search on overlap, after the way `sparrow` (<https://github.com/JeroenGar/sparrow>) packs:
//! every item may overlap for a while, one conflicting item at a time moves to the place where it
//! overlaps least, and cells that stay in conflict get heavier so the search cannot settle in them.
//! A move looks at every shape and position at once, using prefix sums of the cell weights.

use super::beam::{full_score, State};
use super::refine::Rng;
use super::{Reason, SolveOptions};
use crate::candidates::shapes;
use crate::geometry::Occupancy;
use crate::model::{MeasurementTable, Placement, Problem};
use crate::scoring::Scorer;

/// How much a little quality is worth against one weighted cell of overlap; small, so overlap dominates.
const QUALITY: f64 = 0.02;
const GROWTH: f64 = 1.5;
const MAX_WEIGHT: f64 = 1e6;

struct Grid {
    dan: usize,
    lines: usize,
}

impl Grid {
    fn at(&self, line: usize, col: usize) -> usize {
        col * self.lines + line
    }
}

struct Mover {
    index: usize,
    options: Vec<(Placement, f64)>,
    now: Option<Placement>,
}

fn cells(p: &Placement) -> impl Iterator<Item = (usize, usize)> {
    let (x, y, w, span) = (p.x as usize - 1, p.y as usize - 1, p.w as usize, p.span as usize);
    (y..y + span).flat_map(move |c| (x..x + w).map(move |l| (l, c)))
}

/// Tries to fit everything, including the items `st` left out. Returns a layout without overlaps, or `None`.
pub(super) fn separate(
    problem: &Problem,
    table: &MeasurementTable,
    scorer: &Scorer,
    opts: &SolveOptions,
    st: &State,
    rng: &mut Rng,
    max_moves: usize,
) -> Option<State> {
    let paper = problem.paper;
    let grid = Grid {
        dan: paper.dan as usize,
        lines: paper.lines as usize,
    };
    let mut fixed = Occupancy::default();
    let mut kept = Vec::new();
    for (i, at) in &st.placed {
        if problem.items[*i].fixed.is_some() {
            fixed.occupy(at);
            kept.push((*i, *at));
        }
    }

    let mut movers: Vec<Mover> = Vec::new();
    let mut hopeless = Vec::new();
    for (i, item) in problem
        .items
        .iter()
        .enumerate()
        .filter(|(_, it)| it.fixed.is_none())
    {
        let mut options = Vec::new();
        for shape in shapes(item, &paper, table, &opts.slack) {
            for x in 1..=paper.lines - shape.w + 1 {
                for y in 1..=paper.dan - shape.span + 1 {
                    let at = Placement {
                        x,
                        w: shape.w,
                        y,
                        span: shape.span,
                    };
                    if item.role.has_text() && scorer.aspect(&at) > opts.max_aspect {
                        continue;
                    }
                    if fixed.is_free(&at) {
                        options.push((at, scorer.cheap(item, &at)));
                    }
                }
            }
        }
        if options.is_empty() {
            hopeless.push(i);
            continue;
        }
        let now = st.placed.iter().find(|(j, _)| *j == i).map(|(_, a)| *a);
        movers.push(Mover {
            index: i,
            options,
            now,
        });
    }
    if movers.is_empty() {
        return None;
    }

    let size = grid.dan * grid.lines;
    let mut count = vec![0u8; size];
    let mut weight = vec![1.0f64; size];
    for m in &movers {
        if let Some(at) = m.now {
            for (l, c) in cells(&at) {
                count[grid.at(l, c)] += 1;
            }
        }
    }
    // prefix[(c)*(lines+1)+l]: sum of weight*count over lines < l and columns < c
    let stride = grid.lines + 1;
    let mut prefix = vec![0.0f64; (grid.dan + 1) * stride];
    let build = |prefix: &mut Vec<f64>, count: &[u8], weight: &[f64]| {
        for c in 0..grid.dan {
            let mut row = 0.0;
            for l in 0..grid.lines {
                let k = grid.at(l, c);
                row += weight[k] * count[k] as f64;
                prefix[(c + 1) * stride + l + 1] = prefix[c * stride + l + 1] + row;
            }
        }
    };
    let overlap = |prefix: &[f64], p: &Placement| {
        let (x0, x1) = (p.x as usize - 1, p.x as usize - 1 + p.w as usize);
        let (y0, y1) = (p.y as usize - 1, p.y as usize - 1 + p.span as usize);
        prefix[y1 * stride + x1] - prefix[y0 * stride + x1] - prefix[y1 * stride + x0]
            + prefix[y0 * stride + x0]
    };
    let conflicted = |count: &[u8], m: &Mover| {
        m.now
            .is_some_and(|at| cells(&at).any(|(l, c)| count[grid.at(l, c)] > 1))
    };

    let mut moves = 0;
    while moves < max_moves {
        let hot: Vec<usize> = (0..movers.len())
            .filter(|k| movers[*k].now.is_none() || conflicted(&count, &movers[*k]))
            .collect();
        if hot.is_empty() {
            break;
        }
        for _ in 0..hot.len().max(1) {
            moves += 1;
            let k = hot[rng.below(hot.len())];
            if let Some(at) = movers[k].now.take() {
                for (l, c) in cells(&at) {
                    count[grid.at(l, c)] -= 1;
                }
            }
            build(&mut prefix, &count, &weight);
            let mut best: Option<(f64, usize)> = None;
            for (n, (at, quality)) in movers[k].options.iter().enumerate() {
                let cost = overlap(&prefix, at) - QUALITY * quality;
                if best.map_or(true, |(b, _)| cost < b) {
                    best = Some((cost, n));
                }
            }
            let (_, n) = best?;
            let at = movers[k].options[n].0;
            for (l, c) in cells(&at) {
                count[grid.at(l, c)] += 1;
            }
            movers[k].now = Some(at);
        }
        for k in 0..size {
            if count[k] > 1 {
                weight[k] = (weight[k] * GROWTH).min(MAX_WEIGHT);
            }
        }
    }
    if movers.iter().any(|m| m.now.is_none() || conflicted(&count, m)) {
        return None;
    }

    let mut out = State {
        occ: fixed,
        placed: kept,
        unplaced: hopeless.iter().map(|i| (*i, Reason::NoShape)).collect(),
        cheap: 0.0,
        full: 0.0,
    };
    for m in &movers {
        let at = m.now?;
        out.occ.occupy(&at);
        out.placed.push((m.index, at));
    }
    out.full = full_score(problem, scorer, opts, &out);
    Some(out)
}
