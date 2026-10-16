//! The FIB cube schema v1: 24 rows, 90 values, in a namespace of its own.
//!
//! Owned by FIB-1 (D-5300, D-5301). Every Fib Flag Breakout candidate lands in
//! exactly one FULL CELL of this schema — a [`Coords`], one value per row — and
//! the lattice (FIB-6) grows PARTIAL MIXES over it — a [`MixKey`], where a free
//! row is [`OPEN`].
//!
//! # Why this is not [`crate::table`]
//!
//! The condition table holds 370 of the mask's 384 positions, pinned by
//! `the_version_is_the_widened_table`, and its rows carry condition-table
//! semantics — including the indicator rows the operator banned as Fib
//! conditions. Widening [`crate::mask::WORDS`] to make room would re-key every
//! recorded run (`CLAUDE.md` §3 rule 8). So the cube is a SEPARATE namespace:
//! its row index and its value index are its identity, and [`to_mask`] places
//! it in positions 0..=89 of a [`ConditionMask`] that is **never decoded through
//! [`crate::table`]**. Such a mask means nothing to the condition table; it is
//! for FIB-6's support oracle only.
//!
//! # Append-only, by version
//!
//! The row index is the row's identity forever. A new row or a split value is
//! a NEW schema version ([`FIB_CUBE_VERSION`]); v1 cells stay decodable. Value
//! names are labels for buckets GENERATED from each stock's own past, never
//! thresholds. The exit row has no typed labels at all (operator 16 Oct 13:05
//! IST): its four values are slots that the run's [`Generation`] binds to
//! variants of the generated, versioned exit set (FIB-5).
//!
//! # The only typed numbers
//!
//! [`FIXED_NUMBERS`] is the one ledger of every number the FIB strategy path may
//! type (operator 16 Oct 11:24 IST, D-5309). Every numeric literal in this file
//! is one of its values, proved by reading this file with [`numeric_literals`];
//! every later FIB package proves its own files hold no literal but 0 and 1 with
//! [`untyped_numbers`].
//!
//! # Cost
//!
//! [`encode`], [`decode`], [`validate`], [`validate_mix`] and [`contains`] each
//! visit the 24 rows once with no early exit on the hot path: O(1), a constant
//! of this schema version. The bench is `crates/vocab/benches/fib_cube.rs`.

use crate::ConditionMask;

/// The schema version of the FIB cube, and the `vocab_version` term of a FIB
/// run's identity (`runner::fib::identity`). Bumped by a new row or a split
/// value, never by a relabel of a generated bucket.
pub const FIB_CUBE_VERSION: u32 = 1;

/// How many rows schema v1 has. The row index is the row's identity forever.
pub const ROW_COUNT: usize = 24;

/// How many values the 24 rows have together, and so how many positions
/// [`to_mask`] uses. Checked against [`ROWS`] at compile time.
pub const VALUE_COUNT: usize = 90;

/// One full cell: one value index per row, row 0 first.
pub type Coords = [u8; ROW_COUNT];

/// The marker of a free row in a [`MixKey`]: "any value of this row".
///
/// `u8::MAX`, which no row's value index can reach (the widest row has twelve
/// values). A [`Coords`] never holds it, and a partition row never holds it in
/// a mix (see [`validate_mix`]).
pub const OPEN: u8 = u8::MAX;

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
    /// Nested values: a cell holding the narrowest bucket `j` is in every mix
    /// fixing a bucket `k >= j` (a `rank_q1` trade also counts in q2, q3, q4).
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

/// One row of the schema: its name, its kind and its value labels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Row {
    /// The row's canonical name.
    pub name: &'static str,
    /// How its values relate.
    pub kind: RowKind,
    /// The labels of its values, by value index. Labels name generated
    /// buckets; none is a threshold.
    pub values: &'static [&'static str],
}

/// The rows of schema v1, by row index, by name. Implicit discriminants, so no
/// row index is ever typed: the declaration order IS [`ROWS`]' order, proved by
/// `every_row_id_names_its_row`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum RowId {
    /// Row 0: the trade's side against the stock's own move.
    Side,
    /// Row 1: the mover-rank bucket, nested (the only cumulative row).
    MoverRank,
    /// Row 2: how far the stock has moved so far today, own-past terciles.
    MoveSoFar,
    /// Row 3: the nearest Fib rung to the base's far edge.
    FibZone,
    /// Row 4: the distance from the far edge to that rung.
    LevelDistance,
    /// Row 5: where the base sits against that rung.
    LevelRelation,
    /// Row 6: whether a base candle made the day extreme.
    BaseHoldsExtreme,
    /// Row 7: the exit slot (a partition row), bound per run by [`Generation`].
    Exit,
    /// Row 8: the IST weekday.
    Weekday,
    /// Row 9: the generated time-of-day window.
    TimeOfDay,
    /// Row 10: micro base of two or three candles, or a rest.
    BaseKind,
    /// Row 11: the base's shape label (argmax of own-past shape permilles).
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
    /// Row 18: the sign of the trigger's order flow delta against the trade.
    TriggerOrderFlow,
    /// Row 19: the stop size against its own past.
    StopSize,
    /// Row 20: the room to the next level per unit of stop.
    RoomToStop,
    /// Row 21: the entry style (a partition row).
    EntryStyle,
    /// Row 22: the attempt number of the day.
    Attempt,
    /// Row 23: the candle timeframe (a partition row).
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

    /// The index of the value labelled `name` in this row, or `None`.
    ///
    /// O(values of the row): for setting up a caller's own table once, never
    /// for a per-trade path.
    #[must_use]
    pub fn value_of(self, name: &str) -> Option<u8> {
        (0_u8..)
            .zip(self.row().values)
            .find_map(|(index, label)| (*label == name).then_some(index))
    }
}

/// Defines one row as a named constant, so [`RowId::row`] reaches it by name
/// and no row is ever reached by a computed index.
macro_rules! row {
    ($const:ident, $name:literal, $kind:ident, [$($value:literal),+ $(,)?]) => {
        #[doc = concat!("The `", $name, "` row.")]
        pub const $const: Row = Row {
            name: $name,
            kind: RowKind::$kind,
            values: &[$($value),+],
        };
    };
}

row!(
    SIDE_ROW,
    "side",
    Categorical,
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
    ["rank_q1", "rank_q2", "rank_q3", "rank_q4"]
);
row!(
    MOVE_SO_FAR_ROW,
    "move_so_far",
    Categorical,
    ["low", "mid", "high"]
);
row!(
    FIB_ZONE_ROW,
    "fib_zone",
    Categorical,
    ["l0", "l236", "l382", "l500", "l618", "l786", "l1000",]
);
row!(
    LEVEL_DISTANCE_ROW,
    "level_distance",
    Categorical,
    ["on", "near", "far"]
);
row!(
    LEVEL_RELATION_ROW,
    "level_relation",
    Categorical,
    ["above", "below", "spans"]
);
row!(
    BASE_HOLDS_EXTREME_ROW,
    "base_holds_extreme",
    Categorical,
    ["no", "yes"]
);
row!(
    EXIT_ROW,
    "exit",
    Partition,
    ["slot_a", "slot_b", "slot_c", "slot_d"]
);
row!(
    WEEKDAY_ROW,
    "weekday",
    Categorical,
    ["mon", "tue", "wed", "thu", "fri"]
);
row!(
    TIME_OF_DAY_ROW,
    "time_of_day",
    Categorical,
    ["w1", "w2", "w3", "w4", "w5", "w6"]
);
row!(
    BASE_KIND_ROW,
    "base_kind",
    Categorical,
    ["micro2", "micro3", "rest"]
);
row!(
    BASE_SHAPE_ROW,
    "base_shape",
    Categorical,
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
    ["tight", "normal", "wide"]
);
row!(
    BASE_LENGTH_ROW,
    "base_length",
    Categorical,
    ["short", "normal", "long"]
);
row!(
    EDGE_TOUCHES_ROW,
    "edge_touches",
    Categorical,
    ["few", "many"]
);
row!(SQUEEZE_ROW, "squeeze", Categorical, ["yes", "no"]);
row!(
    TRIGGER_BODY_ROW,
    "trigger_body",
    Categorical,
    ["small", "normal", "big"]
);
row!(
    TRIGGER_CLOSE_LOCATION_ROW,
    "trigger_close_location",
    Categorical,
    ["near_extreme", "middle", "far",]
);
row!(
    TRIGGER_ORDER_FLOW_ROW,
    "trigger_order_flow",
    Categorical,
    ["with", "against", "none"]
);
row!(
    STOP_SIZE_ROW,
    "stop_size",
    Categorical,
    ["small", "normal", "large"]
);
row!(
    ROOM_TO_STOP_ROW,
    "room_to_stop",
    Categorical,
    ["low", "mid", "high"]
);
row!(
    ENTRY_STYLE_ROW,
    "entry_style",
    Partition,
    ["close", "break", "retest"]
);
row!(
    ATTEMPT_ROW,
    "attempt",
    Categorical,
    ["first", "second_plus"]
);
row!(
    TIMEFRAME_ROW,
    "timeframe",
    Partition,
    ["m1", "m3", "m5", "m15"]
);

