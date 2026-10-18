//! The FIB cube schema v1: 24 rows in a namespace of its own, with the values
//! of every measured row and every partition row generated per run.
//!
//! Owned by FIB-1 (D-6300, D-6301, D-6312). Every Fib Flag Breakout candidate
//! lands in exactly one FULL CELL — a [`Coords`], one value per row — and the
//! lattice (FIB-6) grows PARTIAL MIXES over it — a [`MixKey`], where a free row
//! is [`OPEN`].
//!
//! # Why this is not [`crate::table`]
//!
//! The condition table holds 370 of the mask's 384 positions, pinned by
//! `the_version_is_the_widened_table`, and its rows carry condition-table
//! semantics — including the indicator rows the operator banned as Fib
//! conditions. Widening [`crate::mask::WORDS`] to make room would re-key every
//! recorded run (`CLAUDE.md` §3 rule 8). So the cube is a SEPARATE namespace:
//! its row index and its value index are its identity, and [`Layout::to_mask`]
//! places it in the low positions of a [`ConditionMask`] that is **never
//! decoded through [`crate::table`]**. Such a mask means nothing to the
//! condition table; it is for FIB-6's support oracle only.
//!
//! # What the code fixes and what the run generates (operator 16 Oct 17:15 IST)
//!
//! The CODE fixes, under [`FIB_CUBE_VERSION`] and `runner::fib::KNOWN_SCHEMAS`:
//! the rows, their kinds, their order and the DEFINITIONAL value lists (side,
//! `fib_zone`, `level_relation`, `base_holds_extreme`, weekday, `base_kind`,
//! `base_shape`, `trigger_order_flow` and the entry-style names). [`descriptor`]
//! hashes those and nothing else.
//!
//! The RUN generates, per direction, into a versioned [`Generation`]: the cut
//! list of every measured row (R1, R2), the mover-rank edges (item d), the
//! time-of-day cuts (item e), the timeframe set (R7), the exit variant set and
//! the entry-style variants. A row's value count is therefore a property of a
//! [`Generation`], and so is everything that depends on it — the radix of the
//! cell code, the mask positions and both paper counts. [`Layout`] holds them;
//! it is built inside [`Generation::new`], which is where the per-run
//! descriptor is validated (FM-F006). The labels a measured row carries
//! (`low`/`mid`/`high` and the rest) are SEED labels only: names for the start,
//! never thresholds, never hashed.
//!
//! # Append-only, by version
//!
//! The row index is the row's identity forever. A new row or a new definitional
//! value is a NEW schema version and v1 cells stay decodable. A generated
//! version is born only at a walk-forward step boundary and step `m` reads only
//! versions born at or before `m` ([`GenerationChain::at`]).
//!
//! # The only typed numbers
//!
//! [`FIXED_NUMBERS`] is the one ledger of every number the FIB strategy path may
//! type (operator 16 Oct 11:24 IST and 17:15 IST, D-6309). Every numeric literal
//! in this file is one of its values, proved by reading this file with
//! [`numeric_literals`]; every later FIB package proves its own files hold no
//! literal but 0 and 1 with [`untyped_numbers`].
//!
//! # Cost
//!
//! [`Layout::encode`], [`Layout::decode`], [`Layout::validate`],
//! [`Layout::validate_mix`] and [`contains`] each visit the 24 rows once:
//! O(1), a constant of this schema version. [`CutList::bucket`] is one binary
//! search over at most [`MAX_CUTS`] cuts (at most eight comparisons), each
//! cross-multiplied in `u128`. The bench is `crates/vocab/benches/fib_cube.rs`.

use crate::ConditionMask;
use core::cmp::Ordering;

/// The schema version of the FIB cube, and the `vocab_version` term of a FIB
/// run's identity (`runner::fib::identity`). Bumped by a new row or a new
/// definitional value, never by a generated cut.
pub const FIB_CUBE_VERSION: u32 = 1;

/// How many rows schema v1 has. The row index is the row's identity forever.
pub const ROW_COUNT: usize = 24;

/// One full cell: one value index per row, row 0 first. The cell key is this
/// byte array; [`Layout::encode`] is its mixed-radix number.
pub type Coords = [u8; ROW_COUNT];

/// The marker of a free row in a [`MixKey`]: "any value of this row".
///
/// `u8::MAX`. A [`Coords`] never holds it, and a partition row never holds it
/// in a mix (see [`Layout::validate_mix`]).
pub const OPEN: u8 = u8::MAX;

/// The most values one row may hold: every byte below [`OPEN`] but one, so a
/// free row's choices (its values and [`OPEN`]) still count in a `u8`. More is
/// refused by name ([`CubeRefusal::RowValuesExceedU8`]).
pub const MAX_ROW_VALUES: usize = (OPEN - 1) as usize;

/// The most cuts one generated list may hold: one fewer than its values.
pub const MAX_CUTS: usize = MAX_ROW_VALUES - 1;

/// A partial mix: one value per row, or [`OPEN`] for a free row.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct MixKey(pub [u8; ROW_COUNT]);

impl MixKey {
    /// The mix with every non-partition row free and the three partition rows
    /// fixed to `cell`'s values: the root of one partition's lattice.
    #[must_use]
    pub fn root_of(cell: &Coords) -> Self {
        let mut key = [OPEN; ROW_COUNT];
        for ((slot, value), row) in key.iter_mut().zip(cell).zip(&ROWS) {
            if row.kind == RowKind::Partition {
                *slot = *value;
            }
        }
        Self(key)
    }
}

/// How a row's values relate to one another.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RowKind {
    /// Exactly one value per cell; a mix fixing value `v` admits cells with `v`.
    Categorical,
    /// Nested values: a cell holding the narrowest group `j` is in every mix
    /// fixing a group `k >= j` (a `rank_g1` trade also counts in every wider
    /// group). The widest group is every ranked decision.
    Cumulative,
    /// One decision yields one trade per value of this row, so the row is never
    /// [`OPEN`] in a mix: an "any" over it would count one event several times.
    Partition,
}

impl RowKind {
    /// The kind's canonical name, as [`descriptor`] writes it.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Categorical => "categorical",
            Self::Cumulative => "cumulative",
            Self::Partition => "partition",
        }
    }
}

/// Where a row's values come from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Source {
    /// Fixed by this schema version: the labels ARE the values, and
    /// [`descriptor`] hashes them.
    Definitional,
    /// Generated per run from each stock's own past (R1, R2): the value count
    /// comes from the [`Generation`]'s cut lists or rank edges, and the labels
    /// are seed labels only — never a threshold, never hashed.
    Measured,
    /// A partition list generated per run (the exit variants, the entry-style
    /// variants, the timeframe widths). Its labels, when it has any, are the
    /// definitional names a variant carries, and [`descriptor`] hashes them.
    Variants,
}

impl Source {
    /// The source's canonical name, as [`descriptor`] writes it.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Definitional => "definitional",
            Self::Measured => "measured",
            Self::Variants => "variants",
        }
    }
}

/// One row of the schema: its name, its kind, where its values come from and
/// its labels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Row {
    /// The row's canonical name.
    pub name: &'static str,
    /// How its values relate.
    pub kind: RowKind,
    /// Where its values come from.
    pub source: Source,
    /// The labels: the values of a definitional row, the seed labels of a
    /// measured row, the definitional names of a variant row.
    pub labels: &'static [&'static str],
}

/// The rows of schema v1, by row index, by name. Implicit discriminants, so no
/// row index is ever typed: the declaration order IS [`ROWS`]' order, proved at
/// compile time by `ids_in_order`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum RowId {
    /// Row 0: the trade's side against the stock's own move.
    Side,
    /// Row 1: the mover-rank group, nested (the only cumulative row).
    MoverRank,
    /// Row 2: how far the stock has moved so far today.
    MoveSoFar,
    /// Row 3: the nearest Fib rung to the base's far edge.
    FibZone,
    /// Row 4: the distance from the far edge to that rung.
    LevelDistance,
    /// Row 5: where the base sits against that rung.
    LevelRelation,
    /// Row 6: whether a base candle made the day extreme.
    BaseHoldsExtreme,
    /// Row 7: the exit variant (a partition row), from the run's exit set.
    Exit,
    /// Row 8: the IST weekday.
    Weekday,
    /// Row 9: the decision-close instant, cut per timeframe.
    TimeOfDay,
    /// Row 10: micro base of two or three candles, or a rest.
    BaseKind,
    /// Row 11: the base's shape label.
    BaseShape,
    /// Row 12: the base's width against its own past.
    BaseTightness,
    /// Row 13: the base's length against its own past.
    BaseLength,
    /// Row 14: touches of the base edge.
    EdgeTouches,
    /// Row 15: whether the base's ranges contract.
    Squeeze,
    /// Row 16: the trigger candle's body.
    TriggerBody,
    /// Row 17: where the trigger closed within its own range.
    TriggerCloseLocation,
    /// Row 18: the trigger's order flow delta against the trade.
    TriggerOrderFlow,
    /// Row 19: the stop size against its own past.
    StopSize,
    /// Row 20: the room to the next level per unit of stop.
    RoomToStop,
    /// Row 21: the entry-style variant (a partition row).
    EntryStyle,
    /// Row 22: the attempt number of the day.
    Attempt,
    /// Row 23: the candle width in whole minutes (a partition row).
    Timeframe,
}

impl RowId {
    /// Every row, in row-index order.
    pub const ALL: [Self; ROW_COUNT] = [
        Self::Side,
        Self::MoverRank,
        Self::MoveSoFar,
        Self::FibZone,
        Self::LevelDistance,
        Self::LevelRelation,
        Self::BaseHoldsExtreme,
        Self::Exit,
        Self::Weekday,
        Self::TimeOfDay,
        Self::BaseKind,
        Self::BaseShape,
        Self::BaseTightness,
        Self::BaseLength,
        Self::EdgeTouches,
        Self::Squeeze,
        Self::TriggerBody,
        Self::TriggerCloseLocation,
        Self::TriggerOrderFlow,
        Self::StopSize,
        Self::RoomToStop,
        Self::EntryStyle,
        Self::Attempt,
        Self::Timeframe,
    ];

    /// The row index.
    #[must_use]
    pub const fn index(self) -> usize {
        self as usize
    }

    /// The row's definition.
    #[must_use]
    pub const fn row(self) -> &'static Row {
        match self {
            Self::Side => &SIDE_ROW,
            Self::MoverRank => &MOVER_RANK_ROW,
            Self::MoveSoFar => &MOVE_SO_FAR_ROW,
            Self::FibZone => &FIB_ZONE_ROW,
            Self::LevelDistance => &LEVEL_DISTANCE_ROW,
            Self::LevelRelation => &LEVEL_RELATION_ROW,
            Self::BaseHoldsExtreme => &BASE_HOLDS_EXTREME_ROW,
            Self::Exit => &EXIT_ROW,
            Self::Weekday => &WEEKDAY_ROW,
            Self::TimeOfDay => &TIME_OF_DAY_ROW,
            Self::BaseKind => &BASE_KIND_ROW,
            Self::BaseShape => &BASE_SHAPE_ROW,
            Self::BaseTightness => &BASE_TIGHTNESS_ROW,
            Self::BaseLength => &BASE_LENGTH_ROW,
            Self::EdgeTouches => &EDGE_TOUCHES_ROW,
            Self::Squeeze => &SQUEEZE_ROW,
            Self::TriggerBody => &TRIGGER_BODY_ROW,
            Self::TriggerCloseLocation => &TRIGGER_CLOSE_LOCATION_ROW,
            Self::TriggerOrderFlow => &TRIGGER_ORDER_FLOW_ROW,
            Self::StopSize => &STOP_SIZE_ROW,
            Self::RoomToStop => &ROOM_TO_STOP_ROW,
            Self::EntryStyle => &ENTRY_STYLE_ROW,
            Self::Attempt => &ATTEMPT_ROW,
            Self::Timeframe => &TIMEFRAME_ROW,
        }
    }

    /// The value index of a DEFINITIONAL row's value labelled `name`, or
    /// `None` (also for every generated row, whose labels are not values).
    ///
    /// O(values of the row): for setting up a caller's own table once, never
    /// for a per-trade path.
    #[must_use]
    pub fn value_of(self, name: &str) -> Option<u8> {
        let row = self.row();
        if row.source != Source::Definitional {
            return None;
        }
        (0_u8..)
            .zip(row.labels)
            .find_map(|(index, label)| (*label == name).then_some(index))
    }
}

/// Defines one row as a named constant, so [`RowId::row`] reaches it by name
/// and no row is ever reached by a computed index.
macro_rules! row {
    ($const:ident, $name:literal, $kind:ident, $source:ident, [$($label:literal),* $(,)?]) => {
        #[doc = concat!("The `", $name, "` row.")]
        pub const $const: Row = Row {
            name: $name,
            kind: RowKind::$kind,
            source: Source::$source,
            labels: &[$($label),*],
        };
    };
}

row!(
    SIDE_ROW,
    "side",
    Categorical,
    Definitional,
    [
        "long_with_gainer",
        "short_with_loser",
        "short_mirror_on_gainer",
        "long_mirror_on_loser",
    ]
);
row!(
    MOVER_RANK_ROW,
    "mover_rank",
    Cumulative,
    Measured,
    ["rank_g1", "rank_g2"]
);
row!(
    MOVE_SO_FAR_ROW,
    "move_so_far",
    Categorical,
    Measured,
    ["low", "mid", "high"]
);
row!(
    FIB_ZONE_ROW,
    "fib_zone",
    Categorical,
    Definitional,
    ["l0", "l236", "l382", "l500", "l618", "l786", "l1000"]
);
row!(
    LEVEL_DISTANCE_ROW,
    "level_distance",
    Categorical,
    Measured,
    ["on", "near", "far"]
);
row!(
    LEVEL_RELATION_ROW,
    "level_relation",
    Categorical,
    Definitional,
    ["above", "below", "spans"]
);
row!(
    BASE_HOLDS_EXTREME_ROW,
    "base_holds_extreme",
    Categorical,
    Definitional,
    ["no", "yes"]
);
row!(EXIT_ROW, "exit", Partition, Variants, []);
row!(
    WEEKDAY_ROW,
    "weekday",
    Categorical,
    Definitional,
    ["mon", "tue", "wed", "thu", "fri"]
);
row!(TIME_OF_DAY_ROW, "time_of_day", Categorical, Measured, []);
row!(
    BASE_KIND_ROW,
    "base_kind",
    Categorical,
    Definitional,
    ["micro2", "micro3", "rest"]
);
row!(
    BASE_SHAPE_ROW,
    "base_shape",
    Categorical,
    Definitional,
    [
        "u",
        "inverted_u",
        "flat",
        "triangle",
        "inside_candles",
        "tight_closes",
        "rising_lows",
        "shakeout_reclaim",
        "v_bottom",
        "partial_u",
        "inside_pair",
        "stair",
    ]
);
row!(
    BASE_TIGHTNESS_ROW,
    "base_tightness",
    Categorical,
    Measured,
    ["tight", "normal", "wide"]
);
row!(
    BASE_LENGTH_ROW,
    "base_length",
    Categorical,
    Measured,
    ["short", "normal", "long"]
);
row!(
    EDGE_TOUCHES_ROW,
    "edge_touches",
    Categorical,
    Measured,
    ["few", "many"]
);
row!(SQUEEZE_ROW, "squeeze", Categorical, Measured, ["yes", "no"]);
row!(
    TRIGGER_BODY_ROW,
    "trigger_body",
    Categorical,
    Measured,
    ["small", "normal", "big"]
);
row!(
    TRIGGER_CLOSE_LOCATION_ROW,
    "trigger_close_location",
    Categorical,
    Measured,
    ["near_extreme", "middle", "far"]
);
row!(
    TRIGGER_ORDER_FLOW_ROW,
    "trigger_order_flow",
    Categorical,
    Definitional,
    ["with", "against", "balanced", "unclassified", "absent"]
);
row!(
    STOP_SIZE_ROW,
    "stop_size",
    Categorical,
    Measured,
    ["small", "normal", "large"]
);
row!(
    ROOM_TO_STOP_ROW,
    "room_to_stop",
    Categorical,
    Measured,
    ["low", "mid", "high"]
);
row!(
    ENTRY_STYLE_ROW,
    "entry_style",
    Partition,
    Variants,
    ["close", "break", "retest"]
);
row!(
    ATTEMPT_ROW,
    "attempt",
    Categorical,
    Measured,
    ["first", "second_plus"]
);
row!(TIMEFRAME_ROW, "timeframe", Partition, Variants, []);

/// The 24 rows of schema v1, in canonical order.
pub const ROWS: [Row; ROW_COUNT] = [
    SIDE_ROW,
    MOVER_RANK_ROW,
    MOVE_SO_FAR_ROW,
    FIB_ZONE_ROW,
    LEVEL_DISTANCE_ROW,
    LEVEL_RELATION_ROW,
    BASE_HOLDS_EXTREME_ROW,
    EXIT_ROW,
    WEEKDAY_ROW,
    TIME_OF_DAY_ROW,
    BASE_KIND_ROW,
    BASE_SHAPE_ROW,
    BASE_TIGHTNESS_ROW,
    BASE_LENGTH_ROW,
    EDGE_TOUCHES_ROW,
    SQUEEZE_ROW,
    TRIGGER_BODY_ROW,
    TRIGGER_CLOSE_LOCATION_ROW,
    TRIGGER_ORDER_FLOW_ROW,
    STOP_SIZE_ROW,
    ROOM_TO_STOP_ROW,
    ENTRY_STYLE_ROW,
    ATTEMPT_ROW,
    TIMEFRAME_ROW,
];

/// Whether every id of `ids` has its position as its row index and names the
/// row `rows` holds there, and the two lists have one length; asserted at
/// compile time over [`RowId::ALL`] and [`ROWS`].
const fn ids_in_order(ids: &[RowId], rows: &[Row]) -> bool {
    let (mut ids, mut rows) = (ids, rows);
    let mut expected = 0;
    let mut ordered = true;
    while let ([id, rest @ ..], [row, more @ ..]) = (ids, rows) {
        ordered &= id.index() == expected;
        ordered &= same_name(id.row().name, row.name);
        expected += 1;
        ids = rest;
        rows = more;
    }
    ordered && ids.is_empty() && rows.is_empty()
}

/// Byte equality of two names, usable at compile time.
const fn same_name(a: &str, b: &str) -> bool {
    let (mut a, mut b) = (a.as_bytes(), b.as_bytes());
    while let ([x, xs @ ..], [y, ys @ ..]) = (a, b) {
        if *x != *y {
            return false;
        }
        a = xs;
        b = ys;
    }
    a.is_empty() && b.is_empty()
}

const _: () = assert!(ids_in_order(&RowId::ALL, &ROWS));

/// The number of entry-style names a variant may carry.
pub const ENTRY_STYLE_NAMES: usize = ENTRY_STYLE_ROW.labels.len();

/// The number of base kinds a cut list may be scoped to.
pub const BASE_KINDS: usize = BASE_KIND_ROW.labels.len();

