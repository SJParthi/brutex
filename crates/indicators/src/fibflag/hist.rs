//! The own-past histograms: every measured value placed in its stock's own
//! committed past, per direction, incrementally — a new day computes only
//! itself (FIB-2, PART A; D-6320 to D-6328).
//!
//! # What is here
//!
//! | Item | What | Decision |
//! |---|---|---|
//! | [`OwnPast`] | one histogram over the whole non-negative `i64` range: a constant bin table, a bin split by one more mantissa bit when a cut needs it | D-6320 |
//! | [`ObservedTick`], [`decision_tick`], [`TickCensus`] | the tick a price series shows: the mode of its committed days' gcds | D-6321 |
//! | [`Measure`], [`BankShape`], [`HistBank`], [`plan_width_passes`] | one stock's histograms, indexed by measure, direction, width, base kind, entry style, exit variant and touch level | D-6324 |
//! | [`OwnPast::encode`], [`ObservedTick::encode`], [`HistBank::encode`] | the checkpoint bodies: sparse, canonical, little-endian | D-6325 |
//! | [`GapDay`], [`silence_in_trade`] | the longest gap between seconds of a clean day, and the silence flag it feeds | D-6326 |
//! | [`step_bp`], [`touches`], [`OwnPast::censored_median`] | the touch step in basis points of the previous extreme, the exact touch test, and the censored duration median | D-6327 |
//!
//! # A day computes only itself, and never reads itself
//!
//! [`OwnPast::observe`] STAGES a value. Every answer — [`OwnPast::mid_rank`],
//! [`OwnPast::permille`], [`OwnPast::value_at_rank`] — reads only the counts
//! [`OwnPast::commit_day`] merged from earlier days, so a value observed today
//! never changes today's answers (`CLAUDE.md` §3 rule 7).
//! [`OwnPast::discard_day`] drops the staged values of a day that is not
//! tradable or that failed mid-way, and leaves the histogram as it was.
//!
//! # No typed warm-up
//!
//! A histogram with no committed day answers `no_own_past`, and one whose
//! committed days hold no value answers `no_observations`; there is no
//! default bucket and no minimum count. Every stock's own past starts at its
//! first stored full session, and the committed-day count
//! ([`OwnPast::committed_days`]) is stamped on each candidate as a reference.
//!
//! # Cost
//!
//! `observe`, `mid_rank` and `permille` are O(1): one bin computed from the
//! value's leading zeros and the next mantissa bits, one counter read or
//! written. `value_at_rank` is one bisection over the cumulative array: its
//! probes are bounded by the constant bin table (ten for the unsplit table).
//! `commit_day` is O(leaves) once per day. Proved by
//! `crates/indicators/tests/fib_hist.rs` and measured by
//! `crates/indicators/benches/fibflag.rs`.
//!
//! Integers only: counts are `u32` per bin and refused by name at their top,
//! never wrapped; totals and positions are `u64`; an answer is an exact
//! fraction ([`vocab::fib_cube::Ratio`]) or a value. This module does no I/O
//! and calls nothing that prints (gates 17 and 22): every refusal is counted
//! in a plain field.

use core::cell::Cell;
use core::mem::size_of;

use vocab::fib_cube::{
    BASE_KINDS, BP_PER_UNIT, CubeRefusal, HIST_CHECKPOINT_GROUP_BITS, HIST_SUB_BINS_PER_POWER,
    PERMILLE_PER_UNIT, Ratio, Search,
};

use super::types::BaseKind;

/// The mantissa bits below a value's top bit that pick its sub-bin: three for
/// the seed's eight sub-bins per power of two (R3).
pub const SUB_BITS: u32 = HIST_SUB_BINS_PER_POWER.trailing_zeros();

const _: () = assert!(HIST_SUB_BINS_PER_POWER.is_power_of_two());

/// The positions a non-negative `i64`'s top bit can take: every bit but the
/// sign. The table covers all of them, so no value overflows it.
pub const OCTAVES: u32 = i64::BITS - 1;

/// The constant bin table: bin 0 holds zero, then
/// [`HIST_SUB_BINS_PER_POWER`] sub-bins per octave up to the top bit.
pub const BASE_BINS: usize = 1 + OCTAVES as usize * (1 << SUB_BITS);

/// The bytes of one histogram that was never split: the struct and its leaf
/// array. A bank's size is known before it is allocated
/// ([`BankShape::unsplit_bytes`]).
pub const UNSPLIT_CELL_BYTES: usize = size_of::<OwnPast>() + BASE_BINS * size_of::<Leaf>();

const _: () = assert!(UNSPLIT_CELL_BYTES > size_of::<OwnPast>());

/// The checkpoint codec of an unsplit histogram.
pub const CODEC_V1: u32 = 1;
/// The checkpoint codec of a histogram with a split bin: v1 plus the per-bin
/// depth table (R3).
pub const CODEC_V2: u32 = CODEC_V1 + 1;

/// The payload of one varint byte.
const GROUP_MASK: u64 = (1 << HIST_CHECKPOINT_GROUP_BITS) - 1;
/// The bit of a varint byte that says another byte follows.
const CONTINUE: u64 = 1 << HIST_CHECKPOINT_GROUP_BITS;

const _: () = assert!(HIST_CHECKPOINT_GROUP_BITS < u8::BITS as u64);

/// The magic of one histogram's checkpoint body.
pub const OWN_PAST_MAGIC: &[u8] = b"BTXFIBH1";
/// The magic of one price series' tick checkpoint body.
pub const TICK_MAGIC: &[u8] = b"BTXFIBT1";
/// The magic of one stock's bank checkpoint body.
pub const BANK_MAGIC: &[u8] = b"BTXFIBB1";

/// A refusal of this module, by name. Nothing here degrades silently.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HistRefusal {
    /// No day has been committed: the stock has no own past yet.
    NoOwnPast,
    /// Days were committed, but none of them held a value (FM-F015).
    NoObservations,
    /// A negative value: the histogram holds non-negative values only.
    NegativeValue {
        /// The value offered.
        value: i64,
    },
    /// A bin's count is at `u32::MAX`: refused, never wrapped (FM-F020).
    CountFull,
    /// The committed-day count is at `u32::MAX`.
    DaysFull,
    /// A rank outside `1..=total`.
    RankOutOfRange {
        /// The rank asked for.
        rank: u64,
        /// The committed total.
        total: u64,
    },
    /// A quantile outside `(0, 1]`.
    QuantileInvalid {
        /// The quantile's numerator.
        num: u64,
        /// The quantile's denominator.
        den: u64,
    },
    /// A checkpoint, a split or a basis break with values staged today.
    StagedNotEmpty,
    /// A split of a bin that is already one integer wide.
    SplitOneValueWide {
        /// The value whose bin was asked to split.
        value: i64,
    },
    /// The raw values offered to a split are not the committed values.
    SplitRawMismatch,
    /// A split that would need more leaves than a `u32` counts.
    LeavesExceedTable,
    /// A price that is zero or negative.
    PriceNotPositive {
        /// The price offered, in paisa.
        price: i64,
    },
    /// A price series with no committed day holding two distinct prices.
    NoTick,
    /// A second that does not follow the one before it.
    SecondOutOfOrder {
        /// The second offered.
        second: i64,
        /// The last second seen.
        last: i64,
    },
    /// A cell key whose fields do not match its measure's scope.
    ScopeMismatch {
        /// The measure's name.
        measure: &'static str,
        /// The field that is present where it must be absent, or absent where
        /// it must be present.
        field: &'static str,
    },
    /// A cell key's field past its dimension.
    CellOutOfRange {
        /// The measure's name.
        measure: &'static str,
        /// The field.
        field: &'static str,
        /// The value offered.
        value: u16,
        /// The dimension's size.
        count: usize,
    },
    /// A bank shape that is not well formed.
    ShapeInvalid {
        /// What is wrong.
        what: &'static str,
    },
    /// A new shape that does not extend the old one (append-only).
    ShapeNotExtension {
        /// What it drops.
        what: &'static str,
    },
    /// One width's histograms alone do not fit the memory budget (17:15 IST
    /// item 3).
    HistbankWidthOverMemory {
        /// The width, in minutes.
        minutes: u16,
        /// The bytes it needs beside the per-stock histograms.
        need: u64,
        /// The budget, in bytes.
        budget: u64,
    },
    /// A checkpoint body that does not decode.
    SnapshotInvalid {
        /// What is wrong.
        what: &'static str,
    },
    /// A ratio the schema refused (never expected: the counts are checked).
    Cube(CubeRefusal),
}

impl HistRefusal {
    /// The refusal's stable name, for a counter or a report.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::NoOwnPast => "no_own_past",
            Self::NoObservations => "no_observations",
            Self::NegativeValue { .. } => "negative_value",
            Self::CountFull => "count_full",
            Self::DaysFull => "days_full",
            Self::RankOutOfRange { .. } => "rank_out_of_range",
            Self::QuantileInvalid { .. } => "quantile_invalid",
            Self::StagedNotEmpty => "staged_not_empty",
            Self::SplitOneValueWide { .. } => "split_one_value_wide",
            Self::SplitRawMismatch => "split_raw_mismatch",
            Self::LeavesExceedTable => "leaves_exceed_table",
            Self::PriceNotPositive { .. } => "price_not_positive",
            Self::NoTick => "no_tick",
            Self::SecondOutOfOrder { .. } => "second_out_of_order",
            Self::ScopeMismatch { .. } => "scope_mismatch",
            Self::CellOutOfRange { .. } => "cell_out_of_range",
            Self::ShapeInvalid { .. } => "shape_invalid",
            Self::ShapeNotExtension { .. } => "shape_not_extension",
            Self::HistbankWidthOverMemory { .. } => "histbank_width_over_memory",
            Self::SnapshotInvalid { .. } => "snapshot_invalid",
            Self::Cube(_) => "cube",
        }
    }
}

impl core::fmt::Display for HistRefusal {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "own past refused ({}): {self:?}", self.name())
    }
}

impl std::error::Error for HistRefusal {}

/// A value as the histogram holds it: non-negative.
fn non_negative(value: i64) -> Result<u64, HistRefusal> {
    u64::try_from(value)
        .ok()
        .ok_or(HistRefusal::NegativeValue { value })
}

/// An index that is inside its table by construction.
fn index(at: u64) -> usize {
    usize::try_from(at).unwrap_or(usize::MAX)
}

/// Applies `change` to the cell at `at`. Every caller's index is inside its
/// buffer by construction and the workspace denies indexing, so the write is
/// checked here, once: an index past the slice changes nothing.
fn change_at<T>(cells: &mut [T], at: usize, change: impl FnOnce(&mut T)) {
    if let Some(cell) = cells.get_mut(at) {
        change(cell);
    }
}

/// The base bin of a non-negative value: zero is bin 0; otherwise the top bit
/// picks the octave and the next [`SUB_BITS`] bits the sub-bin.
fn base_bin(value: u64) -> usize {
    let Some(top) = value.checked_ilog2() else {
        return 0;
    };
    let aligned = if top >= SUB_BITS {
        value >> (top - SUB_BITS)
    } else {
        value << (SUB_BITS - top)
    };
    let mantissa = aligned & (HIST_SUB_BINS_PER_POWER - 1);
    index(u64::from(top) * HIST_SUB_BINS_PER_POWER + mantissa + 1)
}

