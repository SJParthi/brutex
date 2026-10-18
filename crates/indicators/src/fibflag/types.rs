//! Shared value types of the Fib detector, with no behaviour (FIB-1; D-6300).
//!
//! APPEND-ONLY. Later packages only ADD fields and variants, in dependency
//! order: FIB-3 (the base measurements and shape scores), then FIB-4 (the
//! trigger, stop, entry and room). No type here has a `Default`. Prices are
//! paisa (`i64`); instants are microseconds since the Unix epoch, UTC, like
//! [`crate::Candle`]'s.

use crate::Candle;

/// The direction of a trade: the one search a candidate belongs to (operator
/// 16 Oct 16:04 IST). The same type as `vocab::fib_cube::Search`, so the two
/// can never disagree.
pub type TradeDir = vocab::fib_cube::Search;

/// One candle of one width, folded from Zerodha one-minute bars anchored at
/// 09:15 (`pull::fold`), as the detector is fed it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FibCandle {
    /// The candle: open instant, prices in paisa, volume.
    pub candle: Candle,
    /// The width, in whole minutes.
    pub tf_minutes: u16,
    /// Whether every minute of the bucket was inside the session: a short last
    /// bucket is partial and is never a decision candle.
    pub complete: bool,
    /// How many one-minute bars of the bucket were missing from the store.
    pub missing_minutes: u16,
    /// The close instant (the bucket's right edge).
    pub close_ts_micros: i64,
}

/// The stock's own move so far today, as the side rows read it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MoveSide {
    /// Moved up so far.
    Gainer,
    /// Moved down so far.
    Loser,
    /// Not moved.
    Flat,
}

/// The kind of base. The declaration order is the `base_kind` row's value
/// order, proved by `the_kinds_and_styles_are_the_cube_s`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum BaseKind {
    /// A micro base of two candles.
    Micro2,
    /// A micro base of three candles.
    Micro3,
    /// A rest of more candles.
    Rest,
}

impl BaseKind {
    /// The kind's value index in the `base_kind` row.
    #[must_use]
    pub const fn cube_value(self) -> u8 {
        self as u8
    }
}

/// The entry style. The declaration order is the entry-style names' order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum EntryStyle {
    /// Enter at the trigger candle's close.
    Close,
    /// Enter on a break of the trigger candle's extreme.
    Break,
    /// Enter on a retest of the base edge.
    Retest,
}

impl EntryStyle {
    /// The style's index among the entry-style names.
    #[must_use]
    pub const fn name_index(self) -> u8 {
        self as u8
    }
}

/// What the detector is run with. No `Default`: FIB-3 and FIB-4 add every
/// generated value they read (rest lengths, touch tolerances, the stop set),
/// each from the run's generation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct DetectorParams {
    /// The width, in whole minutes.
    pub tf_minutes: u16,
    /// The generation version the values come from.
    pub generation_version: u32,
    /// FIB-3 (D-6342, FM-F023): the ring's capacity, in candles of this width:
    /// the most candles of this width any session of the run's span holds,
    /// from the calendar's session length. Allocated once per worker; a
    /// longer session is refused by name.
    pub ring_capacity: u16,
    /// FIB-3 (D-6344): the most generated rest lengths a long day may pass,
    /// from the run's generation (17:15 IST item 2).
    pub long_rest_lengths: u16,
    /// FIB-3 (D-6344): the most generated rest lengths a short day may pass.
    pub short_rest_lengths: u16,
    /// FIB-3 (D-6342): the most touch levels `q` the long direction's
    /// generation holds (17:15 IST item 1).
    pub long_touch_levels: u16,
    /// FIB-3 (D-6342): the most touch levels `q` of the short direction.
    pub short_touch_levels: u16,
    /// FIB-3 (D-6345, FM-F024): whether a zero-width base is emitted with
    /// its stated limits, or refused (the labelled other reading).
    pub zero_width: ZeroWidthReading,
}

/// What the detector does with a base whose width is zero (FM-F024, D-6345).
/// No `Default`: the reading is named in the run's identity.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ZeroWidthReading {
    /// Emitted, flagged `zero_width`, with the stated limits (flat 1000,
    /// tight closes 1000, partial U 1000, V bottom 0, shakeout 0), proved in
    /// `crates/indicators/tests/fib_base.rs`
    /// (`three_one_price_candles_give_flat_1000_and_set_the_flag`).
    Emit,
    /// Refused by name, `zero_width_base`: the labelled variant.
    Refuse,
}