/// A refusal of this module, by name. Nothing here degrades silently.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CubeRefusal {
    /// A value index at or above its row's value count in this layout.
    ValueOutOfRange {
        /// The row's name.
        row: &'static str,
        /// The offending value index.
        value: u8,
        /// How many values the row has.
        values: usize,
    },
    /// [`OPEN`] inside a full cell, where every row must hold a value.
    OpenInCell {
        /// The row's name.
        row: &'static str,
    },
    /// [`OPEN`] on a partition row of a mix.
    OpenPartitionRow {
        /// The row's name.
        row: &'static str,
    },
    /// A code at or above the layout's paper full-cell count.
    CodeOutOfRange {
        /// The offending code.
        code: u128,
    },
    /// An own-past permille above one thousand.
    PermilleOutOfRange {
        /// The offending permille.
        permille: u16,
    },
    /// A retracement outside `0..=whole`, or a non-positive whole.
    RetraceOutOfRange {
        /// The retracement's part.
        part: i64,
        /// The whole it is a part of.
        whole: i64,
    },
    /// A ratio with a zero denominator.
    RatioZeroDenominator {
        /// The numerator offered.
        num: u64,
    },
    /// A mid-rank pair whose counts do not describe a position in a sample.
    MidRankInvalid {
        /// The count below the value's bin.
        below: u64,
        /// The count in the value's bin.
        in_bin: u64,
        /// The sample's total.
        total: u64,
    },
    /// A quantile outside `(0, 1]`.
    QuantileInvalid {
        /// The quantile's numerator.
        num: u64,
        /// The quantile's denominator.
        den: u64,
    },
    /// A quantile of an empty sample.
    EmptySample,
    /// Rank edges that start below rank one or descend.
    RankEdgesInvalid {
        /// The position of the first offending edge.
        at: usize,
        /// That edge.
        edge: u16,
    },
    /// No timeframe width in the generation.
    TimeframesEmpty,
    /// A width that is zero, past the hard top, or not above the one before.
    TimeframesInvalid {
        /// The position of the offending width.
        at: usize,
        /// That width, in minutes.
        minutes: u16,
    },
    /// A cut list on a row whose values are not cut lists.
    CutOnUnmeasuredRow {
        /// The row's name.
        row: &'static str,
    },
    /// A cut list scoped to a width the generation does not hold.
    CutTimeframeUnknown {
        /// The row's name.
        row: &'static str,
        /// The width, in minutes.
        minutes: u16,
    },
    /// A cut list scoped to a base kind that does not exist.
    CutBaseKindOutOfRange {
        /// The row's name.
        row: &'static str,
        /// The base kind offered.
        base_kind: u8,
    },
    /// Cuts that are not strictly increasing.
    CutsNotIncreasing {
        /// The row's name.
        row: &'static str,
        /// The position of the first cut not above the one before.
        at: usize,
    },
    /// A time-of-day cut list with no width to snap to.
    TimeCutsWithoutTimeframe,
    /// A time-of-day cut that is not a positive whole number of the width's
    /// candles inside the session's decision span.
    TimeCutOffGrid {
        /// The width, in minutes.
        minutes: u16,
        /// The position of the offending cut.
        at: usize,
    },
    /// Cut lists out of canonical (row, width, base kind) order, or repeated.
    CutListsNotCanonical {
        /// The row of the first offending list.
        row: &'static str,
    },
    /// A measured row with no cut list at all.
    MeasuredRowWithoutCuts {
        /// The row's name.
        row: &'static str,
    },
    /// A row with more values than a `u8` can index beside [`OPEN`].
    RowValuesExceedU8 {
        /// The row's name.
        row: &'static str,
        /// How many values it would have.
        values: usize,
    },
    /// A radix product beyond the widest key type (`u128`).
    RadixProductExceedsKey,
    /// A paper count beyond `u128`.
    PaperCountExceedsU128,
    /// More values than the mask has positions.
    FibMaskExceedsWords {
        /// How many values the layout has.
        values: usize,
        /// How many positions the mask has.
        bits: u32,
    },
    /// An exit set with no variant.
    ExitSetEmpty,
    /// No entry-style variant.
    EntryVariantsEmpty,
    /// An entry-style variant naming a style that does not exist.
    EntryStyleUnknown {
        /// The style offered.
        style: u8,
    },
    /// Generated lists out of name order, or repeated.
    GeneratedListsNotCanonical {
        /// The position of the first offending list.
        at: usize,
    },
    /// A generated list born after the generation that holds it.
    ListBornAfterGeneration {
        /// The position of the offending list.
        at: usize,
    },
    /// A version pushed onto another direction's chain.
    ChainSearchMismatch,
    /// A version whose number is not the next one.
    ChainVersionNotNext {
        /// The version expected.
        expected: u32,
        /// The version offered.
        got: u32,
    },
    /// A version born at or before the previous one's step.
    ChainBirthNotLater {
        /// The previous version's birth step.
        previous: u32,
        /// The offered version's birth step.
        got: u32,
    },
    /// A gate year with no tradable day.
    GateYearWithoutDays,
    /// A mix fixing a side value of the other search (operator 16 Oct 16:04 IST).
    SideOutsideSearch {
        /// The search's name.
        search: &'static str,
        /// The side value's label.
        side: &'static str,
    },
}

impl CubeRefusal {
    /// The refusal's stable name, for a counter or a report.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::ValueOutOfRange { .. } => "value_out_of_range",
            Self::OpenInCell { .. } => "open_in_cell",
            Self::OpenPartitionRow { .. } => "open_partition_row",
            Self::CodeOutOfRange { .. } => "code_out_of_range",
            Self::PermilleOutOfRange { .. } => "permille_out_of_range",
            Self::RetraceOutOfRange { .. } => "retrace_out_of_range",
            Self::RatioZeroDenominator { .. } => "ratio_zero_denominator",
            Self::MidRankInvalid { .. } => "mid_rank_invalid",
            Self::QuantileInvalid { .. } => "quantile_invalid",
            Self::EmptySample => "empty_sample",
            Self::RankEdgesInvalid { .. } => "rank_edges_invalid",
            Self::TimeframesEmpty => "timeframes_empty",
            Self::TimeframesInvalid { .. } => "timeframes_invalid",
            Self::CutOnUnmeasuredRow { .. } => "cut_on_unmeasured_row",
            Self::CutTimeframeUnknown { .. } => "cut_timeframe_unknown",
            Self::CutBaseKindOutOfRange { .. } => "cut_base_kind_out_of_range",
            Self::CutsNotIncreasing { .. } => "cuts_not_increasing",
            Self::TimeCutsWithoutTimeframe => "time_cuts_without_timeframe",
            Self::TimeCutOffGrid { .. } => "time_cut_off_grid",
            Self::CutListsNotCanonical { .. } => "cut_lists_not_canonical",
            Self::MeasuredRowWithoutCuts { .. } => "measured_row_without_cuts",
            Self::RowValuesExceedU8 { .. } => "row_values_exceed_u8",
            Self::RadixProductExceedsKey => "radix_product_exceeds_key",
            Self::PaperCountExceedsU128 => "paper_count_exceeds_u128",
            Self::FibMaskExceedsWords { .. } => "fib_mask_exceeds_words",
            Self::ExitSetEmpty => "exit_set_empty",
            Self::EntryVariantsEmpty => "entry_variants_empty",
            Self::EntryStyleUnknown { .. } => "entry_style_unknown",
            Self::GeneratedListsNotCanonical { .. } => "generated_lists_not_canonical",
            Self::ListBornAfterGeneration { .. } => "list_born_after_generation",
            Self::ChainSearchMismatch => "chain_search_mismatch",
            Self::ChainVersionNotNext { .. } => "chain_version_not_next",
            Self::ChainBirthNotLater { .. } => "chain_birth_not_later",
            Self::GateYearWithoutDays => "gate_year_without_days",
            Self::SideOutsideSearch { .. } => "side_outside_search",
        }
    }
}

impl core::fmt::Display for CubeRefusal {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "fib cube refused ({}): {self:?}", self.name())
    }
}

impl std::error::Error for CubeRefusal {}

/// An exact non-negative fraction `num / den`, `den > 0`: a stamp's own-past
/// position or a cut. Compared by cross-multiplication in `u128`
/// ([`Ratio::cmp_exact`]), so no rounding ever decides a bucket.
///
/// The derived equality is REPRESENTATIONAL (`1/2 != 2/4`); compare values
/// with [`Ratio::cmp_exact`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Ratio {
    /// The numerator.
    num: u64,
    /// The denominator, never zero.
    den: u64,
}

impl Ratio {
    /// A whole number: a raw measurement (a rank, a second) used as a stamp or
    /// a cut.
    #[must_use]
    pub const fn whole(value: u64) -> Self {
        Self { num: value, den: 1 }
    }

    /// A checked fraction.
    ///
    /// # Errors
    ///
    /// [`CubeRefusal::RatioZeroDenominator`] when `den` is zero.
    pub const fn new(num: u64, den: u64) -> Result<Self, CubeRefusal> {
        if den == 0 {
            return Err(CubeRefusal::RatioZeroDenominator { num });
        }
        Ok(Self { num, den })
    }

    /// The exact mid-rank position of a value in its own past (R1):
    /// `(2 x below + in_bin) / (2 x total)`, where `below` counts the past
    /// values below the value's bin and `in_bin` those in it.
    ///
    /// # Errors
    ///
    /// [`CubeRefusal::MidRankInvalid`] for an empty past, counts above the
    /// total, or a total too large to double.
    pub fn mid_rank(below: u64, in_bin: u64, total: u64) -> Result<Self, CubeRefusal> {
        let refused = CubeRefusal::MidRankInvalid {
            below,
            in_bin,
            total,
        };
        let within = below.checked_add(in_bin).is_some_and(|n| n <= total);
        let den = total.checked_mul(MID_RANK_SCALE).filter(|_| within);
        let num = below
            .checked_mul(MID_RANK_SCALE)
            .and_then(|n| n.checked_add(in_bin));
        match (num, den) {
            (Some(num), Some(den)) if den > 0 => Ok(Self { num, den }),
            _ => Err(refused),
        }
    }

    /// The numerator.
    #[must_use]
    pub const fn num(self) -> u64 {
        self.num
    }

    /// The denominator.
    #[must_use]
    pub const fn den(self) -> u64 {
        self.den
    }

    /// The exact order of two values: `a/b` against `c/d` as `a x d` against
    /// `c x b` in `u128`, which cannot overflow.
    #[must_use]
    pub fn cmp_exact(self, other: Self) -> Ordering {
        let left = u128::from(self.num) * u128::from(other.den);
        let right = u128::from(other.num) * u128::from(self.den);
        left.cmp(&right)
    }

    /// The canonical bytes: numerator then denominator, little-endian.
    fn push(self, out: &mut Vec<u8>) {
        out.extend_from_slice(&self.num.to_le_bytes());
        out.extend_from_slice(&self.den.to_le_bytes());
    }
}

/// The lower nearest-rank quantile `num / den` of a sorted sample: the value at
/// 1-based rank `ceil(num x n / den)`. The median of an even sample is its
/// lower middle value. R2's seed cut is `lower_nearest_rank(sorted, 1, 2)`;
/// FM-F005's rank edges use the same rule.
///
/// # Errors
///
/// [`CubeRefusal::EmptySample`] for an empty sample and
/// [`CubeRefusal::QuantileInvalid`] unless `0 < num <= den`.
pub fn lower_nearest_rank<T: Copy>(sorted: &[T], num: u64, den: u64) -> Result<T, CubeRefusal> {
    if num == 0 || num > den {
        return Err(CubeRefusal::QuantileInvalid { num, den });
    }
    let n = u128::try_from(sorted.len()).unwrap_or(u128::MAX);
    let rank = (u128::from(num) * n).div_ceil(u128::from(den));
    let at = usize::try_from(rank).unwrap_or(usize::MAX);
    at.checked_sub(1)
        .and_then(|index| sorted.get(index))
        .copied()
        .ok_or(CubeRefusal::EmptySample)
}

/// The narrowest nested mover-rank group a decision rank falls in, with the
/// generated INNER edges (FM-F005): group `k` is `rank <= edges[k]` for
/// `k < edges.len()`, and group `edges.len()` is every ranked decision, so no
/// ranked decision is ever dropped — however much larger a later pool is than
/// the window the edges came from. Rank 0 is "unranked" (FIB-2's convention)
/// and is the only rank with no group.
///
/// The edges must be non-decreasing (a [`Generation`] checks them); a repeated
/// edge is kept, and its group is equivalent to the one before
/// ([`rank_group_is_equivalent`]).
#[must_use]
pub fn rank_bucket(rank: u16, edges: &[u16]) -> Option<u8> {
    if rank == 0 {
        return None;
    }
    let below = edges.partition_point(|edge| *edge < rank);
    u8::try_from(below).ok()
}

/// Whether nested group `group` holds exactly the decisions of the group
/// before it, because the two inner edges are equal (FM-F005): the lattice
/// skips it as equivalent and counts the skip. The widest group is never
/// equivalent by edges alone.
#[must_use]
pub fn rank_group_is_equivalent(edges: &[u16], group: u8) -> bool {
    let at = usize::from(group);
    match (
        at.checked_sub(1).and_then(|before| edges.get(before)),
        edges.get(at),
    ) {
        (Some(before), Some(edge)) => before == edge,
        _ => false,
    }
}

/// One generated cut list: the cuts of one measured row, scoped to one width
/// and one base kind where the row is (R1).
///
/// A stamp's bucket is the number of cuts at or below it, so a value ON a cut
/// goes up. Cuts compare exactly ([`Ratio::cmp_exact`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CutList {
    /// The measured row.
    pub row: RowId,
    /// The width, in minutes, this list belongs to, or `None` for every width.
    pub timeframe: Option<u16>,
    /// The base kind (a `base_kind` value index) this list belongs to, or
    /// `None` for every kind.
    pub base_kind: Option<u8>,
    /// The cuts, strictly increasing.
    pub cuts: Vec<Ratio>,
}

impl CutList {
    /// The bucket of a stamp: how many cuts are at or below it. One binary
    /// search over at most [`MAX_CUTS`] cuts — at most eight exact comparisons.
    #[must_use]
    pub fn bucket(&self, stamp: Ratio) -> u8 {
        let at = self
            .cuts
            .partition_point(|cut| cut.cmp_exact(stamp) != Ordering::Greater);
        u8::try_from(at).unwrap_or(OPEN)
    }

    /// The list's context: the order the generation keeps lists in.
    fn context(&self) -> (RowId, Option<u16>, Option<u8>) {
        (self.row, self.timeframe, self.base_kind)
    }
}

/// How many bytes a digest carried in a [`Generation`] has (BLAKE3's width;
/// `vocab` cannot hash, so the caller supplies it).
pub const DIGEST_BYTES: usize = 32;

/// A generated variant set as the schema sees it: how many variants, under
/// which version, with the digest of the set's canonical bytes. The variants
/// themselves are their owner's (the exit set is FIB-5's).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VariantSet {
    /// The set's version.
    pub version: u32,
    /// The digest of the set's canonical bytes, computed by the caller.
    pub digest: [u8; DIGEST_BYTES],
    /// How many variants it holds: the partition row's value count.
    pub count: u16,
}

/// The generated entry-style variants: each names one of the definitional
/// entry styles (`close`, `break`, `retest`); their parameters are in the set
/// whose digest this carries.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EntryVariants {
    /// The set's version.
    pub version: u32,
    /// The digest of the set's canonical bytes, computed by the caller.
    pub digest: [u8; DIGEST_BYTES],
    /// The style (an index into the entry-style names) of each variant, by
    /// value index.
    pub styles: Vec<u8>,
}

/// A generated list owned by another FIB package (touch levels, histogram
/// split depths, stop offsets, deadlines and the like), carried by name,
/// version, birth step and digest so it enters the run identity without this
/// schema knowing its shape.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GeneratedList {
    /// The list's name, owned by the package that generates it.
    pub name: &'static str,
    /// The list's version.
    pub version: u32,
    /// The walk-forward step it was born at.
    pub birth_step: u32,
    /// The digest of its canonical bytes, computed by the caller.
    pub digest: [u8; DIGEST_BYTES],
}

/// Everything a [`Generation`] is built from. No `Default`: every value is
/// named by the run that generated it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GenerationParts {
    /// The search (direction) this generation belongs to: every generated
    /// number is per direction (operator 16 Oct 16:04 IST).
    pub search: Search,
    /// This version's number in its direction's chain.
    pub version: u32,
    /// The walk-forward step `m` it was born at, from learning months
    /// `[0, m)` only.
    pub birth_step: u32,
    /// The candle widths tried, in whole minutes, strictly increasing, each in
    /// `1..=`[`max_width_minutes`] (R7).
    pub timeframes: Vec<u16>,
    /// The mover-rank INNER edges, non-decreasing, each at least rank one;
    /// repeats are kept (FM-F005).
    pub rank_edges: Vec<u16>,
    /// One list per measured row and scope, in canonical (row, width, base
    /// kind) order. Time-of-day lists are per width, in seconds after 09:15.
    pub cuts: Vec<CutList>,
    /// The exit variant set (FIB-5), time exits and stop offsets included.
    pub exit_set: VariantSet,
    /// The entry-style variants.
    pub entry_variants: EntryVariants,
    /// Lists other packages generate, in name order.
    pub others: Vec<GeneratedList>,
}

/// The width of the cell key, chosen from the descriptor (FM-F006) and
/// recorded in the schema digest.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum KeyWidth {
    /// Every code fits a `u64`.
    Bits64,
    /// Every code fits a `u128`.
    Bits128,
}

impl KeyWidth {
    /// The key's width in bits.
    #[must_use]
    pub const fn bits(self) -> u32 {
        match self {
            Self::Bits64 => u64::BITS,
            Self::Bits128 => u128::BITS,
        }
    }
}

/// The per-run layout a [`Generation`] implies: every row's value count, the
/// cell key's width, the mask positions and the paper counts. Built only inside
/// [`Generation::new`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Layout {
    /// Every row's value count, by row index.
    radices: [u8; ROW_COUNT],
    /// Every row's first mask position, by row index.
    offsets: [usize; ROW_COUNT],
    /// Every row's value count together.
    total: usize,
    /// The product of every row's value count: the paper full-cell count.
    full_cells: u128,
    /// The key width the product needs.
    key: KeyWidth,
}

impl Layout {
    /// The layout of a set of value counts, each already within
    /// [`MAX_ROW_VALUES`] and at least one.
    fn of(radices: [u8; ROW_COUNT]) -> Result<Self, CubeRefusal> {
        let mut offsets = [0; ROW_COUNT];
        let mut total = 0;
        let mut full_cells: u128 = 1;
        for (offset, radix) in offsets.iter_mut().zip(radices) {
            *offset = total;
            total += usize::from(radix);
            full_cells = full_cells
                .checked_mul(u128::from(radix))
                .ok_or(CubeRefusal::RadixProductExceedsKey)?;
        }
        let key = if full_cells <= u128::from(u64::MAX) + 1 {
            KeyWidth::Bits64
        } else {
            KeyWidth::Bits128
        };
        Ok(Self {
            radices,
            offsets,
            total,
            full_cells,
            key,
        })
    }

    /// A row's value count.
    #[must_use]
    pub fn radix(&self, row: RowId) -> u8 {
        self.radices.get(row.index()).copied().unwrap_or(0)
    }

    /// Every row's value count, by row index.
    #[must_use]
    pub const fn radices(&self) -> &[u8; ROW_COUNT] {
        &self.radices
    }

    /// Every row's value count together: how many mask positions a cell needs.
    #[must_use]
    pub const fn total_values(&self) -> usize {
        self.total
    }

    /// The cell key's width.
    #[must_use]
    pub const fn key_width(&self) -> KeyWidth {
        self.key
    }

    /// The number of full cells: the product of every row's value count.
    /// PAPER ONLY — never enumerated; each candidate lands in one cell.
    #[must_use]
    pub const fn paper_full_cells(&self) -> u128 {
        self.full_cells
    }

    /// The number of partial mixes: a free row contributes its value count
    /// plus one (for [`OPEN`]), a partition row its value count. PAPER ONLY —
    /// the lattice visits only what its trade counts reach.
    ///
    /// # Errors
    ///
    /// [`CubeRefusal::PaperCountExceedsU128`] when the product passes `u128`.
    pub fn paper_partial_mixes(&self) -> Result<u128, CubeRefusal> {
        let mut product: u128 = 1;
        for (radix, row) in self.radices.iter().zip(&ROWS) {
            let choices = match row.kind {
                RowKind::Partition => u128::from(*radix),
                RowKind::Categorical | RowKind::Cumulative => u128::from(*radix) + 1,
            };
            product = product
                .checked_mul(choices)
                .ok_or(CubeRefusal::PaperCountExceedsU128)?;
        }
        Ok(product)
    }

    /// Checks a full cell: every row holds a value index below its value count.
    ///
    /// # Errors
    ///
    /// [`CubeRefusal::OpenInCell`] for an [`OPEN`] row and
    /// [`CubeRefusal::ValueOutOfRange`] for any other out-of-range index, naming
    /// the first offending row.
    pub fn validate(&self, coords: &Coords) -> Result<(), CubeRefusal> {
        for ((value, radix), row) in coords.iter().zip(self.radices).zip(&ROWS) {
            if *value == OPEN {
                return Err(CubeRefusal::OpenInCell { row: row.name });
            }
            if *value >= radix {
                return Err(CubeRefusal::ValueOutOfRange {
                    row: row.name,
                    value: *value,
                    values: usize::from(radix),
                });
            }
        }
        Ok(())
    }

    /// Checks a mix: every row is [`OPEN`] or a value index in range, and no
    /// partition row is [`OPEN`].
    ///
    /// # Errors
    ///
    /// [`CubeRefusal::OpenPartitionRow`] or [`CubeRefusal::ValueOutOfRange`],
    /// naming the first offending row.
    pub fn validate_mix(&self, mix: &MixKey) -> Result<(), CubeRefusal> {
        for ((value, radix), row) in mix.0.iter().zip(self.radices).zip(&ROWS) {
            if *value == OPEN {
                if row.kind == RowKind::Partition {
                    return Err(CubeRefusal::OpenPartitionRow { row: row.name });
                }
            } else if *value >= radix {
                return Err(CubeRefusal::ValueOutOfRange {
                    row: row.name,
                    value: *value,
                    values: usize::from(radix),
                });
            }
        }
        Ok(())
    }

    /// The mixed-radix code of a full cell, row 0 most significant. It fits the
    /// layout's [`KeyWidth`].
    ///
    /// # Errors
    ///
    /// Whatever [`Layout::validate`] refuses: an invalid cell has no code.
    pub fn encode(&self, coords: &Coords) -> Result<u128, CubeRefusal> {
        self.validate(coords)?;
        let mut code: u128 = 0;
        for (value, radix) in coords.iter().zip(self.radices) {
            code = code * u128::from(radix) + u128::from(*value);
        }
        Ok(code)
    }

    /// The full cell of a code, the inverse of [`Layout::encode`].
    ///
    /// # Errors
    ///
    /// [`CubeRefusal::CodeOutOfRange`] for a code at or above
    /// [`Layout::paper_full_cells`].
    pub fn decode(&self, code: u128) -> Result<Coords, CubeRefusal> {
        if code >= self.full_cells {
            return Err(CubeRefusal::CodeOutOfRange { code });
        }
        let mut coords: Coords = [0; ROW_COUNT];
        // A code that fits 64 bits is divided in 64 bits: one hardware divide
        // per row, where `u128` costs two library calls (C-F-02).
        if let Ok(mut rest) = u64::try_from(code) {
            for (slot, radix) in coords.iter_mut().zip(self.radices).rev() {
                let radix = u64::from(radix);
                *slot = u8::try_from(rest % radix).unwrap_or(OPEN);
                rest /= radix;
            }
            return Ok(coords);
        }
        let mut rest = code;
        for (slot, radix) in coords.iter_mut().zip(self.radices).rev() {
            let radix = u128::from(radix);
            *slot = u8::try_from(rest % radix).unwrap_or(OPEN);
            rest /= radix;
        }
        Ok(coords)
    }