/// The smallest value of child `child` of base bin `bin` split to `depth`
/// further mantissa bits. Exact for every bin a value can reach.
fn lower_edge(bin: usize, depth: u32, child: u64) -> u64 {
    let Some(at) = u64::try_from(bin).ok().and_then(|b| b.checked_sub(1)) else {
        return 0;
    };
    let top = at / HIST_SUB_BINS_PER_POWER;
    let mantissa = at % HIST_SUB_BINS_PER_POWER;
    // `child` sits below the shifted head, so the sum joins the bits; a sum
    // rather than an or, because an or and an xor of disjoint bits agree.
    let head = u128::from(((HIST_SUB_BINS_PER_POWER + mantissa) << depth) + child);
    let edge = (head << top) >> (SUB_BITS + depth);
    u64::try_from(edge).unwrap_or(u64::MAX)
}

/// The child of a value inside its base bin split to `depth` further bits.
fn child_of(value: u64, depth: u32) -> u64 {
    let top = value.checked_ilog2().unwrap_or(0);
    let shift = top.saturating_sub(SUB_BITS + depth);
    (value >> shift) & ((1 << depth) - 1)
}

/// The per-bin depth table of a histogram with a split bin (R3).
#[derive(Clone, Debug, PartialEq, Eq)]
struct SplitTable {
    /// Per base bin: its first leaf and its split depth.
    bins: Box<[(u32, u8)]>,
    /// Per leaf: its base bin.
    leaf_bin: Box<[u32]>,
}

impl SplitTable {
    /// The table of `depths` (one per base bin), or `None` when a leaf count
    /// would not fit a `u32`.
    fn of(depths: &[u8]) -> Option<Self> {
        let mut bins = Vec::with_capacity(depths.len());
        let mut first = 0_u32;
        for depth in depths {
            bins.push((first, *depth));
            first = first.checked_add(1_u32.checked_shl(u32::from(*depth))?)?;
        }
        let mut leaf_bin = Vec::with_capacity(index(u64::from(first)));
        for (bin, depth) in (0_u32..).zip(depths) {
            leaf_bin.extend(core::iter::repeat_n(bin, index(1_u64 << depth)));
        }
        Some(Self {
            bins: bins.into_boxed_slice(),
            leaf_bin: leaf_bin.into_boxed_slice(),
        })
    }

    /// A bin's first leaf and depth.
    fn bin(&self, bin: usize) -> (usize, u8) {
        let (first, depth) = self.bins.get(bin).copied().unwrap_or((u32::MAX, 0));
        (index(u64::from(first)), depth)
    }

    /// Every bin's depth.
    fn depths(&self) -> Vec<u8> {
        self.bins.iter().map(|(_, depth)| *depth).collect()
    }
}

/// One leaf of a histogram: everything a call reads or writes of it, together.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Leaf {
    /// Committed values in the leaves below this one. Rebuilt only by
    /// [`OwnPast::commit_day`].
    below: u64,
    /// Committed values in this leaf.
    committed: u32,
    /// Values staged today in this leaf.
    staged: u32,
}

impl Leaf {
    /// A leaf holding nothing.
    const EMPTY: Self = Self {
        below: 0,
        committed: 0,
        staged: 0,
    };

    /// Committed values up to and including this leaf.
    fn end(self) -> u64 {
        self.below + u64::from(self.committed)
    }
}

/// An own-past histogram of non-negative `i64` values (D-6320).
///
/// A constant bin table covers the whole range — bin 0, then
/// [`HIST_SUB_BINS_PER_POWER`] sub-bins per power of two up to the top bit —
/// so nothing overflows into a catch-all bin. A bin that a cut must divide is
/// split by one more mantissa bit and rebuilt once from its raw values
/// ([`OwnPast::split`]); a split histogram encodes as [`CODEC_V2`].
///
/// Equality compares the histogram and its counters, never the bench's work
/// counter.
#[derive(Clone, Debug)]
pub struct OwnPast {
    /// The depth table, when any bin is split; `None` means one leaf per bin.
    split: Option<Box<SplitTable>>,
    /// Per leaf: its committed and staged counts and the committed values
    /// below it, in one line, so a call reads one place.
    leaves: Box<[Leaf]>,
    /// Days committed.
    committed_days: u32,
    /// Values committed.
    total: u64,
    /// Values staged today.
    staged_values: u64,
    /// Censored observations committed: known only to exceed what was seen.
    censored: u64,
    /// Censored observations staged today.
    staged_censored: u64,
    /// The largest committed value, exactly.
    max: Option<u64>,
    /// The largest value staged today.
    staged_max: Option<u64>,
    /// Observations refused as negative.
    refused_negative: u64,
    /// Observations refused at a full bin.
    refused_full: u64,
    /// The structural work counter the bench reads: bins touched, probes made
    /// (a plain field, gate 17).
    work: Cell<u64>,
}

/// What equality compares of an [`OwnPast`]: every field but the work counter.
type CompareKey<'a> = (
    (Option<&'a SplitTable>, &'a [Leaf], u32, u64),
    (u64, u64, u64, Option<u64>, Option<u64>, u64, u64),
);

impl PartialEq for OwnPast {
    fn eq(&self, other: &Self) -> bool {
        self.compared() == other.compared()
    }
}

impl Eq for OwnPast {}

/// A median that may be censored (FM-F016): when the censored observations
/// are at least as many as the resolved ones, the median is labelled and
/// applies no cut.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CensoredMedian {
    /// The median of the resolved values (its bin's lower edge).
    pub value: i64,
    /// Whether censored observations are at least as many as resolved ones:
    /// the label "duration median censored".
    pub censored: bool,
}

impl OwnPast {
    /// An empty histogram on the unsplit table: no day committed.
    #[must_use]
    pub fn empty() -> Self {
        Self::with_leaves(None, BASE_BINS)
    }

    /// An empty histogram with `leaves` leaves and the given depth table.
    fn with_leaves(split: Option<Box<SplitTable>>, leaves: usize) -> Self {
        Self {
            split,
            leaves: vec![Leaf::EMPTY; leaves].into_boxed_slice(),
            committed_days: 0,
            total: 0,
            staged_values: 0,
            censored: 0,
            staged_censored: 0,
            max: None,
            staged_max: None,
            refused_negative: 0,
            refused_full: 0,
            work: Cell::new(0),
        }
    }

    /// The leaves: [`BASE_BINS`] until a bin is split.
    #[must_use]
    pub fn leaves(&self) -> usize {
        self.leaves.len()
    }

    /// Days committed: the stamp a candidate carries as a reference.
    #[must_use]
    pub const fn committed_days(&self) -> u32 {
        self.committed_days
    }

    /// Values committed.
    #[must_use]
    pub const fn total(&self) -> u64 {
        self.total
    }

    /// Values staged today.
    #[must_use]
    pub const fn staged_values(&self) -> u64 {
        self.staged_values
    }

    /// Censored observations committed.
    #[must_use]
    pub const fn censored(&self) -> u64 {
        self.censored
    }

    /// The largest committed value, exactly, or `None` with nothing committed.
    #[must_use]
    pub fn max(&self) -> Option<i64> {
        self.max.and_then(|max| i64::try_from(max).ok())
    }

    /// Observations refused as `negative_value`.
    #[must_use]
    pub const fn refused_negative(&self) -> u64 {
        self.refused_negative
    }

    /// Observations refused as `count_full`.
    #[must_use]
    pub const fn refused_full(&self) -> u64 {
        self.refused_full
    }

    /// The structural work done so far: bins touched and probes made.
    #[must_use]
    pub fn work(&self) -> u64 {
        self.work.get()
    }

    /// Adds to the work counter.
    fn did(&self, units: u64) {
        self.work.set(self.work.get().wrapping_add(units));
    }

    /// The bytes this histogram holds, heap included: [`UNSPLIT_CELL_BYTES`]
    /// until a bin is split.
    #[must_use]
    pub fn bytes(&self) -> usize {
        let leaves = self.leaves();
        let table = self.split.as_ref().map_or(0, |table| {
            size_of::<SplitTable>()
                + table.bins.len() * size_of::<(u32, u8)>()
                + table.leaf_bin.len() * size_of::<u32>()
        });
        size_of::<Self>() + leaves * size_of::<Leaf>() + table
    }

    /// The leaf of a non-negative value.
    fn leaf_of(&self, value: u64) -> usize {
        let bin = base_bin(value);
        match &self.split {
            None => bin,
            Some(table) => {
                let (first, depth) = table.bin(bin);
                first + index(child_of(value, u32::from(depth)))
            }
        }
    }

    /// The smallest value of a leaf.
    fn leaf_lower_edge(&self, leaf: usize) -> u64 {
        match &self.split {
            None => lower_edge(leaf, 0, 0),
            Some(table) => {
                let bin = index(u64::from(
                    table.leaf_bin.get(leaf).copied().unwrap_or(u32::MAX),
                ));
                let (first, depth) = table.bin(bin);
                let child = u64::try_from(leaf.saturating_sub(first)).unwrap_or(u64::MAX);
                lower_edge(bin, u32::from(depth), child)
            }
        }
    }

    /// The depth of a value's base bin.
    fn depth_of(&self, value: u64) -> u32 {
        self.split
            .as_ref()
            .map_or(0, |table| u32::from(table.bin(base_bin(value)).1))
    }

    /// The leaf holding `value`: its lower edge and its width in integers.
    ///
    /// # Errors
    ///
    /// [`HistRefusal::NegativeValue`].
    pub fn leaf_bounds(&self, value: i64) -> Result<(i64, u64), HistRefusal> {
        let value = non_negative(value)?;
        let depth = self.depth_of(value);
        let width = value
            .checked_ilog2()
            .and_then(|top| top.checked_sub(SUB_BITS + depth))
            .map_or(1, |bits| 1_u64 << bits);
        let lower = self.leaf_lower_edge(self.leaf_of(value));
        Ok((i64::try_from(lower).unwrap_or(i64::MAX), width))
    }

    /// Stages one value for today. It changes no answer until
    /// [`OwnPast::commit_day`].
    ///
    /// # Errors
    ///
    /// [`HistRefusal::NegativeValue`] and [`HistRefusal::CountFull`], each
    /// counted ([`OwnPast::refused_negative`], [`OwnPast::refused_full`]).
    pub fn observe(&mut self, value: i64) -> Result<(), HistRefusal> {
        let Ok(value) = non_negative(value) else {
            self.refused_negative = self.refused_negative.saturating_add(1);
            return Err(HistRefusal::NegativeValue { value });
        };
        let leaf = self.leaf_of(value);
        let room = self.leaves.get_mut(leaf).filter(|leaf| {
            u64::from(leaf.committed) + u64::from(leaf.staged) < u64::from(u32::MAX)
        });
        let Some(leaf) = room else {
            self.refused_full = self.refused_full.saturating_add(1);
            return Err(HistRefusal::CountFull);
        };
        leaf.staged += 1;
        self.staged_values += 1;
        self.staged_max = Some(self.staged_max.map_or(value, |max| max.max(value)));
        self.did(1);
        Ok(())
    }

    /// Stages one censored observation for today: an outcome known only to
    /// exceed what was seen (FM-F016's `duration_censored`). It enters no bin.
    pub fn observe_censored(&mut self) {
        self.staged_censored = self.staged_censored.saturating_add(1);
        self.did(1);
    }

    /// Merges today's staged values into the committed past and rebuilds the
    /// cumulative array: O(leaves) once per day, O(1) on a day that staged
    /// nothing (`crates/indicators/tests/fib_hist.rs`, bench row H-F-04).
    /// Every answer after it reads the day.
    ///
    /// # Errors
    ///
    /// [`HistRefusal::DaysFull`] at `u32::MAX` committed days; nothing is
    /// merged.
    pub fn commit_day(&mut self) -> Result<(), HistRefusal> {
        let days = self
            .committed_days
            .checked_add(1)
            .ok_or(HistRefusal::DaysFull)?;
        if self.staged_values > 0 {
            let mut running = 0_u64;
            for leaf in &mut self.leaves {
                leaf.committed = leaf.committed.saturating_add(leaf.staged);
                leaf.staged = 0;
                leaf.below = running;
                running += u64::from(leaf.committed);
            }
            self.total = running;
            self.did(u64::try_from(self.leaves()).unwrap_or(u64::MAX));
        }
        self.max = match (self.max, self.staged_max) {
            (Some(a), Some(b)) => Some(a.max(b)),
            (a, b) => a.or(b),
        };
        self.censored = self.censored.saturating_add(self.staged_censored);
        self.committed_days = days;
        self.staged_values = 0;
        self.staged_censored = 0;
        self.staged_max = None;
        self.did(1);
        Ok(())
    }

