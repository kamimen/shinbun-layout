use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

/// Direction of the text on the page.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Writing {
    /// 縦組み: lines run top to bottom and follow each other from the right.
    #[default]
    Vertical,
    /// 横組み: lines run left to right and follow each other from the top.
    Horizontal,
}

/// The page grid of shinbun-css: `dan` columns (段) of `chars` characters, and `lines` lines along the line axis.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Paper {
    /// Vertical or horizontal; vertical when omitted.
    #[serde(default)]
    pub writing: Writing,
    /// Number of columns, 1 to 15.
    pub dan: u8,
    /// Number of lines along the line axis, 1 to 60.
    pub lines: u8,
    /// Characters in one column; 10 when omitted.
    #[serde(default = "default_chars")]
    pub chars: u8,
    /// Length of one column in character sizes when it is not `chars` (horizontal pages are as wide as the paper)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub column_em: Option<f64>,
    /// Space between columns in character sizes; 1 in shinbun-css
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gap_em: Option<f64>,
}

impl Paper {
    /// A paper of `dan` columns and `lines` lines, with `chars` characters in a column.
    pub fn new(writing: Writing, dan: u8, lines: u8, chars: u8) -> Self {
        Self {
            writing,
            dan,
            lines,
            chars,
            column_em: None,
            gap_em: None,
        }
    }

    /// Sets the measured length of a column and the gap between columns, both in character sizes.
    pub fn geometry(mut self, column_em: f64, gap_em: f64) -> Self {
        self.column_em = Some(column_em);
        self.gap_em = Some(gap_em);
        self
    }

    /// Length in character sizes of a frame that spans `span` columns, along the columns.
    pub fn length_em(&self, span: u8) -> f64 {
        let column = self.column_em.unwrap_or(self.chars as f64);
        let gap = self.gap_em.unwrap_or(1.0);
        span as f64 * column + span.saturating_sub(1) as f64 * gap
    }
}

fn default_chars() -> u8 {
    10
}

/// A rectangle on the grid, with the same names and meaning as shinbun-css and shinbun-maker:
/// `x`/`w` run along the line axis (from the right in vertical writing), `y`/`span` along the columns.
/// All values start at 1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Placement {
    /// First line of the frame along the line axis.
    pub x: u8,
    /// Number of lines.
    pub w: u8,
    /// First column from the top.
    pub y: u8,
    /// Number of columns.
    pub span: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
/// What an item is; it decides how the item is sized and where it likes to go.
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum Role {
    /// The title of the paper (題字).
    Nameplate,
    /// An advertisement.
    Ad,
    /// A headline set across the page in the other direction (横見出し).
    Yoko,
    /// A picture with an optional caption.
    Photo,
    /// A boxed text (囲み).
    Box,
    /// A story with a headline and running text.
    Article,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
/// Size of a headline.
#[serde(rename_all = "lowercase")]
pub enum Headline {
    /// The largest headline.
    Xl,
    /// Large.
    L,
    /// Medium.
    M,
    /// Small.
    S,
}

fn default_priority() -> u8 {
    5
}

/// Something to place. The text itself is not needed: the host measures how many lines it needs per column span.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Item {
    /// Unique within a problem.
    pub id: String,
    /// What kind of thing this is.
    pub role: Role,
    /// 1 (minor) to 10 (lead story)
    #[serde(default = "default_priority")]
    pub priority: u8,
    /// Length of the text, used to estimate a good size
    #[serde(default)]
    pub chars: u32,
    /// Size of the headline, for articles.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub headline: Option<Headline>,
    /// The lead story (トップ記事).
    #[serde(default)]
    pub lead: bool,
    /// Width over height of the picture, for photos: only shapes close to it are tried
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ratio: Option<f64>,
    /// The photo has a caption, which takes one line of the frame
    #[serde(default)]
    pub caption: bool,
    /// Fewest columns the item may span.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_span: Option<u8>,
    /// Most columns the item may span.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_span: Option<u8>,
    /// Where the item stays; the others are planned around it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fixed: Option<Placement>,
}

impl Item {
    /// An item with the default importance (5) and no text.
    pub fn new(id: impl Into<String>, role: Role) -> Self {
        Self {
            id: id.into(),
            role,
            priority: default_priority(),
            chars: 0,
            headline: None,
            lead: false,
            ratio: None,
            caption: false,
            min_span: None,
            max_span: None,
            fixed: None,
        }
    }