    /// Refuses a layout with more values than the mask has positions.
    fn maskable(&self) -> Result<(), CubeRefusal> {
        let bits = ConditionMask::BITS;
        if u32::try_from(self.total)
            .ok()
            .is_none_or(|values| values > bits)
        {
            return Err(CubeRefusal::FibMaskExceedsWords {
                values: self.total,
                bits,
            });
        }
        Ok(())
    }

    /// A mask position as the mask's `u32`; every position is below
    /// [`ConditionMask::BITS`] once [`Layout::maskable`] has passed.
    fn bit(position: usize) -> u32 {
        u32::try_from(position).unwrap_or(u32::MAX)
    }

    /// The cell as a [`ConditionMask`] in the FIB NAMESPACE: each row's values
    /// occupy the next positions in row order, one bit per value; the
    /// cumulative row sets its group and every wider one (a thermometer).
    ///
    /// **Never decode this mask through [`crate::table`].** It shares the mask
    /// type and nothing else with the condition table. Used only by FIB-6's
    /// oracle.
    ///
    /// # Errors
    ///
    /// [`CubeRefusal::FibMaskExceedsWords`] when the layout has more values than
    /// the mask has positions, then whatever [`Layout::validate`] refuses.
    pub fn to_mask(&self, coords: &Coords) -> Result<ConditionMask, CubeRefusal> {
        self.maskable()?;
        self.validate(coords)?;
        let mut mask = ConditionMask::ZERO;
        for (((value, radix), offset), row) in
            coords.iter().zip(self.radices).zip(self.offsets).zip(&ROWS)
        {
            let last = match row.kind {
                RowKind::Cumulative => radix,
                RowKind::Categorical | RowKind::Partition => *value + 1,
            };
            for bit in *value..last {
                mask = mask.with_bit(Self::bit(offset + usize::from(bit)));
            }
        }
        Ok(mask)
    }

    /// The mix as a [`ConditionMask`] in the FIB NAMESPACE: one required bit per
    /// fixed row, none for an [`OPEN`] row. A cell's [`Layout::to_mask`] hits it
    /// (`ConditionMask::hits`) exactly when [`contains`] is true.
    ///
    /// # Errors
    ///
    /// [`CubeRefusal::FibMaskExceedsWords`], then whatever
    /// [`Layout::validate_mix`] refuses.
    pub fn mix_mask(&self, mix: &MixKey) -> Result<ConditionMask, CubeRefusal> {
        self.maskable()?;
        self.validate_mix(mix)?;
        let mut mask = ConditionMask::ZERO;
        for (value, offset) in mix.0.iter().zip(self.offsets) {
            if *value != OPEN {
                mask = mask.with_bit(Self::bit(offset + usize::from(*value)));
            }
        }
        Ok(mask)
    }

    /// Every position the layout offers, set: the `mask` term of a FIB run's
    /// identity ("`to_mask` of every offered value").
    ///
    /// # Errors
    ///
    /// [`CubeRefusal::FibMaskExceedsWords`].
    pub fn offered_mask(&self) -> Result<ConditionMask, CubeRefusal> {
        self.maskable()?;
        let mut mask = ConditionMask::ZERO;
        for position in 0..self.total {
            mask = mask.with_bit(Self::bit(position));
        }
        Ok(mask)
    }

    /// The layout's canonical bytes: the key width and every row's value count.
    fn push(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(&self.key.bits().to_le_bytes());
        count(out, ROW_COUNT);
        out.extend_from_slice(&self.radices);
    }
}

/// Whether the cell is in the mix: every row is [`OPEN`], equal, or — on the
/// cumulative row — nested inside the mix's group.
///
/// Both arguments must already have passed [`Layout::validate`] and
/// [`Layout::validate_mix`]; this is the per-row hot path and does not
/// re-check. It visits all 24 rows with no early exit.
#[must_use]
pub fn contains(mix: &MixKey, coords: &Coords) -> bool {
    let mut inside = true;
    for ((want, have), row) in mix.0.iter().zip(coords).zip(&ROWS) {
        let hit = match row.kind {
            RowKind::Cumulative => have <= want,
            RowKind::Categorical | RowKind::Partition => have == want,
        };
        inside &= *want == OPEN || hit;
    }
    inside
}

/// The hard top of the candle width, in whole minutes (R7): the largest `N`
/// for which a smallest operator base ([`MICRO2_CANDLES`] candles) plus a
/// trigger candle, anchored at 09:15, close by 15:09:00. Derived from the
/// operator's times and pattern size, never typed: 118 on the 09:15 grid.
#[must_use]
pub const fn max_width_minutes() -> u64 {
    let span = SESSION_FORCED_WINDOW_START_SECS - SESSION_GRID_ANCHOR_SECS;
    span / SECONDS_PER_MINUTE / (MICRO2_CANDLES + 1)
}

/// The seconds after 09:15 a decision may close in: the span of a
/// time-of-day cut.
const fn decision_span_secs() -> u64 {
    SESSION_FORCED_WINDOW_START_SECS - SESSION_GRID_ANCHOR_SECS
}

/// Refuses a list longer than a `u8` can index beside [`OPEN`].
fn within_u8(row: RowId, values: usize) -> Result<u8, CubeRefusal> {
    u8::try_from(values)
        .ok()
        .filter(|_| values <= MAX_ROW_VALUES)
        .ok_or(CubeRefusal::RowValuesExceedU8 {
            row: row.row().name,
            values,
        })
}

/// The per-run, per-direction generated numbers the schema reads, as one
/// version of its direction's chain, with the [`Layout`] they imply. Built only
/// through [`Generation::new`], which checks every part (FM-F006).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Generation {
    /// The checked parts.
    parts: GenerationParts,
    /// The layout they imply.
    layout: Layout,
}

impl Generation {
    /// A checked generation and its layout.
    ///
    /// # Errors
    ///
    /// By name: [`CubeRefusal::TimeframesEmpty`],
    /// [`CubeRefusal::TimeframesInvalid`], [`CubeRefusal::RankEdgesInvalid`],
    /// [`CubeRefusal::CutOnUnmeasuredRow`],
    /// [`CubeRefusal::CutTimeframeUnknown`],
    /// [`CubeRefusal::CutBaseKindOutOfRange`],
    /// [`CubeRefusal::CutsNotIncreasing`],
    /// [`CubeRefusal::TimeCutsWithoutTimeframe`],
    /// [`CubeRefusal::TimeCutOffGrid`], [`CubeRefusal::CutListsNotCanonical`],
    /// [`CubeRefusal::MeasuredRowWithoutCuts`], [`CubeRefusal::ExitSetEmpty`],
    /// [`CubeRefusal::EntryVariantsEmpty`], [`CubeRefusal::EntryStyleUnknown`],
    /// [`CubeRefusal::GeneratedListsNotCanonical`],
    /// [`CubeRefusal::ListBornAfterGeneration`],
    /// [`CubeRefusal::RowValuesExceedU8`] and
    /// [`CubeRefusal::RadixProductExceedsKey`].
    pub fn new(parts: GenerationParts) -> Result<Self, CubeRefusal> {
        check_timeframes(&parts.timeframes)?;
        check_rank_edges(&parts.rank_edges)?;
        check_cut_lists(&parts)?;
        if parts.exit_set.count == 0 {
            return Err(CubeRefusal::ExitSetEmpty);
        }
        if parts.entry_variants.styles.is_empty() {
            return Err(CubeRefusal::EntryVariantsEmpty);
        }
        let known = u8::try_from(ENTRY_STYLE_NAMES).unwrap_or(u8::MAX);
        if let Some(style) = parts.entry_variants.styles.iter().find(|s| **s >= known) {
            return Err(CubeRefusal::EntryStyleUnknown { style: *style });
        }
        check_others(&parts)?;
        let layout = Layout::of(radices_of(&parts)?)?;
        Ok(Self { parts, layout })
    }

    /// The parts this generation was built from.
    #[must_use]
    pub const fn parts(&self) -> &GenerationParts {
        &self.parts
    }

    /// The layout these parts imply.
    #[must_use]
    pub const fn layout(&self) -> &Layout {
        &self.layout
    }

    /// Whether walk-forward step `step` may read this version: only versions
    /// born at or before the step (R1).
    #[must_use]
    pub const fn usable_at(&self, step: u32) -> bool {
        self.parts.birth_step <= step
    }

    /// The cut list of a row in one exact scope, or `None`. No wider scope is
    /// tried in its place: a caller asks for the scope it generated.
    #[must_use]
    pub fn cuts(
        &self,
        row: RowId,
        timeframe: Option<u16>,
        base_kind: Option<u8>,
    ) -> Option<&CutList> {
        let wanted = (row, timeframe, base_kind);
        self.parts
            .cuts
            .binary_search_by(|list| list.context().cmp(&wanted))
            .ok()
            .and_then(|at| self.parts.cuts.get(at))
    }

    /// [`rank_bucket`] with this generation's edges.
    #[must_use]
    pub fn rank_group(&self, rank: u16) -> Option<u8> {
        rank_bucket(rank, &self.parts.rank_edges)
    }

    /// How many nested groups are equivalent to the one before them (repeated
    /// edges, FM-F005): the count the lattice reports as skipped.
    #[must_use]
    pub fn equivalent_rank_groups(&self) -> usize {
        self.parts
            .rank_edges
            .windows(2)
            .filter(|pair| matches!(pair, [a, b] if a == b))
            .count()
    }

    /// The canonical bytes of this version, for the run identity: a magic, the
    /// schema version, the search, the version and birth step, the widths, the
    /// rank edges, every cut list, the exit set, the entry variants and the
    /// other lists, each framed with its length, little-endian.
    #[must_use]
    pub fn bytes(&self) -> Vec<u8> {
        let parts = &self.parts;
        let mut out = Vec::new();
        out.extend_from_slice(b"BTXFIBG1");
        out.extend_from_slice(&FIB_CUBE_VERSION.to_le_bytes());
        frame(&mut out, parts.search.name().as_bytes());
        out.extend_from_slice(&parts.version.to_le_bytes());
        out.extend_from_slice(&parts.birth_step.to_le_bytes());
        count(&mut out, parts.timeframes.len());
        for minutes in &parts.timeframes {
            out.extend_from_slice(&minutes.to_le_bytes());
        }
        count(&mut out, parts.rank_edges.len());
        for edge in &parts.rank_edges {
            out.extend_from_slice(&edge.to_le_bytes());
        }
        count(&mut out, parts.cuts.len());
        for list in &parts.cuts {
            frame(&mut out, list.row.row().name.as_bytes());
            push_option(&mut out, list.timeframe.map(u64::from));
            push_option(&mut out, list.base_kind.map(u64::from));
            count(&mut out, list.cuts.len());
            for cut in &list.cuts {
                cut.push(&mut out);
            }
        }
        out.extend_from_slice(&parts.exit_set.version.to_le_bytes());
        out.extend_from_slice(&parts.exit_set.digest);
        out.extend_from_slice(&parts.exit_set.count.to_le_bytes());
        out.extend_from_slice(&parts.entry_variants.version.to_le_bytes());
        out.extend_from_slice(&parts.entry_variants.digest);
        frame(&mut out, &parts.entry_variants.styles);
        count(&mut out, parts.others.len());
        for list in &parts.others {
            frame(&mut out, list.name.as_bytes());
            out.extend_from_slice(&list.version.to_le_bytes());
            out.extend_from_slice(&list.birth_step.to_le_bytes());
            out.extend_from_slice(&list.digest);
        }
        out
    }

    /// The PER-RUN DESCRIPTOR (FM-F006): the static [`descriptor`], this
    /// version's [`Generation::bytes`] and its [`Layout`] (key width and every
    /// row's value count). `runner` hashes it into the identity's params and
    /// records it in the append-only schema ledger.
    #[must_use]
    pub fn run_descriptor(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(b"BTXFIBR1");
        frame(&mut out, &descriptor());
        frame(&mut out, &self.bytes());
        self.layout.push(&mut out);
        out
    }
}

/// Refuses an empty width list, a zero width, one past [`max_width_minutes`]
/// or one not above the width before it.
fn check_timeframes(widths: &[u16]) -> Result<(), CubeRefusal> {
    if widths.is_empty() {
        return Err(CubeRefusal::TimeframesEmpty);
    }
    let mut previous = 0;
    for (at, minutes) in widths.iter().enumerate() {
        if *minutes <= previous || u64::from(*minutes) > max_width_minutes() {
            return Err(CubeRefusal::TimeframesInvalid {
                at,
                minutes: *minutes,
            });
        }
        previous = *minutes;
    }
    Ok(())
}

/// Refuses rank edges below rank one or descending. Too many is
/// [`radices_of`]'s refusal, as for every row.
fn check_rank_edges(edges: &[u16]) -> Result<(), CubeRefusal> {
    let mut previous = 1;
    for (at, edge) in edges.iter().enumerate() {
        if *edge < previous {
            return Err(CubeRefusal::RankEdgesInvalid { at, edge: *edge });
        }
        previous = *edge;
    }
    Ok(())
}

/// Checks one time-of-day list: per width, every cut a positive whole number
/// of the width's candles, inside the decision span.
fn check_time_cuts(list: &CutList) -> Result<(), CubeRefusal> {
    let Some(minutes) = list.timeframe else {
        return Err(CubeRefusal::TimeCutsWithoutTimeframe);
    };
    let candle = u64::from(minutes) * SECONDS_PER_MINUTE;
    for (at, cut) in list.cuts.iter().enumerate() {
        let on_grid = cut.den() == 1
            && cut.num() > 0
            && cut.num().is_multiple_of(candle)
            && cut.num() < decision_span_secs();
        if !on_grid {
            return Err(CubeRefusal::TimeCutOffGrid { minutes, at });
        }
    }
    Ok(())
}

/// Checks every cut list, their order, and that every measured row has one.
fn check_cut_lists(parts: &GenerationParts) -> Result<(), CubeRefusal> {
    let mut previous: Option<(RowId, Option<u16>, Option<u8>)> = None;
    for list in &parts.cuts {
        let row = list.row.row();
        if row.source != Source::Measured || list.row == RowId::MoverRank {
            return Err(CubeRefusal::CutOnUnmeasuredRow { row: row.name });
        }
        if let Some(minutes) = list.timeframe
            && parts.timeframes.binary_search(&minutes).is_err()
        {
            return Err(CubeRefusal::CutTimeframeUnknown {
                row: row.name,
                minutes,
            });
        }
        let kinds = u8::try_from(BASE_KINDS).unwrap_or(u8::MAX);
        if let Some(base_kind) = list.base_kind
            && base_kind >= kinds
        {
            return Err(CubeRefusal::CutBaseKindOutOfRange {
                row: row.name,
                base_kind,
            });
        }
        if let Some(at) = list
            .cuts
            .windows(2)
            .position(|pair| matches!(pair, [a, b] if a.cmp_exact(*b) != Ordering::Less))
        {
            return Err(CubeRefusal::CutsNotIncreasing {
                row: row.name,
                at: at + 1,
            });
        }
        if list.row == RowId::TimeOfDay {
            check_time_cuts(list)?;
        }
        if previous.is_some_and(|before| before >= list.context()) {
            return Err(CubeRefusal::CutListsNotCanonical { row: row.name });
        }
        previous = Some(list.context());
    }
    for id in RowId::ALL {
        let measured = id.row().source == Source::Measured && id != RowId::MoverRank;
        if measured && !parts.cuts.iter().any(|list| list.row == id) {
            return Err(CubeRefusal::MeasuredRowWithoutCuts { row: id.row().name });
        }
    }
    Ok(())
}

/// Checks the other packages' lists: name order, and none born later than the
/// generation holding it.
fn check_others(parts: &GenerationParts) -> Result<(), CubeRefusal> {
    for (at, pair) in parts.others.windows(2).enumerate() {
        if matches!(pair, [a, b] if a.name >= b.name) {
            return Err(CubeRefusal::GeneratedListsNotCanonical { at: at + 1 });
        }
    }
    if let Some(at) = parts
        .others
        .iter()
        .position(|list| list.birth_step > parts.birth_step)
    {
        return Err(CubeRefusal::ListBornAfterGeneration { at });
    }
    Ok(())
}

/// Every row's value count under these parts.
fn radices_of(parts: &GenerationParts) -> Result<[u8; ROW_COUNT], CubeRefusal> {
    let mut radices = [0; ROW_COUNT];
    for (slot, id) in radices.iter_mut().zip(RowId::ALL) {
        let row = id.row();
        let values = match (id, row.source) {
            (RowId::MoverRank, _) => parts.rank_edges.len() + 1,
            (RowId::Exit, _) => usize::from(parts.exit_set.count),
            (RowId::EntryStyle, _) => parts.entry_variants.styles.len(),
            (RowId::Timeframe, _) => parts.timeframes.len(),
            (_, Source::Measured) => parts
                .cuts
                .iter()
                .filter(|list| list.row == id)
                .map(|list| list.cuts.len() + 1)
                .max()
                .unwrap_or(1),
            (_, Source::Definitional | Source::Variants) => row.labels.len(),
        };
        *slot = within_u8(id, values)?;
    }
    Ok(radices)
}

/// One direction's generations, oldest first: each version born at a later
/// walk-forward step than the one before (R1).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GenerationChain {
    /// The first version.
    first: Generation,
    /// Every later version, oldest first.
    rest: Vec<Generation>,
}

impl GenerationChain {
    /// A chain of one version.
    #[must_use]
    pub fn new(first: Generation) -> Self {
        Self {
            first,
            rest: Vec::new(),
        }
    }

    /// Appends the next version.
    ///
    /// # Errors
    ///
    /// [`CubeRefusal::ChainSearchMismatch`] for another direction's version,
    /// [`CubeRefusal::ChainVersionNotNext`] unless its number is the last one
    /// plus one, and [`CubeRefusal::ChainBirthNotLater`] unless it was born at
    /// a later step.
    pub fn push(&mut self, next: Generation) -> Result<(), CubeRefusal> {
        let last = self.latest().parts();
        if next.parts.search != last.search {
            return Err(CubeRefusal::ChainSearchMismatch);
        }
        let expected = last.version.saturating_add(1);
        if next.parts.version != expected {
            return Err(CubeRefusal::ChainVersionNotNext {
                expected,
                got: next.parts.version,
            });
        }
        if next.parts.birth_step <= last.birth_step {
            return Err(CubeRefusal::ChainBirthNotLater {
                previous: last.birth_step,
                got: next.parts.birth_step,
            });
        }
        self.rest.push(next);
        Ok(())
    }

    /// The newest version.
    #[must_use]
    pub fn latest(&self) -> &Generation {
        self.rest.last().unwrap_or(&self.first)
    }

    /// The version step `step` reads: the newest born at or before it, or
    /// `None` before the first version's birth. One binary search.
    #[must_use]
    pub fn at(&self, step: u32) -> Option<&Generation> {
        if !self.first.usable_at(step) {
            return None;
        }
        let born = self.rest.partition_point(|version| version.usable_at(step));
        Some(
            born.checked_sub(1)
                .and_then(|at| self.rest.get(at))
                .unwrap_or(&self.first),
        )
    }

    /// Every version, oldest first.
    pub fn versions(&self) -> impl Iterator<Item = &Generation> {
        core::iter::once(&self.first).chain(&self.rest)
    }

    /// The direction this chain belongs to.
    #[must_use]
    pub fn search(&self) -> Search {
        self.latest().parts.search
    }
}

/// One thousand: the whole of a permille.
pub const PERMILLE: u16 = 1_000;

/// The seed buckets of a three-label measured row, the coarse v1 start.
const V1_SEED_BUCKETS: u64 = MOVE_SO_FAR_ROW.labels.len() as u64;

/// The equal-count third an own-past permille fell in when a v1 cell was built
/// from permilles (0 low, 1 mid, 2 high); a value on an edge goes up.
///
/// DECODE ONLY (17:15 IST, item h): R2's cut lookup ([`CutList::bucket`])
/// replaced this for every new cell. Cross-multiplied: no division, no
/// rounding.
///
/// # Errors
///
/// [`CubeRefusal::PermilleOutOfRange`] above one thousand.
pub fn tercile(permille: u16) -> Result<u8, CubeRefusal> {
    if permille > PERMILLE {
        return Err(CubeRefusal::PermilleOutOfRange { permille });
    }
    let scaled = u64::from(permille) * V1_SEED_BUCKETS;
    let mut bucket: u8 = 0;
    let mut edge: u64 = 1;
    while edge < V1_SEED_BUCKETS {
        bucket += u8::from(scaled >= edge * u64::from(PERMILLE));
        edge += 1;
    }
    Ok(bucket)
}

/// The Fibonacci rung 0.236, in permille: `indicators::fib::RUNGS[1]`.
pub const ZONE_RUNG_236: u64 = 236;
/// The Fibonacci rung 0.382, in permille: `indicators::fib::RUNGS[2]`.
pub const ZONE_RUNG_382: u64 = 382;
/// The Fibonacci rung 0.5, in permille: `indicators::fib::RUNGS[3]`.
pub const ZONE_RUNG_500: u64 = 500;
/// The Fibonacci rung 0.618, in permille: `indicators::fib::RUNGS[4]`.
pub const ZONE_RUNG_618: u64 = 618;
/// The Fibonacci rung 0.786, in permille: `indicators::fib::RUNGS[5]`.
pub const ZONE_RUNG_786: u64 = 786;
/// The Fibonacci rung 1, in permille: `indicators::fib::RUNGS[6]`.
pub const ZONE_RUNG_1000: u64 = 1_000;