    /// Drops today's staged values: a non-tradable day, or a day that failed
    /// mid-way, leaves the histogram as it was.
    pub fn discard_day(&mut self) {
        if self.staged_values > 0 {
            self.leaves.iter_mut().for_each(|leaf| leaf.staged = 0);
        }
        self.staged_values = 0;
        self.staged_censored = 0;
        self.staged_max = None;
        self.did(1);
    }

    /// Refuses an answer from a histogram with no own past.
    fn answerable(&self) -> Result<(), HistRefusal> {
        if self.committed_days == 0 {
            return Err(HistRefusal::NoOwnPast);
        }
        if self.total == 0 {
            return Err(HistRefusal::NoObservations);
        }
        Ok(())
    }

    /// The exact mid-rank position of `value` in the committed past (R1):
    /// `(2 x below + in-bin) / (2 x total)`, where `below` counts the values
    /// in leaves below `value`'s and `in-bin` those in its leaf. One leaf
    /// computed, two counters read.
    ///
    /// # Errors
    ///
    /// [`HistRefusal::NegativeValue`], [`HistRefusal::NoOwnPast`] and
    /// [`HistRefusal::NoObservations`].
    pub fn mid_rank(&self, value: i64) -> Result<Ratio, HistRefusal> {
        let value = non_negative(value)?;
        self.answerable()?;
        let leaf = self.leaf_of(value);
        let (below, in_leaf) = self
            .leaves
            .get(leaf)
            .map_or((0, 0), |leaf| (leaf.below, u64::from(leaf.committed)));
        self.did(1);
        Ratio::mid_rank(below, in_leaf, self.total).map_err(HistRefusal::Cube)
    }

    /// The mid-rank position in permille, rounded down: a DISPLAY unit only
    /// (17:15 IST item 1). The numerator is computed in `u128`, so no total
    /// overflows it (FM-F020).
    ///
    /// # Errors
    ///
    /// As [`OwnPast::mid_rank`].
    pub fn permille(&self, value: i64) -> Result<u16, HistRefusal> {
        let rank = self.mid_rank(value)?;
        let permille =
            u128::from(rank.num()) * u128::from(PERMILLE_PER_UNIT) / u128::from(rank.den());
        Ok(u16::try_from(permille).unwrap_or(u16::MAX))
    }

    /// The value at 1-based rank `rank` of the committed past: the lower edge
    /// of the leaf holding it (FM-F012). One bisection over the cumulative
    /// array; its probes are counted in [`OwnPast::work`].
    ///
    /// # Errors
    ///
    /// [`HistRefusal::NoOwnPast`], [`HistRefusal::NoObservations`] and
    /// [`HistRefusal::RankOutOfRange`].
    pub fn value_at_rank(&self, rank: u64) -> Result<i64, HistRefusal> {
        self.answerable()?;
        if rank == 0 || rank > self.total {
            return Err(HistRefusal::RankOutOfRange {
                rank,
                total: self.total,
            });
        }
        let mut probes = 0_u64;
        let leaf = self.leaves.partition_point(|leaf| {
            probes += 1;
            leaf.end() < rank
        });
        self.did(probes);
        Ok(i64::try_from(self.leaf_lower_edge(leaf)).unwrap_or(i64::MAX))
    }

    /// The lower nearest-rank quantile `num / den` of the committed past: the
    /// value at rank `ceil(num x total / den)` ([`OwnPast::value_at_rank`]).
    ///
    /// # Errors
    ///
    /// [`HistRefusal::QuantileInvalid`] unless `0 < num <= den`, then as
    /// [`OwnPast::value_at_rank`].
    pub fn quantile(&self, num: u64, den: u64) -> Result<i64, HistRefusal> {
        if num == 0 || num > den {
            return Err(HistRefusal::QuantileInvalid { num, den });
        }
        self.answerable()?;
        let rank = (u128::from(num) * u128::from(self.total)).div_ceil(u128::from(den));
        self.value_at_rank(u64::try_from(rank).unwrap_or(u64::MAX))
    }

    /// The median of the committed past: the value at the middle rank,
    /// `ceil(total / 2)` (the lower middle of an even sample).
    ///
    /// # Errors
    ///
    /// As [`OwnPast::value_at_rank`].
    pub fn median(&self) -> Result<i64, HistRefusal> {
        self.answerable()?;
        self.value_at_rank(self.total - (self.total >> 1))
    }

    /// The median with its censoring label (FM-F016).
    ///
    /// # Errors
    ///
    /// As [`OwnPast::median`].
    pub fn censored_median(&self) -> Result<CensoredMedian, HistRefusal> {
        let value = self.median()?;
        Ok(CensoredMedian {
            value,
            censored: self.censored >= self.total,
        })
    }

    /// Splits the bin holding `value` by one more mantissa bit and rebuilds
    /// the counts once from `raw`, which must be exactly the committed values
    /// (R3). Earlier answers stay reproducible from the unsplit checkpoint.
    ///
    /// # Errors
    ///
    /// [`HistRefusal::StagedNotEmpty`] while a day is staged,
    /// [`HistRefusal::NegativeValue`], [`HistRefusal::SplitOneValueWide`] for a
    /// bin already one integer wide, [`HistRefusal::SplitRawMismatch`] when
    /// `raw` does not reproduce the committed counts and maximum, and
    /// [`HistRefusal::LeavesExceedTable`]. On any refusal nothing changes.
    pub fn split(&mut self, value: i64, raw: &[i64]) -> Result<(), HistRefusal> {
        if self.staged_values != 0 || self.staged_censored != 0 {
            return Err(HistRefusal::StagedNotEmpty);
        }
        let held = non_negative(value)?;
        let next = self.depth_of(held) + 1;
        let wide = held
            .checked_ilog2()
            .is_some_and(|top| top >= SUB_BITS + next);
        if !wide {
            return Err(HistRefusal::SplitOneValueWide { value });
        }
        let mut raw_values = Vec::with_capacity(raw.len());
        for value in raw {
            raw_values.push(non_negative(*value)?);
        }
        let mut recount = vec![0_u32; self.leaves()];
        for value in &raw_values {
            change_at(&mut recount, self.leaf_of(*value), |count| {
                *count = count.saturating_add(1);
            });
        }
        let raw_max = raw_values.iter().copied().max();
        let committed = self.leaves.iter().map(|leaf| leaf.committed);
        if !recount.iter().copied().eq(committed) || raw_max != self.max {
            return Err(HistRefusal::SplitRawMismatch);
        }
        let mut depths = self
            .split
            .as_ref()
            .map_or_else(|| vec![0; BASE_BINS], |table| table.depths());
        change_at(&mut depths, base_bin(held), |depth| {
            *depth = u8::try_from(next).unwrap_or(u8::MAX);
        });
        let table = SplitTable::of(&depths).ok_or(HistRefusal::LeavesExceedTable)?;
        let leaves = table.leaf_bin.len();
        let mut rebuilt = Self::with_leaves(Some(Box::new(table)), leaves);
        for value in &raw_values {
            let leaf = rebuilt.leaf_of(*value);
            change_at(&mut rebuilt.leaves, leaf, |leaf| {
                leaf.committed = leaf.committed.saturating_add(1);
            });
        }
        rebuilt.rebuild_cumulative();
        rebuilt.committed_days = self.committed_days;
        rebuilt.censored = self.censored;
        rebuilt.max = self.max;
        rebuilt.refused_negative = self.refused_negative;
        rebuilt.refused_full = self.refused_full;
        rebuilt.work.set(self.work.get());
        rebuilt.did(u64::try_from(raw_values.len()).unwrap_or(u64::MAX));
        *self = rebuilt;
        Ok(())
    }

    /// Rebuilds every leaf's count below it, and the total, from the committed
    /// counts.
    fn rebuild_cumulative(&mut self) {
        let mut running = 0_u64;
        for leaf in &mut self.leaves {
            leaf.below = running;
            running += u64::from(leaf.committed);
        }
        self.total = running;
    }

    /// Everything equality compares: the histogram and its counters, never
    /// the bench's work counter.
    fn compared(&self) -> CompareKey<'_> {
        (
            (
                self.split.as_deref(),
                &*self.leaves,
                self.committed_days,
                self.total,
            ),
            (
                self.staged_values,
                self.censored,
                self.staged_censored,
                self.max,
                self.staged_max,
                self.refused_negative,
                self.refused_full,
            ),
        )
    }

    /// Appends this histogram's checkpoint body (D-6325): [`OWN_PAST_MAGIC`],
    /// the codec version ([`CODEC_V1`], or [`CODEC_V2`] with the depth table),
    /// the bin table's size and sub-bits, the counters, then the committed
    /// counts as varint zero-runs and varint counts. Little-endian and
    /// canonical: one histogram has one body. The scan is O(leaves); the bytes
    /// written are O(nonzero leaves).
    ///
    /// # Errors
    ///
    /// [`HistRefusal::StagedNotEmpty`]: a checkpoint is taken after a day is
    /// committed or discarded, never in the middle of one.
    pub fn encode(&self, out: &mut Vec<u8>) -> Result<(), HistRefusal> {
        if self.staged_values != 0 || self.staged_censored != 0 {
            return Err(HistRefusal::StagedNotEmpty);
        }
        out.extend_from_slice(OWN_PAST_MAGIC);
        let version = if self.split.is_some() {
            CODEC_V2
        } else {
            CODEC_V1
        };
        out.extend_from_slice(&version.to_le_bytes());
        out.extend_from_slice(&u32_of(BASE_BINS).to_le_bytes());
        out.extend_from_slice(&SUB_BITS.to_le_bytes());
        if let Some(table) = &self.split {
            put_sparse(out, table.bins.iter().map(|(_, depth)| u64::from(*depth)));
        }
        out.extend_from_slice(&self.committed_days.to_le_bytes());
        out.extend_from_slice(&self.total.to_le_bytes());
        out.extend_from_slice(&self.censored.to_le_bytes());
        put_varint(out, self.max.unwrap_or(0));
        out.extend_from_slice(&self.refused_negative.to_le_bytes());
        out.extend_from_slice(&self.refused_full.to_le_bytes());
        put_sparse(
            out,
            self.leaves.iter().map(|leaf| u64::from(leaf.committed)),
        );
        Ok(())
    }

    /// Decodes one histogram's checkpoint body, which must be the whole of
    /// `bytes`.
    ///
    /// # Errors
    ///
    /// [`HistRefusal::SnapshotInvalid`], naming what is wrong.
    pub fn decode(bytes: &[u8]) -> Result<Self, HistRefusal> {
        let mut reader = Reader { bytes, at: 0 };
        let past = Self::decode_from(&mut reader)?;
        reader.end()?;
        Ok(past)
    }

    /// Decodes one body from a reader.
    fn decode_from(reader: &mut Reader<'_>) -> Result<Self, HistRefusal> {
        reader.magic(OWN_PAST_MAGIC)?;
        let version = reader.u32()?;
        if reader.u32()? != u32_of(BASE_BINS) || reader.u32()? != SUB_BITS {
            return Err(invalid("bin_table"));
        }
        let split = match version {
            CODEC_V1 => None,
            CODEC_V2 => Some(Box::new(read_depths(reader)?)),
            _ => return Err(invalid("version")),
        };
        let leaves = split
            .as_ref()
            .map_or(BASE_BINS, |table| table.leaf_bin.len());
        let mut past = Self::with_leaves(split, leaves);
        past.committed_days = reader.u32()?;
        let total = reader.u64()?;
        past.censored = reader.u64()?;
        let max = reader.varint()?;
        past.refused_negative = reader.u64()?;
        past.refused_full = reader.u64()?;
        let counts = read_sparse(reader, leaves)?;
        let mut highest = None;
        for (at, (leaf, value)) in past.leaves.iter_mut().zip(counts).enumerate() {
            if value > u64::from(u32::MAX) {
                return Err(invalid("counts"));
            }
            if value > 0 {
                highest = Some(at);
            }
            leaf.committed = u32::try_from(value).unwrap_or(u32::MAX);
        }
        past.rebuild_cumulative();
        if past.total != total || (total > 0 && past.committed_days == 0) {
            return Err(invalid("total"));
        }
        past.max = if total == 0 {
            if max != 0 {
                return Err(invalid("max"));
            }
            None
        } else {
            if highest != Some(past.leaf_of(max)) {
                return Err(invalid("max"));
            }
            Some(max)
        };
        Ok(past)
    }
}

