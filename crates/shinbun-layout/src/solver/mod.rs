mod beam;

use crate::candidates::DEFAULT_SLACK;
use crate::model::{Layout, MeasurementTable, Problem, ProblemError};
use crate::scoring::{ScoreBreakdown, Weights};
use crate::validate::{Rules, Violation};
use serde::{Deserialize, Serialize};

pub use beam::{Beam, Constructive};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SolveOptions {
    /// How many partial layouts are kept after each item. 1 is the plain greedy method.
    pub beam_width: usize,
    /// How many placements are tried for each kept layout
    pub branching: usize,
    /// How many other layouts to return besides the best one
    pub alternatives: usize,
    /// Extra widths tried beyond the smallest one that fits
    pub slack: Vec<u8>,
    /// Text frames longer or wider than this ratio are never tried
    pub max_aspect: f64,
    pub weights: Weights,
    pub rules: Rules,
}

impl Default for SolveOptions {
    fn default() -> Self {
        Self {
            beam_width: 16,
            branching: 8,
            alternatives: 3,
            slack: DEFAULT_SLACK.to_vec(),
            max_aspect: 6.0,
            weights: Weights::default(),
            rules: Rules::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Reason {
    /// The measurements say the item cannot fit at any column span
    NoShape,
    /// It fits somewhere in principle, but no free place was left
    NoSpace,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Unplaced {
    pub id: String,
    pub reason: Reason,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Alternative {
    pub layout: Layout,
    pub score: ScoreBreakdown,
    pub warnings: Vec<Violation>,
}

/// Not finding a place for an item is part of the result, not an error.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Solution {
    pub layout: Layout,
    pub unplaced: Vec<Unplaced>,
    pub score: ScoreBreakdown,
    pub warnings: Vec<Violation>,
    pub alternatives: Vec<Alternative>,
}

pub trait Solver {
    fn solve(&self, problem: &Problem, table: &MeasurementTable) -> Result<Solution, ProblemError>;
}