/// The rungs of the `fib_zone` row, in permille, by value index.
///
/// The first seven of `indicators::fib::RUNGS`. Retyped here only because this
/// crate depends on nothing (gate 22) and cannot name `indicators`; the copy is
/// pinned to the original by `runner`'s
/// `fib_zone_rungs_are_the_first_seven_of_indicators_fib_rungs`.
pub const ZONE_RUNGS: [u64; FIB_ZONE_ROW.labels.len()] = [
    0,
    ZONE_RUNG_236,
    ZONE_RUNG_382,
    ZONE_RUNG_500,
    ZONE_RUNG_618,
    ZONE_RUNG_786,
    ZONE_RUNG_1000,
];

/// The `fib_zone` value of a retracement given in permille: the nearest of
/// [`ZONE_RUNGS`], ties to the shallower rung.
///
/// # Errors
///
/// [`CubeRefusal::RetraceOutOfRange`] outside `0..=1000`.
pub fn fib_zone(retrace_permille: i64) -> Result<u8, CubeRefusal> {
    fib_zone_of(retrace_permille, i64::from(PERMILLE))
}

/// The `fib_zone` value of the exact retracement `part / whole`, by
/// cross-multiplied comparison (`|part x 1000 - rung x whole|` in `i128`), so
/// no rounding decides a rung. Ties go to the shallower rung.
///
/// # Errors
///
/// [`CubeRefusal::RetraceOutOfRange`] unless `whole > 0` and
/// `0 <= part <= whole`.
pub fn fib_zone_of(part: i64, whole: i64) -> Result<u8, CubeRefusal> {
    if whole <= 0 || part < 0 || part > whole {
        return Err(CubeRefusal::RetraceOutOfRange { part, whole });
    }
    let scaled = i128::from(part) * i128::from(PERMILLE);
    let mut best: u8 = 0;
    let mut best_gap = i128::MAX;
    for (zone, rung) in (0_u8..).zip(ZONE_RUNGS) {
        let gap = (scaled - i128::from(rung) * i128::from(whole)).abs();
        if gap < best_gap {
            best = zone;
            best_gap = gap;
        }
    }
    Ok(best)
}

/// What one order-flow window call (OF-1) answered for a trigger candle
/// (FM-F004).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum OrderFlowWindow {
    /// The run has no order-flow source (`--order-flow absent`): the row is
    /// constant.
    Off,
    /// No order-flow record exists for the window.
    NoRecord,
    /// Records exist but cover less than the window.
    Partial,
    /// The window is covered, with its classified buy and sell volume and the
    /// volume no side could be given to.
    Covered {
        /// Buyer-initiated volume.
        buy: u64,
        /// Seller-initiated volume.
        sell: u64,
        /// Volume that could not be classified.
        unclassified: u64,
    },
}

/// Why a trigger's order flow is `absent` (FM-F004), each counted by name per
/// month and stage.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AbsentReason {
    /// The run has no order-flow source.
    Off,
    /// No record for the window.
    NoRecord,
    /// Coverage below the window.
    Partial,
    /// A covered window in which nothing traded at all.
    Empty,
}

/// The value indices of the `trigger_order_flow` row. Implicit discriminants
/// in label order, proved by `the_order_flow_values_are_the_row_labels`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum OrderFlowValue {
    /// The order flow delta has the trade's sign.
    With,
    /// The order flow delta has the opposite sign.
    Against,
    /// Classified volume traded and the order flow delta is zero.
    Balanced,
    /// Volume traded and none of it could be classified.
    Unclassified,
    /// No usable order flow; see [`AbsentReason`].
    Absent,
}

impl OrderFlowValue {
    /// The value index in the `trigger_order_flow` row.
    #[must_use]
    pub const fn index(self) -> u8 {
        self as u8
    }
}

/// A trigger's order-flow reading: its row value and, when absent, why.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum OrderFlowReading {
    /// The order flow delta has the trade's sign.
    With,
    /// The order flow delta has the opposite sign.
    Against,
    /// Classified volume traded and the order flow delta is zero.
    Balanced,
    /// Volume traded and none of it could be classified.
    Unclassified,
    /// No usable order flow, and why.
    Absent(AbsentReason),
}

impl OrderFlowReading {
    /// The reading of one window call for a trade of `search`'s direction
    /// (FM-F004). No loop: a few comparisons and one signed subtraction in
    /// `i128`. A pure zero (classified volume, order flow delta zero) is
    /// `balanced` and never pooled with a missing record.
    #[must_use]
    pub fn of(window: OrderFlowWindow, search: Search) -> Self {
        match window {
            OrderFlowWindow::Off => Self::Absent(AbsentReason::Off),
            OrderFlowWindow::NoRecord => Self::Absent(AbsentReason::NoRecord),
            OrderFlowWindow::Partial => Self::Absent(AbsentReason::Partial),
            OrderFlowWindow::Covered {
                buy,
                sell,
                unclassified,
            } => {
                let order_flow_delta = i128::from(buy) - i128::from(sell);
                let classified = u128::from(buy) + u128::from(sell);
                match (classified, unclassified, order_flow_delta.signum(), search) {
                    (0, 0, _, _) => Self::Absent(AbsentReason::Empty),
                    (0, _, _, _) => Self::Unclassified,
                    (_, _, 0, _) => Self::Balanced,
                    (_, _, 1, Search::Long) | (_, _, -1, Search::Short) => Self::With,
                    _ => Self::Against,
                }
            }
        }
    }

    /// The row value.
    #[must_use]
    pub const fn value(self) -> OrderFlowValue {
        match self {
            Self::With => OrderFlowValue::With,
            Self::Against => OrderFlowValue::Against,
            Self::Balanced => OrderFlowValue::Balanced,
            Self::Unclassified => OrderFlowValue::Unclassified,
            Self::Absent(_) => OrderFlowValue::Absent,
        }
    }

    /// The reading's counted name: the row label, or `absent:<reason>`.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::With => "with",
            Self::Against => "against",
            Self::Balanced => "balanced",
            Self::Unclassified => "unclassified",
            Self::Absent(AbsentReason::Off) => "absent:off",
            Self::Absent(AbsentReason::NoRecord) => "absent:no_record",
            Self::Absent(AbsentReason::Partial) => "absent:partial",
            Self::Absent(AbsentReason::Empty) => "absent:empty",
        }
    }
}

/// The trade floor of a window (R5): `Q(w) = 500 x (tradable days in w /
/// tradable days in a gate year)`, the operator's 13:57 IST pro-rating, and the
/// two phase floors that follow from it. Exact: no float, no rounding but the
/// ceilings R5 names.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MinSupport {
    /// Tradable days in the window.
    window_days: u64,
    /// Tradable days in a gate year.
    gate_year_days: u64,
}

impl MinSupport {
    /// The floor of a window of `window_days` tradable days, against a gate year
    /// of `gate_year_days` (both from CAL-1).
    ///
    /// # Errors
    ///
    /// [`CubeRefusal::GateYearWithoutDays`] for a gate year with no tradable
    /// day.
    pub const fn new(window_days: u64, gate_year_days: u64) -> Result<Self, CubeRefusal> {
        if gate_year_days == 0 {
            return Err(CubeRefusal::GateYearWithoutDays);
        }
        Ok(Self {
            window_days,
            gate_year_days,
        })
    }

    /// `Q(w)` exactly, as `500 x window_days / gate_year_days`.
    #[must_use]
    pub fn q(&self) -> Ratio {
        Ratio {
            num: GATE_TRADES_PER_YEAR_MIN.saturating_mul(self.window_days),
            den: self.gate_year_days,
        }
    }

    /// `ceil(Q(w))`: the fewest trades a passing pair holds on the window.
    #[must_use]
    pub fn q_ceil(&self) -> u64 {
        let q = self.q();
        q.num.div_ceil(q.den)
    }

    /// Phase 1's floor per direction, `F1 = ceil(Q / 2)`: a passing pair's
    /// larger side holds at least this many raw rows.
    #[must_use]
    pub fn phase1_floor(&self) -> u64 {
        let q = self.q();
        let den = u128::from(q.den) * u128::from(PAIR_SIDES);
        let floor = u128::from(q.num).div_ceil(den);
        u64::try_from(floor).unwrap_or(u64::MAX)
    }

    /// Phase 2's floor for one direction, `F2 = max(1, ceil(Q) - M)`, where
    /// `M` is the largest raw count among the OTHER direction's phase-1
    /// passers.
    #[must_use]
    pub fn phase2_floor(&self, other_largest: u64) -> u64 {
        self.q_ceil().saturating_sub(other_largest).max(1)
    }
}

/// Which of the two separate searches a side value belongs to (operator
/// 16 Oct 16:04 IST: every mix is ONE direction; no mix pools long and short).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Search {
    /// `long_with_gainer` and `long_mirror_on_loser`.
    Long,
    /// `short_with_loser` and `short_mirror_on_gainer`.
    Short,
}

impl Search {
    /// The search's canonical name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Long => "long",
            Self::Short => "short",
        }
    }

    /// The search a side label belongs to: its first word, `long` or `short`.
    fn of_label(label: &str) -> Self {
        if label.starts_with(Self::Long.name()) {
            Self::Long
        } else {
            Self::Short
        }
    }

    /// The search a side value index belongs to, or `None` out of range.
    #[must_use]
    pub fn of_side(side: u8) -> Option<Self> {
        SIDE_ROW
            .labels
            .get(usize::from(side))
            .map(|label| Self::of_label(label))
    }

    /// Whether a full cell's side belongs to this search. The side is row 0,
    /// read by destructuring rather than by index.
    #[must_use]
    pub fn admits(self, coords: &Coords) -> bool {
        let [side, ..] = *coords;
        Self::of_side(side) == Some(self)
    }

    /// Checks a mix's side for this search: a fixed side value must belong to
    /// this search (an [`OPEN`] side means "either side value of this
    /// search"). The rest of the mix is [`Layout::validate_mix`]'s.
    ///
    /// # Errors
    ///
    /// [`CubeRefusal::SideOutsideSearch`].
    pub fn admits_mix(self, mix: &MixKey) -> Result<(), CubeRefusal> {
        let [side, ..] = mix.0;
        match SIDE_ROW.labels.get(usize::from(side)) {
            Some(label) if Self::of_label(label) != self => Err(CubeRefusal::SideOutsideSearch {
                search: self.name(),
                side: label,
            }),
            _ => Ok(()),
        }
    }
}

/// Appends a `u32` little-endian count.
fn count(out: &mut Vec<u8>, n: usize) {
    out.extend_from_slice(&u32::try_from(n).unwrap_or(u32::MAX).to_le_bytes());
}

/// Appends `bytes` with a `u32` little-endian length in front.
fn frame(out: &mut Vec<u8>, bytes: &[u8]) {
    count(out, bytes.len());
    out.extend_from_slice(bytes);
}

/// Appends an optional value: a presence byte, then the value when present.
fn push_option(out: &mut Vec<u8>, value: Option<u64>) {
    match value {
        None => out.push(0),
        Some(value) => {
            out.push(1);
            out.extend_from_slice(&value.to_le_bytes());
        }
    }
}

/// The schema's canonical bytes, for hashing in `runner` (this crate depends
/// on nothing and cannot hash): a magic, the version, and every row's name,
/// kind and source with its DEFINITIONAL labels (a definitional row's values, a
/// variant row's names; never a measured row's seed labels), then the zone
/// rungs and the open marker, each framed (17:15 IST, item c).
///
/// `runner::fib::KNOWN_SCHEMAS` pins the BLAKE3 of these bytes per version, so
/// an edit here without a version bump fails the build.
#[must_use]
pub fn descriptor() -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(b"BTXFIBC1");
    out.extend_from_slice(&FIB_CUBE_VERSION.to_le_bytes());
    count(&mut out, ROW_COUNT);
    for row in &ROWS {
        frame(&mut out, row.name.as_bytes());
        frame(&mut out, row.kind.name().as_bytes());
        frame(&mut out, row.source.name().as_bytes());
        let hashed: &[&str] = match row.source {
            Source::Definitional | Source::Variants => row.labels,
            Source::Measured => &[],
        };
        count(&mut out, hashed.len());
        for label in hashed {
            frame(&mut out, label.as_bytes());
        }
    }
    count(&mut out, ZONE_RUNGS.len());
    for rung in ZONE_RUNGS {
        out.extend_from_slice(&rung.to_le_bytes());
    }
    out.push(OPEN);
    out
}

/// The category of a fixed number: why it may be typed at all (17:15 IST,
/// FIB-1 item a).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Category {
    /// A Winner Gate threshold, confirmed by the operator (16 Oct 2026): the
    /// only numbers that DECIDE.
    WinnerGate,
    /// A session time the operator set (the 09:15 grid, the forced window and
    /// the auto-exit).
    OperatorSessionTime,
    /// A trade size the gate is judged at.
    TradeSize,
    /// A pattern size the operator named (micro bases of two and three candles).
    OperatorPatternSize,
    /// A Fibonacci ratio, pinned to `indicators::fib::RUNGS`.
    FibonacciRatio,
    /// A unit of measure (basis points, permille, seconds, calendar months).
    Unit,
    /// A representation choice: it sets a width, a resolution or a seed, never
    /// a threshold (R3).
    Representation,
}

impl Category {
    /// The category's canonical name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::WinnerGate => "winner_gate",
            Self::OperatorSessionTime => "operator_session_time",
            Self::TradeSize => "trade_size",
            Self::OperatorPatternSize => "operator_pattern_size",
            Self::FibonacciRatio => "fibonacci_ratio",
            Self::Unit => "unit",
            Self::Representation => "representation",
        }
    }
}

/// One ledger row: name, value, category, decision id.
pub type FixedNumber = (&'static str, u64, Category, &'static str);

/// Win rate at least 51%, net of the entire charges (permille).
pub const GATE_WIN_RATE_MIN_PERMILLE: u64 = 510;
/// Average win at least twice the average loss, in R.
pub const GATE_AVERAGE_WIN_RATIO_MIN: u64 = 2;
/// Profit factor at least two.
pub const GATE_PROFIT_FACTOR_MIN: u64 = 2;
/// Largest win at least twice the largest loss, in R.
pub const GATE_LARGEST_WIN_RATIO_MIN: u64 = 2;
/// At least 500 trades a year counted across the long + short pair, a partial
/// year pro-rated by its tradable days (operator 16 Oct 13:57 and 17:11 IST).
pub const GATE_TRADES_PER_YEAR_MIN: u64 = 500;
/// At least 70% of the unseen months positive (permille).
pub const GATE_UNSEEN_MONTHS_POSITIVE_MIN_PERMILLE: u64 = 700;
/// Every year's net strictly above this many paisa: every year positive.
pub const GATE_EVERY_YEAR_NET_ABOVE_PAISA: u64 = 0;
/// Each side's net on the unseen months strictly above this many paisa
/// (operator 16 Oct 17:11 IST: each side positive on its own).
pub const GATE_EACH_SIDE_UNSEEN_NET_ABOVE_PAISA: u64 = 0;

/// The 09:15:00 IST grid anchor, in seconds after IST midnight.
pub const SESSION_GRID_ANCHOR_SECS: u64 = 33_300;
/// The forced-exit window's first second, 15:09:00 IST.
pub const SESSION_FORCED_WINDOW_START_SECS: u64 = 54_540;
/// The forced-exit window's end, 15:10:00 IST (excluded).
pub const SESSION_FORCED_WINDOW_END_SECS: u64 = 54_600;
/// Every position is out by 15:10:00 IST (the auto-exit).
pub const SESSION_AUTO_EXIT_BY_SECS: u64 = 54_600;

/// Rs 1,00,000, the smaller trade size, in rupees.
pub const TRADE_SIZE_SMALL_RUPEES: u64 = 100_000;
/// Rs 5,00,000, the larger trade size, in rupees.
pub const TRADE_SIZE_LARGE_RUPEES: u64 = 500_000;

/// The two-candle micro base (rule c).
pub const MICRO2_CANDLES: u64 = 2;
/// The three-candle micro base (rule c).
pub const MICRO3_CANDLES: u64 = 3;

/// Basis points in one.
pub const BP_PER_UNIT: u64 = 10_000;
/// Permille in one.
pub const PERMILLE_PER_UNIT: u64 = 1_000;
/// Seconds in a minute.
pub const SECONDS_PER_MINUTE: u64 = 60;
/// Microseconds in a second.
pub const MICROS_PER_SECOND: u64 = 1_000_000;
/// Seconds in a day.
pub const SECONDS_PER_DAY: u64 = 86_400;
/// Calendar months in a gate year (D-6309): the exam and the one-gate-year
/// walk-forward reading are each one gate year (R6).
pub const MONTHS_PER_GATE_YEAR: u64 = 12;
/// Days in a week (the leave-one-out week).
pub const DAYS_PER_WEEK: u64 = 7;
/// Months in a quarter (the leave-one-out quarter; W0's two quarters, R6).
pub const MONTHS_PER_QUARTER: u64 = 3;
/// Months in a half-year (the report's period table).
pub const MONTHS_PER_HALF_YEAR: u64 = 6;

/// FIB-2's histogram SEED: sub-bins per power of two (R3). It sets the first
/// percentile resolution; R2 splits a bin by one more mantissa bit when a cut
/// needs it. Never a threshold.
pub const HIST_SUB_BINS_PER_POWER: u64 = 8;
/// The mid-rank pair doubles its counts so a half bin is an integer (R1).
pub const MID_RANK_SCALE: u64 = 2;
/// A pair is one long and one short setup (operator 16 Oct 17:11 IST).
pub const PAIR_SIDES: u64 = 2;

/// The FIB run files' checksum block, in bytes (FIB-8, FM-F091; D-6444): how
/// many bytes of whole records one verified read covers. A representation
/// choice, never a threshold.
pub const FIB_FILE_BLOCK_BYTES: u64 = 65_536;

/// THE ONE LEDGER of every number the FIB strategy path may type (operator
/// 16 Oct 11:24 IST, narrowed by 17:15 IST item a; D-6309): the Winner Gate
/// thresholds are the only fixed numbers that DECIDE; every other row is a
/// session time, a size, a pattern size, a ratio, a unit or a representation
/// seed, and its category says which. Every other value is generated from each
/// stock's own past.
///
/// Deleted, and refused by the source scan if typed again: the 0.2% minimum
/// stop (13:07 IST), 100 trades a year (13:57 IST), the own exits 15:07:00 and
/// 15:08:00, the report columns 5R/10R/20R and top 5% MFE (R4), every coarse
/// bucket count, the 2^40 histogram cap (R3), the typed timeframes (R7) and
/// the 2.5x room reading (item j).
///
/// APPEND-ONLY. A later FIB package that must type a representation number
/// (a digit width, a block length) appends its row at the end under its own
/// decision id; no row is ever edited or removed.
pub const FIXED_NUMBERS: &[FixedNumber] = &[
    (
        "gate_win_rate_min_permille",
        GATE_WIN_RATE_MIN_PERMILLE,
        Category::WinnerGate,
        "D-6309",
    ),
    (
        "gate_average_win_ratio_min",
        GATE_AVERAGE_WIN_RATIO_MIN,
        Category::WinnerGate,
        "D-6309",
    ),
    (
        "gate_profit_factor_min",
        GATE_PROFIT_FACTOR_MIN,
        Category::WinnerGate,
        "D-6309",
    ),
    (
        "gate_largest_win_ratio_min",
        GATE_LARGEST_WIN_RATIO_MIN,
        Category::WinnerGate,
        "D-6309",
    ),
    (
        "gate_trades_per_year_min",
        GATE_TRADES_PER_YEAR_MIN,
        Category::WinnerGate,
        "D-6309",
    ),
    (
        "gate_unseen_months_positive_min_permille",
        GATE_UNSEEN_MONTHS_POSITIVE_MIN_PERMILLE,
        Category::WinnerGate,
        "D-6309",
    ),
    (
        "gate_every_year_net_above_paisa",
        GATE_EVERY_YEAR_NET_ABOVE_PAISA,
        Category::WinnerGate,
        "D-6309",
    ),
    (
        "gate_each_side_unseen_net_above_paisa",
        GATE_EACH_SIDE_UNSEEN_NET_ABOVE_PAISA,
        Category::WinnerGate,
        "D-6309",
    ),
    (
        "session_grid_anchor_secs",
        SESSION_GRID_ANCHOR_SECS,
        Category::OperatorSessionTime,
        "D-6309",
    ),
    (
        "session_forced_window_start_secs",
        SESSION_FORCED_WINDOW_START_SECS,
        Category::OperatorSessionTime,
        "D-6309",
    ),
    (
        "session_forced_window_end_secs",
        SESSION_FORCED_WINDOW_END_SECS,
        Category::OperatorSessionTime,
        "D-6309",
    ),
    (
        "session_auto_exit_by_secs",
        SESSION_AUTO_EXIT_BY_SECS,
        Category::OperatorSessionTime,
        "D-6309",
    ),
    (
        "trade_size_small_rupees",
        TRADE_SIZE_SMALL_RUPEES,
        Category::TradeSize,
        "D-6309",
    ),
    (
        "trade_size_large_rupees",
        TRADE_SIZE_LARGE_RUPEES,
        Category::TradeSize,
        "D-6309",
    ),
    (
        "micro2_candles",
        MICRO2_CANDLES,
        Category::OperatorPatternSize,
        "D-6309",
    ),
    (
        "micro3_candles",
        MICRO3_CANDLES,
        Category::OperatorPatternSize,
        "D-6309",
    ),
    (
        "zone_rung_236 = indicators::fib::RUNGS[1]",
        ZONE_RUNG_236,
        Category::FibonacciRatio,
        "D-6309",
    ),
    (
        "zone_rung_382 = indicators::fib::RUNGS[2]",
        ZONE_RUNG_382,
        Category::FibonacciRatio,
        "D-6309",
    ),
    (
        "zone_rung_500 = indicators::fib::RUNGS[3]",
        ZONE_RUNG_500,
        Category::FibonacciRatio,
        "D-6309",
    ),
    (
        "zone_rung_618 = indicators::fib::RUNGS[4]",
        ZONE_RUNG_618,
        Category::FibonacciRatio,
        "D-6309",
    ),
    (
        "zone_rung_786 = indicators::fib::RUNGS[5]",
        ZONE_RUNG_786,
        Category::FibonacciRatio,
        "D-6309",
    ),
    (
        "zone_rung_1000 = indicators::fib::RUNGS[6]",
        ZONE_RUNG_1000,
        Category::FibonacciRatio,
        "D-6309",
    ),
    ("bp_per_unit", BP_PER_UNIT, Category::Unit, "D-6309"),
    (
        "permille_per_unit",
        PERMILLE_PER_UNIT,
        Category::Unit,
        "D-6309",
    ),
    (
        "seconds_per_minute",
        SECONDS_PER_MINUTE,
        Category::Unit,
        "D-6309",
    ),
    (
        "micros_per_second",
        MICROS_PER_SECOND,
        Category::Unit,
        "D-6309",
    ),
    ("seconds_per_day", SECONDS_PER_DAY, Category::Unit, "D-6309"),
    (
        "months_per_gate_year",
        MONTHS_PER_GATE_YEAR,
        Category::Unit,
        "D-6309",
    ),
    ("days_per_week", DAYS_PER_WEEK, Category::Unit, "D-6309"),
    (
        "months_per_quarter",
        MONTHS_PER_QUARTER,
        Category::Unit,
        "D-6309",
    ),
    (
        "months_per_half_year",
        MONTHS_PER_HALF_YEAR,
        Category::Unit,
        "D-6309",
    ),
    (
        "hist_sub_bins_per_power",
        HIST_SUB_BINS_PER_POWER,
        Category::Representation,
        "D-6309",
    ),
    (
        "schema_row_count",
        ROW_COUNT as u64,
        Category::Representation,
        "D-6301",
    ),
    (
        "digest_bytes",
        DIGEST_BYTES as u64,
        Category::Representation,
        "D-6303",
    ),
    (
        "mid_rank_scale",
        MID_RANK_SCALE,
        Category::Representation,
        "D-6312",
    ),
    ("pair_sides", PAIR_SIDES, Category::Representation, "D-6312"),
    (
        "fib_file_block_bytes",
        FIB_FILE_BLOCK_BYTES,
        Category::Representation,
        "D-6444",
    ),
];