/// The 24 rows of schema v1, in canonical order. Row 7's values are exit SLOTS
/// with no typed exit meaning (operator 16 Oct 13:05 IST).
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

/// The number of full cells: the product of every row's value count. A PAPER
/// count — never enumerated; each candidate lands in one cell.
pub const PAPER_FULL_CELLS: u64 = 5_485_491_486_720;

/// The number of partial mixes: free rows contribute their value count plus
/// one (for [`OPEN`]), partition rows their value count. A PAPER count — the
/// lattice visits only what its trade counts reach.
pub const PAPER_PARTIAL_MIXES: u64 = 1_780_773_971_558_400;

/// How many values a row has, as a `u64` radix.
const fn arity(row: &Row) -> u64 {
    row.values.len() as u64
}

/// The product of every row's value count, at compile time.
const fn radix_product() -> u64 {
    let mut rows: &[Row] = &ROWS;
    let mut product: u64 = 1;
    while let [row, rest @ ..] = rows {
        product *= arity(row);
        rows = rest;
    }
    product
}

/// The product of every row's mix choices, at compile time.
const fn mix_product() -> u64 {
    let mut rows: &[Row] = &ROWS;
    let mut product: u64 = 1;
    while let [row, rest @ ..] = rows {
        let choices = match row.kind {
            RowKind::Partition => arity(row),
            RowKind::Categorical | RowKind::Cumulative => arity(row) + 1,
        };
        product *= choices;
        rows = rest;
    }
    product
}

/// The total of every row's value count, at compile time.
const fn value_total() -> usize {
    let mut rows: &[Row] = &ROWS;
    let mut total = 0;
    while let [row, rest @ ..] = rows {
        total += row.values.len();
        rows = rest;
    }
    total
}

