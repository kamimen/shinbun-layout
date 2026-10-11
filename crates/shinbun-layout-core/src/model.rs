use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Writing {
    #[default]
    Vertical,
    Horizontal,
}

/// The page grid of shinbun-css: `dan` columns (段) of `chars` characters, and `lines` lines along the line axis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Paper {
    #[serde(default)]
    pub writing: Writing,
    pub dan: u8,
    pub lines: u8,
    #[serde(default = "default_chars")]
    pub chars: u8,
}

fn default_chars() -> u8 {
    10
}

/// A rectangle on the grid, with the same names and meaning as shinbun-css and shinbun-maker:
/// `x`/`w` run along the line axis (from the right in vertical writing), `y`/`span` along the columns.
/// All values start at 1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Placement {
    pub x: u8,
    pub w: u8,
    pub y: u8,
    pub span: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    Nameplate,
    Ad,
    Yoko,
    Photo,
    Box,
    Article,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Headline {
    Xl,
    L,
    M,
    S,
}

fn default_priority() -> u8 {
    5
}

/// Something to place. The text itself is not needed: the host measures how many lines it needs per column span.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Item {
    pub id: String,
    pub role: Role,
    /// 1 (minor) to 10 (lead story)
    #[serde(default = "default_priority")]
    pub priority: u8,
    /// Length of the text, used to estimate a good size
    #[serde(default)]
    pub chars: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub headline: Option<Headline>,
    #[serde(default)]
    pub lead: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_span: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_span: Option<u8>,
    /// A fixed item keeps this placement
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fixed: Option<Placement>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Problem {
    pub paper: Paper,
    pub items: Vec<Item>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProblemError {
    Empty,
    GridTooLarge { dan: u8, lines: u8 },
    DuplicateId(String),
    PriorityOutOfRange(String),
    FixedOutOfBounds(String),
    FixedOverlap(String, String),
    BadSpanRange(String),
}

impl fmt::Display for ProblemError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => write!(f, "the paper has no columns or lines"),
            Self::GridTooLarge { dan, lines } => {
                write!(f, "the grid {dan} columns x {lines} lines is larger than 15 x 60")
            }
            Self::DuplicateId(id) => write!(f, "item id {id:?} is used twice"),
            Self::PriorityOutOfRange(id) => write!(f, "item {id:?}: priority must be 1 to 10"),
            Self::FixedOutOfBounds(id) => write!(f, "item {id:?}: the fixed placement is outside the paper"),
            Self::FixedOverlap(a, b) => write!(f, "fixed items {a:?} and {b:?} overlap"),
            Self::BadSpanRange(id) => write!(f, "item {id:?}: min_span is larger than max_span"),
        }
    }
}

impl std::error::Error for ProblemError {}

impl Problem {
    /// Rejects input that cannot be a problem at all. An infeasible but well-formed problem is not an error.
    pub fn check(&self) -> Result<(), ProblemError> {
        use crate::geometry::{in_bounds, overlaps};
        let p = &self.paper;
        if p.dan == 0 || p.lines == 0 || p.chars == 0 {
            return Err(ProblemError::Empty);
        }
        if p.dan > 15 || p.lines > 60 {
            return Err(ProblemError::GridTooLarge {
                dan: p.dan,
                lines: p.lines,
            });
        }
        let mut seen = std::collections::HashSet::new();
        for it in &self.items {
            if !seen.insert(it.id.as_str()) {
                return Err(ProblemError::DuplicateId(it.id.clone()));
            }
            if !(1..=10).contains(&it.priority) {
                return Err(ProblemError::PriorityOutOfRange(it.id.clone()));
            }
            if let (Some(lo), Some(hi)) = (it.min_span, it.max_span) {
                if lo > hi {
                    return Err(ProblemError::BadSpanRange(it.id.clone()));
                }
            }
            if let Some(at) = &it.fixed {
                if !in_bounds(p, at) {
                    return Err(ProblemError::FixedOutOfBounds(it.id.clone()));
                }
            }
        }
        let fixed: Vec<_> = self
            .items
            .iter()
            .filter_map(|i| i.fixed.map(|a| (&i.id, a)))
            .collect();
        for (i, (a, pa)) in fixed.iter().enumerate() {
            for (b, pb) in &fixed[i + 1..] {
                if overlaps(pa, pb) {
                    return Err(ProblemError::FixedOverlap((*a).clone(), (*b).clone()));
                }
            }
        }
        Ok(())
    }

    pub fn item(&self, id: &str) -> Option<&Item> {
        self.items.iter().find(|i| i.id == id)
    }
}

/// One question for the host: how many lines does this item need if it spans `span` columns?
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MeasureQuery {
    pub id: String,
    pub span: u8,
}

/// The answer. `lines` is the smallest line count that fits, or `None` if it cannot fit at this span.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Measure {
    pub id: String,
    pub span: u8,
    pub lines: Option<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lookup {
    Needs(u8),
    CannotFit,
    Missing,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(from = "Vec<Measure>", into = "Vec<Measure>")]
pub struct MeasurementTable {
    map: HashMap<(String, u8), Option<u8>>,
}

impl From<Vec<Measure>> for MeasurementTable {
    fn from(entries: Vec<Measure>) -> Self {
        Self {
            map: entries.into_iter().map(|m| ((m.id, m.span), m.lines)).collect(),
        }
    }
}

impl From<MeasurementTable> for Vec<Measure> {
    fn from(t: MeasurementTable) -> Self {
        let mut v: Vec<Measure> = t
            .map
            .into_iter()
            .map(|((id, span), lines)| Measure { id, span, lines })
            .collect();
        v.sort_by(|a, b| (&a.id, a.span).cmp(&(&b.id, b.span)));
        v
    }
}

impl MeasurementTable {
    pub fn get(&self, id: &str, span: u8) -> Lookup {
        match self.map.get(&(id.to_owned(), span)) {
            Some(Some(n)) => Lookup::Needs(*n),
            Some(None) => Lookup::CannotFit,
            None => Lookup::Missing,
        }
    }

    pub fn insert(&mut self, id: &str, span: u8, lines: Option<u8>) {
        self.map.insert((id.to_owned(), span), lines);
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Placed {
    pub id: String,
    #[serde(flatten)]
    pub at: Placement,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Layout {
    pub placements: Vec<Placed>,
}

impl Layout {
    pub fn get(&self, id: &str) -> Option<&Placement> {
        self.placements.iter().find(|p| p.id == id).map(|p| &p.at)
    }
}

impl Role {
    /// Whether the item holds running text, so its size follows from the text rather than from taste.
    pub fn has_text(self) -> bool {
        matches!(self, Role::Article | Role::Box)
    }
}
