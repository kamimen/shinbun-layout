//! Data model, geometry and validation. No solving happens here, so anything that produces layouts
//! (the solvers, a GUI, a hand-written file) can be checked with the same rules.

pub mod geometry;
pub mod model;
pub mod validate;

pub use model::*;
pub use validate::{validate, Pattern, Rules, Violation};