    /// Importance from 1 (minor) to 10 (lead story).
    pub fn priority(mut self, priority: u8) -> Self {
        self.priority = priority;
        self
    }

    /// Length of the text.
    pub fn chars(mut self, chars: u32) -> Self {
        self.chars = chars;
        self
    }

    /// Size of the headline.
    pub fn headline(mut self, headline: Headline) -> Self {
        self.headline = Some(headline);
        self
    }

    /// Marks the lead story.
    pub fn lead(mut self) -> Self {
        self.lead = true;
        self
    }

    /// Width over height of the picture.
    pub fn ratio(mut self, ratio: f64) -> Self {
        self.ratio = Some(ratio);
        self
    }

    /// The photo has a caption.
    pub fn caption(mut self) -> Self {
        self.caption = true;
        self
    }

    /// Limits the number of columns the item may span.
    pub fn span_range(mut self, min: u8, max: u8) -> Self {
        self.min_span = Some(min);
        self.max_span = Some(max);
        self
    }

    /// Keeps the item at `at`.
    pub fn fixed(mut self, at: Placement) -> Self {
        self.fixed = Some(at);
        self
    }
}

/// Everything the solver needs to know: the paper and the items.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Problem {
    /// The page grid.
    pub paper: Paper,
    /// Everything to place, fixed items included.
    pub items: Vec<Item>,
}

impl Problem {
    /// A problem of the items on the paper.
    pub fn new(paper: Paper, items: Vec<Item>) -> Self {
        Self { paper, items }
    }
}

/// Why a problem cannot be solved at all: the input is malformed, not merely infeasible.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ProblemError {
    /// The paper has no columns, lines or characters.
    Empty,
    /// The grid is larger than 15 columns by 60 lines.
    GridTooLarge {
        /// Columns asked for.
        dan: u8,
        /// Lines asked for.
        lines: u8,
    },
    /// Two items share an id.
    DuplicateId(String),
    /// The priority of the item is not between 1 and 10.
    PriorityOutOfRange(String),
    /// The fixed placement of the item is outside the paper.
    FixedOutOfBounds(String),
    /// Two fixed items overlap.
    FixedOverlap(String, String),
    /// `min_span` of the item is larger than `max_span`.
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

    /// The item with this id.
    pub fn item(&self, id: &str) -> Option<&Item> {
        self.items.iter().find(|i| i.id == id)
    }
}

/// One question for the host: how many lines does this item need if it spans `span` columns?
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MeasureQuery {
    /// The item.
    pub id: String,
    /// The number of columns.
    pub span: u8,
}

/// The answer. `lines` is the smallest line count that fits, or `None` if it cannot fit at this span.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Measure {
    /// The item.
    pub id: String,
    /// The number of columns.
    pub span: u8,
    /// The smallest number of lines, or `None` if the item cannot fit.
    pub lines: Option<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// What the table says about an item at a column count.
pub enum Lookup {
    /// The item needs this many lines.
    Needs(u8),
    /// The measurements say the item fits at no width.
    CannotFit,
    /// There is no measurement for this item and span.
    Missing,
}

/// The answers to the host's measurements, as a table from (item, span) to lines. It is a list of [`Measure`] in JSON.
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
    /// What the table says about `id` at `span` columns.
    pub fn get(&self, id: &str, span: u8) -> Lookup {
        match self.map.get(&(id.to_owned(), span)) {
            Some(Some(n)) => Lookup::Needs(*n),
            Some(None) => Lookup::CannotFit,
            None => Lookup::Missing,
        }
    }

    /// Records that `id` needs `lines` lines at `span` columns, or cannot fit (`None`).
    pub fn insert(&mut self, id: &str, span: u8, lines: Option<u8>) {
        self.map.insert((id.to_owned(), span), lines);
    }

    /// Number of answers.
    pub fn len(&self) -> usize {
        self.map.len()
    }

    /// Whether there are no answers.
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
/// One item and its place.
pub struct Placed {
    /// The item.
    pub id: String,
    /// Where it goes.
    #[serde(flatten)]
    pub at: Placement,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
/// Where every placed item goes.
pub struct Layout {
    /// Every placed item, fixed ones included.
    pub placements: Vec<Placed>,
}

impl Layout {
    /// The place of `id`, if it is placed.
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
