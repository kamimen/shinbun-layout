//! Candidate generation, scoring and solvers. Depends only on `shinbun-layout-core`.

pub mod candidates;
pub mod scoring;
pub mod solver;

pub use candidates::required_measurements;
pub use scoring::{ScoreBreakdown, Weights};
pub use solver::{Alternative, Beam, Constructive, Reason, Solution, SolveOptions, Solver, Unplaced};
