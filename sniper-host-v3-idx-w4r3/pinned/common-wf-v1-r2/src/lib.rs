//! `common_wf` crate v1: THE common walk-forward and train/test CORE of Brutex (REQ 72b; REQ 73 / 73d; REQ 76f: the sniper is its
//! first client). Plan: `state/common-wf-20261113/COMMON-WF-PLAN.md` v5 (sha256 16 `11136613fc95dcff`), signed by MAIN at
//! 2026-11-13 15:36:02 IST with the BINDING amendments A1-A4 (`MAIN-SIGNED-COMMON-WF-20261113.md`), which win over plan lines.
//!
//! | Module | Role (plan section) |
//! |---|---|
//! | [`date`] | IST session dates and month keys (P1) |
//! | [`cal`] | the dated session table, fail closed (P2) |
//! | [`windows`] | windows as DATA: rows, window sets, pieces, the per-window YEAR UNITS (DV1), validation V1-V14 (P3-P5, 4.1) |
//! | [`rules`] | RULES values, declared grids with signed provenance, partitions with per-value provenance (4.5, P21, P22, A2, A4) |
//! | [`dd`] | the drawdown monoid (lifted from evalcore `accum.rs:9-55`, checked) |
//! | [`acc`] | `Acc`: add / add_session_mtm / finish, O(1) a trade, prefix snapshots, the unit masks (6.1-6.2) |
//! | [`views`] | views as data, E1-E7, K1-K17, keys, tickets, `Lists` and `offer` (P13-P18, P21, 6.3) |
//! | [`verdict`] | P10 / P11: the verdict of a window row on a pick, REQ 74's replays (6.4) |
//! | [`words`] | `WORDS-STD-v1.tsv`, V-W1, `fmt_rs` (lifted from round 3's `common.rs:131-148`) (4.3, section 7) |
//! | [`report`] | every printed line: readings, the pick label, ties, margin, misses, the luck line, the per-search method line (P17-P19, P23) |
//! | [`rows`] | DB rows W1-W9 with frozen field orders, the hash-chained W8 audit, the MANIFEST, SHA-256 (section 13) |
//!
//! Rules: std only, no dependency, no I/O, no float; money in i64 paisa with checked arithmetic (a refusal, never a wrap). The
//! cargo feature `red` swaps in the most lenient reading through the same API (the red round); a host refuses a headline or search
//! run built with it ([`refuse_if_red`]) or under any legacy profile ([`refuse_if_legacy`]). Legacy profiles and lane migrations
//! are NOT in this build (MAIN's signature, scope item 2): [`Profile`] names them only so the refusal is typed.
#![allow(clippy::doc_markdown, reason = "the docs quote the plan's own names (REQ 73, P21, W27, ...) and IST instants, not code")]

pub mod acc;
pub mod cal;
pub mod date;
pub mod dd;
pub mod report;
pub mod rows;
pub mod rules;
pub mod verdict;
pub mod views;
pub mod windows;
pub mod words;
#[cfg(test)]
mod tests_props;
#[cfg(test)]
mod tests_repair;
#[cfg(test)]
mod tests_repair2;
#[cfg(test)]
mod tests_vec;

use std::fmt;

pub use acc::{Acc, Facts, Learn, MonthCell, Steps, Trade, UnitCell};
pub use cal::{CalRow, Calendar, DayKind};
pub use rules::{DeclaredGrid, GridRole, HoleSource, MtmMarks, PartitionDef, Provenance, RuleProvenance, RunRules, TradeOrder};
pub use verdict::{NotRun, PickKind, PickWin, Reading, Verdict, verdict, verdict_with_replays};
pub use views::{Code, Counts, Key, Kept, Lists, Ticket, Tickets, Views};
pub use windows::{WindowSets, Windows};
pub use words::Words;