/// The Winner Gate's clauses that carry no number of their own (17:15 IST
/// item a; the gate itself is FIB-7's, D-6310): named here so the `FIXED_NUMBERS`
/// digest covers them.
pub const GATE_CLAUSES: &[(&str, &str)] = &[
    ("every_year_positive", "D-6309"),
    ("each_side_quality_and_unseen_positive", "D-6309"),
    ("walk_forward_unseen_months_pass_together", "D-6309"),
    ("learned_loss_floor", "D-6309"),
    ("leave_one_out_day", "D-6309"),
    ("leave_one_out_week", "D-6309"),
    ("leave_one_out_fortnight", "D-6309"),
    ("leave_one_out_month", "D-6309"),
    ("leave_one_out_quarter", "D-6309"),
];

/// The ledger's canonical bytes, for the `FIXED_NUMBERS` digest a pair identity
/// carries (FM-F007): every row (name, value, category, decision id) and every
/// gate clause, framed, in ledger order.
#[must_use]
pub fn fixed_numbers_bytes() -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(b"BTXFIBN1");
    count(&mut out, FIXED_NUMBERS.len());
    for (name, value, category, decision) in FIXED_NUMBERS {
        frame(&mut out, name.as_bytes());
        out.extend_from_slice(&value.to_le_bytes());
        frame(&mut out, category.name().as_bytes());
        frame(&mut out, decision.as_bytes());
    }
    count(&mut out, GATE_CLAUSES.len());
    for (name, decision) in GATE_CLAUSES {
        frame(&mut out, name.as_bytes());
        frame(&mut out, decision.as_bytes());
    }
    out
}

/// Whether `value` is the value of some [`FIXED_NUMBERS`] row.
#[must_use]
pub fn is_fixed_number(value: u64) -> bool {
    FIXED_NUMBERS.iter().any(|(_, fixed, _, _)| *fixed == value)
}

/// One numeric literal found by [`numeric_literals`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NumericLiteral {
    /// The 1-based source line.
    pub line: usize,
    /// The literal as written, suffix included.
    pub text: String,
    /// Its value when it is a plain decimal integer (underscores and an
    /// integer suffix allowed); `None` for a hexadecimal, octal, binary,
    /// exponent or fractional literal, which a source scan must refuse.
    pub value: Option<u64>,
}

/// Whether `suffix` is an integer type suffix a decimal literal may carry
/// (or none at all).
fn is_integer_suffix(suffix: &str) -> bool {
    matches!(
        suffix,
        "" | "u8"
            | "u16"
            | "u32"
            | "u64"
            | "u128"
            | "usize"
            | "i8"
            | "i16"
            | "i32"
            | "i64"
            | "i128"
            | "isize"
    )
}

/// Whether a character continues an identifier or a literal token.
fn is_word(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// The source with every comment, string literal and character literal
/// replaced by spaces (newlines kept), so what is left is code.
fn code_only(source: &str) -> Vec<char> {
    let chars: Vec<char> = source.chars().collect();
    let mut out = chars.clone();
    let mut at = 0;
    let blank = |out: &mut Vec<char>, from: usize, to: usize| {
        for c in out.iter_mut().take(to).skip(from) {
            if *c != '\n' {
                *c = ' ';
            }
        }
    };
    let get = |i: usize| chars.get(i).copied();
    while let Some(c) = get(at) {
        let next = get(at + 1);
        let prev = at.checked_sub(1).and_then(get);
        let end = if c == '/' && next == Some('/') {
            let mut end = at;
            while get(end).is_some_and(|c| c != '\n') {
                end += 1;
            }
            end
        } else if c == '/' && next == Some('*') {
            let mut depth = 0_usize;
            let mut end = at;
            while let Some(c) = get(end) {
                if c == '/' && get(end + 1) == Some('*') {
                    depth += 1;
                    end += 1;
                } else if c == '*' && get(end + 1) == Some('/') {
                    depth -= 1;
                    end += 1;
                    if depth == 0 {
                        end += 1;
                        break;
                    }
                }
                end += 1;
            }
            end
        } else if c == 'r'
            && !prev.is_some_and(|p| is_word(p) && p != 'b')
            && matches!(next, Some('"' | '#'))
        {
            let mut hashes = 0;
            while get(at + 1 + hashes) == Some('#') {
                hashes += 1;
            }
            if get(at + 1 + hashes) == Some('"') {
                let mut end = at + hashes + 2;
                loop {
                    match get(end) {
                        None => break,
                        Some('"') if (1..=hashes).all(|h| get(end + h) == Some('#')) => {
                            end += hashes + 1;
                            break;
                        }
                        Some(_) => end += 1,
                    }
                }
                end
            } else {
                at + 1
            }
        } else if c == '"' {
            let mut end = at + 1;
            loop {
                match get(end) {
                    None => break,
                    Some('\\') => end += 2,
                    Some('"') => {
                        end += 1;
                        break;
                    }
                    Some(_) => end += 1,
                }
            }
            end
        } else if c == '\'' && next == Some('\\') {
            // Past the backslash AND the escaped character, so `'\''` closes
            // on its own last quote rather than the escaped one.
            let mut end = at + 3;
            while get(end).is_some_and(|c| c != '\'') {
                end += 1;
            }
            end + 1
        } else if c == '\'' && get(at + 2) == Some('\'') {
            at + 3
        } else {
            at + 1
        };
        if end > at + 1 {
            blank(&mut out, at, end);
        }
        at = end;
    }
    out
}

/// The value of a decimal integer literal, or `None` for any other form.
fn decimal_value(text: &str) -> Option<u64> {
    let digits_end = text
        .find(|c: char| !(c.is_ascii_digit() || c == '_'))
        .unwrap_or(text.len());
    let (digits, suffix) = text.split_at(digits_end);
    if !is_integer_suffix(suffix) {
        return None;
    }
    digits.replace('_', "").parse().ok()
}

/// Every numeric literal in `source`, outside comments, strings and character
/// literals, in source order.
///
/// A token starting with a digit that does not continue an identifier is a
/// literal; one right after a single `.` is a tuple field or a fraction and is
/// not (the fraction's integer part is reported with no value). A range's end
/// (`..10`) is a literal.
#[must_use]
pub fn numeric_literals(source: &str) -> Vec<NumericLiteral> {
    let code = code_only(source);
    let get = |i: usize| code.get(i).copied();
    let mut out = Vec::new();
    let mut line = 1;
    let mut at = 0;
    while let Some(c) = get(at) {
        if c == '\n' {
            line += 1;
        }
        let prev = at.checked_sub(1).and_then(get);
        let field = prev == Some('.') && at.checked_sub(2).and_then(get) != Some('.');
        if !c.is_ascii_digit() || prev.is_some_and(is_word) || field {
            at += 1;
            continue;
        }
        let mut end = at;
        while get(end).is_some_and(is_word) {
            end += 1;
        }
        let text: String = code.iter().take(end).skip(at).collect();
        let fraction = get(end) == Some('.') && get(end + 1).is_some_and(|c| c.is_ascii_digit());
        let value = if fraction { None } else { decimal_value(&text) };
        out.push(NumericLiteral { line, text, value });
        at = end;
    }
    out
}

/// The shipping part of a source file: everything before its first line that
/// is exactly `#[cfg(test)]` (the convention every FIB file keeps: its test
/// module is last).
#[must_use]
pub fn shipping_part(source: &str) -> &str {
    let mut offset = 0;
    for line in source.split_inclusive('\n') {
        if line.trim_end() == "#[cfg(test)]" {
            return source.get(..offset).unwrap_or(source);
        }
        offset += line.len();
    }
    source
}

/// The literals in the shipping part of `source` other than 0 and 1: what a
/// FIB strategy file may not contain (operator 16 Oct 11:24 IST). A later FIB
/// package's source-scan test asserts this is empty for each of its files.
#[must_use]
pub fn untyped_numbers(source: &str) -> Vec<NumericLiteral> {
    numeric_literals(shipping_part(source))
        .into_iter()
        .filter(|literal| !matches!(literal.value, Some(0 | 1)))
        .collect()
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::too_many_lines,
    reason = "the exception every test module in this workspace takes: a test \
              that cannot panic cannot fail."
)]
mod tests {
    //! The FIB cube schema v1, proved against its specification (FIB-1,
    //! D-6301, D-6312 to D-6315).

    use super::*;

    /// A deterministic generator for the seeded property tests (splitmix64).
    struct Seeded(u64);

    impl Seeded {
        fn next(&mut self) -> u64 {
            self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
            let mut z = self.0;
            z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
            z ^ (z >> 31)
        }

        fn below(&mut self, n: u8) -> u8 {
            u8::try_from(self.next() % u64::from(n)).unwrap()
        }

        fn cell(&mut self, layout: &Layout) -> Coords {
            let mut c = [0; ROW_COUNT];
            for (slot, radix) in c.iter_mut().zip(layout.radices()) {
                *slot = self.below(*radix);
            }
            c
        }

        fn mix(&mut self, layout: &Layout) -> MixKey {
            let mut m = [OPEN; ROW_COUNT];
            for ((slot, radix), row) in m.iter_mut().zip(layout.radices()).zip(&ROWS) {
                let free = row.kind != RowKind::Partition && self.next().is_multiple_of(3);
                if !free {
                    *slot = self.below(*radix);
                }
            }
            MixKey(m)
        }
    }

    fn half() -> Ratio {
        Ratio::new(1, 2).unwrap()
    }

    fn seed_list(row: RowId) -> CutList {
        CutList {
            row,
            timeframe: None,
            base_kind: None,
            cuts: vec![half()],
        }
    }

    fn time_list(minutes: u16, candles: &[u64]) -> CutList {
        CutList {
            row: RowId::TimeOfDay,
            timeframe: Some(minutes),
            base_kind: None,
            cuts: candles
                .iter()
                .map(|n| Ratio::whole(n * u64::from(minutes) * 60))
                .collect(),
        }
    }

    /// Every measured row's lists in canonical order: one seed cut each, three
    /// time lists (one per width) and two base-kind-scoped tightness lists.
    fn cut_lists() -> Vec<CutList> {
        let mut lists = Vec::new();
        for id in RowId::ALL {
            match id {
                RowId::TimeOfDay => {
                    lists.push(time_list(1, &[90]));
                    lists.push(time_list(2, &[45]));
                    lists.push(time_list(4, &[20, 40]));
                }
                RowId::BaseTightness => {
                    lists.push(CutList {
                        row: id,
                        timeframe: None,
                        base_kind: Some(0),
                        cuts: vec![Ratio::new(1, 3).unwrap(), Ratio::new(2, 3).unwrap()],
                    });
                    lists.push(CutList {
                        row: id,
                        timeframe: None,
                        base_kind: Some(2),
                        cuts: vec![half()],
                    });
                }
                _ if id.row().source == Source::Measured && id != RowId::MoverRank => {
                    lists.push(seed_list(id));
                }
                _ => {}
            }
        }
        lists
    }

    fn parts() -> GenerationParts {
        GenerationParts {
            search: Search::Long,
            version: 1,
            birth_step: 6,
            timeframes: vec![1, 2, 4],
            rank_edges: vec![3, 8, 8, 20],
            cuts: cut_lists(),
            exit_set: VariantSet {
                version: 7,
                digest: [0xAB; DIGEST_BYTES],
                count: 4,
            },
            entry_variants: EntryVariants {
                version: 2,
                digest: [0xCD; DIGEST_BYTES],
                styles: vec![0, 1, 2],
            },
            others: vec![GeneratedList {
                name: "touch_levels",
                version: 1,
                birth_step: 6,
                digest: [7; DIGEST_BYTES],
            }],
        }
    }

    fn generation() -> Generation {
        Generation::new(parts()).unwrap()
    }

    fn layout() -> Layout {
        *generation().layout()
    }

    /// The paper counts recomputed independently of [`Layout`].
    fn formulas(radices: &[u8; ROW_COUNT]) -> (u128, Option<u128>) {
        let full = radices.iter().map(|r| u128::from(*r)).product();
        let partial = radices.iter().zip(&ROWS).try_fold(1_u128, |acc, (r, row)| {
            let choices = match row.kind {
                RowKind::Partition => u128::from(*r),
                _ => u128::from(*r) + 1,
            };
            acc.checked_mul(choices)
        });
        (full, partial)
    }

    #[test]
    fn the_schema_is_twenty_four_rows_in_canonical_order_with_its_sources() {
        let names: Vec<&str> = ROWS.iter().map(|r| r.name).collect();
        assert_eq!(
            names,
            [
                "side",
                "mover_rank",
                "move_so_far",
                "fib_zone",
                "level_distance",
                "level_relation",
                "base_holds_extreme",
                "exit",
                "weekday",
                "time_of_day",
                "base_kind",
                "base_shape",
                "base_tightness",
                "base_length",
                "edge_touches",
                "squeeze",
                "trigger_body",
                "trigger_close_location",
                "trigger_order_flow",
                "stop_size",
                "room_to_stop",
                "entry_style",
                "attempt",
                "timeframe",
            ]
        );
        assert_eq!(ROW_COUNT, 24);
        assert_eq!(FIB_CUBE_VERSION, 1);
        assert!(ids_in_order(&RowId::ALL, &ROWS));
        // Each way the order can break is refused: an index off its place, a
        // row of another name, and lists of two lengths.
        assert!(!ids_in_order(
            &[RowId::Side, RowId::Side],
            &[SIDE_ROW, SIDE_ROW]
        ));
        assert!(!ids_in_order(
            &[RowId::Side, RowId::MoverRank],
            &[MOVER_RANK_ROW, SIDE_ROW]
        ));
        assert!(!ids_in_order(&[RowId::Side], &[SIDE_ROW, MOVER_RANK_ROW]));
        assert!(!ids_in_order(&[RowId::Side, RowId::MoverRank], &[SIDE_ROW]));
        assert!(ids_in_order(&[RowId::Side], &[SIDE_ROW]));
        assert!(same_name("abc", "abc"));
        assert!(!same_name("abc", "abd"));
        assert!(!same_name("ab", "abc"));
        assert!(!same_name("abc", "ab"));
        let kinds: Vec<(usize, RowKind)> = ROWS
            .iter()
            .enumerate()
            .filter(|(_, r)| r.kind != RowKind::Categorical)
            .map(|(i, r)| (i, r.kind))
            .collect();
        assert_eq!(
            kinds,
            [
                (1, RowKind::Cumulative),
                (7, RowKind::Partition),
                (21, RowKind::Partition),
                (23, RowKind::Partition),
            ]
        );
        // Item c: the definitional rows are exactly these, the measured rows
        // exactly these, and every partition list is generated.
        let of = |source: Source| -> Vec<&str> {
            ROWS.iter()
                .filter(|r| r.source == source)
                .map(|r| r.name)
                .collect()
        };
        assert_eq!(
            of(Source::Definitional),
            [
                "side",
                "fib_zone",
                "level_relation",
                "base_holds_extreme",
                "weekday",
                "base_kind",
                "base_shape",
                "trigger_order_flow",
            ]
        );
        assert_eq!(
            of(Source::Measured),
            [
                "mover_rank",
                "move_so_far",
                "level_distance",
                "time_of_day",
                "base_tightness",
                "base_length",
                "edge_touches",
                "squeeze",
                "trigger_body",
                "trigger_close_location",
                "stop_size",
                "room_to_stop",
                "attempt",
            ]
        );
        assert_eq!(of(Source::Variants), ["exit", "entry_style", "timeframe"]);
        assert_eq!(
            SIDE_ROW.labels,
            [
                "long_with_gainer",
                "short_with_loser",
                "short_mirror_on_gainer",
                "long_mirror_on_loser"
            ]
        );
        assert_eq!(
            FIB_ZONE_ROW.labels,
            ["l0", "l236", "l382", "l500", "l618", "l786", "l1000"]
        );
        assert_eq!(BASE_SHAPE_ROW.labels.len(), 12);
        assert_eq!(BASE_SHAPE_ROW.labels[11], "stair");
        // FM-F004: row 18 has five values and none of them is `none`.
        assert_eq!(
            TRIGGER_ORDER_FLOW_ROW.labels,
            ["with", "against", "balanced", "unclassified", "absent"]
        );
        // Operator 13:05 IST and item f: the exit row has no typed label; R7:
        // nor has the timeframe row; item e: nor has time of day.
        assert!(EXIT_ROW.labels.is_empty());
        assert!(TIMEFRAME_ROW.labels.is_empty());
        assert!(TIME_OF_DAY_ROW.labels.is_empty());
        assert_eq!(ENTRY_STYLE_ROW.labels, ["close", "break", "retest"]);
        assert_eq!(MOVER_RANK_ROW.labels, ["rank_g1", "rank_g2"]);
        assert_eq!(ENTRY_STYLE_NAMES, 3);
        assert_eq!(BASE_KINDS, 3);
        assert_eq!(MAX_ROW_VALUES, 254);
        assert_eq!(MAX_CUTS, 253);
        for (kind, name) in [
            (RowKind::Categorical, "categorical"),
            (RowKind::Cumulative, "cumulative"),
            (RowKind::Partition, "partition"),
        ] {
            assert_eq!(kind.name(), name);
        }
        for (source, name) in [
            (Source::Definitional, "definitional"),
            (Source::Measured, "measured"),
            (Source::Variants, "variants"),
        ] {
            assert_eq!(source.name(), name);
        }
    }

    #[test]
    fn every_row_id_names_its_row_and_only_definitional_labels_are_values() {
        for (index, (id, row)) in RowId::ALL.iter().zip(&ROWS).enumerate() {
            assert_eq!(id.index(), index);
            assert_eq!(id.row(), row);
            for (value, label) in (0_u8..).zip(row.labels) {
                let expected = (row.source == Source::Definitional).then_some(value);
                assert_eq!(id.value_of(label), expected, "{label}");
            }
            assert_eq!(id.value_of("no_such_value"), None);
        }
        assert_eq!(RowId::TriggerOrderFlow.value_of("absent"), Some(4));
        assert_eq!(RowId::MoveSoFar.value_of("low"), None, "a seed label");
        assert_eq!(RowId::EntryStyle.value_of("close"), None, "a variant name");
    }