impl ZeroWidthReading {
    /// The reading's canonical name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Emit => "emit_with_limits",
            Self::Refuse => "refuse_flagged",
        }
    }
}

/// One base as the detector emits it at a candle close.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct BaseSnapshot {
    /// The kind of base.
    pub kind: BaseKind,
    /// The direction it is measured for (only the geometry is mirrored).
    pub direction: TradeDir,
    /// The base's high, in paisa.
    pub high: i64,
    /// The base's low, in paisa.
    pub low: i64,
    /// How many candles the base spans.
    pub candles: u16,
    /// The close instant of the base's first candle.
    pub first_close_ts_micros: i64,
    /// The close instant of the base's last candle.
    pub last_close_ts_micros: i64,
    /// FIB-3: the day index of the base's first candle (0 is the day's first
    /// accepted candle of this width).
    pub first_index: u32,
    /// FIB-3: the day index of the base's last candle, the candle it ended at.
    pub last_index: u32,
    /// FIB-3 (D-6341, D-6342): every raw measurement, in this direction's
    /// view. No percentile is applied here: the caller owns each stock's own
    /// past.
    pub measures: super::measure::BaseMeasures,
    /// FIB-3 (D-6343): the shape scores of the base's kind, in permille of
    /// its width.
    pub shape: super::shape::ShapeScores,
    /// FIB-3 (D-6341, D-6346): the ladder the base was measured against.
    pub ladder: super::extremes::LadderStamp,
    /// FIB-3 (D-6342, FM-F015): whether this direction had a touch tolerance
    /// today. When `false` the touch counts are absent and the base is
    /// counted `no_touch_tolerance`; the caller refuses it before cubing.
    pub touches_known: bool,
}

/// One candidate: a base, a trigger and a plan, landing in one cell.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Candidate {
    /// The stock's slot in the day's point-in-time pool.
    pub stock_slot: u16,
    /// The IST day number ([`crate::ist_day`]).
    pub ist_day: i64,
    /// The width, in whole minutes.
    pub tf_minutes: u16,
    /// The decision candle's close instant.
    pub decision_close_ts_micros: i64,
    /// The kind of base.
    pub kind: BaseKind,
    /// The entry style.
    pub style: EntryStyle,
    /// The trade's direction.
    pub direction: TradeDir,
    /// The base.
    pub base: BaseSnapshot,
}

/// Why the detector refused a candle, counted by name. APPEND-ONLY: a new
/// reason is a new variant at the end.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Refusal {
    /// A partial bucket: never a decision candle.
    PartialCandle,
    /// A bucket with missing one-minute bars.
    MissingMinutes,
    /// A candle that does not close after the one before it.
    OutOfOrder,
    /// A width beyond `vocab::fib_cube::max_width_minutes`.
    WidthBeyondTop,
    /// FIB-3: a record `crate::Candle::check_evaluable` refuses (`high <
    /// low`, a price at or below zero, ...): never folded.
    CorruptCandle,
    /// FIB-3: a candle of another width than the detector's.
    WidthMismatch,
    /// FIB-3: a candle of an IST day the detector was not begun for.
    DayNotBegun,
    /// FIB-3 (FM-F023): a session with more candles of this width than the
    /// ring was allocated for, or a candle past the day's session length.
    SessionLongerThanRing,
    /// FIB-3: a day begun without a positive observed tick.
    NoObservedTick,
    /// FIB-3: a day's generated lists longer than the run pre-sized.
    GenerationBeyondCapacity,
    /// FIB-3: a day's generated rest lengths not strictly ascending.
    RestLengthsNotAscending,
    /// FIB-3: a micro base whose candles are not contiguous on the width's
    /// 09:15-anchored grid.
    GapInBase,
    /// FIB-3 (D-6342): a day range whose adjacent rungs are not one observed
    /// tick apart: no ladder, so no base is measured at this candle.
    NoLadder,
    /// FIB-3 (FM-F015): a base of a direction with no touch tolerance today.
    NoTouchTolerance,
    /// FIB-3 (FM-F024): a zero-width base under the refusing reading.
    ZeroWidthBase,
}

impl Refusal {
    /// Every reason, in declaration order.
    pub const ALL: &[Self] = &[
        Self::PartialCandle,
        Self::MissingMinutes,
        Self::OutOfOrder,
        Self::WidthBeyondTop,
        Self::CorruptCandle,
        Self::WidthMismatch,
        Self::DayNotBegun,
        Self::SessionLongerThanRing,
        Self::NoObservedTick,
        Self::GenerationBeyondCapacity,
        Self::RestLengthsNotAscending,
        Self::GapInBase,
        Self::NoLadder,
        Self::NoTouchTolerance,
        Self::ZeroWidthBase,
    ];

