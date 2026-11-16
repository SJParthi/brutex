//! CRATE v2.1 (main 5b507203, 2026-11-14; SNIPER-IDXOPT-PLAN v2 s9.2 N1, MAIN-SIGNED-IDXOPT-20261114 Q10 / Q20 / C11): the FROZEN crate v2
//! (`scratch/common-fill-v2`, SHA256SUMS sha256 e3b911350f56a5f9...) plus three numbered changes, each with its red tests first
//! (`CHANGES-v21.md`):
//!
//! - C9, the BLOCK-LENGTH DIAL of REQ 73b (operator 2026-11-13 10:26 IST: index options fill in ONE-second blocks; promoted from the idx1s
//!   lane build `scratch/common-fill-v2c9-idx1s`, sha16 16457d879b983bc6): [`Tape::with_block`] sets a tape's block length; every block
//!   read of the walk family follows it (tests_c9.rs).
//! - C10, the gap tables ([`GapTables`], one lattice residue per second of the block), the paired legs ([`legs::walk_pair`]) and the
//!   shared-volume ledger ([`walk_ledger`], [`Ledger::open_blk`]) on EVERY block length, so a one-second walk keeps its O(1) gap skips and
//!   EVERY-mode re-entries run (tests_c10.rs).
//! - C11, [`limits::size_units_closest`]: SNIPER s16's CLOSEST whole count (exactly half way the lower; 0, refused, when one unit costs
//!   twice the target or more; the free-cash cap), beside the unchanged floor [`limits::size_units`] (tests_c11.rs).
//!
//! At the default 2-second block (and with `size_units` for `qty_rule = floor`) every reading is crate v2's, byte for byte: crate v2's
//! 70 tests, its vectors and its matrix pass unchanged (G-CHG). [`VERSION`] stays "v2" (the RULES `fill_crate_version` hosts check and
//! crate v2's own test pin); [`RELEASE`] = "v2.1" and [`CHANGES_SINCE_V2`] name this build beside the sha.
//!
//! `common_fill` crate v2, THE STANDARD: the common fill module of Brutex (design `state/common-fill-20261112/COMMON-FILL-STANDARD-DESIGN.md`
//! v2, sha256 `99ec39e8f95be697`; MAIN's signature D1-D30, `MAIN-SIGNED-COMMON-FILL-20261112.md`, 2026-11-12 23:10:48 IST).
//!
//! Crate v2 = crate v1 (THE LIFT of sample-6's checked `req70-2s`, frozen `cdc5315052598f2f`, gate G1) + section 4's API completed with no
//! reading changed (step A0) + the numbered changes C1-C8 of design 9.1, each applied ALONE, with its MAIN date, its vectors, its red
//! test first and its G-CHG equality against the step before (`CHANGES.md`).
//!
//! | Module | Role (design section 4) |
//! |---|---|
//! | [`tape`] | one instrument-day: rows, the O(1) second index, the O(1) range table, one block read, the view [`Tape`] |
//! | [`gaps`] | the gap-skip tables of 4.3 (zero-capacity runs, runs past the protection limit), per lattice parity |
//! | [`std70`] | THE STANDARD's walk: every masked level in ONE pass; [`first_fill`]; [`capacity_between`]; [`FillPath`] |
//! | [`limits`] | sizing and protection prices (S8, S11) |
//! | [`money`] | REQ 70a amounts and the event-ordered free cash (S12) |
//! | [`legs`] | hedged option pairs (S18, D14) |
//! | [`path`] | run-up / run-down; REQ 71's rules and path facts (S16, S19) |
//! | [`law`] | the headline and each comparison behind one call |
//! | [`cmp`] | the ONE old-rule engine kept in the crate: the per-second REQ 58b core (the red reading) |
//! | [`rules`] | the fail-closed RULES parser of the `fill_*` rows |
//! | [`rows`] | the DB rows of section 12, frozen field orders (no I/O) |
//!
//! Rules: std only, no dependency, no I/O, no float. Prices are i64 paisa, instants UTC microseconds, units u64; every per-block sum is
//! checked (`checked_*`, a refusal never a wrap: the crate does not rely on `overflow-checks`, design S1 / OB7); run totals are i128.
//! The cargo feature `red` swaps the fill law for the old rule through the same API (the red round); a host refuses a headline or search
//! run built with it, or with any named mutant m1-m11 ([`refuse_if_red`], [`RED`]).
#![allow(clippy::doc_markdown, reason = "the docs quote the design's own names (REQ 70a, S12, C1, ...) and IST instants, not code")]
#![allow(clippy::many_single_char_names, clippy::similar_names, reason = "the lifted code keeps sample-6's variable names (i, at, b, k, l, h, ...) so it reads line by line against o5s_fill.rs")]