/// Whether every row index of [`RowId::ALL`] is its position, and every
/// [`RowId::row`] is the row [`ROWS`] holds there, at compile time.
const fn row_ids_in_order() -> bool {
    let mut ids: &[RowId] = &RowId::ALL;
    let mut rows: &[Row] = &ROWS;
    let mut expected = 0;
    let mut ordered = true;
    while let ([id, rest @ ..], [row, more @ ..]) = (ids, rows) {
        ordered &= id.index() == expected;
        ordered &= same_name(id.row().name, row.name);
        expected += 1;
        ids = rest;
        rows = more;
    }
    ordered
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

const _: () = assert!(radix_product() == PAPER_FULL_CELLS);
const _: () = assert!(mix_product() == PAPER_PARTIAL_MIXES);
const _: () = assert!(value_total() == VALUE_COUNT);
const _: () = assert!(row_ids_in_order());

/// How many slots the exit row reserves in this schema version.
pub const EXIT_SLOTS: usize = EXIT_ROW.values.len();

/// How many nested mover-rank buckets there are, and so how many rank edges a
/// [`Generation`] carries.
pub const RANK_EDGES: usize = MOVER_RANK_ROW.values.len();

/// How many time-of-day windows there are, and so how many window starts a
/// [`Generation`] carries.
pub const TIME_WINDOWS: usize = TIME_OF_DAY_ROW.values.len();

/// A refusal of this module, by name. Nothing here degrades silently.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CubeRefusal {
    /// A value index at or above its row's value count.
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
    /// A code at or above [`PAPER_FULL_CELLS`].
    CodeOutOfRange {
        /// The offending code.
        code: u64,
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
    /// Rank edges that are not non-decreasing or start below rank one.
    RankEdgesInvalid {
        /// The edges offered.
        edges: [u16; RANK_EDGES],
    },
    /// Time-window starts that are not non-decreasing or leave the day.
    TimeStartsInvalid {
        /// The starts offered.
        starts: [u32; TIME_WINDOWS],
    },
    /// No exit slot is bound to a variant.
    ExitSlotsEmpty,
    /// Two exit slots bound to one variant.
    ExitSlotRepeated {
        /// The repeated variant id.
        variant: u32,
    },
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
            Self::RankEdgesInvalid { .. } => "rank_edges_invalid",
            Self::TimeStartsInvalid { .. } => "time_starts_invalid",
            Self::ExitSlotsEmpty => "exit_slots_empty",
            Self::ExitSlotRepeated { .. } => "exit_slot_repeated",
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

/// Checks a full cell: every row holds a value index below its value count.
///
/// # Errors
///
/// [`CubeRefusal::OpenInCell`] for an [`OPEN`] row and
/// [`CubeRefusal::ValueOutOfRange`] for any other out-of-range index, naming the
/// first offending row.
pub fn validate(coords: &Coords) -> Result<(), CubeRefusal> {
    for (value, row) in coords.iter().zip(&ROWS) {
        if *value == OPEN {
            return Err(CubeRefusal::OpenInCell { row: row.name });
        }
        if usize::from(*value) >= row.values.len() {
            return Err(CubeRefusal::ValueOutOfRange {
                row: row.name,
                value: *value,
                values: row.values.len(),
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
pub fn validate_mix(mix: &MixKey) -> Result<(), CubeRefusal> {
    for (value, row) in mix.0.iter().zip(&ROWS) {
        if *value == OPEN {
            if row.kind == RowKind::Partition {
                return Err(CubeRefusal::OpenPartitionRow { row: row.name });
            }
        } else if usize::from(*value) >= row.values.len() {
            return Err(CubeRefusal::ValueOutOfRange {
                row: row.name,
                value: *value,
                values: row.values.len(),
            });
        }
    }
    Ok(())
}

/// The mixed-radix code of a full cell, row 0 most significant.
///
/// # Errors
///
/// Whatever [`validate`] refuses: an invalid cell has no code.
pub fn encode(coords: &Coords) -> Result<u64, CubeRefusal> {
    validate(coords)?;
    let mut code: u64 = 0;
    for (value, row) in coords.iter().zip(&ROWS) {
        code = code * arity(row) + u64::from(*value);
    }
    Ok(code)
}

/// The full cell of a code, the inverse of [`encode`].
///
/// # Errors
///
/// [`CubeRefusal::CodeOutOfRange`] for a code at or above [`PAPER_FULL_CELLS`].
pub fn decode(code: u64) -> Result<Coords, CubeRefusal> {
    if code >= PAPER_FULL_CELLS {
        return Err(CubeRefusal::CodeOutOfRange { code });
    }
    let mut coords: Coords = [0; ROW_COUNT];
    let mut rest = code;
    for (slot, row) in coords.iter_mut().zip(&ROWS).rev() {
        let radix = arity(row);
        *slot = u8::try_from(rest % radix).unwrap_or(OPEN);
        rest /= radix;
    }
    // Every digit is below its radix by construction, so this cannot refuse;
    // it is here so a broken digit loop is a refusal and not a wrong cell.
    validate(&coords).map(|()| coords)
}

/// Whether the cell is in the mix: every row is [`OPEN`], equal, or — on the
/// cumulative row — nested inside the mix's bucket.
///
/// Both arguments must already have passed [`validate`] and [`validate_mix`];
/// this is the per-row hot path and does not re-check. It visits all 24 rows
/// with no early exit.
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

/// The cell as a [`ConditionMask`] in the FIB NAMESPACE: each row's values
/// occupy the next positions in row order (0..=89 in all), one bit per value;
/// the cumulative row sets its bucket and every wider one (a thermometer).
///
/// **Never decode this mask through [`crate::table`].** It shares the mask type
/// and nothing else with the condition table. Used only by FIB-6's oracle.
///
/// # Errors
///
/// Whatever [`validate`] refuses.
pub fn to_mask(coords: &Coords) -> Result<ConditionMask, CubeRefusal> {
    validate(coords)?;
    let mut mask = ConditionMask::ZERO;
    let mut offset: u32 = 0;
    for (value, row) in coords.iter().zip(&ROWS) {
        let width = row_width(row);
        let mut bit = u32::from(*value);
        mask = mask.with_bit(offset + bit);
        if row.kind == RowKind::Cumulative {
            while bit < width {
                mask = mask.with_bit(offset + bit);
                bit += 1;
            }
        }
        offset += width;
    }
    Ok(mask)
}

/// The mix as a [`ConditionMask`] in the FIB NAMESPACE: one required bit per
/// fixed row, none for an [`OPEN`] row. A cell's [`to_mask`] hits it
/// (`ConditionMask::hits`) exactly when [`contains`] is true.
///
/// # Errors
///
/// Whatever [`validate_mix`] refuses.
pub fn mix_mask(mix: &MixKey) -> Result<ConditionMask, CubeRefusal> {
    validate_mix(mix)?;
    let mut mask = ConditionMask::ZERO;
    let mut offset: u32 = 0;
    for (value, row) in mix.0.iter().zip(&ROWS) {
        if *value != OPEN {
            mask = mask.with_bit(offset + u32::from(*value));
        }
        offset += row_width(row);
    }
    Ok(mask)
}

/// A row's value count as a mask width.
fn row_width(row: &Row) -> u32 {
    u32::try_from(row.values.len()).unwrap_or(u32::MAX)
}

/// The equal-count buckets of [`tercile`].
pub const TERCILE_BUCKETS: u64 = 3;

/// The equal-count buckets of [`median`].
pub const MEDIAN_BUCKETS: u64 = 2;

/// One thousand: the whole of a permille.
pub const PERMILLE: u16 = 1_000;

/// The equal-count bucket of an own-past permille among `buckets` buckets: the
/// number of interior edges `k * 1000 / buckets` at or below it, so a value on
/// an edge goes UP. Cross-multiplied: no division, no rounding.
fn equal_count(permille: u16, buckets: u64) -> Result<u8, CubeRefusal> {
    if permille > PERMILLE {
        return Err(CubeRefusal::PermilleOutOfRange { permille });
    }
    let scaled = u64::from(permille) * buckets;
    let mut bucket: u8 = 0;
    let mut edge: u64 = 1;
    while edge < buckets {
        bucket += u8::from(scaled >= edge * u64::from(PERMILLE));
        edge += 1;
    }
    Ok(bucket)
}

/// The equal-count third of the stock's own past an own-past permille falls in
/// (0 low, 1 mid, 2 high): the coarse start "2-3 buckets per row". A value on
/// an edge goes up.
///
/// Takes a PERMILLE of the stock's own past (FIB-2's `OwnPast::permille`),
/// never a raw value.
///
/// # Errors
///
/// [`CubeRefusal::PermilleOutOfRange`] above one thousand.
pub fn tercile(permille: u16) -> Result<u8, CubeRefusal> {
    equal_count(permille, TERCILE_BUCKETS)
}

/// The equal-count half of the stock's own past an own-past permille falls in
/// (0 below the median, 1 at or above it). A value on the median goes up.
///
/// # Errors
///
/// [`CubeRefusal::PermilleOutOfRange`] above one thousand.
pub fn median(permille: u16) -> Result<u8, CubeRefusal> {
    equal_count(permille, MEDIAN_BUCKETS)
}

/// The narrowest nested rank bucket a rank falls in, with the edges from the
/// run's [`Generation`]: bucket `k` iff `rank <= edges[k]` and no narrower
/// bucket holds it. Rank 0 is "unranked" (FIB-2's convention) and has no
/// bucket; neither has a rank above the widest edge.
#[must_use]
pub fn rank_bucket(rank: u16, edges: [u16; RANK_EDGES]) -> Option<u8> {
    if rank == 0 {
        return None;
    }
    (0_u8..)
        .zip(edges)
        .find_map(|(bucket, edge)| (rank <= edge).then_some(bucket))
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
pub const ZONE_RUNGS: [u64; FIB_ZONE_ROW.values.len()] = [
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
            .values
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

    /// Checks a mix for this search: [`validate_mix`], and a fixed side value
    /// must belong to this search (an [`OPEN`] side means "either side value of
    /// this search").
    ///
    /// # Errors
    ///
    /// Whatever [`validate_mix`] refuses, or
    /// [`CubeRefusal::SideOutsideSearch`].
    pub fn validate_mix(self, mix: &MixKey) -> Result<(), CubeRefusal> {
        validate_mix(mix)?;
        let [side, ..] = mix.0;
        match SIDE_ROW.values.get(usize::from(side)) {
            Some(label) if Self::of_label(label) != self => Err(CubeRefusal::SideOutsideSearch {
                search: self.name(),
                side: label,
            }),
            _ => Ok(()),
        }
    }
}

/// How many bytes a digest carried in a [`Generation`] has (BLAKE3's width;
/// `vocab` cannot hash, so the caller supplies it).
pub const DIGEST_BYTES: usize = 32;

/// The value of an exit slot no variant is bound to.
pub const EXIT_SLOT_UNBOUND: u32 = u32::MAX;

/// Everything a [`Generation`] is built from. No `Default`: every value is
/// named by the run that generated it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GenerationParts {
    /// The search (direction) this generation belongs to: every generated
    /// number is per direction (operator 16 Oct 16:04 IST).
    pub search: Search,
    /// The nested rank edges (inclusive upper ranks of `rank_q1..rank_q4`): the
    /// equal-count quartiles of this search's decision ranks pooled over the
    /// first learning year only.
    pub rank_edges: [u16; RANK_EDGES],
    /// The first second (after IST midnight) of each time-of-day window
    /// `w1..w6`; window 6 runs to the end of the day. Starts 2..6 are the
    /// equal-count sextile boundaries of this search's decision instants over
    /// the first learning year only; start 1 is the session's grid anchor.
    pub time_starts: [u32; TIME_WINDOWS],
    /// The version of the generated exit variant set (FIB-5).
    pub exit_set_version: u32,
    /// The digest of that set's canonical bytes, computed by the caller.
    pub exit_set_digest: [u8; DIGEST_BYTES],
    /// The variant id bound to each exit slot, or [`EXIT_SLOT_UNBOUND`].
    pub exit_slots: [u32; EXIT_SLOTS],
}

/// The per-run, per-direction generated numbers the schema reads: the rank
/// edges, the time-of-day windows and the exit-slot binding, plus the schema
/// version. Built only through [`Generation::new`], which checks every part.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Generation {
    /// The checked parts.
    parts: GenerationParts,
}

impl Generation {
    /// A checked generation.
    ///
    /// # Errors
    ///
    /// [`CubeRefusal::RankEdgesInvalid`] unless the edges are non-decreasing and
    /// the first is at least rank one; [`CubeRefusal::TimeStartsInvalid`]
    /// unless the starts are non-decreasing and inside the day;
    /// [`CubeRefusal::ExitSlotsEmpty`] when no slot is bound;
    /// [`CubeRefusal::ExitSlotRepeated`] when two slots name one variant.
    pub fn new(parts: GenerationParts) -> Result<Self, CubeRefusal> {
        let edges = parts.rank_edges;
        let [first_edge, ..] = edges;
        if first_edge == 0 || edges.windows(2).any(descending) {
            return Err(CubeRefusal::RankEdgesInvalid { edges });
        }
        let starts = parts.time_starts;
        let day = u32::try_from(SECONDS_PER_DAY).unwrap_or(u32::MAX);
        if starts.windows(2).any(descending) || starts.last().is_some_and(|last| *last >= day) {
            return Err(CubeRefusal::TimeStartsInvalid { starts });
        }
        let slots = parts.exit_slots;
        if slots.iter().all(|slot| *slot == EXIT_SLOT_UNBOUND) {
            return Err(CubeRefusal::ExitSlotsEmpty);
        }
        for (at, slot) in slots.iter().enumerate() {
            let repeated = slots.iter().skip(at + 1).any(|other| other == slot);
            if *slot != EXIT_SLOT_UNBOUND && repeated {
                return Err(CubeRefusal::ExitSlotRepeated { variant: *slot });
            }
        }
        Ok(Self { parts })
    }

    /// The parts this generation was built from.
    #[must_use]
    pub const fn parts(&self) -> &GenerationParts {
        &self.parts
    }

    /// The schema version this generation reads ([`FIB_CUBE_VERSION`]).
    #[must_use]
    pub const fn schema_version(&self) -> u32 {
        FIB_CUBE_VERSION
    }

    /// [`rank_bucket`] with this generation's edges.
    #[must_use]
    pub fn rank_bucket(&self, rank: u16) -> Option<u8> {
        rank_bucket(rank, self.parts.rank_edges)
    }

    /// The time-of-day window a second after IST midnight falls in: the last
    /// window whose start is at or before it; `None` before window 1.
    #[must_use]
    pub fn time_window(&self, second_of_day: u32) -> Option<u8> {
        let mut window = None;
        for (index, start) in (0_u8..).zip(self.parts.time_starts) {
            if start <= second_of_day {
                window = Some(index);
            }
        }
        window
    }

    /// The exit slot a variant id is bound to, or `None`.
    #[must_use]
    pub fn exit_slot(&self, variant: u32) -> Option<u8> {
        if variant == EXIT_SLOT_UNBOUND {
            return None;
        }
        (0_u8..)
            .zip(self.parts.exit_slots)
            .find_map(|(slot, bound)| (bound == variant).then_some(slot))
    }

    /// The canonical bytes for the run identity: a magic, the schema version,
    /// the search, the rank edges, the window starts and the exit binding, each
    /// framed with its length, little-endian.
    #[must_use]
    pub fn bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(b"BTXFIBG1");
        out.extend_from_slice(&FIB_CUBE_VERSION.to_le_bytes());
        frame(&mut out, self.parts.search.name().as_bytes());
        count(&mut out, RANK_EDGES);
        for edge in self.parts.rank_edges {
            out.extend_from_slice(&edge.to_le_bytes());
        }
        count(&mut out, TIME_WINDOWS);
        for start in self.parts.time_starts {
            out.extend_from_slice(&start.to_le_bytes());
        }
        out.extend_from_slice(&self.parts.exit_set_version.to_le_bytes());
        out.extend_from_slice(&self.parts.exit_set_digest);
        count(&mut out, EXIT_SLOTS);
        for slot in self.parts.exit_slots {
            out.extend_from_slice(&slot.to_le_bytes());
        }
        out
    }
}

/// Whether a window of two is strictly descending (`[a, b]` with `a > b`).
fn descending<T: PartialOrd>(pair: &[T]) -> bool {
    matches!(pair, [a, b] if a > b)
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

/// The schema's canonical bytes, for hashing in `runner` (this crate depends
/// on nothing and cannot hash): a magic, the version, every row's name, kind
/// and value labels, the zone rungs and the bucket counts, each framed.
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
        count(&mut out, row.values.len());
        for value in row.values {
            frame(&mut out, value.as_bytes());
        }
    }
    count(&mut out, ZONE_RUNGS.len());
    for rung in ZONE_RUNGS {
        out.extend_from_slice(&rung.to_le_bytes());
    }
    out.extend_from_slice(&TERCILE_BUCKETS.to_le_bytes());
    out.extend_from_slice(&MEDIAN_BUCKETS.to_le_bytes());
    out.push(OPEN);
    out
}

/// The category of a fixed number: why it may be typed at all.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Category {
    /// A Winner Gate threshold, confirmed by the operator (16 Oct 2026).
    WinnerGate,
    /// A session time the operator set (the 09:15 grid, the own exits, the
    /// forced window) or a candle length on that grid.
    OperatorSessionTime,
    /// A trade size the gate is judged at.
    TradeSize,
    /// A pattern size the operator named (micro bases of two and three candles).
    OperatorPatternSize,
    /// A Fibonacci ratio, pinned to `indicators::fib::RUNGS`.
    FibonacciRatio,
    /// A unit of measure (basis points, permille, seconds, calendar months).
    Unit,
    /// A representation choice: it sets a width or a resolution, never a
    /// threshold.
    Representation,
    /// A row's value count in this schema version (refined by a later
    /// generation, which is a new schema version).
    CoarseBucketCount,
    /// A report column the operator named; it decides nothing.
    OperatorReportColumn,
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
            Self::CoarseBucketCount => "coarse_bucket_count",
            Self::OperatorReportColumn => "operator_report_column",
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
/// At least 500 trades a year, counted across both sides of a pair (operator
/// 16 Oct 13:57 IST; supersedes 100).
pub const GATE_TRADES_PER_YEAR_MIN: u64 = 500;
/// At least 70% of the unseen months positive (permille).
pub const GATE_UNSEEN_MONTHS_POSITIVE_MIN_PERMILLE: u64 = 700;

/// The 09:15:00 IST grid anchor, in seconds after IST midnight.
pub const SESSION_GRID_ANCHOR_SECS: u64 = 33_300;
/// The earlier own exit, 15:07:00 IST.
pub const SESSION_OWN_EXIT_EARLY_SECS: u64 = 54_420;
/// The later own exit, 15:08:00 IST.
pub const SESSION_OWN_EXIT_LATE_SECS: u64 = 54_480;
/// The forced-exit window's first second, 15:09:00 IST.
pub const SESSION_FORCED_WINDOW_START_SECS: u64 = 54_540;
/// The forced-exit window's end, 15:10:00 IST (excluded).
pub const SESSION_FORCED_WINDOW_END_SECS: u64 = 54_600;
/// The 3-minute candle (the `m3` timeframe).
pub const TIMEFRAME_M3_MINUTES: u64 = 3;
/// The 5-minute candle (the `m5` timeframe).
pub const TIMEFRAME_M5_MINUTES: u64 = 5;
/// The 15-minute candle (the `m15` timeframe).
pub const TIMEFRAME_M15_MINUTES: u64 = 15;

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
/// Calendar months in a gate year (D-5309): the first walk-forward window and
/// the exam length are each one gate year.
pub const MONTHS_PER_GATE_YEAR: u64 = 12;
/// Days in a week (the leave-one-out week).
pub const DAYS_PER_WEEK: u64 = 7;
/// Days in a fortnight (the leave-one-out fortnight).
pub const DAYS_PER_FORTNIGHT: u64 = 14;
/// Months in a quarter (the leave-one-out quarter).
pub const MONTHS_PER_QUARTER: u64 = 3;
/// Months in a half-year (the report's period table).
pub const MONTHS_PER_HALF_YEAR: u64 = 6;

/// FIB-2's histogram: sub-bins per power of two (sets percentile resolution).
pub const HIST_SUB_BINS_PER_POWER: u64 = 8;
/// FIB-2's histogram: the highest power of two before the overflow bin.
pub const HIST_TOP_POWER: u64 = 40;

/// The giant-winner report column at 5R (SCORECARD+).
pub const REPORT_GIANT_WIN_5R: u64 = 5;
/// The giant-winner report column at 10R (SCORECARD+).
pub const REPORT_GIANT_WIN_10R: u64 = 10;
/// The giant-winner report column at 20R (SCORECARD+).
pub const REPORT_GIANT_WIN_20R: u64 = 20;
/// FIND THE MASSIVE WINNERS: the top 5% by MFE (permille).
pub const REPORT_TOP_MFE_PERMILLE: u64 = 50;
/// The operator's "room of about 2.5x the stop" (rule d), in thousandths: a
/// labelled reference split in reports only, never a filter.
pub const REPORT_ROOM_TO_STOP_REFERENCE_MILLI: u64 = 2_500;

/// THE ONE LEDGER of every number the FIB strategy path may type (operator
/// 16 Oct 11:24 IST, D-5309): the Winner Gate thresholds are the only fixed
/// numbers that DECIDE; every other row here is a session time, a size, a
/// pattern size, a ratio, a unit, a representation, a coarse bucket count or a
/// report column, and its category says which. Every other value is generated
/// from each stock's own past.
///
/// The minimum stop of 0.2% is NOT here: removed by the operator on
/// 16 Oct 13:07 IST.
pub const FIXED_NUMBERS: &[FixedNumber] = &[
    (
        "gate_win_rate_min_permille",
        GATE_WIN_RATE_MIN_PERMILLE,
        Category::WinnerGate,
        "D-5309",
    ),
    (
        "gate_average_win_ratio_min",
        GATE_AVERAGE_WIN_RATIO_MIN,
        Category::WinnerGate,
        "D-5309",
    ),
    (
        "gate_profit_factor_min",
        GATE_PROFIT_FACTOR_MIN,
        Category::WinnerGate,
        "D-5309",
    ),
    (
        "gate_largest_win_ratio_min",
        GATE_LARGEST_WIN_RATIO_MIN,
        Category::WinnerGate,
        "D-5309",
    ),
    (
        "gate_trades_per_year_min",
        GATE_TRADES_PER_YEAR_MIN,
        Category::WinnerGate,
        "D-5309",
    ),
    (
        "gate_unseen_months_positive_min_permille",
        GATE_UNSEEN_MONTHS_POSITIVE_MIN_PERMILLE,
        Category::WinnerGate,
        "D-5309",
    ),
    (
        "session_grid_anchor_secs",
        SESSION_GRID_ANCHOR_SECS,
        Category::OperatorSessionTime,
        "D-5309",
    ),
    (
        "session_own_exit_early_secs",
        SESSION_OWN_EXIT_EARLY_SECS,
        Category::OperatorSessionTime,
        "D-5309",
    ),
    (
        "session_own_exit_late_secs",
        SESSION_OWN_EXIT_LATE_SECS,
        Category::OperatorSessionTime,
        "D-5309",
    ),
    (
        "session_forced_window_start_secs",
        SESSION_FORCED_WINDOW_START_SECS,
        Category::OperatorSessionTime,
        "D-5309",
    ),
    (
        "session_forced_window_end_secs",
        SESSION_FORCED_WINDOW_END_SECS,
        Category::OperatorSessionTime,
        "D-5309",
    ),
    (
        "timeframe_m3_minutes",
        TIMEFRAME_M3_MINUTES,
        Category::OperatorSessionTime,
        "D-5309",
    ),
    (
        "timeframe_m5_minutes",
        TIMEFRAME_M5_MINUTES,
        Category::OperatorSessionTime,
        "D-5309",
    ),
    (
        "timeframe_m15_minutes",
        TIMEFRAME_M15_MINUTES,
        Category::OperatorSessionTime,
        "D-5309",
    ),
    (
        "trade_size_small_rupees",
        TRADE_SIZE_SMALL_RUPEES,
        Category::TradeSize,
        "D-5309",
    ),
    (
        "trade_size_large_rupees",
        TRADE_SIZE_LARGE_RUPEES,
        Category::TradeSize,
        "D-5309",
    ),
    (
        "micro2_candles",
        MICRO2_CANDLES,
        Category::OperatorPatternSize,
        "D-5309",
    ),
    (
        "micro3_candles",
        MICRO3_CANDLES,
        Category::OperatorPatternSize,
        "D-5309",
    ),
    (
        "zone_rung_236 = indicators::fib::RUNGS[1]",
        ZONE_RUNG_236,
        Category::FibonacciRatio,
        "D-5309",
    ),
    (
        "zone_rung_382 = indicators::fib::RUNGS[2]",
        ZONE_RUNG_382,
        Category::FibonacciRatio,
        "D-5309",
    ),
    (
        "zone_rung_500 = indicators::fib::RUNGS[3]",
        ZONE_RUNG_500,
        Category::FibonacciRatio,
        "D-5309",
    ),
    (
        "zone_rung_618 = indicators::fib::RUNGS[4]",
        ZONE_RUNG_618,
        Category::FibonacciRatio,
        "D-5309",
    ),
    (
        "zone_rung_786 = indicators::fib::RUNGS[5]",
        ZONE_RUNG_786,
        Category::FibonacciRatio,
        "D-5309",
    ),
    (
        "zone_rung_1000 = indicators::fib::RUNGS[6]",
        ZONE_RUNG_1000,
        Category::FibonacciRatio,
        "D-5309",
    ),
    ("bp_per_unit", BP_PER_UNIT, Category::Unit, "D-5309"),
    (
        "permille_per_unit",
        PERMILLE_PER_UNIT,
        Category::Unit,
        "D-5309",
    ),
    (
        "seconds_per_minute",
        SECONDS_PER_MINUTE,
        Category::Unit,
        "D-5309",
    ),
    (
        "micros_per_second",
        MICROS_PER_SECOND,
        Category::Unit,
        "D-5309",
    ),
    ("seconds_per_day", SECONDS_PER_DAY, Category::Unit, "D-5309"),
    (
        "months_per_gate_year",
        MONTHS_PER_GATE_YEAR,
        Category::Unit,
        "D-5309",
    ),
    ("days_per_week", DAYS_PER_WEEK, Category::Unit, "D-5309"),
    (
        "days_per_fortnight",
        DAYS_PER_FORTNIGHT,
        Category::Unit,
        "D-5309",
    ),
    (
        "months_per_quarter",
        MONTHS_PER_QUARTER,
        Category::Unit,
        "D-5309",
    ),
    (
        "months_per_half_year",
        MONTHS_PER_HALF_YEAR,
        Category::Unit,
        "D-5309",
    ),
    (
        "hist_sub_bins_per_power",
        HIST_SUB_BINS_PER_POWER,
        Category::Representation,
        "D-5309",
    ),
    (
        "hist_top_power",
        HIST_TOP_POWER,
        Category::Representation,
        "D-5309",
    ),
    (
        "schema_row_count",
        ROW_COUNT as u64,
        Category::Representation,
        "D-5301",
    ),
    (
        "schema_value_count",
        VALUE_COUNT as u64,
        Category::Representation,
        "D-5301",
    ),
    (
        "digest_bytes",
        DIGEST_BYTES as u64,
        Category::Representation,
        "D-5303",
    ),
    (
        "paper_full_cells",
        PAPER_FULL_CELLS,
        Category::Representation,
        "D-5301",
    ),
    (
        "paper_partial_mixes",
        PAPER_PARTIAL_MIXES,
        Category::Representation,
        "D-5301",
    ),
    (
        "tercile_buckets",
        TERCILE_BUCKETS,
        Category::CoarseBucketCount,
        "D-5301",
    ),
    (
        "median_buckets",
        MEDIAN_BUCKETS,
        Category::CoarseBucketCount,
        "D-5301",
    ),
    ("values:side", 4, Category::CoarseBucketCount, "D-5301"),
    (
        "values:mover_rank",
        4,
        Category::CoarseBucketCount,
        "D-5301",
    ),
    (
        "values:move_so_far",
        3,
        Category::CoarseBucketCount,
        "D-5301",
    ),
    ("values:fib_zone", 7, Category::CoarseBucketCount, "D-5301"),
    (
        "values:level_distance",
        3,
        Category::CoarseBucketCount,
        "D-5301",
    ),
    (
        "values:level_relation",
        3,
        Category::CoarseBucketCount,
        "D-5301",
    ),
    (
        "values:base_holds_extreme",
        2,
        Category::CoarseBucketCount,
        "D-5301",
    ),
    ("values:exit", 4, Category::CoarseBucketCount, "D-5301"),
    ("values:weekday", 5, Category::CoarseBucketCount, "D-5301"),
    (
        "values:time_of_day",
        6,
        Category::CoarseBucketCount,
        "D-5301",
    ),
    ("values:base_kind", 3, Category::CoarseBucketCount, "D-5301"),
    (
        "values:base_shape",
        12,
        Category::CoarseBucketCount,
        "D-5301",
    ),
    (
        "values:base_tightness",
        3,
        Category::CoarseBucketCount,
        "D-5301",
    ),
    (
        "values:base_length",
        3,
        Category::CoarseBucketCount,
        "D-5301",
    ),
    (
        "values:edge_touches",
        2,
        Category::CoarseBucketCount,
        "D-5301",
    ),
    ("values:squeeze", 2, Category::CoarseBucketCount, "D-5301"),
    (
        "values:trigger_body",
        3,
        Category::CoarseBucketCount,
        "D-5301",
    ),
    (
        "values:trigger_close_location",
        3,
        Category::CoarseBucketCount,
        "D-5301",
    ),
    (
        "values:trigger_order_flow",
        3,
        Category::CoarseBucketCount,
        "D-5301",
    ),
    ("values:stop_size", 3, Category::CoarseBucketCount, "D-5301"),
    (
        "values:room_to_stop",
        3,
        Category::CoarseBucketCount,
        "D-5301",
    ),
    (
        "values:entry_style",
        3,
        Category::CoarseBucketCount,
        "D-5301",
    ),
    ("values:attempt", 2, Category::CoarseBucketCount, "D-5301"),
    ("values:timeframe", 4, Category::CoarseBucketCount, "D-5301"),
    (
        "report_giant_win_5r",
        REPORT_GIANT_WIN_5R,
        Category::OperatorReportColumn,
        "D-5309",
    ),
    (
        "report_giant_win_10r",
        REPORT_GIANT_WIN_10R,
        Category::OperatorReportColumn,
        "D-5309",
    ),
    (
        "report_giant_win_20r",
        REPORT_GIANT_WIN_20R,
        Category::OperatorReportColumn,
        "D-5309",
    ),
    (
        "report_top_mfe_permille",
        REPORT_TOP_MFE_PERMILLE,
        Category::OperatorReportColumn,
        "D-5309",
    ),
    (
        "report_room_to_stop_reference_milli",
        REPORT_ROOM_TO_STOP_REFERENCE_MILLI,
        Category::OperatorReportColumn,
        "D-5309",
    ),
];

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
    reason = "the exception every test module in this workspace takes: a test \
              that cannot panic cannot fail."
)]
mod tests {
    //! The FIB cube schema v1, proved against its specification (FIB-1, D-5301).

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

        fn below(&mut self, n: usize) -> u8 {
            u8::try_from(self.next() % n as u64).unwrap()
        }

        fn cell(&mut self) -> Coords {
            let mut c = [0; ROW_COUNT];
            for (slot, row) in c.iter_mut().zip(&ROWS) {
                *slot = self.below(row.values.len());
            }
            c
        }

        fn mix(&mut self) -> MixKey {
            let mut m = [OPEN; ROW_COUNT];
            for (slot, row) in m.iter_mut().zip(&ROWS) {
                let free = row.kind != RowKind::Partition && self.next().is_multiple_of(3);
                if !free {
                    *slot = self.below(row.values.len());
                }
            }
            MixKey(m)
        }
    }

    fn generation() -> Generation {
        Generation::new(GenerationParts {
            search: Search::Long,
            rank_edges: [3, 9, 20, 41],
            time_starts: [33_300, 36_000, 39_600, 43_200, 46_800, 50_400],
            exit_set_version: 7,
            exit_set_digest: [0xAB; DIGEST_BYTES],
            exit_slots: [11, 12, EXIT_SLOT_UNBOUND, 14],
        })
        .unwrap()
    }

    #[test]
    fn the_schema_is_twenty_four_rows_and_ninety_values_in_canonical_order() {
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
        assert_eq!(ROWS.iter().map(|r| r.values.len()).sum::<usize>(), 90);
        assert_eq!(VALUE_COUNT, 90);
        assert_eq!(ROW_COUNT, 24);
        assert_eq!(FIB_CUBE_VERSION, 1);
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
        assert_eq!(
            SIDE_ROW.values,
            [
                "long_with_gainer",
                "short_with_loser",
                "short_mirror_on_gainer",
                "long_mirror_on_loser"
            ]
        );
        assert_eq!(
            FIB_ZONE_ROW.values,
            ["l0", "l236", "l382", "l500", "l618", "l786", "l1000"]
        );
        assert_eq!(BASE_SHAPE_ROW.values.len(), 12);
        assert_eq!(BASE_SHAPE_ROW.values[11], "stair");
        assert_eq!(TIMEFRAME_ROW.values, ["m1", "m3", "m5", "m15"]);
        // The exit row has NO typed exit label (operator 16 Oct 13:05 IST).
        for label in EXIT_ROW.values {
            for typed in ["next_level", "ext_1272", "ext_1618", "trail"] {
                assert_ne!(*label, typed);
            }
        }
    }

    #[test]
    fn both_paper_counts_are_the_radix_products() {
        let full: u64 = ROWS.iter().map(|r| r.values.len() as u64).product();
        assert_eq!(full, 5_485_491_486_720);
        assert_eq!(PAPER_FULL_CELLS, full);
        let partial: u64 = ROWS
            .iter()
            .map(|r| match r.kind {
                RowKind::Partition => r.values.len() as u64,
                _ => r.values.len() as u64 + 1,
            })
            .product();
        assert_eq!(partial, 1_780_773_971_558_400);
        assert_eq!(PAPER_PARTIAL_MIXES, partial);
        assert_eq!(radix_product(), PAPER_FULL_CELLS);
        assert_eq!(mix_product(), PAPER_PARTIAL_MIXES);
        assert_eq!(value_total(), VALUE_COUNT);
        assert!(row_ids_in_order());
        assert!(same_name("abc", "abc"));
        assert!(!same_name("abc", "abd"));
        assert!(!same_name("ab", "abc"));
        assert!(!same_name("abc", "ab"));
    }

    #[test]
    fn every_row_id_names_its_row() {
        for (index, (id, row)) in RowId::ALL.iter().zip(&ROWS).enumerate() {
            assert_eq!(id.index(), index);
            assert_eq!(id.row(), row);
            for (value, label) in (0_u8..).zip(row.values) {
                assert_eq!(id.value_of(label), Some(value), "{label}");
            }
            assert_eq!(id.value_of("no_such_value"), None);
        }
        assert_eq!(EXIT_SLOTS, 4);
        assert_eq!(RANK_EDGES, 4);
        assert_eq!(TIME_WINDOWS, 6);
    }

    #[test]
    fn encode_and_decode_round_trip_on_every_radix_boundary() {
        // For every row: the cell with that row at its last value and every
        // other row at zero, and the cell with every row at its last value.
        let last: Coords = {
            let mut c = [0; ROW_COUNT];
            for (slot, row) in c.iter_mut().zip(&ROWS) {
                *slot = u8::try_from(row.values.len() - 1).unwrap();
            }
            c
        };
        assert_eq!(encode(&last).unwrap(), PAPER_FULL_CELLS - 1);
        assert_eq!(decode(PAPER_FULL_CELLS - 1).unwrap(), last);
        assert_eq!(encode(&[0; ROW_COUNT]).unwrap(), 0);
        assert_eq!(decode(0).unwrap(), [0; ROW_COUNT]);
        let mut place: u64 = 1;
        for row in (0..ROW_COUNT).rev() {
            let mut c = [0; ROW_COUNT];
            c[row] = 1;
            assert_eq!(encode(&c).unwrap(), place, "row {row}");
            assert_eq!(decode(place).unwrap(), c);
            // Just below the boundary: every less significant row at its last value.
            let mut below = [0; ROW_COUNT];
            below[row + 1..].copy_from_slice(&last[row + 1..]);
            assert_eq!(encode(&below).unwrap(), place - 1, "row {row}");
            assert_eq!(decode(place - 1).unwrap(), below);
            c[row] = last[row];
            let code = encode(&c).unwrap();
            assert_eq!(decode(code).unwrap(), c);
            place *= ROWS[row].values.len() as u64;
        }
        assert_eq!(place, PAPER_FULL_CELLS);
    }

    #[test]
    fn encode_and_decode_round_trip_on_100000_seeded_cells() {
        let mut rng = Seeded(0x5EED_F1B1);
        let mut previous = None;
        for _ in 0..100_000 {
            let cell = rng.cell();
            let code = encode(&cell).unwrap();
            assert!(code < PAPER_FULL_CELLS);
            assert_eq!(decode(code).unwrap(), cell);
            // Row 0 most significant: comparing codes compares cells lexically.
            if let Some((pc, pcode)) = previous {
                assert_eq!(cell.cmp(&pc), code.cmp(&pcode));
            }
            previous = Some((cell, code));
        }
    }

    #[test]
    fn every_refusal_is_named() {
        let mut cell = [0; ROW_COUNT];
        cell[3] = 7;
        let refusal = validate(&cell).unwrap_err();
        assert_eq!(
            refusal,
            CubeRefusal::ValueOutOfRange {
                row: "fib_zone",
                value: 7,
                values: 7
            }
        );
        assert_eq!(refusal.name(), "value_out_of_range");
        assert_eq!(encode(&cell), Err(refusal));
        assert_eq!(to_mask(&cell), Err(refusal));
        cell[3] = 0;
        cell[5] = OPEN;
        assert_eq!(
            validate(&cell),
            Err(CubeRefusal::OpenInCell {
                row: "level_relation"
            })
        );
        assert_eq!(validate(&cell).unwrap_err().name(), "open_in_cell");
        assert_eq!(
            decode(PAPER_FULL_CELLS),
            Err(CubeRefusal::CodeOutOfRange {
                code: PAPER_FULL_CELLS
            })
        );
        assert_eq!(decode(u64::MAX).unwrap_err().name(), "code_out_of_range");
    }

    #[test]
    fn every_mix_bucket_and_generation_refusal_is_named() {
        let mut mix = MixKey([OPEN; ROW_COUNT]);
        mix.0[7] = 0;
        mix.0[21] = 0;
        assert_eq!(
            validate_mix(&mix),
            Err(CubeRefusal::OpenPartitionRow { row: "timeframe" })
        );
        assert_eq!(validate_mix(&mix).unwrap_err().name(), "open_partition_row");
        assert_eq!(mix_mask(&mix).unwrap_err().name(), "open_partition_row");
        mix.0[23] = 4;
        assert_eq!(
            validate_mix(&mix),
            Err(CubeRefusal::ValueOutOfRange {
                row: "timeframe",
                value: 4,
                values: 4
            })
        );
        assert_eq!(
            tercile(1_001),
            Err(CubeRefusal::PermilleOutOfRange { permille: 1_001 })
        );
        assert_eq!(
            median(u16::MAX).unwrap_err().name(),
            "permille_out_of_range"
        );
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
        let names = [
            CubeRefusal::RankEdgesInvalid {
                edges: [0; RANK_EDGES],
            }
            .name(),
            CubeRefusal::TimeStartsInvalid {
                starts: [0; TIME_WINDOWS],
            }
            .name(),
            CubeRefusal::ExitSlotsEmpty.name(),
            CubeRefusal::ExitSlotRepeated { variant: 1 }.name(),
            CubeRefusal::SideOutsideSearch {
                search: "long",
                side: "short_with_loser",
            }
            .name(),
        ];
        assert_eq!(
            names,
            [
                "rank_edges_invalid",
                "time_starts_invalid",
                "exit_slots_empty",
                "exit_slot_repeated",
                "side_outside_search",
            ]
        );
        let shown = CubeRefusal::OpenInCell { row: "side" }.to_string();
        assert_eq!(
            shown,
            "fib cube refused (open_in_cell): OpenInCell { row: \"side\" }"
        );
        let boxed: Box<dyn std::error::Error> = Box::new(CubeRefusal::ExitSlotsEmpty);
        assert!(boxed.to_string().contains("exit_slots_empty"));
    }

    #[test]
    fn validate_mix_refuses_every_open_partition_row_and_accepts_free_rows() {
        let cell = [0; ROW_COUNT];
        let root = MixKey::root_of(&cell);
        assert_eq!(validate_mix(&root), Ok(()));
        for (index, row) in ROWS.iter().enumerate() {
            let mut mix = MixKey(cell);
            mix.0[index] = OPEN;
            let verdict = validate_mix(&mix);
            if row.kind == RowKind::Partition {
                assert_eq!(
                    verdict,
                    Err(CubeRefusal::OpenPartitionRow { row: row.name })
                );
            } else {
                assert_eq!(verdict, Ok(()), "{}", row.name);
            }
            mix.0[index] = u8::try_from(row.values.len()).unwrap();
            assert_eq!(
                validate_mix(&mix),
                Err(CubeRefusal::ValueOutOfRange {
                    row: row.name,
                    value: mix.0[index],
                    values: row.values.len()
                })
            );
        }
    }

    #[test]
    fn root_of_frees_every_row_but_the_three_partition_rows() {
        let mut rng = Seeded(7);
        let cell = rng.cell();
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
        // A rank_q1 cell is in a rank_q3 mix; a rank_q3 cell is never in a
        // rank_q2 mix.
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
        let mut rng = Seeded(0x00C0_FFEE);
        for _ in 0..20_000 {
            let cell = rng.cell();
            let mix = rng.mix();
            let cm = to_mask(&cell).unwrap();
            let mm = mix_mask(&mix).unwrap();
            assert_eq!(cm.hits(&mm), contains(&mix, &cell), "{cell:?} {mix:?}");
            assert_eq!(cm.popcount() as usize, ROW_COUNT + 3 - usize::from(cell[1]));
            // Every position used is below the 90 the schema owns.
            for bit in u32::try_from(VALUE_COUNT).unwrap()..ConditionMask::BITS {
                assert!(!cm.get(bit));
            }
        }
        // The thermometer: rank_q1 sets all four rank bits, rank_q4 one.
        let mut cell = [0; ROW_COUNT];
        let q1 = to_mask(&cell).unwrap();
        for bit in 4..8 {
            assert!(q1.get(bit));
        }
        cell[1] = 3;
        let q4 = to_mask(&cell).unwrap();
        assert_eq!((4..8).filter(|b| q4.get(*b)).count(), 1);
        assert!(q4.get(7));
        // The open mix requires nothing.
        let free = mix_mask(&MixKey::root_of(&cell)).unwrap();
        assert_eq!(free.popcount(), 3);
    }

    #[test]
    fn to_mask_is_anti_monotone_adding_a_required_value_only_removes_containment() {
        let mut rng = Seeded(0xA11CE);
        for _ in 0..5_000 {
            let cell = rng.cell();
            let mut mix = MixKey::root_of(&rng.cell());
            let cm = to_mask(&cell).unwrap();
            let mut was = cm.hits(&mix_mask(&mix).unwrap());
            for (index, row) in ROWS.iter().enumerate() {
                if mix.0[index] != OPEN {
                    continue;
                }
                mix.0[index] = rng.below(row.values.len());
                let now = cm.hits(&mix_mask(&mix).unwrap());
                assert!(
                    !now || was,
                    "a required value added containment at row {index}"
                );
                assert_eq!(now, contains(&mix, &cell));
                was = now;
            }
        }
    }

    #[test]
    fn tercile_and_median_take_a_permille_and_an_edge_goes_up() {
        assert_eq!(tercile(0), Ok(0));
        assert_eq!(tercile(333), Ok(0));
        assert_eq!(tercile(334), Ok(1));
        assert_eq!(tercile(666), Ok(1));
        assert_eq!(tercile(667), Ok(2));
        assert_eq!(tercile(1_000), Ok(2));
        assert_eq!(median(0), Ok(0));
        assert_eq!(median(499), Ok(0));
        assert_eq!(median(500), Ok(1), "on the edge goes up");
        assert_eq!(median(1_000), Ok(1));
        assert_eq!(equal_count(250, 4), Ok(1), "on the edge goes up");
        assert_eq!(equal_count(249, 4), Ok(0));
        assert_eq!(equal_count(750, 4), Ok(3));
        // Equal counts: over the 1,001 permilles each third holds a third.
        let mut counts = [0_u32; 3];
        for p in 0..=1_000 {
            counts[usize::from(tercile(p).unwrap())] += 1;
        }
        assert_eq!(counts, [334, 333, 334]);
    }

    #[test]
    fn rank_bucket_on_every_edge_of_a_synthetic_generation_and_beyond_the_widest() {
        let g = generation();
        let edges = g.parts().rank_edges;
        assert_eq!(edges, [3, 9, 20, 41]);
        assert_eq!(g.rank_bucket(0), None, "unranked");
        assert_eq!(rank_bucket(0, edges), None);
        for (bucket, edge) in (0_u8..).zip(edges) {
            assert_eq!(g.rank_bucket(edge), Some(bucket), "on the edge");
            assert_eq!(g.rank_bucket(edge + 1), (bucket < 3).then_some(bucket + 1));
        }
        assert_eq!(g.rank_bucket(1), Some(0));
        assert_eq!(g.rank_bucket(42), None, "beyond the widest edge");
        assert_eq!(g.rank_bucket(u16::MAX), None);
        // Equal edges are allowed: the narrowest bucket wins.
        assert_eq!(rank_bucket(5, [5, 5, 6, 7]), Some(0));
        assert_eq!(rank_bucket(6, [5, 5, 6, 7]), Some(2));
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
        // Exact ratios: a third of the range is nearer 382 than 236.
        assert_eq!(fib_zone_of(1, 3), Ok(2));
        // A tie in exact arithmetic that rounding would break: 309 / 1000 is the
        // 236 / 382 midpoint; 309_000 / 1_000_000 is the same tie.
        assert_eq!(fib_zone_of(309_000, 1_000_000), Ok(1));
        assert_eq!(fib_zone_of(309_001, 1_000_000), Ok(2));
        assert_eq!(fib_zone_of(i64::MAX, i64::MAX), Ok(6));
        assert_eq!(fib_zone_of(0, i64::MAX), Ok(0));
        assert_eq!(ZONE_RUNGS, [0, 236, 382, 500, 618, 786, 1_000]);
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
            Search::Long.validate_mix(&mix),
            Ok(()),
            "an open side is either side of the search"
        );
        mix.0[0] = 1;
        assert_eq!(
            Search::Long.validate_mix(&mix),
            Err(CubeRefusal::SideOutsideSearch {
                search: "long",
                side: "short_with_loser"
            })
        );
        assert_eq!(Search::Short.validate_mix(&mix), Ok(()));
        mix.0[0] = 3;
        assert_eq!(Search::Long.validate_mix(&mix), Ok(()));
        mix.0[23] = OPEN;
        assert_eq!(
            Search::Long.validate_mix(&mix),
            Err(CubeRefusal::OpenPartitionRow { row: "timeframe" })
        );
        assert_eq!(Search::Long.name(), "long");
        assert_eq!(Search::Short.name(), "short");
    }

    #[test]
    fn a_generation_refuses_every_malformed_part_by_name() {
        let good = *generation().parts();
        let mut p = good;
        p.rank_edges = [0, 9, 20, 41];
        assert_eq!(
            Generation::new(p),
            Err(CubeRefusal::RankEdgesInvalid {
                edges: p.rank_edges
            })
        );
        p.rank_edges = [3, 9, 8, 41];
        assert_eq!(Generation::new(p).unwrap_err().name(), "rank_edges_invalid");
        p = good;
        p.time_starts = [33_300, 36_000, 35_999, 43_200, 46_800, 50_400];
        assert_eq!(
            Generation::new(p),
            Err(CubeRefusal::TimeStartsInvalid {
                starts: p.time_starts
            })
        );
        p.time_starts = [33_300, 36_000, 39_600, 43_200, 46_800, 86_400];
        assert_eq!(
            Generation::new(p).unwrap_err().name(),
            "time_starts_invalid"
        );
        p.time_starts = [33_300, 36_000, 39_600, 43_200, 46_800, 86_399];
        assert!(
            Generation::new(p).is_ok(),
            "the last second of the day is inside it"
        );
        p = good;
        p.exit_slots = [EXIT_SLOT_UNBOUND; EXIT_SLOTS];
        assert_eq!(Generation::new(p), Err(CubeRefusal::ExitSlotsEmpty));
        p.exit_slots = [5, EXIT_SLOT_UNBOUND, 6, 5];
        assert_eq!(
            Generation::new(p),
            Err(CubeRefusal::ExitSlotRepeated { variant: 5 })
        );
        p.exit_slots = [5, EXIT_SLOT_UNBOUND, EXIT_SLOT_UNBOUND, 6];
        assert!(
            Generation::new(p).is_ok(),
            "two unbound slots are not a repeat"
        );
        // Equal starts and equal edges are allowed (generated from sparse data).
        p = good;
        p.rank_edges = [4, 4, 4, 4];
        p.time_starts = [33_300; TIME_WINDOWS];
        let g = Generation::new(p).unwrap();
        assert_eq!(
            g.time_window(33_300),
            Some(5),
            "the last window of equal starts"
        );
    }

    #[test]
    fn a_generation_reads_windows_and_exit_slots() {
        let g = generation();
        assert_eq!(g.schema_version(), FIB_CUBE_VERSION);
        assert_eq!(g.time_window(0), None);
        assert_eq!(g.time_window(33_299), None, "before window 1");
        assert_eq!(g.time_window(33_300), Some(0));
        assert_eq!(g.time_window(35_999), Some(0));
        assert_eq!(g.time_window(36_000), Some(1));
        assert_eq!(g.time_window(50_399), Some(4));
        assert_eq!(g.time_window(50_400), Some(5));
        assert_eq!(
            g.time_window(86_399),
            Some(5),
            "window 6 runs to the end of the day"
        );
        assert_eq!(g.exit_slot(11), Some(0));
        assert_eq!(g.exit_slot(12), Some(1));
        assert_eq!(g.exit_slot(14), Some(3));
        assert_eq!(g.exit_slot(13), None);
        assert_eq!(
            g.exit_slot(EXIT_SLOT_UNBOUND),
            None,
            "unbound is never a variant"
        );
    }

    fn hex(bytes: &[u8]) -> String {
        bytes.iter().fold(String::new(), |mut out, b| {
            use std::fmt::Write as _;
            let _ = write!(out, "{b:02x}");
            out
        })
    }

    #[test]
    fn the_generation_bytes_are_stable() {
        let g = generation();
        let bytes = g.bytes();
        assert_eq!(
            hex(&bytes),
            GENERATION_GOLDEN,
            "the canonical generation bytes changed: every FIB run identity re-keys"
        );
        // Every part moves the bytes.
        let mut p = *g.parts();
        p.search = Search::Short;
        assert_ne!(Generation::new(p).unwrap().bytes(), bytes);
        let mut p = *g.parts();
        p.rank_edges[3] = 42;
        assert_ne!(Generation::new(p).unwrap().bytes(), bytes);
        let mut p = *g.parts();
        p.time_starts[5] = 50_401;
        assert_ne!(Generation::new(p).unwrap().bytes(), bytes);
        let mut p = *g.parts();
        p.exit_set_version = 8;
        assert_ne!(Generation::new(p).unwrap().bytes(), bytes);
        let mut p = *g.parts();
        p.exit_set_digest[31] = 0;
        assert_ne!(Generation::new(p).unwrap().bytes(), bytes);
        let mut p = *g.parts();
        p.exit_slots[2] = 13;
        assert_ne!(Generation::new(p).unwrap().bytes(), bytes);
    }

    const GENERATION_GOLDEN: &str = "425458464942473101000000040000006c6f6e670400000003000900140029000600000014820000a08c0000b09a0000c0a80000d0b60000e0c4000007000000abababababababababababababababababababababababababababababababab040000000b0000000c000000ffffffff0e000000";

    #[test]
    fn the_descriptor_names_every_row_kind_value_and_rung() {
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
            for value in row.values {
                assert!(text.contains(value));
            }
        }
        assert_eq!(d.last(), Some(&OPEN));
        assert_eq!((d.len(), fnv1a(&d)), DESCRIPTOR_GOLDEN);
        assert_eq!(RowKind::Categorical.name(), "categorical");
        assert_eq!(RowKind::Cumulative.name(), "cumulative");
        assert_eq!(RowKind::Partition.name(), "partition");
    }

    /// The descriptor's length and FNV-1a 64 checksum (this crate cannot hash with
    /// BLAKE3; `runner::fib::KNOWN_SCHEMAS` pins that digest).
    const DESCRIPTOR_GOLDEN: (usize, u64) = (1_765, 16_192_446_222_292_617_295);

    fn fnv1a(bytes: &[u8]) -> u64 {
        bytes.iter().fold(0xCBF2_9CE4_8422_2325, |h, b| {
            (h ^ u64::from(*b)).wrapping_mul(0x0100_0000_01B3)
        })
    }

    #[test]
    fn fixed_numbers_names_every_literal_of_this_file() {
        let source = include_str!("fib_cube.rs");
        let shipping = shipping_part(source);
        assert!(
            shipping.len() < source.len(),
            "the test module marker was found"
        );
        let literals = numeric_literals(shipping);
        assert!(
            literals.len() > 50,
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
        // The ledger itself: unique names, a decision id from FIB-1's block, and
        // the coarse counts equal to the rows.
        let mut names: Vec<&str> = FIXED_NUMBERS.iter().map(|(n, _, _, _)| *n).collect();
        names.sort_unstable();
        let before = names.len();
        names.dedup();
        assert_eq!(names.len(), before, "a ledger name is repeated");
        for (name, _, _, decision) in FIXED_NUMBERS {
            let n: u32 = decision.strip_prefix("D-").unwrap().parse().unwrap();
            assert!((5300..=5319).contains(&n), "{name}: {decision}");
        }
        for row in &ROWS {
            let key = format!("values:{}", row.name);
            let entry = FIXED_NUMBERS.iter().find(|(n, _, _, _)| *n == key).unwrap();
            assert_eq!(entry.1, row.values.len() as u64, "{key}");
            assert_eq!(entry.2, Category::CoarseBucketCount);
        }
        // The 0.2% minimum stop is gone (operator 16 Oct 13:07 IST), and the
        // trade count is 500 a year (13:57 IST).
        assert!(
            FIXED_NUMBERS
                .iter()
                .all(|(n, _, _, _)| !n.contains("stop_min"))
        );
        assert_eq!(GATE_TRADES_PER_YEAR_MIN, 500);
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
            Category::CoarseBucketCount,
            Category::OperatorReportColumn,
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
                "coarse_bucket_count",
                "operator_report_column",
            ]
        );
        for category in all {
            assert!(
                FIXED_NUMBERS.iter().any(|(_, _, c, _)| *c == category),
                "{category:?}"
            );
        }
        assert_eq!(SESSION_GRID_ANCHOR_SECS, 9 * 3_600 + 15 * 60);
        assert_eq!(SESSION_OWN_EXIT_EARLY_SECS, 15 * 3_600 + 7 * 60);
        assert_eq!(SESSION_OWN_EXIT_LATE_SECS, 15 * 3_600 + 8 * 60);
        assert_eq!(SESSION_FORCED_WINDOW_START_SECS, 15 * 3_600 + 9 * 60);
        assert_eq!(SESSION_FORCED_WINDOW_END_SECS, 15 * 3_600 + 10 * 60);
        assert_eq!(SECONDS_PER_DAY, 24 * 3_600);
    }

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