/// A `u32` of a table size that fits one by construction.
fn u32_of(size: usize) -> u32 {
    u32::try_from(size).unwrap_or(u32::MAX)
}

/// A checkpoint refusal naming what is wrong.
const fn invalid(what: &'static str) -> HistRefusal {
    HistRefusal::SnapshotInvalid { what }
}

/// Appends one canonical varint: [`HIST_CHECKPOINT_GROUP_BITS`] payload bits
/// per byte, least significant group first.
fn put_varint(out: &mut Vec<u8>, mut value: u64) {
    loop {
        let low = value & GROUP_MASK;
        value >>= HIST_CHECKPOINT_GROUP_BITS;
        let more = if value == 0 { 0 } else { CONTINUE };
        let [byte, ..] = (low | more).to_le_bytes();
        out.push(byte);
        if value == 0 {
            return;
        }
    }
}

/// Appends a sparse list: the count of nonzero entries, then for each one
/// the run of zeros before it and its value, all varints.
fn put_sparse(out: &mut Vec<u8>, values: impl Iterator<Item = u64> + Clone) {
    let nonzero = values.clone().filter(|value| *value != 0).count();
    put_varint(out, u64::try_from(nonzero).unwrap_or(u64::MAX));
    let mut zeros = 0_u64;
    for value in values {
        if value == 0 {
            zeros += 1;
        } else {
            put_varint(out, zeros);
            put_varint(out, value);
            zeros = 0;
        }
    }
}

/// A sparse list of `len` entries, zeros included. A position past `len`, or
/// a zero written as a value, is refused: the list is canonical.
fn read_sparse(reader: &mut Reader<'_>, len: usize) -> Result<Vec<u64>, HistRefusal> {
    let nonzero = reader.varint()?;
    let mut values = vec![0_u64; len];
    let mut next = 0_u64;
    for _ in 0..nonzero {
        let at = next
            .checked_add(reader.varint()?)
            .ok_or(invalid("sparse"))?;
        let value = reader.varint()?;
        let slot = values
            .get_mut(index(at))
            .filter(|_| value != 0)
            .ok_or(invalid("sparse"))?;
        *slot = value;
        next = at + 1;
    }
    Ok(values)
}

/// Reads a depth table (a sparse list over the base bins) and checks every
/// split depth against its bin's width.
fn read_depths(reader: &mut Reader<'_>) -> Result<SplitTable, HistRefusal> {
    let depths = read_sparse(reader, BASE_BINS)?;
    let mut table = Vec::with_capacity(BASE_BINS);
    for (bin, depth) in depths.into_iter().enumerate() {
        let edge = lower_edge(bin, 0, 0);
        let depth = u32::try_from(depth).unwrap_or(u32::MAX);
        let fits = edge
            .checked_ilog2()
            .is_some_and(|top| top >= SUB_BITS + depth);
        if depth != 0 && !fits {
            return Err(invalid("depth"));
        }
        table.push(u8::try_from(depth).unwrap_or(u8::MAX));
    }
    if table.iter().all(|depth| *depth == 0) {
        return Err(invalid("depth"));
    }
    SplitTable::of(&table).ok_or(invalid("leaves"))
}

/// A cursor over a checkpoint body.
struct Reader<'a> {
    /// The body.
    bytes: &'a [u8],
    /// The next unread byte.
    at: usize,
}

impl Reader<'_> {
    /// The next `len` bytes.
    fn slice(&mut self, len: usize) -> Result<&[u8], HistRefusal> {
        let start = self.at;
        let bytes = start
            .checked_add(len)
            .and_then(|end| self.bytes.get(start..end))
            .ok_or(invalid("truncated"))?;
        self.at = start + len;
        Ok(bytes)
    }

    /// The next `N` bytes.
    fn take<const N: usize>(&mut self) -> Result<[u8; N], HistRefusal> {
        let mut bytes = [0; N];
        bytes.copy_from_slice(self.slice(N)?);
        Ok(bytes)
    }

    /// The magic `expected`.
    fn magic(&mut self, expected: &[u8]) -> Result<(), HistRefusal> {
        if self.slice(expected.len())? == expected {
            Ok(())
        } else {
            Err(invalid("magic"))
        }
    }

    /// A little-endian `u32`.
    fn u32(&mut self) -> Result<u32, HistRefusal> {
        Ok(u32::from_le_bytes(self.take()?))
    }

    /// A little-endian `u64`.
    fn u64(&mut self) -> Result<u64, HistRefusal> {
        Ok(u64::from_le_bytes(self.take()?))
    }

    /// One canonical varint.
    fn varint(&mut self) -> Result<u64, HistRefusal> {
        let mut value = 0_u64;
        let mut shift = 0_u64;
        loop {
            let [byte] = self.take::<1>()?;
            let low = u64::from(byte) & GROUP_MASK;
            let fits = shift < u64::from(u64::BITS)
                && low.checked_shl(u32_of(index(shift))).unwrap_or(0) >> shift == low;
            if !fits {
                return Err(invalid("varint_too_long"));
            }
            value |= low << shift;
            if u64::from(byte) & CONTINUE == 0 {
                if shift > 0 && low == 0 {
                    return Err(invalid("varint_not_canonical"));
                }
                return Ok(value);
            }
            shift += HIST_CHECKPOINT_GROUP_BITS;
        }
    }

    /// Refuses unread bytes.
    fn end(&self) -> Result<(), HistRefusal> {
        if self.at == self.bytes.len() {
            Ok(())
        } else {
            Err(invalid("trailing"))
        }
    }
}

/// The greatest common divisor of two magnitudes.
const fn gcd(mut a: u64, mut b: u64) -> u64 {
    while b != 0 {
        let rest = a % b;
        a = b;
        b = rest;
    }
    a
}

/// The tick a price series SHOWS, because the tick size by price and date is
/// UNVERIFIED in `docs/00-charter.md` (D-6321, FM-F014).
///
/// Each day stages the gcd of the absolute differences between its
/// consecutive distinct prices (none on a day with fewer than two distinct
/// prices). The tick is the MODE of the committed days' gcds since the
/// series' last basis break, ties to the larger: one odd print does not force
/// one paisa forever, and a coarser tick is seen again. Decision and raw
/// series are separate values of this type.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObservedTick {
    /// Today's last price.
    last_price: Option<i64>,
    /// Today's running gcd; zero until two distinct prices are seen.
    day_gcd: u64,
    /// `(gcd, days)` of the committed days since the last basis break,
    /// ascending by gcd.
    days: Vec<(u64, u32)>,
    /// The mode `(gcd, days)`.
    mode: Option<(u64, u32)>,
    /// Days committed since the last basis break.
    committed_days: u32,
    /// Of those, the days that had a gcd.
    gcd_days: u32,
    /// Basis breaks seen.
    breaks: u32,
    /// Prices refused as not positive.
    refused_prices: u64,
}

impl ObservedTick {
    /// A series with nothing observed.
    #[must_use]
    pub const fn empty() -> Self {
        Self {
            last_price: None,
            day_gcd: 0,
            days: Vec::new(),
            mode: None,
            committed_days: 0,
            gcd_days: 0,
            breaks: 0,
            refused_prices: 0,
        }
    }

    /// Stages one price of today, in time order.
    ///
    /// # Errors
    ///
    /// [`HistRefusal::PriceNotPositive`], counted.
    pub fn observe(&mut self, price: i64) -> Result<(), HistRefusal> {
        if price <= 0 {
            self.refused_prices = self.refused_prices.saturating_add(1);
            return Err(HistRefusal::PriceNotPositive { price });
        }
        if let Some(last) = self.last_price {
            self.day_gcd = gcd(self.day_gcd, last.abs_diff(price));
        }
        self.last_price = Some(price);
        Ok(())
    }

    /// Commits today: its gcd, when it has one, joins the mode's map. O(distinct
    /// gcds) per commit.
    ///
    /// # Errors
    ///
    /// [`HistRefusal::DaysFull`]; nothing changes.
    pub fn commit_day(&mut self) -> Result<(), HistRefusal> {
        let days = self
            .committed_days
            .checked_add(1)
            .ok_or(HistRefusal::DaysFull)?;
        if self.day_gcd != 0 {
            let at = self.days.partition_point(|(g, _)| *g < self.day_gcd);
            match self.days.get_mut(at) {
                Some((g, count)) if *g == self.day_gcd => *count = count.saturating_add(1),
                _ => self.days.insert(at, (self.day_gcd, 1)),
            }
            self.gcd_days = self.gcd_days.saturating_add(1);
            self.mode = self
                .days
                .iter()
                .copied()
                .max_by_key(|(g, count)| (*count, *g));
        }
        self.committed_days = days;
        self.discard_day();
        Ok(())
    }

    /// Drops today's prices.
    pub const fn discard_day(&mut self) {
        self.last_price = None;
        self.day_gcd = 0;
    }

    /// A basis break (a corporate action, a re-based series): the committed
    /// map restarts.
    ///
    /// # Errors
    ///
    /// [`HistRefusal::StagedNotEmpty`]: a break falls between days.
    pub fn basis_break(&mut self) -> Result<(), HistRefusal> {
        if self.last_price.is_some() {
            return Err(HistRefusal::StagedNotEmpty);
        }
        self.days.clear();
        self.mode = None;
        self.committed_days = 0;
        self.gcd_days = 0;
        self.breaks = self.breaks.saturating_add(1);
        Ok(())
    }

    /// The observed tick, in paisa.
    ///
    /// # Errors
    ///
    /// [`HistRefusal::NoTick`] while no committed day since the last break had
    /// two distinct prices.
    pub fn tick(&self) -> Result<i64, HistRefusal> {
        self.mode
            .and_then(|(g, _)| i64::try_from(g).ok())
            .ok_or(HistRefusal::NoTick)
    }

    /// Committed days with a gcd that differs from the tick: `odd_tick_day`.
    #[must_use]
    pub fn odd_tick_days(&self) -> u32 {
        self.gcd_days - self.mode.map_or(0, |(_, days)| days)
    }

    /// Days committed since the last basis break.
    #[must_use]
    pub const fn committed_days(&self) -> u32 {
        self.committed_days
    }

    /// Basis breaks seen.
    #[must_use]
    pub const fn breaks(&self) -> u32 {
        self.breaks
    }

    /// Prices refused as not positive.
    #[must_use]
    pub const fn refused_prices(&self) -> u64 {
        self.refused_prices
    }