    #[test]
    fn paper_counts_follow_the_formulas_on_synthetic_generations() {
        let g = generation();
        let l = g.layout();
        let radices: Vec<u8> = l.radices().to_vec();
        assert_eq!(
            radices,
            [
                4, 5, 2, 7, 2, 3, 2, 4, 5, 3, 3, 12, 3, 2, 2, 2, 2, 2, 5, 2, 2, 3, 2, 3
            ]
        );
        let (full, partial) = formulas(l.radices());
        assert_eq!(l.paper_full_cells(), full);
        assert_eq!(l.paper_partial_mixes(), Ok(partial.unwrap()));
        assert_eq!(full, 250_822_656_000, "paper only");
        assert_eq!(partial, Some(183_372_143_984_640), "paper only");
        assert_eq!(l.total_values(), 82);
        assert_eq!(l.key_width(), KeyWidth::Bits64);
        // Radix 5 on row 18 (FM-F004): it contributes 5 cells and 6 mix choices.
        assert_eq!(l.radix(RowId::TriggerOrderFlow), 5);
        let mut without = *l.radices();
        without[RowId::TriggerOrderFlow.index()] = 1;
        let (full_1, partial_1) = formulas(&without);
        assert_eq!(full, full_1 * 5);
        assert_eq!(partial.unwrap(), partial_1.unwrap() / 2 * 6);
        // The pure seed: every measured row one median cut, one rank edge, one
        // width, one exit, one entry variant — every generated row two buckets
        // and every partition one value.
        let mut p = parts();
        p.timeframes = vec![1];
        p.rank_edges = vec![5];
        p.cuts = RowId::ALL
            .iter()
            .filter(|id| id.row().source == Source::Measured && **id != RowId::MoverRank)
            .map(|id| {
                if *id == RowId::TimeOfDay {
                    time_list(1, &[180])
                } else {
                    seed_list(*id)
                }
            })
            .collect();
        p.exit_set.count = 1;
        p.entry_variants.styles = vec![0];
        let seed = Generation::new(p).unwrap();
        let s = seed.layout();
        let (full, partial) = formulas(s.radices());
        assert_eq!(full, 151_200 * 2_u128.pow(13), "definitional x 2^13");
        assert_eq!(s.paper_full_cells(), full);
        assert_eq!(s.paper_partial_mixes(), Ok(partial.unwrap()));
        assert_eq!(partial, Some(898_560 * 3_u128.pow(13)));
    }

    #[test]
    fn exit_set_of_seven_round_trips_and_counts() {
        let mut p = parts();
        p.exit_set.count = 7;
        let g = Generation::new(p).unwrap();
        let l = g.layout();
        assert_eq!(l.radix(RowId::Exit), 7);
        let (full, partial) = formulas(l.radices());
        assert_eq!(l.paper_full_cells(), full);
        assert_eq!(full, layout().paper_full_cells() / 4 * 7);
        assert_eq!(l.paper_partial_mixes(), Ok(partial.unwrap()));
        let exit = RowId::Exit.index();
        let mut cell = [0; ROW_COUNT];
        for slot in 0..7 {
            cell[exit] = slot;
            let code = l.encode(&cell).unwrap();
            assert_eq!(l.decode(code).unwrap(), cell);
        }
        cell[exit] = 7;
        assert_eq!(
            l.encode(&cell),
            Err(CubeRefusal::ValueOutOfRange {
                row: "exit",
                value: 7,
                values: 7
            })
        );
        // A different exit set is a different per-run descriptor.
        assert_ne!(g.run_descriptor(), generation().run_descriptor());
    }

    #[test]
    fn encode_and_decode_round_trip_on_every_radix_boundary() {
        let l = layout();
        let last: Coords = {
            let mut c = [0; ROW_COUNT];
            for (slot, radix) in c.iter_mut().zip(l.radices()) {
                *slot = radix - 1;
            }
            c
        };
        assert_eq!(l.encode(&last).unwrap(), l.paper_full_cells() - 1);
        assert_eq!(l.decode(l.paper_full_cells() - 1).unwrap(), last);
        assert_eq!(l.encode(&[0; ROW_COUNT]).unwrap(), 0);
        assert_eq!(l.decode(0).unwrap(), [0; ROW_COUNT]);
        let mut place: u128 = 1;
        for row in (0..ROW_COUNT).rev() {
            let radix = l.radices()[row];
            let mut c = [0; ROW_COUNT];
            if radix > 1 {
                c[row] = 1;
                assert_eq!(l.encode(&c).unwrap(), place, "row {row}");
                assert_eq!(l.decode(place).unwrap(), c);
            }
            let mut below = [0; ROW_COUNT];
            below[row + 1..].copy_from_slice(&last[row + 1..]);
            assert_eq!(l.encode(&below).unwrap(), place - 1, "row {row}");
            assert_eq!(l.decode(place - 1).unwrap(), below);
            c[row] = last[row];
            let code = l.encode(&c).unwrap();
            assert_eq!(l.decode(code).unwrap(), c);
            place *= u128::from(radix);
        }
        assert_eq!(place, l.paper_full_cells());
    }

    #[test]
    fn encode_and_decode_round_trip_on_100000_seeded_cells() {
        let l = layout();
        let mut rng = Seeded(0x5EED_F1B1);
        let mut previous = None;
        for _ in 0..100_000 {
            let cell = rng.cell(&l);
            let code = l.encode(&cell).unwrap();
            assert!(code < l.paper_full_cells());
            assert_eq!(l.decode(code).unwrap(), cell);
            // Row 0 most significant: comparing codes compares cells lexically.
            if let Some((pc, pcode)) = previous {
                assert_eq!(cell.cmp(&pc), code.cmp(&pcode));
            }
            previous = Some((cell, code));
        }
    }

    #[test]
    fn every_cell_and_mix_refusal_is_named() {
        let l = layout();
        let mut cell = [0; ROW_COUNT];
        cell[3] = 7;
        let refusal = l.validate(&cell).unwrap_err();
        assert_eq!(
            refusal,
            CubeRefusal::ValueOutOfRange {
                row: "fib_zone",
                value: 7,
                values: 7
            }
        );
        assert_eq!(refusal.name(), "value_out_of_range");
        assert_eq!(l.encode(&cell), Err(refusal));
        assert_eq!(l.to_mask(&cell), Err(refusal));
        cell[3] = 0;
        cell[5] = OPEN;
        assert_eq!(
            l.validate(&cell),
            Err(CubeRefusal::OpenInCell {
                row: "level_relation"
            })
        );
        let full = l.paper_full_cells();
        assert_eq!(
            l.decode(full),
            Err(CubeRefusal::CodeOutOfRange { code: full })
        );
        assert_eq!(l.decode(u128::MAX).unwrap_err().name(), "code_out_of_range");
        let mut mix = MixKey([OPEN; ROW_COUNT]);
        mix.0[7] = 0;
        mix.0[21] = 0;
        assert_eq!(
            l.validate_mix(&mix),
            Err(CubeRefusal::OpenPartitionRow { row: "timeframe" })
        );
        assert_eq!(l.mix_mask(&mix).unwrap_err().name(), "open_partition_row");
        mix.0[23] = 3;
        assert_eq!(
            l.validate_mix(&mix),
            Err(CubeRefusal::ValueOutOfRange {
                row: "timeframe",
                value: 3,
                values: 3
            })
        );
        let shown = CubeRefusal::OpenInCell { row: "side" }.to_string();
        assert_eq!(
            shown,
            "fib cube refused (open_in_cell): OpenInCell { row: \"side\" }"
        );
        let boxed: Box<dyn std::error::Error> = Box::new(CubeRefusal::ExitSetEmpty);
        assert!(boxed.to_string().contains("exit_set_empty"));
    }

    #[test]
    fn every_refusal_has_its_own_name() {
        let all = [
            CubeRefusal::ValueOutOfRange {
                row: "r",
                value: 0,
                values: 0,
            },
            CubeRefusal::OpenInCell { row: "r" },
            CubeRefusal::OpenPartitionRow { row: "r" },
            CubeRefusal::CodeOutOfRange { code: 0 },
            CubeRefusal::PermilleOutOfRange { permille: 0 },
            CubeRefusal::RetraceOutOfRange { part: 0, whole: 0 },
            CubeRefusal::RatioZeroDenominator { num: 0 },
            CubeRefusal::MidRankInvalid {
                below: 0,
                in_bin: 0,
                total: 0,
            },
            CubeRefusal::QuantileInvalid { num: 0, den: 0 },
            CubeRefusal::EmptySample,
            CubeRefusal::RankEdgesInvalid { at: 0, edge: 0 },
            CubeRefusal::TimeframesEmpty,
            CubeRefusal::TimeframesInvalid { at: 0, minutes: 0 },
            CubeRefusal::CutOnUnmeasuredRow { row: "r" },
            CubeRefusal::CutTimeframeUnknown {
                row: "r",
                minutes: 0,
            },
            CubeRefusal::CutBaseKindOutOfRange {
                row: "r",
                base_kind: 0,
            },
            CubeRefusal::CutsNotIncreasing { row: "r", at: 0 },
            CubeRefusal::TimeCutsWithoutTimeframe,
            CubeRefusal::TimeCutOffGrid { minutes: 0, at: 0 },
            CubeRefusal::CutListsNotCanonical { row: "r" },
            CubeRefusal::MeasuredRowWithoutCuts { row: "r" },
            CubeRefusal::RowValuesExceedU8 {
                row: "r",
                values: 0,
            },
            CubeRefusal::RadixProductExceedsKey,
            CubeRefusal::PaperCountExceedsU128,
            CubeRefusal::FibMaskExceedsWords { values: 0, bits: 0 },
            CubeRefusal::ExitSetEmpty,
            CubeRefusal::EntryVariantsEmpty,
            CubeRefusal::EntryStyleUnknown { style: 0 },
            CubeRefusal::GeneratedListsNotCanonical { at: 0 },
            CubeRefusal::ListBornAfterGeneration { at: 0 },
            CubeRefusal::ChainSearchMismatch,
            CubeRefusal::ChainVersionNotNext {
                expected: 0,
                got: 0,
            },
            CubeRefusal::ChainBirthNotLater {
                previous: 0,
                got: 0,
            },
            CubeRefusal::GateYearWithoutDays,
            CubeRefusal::SideOutsideSearch {
                search: "long",
                side: "short_with_loser",
            },
        ];
        let mut names: Vec<&str> = all.iter().map(CubeRefusal::name).collect();
        assert!(names.iter().all(|n| {
            n.chars()
                .all(|c| c.is_ascii_lowercase() || c == '_' || c.is_ascii_digit())
        }));
        names.sort_unstable();
        let before = names.len();
        names.dedup();
        assert_eq!(names.len(), before, "two refusals share a name");
        assert_eq!(before, 35);
    }

    #[test]
    fn validate_mix_refuses_every_open_partition_row_and_accepts_free_rows() {
        let l = layout();
        let cell = [0; ROW_COUNT];
        let root = MixKey::root_of(&cell);
        assert_eq!(l.validate_mix(&root), Ok(()));
        for (index, row) in ROWS.iter().enumerate() {
            let mut mix = MixKey(cell);
            mix.0[index] = OPEN;
            let verdict = l.validate_mix(&mix);
            if row.kind == RowKind::Partition {
                assert_eq!(
                    verdict,
                    Err(CubeRefusal::OpenPartitionRow { row: row.name })
                );
            } else {
                assert_eq!(verdict, Ok(()), "{}", row.name);
            }
            mix.0[index] = l.radices()[index];
            assert_eq!(
                l.validate_mix(&mix),
                Err(CubeRefusal::ValueOutOfRange {
                    row: row.name,
                    value: mix.0[index],
                    values: usize::from(l.radices()[index])
                })
            );
        }
    }

    #[test]
    fn root_of_frees_every_row_but_the_three_partition_rows() {
        let l = layout();
        let mut rng = Seeded(7);
        let cell = rng.cell(&l);
        let root = MixKey::root_of(&cell);
        for ((value, have), row) in root.0.iter().zip(&cell).zip(&ROWS) {
            if row.kind == RowKind::Partition {
                assert_eq!(value, have);
            } else {
                assert_eq!(*value, OPEN);
            }
        }
        assert!(contains(&root, &cell));
    }

    #[test]
    fn contains_nests_the_rank_and_matches_every_other_row_exactly() {
        let mut cell = [0; ROW_COUNT];
        let rank = RowId::MoverRank.index();
        let mut mix = MixKey::root_of(&cell);
        // A rank_g1 cell is in a rank_g3 mix; a rank_g3 cell is never in a
        // rank_g2 mix.
        cell[rank] = 0;
        mix.0[rank] = 2;
        assert!(contains(&mix, &cell));
        cell[rank] = 2;
        assert!(contains(&mix, &cell));
        cell[rank] = 3;
        assert!(!contains(&mix, &cell));
        cell[rank] = 2;
        mix.0[rank] = 1;
        assert!(!contains(&mix, &cell));
        mix.0[rank] = OPEN;
        assert!(contains(&mix, &cell));
        // A categorical row matches exactly: neither above nor below.
        let shape = RowId::BaseShape.index();
        cell[shape] = 5;
        for want in 0..12_u8 {
            mix.0[shape] = want;
            assert_eq!(contains(&mix, &cell), want == 5, "{want}");
        }
        // A partition row matches exactly too.
        mix.0[shape] = OPEN;
        let tf = RowId::Timeframe.index();
        mix.0[tf] = 1;
        assert!(!contains(&mix, &cell));
        cell[tf] = 1;
        assert!(contains(&mix, &cell));
        // The last row decides even when every row before it matches.
        cell[tf] = 2;
        assert!(!contains(&mix, &cell));
    }

    #[test]
    fn to_mask_hits_a_mix_mask_exactly_when_contains_says_so() {
        let l = layout();
        let rank_radix = usize::from(l.radix(RowId::MoverRank));
        let mut rng = Seeded(0x00C0_FFEE);
        for _ in 0..20_000 {
            let cell = rng.cell(&l);
            let mix = rng.mix(&l);
            let cm = l.to_mask(&cell).unwrap();
            let mm = l.mix_mask(&mix).unwrap();
            assert_eq!(cm.hits(&mm), contains(&mix, &cell), "{cell:?} {mix:?}");
            // One bit per row, plus the thermometer's wider rank groups.
            assert_eq!(
                cm.popcount() as usize,
                ROW_COUNT + rank_radix - 1 - usize::from(cell[1])
            );
            for bit in u32::try_from(l.total_values()).unwrap()..ConditionMask::BITS {
                assert!(!cm.get(bit));
            }
        }
        // The thermometer: rank_g1 sets every rank bit, the widest group one.
        let offset = 4_u32;
        let mut cell = [0; ROW_COUNT];
        let g1 = l.to_mask(&cell).unwrap();
        for bit in offset..offset + 5 {
            assert!(g1.get(bit));
        }
        cell[1] = 4;
        let widest = l.to_mask(&cell).unwrap();
        assert_eq!((offset..offset + 5).filter(|b| widest.get(*b)).count(), 1);
        assert!(widest.get(offset + 4));
        // The open mix requires nothing but its partitions.
        let free = l.mix_mask(&MixKey::root_of(&cell)).unwrap();
        assert_eq!(free.popcount(), 3);
        // Every offered position, set: the identity's mask term.
        let offered = l.offered_mask().unwrap();
        assert_eq!(offered.popcount() as usize, l.total_values());
        assert!(offered.hits(&widest));
    }

    #[test]
    fn to_mask_is_anti_monotone_adding_a_required_value_only_removes_containment() {
        let l = layout();
        let mut rng = Seeded(0xA11CE);
        for _ in 0..5_000 {
            let cell = rng.cell(&l);
            let mut mix = MixKey::root_of(&rng.cell(&l));
            let cm = l.to_mask(&cell).unwrap();
            let mut was = cm.hits(&l.mix_mask(&mix).unwrap());
            for index in 0..ROW_COUNT {
                if mix.0[index] != OPEN {
                    continue;
                }
                mix.0[index] = rng.below(l.radices()[index]);
                let now = cm.hits(&l.mix_mask(&mix).unwrap());
                assert!(
                    !now || was,
                    "a required value added containment at row {index}"
                );
                assert_eq!(now, contains(&mix, &cell));
                was = now;
            }
        }
    }

    /// Every generated row at `radix` values, the partitions as given.
    fn wide(radix: usize, exits: u16, styles: usize, widths: usize) -> GenerationParts {
        let cuts = radix - 1;
        let mut p = parts();
        p.timeframes = (1..=u16::try_from(widths).unwrap()).collect();
        p.rank_edges = (1..=u16::try_from(cuts).unwrap()).collect();
        p.cuts = RowId::ALL
            .iter()
            .filter(|id| id.row().source == Source::Measured && **id != RowId::MoverRank)
            .map(|id| CutList {
                row: *id,
                timeframe: (*id == RowId::TimeOfDay).then_some(1),
                base_kind: None,
                cuts: (1..=u64::try_from(cuts).unwrap())
                    .map(|n| Ratio::whole(n * 60))
                    .collect(),
            })
            .collect();
        p.exit_set.count = exits;
        p.entry_variants.styles = vec![0; styles];
        p
    }

    #[test]
    fn each_overflow_is_refused_by_name() {
        // A row beyond the value type: 254 values pass, 255 are refused.
        let edge = Generation::new(wide(MAX_ROW_VALUES, 1, 1, 1)).unwrap();
        assert_eq!(usize::from(edge.layout().radix(RowId::MoveSoFar)), 254);
        let mut p = wide(MAX_ROW_VALUES, 1, 1, 1);
        p.cuts[0].cuts.push(Ratio::whole(u64::MAX));
        assert_eq!(
            Generation::new(p),
            Err(CubeRefusal::RowValuesExceedU8 {
                row: "move_so_far",
                values: 255
            })
        );
        let mut p = wide(2, 1, 1, 1);
        p.rank_edges = vec![1; MAX_ROW_VALUES];
        assert_eq!(
            Generation::new(p).unwrap_err(),
            CubeRefusal::RowValuesExceedU8 {
                row: "mover_rank",
                values: 255
            }
        );
        let mut p = wide(2, 255, 1, 1);
        assert_eq!(
            Generation::new(p.clone()).unwrap_err().name(),
            "row_values_exceed_u8"
        );
        p.exit_set.count = 1;
        p.entry_variants.styles = vec![0; 255];
        assert_eq!(
            Generation::new(p).unwrap_err(),
            CubeRefusal::RowValuesExceedU8 {
                row: "entry_style",
                values: 255
            }
        );
        // A radix product beyond the key type: thirteen measured rows at 254
        // values and two partitions at 254 pass 2^128.
        assert_eq!(
            Generation::new(wide(MAX_ROW_VALUES, 254, 254, 1)),
            Err(CubeRefusal::RadixProductExceedsKey)
        );
        // The full product fits u128 while the partial product does not.
        let tight = Generation::new(wide(MAX_ROW_VALUES, 40, 1, 1)).unwrap();
        let l = tight.layout();
        assert_eq!(l.key_width(), KeyWidth::Bits128);
        assert!(l.paper_full_cells() > u128::from(u64::MAX));
        assert_eq!(
            l.paper_partial_mixes(),
            Err(CubeRefusal::PaperCountExceedsU128)
        );
        let (full, partial) = formulas(l.radices());
        assert_eq!(full, l.paper_full_cells());
        assert_eq!(partial, None);
        // More values than mask positions: the cell still encodes, the mask
        // refuses.
        let wide_cell = [0; ROW_COUNT];
        assert!(l.encode(&wide_cell).is_ok());
        let refused = CubeRefusal::FibMaskExceedsWords {
            values: l.total_values(),
            bits: ConditionMask::BITS,
        };
        assert_eq!(l.to_mask(&wide_cell), Err(refused));
        assert_eq!(l.mix_mask(&MixKey::root_of(&wide_cell)), Err(refused));
        assert_eq!(l.offered_mask(), Err(refused));
        assert_eq!(refused.name(), "fib_mask_exceeds_words");
        // Exactly the mask's width is maskable; one value more is not.
        // Twenty-four rows of sixteen values are 384.
        let mut full = [16; ROW_COUNT];
        let exact = Layout::of(full).unwrap();
        assert_eq!(exact.total_values(), 384);
        assert!(exact.to_mask(&[0; ROW_COUNT]).is_ok());
        assert_eq!(exact.offered_mask().unwrap().popcount(), 384);
        full[0] += 1;
        let over = Layout::of(full).unwrap();
        assert_eq!(
            over.to_mask(&[0; ROW_COUNT]).unwrap_err().name(),
            "fib_mask_exceeds_words"
        );
    }

    #[test]
    fn the_key_width_is_chosen_from_the_descriptor() {
        assert_eq!(layout().key_width(), KeyWidth::Bits64);
        assert_eq!(KeyWidth::Bits64.bits(), 64);
        assert_eq!(KeyWidth::Bits128.bits(), 128);
        let mut sixteen = [1; ROW_COUNT];
        for slot in sixteen.iter_mut().take(16) {
            *slot = 16;
        }
        let at_64 = Layout::of(sixteen).unwrap();
        assert_eq!(at_64.paper_full_cells(), 1_u128 << 64);
        assert_eq!(at_64.key_width(), KeyWidth::Bits64, "2^64 codes fit u64");
        let mut r = *at_64.radices();
        r[16] = 2;
        assert_eq!(Layout::of(r).unwrap().key_width(), KeyWidth::Bits128);
    }