/// Money: i64 paisa, net after every charge (P1).
pub type Paisa = i64;
/// An IST session date: days since 1970-01-01.
pub type Day = i32;
/// A month key: year * 12 + (month - 1).
pub type Mk = i32;
/// Pieces per window set (P5): bits of a u32.
pub const MAX_PIECES: usize = 32;
/// Window sets per run (P5, F7).
pub const MAX_SETS: usize = 8;
/// Year units per window set (P5, DV1): bits of a u64.
pub const MAX_PARTS: usize = 64;
/// Criteria a view's key may hold (4.2).
pub const MAX_KEY: usize = 12;
/// Distinct K13 bars a views file may hold (B5 (c)).
pub const MAX_BARS: usize = 4;
/// Distinct E4 floors a views file may hold (one unit mask each, so E4 stays one mask operation).
pub const MAX_FLOORS: usize = 8;
/// Tickets (paths) an offer carries (P21, Y6).
pub const MAX_TICKETS: usize = 4;
/// Windows of one set (a u64 mask over the set's windows).
pub const MAX_SET_WINDOWS: usize = 64;
/// The ticket grid index of a ticket with no declared grid (P21, S1).
pub const NO_GRID: u8 = u8::MAX;
/// The ticket stage of a path with no shaping stage of this run.
pub const NO_STAGE: u8 = u8::MAX;
/// The crate version printed in every identity line.
pub const VERSION: &str = "common_wf 1.0.0";
/// The red-first switch (plan section 3): true only in a `--features red` build.
pub const RED: bool = cfg!(feature = "red");
/// A named-mutant build (plan 11.4): never a headline or search run.
pub const MUTANT: bool = cfg!(any(feature = "m1", feature = "m3", feature = "m4", feature = "m5", feature = "m7", feature = "m10", feature = "m11", feature = "m16", feature = "m17", feature = "m22", feature = "m26", feature = "m27", feature = "m28", feature = "m29", feature = "m30", feature = "m34", feature = "m36", feature = "m37", feature = "m38", feature = "m42", feature = "m44", feature = "m45", feature = "m46", feature = "m47", feature = "m48", feature = "m49", feature = "m50", feature = "m51", feature = "m52", feature = "m53"));

/// A u64 mask over a window set's year units (P5, DV1).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct PartMask(pub u64);
/// A u32 mask over a window set's pieces.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct PieceMask(pub u32);
/// A window row: its index in the WINDOWS file.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WinId(pub u8);
/// A view: its index in the VIEWS file.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ViewId(pub u16);
/// A partition id the host gives (P22): one SEARCH per real partition (MAIN 14:31:36 IST, A4).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Partition(pub u32);
/// A window set (P5, F7, R5).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SetId(pub u8);

/// The hull of every month whose outcomes or features a path read (P21); EMPTY = from > to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ReadSet {
    /// The first month read.
    pub from: Mk,
    /// The last month read.
    pub to: Mk,
}
impl ReadSet {
    /// The empty read set: no month's outcome or feature was read (a LATTICE grid with no shaping stage).
    pub const EMPTY: Self = Self { from: 1, to: 0 };
    /// Is it the empty set?
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.from > self.to
    }
    /// The hull of two read sets.
    #[must_use]
    pub fn hull(self, b: Self) -> Self {
        if self.is_empty() {
            b
        } else if b.is_empty() {
            self
        } else {
            Self { from: self.from.min(b.from), to: self.to.max(b.to) }
        }
    }
    /// Does `self` contain `b` (an empty `b` is contained in every set)?
    #[must_use]
    pub const fn contains(&self, b: &Self) -> bool {
        if b.is_empty() {
            true
        } else if self.is_empty() {
            false
        } else {
            self.from <= b.from && b.to <= self.to
        }
    }
}