pub mod cmp;
pub mod gaps;
pub mod law;
pub mod ledger;
pub mod legs;
pub mod limits;
pub mod money;
pub mod path;
pub mod rows;
pub mod rules;
pub mod std70;
pub mod tape;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_g0;
#[cfg(test)]
mod tests_v2;
#[cfg(test)]
mod tests_c9;
#[cfg(test)]
mod tests_c10;
#[cfg(test)]
mod tests_c11;

pub use gaps::GapTables;
pub use ledger::Ledger;
pub use law::{Law, walk_law};
pub use path::runs;
pub use std70::{BlockFill, FillPath, Kind, LevelFill, Order, Recorder, Steps, Why, capacity_between, first_fill, walk, walk_ledger};
pub use tape::{Block, RangeMm, Row, SecIn, SecIndex, Seg, Tape, find_holes};

use std::fmt;

/// An instant: UTC microseconds since the Unix epoch (= `live_timing::Us`).
pub type Us = i64;
/// One second in microseconds.
pub const US_S: Us = 1_000_000;
/// A price or an amount in paisa.
pub type Paisa = i64;
/// Units: shares, or option units (lots x lot size).
pub type Units = u64;
/// REQ 70: one block is two seconds (the red build: 1 second, the old rule's per-second step; mutant m1: 3 seconds).
pub const BLOCK_S: i64 = if cfg!(feature = "red") {
    1
} else if cfg!(feature = "m1") {
    3
} else {
    2
};
/// One block in microseconds (the default block of every [`Tape`]; CHANGE C9: [`Tape::with_block`] sets another length per tape).
pub const BLOCK_US: Us = BLOCK_S * US_S;
/// CHANGE C9 (REQ 73b): the longest block length [`Tape::with_block`] accepts, whole seconds (the dial REQ 73b names is 1 s and 2 s).
pub const BLOCK_MAX_S: i64 = 60;
/// CHANGE C9 (REQ 73b): the one-second law's name (index options: the block is ONE second from the deciding candle's end).
pub const LAW_1S: &str = "req73b-1s";

/// CHANGE C9: the law name of a block length (1 s: [`LAW_1S`], 2 s: [`LAW_STD`], any other length: "req70-blk", unnamed by the operator).
#[must_use]
pub const fn law_of_block(secs: i64) -> &'static str {
    match secs {
        1 => LAW_1S,
        2 => LAW_STD,
        _ => "req70-blk",
    }
}
/// REQ 70b: at most this many share levels in one pass (sample-6's `R70_MAXL`).
pub const MAXL: usize = 8;
/// The law's name (THE STANDARD).
pub const LAW_STD: &str = "req70-2s";
/// The with-money-cap comparison's name.
pub const LAW_STD_CAP: &str = "req70-2s-cap";
/// This crate's version: "v2" = THE STANDARD (crate v1 was THE LIFT, never a headline, D28).
pub const VERSION: &str = "v2";
/// CRATE v2.1: this build's release name (hosts print it beside the sha and [`VERSION`]).
pub const RELEASE: &str = "v2.1";
/// CRATE v2.1: the numbered changes this build carries beyond the frozen crate v2 (its readings at the default 2-second block and with
/// [`limits::size_units`] are crate v2's: [`VERSION`] stays "v2", the RULES `fill_crate_version` hosts check). Hosts print it beside the sha.
pub const CHANGES_SINCE_V2: &str = "C9+C10+C11";
/// The idx1s lane build's name for [`CHANGES_SINCE_V2`] (kept so a lane host reads the same constant; its value now names C10 and C11 too).
pub const LANE_CHANGES: &str = CHANGES_SINCE_V2;
/// The step of crate v2 these sources carry (A0, C1 ... C8, then G0 = the frozen crate v2: C1-C8 and the G0 repairs, CHANGES.md; then the
/// v2.1 changes C9-C11, CHANGES-v21.md): printed by hosts beside the sha (design 9.1: each change a new run id).
pub const STEP: &str = "G0+C9+C10+C11";
/// True when the crate is built with the `red` feature or any named mutant: hosts refuse headline and search runs then.
pub const RED: bool = cfg!(any(
    feature = "red",
    feature = "m1",
    feature = "m2",
    feature = "m3",
    feature = "m4",
    feature = "m5",
    feature = "m6",
    feature = "m7",
    feature = "m8",
    feature = "m9",
    feature = "m10",
    feature = "m11"
));