    /// A CODE PAST 2^64 DECODES THROUGH `u128` AND ONE BELOW IT THROUGH `u64`,
    /// to the same digits either way: the codes either side of 2^64 and a
    /// thousand spread above it round-trip, and 2^64 itself is a one in row
    /// 0's place.
    #[test]
    fn codes_either_side_of_two_to_the_sixty_four_decode_exactly() {
        let mut radices = [1; ROW_COUNT];
        for slot in radices.iter_mut().take(17) {
            *slot = 16;
        }
        let l = Layout::of(radices).unwrap();
        let full = l.paper_full_cells();
        assert_eq!(full, 1_u128 << 68);
        let edge = u128::from(u64::MAX);
        let mut below = [0; ROW_COUNT];
        below[1..17].fill(15);
        assert_eq!(l.decode(edge).unwrap(), below);
        let mut past = [0; ROW_COUNT];
        past[0] = 1;
        assert_eq!(l.decode(edge + 1).unwrap(), past);
        let mut last = [0; ROW_COUNT];
        last[..17].fill(15);
        assert_eq!(l.decode(full - 1).unwrap(), last);
        let spread =
            (0..1_000_u128).map(|k| edge + 1 + (k * 0x9E37_79B9_7F4A_7C15) % (full - edge - 1));
        for code in [0, edge - 1, edge, edge + 1, edge + 2, full - 1]
            .into_iter()
            .chain(spread)
        {
            let cell = l.decode(code).unwrap();
            assert_eq!(l.encode(&cell).unwrap(), code, "{code}");
        }
    }

    #[test]
    fn tercile_is_decode_only_and_an_edge_goes_up() {
        assert_eq!(tercile(0), Ok(0));
        assert_eq!(tercile(333), Ok(0));
        assert_eq!(tercile(334), Ok(1));
        assert_eq!(tercile(666), Ok(1));
        assert_eq!(tercile(667), Ok(2));
        assert_eq!(tercile(1_000), Ok(2));
        assert_eq!(
            tercile(1_001),
            Err(CubeRefusal::PermilleOutOfRange { permille: 1_001 })
        );
        let mut counts = [0_u32; 3];
        for p in 0..=1_000 {
            counts[usize::from(tercile(p).unwrap())] += 1;
        }
        assert_eq!(counts, [334, 333, 334]);
        assert_eq!(V1_SEED_BUCKETS, 3);
    }

    #[test]
    fn rank_bucket_on_synthetic_generations_of_one_two_and_many_edges() {
        // No inner edge: one group, every ranked decision.
        assert_eq!(rank_bucket(0, &[]), None, "unranked");
        assert_eq!(rank_bucket(1, &[]), Some(0));
        assert_eq!(rank_bucket(u16::MAX, &[]), Some(0));
        // One edge (the seed's median cut): two groups.
        assert_eq!(rank_bucket(7, &[7]), Some(0), "on the edge");
        assert_eq!(rank_bucket(8, &[7]), Some(1));
        // Many edges, and FM-F005: beyond the widest inner edge is the widest
        // group, never None.
        let edges = [3, 8, 20];
        assert_eq!(rank_bucket(57, &edges), Some(3), "rank_g4, not None");
        for (group, edge) in (0_u8..).zip(edges) {
            assert_eq!(rank_bucket(edge, &edges), Some(group));
            assert_eq!(rank_bucket(edge + 1, &edges), Some(group + 1));
        }
        assert_eq!(rank_bucket(1, &edges), Some(0));
        // The generation's own edges, with a repeat kept.
        let g = generation();
        assert_eq!(g.parts().rank_edges, [3, 8, 8, 20]);
        assert_eq!(g.rank_group(0), None);
        assert_eq!(
            g.rank_group(8),
            Some(1),
            "the narrowest of the equal groups"
        );
        assert_eq!(g.rank_group(9), Some(3));
        assert_eq!(g.rank_group(21), Some(4));
        // A later, larger pool refuses no ranked decision.
        for rank in 1..=u16::MAX {
            assert!(g.rank_group(rank).is_some(), "{rank}");
        }
    }

    #[test]
    fn duplicate_rank_edges_are_kept_counted_and_golden() {
        let edges = [5, 5, 9, 9, 9];
        assert!(!rank_group_is_equivalent(&edges, 0));
        assert!(rank_group_is_equivalent(&edges, 1));
        assert!(!rank_group_is_equivalent(&edges, 2));
        assert!(rank_group_is_equivalent(&edges, 3));
        assert!(rank_group_is_equivalent(&edges, 4));
        assert!(!rank_group_is_equivalent(&edges, 5), "the widest group");
        assert!(!rank_group_is_equivalent(&edges, OPEN));
        assert_eq!(rank_bucket(5, &edges), Some(0));
        assert_eq!(rank_bucket(6, &edges), Some(2));
        assert_eq!(rank_bucket(10, &edges), Some(5));
        let mut p = parts();
        p.rank_edges = edges.to_vec();
        let g = Generation::new(p).unwrap();
        assert_eq!(g.equivalent_rank_groups(), 3);
        assert_eq!(generation().equivalent_rank_groups(), 1);
        assert_eq!(g.layout().radix(RowId::MoverRank), 6, "kept, not merged");
        let bytes = g.bytes();
        let at = bytes
            .windows(4)
            .position(|w| w == 5_u32.to_le_bytes())
            .unwrap();
        assert_eq!(
            hex(&bytes[at..at + 14]),
            "0500000005000500090009000900",
            "the repeated edges are recorded as given"
        );
    }

    #[test]
    fn lower_nearest_rank_takes_the_lower_middle() {
        assert_eq!(lower_nearest_rank(&[1, 2, 3, 4], 1, 2), Ok(2));
        assert_eq!(lower_nearest_rank(&[1, 2, 3], 1, 2), Ok(2));
        assert_eq!(lower_nearest_rank(&[1, 2, 3, 4], 1, 1), Ok(4));
        assert_eq!(lower_nearest_rank(&[1, 2, 3, 4], 1, 1_000), Ok(1));
        assert_eq!(lower_nearest_rank(&[9], 1, 2), Ok(9));
        assert_eq!(lower_nearest_rank(&[3, 8, 20, 41], 3, 4), Ok(20));
        assert_eq!(
            lower_nearest_rank::<u16>(&[], 1, 2),
            Err(CubeRefusal::EmptySample)
        );
        assert_eq!(
            lower_nearest_rank(&[1], 0, 2),
            Err(CubeRefusal::QuantileInvalid { num: 0, den: 2 })
        );
        assert_eq!(
            lower_nearest_rank(&[1], 3, 2).unwrap_err().name(),
            "quantile_invalid"
        );
        assert_eq!(
            lower_nearest_rank(&[1], 1, 0).unwrap_err().name(),
            "quantile_invalid"
        );
    }

    #[test]
    fn a_cut_list_buckets_exactly_and_a_value_on_a_cut_goes_up() {
        let list = CutList {
            row: RowId::StopSize,
            timeframe: None,
            base_kind: None,
            cuts: vec![Ratio::new(1, 3).unwrap(), half(), Ratio::new(2, 3).unwrap()],
        };
        assert_eq!(list.bucket(Ratio::whole(0)), 0);
        assert_eq!(list.bucket(Ratio::new(333, 1_000).unwrap()), 0);
        assert_eq!(
            list.bucket(Ratio::new(1, 3).unwrap()),
            1,
            "on a cut goes up"
        );
        assert_eq!(list.bucket(Ratio::new(2, 6).unwrap()), 1, "the same value");
        assert_eq!(list.bucket(Ratio::new(334, 1_000).unwrap()), 1);
        assert_eq!(list.bucket(half()), 2);
        assert_eq!(list.bucket(Ratio::new(2, 3).unwrap()), 3);
        assert_eq!(list.bucket(Ratio::whole(1)), 3);
        assert_eq!(list.bucket(Ratio::new(u64::MAX, 1).unwrap()), 3);
        // The extremes of u64 compare exactly in u128.
        let tiny = Ratio::new(1, u64::MAX).unwrap();
        let huge = Ratio::new(u64::MAX, 1).unwrap();
        assert_eq!(tiny.cmp_exact(huge), Ordering::Less);
        assert_eq!(huge.cmp_exact(tiny), Ordering::Greater);
        assert_eq!(
            Ratio::new(u64::MAX, u64::MAX)
                .unwrap()
                .cmp_exact(Ratio::whole(1)),
            Ordering::Equal
        );
        assert_eq!(half().num(), 1);
        assert_eq!(half().den(), 2);
        // The generation finds a list by its exact scope and no other.
        let g = generation();
        let tight = RowId::BaseTightness;
        assert_eq!(g.cuts(tight, None, Some(0)).unwrap().cuts.len(), 2);
        assert_eq!(g.cuts(tight, None, Some(2)).unwrap().cuts.len(), 1);
        assert_eq!(g.cuts(tight, None, Some(1)), None, "no wider scope tried");
        assert_eq!(g.cuts(tight, None, None), None);
        let time = g.cuts(RowId::TimeOfDay, Some(4), None).unwrap();
        assert_eq!(time.bucket(Ratio::whole(4_799)), 0);
        assert_eq!(time.bucket(Ratio::whole(4_800)), 1);
        assert_eq!(time.bucket(Ratio::whole(9_600)), 2);
        assert_eq!(g.cuts(RowId::TimeOfDay, Some(3), None), None);
        assert_eq!(g.cuts(RowId::FibZone, None, None), None);
        // A list with more cuts than a u8 indexes answers OPEN, which every
        // layout refuses.
        let overfull = CutList {
            row: RowId::StopSize,
            timeframe: None,
            base_kind: None,
            cuts: (0..300).map(Ratio::whole).collect(),
        };
        assert_eq!(overfull.bucket(Ratio::whole(299)), OPEN);
    }

    #[test]
    fn a_ratio_and_a_mid_rank_refuse_by_name() {
        assert_eq!(
            Ratio::new(3, 0),
            Err(CubeRefusal::RatioZeroDenominator { num: 3 })
        );
        let r = Ratio::mid_rank(2, 1, 5).unwrap();
        assert_eq!((r.num(), r.den()), (5, 10));
        assert_eq!(Ratio::mid_rank(0, 5, 5).unwrap().num(), 5);
        assert_eq!(Ratio::mid_rank(5, 0, 5).unwrap().num(), 10);
        for (below, in_bin, total) in [
            (0, 0, 0),
            (3, 3, 5),
            (u64::MAX, 1, u64::MAX),
            (0, 1, u64::MAX),
            (u64::MAX / 2 + 1, 0, u64::MAX / 2 + 1),
        ] {
            assert_eq!(
                Ratio::mid_rank(below, in_bin, total),
                Err(CubeRefusal::MidRankInvalid {
                    below,
                    in_bin,
                    total
                }),
                "{below} {in_bin} {total}"
            );
        }
        let edge = u64::MAX / 2;
        assert_eq!(Ratio::mid_rank(edge, 0, edge).unwrap().den(), edge * 2);
    }

    #[test]
    fn a_generation_refuses_every_malformed_part_by_name() {
        let check = |edit: &dyn Fn(&mut GenerationParts), want: CubeRefusal| {
            let mut p = parts();
            edit(&mut p);
            assert_eq!(Generation::new(p), Err(want), "{}", want.name());
        };
        check(&|p| p.timeframes.clear(), CubeRefusal::TimeframesEmpty);
        check(
            &|p| p.timeframes = vec![0, 2],
            CubeRefusal::TimeframesInvalid { at: 0, minutes: 0 },
        );
        check(
            &|p| p.timeframes = vec![1, 2, 119],
            CubeRefusal::TimeframesInvalid {
                at: 2,
                minutes: 119,
            },
        );
        check(
            &|p| p.timeframes = vec![1, 4, 2],
            CubeRefusal::TimeframesInvalid { at: 2, minutes: 2 },
        );
        check(
            &|p| p.timeframes = vec![1, 1],
            CubeRefusal::TimeframesInvalid { at: 1, minutes: 1 },
        );
        check(
            &|p| p.rank_edges = vec![0, 3],
            CubeRefusal::RankEdgesInvalid { at: 0, edge: 0 },
        );
        check(
            &|p| p.rank_edges = vec![3, 9, 8],
            CubeRefusal::RankEdgesInvalid { at: 2, edge: 8 },
        );
        check(
            &|p| p.cuts.insert(0, seed_list(RowId::Side)),
            CubeRefusal::CutOnUnmeasuredRow { row: "side" },
        );
        check(
            &|p| p.cuts.insert(0, seed_list(RowId::MoverRank)),
            CubeRefusal::CutOnUnmeasuredRow { row: "mover_rank" },
        );
        check(
            &|p| p.cuts[0].timeframe = Some(3),
            CubeRefusal::CutTimeframeUnknown {
                row: "move_so_far",
                minutes: 3,
            },
        );
        check(
            &|p| p.cuts[0].base_kind = Some(3),
            CubeRefusal::CutBaseKindOutOfRange {
                row: "move_so_far",
                base_kind: 3,
            },
        );
        check(
            &|p| p.cuts[0].cuts = vec![half(), Ratio::new(2, 4).unwrap()],
            CubeRefusal::CutsNotIncreasing {
                row: "move_so_far",
                at: 1,
            },
        );
        check(
            &|p| p.cuts[0].cuts = vec![Ratio::whole(2), half()],
            CubeRefusal::CutsNotIncreasing {
                row: "move_so_far",
                at: 1,
            },
        );
        let time_at = parts()
            .cuts
            .iter()
            .position(|l| l.row == RowId::TimeOfDay)
            .unwrap();
        check(
            &|p| p.cuts[time_at].timeframe = None,
            CubeRefusal::TimeCutsWithoutTimeframe,
        );
        for bad in [
            Ratio::new(5_400, 2).unwrap(),
            Ratio::whole(0),
            Ratio::whole(5_430),
            Ratio::whole(21_240),
        ] {
            check(
                &|p| p.cuts[time_at].cuts = vec![bad],
                CubeRefusal::TimeCutOffGrid { minutes: 1, at: 0 },
            );
        }
        check(
            &|p| {
                let first = p.cuts[0].clone();
                p.cuts.insert(1, first);
            },
            CubeRefusal::CutListsNotCanonical { row: "move_so_far" },
        );
        check(
            &|p| p.cuts.swap(0, 1),
            CubeRefusal::CutListsNotCanonical { row: "move_so_far" },
        );
        check(
            &|p| p.cuts.retain(|l| l.row != RowId::Attempt),
            CubeRefusal::MeasuredRowWithoutCuts { row: "attempt" },
        );
        check(&|p| p.exit_set.count = 0, CubeRefusal::ExitSetEmpty);
        check(
            &|p| p.entry_variants.styles.clear(),
            CubeRefusal::EntryVariantsEmpty,
        );
        check(
            &|p| p.entry_variants.styles = vec![0, 3],
            CubeRefusal::EntryStyleUnknown { style: 3 },
        );
        check(
            &|p| {
                let first = p.others[0];
                p.others.push(first);
            },
            CubeRefusal::GeneratedListsNotCanonical { at: 1 },
        );
        check(
            &|p| p.others[0].birth_step = 7,
            CubeRefusal::ListBornAfterGeneration { at: 0 },
        );
        // Accepted shapes: repeated rank edges, no rank edge, the last second
        // of the decision span's grid, one width at the hard top, two other
        // packages' lists in name order.
        let mut p = parts();
        p.rank_edges.clear();
        p.timeframes = vec![1, 2, 4, 118];
        p.cuts[time_at].cuts = vec![Ratio::whole(21_180)];
        let first = p.others[0];
        p.others.push(GeneratedList {
            name: "touch_levels_z",
            ..first
        });
        let g = Generation::new(p).unwrap();
        assert_eq!(g.layout().radix(RowId::MoverRank), 1);
        assert_eq!(g.layout().radix(RowId::Timeframe), 4);
    }

    #[test]
    fn a_generation_reports_what_it_holds() {
        let g = generation();
        assert_eq!(g.parts(), &parts());
        assert!(!g.usable_at(5), "born at step 6");
        assert!(g.usable_at(6));
        assert!(g.usable_at(u32::MAX));
    }

    #[test]
    fn a_chain_reads_only_versions_born_at_or_before_the_step() {
        let first = generation();
        let mut chain = GenerationChain::new(first.clone());
        assert_eq!(chain.at(5), None, "before the first version");
        assert_eq!(chain.at(6), Some(&first));
        assert_eq!(chain.latest(), &first);
        let mut p = parts();
        p.version = 2;
        p.birth_step = 9;
        p.rank_edges = vec![4, 10];
        let second = Generation::new(p.clone()).unwrap();
        chain.push(second.clone()).unwrap();
        p.version = 3;
        p.birth_step = 12;
        let third = Generation::new(p.clone()).unwrap();
        chain.push(third.clone()).unwrap();
        assert_eq!(chain.at(8), Some(&first));
        assert_eq!(chain.at(9), Some(&second));
        assert_eq!(chain.at(11), Some(&second));
        assert_eq!(chain.at(12), Some(&third));
        assert_eq!(chain.at(u32::MAX), Some(&third));
        assert_eq!(chain.latest(), &third);
        assert_eq!(chain.versions().count(), 3);
        assert_eq!(chain.search(), Search::Long);
        let mut wrong = p.clone();
        wrong.version = 4;
        wrong.birth_step = 13;
        wrong.search = Search::Short;
        assert_eq!(
            chain.push(Generation::new(wrong).unwrap()),
            Err(CubeRefusal::ChainSearchMismatch)
        );
        let mut skip = p.clone();
        skip.version = 5;
        skip.birth_step = 13;
        assert_eq!(
            chain.push(Generation::new(skip).unwrap()),
            Err(CubeRefusal::ChainVersionNotNext {
                expected: 4,
                got: 5
            })
        );
        let mut same_step = p;
        same_step.version = 4;
        assert_eq!(
            chain.push(Generation::new(same_step).unwrap()),
            Err(CubeRefusal::ChainBirthNotLater {
                previous: 12,
                got: 12
            })
        );
        assert_eq!(
            chain.versions().count(),
            3,
            "a refused push changes nothing"
        );
    }

    fn hex(bytes: &[u8]) -> String {
        bytes.iter().fold(String::new(), |mut out, b| {
            use std::fmt::Write as _;
            let _ = write!(out, "{b:02x}");
            out
        })
    }

    fn fnv1a(bytes: &[u8]) -> u64 {
        bytes.iter().fold(0xCBF2_9CE4_8422_2325, |h, b| {
            (h ^ u64::from(*b)).wrapping_mul(0x0100_0000_01B3)
        })
    }

    #[test]
    fn the_generation_bytes_are_stable_and_every_part_moves_them() {
        let g = generation();
        let bytes = g.bytes();
        assert_eq!(&bytes[..8], b"BTXFIBG1");
        assert_eq!(
            (bytes.len(), fnv1a(&bytes)),
            GENERATION_GOLDEN,
            "the canonical generation bytes changed: every FIB run identity re-keys"
        );
        let moves = |edit: &dyn Fn(&mut GenerationParts)| {
            let mut p = parts();
            edit(&mut p);
            assert_ne!(Generation::new(p).unwrap().bytes(), bytes);
        };
        moves(&|p| p.search = Search::Short);
        moves(&|p| p.version = 2);
        moves(&|p| p.birth_step = 7);
        moves(&|p| p.timeframes.push(8));
        moves(&|p| p.rank_edges[3] = 21);
        moves(&|p| p.cuts[0].cuts[0] = Ratio::new(1, 4).unwrap());
        moves(&|p| p.cuts[0].cuts[0] = Ratio::new(2, 4).unwrap());
        moves(&|p| p.exit_set.version = 8);
        moves(&|p| p.exit_set.digest[31] = 0);
        moves(&|p| p.exit_set.count = 5);
        moves(&|p| p.entry_variants.version = 3);
        moves(&|p| p.entry_variants.digest[0] = 0);
        moves(&|p| p.entry_variants.styles = vec![0, 1]);
        moves(&|p| p.others.clear());
        moves(&|p| p.others[0].digest[0] = 0);
        moves(&|p| p.others[0].version = 2);
        moves(&|p| p.others[0].birth_step = 5);
        moves(&|p| p.others[0].name = "touch_levels_b");
        // The scopes are framed: an absent scope and scope zero differ.
        moves(&|p| {
            let at = p.cuts.iter().position(|l| l.row == RowId::Attempt).unwrap();
            p.cuts[at].base_kind = Some(0);
        });
        // The per-run descriptor carries the static descriptor, the bytes and
        // the layout.
        let run = g.run_descriptor();
        assert_eq!(&run[..8], b"BTXFIBR1");
        let d = descriptor();
        assert_eq!(&run[12..12 + d.len()], d.as_slice());
        assert!(run.windows(bytes.len()).any(|w| w == bytes.as_slice()));
        assert!(run.ends_with(g.layout().radices()));
        assert_eq!(
            (run.len(), fnv1a(&run)),
            RUN_DESCRIPTOR_GOLDEN,
            "the per-run descriptor changed"
        );
    }

    const GENERATION_GOLDEN: (usize, u64) = (835, 3_930_354_247_708_419_042);
    const RUN_DESCRIPTOR_GOLDEN: (usize, u64) = (2_579, 16_283_875_604_159_367_194);