    /// Appends this series' checkpoint body (D-6325): [`TICK_MAGIC`],
    /// [`CODEC_V1`], the counters, then every `(gcd, days)` as varints in
    /// ascending gcd order.
    ///
    /// # Errors
    ///
    /// [`HistRefusal::StagedNotEmpty`] while today holds a price.
    pub fn encode(&self, out: &mut Vec<u8>) -> Result<(), HistRefusal> {
        if self.last_price.is_some() {
            return Err(HistRefusal::StagedNotEmpty);
        }
        out.extend_from_slice(TICK_MAGIC);
        out.extend_from_slice(&CODEC_V1.to_le_bytes());
        out.extend_from_slice(&self.committed_days.to_le_bytes());
        out.extend_from_slice(&self.breaks.to_le_bytes());
        out.extend_from_slice(&self.refused_prices.to_le_bytes());
        put_varint(out, u64::try_from(self.days.len()).unwrap_or(u64::MAX));
        for (g, days) in &self.days {
            put_varint(out, *g);
            put_varint(out, u64::from(*days));
        }
        Ok(())
    }

    /// Decodes one series' checkpoint body, which must be the whole of
    /// `bytes`.
    ///
    /// # Errors
    ///
    /// [`HistRefusal::SnapshotInvalid`], naming what is wrong.
    pub fn decode(bytes: &[u8]) -> Result<Self, HistRefusal> {
        let mut reader = Reader { bytes, at: 0 };
        reader.magic(TICK_MAGIC)?;
        if reader.u32()? != CODEC_V1 {
            return Err(invalid("version"));
        }
        let mut tick = Self::empty();
        tick.committed_days = reader.u32()?;
        tick.breaks = reader.u32()?;
        tick.refused_prices = reader.u64()?;
        let entries = reader.varint()?;
        let mut previous = 0_u64;
        for _ in 0..entries {
            let g = reader.varint()?;
            let days = u32::try_from(reader.varint()?)
                .ok()
                .filter(|days| *days > 0)
                .ok_or(invalid("tick_days"))?;
            if g <= previous || i64::try_from(g).is_err() {
                return Err(invalid("tick_order"));
            }
            previous = g;
            tick.days.push((g, days));
            tick.gcd_days = tick
                .gcd_days
                .checked_add(days)
                .ok_or(invalid("tick_days"))?;
        }
        if tick.gcd_days > tick.committed_days {
            return Err(invalid("tick_days"));
        }
        tick.mode = tick
            .days
            .iter()
            .copied()
            .max_by_key(|(g, count)| (*count, *g));
        reader.end()?;
        Ok(tick)
    }
}

/// Whether a span's decision prices are on an adjusted basis (DATA-1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Basis {
    /// Unadjusted: the decision series' own tick is observable.
    Unadjusted,
    /// Adjusted by DATA-1's per-stock-day factor pair: the decision tick is
    /// unobservable, and the GDFL raw tick converted by the pair is used.
    Adjusted {
        /// The Zerodha close of the pair, in paisa.
        zerodha_close: i64,
        /// The GDFL close of the pair, in paisa.
        gdfl_close: i64,
    },
}

/// The tick a decision reads (FM-F014).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DecisionTick {
    /// The decision series' own observed tick, in paisa.
    Observed {
        /// The tick.
        tick: i64,
    },
    /// The decision tick is unobservable on an adjusted span; this is the GDFL
    /// raw tick converted exactly to the Zerodha basis, `num / den` paisa,
    /// reduced.
    Unobservable {
        /// The converted tick's numerator.
        num: u128,
        /// The converted tick's denominator.
        den: u128,
    },
}

/// The tick a decision on `basis` reads: the decision series' own on an
/// unadjusted span; on an adjusted one, the raw series' tick times
/// `zerodha_close / gdfl_close`, exactly, labelled unobservable.
///
/// # Errors
///
/// [`HistRefusal::NoTick`] from the series read, and
/// [`HistRefusal::PriceNotPositive`] for a factor price that is not positive.
pub fn decision_tick(
    decision: &ObservedTick,
    basis: Basis,
    raw: &ObservedTick,
) -> Result<DecisionTick, HistRefusal> {
    match basis {
        Basis::Unadjusted => Ok(DecisionTick::Observed {
            tick: decision.tick()?,
        }),
        Basis::Adjusted {
            zerodha_close,
            gdfl_close,
        } => {
            let zerodha = positive(zerodha_close)?;
            let gdfl = positive(gdfl_close)?;
            let raw_tick = u128::from(positive(raw.tick()?)?);
            let num = raw_tick * u128::from(zerodha);
            let den = u128::from(gdfl);
            let common = gcd_wide(num, den);
            Ok(DecisionTick::Unobservable {
                num: num / common,
                den: den / common,
            })
        }
    }
}

/// A positive price as a magnitude.
fn positive(price: i64) -> Result<u64, HistRefusal> {
    u64::try_from(price)
        .ok()
        .filter(|price| *price > 0)
        .ok_or(HistRefusal::PriceNotPositive { price })
}

/// The greatest common divisor of two wide magnitudes.
const fn gcd_wide(mut a: u128, mut b: u128) -> u128 {
    while b != 0 {
        let rest = a % b;
        a = b;
        b = rest;
    }
    a
}

/// How an observed tick compares with a reference tick (the report column
/// against `costs::rate::TICK`; `indicators` cannot name `costs`, so the
/// caller passes it).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TickComparison {
    /// The same tick.
    Equal,
    /// Finer than the reference.
    Finer,
    /// Coarser than the reference.
    Coarser,
    /// No observed tick.
    NoTick,
}

/// Counts of [`TickComparison`]s, for the report only: plain fields.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TickCensus {
    /// Series whose tick equals the reference.
    pub equal: u64,
    /// Series whose tick is finer.
    pub finer: u64,
    /// Series whose tick is coarser.
    pub coarser: u64,
    /// Series with no tick.
    pub no_tick: u64,
}

impl TickCensus {
    /// Every count at zero.
    #[must_use]
    pub const fn zero() -> Self {
        Self {
            equal: 0,
            finer: 0,
            coarser: 0,
            no_tick: 0,
        }
    }

    /// Compares one series with `reference` and counts the answer.
    pub fn record(&mut self, series: &ObservedTick, reference: i64) -> TickComparison {
        let comparison = match series.tick() {
            Err(_) => TickComparison::NoTick,
            Ok(tick) if tick == reference => TickComparison::Equal,
            Ok(tick) if tick < reference => TickComparison::Finer,
            Ok(_) => TickComparison::Coarser,
        };
        let count = match comparison {
            TickComparison::Equal => &mut self.equal,
            TickComparison::Finer => &mut self.finer,
            TickComparison::Coarser => &mut self.coarser,
            TickComparison::NoTick => &mut self.no_tick,
        };
        *count = count.saturating_add(1);
        comparison
    }
}

/// How a day's GDFL seconds ended (FM-F018).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DayFeed {
    /// The day's execution feed is whole: its longest gap is observed.
    Clean,
    /// The day is an `execution_feed_hole` day: its longest gap is never
    /// observed, so a hole cannot raise the threshold.
    ExecutionFeedHole,
}

/// One day's longest gap between consecutive GDFL seconds, measured only
/// inside one continuous phase (FM-F018, D-6326). An overnight gap, a halt
/// and a session-window edge are never a gap: [`GapDay::phase_break`] and
/// [`GapDay::end_day`] start a new phase.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GapDay {
    /// The last second of the current phase.
    phase_last: Option<i64>,
    /// The last second of the day, across phases (for the order check).
    day_last: Option<i64>,
    /// The longest gap of the day so far, in seconds.
    longest: Option<u64>,
    /// Seconds refused as out of order.
    refused: u64,
}

impl GapDay {
    /// A day with no second seen.
    #[must_use]
    pub const fn empty() -> Self {
        Self {
            phase_last: None,
            day_last: None,
            longest: None,
            refused: 0,
        }
    }

    /// One GDFL second of the day, in time order (seconds of any epoch).
    ///
    /// # Errors
    ///
    /// [`HistRefusal::SecondOutOfOrder`] for a second at or before the day's
    /// last, counted.
    pub fn second(&mut self, second: i64) -> Result<(), HistRefusal> {
        if let Some(last) = self.day_last.filter(|last| second <= *last) {
            self.refused = self.refused.saturating_add(1);
            return Err(HistRefusal::SecondOutOfOrder { second, last });
        }
        if let Some(last) = self.phase_last {
            let gap = second.abs_diff(last);
            self.longest = Some(self.longest.map_or(gap, |longest| longest.max(gap)));
        }
        self.phase_last = Some(second);
        self.day_last = Some(second);
        Ok(())
    }

    /// A phase ends (a halt, a session-window edge): the next second starts a
    /// new phase, and the gap across is never measured.
    pub const fn phase_break(&mut self) {
        self.phase_last = None;
    }

    /// Seconds refused as out of order.
    #[must_use]
    pub const fn refused(&self) -> u64 {
        self.refused
    }

    /// Ends the day: the longest gap to observe, or `None` for a day with no
    /// gap or an `execution_feed_hole` day. The tracker restarts.
    pub fn end_day(&mut self, feed: DayFeed) -> Option<i64> {
        let longest = self.longest.filter(|_| feed == DayFeed::Clean);
        *self = Self {
            refused: self.refused,
            ..Self::empty()
        };
        longest.and_then(|gap| i64::try_from(gap).ok())
    }
}

/// The silence flag of a gap inside a trade (FM-F018).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Silence {
    /// No clean committed day holds a gap: the flag is unknown, never true.
    Unknown,
    /// Longer than every clean committed day's longest gap (permille 1000).
    Silent,
    /// Within the longest gap a clean committed day held.
    Quiet,
}

/// The main reading of `silence_in_trade`: a gap is silent when it is longer
/// than the longest gap of every clean committed day, read exactly from the
/// stock's `MaxSecondGap` histogram. The second, labelled reading is that
/// histogram's [`OwnPast::median`].
#[must_use]
pub fn silence_in_trade(max_second_gap: &OwnPast, gap_secs: i64) -> Silence {
    match max_second_gap.max() {
        None => Silence::Unknown,
        Some(max) if gap_secs > max => Silence::Silent,
        Some(_) => Silence::Quiet,
    }
}

/// The step of one candle's extreme from the previous candle's, in basis
/// points of the previous extreme, rounded toward zero (FM-F017, D-6327):
/// `|extreme - previous| x 10_000 / previous`, in `i128`. Long passes highs,
/// short passes lows, each into its own direction's histogram, so the
/// tolerance is relative and the short side never reads highs.
///
/// # Errors
///
/// [`HistRefusal::PriceNotPositive`] for a previous extreme that is not
/// positive.
pub fn step_bp(previous: i64, extreme: i64) -> Result<i64, HistRefusal> {
    let base = positive(previous)?;
    let step = u128::from(extreme.abs_diff(previous)) * u128::from(BP_PER_UNIT) / u128::from(base);
    Ok(i64::try_from(step).unwrap_or(i64::MAX))
}

/// Whether `x` touches `edge` within `tolerance_bp` basis points of the edge,
/// exactly: `|x - edge| x 10_000 <= tolerance_bp x edge`, in `i128` (FM-F017).
#[must_use]
pub fn touches(x: i64, edge: i64, tolerance_bp: i64) -> bool {
    let distance = i128::from(x.abs_diff(edge)) * i128::from(BP_PER_UNIT);
    distance <= i128::from(tolerance_bp) * i128::from(edge)
}