/// The law this build carries: "std70" (green), "red" (the old rule) or "mutant" (a named mutant m1-m11). Every host prints it as
/// `law_compiled` (design S21, T1).
#[must_use]
pub const fn law_compiled() -> &'static str {
    if cfg!(feature = "red") {
        "red"
    } else if RED {
        "mutant"
    } else {
        "std70"
    }
}

/// Every host's headline and search door calls this first (design section 3, critic B B3): a red or mutant build refuses.
///
/// # Errors
/// [`FillErr::RedBuild`] when the crate was built with the `red` feature or a named mutant.
pub const fn refuse_if_red() -> Result<(), FillErr> {
    if RED { Err(FillErr::RedBuild) } else { Ok(()) }
}

/// A side of one order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Side {
    /// A buy: fills at a block's highest HIGH.
    Buy,
    /// A sale: fills at a block's lowest LOW.
    Sell,
}

impl Side {
    /// The other side.
    #[must_use]
    pub const fn other(self) -> Self {
        match self {
            Self::Buy => Self::Sell,
            Self::Sell => Self::Buy,
        }
    }
}

/// The GLOBAL dial levels one call works on (bit l = dial level l). `want`, `out` and the record are always indexed by the GLOBAL level,
/// [`MAXL`] wide, so a host that walks a group of levels can never mix levels up (design 4.1, critic B B2).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct LevelMask(pub u8);

impl LevelMask {
    /// The first `n` levels (all of a dial of `n` levels; `n` above [`MAXL`] is held at [`MAXL`]).
    #[must_use]
    pub const fn all(n: usize) -> Self {
        if n >= MAXL { Self(u8::MAX) } else { Self((1_u8 << n) - 1) }
    }

    /// Level `l` alone (an index at or above [`MAXL`] gives an empty mask).
    #[must_use]
    pub const fn one(l: usize) -> Self {
        if l < MAXL { Self(1_u8 << l) } else { Self(0) }
    }

    /// Whether level `l` is in the mask.
    #[must_use]
    pub const fn has(self, l: usize) -> bool {
        l < MAXL && (self.0 >> l) & 1 == 1
    }
}

/// REQ 70b: the share dial, basis points of each block's traded volume, strictly increasing, each 1..=10,000, 1..=[`MAXL`] levels.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Dial {
    bp: [u16; MAXL],
    n: u8,
}

impl Dial {
    /// The operator's default dial (REQ 70b): 10 / 25 / 50 / 75 / 100%.
    pub const DEFAULT_BP: [u16; 5] = [1_000, 2_500, 5_000, 7_500, 10_000];

    /// A dial of the given levels.
    ///
    /// # Errors
    /// [`FillErr::DialInvalid`] unless 1..=[`MAXL`] levels, each 1..=10,000 bp, strictly increasing.
    pub fn new(levels: &[u16]) -> Result<Self, FillErr> {
        if levels.is_empty() || levels.len() > MAXL {
            return Err(FillErr::DialInvalid);
        }
        let mut bp = [0_u16; MAXL];
        let mut prev = 0_u16;
        for (slot, &b) in bp.iter_mut().zip(levels) {
            if b == 0 || b > 10_000 || b <= prev {
                return Err(FillErr::DialInvalid);
            }
            *slot = b;
            prev = b;
        }
        let n = u8::try_from(levels.len()).map_err(|_| FillErr::DialInvalid)?;
        Ok(Self { bp, n })
    }

    /// The number of levels.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.n as usize
    }

    /// Never true (a dial has at least one level); for clippy's `len_without_is_empty`.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.n == 0
    }

    /// The levels, bp.
    #[must_use]
    pub fn levels(&self) -> &[u16] {
        self.bp.get(..self.len()).unwrap_or(&[])
    }

    /// Level `l` (bp), if the dial has it.
    #[must_use]
    pub fn bp(&self, l: usize) -> Option<u16> {
        self.levels().get(l).copied()
    }
}