    #[test]
    fn the_descriptor_hashes_rows_kinds_and_definitional_lists_only() {
        let d = descriptor();
        assert_eq!(&d[..8], b"BTXFIBC1");
        assert_eq!(&d[8..12], &1_u32.to_le_bytes());
        assert_eq!(&d[12..16], &24_u32.to_le_bytes());
        assert_eq!(&d[16..20], &4_u32.to_le_bytes());
        assert_eq!(&d[20..24], b"side");
        let text = String::from_utf8_lossy(&d);
        for row in &ROWS {
            assert!(text.contains(row.name));
            assert!(text.contains(row.kind.name()));
            assert!(text.contains(row.source.name()));
            if row.source != Source::Measured {
                for label in row.labels {
                    assert!(text.contains(label), "{label}");
                }
            }
        }
        // Seed labels are never hashed: renaming one is not a schema change.
        for seed in [
            "rank_g1",
            "rank_g2",
            "second_plus",
            "near_extreme",
            "middle",
            "wide",
        ] {
            assert!(!text.contains(seed), "{seed}");
        }
        assert_eq!(d.last(), Some(&OPEN));
        assert_eq!((d.len(), fnv1a(&d)), DESCRIPTOR_GOLDEN);
    }

    /// The descriptor's length and FNV-1a 64 checksum (this crate cannot hash
    /// with BLAKE3; `runner::fib::KNOWN_SCHEMAS` pins that digest).
    const DESCRIPTOR_GOLDEN: (usize, u64) = (1_696, 3_661_760_530_698_320_215);

    #[test]
    fn three_synthetic_triggers_read_no_record_unclassified_and_balanced() {
        let read = |w| OrderFlowReading::of(w, Search::Long);
        assert_eq!(
            read(OrderFlowWindow::NoRecord),
            OrderFlowReading::Absent(AbsentReason::NoRecord)
        );
        assert_eq!(read(OrderFlowWindow::NoRecord).name(), "absent:no_record");
        let unclassified = OrderFlowWindow::Covered {
            buy: 0,
            sell: 0,
            unclassified: 40,
        };
        assert_eq!(read(unclassified), OrderFlowReading::Unclassified);
        let balanced = OrderFlowWindow::Covered {
            buy: 25,
            sell: 25,
            unclassified: 9,
        };
        assert_eq!(read(balanced), OrderFlowReading::Balanced);
        // The other absences, each by name.
        for (window, name) in [
            (OrderFlowWindow::Off, "absent:off"),
            (OrderFlowWindow::Partial, "absent:partial"),
            (
                OrderFlowWindow::Covered {
                    buy: 0,
                    sell: 0,
                    unclassified: 0,
                },
                "absent:empty",
            ),
        ] {
            assert_eq!(read(window).value(), OrderFlowValue::Absent);
            assert_eq!(read(window).name(), name);
        }
        // With and against, by the trade's direction.
        let buying = OrderFlowWindow::Covered {
            buy: u64::MAX,
            sell: 0,
            unclassified: 0,
        };
        let selling = OrderFlowWindow::Covered {
            buy: 1,
            sell: u64::MAX,
            unclassified: u64::MAX,
        };
        assert_eq!(
            OrderFlowReading::of(buying, Search::Long),
            OrderFlowReading::With
        );
        assert_eq!(
            OrderFlowReading::of(buying, Search::Short),
            OrderFlowReading::Against
        );
        assert_eq!(
            OrderFlowReading::of(selling, Search::Short),
            OrderFlowReading::With
        );
        assert_eq!(
            OrderFlowReading::of(selling, Search::Long),
            OrderFlowReading::Against
        );
        for (reading, name) in [
            (OrderFlowReading::With, "with"),
            (OrderFlowReading::Against, "against"),
            (OrderFlowReading::Balanced, "balanced"),
            (OrderFlowReading::Unclassified, "unclassified"),
        ] {
            assert_eq!(reading.name(), name);
            assert_eq!(
                RowId::TriggerOrderFlow.value_of(name),
                Some(reading.value().index())
            );
        }
        // No balanced mix holds an absent row.
        let l = layout();
        let row = RowId::TriggerOrderFlow.index();
        let mut cell = [0; ROW_COUNT];
        cell[row] = read(OrderFlowWindow::NoRecord).value().index();
        let mut mix = MixKey::root_of(&cell);
        mix.0[row] = read(balanced).value().index();
        assert_eq!(l.validate(&cell), Ok(()));
        assert_eq!(l.validate_mix(&mix), Ok(()));
        assert!(!contains(&mix, &cell));
        assert!(!l.to_mask(&cell).unwrap().hits(&l.mix_mask(&mix).unwrap()));
    }

    #[test]
    fn the_order_flow_values_are_the_row_labels() {
        let values = [
            OrderFlowValue::With,
            OrderFlowValue::Against,
            OrderFlowValue::Balanced,
            OrderFlowValue::Unclassified,
            OrderFlowValue::Absent,
        ];
        for (value, label) in values.iter().zip(TRIGGER_ORDER_FLOW_ROW.labels) {
            assert_eq!(RowId::TriggerOrderFlow.value_of(label), Some(value.index()));
        }
        assert_eq!(values.len(), TRIGGER_ORDER_FLOW_ROW.labels.len());
    }

    #[test]
    fn min_support_is_the_pro_rated_trade_floor_and_its_two_phases() {
        let year = MinSupport::new(248, 248).unwrap();
        assert_eq!(year.q().cmp_exact(Ratio::whole(500)), Ordering::Equal);
        assert_eq!(year.q_ceil(), 500);
        assert_eq!(year.phase1_floor(), 250);
        assert_eq!(year.phase2_floor(320), 180);
        assert_eq!(year.phase2_floor(500), 1, "never below one");
        assert_eq!(year.phase2_floor(u64::MAX), 1);
        // Two quarters of a 248-day year: Q = 500 x 124 / 248 = 250 exactly.
        let half_year = MinSupport::new(124, 248).unwrap();
        assert_eq!(half_year.q_ceil(), 250);
        assert_eq!(half_year.phase1_floor(), 125);
        // A window that does not divide: Q = 500 x 61 / 247 = 123.48...
        let odd = MinSupport::new(61, 247).unwrap();
        assert_eq!(odd.q_ceil(), 124);
        assert_eq!(odd.phase1_floor(), 62, "ceil(61.74)");
        assert_eq!(odd.phase2_floor(100), 24);
        assert_eq!(MinSupport::new(1, 0), Err(CubeRefusal::GateYearWithoutDays));
        let empty = MinSupport::new(0, 248).unwrap();
        assert_eq!(empty.q_ceil(), 0);
        assert_eq!(empty.phase1_floor(), 0);
        assert_eq!(empty.phase2_floor(0), 1);
        let huge = MinSupport::new(u64::MAX, 1).unwrap();
        assert_eq!(huge.q().num(), u64::MAX, "saturates, never wraps");
    }

    #[test]
    fn the_hard_top_width_is_derived_and_is_118_on_the_0915_grid() {
        assert_eq!(max_width_minutes(), 118);
        // The smallest base (two candles) plus a trigger at 118 minutes closes
        // exactly at 15:09:00; at 119 it would not.
        let close = |n: u64| SESSION_GRID_ANCHOR_SECS + (MICRO2_CANDLES + 1) * n * 60;
        assert_eq!(close(118), SESSION_FORCED_WINDOW_START_SECS);
        assert!(close(119) > SESSION_FORCED_WINDOW_START_SECS);
        assert_eq!(decision_span_secs(), 21_240);
    }

    #[test]
    fn a_search_admits_only_its_own_side_values() {
        assert_eq!(Search::of_side(0), Some(Search::Long));
        assert_eq!(Search::of_side(1), Some(Search::Short));
        assert_eq!(Search::of_side(2), Some(Search::Short));
        assert_eq!(Search::of_side(3), Some(Search::Long));
        assert_eq!(Search::of_side(4), None);
        assert_eq!(Search::of_side(OPEN), None);
        let mut cell = [0; ROW_COUNT];
        assert!(Search::Long.admits(&cell));
        assert!(!Search::Short.admits(&cell));
        cell[0] = 2;
        assert!(Search::Short.admits(&cell));
        let mut mix = MixKey::root_of(&cell);
        assert_eq!(
            Search::Long.admits_mix(&mix),
            Ok(()),
            "an open side is either side of the search"
        );
        mix.0[0] = 1;
        assert_eq!(
            Search::Long.admits_mix(&mix),
            Err(CubeRefusal::SideOutsideSearch {
                search: "long",
                side: "short_with_loser"
            })
        );
        assert_eq!(Search::Short.admits_mix(&mix), Ok(()));
        mix.0[0] = 3;
        assert_eq!(Search::Long.admits_mix(&mix), Ok(()));
        assert_eq!(Search::Long.name(), "long");
        assert_eq!(Search::Short.name(), "short");
    }

    #[test]
    fn fib_zone_at_every_rung_midpoint_and_tie() {
        let rungs = [0_i64, 236, 382, 500, 618, 786, 1_000];
        for (zone, rung) in (0_u8..).zip(rungs) {
            assert_eq!(fib_zone(rung), Ok(zone), "on rung {rung}");
        }
        for (zone, pair) in (0_u8..).zip(rungs.windows(2)) {
            let mid = pair[0].midpoint(pair[1]);
            assert_eq!((pair[0] + pair[1]) % 2, 0, "every midpoint is an integer");
            assert_eq!(fib_zone(mid), Ok(zone), "a tie at {mid} goes shallower");
            assert_eq!(fib_zone(mid + 1), Ok(zone + 1), "past {mid} goes deeper");
            assert_eq!(fib_zone(mid - 1), Ok(zone));
        }
        assert_eq!(fib_zone_of(1, 3), Ok(2));
        assert_eq!(fib_zone_of(309_000, 1_000_000), Ok(1));
        assert_eq!(fib_zone_of(309_001, 1_000_000), Ok(2));
        assert_eq!(fib_zone_of(i64::MAX, i64::MAX), Ok(6));
        assert_eq!(fib_zone_of(0, i64::MAX), Ok(0));
        assert_eq!(ZONE_RUNGS, [0, 236, 382, 500, 618, 786, 1_000]);
        assert_eq!(
            fib_zone(1_001),
            Err(CubeRefusal::RetraceOutOfRange {
                part: 1_001,
                whole: 1_000
            })
        );
        assert_eq!(fib_zone(-1).unwrap_err().name(), "retrace_out_of_range");
        assert_eq!(
            fib_zone_of(0, 0).unwrap_err().name(),
            "retrace_out_of_range"
        );
        assert_eq!(
            fib_zone_of(0, -5).unwrap_err().name(),
            "retrace_out_of_range"
        );
    }

    /// Every number the 17:15 IST sweep deleted (item a and item j), with the
    /// value it carried.
    const DELETED: &[(&str, u64)] = &[
        ("gate_stop_min_bp", 20),
        ("gate_trades_per_year_min_old", 100),
        ("session_own_exit_early_secs", 54_420),
        ("session_own_exit_late_secs", 54_480),
        ("report_giant_win_5r", 5),
        ("report_giant_win_10r", 10),
        ("report_giant_win_20r", 20),
        ("report_top_mfe_permille", 50),
        ("report_room_to_stop_reference_milli", 2_500),
        ("hist_top_power", 40),
        ("timeframe_m5_minutes", 5),
        ("timeframe_m15_minutes", 15),
        ("days_per_fortnight", 14),
    ];

    #[test]
    fn fixed_numbers_names_every_literal_of_this_file_and_refuses_every_deleted_one() {
        let source = include_str!("fib_cube.rs");
        let shipping = shipping_part(source);
        assert!(
            shipping.len() < source.len(),
            "the test module marker was found"
        );
        let literals = numeric_literals(shipping);
        assert!(
            literals.len() > 40,
            "the scan saw the ledger: {}",
            literals.len()
        );
        for literal in &literals {
            let value = literal.value.unwrap_or_else(|| {
                panic!(
                    "line {}: `{}` is not a plain decimal literal",
                    literal.line, literal.text
                )
            });
            assert!(
                value <= 1 || is_fixed_number(value),
                "line {}: `{}` is typed but is not in FIXED_NUMBERS",
                literal.line,
                literal.text
            );
        }
        // Item a: every deleted number is out of the ledger, by name and by
        // value, so typing it again fails the scan above.
        for (name, value) in DELETED {
            assert!(!is_fixed_number(*value), "{name} = {value} is back");
            assert!(
                FIXED_NUMBERS.iter().all(|(n, _, _, _)| n != name),
                "{name} is back"
            );
        }
        assert!(
            FIXED_NUMBERS
                .iter()
                .all(|(n, _, _, _)| !n.contains("stop_min") && !n.starts_with("values:")),
            "no minimum stop (13:07 IST) and no coarse bucket count"
        );
        // Unique names, and a decision id inside the FIB plan's blocks
        // (numbers 6300 to 6579 after FM-F008's move, D-6311).
        let mut names: Vec<&str> = FIXED_NUMBERS.iter().map(|(n, _, _, _)| *n).collect();
        names.sort_unstable();
        let before = names.len();
        names.dedup();
        assert_eq!(names.len(), before, "a ledger name is repeated");
        let in_block = |decision: &str| {
            let n: u32 = decision.strip_prefix("D-").unwrap().parse().unwrap();
            (6300..=6579).contains(&n)
        };
        for (name, _, _, decision) in FIXED_NUMBERS {
            assert!(in_block(decision), "{name}: {decision}");
        }
        for (name, decision) in GATE_CLAUSES {
            assert!(in_block(decision), "{name}: {decision}");
        }
        let gate: Vec<(&str, u64)> = FIXED_NUMBERS
            .iter()
            .filter(|(_, _, c, _)| *c == Category::WinnerGate)
            .map(|(n, v, _, _)| (*n, *v))
            .collect();
        assert_eq!(
            gate,
            [
                ("gate_win_rate_min_permille", 510),
                ("gate_average_win_ratio_min", 2),
                ("gate_profit_factor_min", 2),
                ("gate_largest_win_ratio_min", 2),
                ("gate_trades_per_year_min", 500),
                ("gate_unseen_months_positive_min_permille", 700),
                ("gate_every_year_net_above_paisa", 0),
                ("gate_each_side_unseen_net_above_paisa", 0),
            ]
        );
        assert!(is_fixed_number(54_600));
        assert!(!is_fixed_number(54_601));
    }

    #[test]
    fn every_category_has_a_name_and_a_ledger_row() {
        let all = [
            Category::WinnerGate,
            Category::OperatorSessionTime,
            Category::TradeSize,
            Category::OperatorPatternSize,
            Category::FibonacciRatio,
            Category::Unit,
            Category::Representation,
        ];
        let names: Vec<&str> = all.iter().map(|c| c.name()).collect();
        assert_eq!(
            names,
            [
                "winner_gate",
                "operator_session_time",
                "trade_size",
                "operator_pattern_size",
                "fibonacci_ratio",
                "unit",
                "representation",
            ]
        );
        for category in all {
            assert!(
                FIXED_NUMBERS.iter().any(|(_, _, c, _)| *c == category),
                "{category:?}"
            );
        }
        assert_eq!(SESSION_GRID_ANCHOR_SECS, 9 * 3_600 + 15 * 60);
        assert_eq!(SESSION_FORCED_WINDOW_START_SECS, 15 * 3_600 + 9 * 60);
        assert_eq!(SESSION_FORCED_WINDOW_END_SECS, 15 * 3_600 + 10 * 60);
        assert_eq!(SESSION_AUTO_EXIT_BY_SECS, SESSION_FORCED_WINDOW_END_SECS);
        assert_eq!(SECONDS_PER_DAY, 24 * 3_600);
    }

    #[test]
    fn the_fixed_numbers_bytes_cover_every_row_and_clause() {
        let bytes = fixed_numbers_bytes();
        assert_eq!(&bytes[..8], b"BTXFIBN1");
        let text = String::from_utf8_lossy(&bytes);
        for (name, value, category, decision) in FIXED_NUMBERS {
            assert!(text.contains(name));
            assert!(text.contains(category.name()));
            assert!(text.contains(decision));
            assert!(bytes.windows(8).any(|w| w == value.to_le_bytes()));
        }
        for (name, _) in GATE_CLAUSES {
            assert!(text.contains(name));
        }
        assert_eq!(
            (bytes.len(), fnv1a(&bytes)),
            FIXED_NUMBERS_GOLDEN,
            "the ledger changed: every pair identity re-keys"
        );
    }

    const FIXED_NUMBERS_GOLDEN: (usize, u64) = (2_668, 2_514_577_390_907_684_871);

    #[test]
    fn the_literal_scanner_skips_comments_strings_chars_and_fields() {
        let src = "let a = 12; // 99\nlet b = \"7 8\"; /* 5 /* 6 */ 4 */ let c = 'x';\n\
                   let d = b'9'; let e = '\\''; let f = 3_000u64; let g = t.0 + t.1;\n\
                   let h = 1.5; let i = 0xFF; let j = 1e3; let k = 2..10; let l = r#\"77\"#;\n\
                   fn m<'a>(x: &'a u8) -> u8 { 33 } let n = br\"88\"; let o = r\"66\"; let p = '\\n';";
        let literals = numeric_literals(src);
        let found: Vec<(usize, &str, Option<u64>)> = literals
            .iter()
            .map(|l| (l.line, l.text.as_str(), l.value))
            .collect();
        assert_eq!(
            found,
            [
                (1, "12", Some(12)),
                (3, "3_000u64", Some(3_000)),
                (4, "1", None),
                (4, "0xFF", None),
                (4, "1e3", None),
                (4, "2", Some(2)),
                (4, "10", Some(10)),
                (5, "33", Some(33)),
            ]
        );
        assert_eq!(decimal_value("7usize"), Some(7));
        assert_eq!(
            decimal_value("7_i64"),
            Some(7),
            "an underscore before a suffix is Rust"
        );
        assert_eq!(decimal_value("7f"), None);
        assert_eq!(
            decimal_value("99999999999999999999999"),
            None,
            "overflow is not a value"
        );
        // An unterminated string and block comment end at the end of the source.
        assert!(numeric_literals("\"5").is_empty());
        assert!(numeric_literals("/* 5").is_empty());
        assert!(numeric_literals("r\"5").is_empty());
        assert_eq!(
            numeric_literals("r#x 5").len(),
            1,
            "a raw identifier is code"
        );
        assert!(numeric_literals("'\\u{35}'").is_empty());
        assert!(
            numeric_literals("x5 _5").is_empty(),
            "an identifier is not a literal"
        );
    }

    /// `code_only` blanks EXACTLY every comment, string and character literal
    /// — no more, no less — and keeps every newline: each case is the whole
    /// output, not the literals found in it.
    #[test]
    fn code_only_blanks_exactly_every_comment_string_and_character_literal() {
        let shown = |src: &str| code_only(src).into_iter().collect::<String>();
        let spaces = |n: usize| " ".repeat(n);
        // A line comment runs to the end of its line, the newline kept.
        assert_eq!(shown("a // b\nc"), "a     \nc");
        assert_eq!(shown("a // b"), "a     ");
        // A block comment nests, keeps its newlines, and an unterminated one
        // runs to the end.
        let nested = "/* b /* c */ d */";
        assert_eq!(
            shown(&format!("a {nested} e")),
            format!("a {} e", spaces(nested.len()))
        );
        assert_eq!(shown("x/*\n*/y"), "x  \n  y");
        // `/*/` opens a comment and does not close it.
        assert_eq!(shown("a /*/ b */ c"), format!("a {} c", spaces(8)));
        assert_eq!(shown("x /* y"), "x     ");
        assert_eq!(shown("x /* a */ / * y"), "x         / * y");
        // A raw string closes only on a quote followed by all its hashes.
        let raw = "r##\"a\"#b\"##";
        assert_eq!(
            shown(&format!("x {raw} y")),
            format!("x {} y", spaces(raw.len()))
        );
        assert_eq!(shown("x r\"a\\\" y"), format!("x{}y", spaces(7)));
        // After `b` it is still raw (a byte string); after another word
        // character it is an identifier followed by an ordinary string, in
        // which a backslash escapes the quote.
        assert_eq!(shown("br\"a\\\"b"), "b     b");
        assert_eq!(shown("xr\"a\\\"b"), "xr     ");
        assert_eq!(shown("r#x 5"), "r#x 5", "a raw identifier is code");
        assert_eq!(shown("r#\"a\"#"), spaces(6));
        assert_eq!(shown("r##x"), "r##x");
        // An ordinary string: escapes, newlines kept, unterminated to the end.
        assert_eq!(shown("a \"b\\\"c\" d"), "a        d");
        assert_eq!(shown("\"a\nb\" c"), "  \n   c");
        assert_eq!(shown("\"ab\\"), spaces(4));
        // Character literals, escaped or not, and a lifetime left alone.
        assert_eq!(
            shown("f('\\'', '\\n', 'x', &'a u8)"),
            "f(    ,     ,    , &'a u8)"
        );
        assert_eq!(shown("'\\u{35}' 7"), "         7");
        assert_eq!(shown("'"), "'");
        assert_eq!(shown("a/b*c"), "a/b*c", "a lone slash is code");
    }

    #[test]
    fn shipping_part_stops_at_the_test_module_and_untyped_numbers_allows_zero_and_one() {
        let src = "const A: u8 = 0;\nconst B: u8 = 1;\nconst C: u8 = 7;\n#[cfg(test)]\nmod t { const D: u8 = 9; }\n";
        assert_eq!(
            shipping_part(src),
            "const A: u8 = 0;\nconst B: u8 = 1;\nconst C: u8 = 7;\n"
        );
        let untyped = untyped_numbers(src);
        assert_eq!(untyped.len(), 1);
        assert_eq!(untyped[0].text, "7");
        assert_eq!(untyped[0].line, 3);
        assert_eq!(shipping_part("const A: u8 = 7;"), "const A: u8 = 7;");
        assert_eq!(
            untyped_numbers("let x = 0x1;")[0].value,
            None,
            "a hex zero-one is still refused"
        );
    }
}
