//! F53 `book::merge_candidates` and F54 `book::SlotAllocator` (PREVIEW-SPEC section 3.6).
//!
//! Candidates are taken in the order (instant, symbol byte order); a candidate is ignored
//! if its stock holds a position ("signal while open"), holds no slot when it cannot
//! fill (Q6: entry not filled, zero quantity, no ratio), is skipped when 20 positions are
//! open ("slots full"), else admitted. A position holds its slot from its signal instant
//! to its exit fill second, inclusive.

use std::collections::BTreeMap;

use crate::cond::Side;
use crate::sizing::SLOTS;
use crate::time::Day;
use crate::trade::Outcome;

/// One signal's candidate: its key and its portfolio-free outcome.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Candidate {
    /// The signal's day.
    pub day: Day,
    /// The signal candle's close instant T.
    pub t: u32,
    /// The stock's spelling.
    pub symbol: String,
    /// The side.
    pub side: Side,
    /// What its would-be trade did.
    pub outcome: Outcome,
}

impl Candidate {
    /// The merge key: (day, T, symbol bytes).
    #[must_use]
    pub fn key(&self) -> (Day, u32, &[u8]) {
        (self.day, self.t, self.symbol.as_bytes())
    }

    /// The exit instant (day, second) of a filled candidate.
    #[must_use]
    pub const fn exit_at(&self) -> Option<(Day, u32)> {
        match self.outcome {
            Outcome::Filled(s) => Some((self.day, s.exit.second)),
            _ => None,
        }
    }
}

/// Which candidates a book takes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum View {
    /// Long candidates only.
    Long,
    /// Short candidates only.
    Short,
    /// Both sides in one book, sharing the slots and the one-position-per-stock rule.
    Both,
}

impl View {
    /// The three accounts, in report order.
    pub const ALL: [Self; 3] = [Self::Long, Self::Short, Self::Both];

    /// The view takes `side`.
    #[must_use]
    pub fn takes(self, side: Side) -> bool {
        match self {
            Self::Long => side == Side::Long,
            Self::Short => side == Side::Short,
            Self::Both => true,
        }
    }

    /// `LONG`, `SHORT` or `BOTH`.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Long => "LONG",
            Self::Short => "SHORT",
            Self::Both => "BOTH",
        }
    }
}

/// Why a merge was refused: one key twice (a bug upstream).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DuplicateKey {
    /// The day.
    pub day: Day,
    /// The instant.
    pub t: u32,
    /// The symbol.
    pub symbol: String,
}

/// F53: every stock's candidates the view takes, in the order (day, T, symbol bytes).
///
/// # Errors
/// [`DuplicateKey`] when one (day, T, symbol) appears twice.
pub fn merge(per_stock: &[Vec<Candidate>], view: View) -> Result<Vec<Candidate>, DuplicateKey> {
    let mut out: Vec<Candidate> = per_stock.iter().flatten().filter(|c| view.takes(c.side)).cloned().collect();
    out.sort_by(|a, b| a.key().cmp(&b.key()));
    for (a, b) in out.iter().zip(out.iter().skip(1)) {
        if a.key() == b.key() {
            return Err(DuplicateKey { day: b.day, t: b.t, symbol: b.symbol.clone() });
        }
    }
    Ok(out)
}

/// What the allocator did with one candidate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Admission {
    /// Admitted: it holds a slot until its exit second.
    Admitted,
    /// Its stock already holds a position.
    WhileOpen,
    /// It cannot fill and holds no slot.
    Unfillable,
    /// Twenty positions are open.
    SlotsFull,
}

/// F54: the 20 slots of one book.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Slots {
    open: BTreeMap<String, (Day, u32)>,
}

impl Slots {
    /// Positions open at the instant (day, t): every exit at or after it.
    fn release(&mut self, day: Day, t: u32) {
        self.open.retain(|_, exit| *exit >= (day, t));
    }

    /// Offers one candidate at its instant.
    pub fn offer(&mut self, c: &Candidate) -> Admission {
        self.release(c.day, c.t);
        if self.open.contains_key(&c.symbol) {
            return Admission::WhileOpen;
        }
        let Some(exit) = c.exit_at() else {
            return Admission::Unfillable;
        };
        if self.open.len() >= SLOTS {
            return Admission::SlotsFull;
        }
        self.open.insert(c.symbol.clone(), exit);
        Admission::Admitted
    }

    /// Positions open now.
    #[must_use]
    pub fn len(&self) -> usize {
        self.open.len()
    }

    /// No position open.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.open.is_empty()
    }
}

#[cfg(test)]
#[path = "book_tests.rs"]
mod tests;
