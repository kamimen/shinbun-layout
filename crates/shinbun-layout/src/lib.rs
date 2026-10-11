//! Automatic layout of Japanese newspaper pages (新聞の紙面).
//!
//! The crate places articles, photos and boxes on the column grid of shinbun-css. It never renders text:
//! how many lines an item needs at each column span is measured by the host (a browser) and passed in as a
//! [`MeasurementTable`]. Using it takes three steps:
//!
//! 1. [`required_measurements`] says which sizes to measure.
//! 2. The host measures them and builds a [`MeasurementTable`].
//! 3. [`solve`] returns placements in the same `{x, w, y, span}` form that shinbun-maker and shinbun-cli use.
//!
//! [`validate`] checks any layout, whoever made it.

pub use shinbun_layout_core::*;
pub use shinbun_layout_solve::*;

/// Solves with the default beam search.
pub fn solve(
    problem: &Problem,
    table: &MeasurementTable,
    options: &SolveOptions,
) -> Result<Solution, ProblemError> {
    Beam::new(options.clone()).solve(problem, table)
}

/// JSON form of [`required_measurements`], for hosts that cannot call Rust directly.
pub fn required_measurements_json(problem_json: &str) -> Result<String, String> {
    let problem: Problem = serde_json::from_str(problem_json).map_err(|e| format!("invalid problem: {e}"))?;
    problem.check().map_err(|e| e.to_string())?;
    serde_json::to_string(&required_measurements(&problem)).map_err(|e| e.to_string())
}

/// JSON form of [`solve`]. `options_json` may be empty to use the defaults.
pub fn solve_json(problem_json: &str, table_json: &str, options_json: &str) -> Result<String, String> {
    let problem: Problem = serde_json::from_str(problem_json).map_err(|e| format!("invalid problem: {e}"))?;
    let table: MeasurementTable =
        serde_json::from_str(table_json).map_err(|e| format!("invalid measurements: {e}"))?;
    let options: SolveOptions = if options_json.trim().is_empty() {
        SolveOptions::default()
    } else {
        serde_json::from_str(options_json).map_err(|e| format!("invalid options: {e}"))?
    };
    let solution = solve(&problem, &table, &options).map_err(|e| e.to_string())?;
    serde_json::to_string(&solution).map_err(|e| e.to_string())
}