    /// The reason's counted name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::PartialCandle => "partial_candle",
            Self::MissingMinutes => "missing_minutes",
            Self::OutOfOrder => "out_of_order",
            Self::WidthBeyondTop => "width_beyond_top",
            Self::CorruptCandle => "corrupt_candle",
            Self::WidthMismatch => "width_mismatch",
            Self::DayNotBegun => "day_not_begun",
            Self::SessionLongerThanRing => "session_longer_than_ring",
            Self::NoObservedTick => "no_observed_tick",
            Self::GenerationBeyondCapacity => "generation_beyond_capacity",
            Self::RestLengthsNotAscending => "rest_lengths_not_ascending",
            Self::GapInBase => "gap_in_base",
            Self::NoLadder => "no_ladder",
            Self::NoTouchTolerance => "no_touch_tolerance",
            Self::ZeroWidthBase => "zero_width_base",
        }
    }
}

/// How many refusal reasons there are.
pub const REFUSALS: usize = Refusal::ALL.len();

/// One counter per refusal reason: plain fields, no static and no atomic
/// (gate 17).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct RefusalCounts {
    /// The counts, by reason.
    counts: [u64; REFUSALS],
}

impl RefusalCounts {
    /// Every counter at zero.
    #[must_use]
    pub const fn zero() -> Self {
        Self {
            counts: [0; REFUSALS],
        }
    }

    /// Counts one refusal.
    pub fn record(&mut self, refusal: Refusal) {
        for (count, reason) in self.counts.iter_mut().zip(Refusal::ALL) {
            if *reason == refusal {
                *count = count.saturating_add(1);
            }
        }
    }

    /// How many refusals of one reason were counted.
    #[must_use]
    pub fn get(&self, refusal: Refusal) -> u64 {
        self.counts
            .iter()
            .zip(Refusal::ALL)
            .filter(|(_, reason)| **reason == refusal)
            .map(|(count, _)| *count)
            .sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vocab::fib_cube::{ENTRY_STYLE_ROW, RowId, Search};

    #[test]
    fn the_kinds_and_styles_are_the_cube_s() {
        for (kind, label) in [
            (BaseKind::Micro2, "micro2"),
            (BaseKind::Micro3, "micro3"),
            (BaseKind::Rest, "rest"),
        ] {
            assert_eq!(RowId::BaseKind.value_of(label), Some(kind.cube_value()));
        }
        for (style, label) in [
            (EntryStyle::Close, "close"),
            (EntryStyle::Break, "break"),
            (EntryStyle::Retest, "retest"),
        ] {
            assert_eq!(
                ENTRY_STYLE_ROW.labels.get(usize::from(style.name_index())),
                Some(&label)
            );
        }
        let direction: TradeDir = Search::Short;
        assert_eq!(direction.name(), "short");
    }

    #[test]
    fn every_refusal_is_counted_by_name() {
        let mut counts = RefusalCounts::zero();
        counts.record(Refusal::MissingMinutes);
        counts.record(Refusal::MissingMinutes);
        counts.record(Refusal::WidthBeyondTop);
        assert_eq!(counts.get(Refusal::PartialCandle), 0);
        assert_eq!(counts.get(Refusal::MissingMinutes), 2);
        assert_eq!(counts.get(Refusal::OutOfOrder), 0);
        assert_eq!(counts.get(Refusal::WidthBeyondTop), 1);
        let names: Vec<&str> = Refusal::ALL.iter().map(|r| r.name()).collect();
        assert_eq!(
            names,
            [
                "partial_candle",
                "missing_minutes",
                "out_of_order",
                "width_beyond_top",
                // FIB-3, appended (D-6347).
                "corrupt_candle",
                "width_mismatch",
                "day_not_begun",
                "session_longer_than_ring",
                "no_observed_tick",
                "generation_beyond_capacity",
                "rest_lengths_not_ascending",
                "gap_in_base",
                "no_ladder",
                "no_touch_tolerance",
                "zero_width_base",
            ]
        );
        assert_eq!(REFUSALS, 15);
        assert_eq!(ZeroWidthReading::Emit.name(), "emit_with_limits");
        assert_eq!(ZeroWidthReading::Refuse.name(), "refuse_flagged");
        let mut full = RefusalCounts {
            counts: [u64::MAX; REFUSALS],
        };
        full.record(Refusal::OutOfOrder);
        assert_eq!(full.get(Refusal::OutOfOrder), u64::MAX, "saturates");
    }
}
