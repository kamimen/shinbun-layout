//! The layout problem as a 0-1 program.
//!
//! One variable per (item, shape, position). Each item takes at most one, each cell is covered at most once.
//! The objective holds the parts of the score that belong to a single item; the hierarchy and
//! forbidden-pattern terms involve pairs of items and are left out, so compare with `separable`.

use good_lp::{constraint, default_solver, variable, variables, Expression, Solution, SolverModel, Variable};
use shinbun_layout::candidates::shapes;
use shinbun_layout::geometry::{area, in_bounds, Occupancy};
use shinbun_layout::scoring::Scorer;
use shinbun_layout::{Layout, MeasurementTable, Placed, Placement, Problem, SolveOptions};

pub struct Exact {
    pub layout: Layout,
    pub objective: f64,
}

/// Score of one placed item, without the terms that involve other items.
pub fn separable(problem: &Problem, table: &MeasurementTable, opts: &SolveOptions, layout: &Layout) -> f64 {
    let scorer = Scorer::new(problem, table, opts.weights);
    let total_cells = problem.paper.dan as f64 * problem.paper.lines as f64;
    let movable = problem.items.iter().filter(|i| i.fixed.is_none()).count() as f64;
    layout
        .placements
        .iter()
        .filter_map(|p| problem.item(&p.id).map(|it| (it, p)))
        .filter(|(it, _)| it.fixed.is_none())
        .map(|(it, p)| gain(&scorer, opts, it, &p.at, total_cells, movable))
        .sum()
}

fn gain(
    scorer: &Scorer,
    opts: &SolveOptions,
    item: &shinbun_layout::Item,
    at: &Placement,
    total_cells: f64,
    movable: f64,
) -> f64 {
    let w = &opts.weights;
    let s = scorer.item_score(item, at);
    let pw = 0.5 + 0.5 * item.priority as f64 / 10.0;
    w.size * s.size * pw
        + w.shape * s.shape
        + w.position * s.position * pw
        + w.coverage * (area(at) as f64 / total_cells) * movable.max(1.0)
        + w.unplaced * item.priority as f64 / 10.0
}

pub fn solve(problem: &Problem, table: &MeasurementTable, opts: &SolveOptions) -> Result<Exact, String> {
    problem.check().map_err(|e| e.to_string())?;
    let paper = problem.paper;
    let scorer = Scorer::new(problem, table, opts.weights);
    let total_cells = paper.dan as f64 * paper.lines as f64;
    let movable = problem.items.iter().filter(|i| i.fixed.is_none()).count() as f64;

    let mut fixed = Occupancy::default();
    for it in &problem.items {
        if let Some(at) = it.fixed {
            fixed.occupy(&at);
        }
    }

    let mut vars = variables!();
    let mut columns: Vec<(usize, Placement, Variable)> = Vec::new();
    let mut objective = Expression::from(0.0);
    for (i, item) in problem
        .items
        .iter()
        .enumerate()
        .filter(|(_, it)| it.fixed.is_none())
    {
        for shape in shapes(item, &paper, table, &opts.slack) {
            for x in 1..=paper.lines.saturating_sub(shape.w) + 1 {
                for y in 1..=paper.dan.saturating_sub(shape.span) + 1 {
                    let at = Placement {
                        x,
                        w: shape.w,
                        y,
                        span: shape.span,
                    };
                    let sliver = item.role.has_text() && scorer.aspect(&at) > opts.max_aspect;
                    if sliver || !in_bounds(&paper, &at) || !fixed.is_free(&at) {
                        continue;
                    }
                    let v = vars.add(variable().binary());
                    objective += gain(&scorer, opts, item, &at, total_cells, movable) * v;
                    columns.push((i, at, v));
                }
            }
        }
    }

    let mut model = vars
        .maximise(objective)
        .using(default_solver)
        .set_time_limit(60.0);
    model = model.set_mip_rel_gap(0.005).map_err(|e| format!("{e:?}"))?;
    for i in 0..problem.items.len() {
        let mine: Expression = columns.iter().filter(|c| c.0 == i).map(|c| c.2).sum();
        model = model.with(constraint!(mine <= 1.0));
    }
    for line in 1..=paper.lines {
        for col in 1..=paper.dan {
            let cell: Expression = columns
                .iter()
                .filter(|(_, at, _)| {
                    line >= at.x && line < at.x + at.w && col >= at.y && col < at.y + at.span
                })
                .map(|c| c.2)
                .sum();
            model = model.with(constraint!(cell <= 1.0));
        }
    }
    let solved = model.solve().map_err(|e| e.to_string())?;

    let mut placements: Vec<Placed> = problem
        .items
        .iter()
        .filter_map(|it| {
            it.fixed.map(|at| Placed {
                id: it.id.clone(),
                at,
            })
        })
        .collect();
    for (i, at, v) in &columns {
        if solved.value(*v) > 0.5 {
            placements.push(Placed {
                id: problem.items[*i].id.clone(),
                at: *at,
            });
        }
    }
    let layout = Layout { placements };
    let objective = separable(problem, table, opts, &layout);
    Ok(Exact { layout, objective })
}

/// The real score of a layout, including the terms between items.
pub fn total(problem: &Problem, table: &MeasurementTable, opts: &SolveOptions, layout: &Layout) -> f64 {
    let scorer = Scorer::new(problem, table, opts.weights);
    let warnings = shinbun_layout::validate(problem, table, layout, &opts.rules);
    let forbidden = warnings.iter().filter(|w| !w.is_hard()).count();
    let placed: std::collections::HashSet<&str> = layout.placements.iter().map(|p| p.id.as_str()).collect();
    let unplaced: Vec<&shinbun_layout::Item> = problem
        .items
        .iter()
        .filter(|i| !placed.contains(i.id.as_str()))
        .collect();
    scorer.total(problem, layout, &unplaced, forbidden).total
}