/// One measure of the bank. APPEND-ONLY: the declaration order is the bank's
/// layout, and a new measure is a new variant at the end.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Measure {
    /// The base's width, in basis points.
    BaseWidthBp,
    /// The base's length, in candles.
    BaseLength,
    /// Touches of the base edge, at one touch level.
    Touches,
    /// The base's range contraction, in permille.
    SqueezePermille,
    /// The trigger candle's body, in permille.
    TriggerBodyPermille,
    /// Where the trigger closed in its range, in permille.
    TriggerCloseLocPermille,
    /// The stop, in basis points.
    StopBp,
    /// The room to the next level per unit of stop, in thousandths.
    RoomToStopMilli,
    /// The distance to the nearest Fib level, in permille.
    LevelDistancePermille,
    /// Shape score 0.
    Shape0,
    /// Shape score 1.
    Shape1,
    /// Shape score 2.
    Shape2,
    /// Shape score 3.
    Shape3,
    /// Shape score 4.
    Shape4,
    /// Shape score 5.
    Shape5,
    /// Shape score 6.
    Shape6,
    /// Shape score 7.
    Shape7,
    /// Shape score 8.
    Shape8,
    /// Shape score 9.
    Shape9,
    /// Shape score 10.
    Shape10,
    /// Shape score 11.
    Shape11,
    /// The step of a candle's extreme from the previous candle's, in basis
    /// points of the previous extreme ([`step_bp`]): highs for long, lows for
    /// short. Its level-q value is FIB-3's touch tolerance.
    CandleHighStep,
    /// A resolved trade's duration, entry to exit, in seconds (FM-F016).
    TradeDurationSecs,
    /// The stock's absolute move so far at every Zerodha minute close of a
    /// tradable day, in basis points: a market reference only (17:15 IST item 6).
    AbsMoveBp,
    /// A clean day's longest gap between GDFL seconds ([`GapDay`]).
    MaxSecondGap,
    /// The stock's absolute move so far at the direction's own decision
    /// instants, in basis points: the source of the `move_so_far` cuts.
    MoveSoFarBp,
    /// The far-edge excursion of a reference walk, in ticks (17:15 IST item 9).
    FarEdgeExcursionTicks,
    /// The break overshoot of a reference walk, in ticks.
    BreakOvershootTicks,
    /// The time from entry to the best price, in seconds.
    TimeToBestSecs,
    /// The clock of the last peak, in seconds after 09:15.
    LastPeakClockSecs,
    /// The candles of the pullback.
    PullbackCandles,
    /// The run-up to the maximum favourable excursion, in planned milli-R
    /// (FM-F011).
    WinnerRunUpMilliR,
    /// The largest open-profit give-back, in milli-R.
    OpenProfitGiveBackMilliR,
    /// A swing's size, in basis points.
    SwingSizeBp,
    /// The maximum adverse excursion, in milli-R.
    MaeMilliR,
}

/// The dimensions a measure varies over.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Scope {
    /// One cell per stock: no direction, no width.
    Stock,
    /// Per direction and width.
    Width,
    /// Per direction, width and base kind.
    WidthKind,
    /// Per direction, width, base kind and touch level.
    WidthKindLevel,
    /// Per direction, width and entry style.
    WidthStyle,
    /// Per direction, width, entry style and exit variant.
    WidthStyleVariant,
}

impl Measure {
    /// Every measure, in declaration (layout) order.
    pub const ALL: &[Self] = &[
        Self::BaseWidthBp,
        Self::BaseLength,
        Self::Touches,
        Self::SqueezePermille,
        Self::TriggerBodyPermille,
        Self::TriggerCloseLocPermille,
        Self::StopBp,
        Self::RoomToStopMilli,
        Self::LevelDistancePermille,
        Self::Shape0,
        Self::Shape1,
        Self::Shape2,
        Self::Shape3,
        Self::Shape4,
        Self::Shape5,
        Self::Shape6,
        Self::Shape7,
        Self::Shape8,
        Self::Shape9,
        Self::Shape10,
        Self::Shape11,
        Self::CandleHighStep,
        Self::TradeDurationSecs,
        Self::AbsMoveBp,
        Self::MaxSecondGap,
        Self::MoveSoFarBp,
        Self::FarEdgeExcursionTicks,
        Self::BreakOvershootTicks,
        Self::TimeToBestSecs,
        Self::LastPeakClockSecs,
        Self::PullbackCandles,
        Self::WinnerRunUpMilliR,
        Self::OpenProfitGiveBackMilliR,
        Self::SwingSizeBp,
        Self::MaeMilliR,
    ];

    /// The measure's stable name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::BaseWidthBp => "base_width_bp",
            Self::BaseLength => "base_length",
            Self::Touches => "touches",
            Self::SqueezePermille => "squeeze_permille",
            Self::TriggerBodyPermille => "trigger_body_permille",
            Self::TriggerCloseLocPermille => "trigger_close_loc_permille",
            Self::StopBp => "stop_bp",
            Self::RoomToStopMilli => "room_to_stop_milli",
            Self::LevelDistancePermille => "level_distance_permille",
            Self::Shape0 => "shape_0",
            Self::Shape1 => "shape_1",
            Self::Shape2 => "shape_2",
            Self::Shape3 => "shape_3",
            Self::Shape4 => "shape_4",
            Self::Shape5 => "shape_5",
            Self::Shape6 => "shape_6",
            Self::Shape7 => "shape_7",
            Self::Shape8 => "shape_8",
            Self::Shape9 => "shape_9",
            Self::Shape10 => "shape_10",
            Self::Shape11 => "shape_11",
            Self::CandleHighStep => "candle_high_step_bp",
            Self::TradeDurationSecs => "trade_duration_secs",
            Self::AbsMoveBp => "abs_move_bp",
            Self::MaxSecondGap => "max_second_gap",
            Self::MoveSoFarBp => "move_so_far_bp",
            Self::FarEdgeExcursionTicks => "far_edge_excursion_ticks",
            Self::BreakOvershootTicks => "break_overshoot_ticks",
            Self::TimeToBestSecs => "time_to_best_secs",
            Self::LastPeakClockSecs => "last_peak_clock_secs",
            Self::PullbackCandles => "pullback_candles",
            Self::WinnerRunUpMilliR => "winner_run_up_milli_r",
            Self::OpenProfitGiveBackMilliR => "open_profit_give_back_milli_r",
            Self::SwingSizeBp => "swing_size_bp",
            Self::MaeMilliR => "mae_milli_r",
        }
    }

    /// The dimensions the measure varies over.
    #[must_use]
    pub const fn scope(self) -> Scope {
        match self {
            Self::Touches => Scope::WidthKindLevel,
            Self::CandleHighStep | Self::MoveSoFarBp => Scope::Width,
            Self::TradeDurationSecs => Scope::WidthStyleVariant,
            Self::AbsMoveBp | Self::MaxSecondGap => Scope::Stock,
            Self::FarEdgeExcursionTicks
            | Self::BreakOvershootTicks
            | Self::TimeToBestSecs
            | Self::LastPeakClockSecs
            | Self::PullbackCandles
            | Self::WinnerRunUpMilliR
            | Self::OpenProfitGiveBackMilliR
            | Self::SwingSizeBp
            | Self::MaeMilliR => Scope::WidthStyle,
            _ => Scope::WidthKind,
        }
    }

    /// The measure's position in [`Measure::ALL`].
    const fn position(self) -> usize {
        self as usize
    }
}

/// The generated dimensions of one direction's histograms, from its
/// generation version (no `Default`: every value is the run's).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Dims {
    /// The candle widths tried, in whole minutes, strictly increasing.
    pub widths: Vec<u16>,
    /// The entry-style variants.
    pub styles: u16,
    /// The exit variants.
    pub variants: u16,
    /// The touch levels (quantile positions tried).
    pub levels: u16,
}

impl Dims {
    /// The cells one measure of `scope` takes at one width of this direction
    /// (none for a per-stock measure). At most `3 x u16::MAX x u16::MAX`, so
    /// it never overflows a 64-bit `usize`.
    fn per_width(&self, scope: Scope) -> usize {
        let (levels, styles) = (usize::from(self.levels), usize::from(self.styles));
        match scope {
            Scope::Stock => 0,
            Scope::Width => 1,
            Scope::WidthKind => BASE_KINDS,
            Scope::WidthKindLevel => BASE_KINDS * levels,
            Scope::WidthStyle => styles,
            Scope::WidthStyleVariant => styles * usize::from(self.variants),
        }
    }

    /// The cells one measure of `scope` takes in this direction.
    fn cells(&self, scope: Scope) -> Option<usize> {
        self.widths.len().checked_mul(self.per_width(scope))
    }

    /// The cells every measure takes at one width of this direction.
    fn width_cells(&self) -> usize {
        Measure::ALL
            .iter()
            .map(|measure| self.per_width(measure.scope()))
            .sum()
    }

    /// Checks the dimensions.
    fn check(&self) -> Result<(), HistRefusal> {
        if self.widths.is_empty() {
            return Err(HistRefusal::ShapeInvalid {
                what: "widths_empty",
            });
        }
        let increasing = self
            .widths
            .iter()
            .zip(self.widths.iter().skip(1))
            .all(|(a, b)| a < b);
        if !increasing || self.widths.first() == Some(&0) {
            return Err(HistRefusal::ShapeInvalid {
                what: "widths_not_increasing",
            });
        }
        if self.styles == 0 || self.variants == 0 || self.levels == 0 {
            return Err(HistRefusal::ShapeInvalid {
                what: "dimension_zero",
            });
        }
        Ok(())
    }
}

/// The two searches, long first: the order of the bank's tables.
const SEARCHES: [Search; DIRECTIONS] = [Search::Long, Search::Short];

/// How many directions a bank holds.
const DIRECTIONS: usize = [Search::Long, Search::Short].len();

/// The position of a direction in the bank's tables.
const fn side(dir: Search) -> usize {
    match dir {
        Search::Long => 0,
        Search::Short => 1,
    }
}

/// One cell of a bank: a measure and the fields its scope needs, each an
/// index into its direction's generated list. A field the scope does not use
/// must be `None`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CellKey {
    /// The measure.
    pub measure: Measure,
    /// The direction; `None` for a per-stock measure.
    pub dir: Option<Search>,
    /// The width's index in the direction's widths.
    pub width: Option<u16>,
    /// The base kind.
    pub kind: Option<BaseKind>,
    /// The entry-style variant's index.
    pub style: Option<u16>,
    /// The exit variant's index.
    pub variant: Option<u16>,
    /// The touch level's index.
    pub level: Option<u16>,
}

impl CellKey {
    /// A per-stock cell.
    #[must_use]
    pub const fn stock(measure: Measure) -> Self {
        Self {
            measure,
            dir: None,
            width: None,
            kind: None,
            style: None,
            variant: None,
            level: None,
        }
    }

    /// A cell of one direction and width; add the other fields with the
    /// builder methods.
    #[must_use]
    pub const fn at(measure: Measure, dir: Search, width: u16) -> Self {
        Self {
            dir: Some(dir),
            width: Some(width),
            ..Self::stock(measure)
        }
    }

    /// With a base kind.
    #[must_use]
    pub const fn kind(self, kind: BaseKind) -> Self {
        Self {
            kind: Some(kind),
            ..self
        }
    }

    /// With an entry-style variant.
    #[must_use]
    pub const fn style(self, style: u16) -> Self {
        Self {
            style: Some(style),
            ..self
        }
    }

    /// With an exit variant.
    #[must_use]
    pub const fn variant(self, variant: u16) -> Self {
        Self {
            variant: Some(variant),
            ..self
        }
    }

    /// With a touch level.
    #[must_use]
    pub const fn level(self, level: u16) -> Self {
        Self {
            level: Some(level),
            ..self
        }
    }
}

