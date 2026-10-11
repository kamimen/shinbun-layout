mod beam;
mod refine;
mod separate;

use crate::candidates::DEFAULT_SLACK;
use crate::model::{Layout, MeasurementTable, Problem, ProblemError};
use crate::scoring::{ScoreBreakdown, Weights};
use crate::validate::{Rules, Violation};
use serde::{Deserialize, Serialize};

pub use beam::{Beam, Constructive};
pub use refine::Refine;

/// How hard and in what way to search. Everything has a sensible default.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
#[non_exhaustive]
pub struct SolveOptions {
    /// How many partial layouts are kept after each item. 1 is the plain greedy method.
    pub beam_width: usize,
    /// How many placements are tried for each kept layout
    pub branching: usize,
    /// How many other layouts to return besides the best one
    pub alternatives: usize,
    /// Extra widths tried beyond the smallest one that fits
    pub slack: Vec<u8>,
    /// Ruin-and-recreate steps after the beam search; used by `Refine` only
    pub refine_iterations: usize,
    /// Moves allowed for the overlap search that tries to fit left-out items; 0 turns it off
    pub separate_moves: usize,
    /// Seed for `Refine`; the same seed gives the same layout
    pub seed: u64,
    /// Text frames longer or wider than this ratio are never tried
    pub max_aspect: f64,
    /// How much each part of the score counts.
    pub weights: Weights,
    /// Which style patterns are looked for.
    pub rules: Rules,
}

impl SolveOptions {
    /// Partial layouts kept after each item.
    pub fn beam_width(mut self, n: usize) -> Self {
        self.beam_width = n;
        self
    }

    /// Placements tried for each kept layout.
    pub fn branching(mut self, n: usize) -> Self {
        self.branching = n;
        self
    }

    /// Other layouts returned besides the best one.
    pub fn alternatives(mut self, n: usize) -> Self {
        self.alternatives = n;
        self
    }

    /// Ruin-and-recreate steps after the beam search.
    pub fn refine_iterations(mut self, n: usize) -> Self {
        self.refine_iterations = n;
        self
    }

    /// Moves for the overlap search that fits left-out items; 0 turns it off.
    pub fn separate_moves(mut self, n: usize) -> Self {
        self.separate_moves = n;
        self
    }

    /// Seed of the search.
    pub fn seed(mut self, seed: u64) -> Self {
        self.seed = seed;
        self
    }

    /// Ratio beyond which text frames are never tried.
    pub fn max_aspect(mut self, ratio: f64) -> Self {
        self.max_aspect = ratio;
        self
    }

    /// Weights of the score.
    pub fn weights(mut self, weights: Weights) -> Self {
        self.weights = weights;
        self
    }

    /// Style patterns to look for.
    pub fn rules(mut self, rules: Rules) -> Self {
        self.rules = rules;
        self
    }
}

impl Default for SolveOptions {
    fn default() -> Self {
        Self {
            beam_width: 16,
            branching: 8,
            alternatives: 3,
            slack: DEFAULT_SLACK.to_vec(),
            refine_iterations: 400,
            separate_moves: 6000,
            seed: 1,
            max_aspect: 6.0,
            weights: Weights::default(),
            rules: Rules::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
/// Why an item was left out.
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum Reason {
    /// The measurements say the item cannot fit at any column span
    NoShape,
    /// It fits somewhere in principle, but no free place was left
    NoSpace,
}

/// An item that could not be placed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Unplaced {
    /// The item.
    pub id: String,
    /// Why.
    pub reason: Reason,
}

/// Another layout worth looking at.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Alternative {
    /// The placements.
    pub layout: Layout,
    /// Its score.
    pub score: ScoreBreakdown,
    /// Its style warnings.
    pub warnings: Vec<Violation>,
}

/// Not finding a place for an item is part of the result, not an error.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Solution {
    /// The best layout found.
    pub layout: Layout,
    /// Items that were left out.
    pub unplaced: Vec<Unplaced>,
    /// The score of the best layout.
    pub score: ScoreBreakdown,
    /// Style warnings of the best layout.
    pub warnings: Vec<Violation>,
    /// Other good layouts, best first.
    pub alternatives: Vec<Alternative>,
}

/// Something that turns a problem and its measurements into a solution.
pub trait Solver {
    /// Solves `problem`, or says why it is malformed.
    fn solve(&self, problem: &Problem, table: &MeasurementTable) -> Result<Solution, ProblemError>;
}