/// Every refusal the crate names (plan section 5, U8), plus the builder's additions and the repair's (CHANGES.md).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WfErr {
    /// A checked sum overflowed (P1): the run is refused, never wrapped.
    Overflow,
    /// More than 32 pieces in one window set.
    TooManyPieces,
    /// More than 64 year units in one window set.
    TooManyParts,
    /// More than 8 window sets.
    TooManySets,
    /// An `Acc`, `Facts` or window of another window set (R5).
    SetMismatch,
    /// A trade or mark out of the run's order (P6).
    OutOfOrder,
    /// A trade dated outside the period or the subject's data span (P6).
    TradeOutsidePeriod,
    /// A trade on a session that is not walked (P6).
    TradeOnUnwalkedSession,
    /// A month-fed `Acc` offered to a view reading K5, K6 or K7 (F9).
    NeedsTrades,
    /// A hole trade with `hole_source = none` (P7).
    HoleWithoutSource,
    /// An `@alt` reading tag in a standard views file (P15).
    AltInStandard,
    /// A key that would read a test month (P20).
    KeyReadsTest,
    /// A 1-member LATTICE grid without a MAIN line (P21).
    OneMemberLattice,
    /// A ticket of a FIXED grid offered (P13, BX2).
    FixedOffered,
    /// A fifth non-dominated ticket (Y6).
    TooManyTickets,
    /// A session-end mark with `mtm_marks = none` (R3).
    MarksWithoutRule,
    /// A mark on an unwalked session (X1).
    MarkOnUnwalkedSession,
    /// A second offer of one code in one (Lists, partition) (F3; feature `offer_once_check`).
    DoubleOffer,
    /// A headline or search door of a `red` build.
    RedBuild,
    /// A headline under a legacy profile (OB30).
    LegacyHeadline,
    /// A merge or INC across identities (F12).
    IdentityMismatch,
    /// The list bound above the lane's guard (F16).
    ListsTooBig,
    /// A PAPER reading of a month before RULES `paper_from` (Z2).
    PaperSeen,
    /// RULES `paper_from` not later than the signing month and every CHOSEN read set (Z2, F3).
    PaperFromSeen,
    /// A data-file row refused by its rule: file W(indows) / V(iews) / T (words) / C(alendar) / R(ules) / P(artitions).
    BadRow {
        /// The file letter as an ASCII byte.
        file: u8,
        /// The 1-based data row (0 = the file as a whole).
        row: u32,
        /// The rule number (V1-V14, 4.2's rules, V-W1, ...).
        rule: u8,
    },
    /// A PASSED line without its proof step or data labels (F15, D41) [builder's addition: the plan names the refusal, no member].
    ProofMissing,
    /// A pick whose partition has no W9 row (Y4, OB54) [builder's addition].
    NoPartitionRow,
    /// A method line over picks of two searches (MAIN 14:31:36 IST: never across searches) [builder's addition]: an item whose
    /// partition is not the line's partition (repair of check-build-v1 B1).
    SearchMismatch,
    /// A date or month that does not parse.
    BadDay,
    /// A method line whose items are not exactly ONE item per honest row in use of the run's own window sets (no row missing,
    /// none twice, none of another row or of a row not in use), an item whose view is not its row's HEADLINE view, window sets
    /// that do not name every honest row of the file exactly once (a set doubled or dropped), or an item word and pick code that
    /// disagree (a word but "no pick" without a code, "no pick" with one) (repair of check-build-v1 B1 and of check-build-v1-r2
    /// B1-R; REQ 73d; MAIN 14:31:36 IST).
    MethodItems,
    /// A REAL partition value with no declared provenance: no `prov_<dim>`, no MAIN ruling and no signed RULES
    /// `partition_provenance_default` (A2; repair of check-build-v1 B2: fail closed, naming the partition and the dimension).
    NoPartitionProvenance {
        /// The partition id.
        partition: u32,
        /// The dimension (one of `rules::PROV_DIMS`).
        dim: &'static str,
    },
    /// A declared partition provenance that contradicts MAIN's standing ruling of that value (A2: ADANIENT alone is
    /// RULED_KNOWINGLY(2026-04); repair of check-build-v1 B2).
    RulingContradicted {
        /// The partition id.
        partition: u32,
        /// The dimension.
        dim: &'static str,
    },
    /// A carried trade (exit after its entry session) in a run whose RULES declare neither `carried=yes` nor session-end marks
    /// (P6, R3: every result of a run with a carried trade is labelled so; repair of check-build-v1 F2).
    CarriedUndeclared,
}
impl fmt::Display for WfErr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BadRow { file, row, rule } => write!(f, "BadRow({},{row},{rule})", char::from(*file)),
            Self::NoPartitionProvenance { partition, dim } => write!(f, "NoPartitionProvenance(p{partition},{dim})"),
            Self::RulingContradicted { partition, dim } => write!(f, "RulingContradicted(p{partition},{dim})"),
            other => write!(f, "{other:?}"),
        }
    }
}
impl std::error::Error for WfErr {}

/// The verdict profiles. Only `Standard` is built in v1; the legacy names exist so a headline under one is refused by type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Profile {
    /// THE standard (this plan).
    Standard,
    /// A lane's legacy reading (regression only; not built in v1).
    Legacy(&'static str),
}

/// Every host's headline and search door calls this first (the red runtime guard).
///
/// # Errors
/// `RedBuild` in a `red`, mutant or `offer_once_check` build.
pub const fn refuse_if_red() -> Result<(), WfErr> {
    if RED || MUTANT || cfg!(feature = "offer_once_check") { Err(WfErr::RedBuild) } else { Ok(()) }
}

/// A headline under any legacy profile is refused (OB30).
///
/// # Errors
/// `LegacyHeadline` for every profile but `Standard`.
pub const fn refuse_if_legacy(p: Profile) -> Result<(), WfErr> {
    match p {
        Profile::Standard => Ok(()),
        Profile::Legacy(_) => Err(WfErr::LegacyHeadline),
    }
}

/// Checked i64 addition (P1).
#[inline]
pub(crate) fn add(a: i64, b: i64) -> Result<i64, WfErr> {
    a.checked_add(b).ok_or(WfErr::Overflow)
}
/// Checked i64 subtraction (P1).
#[inline]
pub(crate) fn sub(a: i64, b: i64) -> Result<i64, WfErr> {
    a.checked_sub(b).ok_or(WfErr::Overflow)
}
/// The u64 count as an i64 key part (saturating at i64::MAX, a count never reaches it).
#[inline]
pub(crate) fn as_i64(x: u64) -> i64 {
    i64::try_from(x).unwrap_or(i64::MAX)
}
/// The 64-bit mixer of the order-free learning fingerprint (P18, S5) and of identity hashes.
#[inline]
pub(crate) const fn mix(mut x: u64) -> u64 {
    x = (x ^ (x >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    x ^ (x >> 31)
}
/// The FNV-1a hash of a text (identity hashes of the data files).
pub(crate) fn text_hash(s: &str) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in s.bytes() {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    h
}