/// Every refusal of the crate, each named (design 4.1, S22: "every refusal is named and counted": the host counts them).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FillErr {
    /// A row stamp, an order's anchor or an index span start not on a whole second (S1, S4; OB4, OB9).
    NotWholeSecond,
    /// Rows not in time order (OB4).
    Unsorted,
    /// A tape built from another day's index, range table or gap tables, or more rows than a u32 index holds.
    IndexMismatch,
    /// A day index asked to span more than 4 days (never a day's index: a guard against an absurd span).
    SpanTooLong,
    /// A dial that does not rise, a level outside 1..=10,000 bp, too many levels, or a mask naming a level the dial does not have (OB22).
    DialInvalid,
    /// A checked sum or product left its type (OB7): refused, never wrapped.
    Overflow,
    /// A headline or search run asked of a `red` or mutant build (OB38).
    RedBuild,
    /// A row with volume and an invalid price (S2, change C2: the instrument-day is refused; OB6).
    BadPrint,
    /// An anchor inside the signal candle (REQ 56; OB9).
    InsideSignalCandle,
    /// A quantity that is not a whole number of lots, or a lot of 0 (OB8).
    LotMismatch,
    /// A stock option reaching the fill module on its own expiry day (S18, D15: the selector guard; OB18).
    StockOptionOnOwnExpiry,
    /// No traded second to size from (S11, `no_reference`).
    NoReference,
    /// Free cash below 0 handed to the money module (must never occur; OB25).
    NegativeFreeCash,
    /// A session table that is not sorted, overlaps, or is not on whole seconds (S3).
    SegmentsInvalid,
    /// A RULES row the fill parser does not know, a missing or repeated row, or a value outside its set (fail closed, design 3).
    RulesInvalid,
    /// Two legs of a pair with different anchors or units (S18); CHANGE C10: or on different block lengths.
    PairMismatch,
    /// CHANGE C9: a block length outside 1..=[`BLOCK_MAX_S`] whole seconds ([`Tape::with_block`]).
    BlockInvalid,
    /// CHANGE C9 / C10: gap tables handed to a tape of another block length than they were built on ([`Tape::with_gaps`]). (C9 also
    /// returned it from the paired legs and the shared-volume ledger at a length other than 2 s; C10 serves them at every length.)
    BlockNotServed,
}

impl fmt::Display for FillErr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::NotWholeSecond => "common_fill: a stamp or anchor not on a whole second (refused)",
            Self::Unsorted => "common_fill: rows not in time order (refused)",
            Self::IndexMismatch => "common_fill: a tape of another day's index, range table or gap tables (refused)",
            Self::SpanTooLong => "common_fill: a day index spanning more than 4 days (refused)",
            Self::DialInvalid => "common_fill: the share dial must rise inside 1..=10000 bp, at most 8 levels, masks inside the dial (refused)",
            Self::Overflow => "common_fill: a checked sum left its type (refused, never wrapped)",
            Self::RedBuild => "common_fill: built with the red feature or a named mutant: no headline or search run (refused)",
            Self::BadPrint => "common_fill: a row with volume and an invalid price (the instrument-day is refused, S2)",
            Self::InsideSignalCandle => "common_fill: an anchor inside the signal candle (REQ 56, refused)",
            Self::LotMismatch => "common_fill: a quantity that is not a whole number of lots (refused)",
            Self::StockOptionOnOwnExpiry => "common_fill: a stock option on its own expiry day (the selector guard, refused)",
            Self::NoReference => "common_fill: no traded second to size from (no_reference)",
            Self::NegativeFreeCash => "common_fill: free cash below 0 (must never occur, refused)",
            Self::SegmentsInvalid => "common_fill: a session table not sorted, overlapping or off whole seconds (refused)",
            Self::RulesInvalid => "common_fill: a fill RULES row unknown, missing, repeated or outside its set (refused)",
            Self::PairMismatch => "common_fill: two legs of a pair with different anchors or units (refused)",
            Self::BlockInvalid => "common_fill: a block length outside 1..=60 whole seconds (C9, refused)",
            Self::BlockNotServed => "common_fill: gap tables built on another block length than the tape's (C10, refused)",
        })
    }
}

impl std::error::Error for FillErr {}
