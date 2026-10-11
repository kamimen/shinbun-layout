//! Automatic layout of Japanese newspaper pages (新聞の紙面) on the column grid of shinbun-css.
//!
//! [`required_measurements`] says which sizes the host has to measure, the host builds a
//! [`MeasurementTable`], and [`solve`] returns the placements. [`validate()`](fn@validate) checks any layout.
//!
//! ```
//! use shinbun_layout::*;
//!
//! let paper = Paper::new(Writing::Vertical, 6, 24, 10);
//! let problem = Problem::new(
//!     paper,
//!     vec![
//!         Item::new("lead", Role::Article).priority(10).chars(300).headline(Headline::L).lead(),
//!         Item::new("photo", Role::Photo).ratio(1.5),
//!     ],
//! );
//!
//! // The host measures these in a browser; this answer is made up: ten characters a line, three lines of headline.
//! let mut table = MeasurementTable::default();
//! for q in required_measurements(&problem) {
//!     let lines = if q.id == "lead" { 300u32.div_ceil(q.span as u32 * 10) as u8 + 3 } else { 1 };
//!     table.insert(&q.id, q.span, Some(lines));
//! }
//!
//! let solution = solve(&problem, &table, &SolveOptions::default()).unwrap();
//! assert!(solution.unplaced.is_empty());
//! assert!(solution.layout.get("lead").is_some());
//! ```

#![warn(missing_docs)]

#[allow(missing_docs)]
mod candidates;
#[allow(missing_docs)]
mod geometry;
pub mod json;
mod model;
#[allow(missing_docs)]
mod scoring;
mod solver;
mod validate;

#[doc(hidden)]
pub mod __internal {
    pub use crate::candidates::shapes;
    pub use crate::geometry::{area, in_bounds, Occupancy};
    pub use crate::scoring::{unplaced_cost, Scorer};
}

pub use candidates::required_measurements;
pub use model::{
    Headline, Item, Layout, Lookup, Measure, MeasureQuery, MeasurementTable, Paper, Placed, Placement,
    Problem, ProblemError, Role, Writing,
};
pub use scoring::{ScoreBreakdown, Weights};
pub use solver::{Alternative, Beam, Constructive, Reason, Refine, Solution, SolveOptions, Solver, Unplaced};
pub use validate::{validate, Pattern, Rules, Violation};

/// Solves with beam search, then improves the result for `options.refine_iterations` steps.
pub fn solve(
    problem: &Problem,
    table: &MeasurementTable,
    options: &SolveOptions,
) -> Result<Solution, ProblemError> {
    Refine::new(options.clone()).solve(problem, table)
}
