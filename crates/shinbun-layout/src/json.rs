//! The same calls with JSON text in and out, for hosts that are not written in Rust.

use crate::model::{MeasurementTable, Problem, ProblemError};
use crate::solver::SolveOptions;
use std::fmt;

/// Why a JSON call failed.
#[derive(Debug)]
#[non_exhaustive]
pub enum Error {
    /// The problem is not valid JSON of the right shape.
    Problem(serde_json::Error),
    /// The measurement table is not valid JSON of the right shape.
    Measurements(serde_json::Error),
    /// The options are not valid JSON of the right shape.
    Options(serde_json::Error),
    /// The problem is well-formed JSON but cannot be a problem (duplicate ids, a fixed item outside the paper, ...).
    Invalid(ProblemError),
    /// The result could not be written as JSON.
    Serialize(serde_json::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Problem(e) => write!(f, "invalid problem: {e}"),
            Self::Measurements(e) => write!(f, "invalid measurements: {e}"),
            Self::Options(e) => write!(f, "invalid options: {e}"),
            Self::Invalid(e) => write!(f, "{e}"),
            Self::Serialize(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for Error {}

fn problem(json: &str) -> Result<Problem, Error> {
    serde_json::from_str(json).map_err(Error::Problem)
}

/// [`required_measurements`](crate::required_measurements) for a problem given as JSON.
pub fn required_measurements(problem_json: &str) -> Result<String, Error> {
    let problem = problem(problem_json)?;
    problem.check().map_err(Error::Invalid)?;
    serde_json::to_string(&crate::required_measurements(&problem)).map_err(Error::Serialize)
}

/// [`solve`](crate::solve) with everything given as JSON. `options_json` may be empty to use the defaults.
pub fn solve(problem_json: &str, table_json: &str, options_json: &str) -> Result<String, Error> {
    let problem = problem(problem_json)?;
    let table: MeasurementTable = serde_json::from_str(table_json).map_err(Error::Measurements)?;
    let options: SolveOptions = if options_json.trim().is_empty() {
        SolveOptions::default()
    } else {
        serde_json::from_str(options_json).map_err(Error::Options)?
    };
    let solution = crate::solve(&problem, &table, &options).map_err(Error::Invalid)?;
    serde_json::to_string(&solution).map_err(Error::Serialize)
}