/// One field of a key against its dimension: present exactly when the scope
/// uses it, and inside the dimension.
fn field(
    key: &CellKey,
    name: &'static str,
    value: Option<u16>,
    used: bool,
    count: usize,
) -> Result<usize, HistRefusal> {
    let measure = key.measure.name();
    match (value, used) {
        (None, false) => Ok(0),
        (Some(value), true) if usize::from(value) < count => Ok(usize::from(value)),
        (Some(value), true) => Err(HistRefusal::CellOutOfRange {
            measure,
            field: name,
            value,
            count,
        }),
        _ => Err(HistRefusal::ScopeMismatch {
            measure,
            field: name,
        }),
    }
}

/// The layout of one stock's bank: every measure's cells for each direction,
/// in [`Measure::ALL`] order, long before short (D-6324). Built once per
/// generation version; a cell's index is O(1) mixed-radix arithmetic
/// (`crates/indicators/tests/fib_hist.rs`, bench rows H-F-06 and H-F-07).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BankShape {
    /// The two directions' dimensions, long first.
    dims: [Dims; DIRECTIONS],
    /// Per measure and direction: the first cell.
    starts: Vec<[usize; DIRECTIONS]>,
    /// Cells in all.
    cells: usize,
}

impl BankShape {
    /// A checked shape.
    ///
    /// # Errors
    ///
    /// [`HistRefusal::ShapeInvalid`] for an empty or unordered width list, a
    /// zero dimension, or more cells than a `usize` counts.
    pub fn new(long: Dims, short: Dims) -> Result<Self, HistRefusal> {
        long.check()?;
        short.check()?;
        let dims = [long, short];
        let mut starts = Vec::with_capacity(Measure::ALL.len());
        let mut next = 0_usize;
        let too_many = HistRefusal::ShapeInvalid {
            what: "too_many_cells",
        };
        for measure in Measure::ALL {
            let scope = measure.scope();
            if scope == Scope::Stock {
                starts.push([next, next]);
                next = next.checked_add(1).ok_or(too_many)?;
                continue;
            }
            let mut pair = [0; DIRECTIONS];
            for (start, dim) in pair.iter_mut().zip(&dims) {
                *start = next;
                next = next
                    .checked_add(dim.cells(scope).ok_or(too_many)?)
                    .ok_or(too_many)?;
            }
            starts.push(pair);
        }
        Ok(Self {
            dims,
            starts,
            cells: next,
        })
    }

    /// A direction's dimensions.
    #[must_use]
    pub const fn dims(&self, dir: Search) -> &Dims {
        let [long, short] = &self.dims;
        match dir {
            Search::Long => long,
            Search::Short => short,
        }
    }

    /// Cells in all.
    #[must_use]
    pub const fn cell_count(&self) -> usize {
        self.cells
    }

    /// The bytes the bank takes before any bin is split.
    #[must_use]
    pub fn unsplit_bytes(&self) -> u64 {
        bytes_of(self.cells)
    }

    /// The index of a cell.
    ///
    /// # Errors
    ///
    /// [`HistRefusal::ScopeMismatch`] for a field present where the measure's
    /// scope has none or absent where it has one, and
    /// [`HistRefusal::CellOutOfRange`] for a field past its dimension.
    pub fn index(&self, key: &CellKey) -> Result<usize, HistRefusal> {
        let scope = key.measure.scope();
        let stock = scope == Scope::Stock;
        let dir = match key.dir {
            None if stock => Search::Long,
            Some(dir) if !stock => dir,
            _ => {
                return Err(HistRefusal::ScopeMismatch {
                    measure: key.measure.name(),
                    field: "dir",
                });
            }
        };
        let dims = self.dims(dir);
        let kinds = matches!(scope, Scope::WidthKind | Scope::WidthKindLevel);
        let styles = matches!(scope, Scope::WidthStyle | Scope::WidthStyleVariant);
        let width = field(key, "width", key.width, !stock, dims.widths.len())?;
        let kind = key.kind.map(|kind| u16::from(kind.cube_value()));
        let kind = field(key, "kind", kind, kinds, BASE_KINDS)?;
        let level_count = usize::from(dims.levels);
        let level = field(
            key,
            "level",
            key.level,
            scope == Scope::WidthKindLevel,
            level_count,
        )?;
        let style = field(key, "style", key.style, styles, usize::from(dims.styles))?;
        let variant_count = usize::from(dims.variants);
        let variant = field(
            key,
            "variant",
            key.variant,
            scope == Scope::WidthStyleVariant,
            variant_count,
        )?;
        let offset = match scope {
            Scope::Stock | Scope::Width => width,
            Scope::WidthKind => width * BASE_KINDS + kind,
            Scope::WidthKindLevel => (width * BASE_KINDS + kind) * level_count + level,
            Scope::WidthStyle => width * usize::from(dims.styles) + style,
            Scope::WidthStyleVariant => {
                (width * usize::from(dims.styles) + style) * variant_count + variant
            }
        };
        let start = self
            .starts
            .get(key.measure.position())
            .map_or(0, |pair| pair.get(side(dir)).copied().unwrap_or(0));
        Ok(start + offset)
    }

    /// Every cell's key, in index order: the bank's layout written out once.
    #[must_use]
    pub fn keys(&self) -> Vec<CellKey> {
        let mut keys = Vec::with_capacity(self.cells);
        for measure in Measure::ALL {
            if measure.scope() == Scope::Stock {
                keys.push(CellKey::stock(*measure));
                continue;
            }
            for dir in SEARCHES {
                self.keys_of(*measure, dir, &mut keys);
            }
        }
        keys
    }

    /// One measure's keys in one direction, in index order.
    fn keys_of(&self, measure: Measure, dir: Search, keys: &mut Vec<CellKey>) {
        let dims = self.dims(dir);
        let kinds = [BaseKind::Micro2, BaseKind::Micro3, BaseKind::Rest];
        for width in (0_u16..).take(dims.widths.len()) {
            let base = CellKey::at(measure, dir, width);
            match measure.scope() {
                Scope::Stock | Scope::Width => keys.push(base),
                Scope::WidthKind => keys.extend(kinds.map(|kind| base.kind(kind))),
                Scope::WidthKindLevel => {
                    for kind in kinds {
                        keys.extend((0..dims.levels).map(|level| base.kind(kind).level(level)));
                    }
                }
                Scope::WidthStyle => keys.extend((0..dims.styles).map(|style| base.style(style))),
                Scope::WidthStyleVariant => {
                    for style in 0..dims.styles {
                        keys.extend(
                            (0..dims.variants).map(|variant| base.style(style).variant(variant)),
                        );
                    }
                }
            }
        }
    }

    /// Every width of either direction, ascending, once each, with the cells
    /// it takes across both directions: one merge walk of the two lists.
    fn width_cells(&self) -> Vec<(u16, usize)> {
        let [long, short] = &self.dims;
        let (long_cells, short_cells) = (long.width_cells(), short.width_cells());
        let mut widths = Vec::with_capacity(long.widths.len() + short.widths.len());
        let (mut a, mut b) = (
            long.widths.iter().peekable(),
            short.widths.iter().peekable(),
        );
        loop {
            let next = match (a.peek().copied(), b.peek().copied()) {
                (Some(x), Some(y)) if x < y => a.next().map(|w| (*w, long_cells)),
                (Some(x), Some(y)) if x > y => b.next().map(|w| (*w, short_cells)),
                (Some(_), Some(_)) => {
                    b.next();
                    a.next().map(|w| (*w, long_cells + short_cells))
                }
                (Some(_), None) => a.next().map(|w| (*w, long_cells)),
                (None, Some(_)) => b.next().map(|w| (*w, short_cells)),
                (None, None) => None,
            };
            let Some(width) = next else {
                return widths;
            };
            widths.push(width);
        }
    }
}

/// The bytes of `cells` unsplit histograms.
fn bytes_of(cells: usize) -> u64 {
    u64::try_from(cells)
        .unwrap_or(u64::MAX)
        .saturating_mul(u64::try_from(UNSPLIT_CELL_BYTES).unwrap_or(u64::MAX))
}

/// The width passes a run takes when every stock's bank does not fit at once
/// (17:15 IST item 3): each width in exactly one pass, the per-stock
/// histograms in every pass.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WidthPasses {
    /// Every width of either direction, ascending.
    pub widths: Vec<u16>,
    /// The pass of each width, aligned with `widths`.
    pub pass_of: Vec<u16>,
    /// The bytes of each pass, the per-stock histograms included.
    pub pass_bytes: Vec<u64>,
    /// The bytes of the per-stock histograms of every stock, in every pass.
    pub fixed_bytes: u64,
}

/// Plans the width passes of `stocks` banks of `shape` within `budget` bytes
/// (R10's measured memory): one pass when everything fits, otherwise the
/// fewest passes the widths can be spread over, largest first into the
/// lightest pass, so the passes cost about the same. O(widths x passes), once
/// per run.
///
/// # Errors
///
/// [`HistRefusal::HistbankWidthOverMemory`] only when one width alone, beside
/// the per-stock histograms, does not fit.
pub fn plan_width_passes(
    shape: &BankShape,
    stocks: u64,
    budget: u64,
) -> Result<WidthPasses, HistRefusal> {
    let (widths, cells): (Vec<u16>, Vec<usize>) = shape.width_cells().into_iter().unzip();
    let stock_cells = Measure::ALL
        .iter()
        .filter(|measure| measure.scope() == Scope::Stock)
        .count();
    let fixed_bytes = bytes_of(stock_cells).saturating_mul(stocks);
    let costs: Vec<u64> = cells
        .iter()
        .map(|cells| bytes_of(*cells).saturating_mul(stocks))
        .collect();
    let capacity = budget.saturating_sub(fixed_bytes);
    for (minutes, cost) in widths.iter().zip(&costs) {
        if *cost > capacity {
            return Err(HistRefusal::HistbankWidthOverMemory {
                minutes: *minutes,
                need: *cost,
                budget: capacity,
            });
        }
    }
    let total: u64 = costs.iter().sum();
    let mut passes = total.div_ceil(capacity.max(1)).max(1);
    loop {
        let mut loads = vec![0_u64; index(passes)];
        let mut pass_of = vec![0_u16; widths.len()];
        let mut order: Vec<usize> = (0..widths.len()).collect();
        order.sort_by_key(|at| core::cmp::Reverse(costs.get(*at).copied().unwrap_or(0)));
        let mut fits = true;
        for at in order {
            let cost = costs.get(at).copied().unwrap_or(0);
            let lightest = loads
                .iter()
                .enumerate()
                .min_by_key(|(_, load)| **load)
                .map_or(0, |(pass, _)| pass);
            change_at(&mut loads, lightest, |load| {
                *load += cost;
                fits &= *load <= capacity;
            });
            change_at(&mut pass_of, at, |slot| {
                *slot = u16::try_from(lightest).unwrap_or(u16::MAX);
            });
        }
        if fits {
            return Ok(WidthPasses {
                widths,
                pass_of,
                pass_bytes: loads.iter().map(|load| load + fixed_bytes).collect(),
                fixed_bytes,
            });
        }
        passes += 1;
    }
}

/// Reads one direction's dimensions from a bank body.
fn read_dims(reader: &mut Reader<'_>) -> Result<Dims, HistRefusal> {
    let small = |value: u64| u16::try_from(value).ok().ok_or(invalid("shape"));
    let count = reader.varint()?;
    let mut widths = Vec::new();
    for _ in 0..count {
        widths.push(small(reader.varint()?)?);
    }
    Ok(Dims {
        widths,
        styles: small(reader.varint()?)?,
        variants: small(reader.varint()?)?,
        levels: small(reader.varint()?)?,
    })
}

/// One stock's own-past histograms (D-6324): a boxed slice of [`OwnPast`]
/// cells laid out by a [`BankShape`], pre-sized once per generation version.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HistBank {
    /// The layout.
    shape: BankShape,
    /// The cells, in index order.
    cells: Box<[OwnPast]>,
}

impl HistBank {
    /// An empty bank of `shape`.
    #[must_use]
    pub fn new(shape: BankShape) -> Self {
        let cells = (0..shape.cells).map(|_| OwnPast::empty()).collect();
        Self { shape, cells }
    }

    /// The layout.
    #[must_use]
    pub const fn shape(&self) -> &BankShape {
        &self.shape
    }

    /// One cell.
    ///
    /// # Errors
    ///
    /// As [`BankShape::index`].
    pub fn cell(&self, key: &CellKey) -> Result<&OwnPast, HistRefusal> {
        let at = self.shape.index(key)?;
        self.cells.get(at).ok_or(HistRefusal::ShapeInvalid {
            what: "cell_missing",
        })
    }

    /// One cell, to observe into.
    ///
    /// # Errors
    ///
    /// As [`BankShape::index`].
    pub fn cell_mut(&mut self, key: &CellKey) -> Result<&mut OwnPast, HistRefusal> {
        let at = self.shape.index(key)?;
        self.cells.get_mut(at).ok_or(HistRefusal::ShapeInvalid {
            what: "cell_missing",
        })
    }

    /// Stages one value in one cell.
    ///
    /// # Errors
    ///
    /// As [`HistBank::cell_mut`] and [`OwnPast::observe`].
    pub fn observe(&mut self, key: &CellKey, value: i64) -> Result<(), HistRefusal> {
        self.cell_mut(key)?.observe(value)
    }

    /// Commits today in every cell: a cell that staged nothing still counts
    /// the day, so its own past holds the day with no observation.
    ///
    /// # Errors
    ///
    /// [`HistRefusal::DaysFull`] when any cell is full; nothing is committed.
    pub fn commit_day(&mut self) -> Result<(), HistRefusal> {
        if self
            .cells
            .iter()
            .any(|cell| cell.committed_days == u32::MAX)
        {
            return Err(HistRefusal::DaysFull);
        }
        for cell in &mut self.cells {
            cell.commit_day()?;
        }
        Ok(())
    }

    /// Drops today in every cell.
    pub fn discard_day(&mut self) {
        for cell in &mut self.cells {
            cell.discard_day();
        }
    }

    /// The bytes the bank holds, split bins included.
    #[must_use]
    pub fn bytes(&self) -> u64 {
        self.cells
            .iter()
            .map(|cell| u64::try_from(cell.bytes()).unwrap_or(u64::MAX))
            .sum()
    }

    /// The bank re-laid on a shape that EXTENDS this one — a width added at a
    /// step, or a style, variant or level appended — keeping every existing
    /// cell. A new cell starts with no own past; the caller builds it once
    /// from that width's committed history, then incrementally.
    ///
    /// # Errors
    ///
    /// [`HistRefusal::ShapeNotExtension`] when `shape` drops a width or
    /// shrinks a dimension.
    pub fn regrow(self, shape: BankShape) -> Result<Self, HistRefusal> {
        let mut maps = [Vec::new(), Vec::new()];
        for (map, dir) in maps.iter_mut().zip(SEARCHES) {
            let (old, new) = (self.shape.dims(dir), shape.dims(dir));
            if new.styles < old.styles || new.variants < old.variants || new.levels < old.levels {
                return Err(HistRefusal::ShapeNotExtension { what: "dimension" });
            }
            let mut newer = (0_u16..).zip(&new.widths);
            for minutes in &old.widths {
                match newer.find(|(_, width)| *width >= minutes) {
                    Some((at, width)) if width == minutes => map.push(at),
                    _ => return Err(HistRefusal::ShapeNotExtension { what: "width" }),
                }
            }
        }
        let mut cells: Vec<OwnPast> = (0..shape.cells).map(|_| OwnPast::empty()).collect();
        for (key, cell) in self.shape.keys().into_iter().zip(self.cells) {
            let mut moved = key;
            if let (Some(dir), Some(width)) = (key.dir, key.width) {
                let map = maps.get(side(dir)).map_or(&[][..], Vec::as_slice);
                moved.width = map.get(usize::from(width)).copied();
            }
            let at = shape.index(&moved)?;
            change_at(&mut cells, at, |slot| *slot = cell);
        }
        Ok(Self {
            shape,
            cells: cells.into_boxed_slice(),
        })
    }

    /// Appends the bank's checkpoint body (D-6325): [`BANK_MAGIC`],
    /// [`CODEC_V1`], the shape (each direction's widths, styles, variants and
    /// levels), then every cell's [`OwnPast::encode`] body in index order.
    ///
    /// # Errors
    ///
    /// [`HistRefusal::StagedNotEmpty`] from any cell.
    pub fn encode(&self, out: &mut Vec<u8>) -> Result<(), HistRefusal> {
        out.extend_from_slice(BANK_MAGIC);
        out.extend_from_slice(&CODEC_V1.to_le_bytes());
        for dims in &self.shape.dims {
            put_varint(out, u64::try_from(dims.widths.len()).unwrap_or(u64::MAX));
            for minutes in &dims.widths {
                put_varint(out, u64::from(*minutes));
            }
            put_varint(out, u64::from(dims.styles));
            put_varint(out, u64::from(dims.variants));
            put_varint(out, u64::from(dims.levels));
        }
        put_varint(out, u64::try_from(self.cells.len()).unwrap_or(u64::MAX));
        for cell in &self.cells {
            cell.encode(out)?;
        }
        Ok(())
    }

    /// Decodes one bank's checkpoint body, which must be the whole of `bytes`.
    ///
    /// # Errors
    ///
    /// [`HistRefusal::SnapshotInvalid`], naming what is wrong, and a shape
    /// refusal for a shape that does not check.
    pub fn decode(bytes: &[u8]) -> Result<Self, HistRefusal> {
        let mut reader = Reader { bytes, at: 0 };
        reader.magic(BANK_MAGIC)?;
        if reader.u32()? != CODEC_V1 {
            return Err(invalid("version"));
        }
        let long = read_dims(&mut reader)?;
        let short = read_dims(&mut reader)?;
        let shape = BankShape::new(long, short)?;
        if reader.varint()? != u64::try_from(shape.cells).unwrap_or(u64::MAX) {
            return Err(invalid("cells"));
        }
        let mut cells = Vec::with_capacity(shape.cells);
        for _ in 0..shape.cells {
            cells.push(OwnPast::decode_from(&mut reader)?);
        }
        reader.end()?;
        Ok(Self {
            shape,
            cells: cells.into_boxed_slice(),
        })
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "a test that cannot panic cannot fail"
)]
mod tests {
    use super::*;
    use vocab::fib_cube::untyped_numbers;

    /// NO TYPED NUMBERS (operator 16 Oct 11:24 IST): this file's shipping part
    /// holds no numeric literal but 0 and 1.
    #[test]
    fn hist_types_no_number() {
        let typed = untyped_numbers(include_str!("hist.rs"));
        assert!(typed.is_empty(), "hist.rs types {typed:?}");
    }

    #[test]
    fn the_bin_table_is_the_seed_s() {
        assert_eq!(SUB_BITS, 3);
        assert_eq!(OCTAVES, 63);
        assert_eq!(BASE_BINS, 505);
        assert_eq!(base_bin(0), 0);
        assert_eq!(base_bin(1), 1);
        assert_eq!(base_bin(2), 9);
        assert_eq!(base_bin(3), 13);
        assert_eq!(base_bin(7), 23);
        assert_eq!(base_bin(8), 25);
        assert_eq!(base_bin(15), 32);
        assert_eq!(base_bin(16), 33);
        assert_eq!(base_bin(17), 33);
        assert_eq!(base_bin(u64::try_from(i64::MAX).unwrap()), BASE_BINS - 1);
        for bin in [1, 9, 13, 17, 19, 21, 23, 25, 32, 33, 100, BASE_BINS - 1] {
            let edge = lower_edge(bin, 0, 0);
            assert_eq!(base_bin(edge), bin, "bin {bin} edge {edge}");
        }
        assert_eq!(lower_edge(0, 0, 0), 0);
        assert_eq!(lower_edge(33, 1, 1), 17);
        assert_eq!(child_of(17, 1), 1);
        assert_eq!(child_of(16, 1), 0);
    }

    #[test]
    fn every_value_below_two_thousand_lands_at_or_above_its_leaf_s_edge() {
        let past = OwnPast::empty();
        let mut previous = 0;
        for value in 0..2_000_u64 {
            let leaf = past.leaf_of(value);
            assert!(leaf >= previous, "leaves ascend with values");
            previous = leaf;
            let edge = past.leaf_lower_edge(leaf);
            assert!(edge <= value, "{value}");
            assert_eq!(base_bin(edge), leaf, "{value}");
        }
    }

    #[test]
    fn varints_are_canonical_and_bounded() {
        for value in [0, 1, 127, 128, 300, u64::from(u32::MAX), u64::MAX] {
            let mut out = Vec::new();
            put_varint(&mut out, value);
            let mut reader = Reader { bytes: &out, at: 0 };
            assert_eq!(reader.varint().unwrap(), value);
            reader.end().unwrap();
        }
        let not_canonical = [0x80, 0x00];
        let mut reader = Reader {
            bytes: &not_canonical,
            at: 0,
        };
        assert_eq!(reader.varint(), Err(invalid("varint_not_canonical")));
        let too_long = [0xFF; 11];
        let mut reader = Reader {
            bytes: &too_long,
            at: 0,
        };
        assert_eq!(reader.varint(), Err(invalid("varint_too_long")));
        let overflow = [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x02];
        let mut reader = Reader {
            bytes: &overflow,
            at: 0,
        };
        assert_eq!(reader.varint(), Err(invalid("varint_too_long")));
        let mut reader = Reader {
            bytes: &[0x80],
            at: 0,
        };
        assert_eq!(reader.varint(), Err(invalid("truncated")));
        // Ten bytes that fit 64 bits and say another follows: the eleventh
        // would start past the top bit.
        let past_top = [
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x81, 0x00,
        ];
        let mut reader = Reader {
            bytes: &past_top,
            at: 0,
        };
        assert_eq!(reader.varint(), Err(invalid("varint_too_long")));
    }

    #[test]
    fn a_split_table_that_cannot_count_its_leaves_is_none() {
        assert!(SplitTable::of(&[u8::MAX]).is_none());
        assert!(SplitTable::of(&[u8::try_from(u32::BITS - 1).unwrap(); 2]).is_none());
        let table = SplitTable::of(&[0, 1, 0]).unwrap();
        assert_eq!(&*table.leaf_bin, &[0, 1, 1, 2]);
        assert_eq!(table.bin(1), (1, 1));
        assert_eq!(table.bin(7), (index(u64::from(u32::MAX)), 0));
    }

    #[test]
    fn a_checked_write_past_its_slice_changes_nothing() {
        let mut cells = [0_u32, 0];
        change_at(&mut cells, 1, |cell| *cell = 7);
        assert_eq!(cells, [0, 7]);
        change_at(&mut cells, 2, |cell| *cell = 9);
        assert_eq!(cells, [0, 7], "an index past the slice is not written");
    }

    #[test]
    fn work_is_counted_and_never_compared() {
        let mut a = OwnPast::empty();
        let b = OwnPast::empty();
        a.observe(5).unwrap();
        a.discard_day();
        assert!(a.work() > 0);
        assert_eq!(a, b, "the work counter is not state");
        assert_eq!(gcd(0, 0), 0);
        assert_eq!(gcd_wide(12, 18), 6);
    }
}
