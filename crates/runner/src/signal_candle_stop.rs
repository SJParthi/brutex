//! One causal, index-only stop rule, distinct from every saved exit-grid policy.
//!
//! A completed signal candle fixes the absolute stop. The immediate next
//! one-minute open is the entry; the stop or the unique accepted 15:09 close
//! ends the position. There is no target, trail or holding-horizon alternative.
//! The runner binds caller-supplied source evidence; the stored caller must
//! still supply its calendar/checksum authority and existing statistical checks.
//!
//! Preparation is O(signal + execution + reference rows), with O(rows) retained
//! alignment/acceptance facts. One expression is O(signal rows + held minutes),
//! with fixed trading state and bounded-by-admission event/trade/day output.
//! Prices are integer paisa. Printed stop brackets are bounds, not tick fills.

use brutex_core::blake3::Hasher;
use indicators::Candle;
use indicators::column::{Column, Sourced};

use crate::excursion::Side;
use crate::exit_grid_policy::{ExecutionSeriesV1, column_digest_v1};
use crate::expression::{Expression, Summary as TruthSummary, Truth};
use crate::identity::{DailyReferenceBinding, Direction, Params, Run};
use crate::research_family::ResearchFamilyV1;
use crate::trade::SliceFacts;

const MINUTE: i64 = 60_000_000;
const DAY: i64 = 86_400_000_000;
const IST: i64 = indicators::IST_OFFSET_MICROS;

/// The only execution rule defined by this format.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Policy;
impl Policy {
    /// Completed-candle stop, printed-open entry and exact 15:10 close.
    pub const V1: Self = Self;
    /// Exact policy record width.
    pub const BYTE_LEN: usize = 80;
    /// Canonical independent rule bytes; old exit-grid bytes are unchanged.
    #[must_use]
    pub fn canonical_bytes(self) -> [u8; Self::BYTE_LEN] {
        encode(
            *b"BRSCPO01",
            [
                1,
                1,
                1,
                bits(crate::outcome::FORCED_EXIT_MINUTE),
                1,
                1,
                1,
                1,
                1,
            ],
        )
    }
    /// Exact policy identity.
    #[must_use]
    pub fn digest(self) -> [u8; 32] {
        hash(&self.canonical_bytes())
    }
    /// Reopen only this exact policy, including every reserved choice.
    /// # Errors
    /// Refuses any changed width or byte.
    pub fn decode(bytes: &[u8]) -> Result<Self, Error> {
        (bytes == Self::V1.canonical_bytes())
            .then_some(Self)
            .ok_or(Error::Encoding)
    }
}

/// Immutable source inputs; supplied masks are bound, never treated as receipts.
pub struct Source<'a> {
    /// Canonical instrument/feed/commit/calendar and actual execution OHLCV.
    pub series: ExecutionSeriesV1<'a>,
    /// Original rung OHLCV, before any projection to entry coordinates.
    pub signal_bars: &'a [Candle],
    /// Original causal evaluator column over exactly those signal bars.
    pub signal_column: &'a Column,
    /// Complete one-minute reference stream containing the execution subspan.
    pub minute_context: &'a [Candle],
    /// Exact daily/reference/calendar identity terms supplied by the loader.
    pub daily_reference: DailyReferenceBinding<'a>,
    /// One of the eight canonical intraday timeframe labels.
    pub timeframe: &'a str,
    /// Original runtime search/catalog parameters; no hidden default.
    pub params: Params,
    /// Inclusive loader-attested calendar start, including closed boundary days.
    pub first_day: i64,
    /// Inclusive loader-attested calendar end; absent dates are not fabricated.
    pub last_day: i64,
}

/// A prepared source whose alignment and execution verdict cannot be replaced.
pub struct Prepared<'a> {
    policy: Policy,
    source: Source<'a>,
    family: ResearchFamilyV1,
    alignment: Vec<Option<usize>>,
    facts: SliceFacts,
    duplicates: Vec<bool>,
    periods: Vec<Period>,
    period_of: Vec<usize>,
    data_digest: [u8; 32],
    source_id: [u8; 32],
    signal_length: i64,
}

/// A complete construction/evaluation refusal; no partial success is returned.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    /// Wrong instrument, direction or intraday timeframe.
    Scope,
    /// Missing, foreign, out-of-order or malformed source identity/column.
    Source,
    /// A fixed-width integer operation overflowed.
    Arithmetic,
    /// The explicit evidence admission cannot hold this complete result.
    Bound,
    /// An additional output allocation failed.
    Allocation,
    /// Invalid version, width, discriminant or internally inconsistent bytes.
    Encoding,
}
impl std::fmt::Display for Error {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        out.write_str(match self {
            Self::Scope => "signal-candle stop requires exact NSE NIFTY/BANKNIFTY, a directed program and an intraday timeframe",
            Self::Source => "signal-candle stop source identity, order, reference or checked column refused",
            Self::Arithmetic => "signal-candle stop exact arithmetic overflowed",
            Self::Bound => "signal-candle stop complete evidence exceeds explicit admission",
            Self::Allocation => "signal-candle stop evidence allocation refused",
            Self::Encoding => "signal-candle stop canonical record refused",
        })
    }
}
impl std::error::Error for Error {}

/// Why a definite signal did not yield a priceable round trip.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u64)]
pub enum Reason {
    /// No refusal: the event points to its exact trade.
    None = 0,
    /// The immediate next minute is absent before the 15:10 boundary; no later
    /// entry is substituted. A real one-minute data hole.
    Unreachable = 1,
    /// Its entry would open at or after the 15:10 boundary, whether or not that
    /// minute exists: the session's last bucket lands here (D-0603).
    TooLate = 2,
    /// The actual entry open is at/beyond its stop, so no position is opened.
    GapInvalid = 3,
    /// Another position or conservative refusal hold owns this minute.
    WhileOpen = 4,
    /// The immediate execution record is refused or duplicated.
    EntryRefused = 5,
    /// A held minute is refused, duplicated or missing before exit is known.
    PathRefused = 6,
    /// No unique accepted exact 15:09 closing record proves forced exit.
    ClosingMinute = 7,
}
impl Reason {
    /// Stable human-readable reason, not an inferred completion state.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::None => "priced",
            Self::Unreachable => "immediate_entry_minute_missing",
            Self::TooLate => "entry_at_or_after_1510",
            Self::GapInvalid => "entry_open_at_or_beyond_stop",
            Self::WhileOpen => "position_still_occupied",
            Self::EntryRefused => "entry_minute_refused_or_duplicated",
            Self::PathRefused => "held_path_missing_refused_or_duplicated",
            Self::ClosingMinute => "unique_accepted_1509_close_missing",
        }
    }
    fn decode(value: u64) -> Result<Self, Error> {
        match value {
            0 => Ok(Self::None),
            1 => Ok(Self::Unreachable),
            2 => Ok(Self::TooLate),
            3 => Ok(Self::GapInvalid),
            4 => Ok(Self::WhileOpen),
            5 => Ok(Self::EntryRefused),
            6 => Ok(Self::PathRefused),
            7 => Ok(Self::ClosingMinute),
            _ => Err(Error::Encoding),
        }
    }
}

/// The sole order or exact closing clock that ended a trade.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u64)]
pub enum ExitReason {
    /// Fixed absolute completed-candle low/high stop.
    Stop = 1,
    /// Exact closing print at 15:10, not a nearest available minute.
    Forced1510 = 2,
}
impl ExitReason {
    /// Stable display name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Stop => "signal_candle_stop",
            Self::Forced1510 => "forced_1510_close",
        }
    }
}

/// Complete exact round-trip evidence. Indexes are portable fixed-width values.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Trade {
    /// Original coarse signal candle index; never its projected entry index.
    pub signal_bar: u64,
    /// Actual one-minute entry index.
    pub entry_bar: u64,
    /// Actual one-minute exit index.
    pub exit_bar: u64,
    /// Original signal candle open timestamp.
    pub signal_micros: i64,
    /// Completed signal candle boundary, also the exact entry timestamp.
    pub signal_close_micros: i64,
    /// Actual entry open timestamp.
    pub entry_micros: i64,
    /// Exit bar's one-minute open timestamp.
    pub exit_bar_micros: i64,
    /// Earliest observed exit instant: minute open for stops, 15:10 for close.
    pub exit_from_micros: i64,
    /// Exclusive stop-minute end, or the same exact 15:10 closing instant.
    pub exit_until_micros: i64,
    /// Frozen absolute signal-candle stop in paisa.
    pub stop_paisa: i64,
    /// Actual printed entry open, identical in both readings.
    pub entry_paisa: i64,
    /// Optimistic observed stop trigger/gap open, or exact forced close.
    pub optimistic_exit_paisa: i64,
    /// Adverse printed stop extreme, or exact forced close.
    pub pessimistic_exit_paisa: i64,
    /// Gross per-unit optimistic paisa, excluding costs.
    pub optimistic_paisa: i64,
    /// Gross per-unit pessimistic paisa, excluding costs.
    pub pessimistic_paisa: i64,
    /// Maximum adverse printed excursion through the exit minute, in paisa.
    pub adverse_paisa: i64,
    /// Guaranteed favourable excursion; a stop minute contributes only its open.
    pub favourable_paisa: i64,
    /// Number of occupied one-minute bars, including entry and exit.
    pub holding_minutes: u64,
    /// Exact cause of exit, with no target/trail/horizon variants.
    pub exit_reason: ExitReason,
    /// A previously resting stop was already marketable at the opening print.
    pub gapped: bool,
}
impl Trade {
    /// Exact record width, independent of Rust layout or pointer width.
    pub const BYTE_LEN: usize = 168;
    /// Fixed canonical evidence bytes.
    #[must_use]
    pub fn canonical_bytes(self) -> [u8; Self::BYTE_LEN] {
        encode(
            *b"BRSCTR01",
            [
                self.signal_bar,
                self.entry_bar,
                self.exit_bar,
                bits(self.signal_micros),
                bits(self.signal_close_micros),
                bits(self.entry_micros),
                bits(self.exit_bar_micros),
                bits(self.exit_from_micros),
                bits(self.exit_until_micros),
                bits(self.stop_paisa),
                bits(self.entry_paisa),
                bits(self.optimistic_exit_paisa),
                bits(self.pessimistic_exit_paisa),
                bits(self.optimistic_paisa),
                bits(self.pessimistic_paisa),
                bits(self.adverse_paisa),
                bits(self.favourable_paisa),
                self.holding_minutes,
                self.exit_reason as u64,
                u64::from(self.gapped),
            ],
        )
    }
    /// Decode fixed evidence without minting an evaluated-source capability.
    /// # Errors
    /// Refuses invalid width, order, bounds, time window or exit kind.
    pub fn decode(bytes: &[u8]) -> Result<Self, Error> {
        let [
            signal_bar,
            entry_bar,
            exit_bar,
            signal,
            close,
            entry,
            exit_bar_time,
            from,
            until,
            stop,
            price,
            best_exit,
            worst_exit,
            best,
            worst,
            adverse,
            favourable,
            holding,
            reason,
            gapped,
        ] = decode(bytes, *b"BRSCTR01")?;
        let row = Self {
            signal_bar,
            entry_bar,
            exit_bar,
            signal_micros: signed(signal),
            signal_close_micros: signed(close),
            entry_micros: signed(entry),
            exit_bar_micros: signed(exit_bar_time),
            exit_from_micros: signed(from),
            exit_until_micros: signed(until),
            stop_paisa: signed(stop),
            entry_paisa: signed(price),
            optimistic_exit_paisa: signed(best_exit),
            pessimistic_exit_paisa: signed(worst_exit),
            optimistic_paisa: signed(best),
            pessimistic_paisa: signed(worst),
            adverse_paisa: signed(adverse),
            favourable_paisa: signed(favourable),
            holding_minutes: holding,
            exit_reason: match reason {
                1 => ExitReason::Stop,
                2 => ExitReason::Forced1510,
                _ => return Err(Error::Encoding),
            },
            gapped: gapped == 1,
        };
        if gapped > 1 {
            return Err(Error::Encoding);
        }
        row.validate()?;
        Ok(row)
    }
    fn validate(self) -> Result<(), Error> {
        let row = self;
        let end = row
            .exit_bar_micros
            .checked_add(MINUTE)
            .ok_or(Error::Encoding)?;
        let same_day = ist_day(row.entry_micros)? == ist_day(row.exit_bar_micros)?;
        if row.entry_bar > row.exit_bar
            || row.signal_micros >= row.entry_micros
            || row.signal_close_micros != row.entry_micros
            || row.entry_micros > row.exit_bar_micros
            || !same_day
            || row.stop_paisa <= 0
            || row.entry_paisa <= 0
            || row.optimistic_exit_paisa <= 0
            || row.pessimistic_exit_paisa <= 0
            || row.pessimistic_paisa > row.optimistic_paisa
            || row.adverse_paisa < 0
            || row.favourable_paisa < 0
            || row.holding_minutes
                != u64::try_from((end - row.entry_micros) / MINUTE).map_err(|_| Error::Encoding)?
        {
            return Err(Error::Encoding);
        }
        match row.exit_reason {
            ExitReason::Stop
                if row.exit_from_micros == row.exit_bar_micros && row.exit_until_micros == end => {}
            ExitReason::Forced1510
                if row.exit_from_micros == end
                    && row.exit_until_micros == end
                    && end
                        == forced_open(row.entry_micros)?
                            .checked_add(MINUTE)
                            .ok_or(Error::Encoding)?
                    && row.optimistic_exit_paisa == row.pessimistic_exit_paisa
                    && !row.gapped => {}
            _ => return Err(Error::Encoding),
        }
        let pnl = match row.entry_paisa.cmp(&row.stop_paisa) {
            std::cmp::Ordering::Greater => (
                row.optimistic_exit_paisa.checked_sub(row.entry_paisa),
                row.pessimistic_exit_paisa.checked_sub(row.entry_paisa),
            ),
            std::cmp::Ordering::Less => (
                row.entry_paisa.checked_sub(row.optimistic_exit_paisa),
                row.entry_paisa.checked_sub(row.pessimistic_exit_paisa),
            ),
            std::cmp::Ordering::Equal => return Err(Error::Encoding),
        };
        if pnl != (Some(row.optimistic_paisa), Some(row.pessimistic_paisa))
            || row.exit_reason == ExitReason::Stop && row.optimistic_paisa >= 0
            || row.entry_micros.rem_euclid(MINUTE) != 0
            || row.exit_bar_micros.rem_euclid(MINUTE) != 0
            || row.exit_bar_micros > forced_open(row.entry_micros)?
            || row
                .exit_bar
                .checked_sub(row.entry_bar)
                .and_then(|n| n.checked_add(1))
                != Some(row.holding_minutes)
        {
            return Err(Error::Encoding);
        }
        Ok(())
    }
    /// Shared numerical row shape only; this does not mint a fixed-grid proof.
    /// # Errors
    /// Refuses platform index or exact ppm conversion overflow.
    pub fn statistical_row(self) -> Result<crate::grid::TradeRow, Error> {
        Ok(crate::grid::TradeRow {
            signal_bar: usize::try_from(self.signal_bar).map_err(|_| Error::Arithmetic)?,
            entry_bar: usize::try_from(self.entry_bar).map_err(|_| Error::Arithmetic)?,
            exit_bar: usize::try_from(self.exit_bar).map_err(|_| Error::Arithmetic)?,
            best: self.optimistic_paisa,
            worst: self.pessimistic_paisa,
            entry_micros: self.entry_micros,
            exit_micros: self.exit_bar_micros,
            adverse: ppm(self.adverse_paisa, self.entry_paisa)?,
            adverse_paisa: self.adverse_paisa,
            favourable: ppm(self.favourable_paisa, self.entry_paisa)?,
            favourable_paisa: self.favourable_paisa,
        })
    }
}

/// One definite signal, including every skipped or unpriceable position.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Event {
    /// Original source candle index.
    pub signal_bar: u64,
    /// Original source open timestamp.
    pub signal_micros: i64,
    /// Required immediate entry timestamp.
    pub signal_close_micros: i64,
    /// Frozen signal-candle boundary in paisa.
    pub stop_paisa: i64,
    /// Stable disposition reason; None requires an exact trade index.
    pub reason: Reason,
    /// Exact immediate entry coordinate, when present.
    pub entry_bar: Option<u64>,
    /// Exact immediate entry timestamp, paired with the entry coordinate.
    pub entry_micros: Option<i64>,
    /// Inclusive occupied minute's open; refused paths reserve through15:09.
    pub occupied_through_micros: Option<i64>,
    /// Exact index into this evaluation's trade rows, never a display guess.
    pub trade_index: Option<u64>,
}
impl Event {
    /// Fixed event record width.
    pub const BYTE_LEN: usize = 112;
    /// Canonical optional coordinates use separate presence bits.
    #[must_use]
    pub fn canonical_bytes(self) -> [u8; Self::BYTE_LEN] {
        encode(
            *b"BRSCEV01",
            [
                self.signal_bar,
                bits(self.signal_micros),
                bits(self.signal_close_micros),
                bits(self.stop_paisa),
                self.reason as u64,
                1,
                u64::from(self.entry_bar.is_some()),
                self.entry_bar.unwrap_or(0),
                bits(self.entry_micros.unwrap_or(0)),
                u64::from(self.occupied_through_micros.is_some()),
                bits(self.occupied_through_micros.unwrap_or(0)),
                u64::from(self.trade_index.is_some()),
                self.trade_index.unwrap_or(0),
            ],
        )
    }
    /// Decode evidence; evaluation identity is a separate authority.
    /// # Errors
    /// Refuses inconsistent option flags, disposition or timestamps.
    pub fn decode(bytes: &[u8]) -> Result<Self, Error> {
        let [
            signal_bar,
            signal,
            close,
            stop,
            reason,
            version,
            has_entry,
            entry_bar,
            entry,
            has_hold,
            hold,
            has_trade,
            trade,
        ] = decode(bytes, *b"BRSCEV01")?;
        let row = Self {
            signal_bar,
            signal_micros: signed(signal),
            signal_close_micros: signed(close),
            stop_paisa: signed(stop),
            reason: Reason::decode(reason)?,
            entry_bar: option(has_entry, entry_bar)?,
            entry_micros: option(has_entry, entry)?.map(signed),
            occupied_through_micros: option(has_hold, hold)?.map(signed),
            trade_index: option(has_trade, trade)?,
        };
        if version != 1
            || row.signal_micros >= row.signal_close_micros
            || row.stop_paisa <= 0
            || row
                .entry_micros
                .is_some_and(|value| value != row.signal_close_micros)
            || row
                .occupied_through_micros
                .is_some_and(|value| value < row.signal_close_micros)
            || (row.reason == Reason::None) != row.trade_index.is_some()
            || row.trade_index.is_some()
                && (row.entry_bar.is_none() || row.occupied_through_micros.is_none())
        {
            return Err(Error::Encoding);
        }
        Ok(row)
    }
}

/// An actually observed IST date. Expected-but-absent dates are not fabricated.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Period {
    /// IST civil day since the epoch.
    pub day: i64,
    /// Actual offered one-minute rows on this date.
    pub offered_bars: u64,
    /// Unique accepted one-minute rows on this date.
    pub accepted_bars: u64,
    /// Refused/duplicate rows and missing adjacent in-session minutes.
    pub unavailable_minutes: u64,
    /// A unique accepted exact15:09 closing record exists.
    pub close_verified: bool,
    /// Actual priceable round trips on this date.
    pub trades: u64,
    /// Round trips with strictly positive pessimistic gross return.
    pub wins: u64,
    /// Exact summed gross optimistic paisa, with no capital multiplier.
    pub optimistic_paisa: i64,
    /// Exact summed gross pessimistic paisa, with no capital multiplier.
    pub pessimistic_paisa: i64,
    /// Entered/conservatively occupied paths whose price could not be proved.
    pub refused: u64,
    /// All definite signals with an exact entry coordinate on this date.
    pub signals: u64,
}
impl Period {
    /// Exact versioned period record width.
    pub const BYTE_LEN: usize = 96;
    /// Canonical zero-trade evidence stays distinct from a missing day.
    #[must_use]
    pub fn canonical_bytes(self) -> [u8; Self::BYTE_LEN] {
        encode(
            *b"BRSCPD01",
            [
                bits(self.day),
                self.offered_bars,
                self.accepted_bars,
                self.unavailable_minutes,
                u64::from(self.close_verified),
                self.trades,
                self.wins,
                bits(self.optimistic_paisa),
                bits(self.pessimistic_paisa),
                self.refused,
                self.signals,
            ],
        )
    }
    /// Decode counters without inventing an exchange-calendar assertion.
    /// # Errors
    /// Refuses invalid widths, flags or inconsistent counts and money.
    pub fn decode(bytes: &[u8]) -> Result<Self, Error> {
        let [
            day,
            offered_bars,
            accepted_bars,
            unavailable_minutes,
            close,
            trades,
            wins,
            best,
            worst,
            refused,
            signals,
        ] = decode(bytes, *b"BRSCPD01")?;
        let row = Self {
            day: signed(day),
            offered_bars,
            accepted_bars,
            unavailable_minutes,
            close_verified: close == 1,
            trades,
            wins,
            optimistic_paisa: signed(best),
            pessimistic_paisa: signed(worst),
            refused,
            signals,
        };
        if close > 1
            || row.offered_bars == 0
            || row.accepted_bars > row.offered_bars
            || row.wins > row.trades
            || row
                .trades
                .checked_add(row.refused)
                .is_none_or(|count| count > row.signals)
            || row.pessimistic_paisa > row.optimistic_paisa
            || row.trades == 0 && (row.optimistic_paisa != 0 || row.pessimistic_paisa != 0)
        {
            return Err(Error::Encoding);
        }
        Ok(row)
    }
}

/// Exact gross accounting and complete signal disposition counts.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Metrics {
    /// Actual completed round trips.
    pub trades: u64,
    /// Strictly positive pessimistic round trips.
    pub wins: u64,
    /// Checked gross optimistic total.
    pub optimistic_paisa: i64,
    /// Checked gross pessimistic total.
    pub pessimistic_paisa: i64,
    /// Maximum peak-to-trough drawdown of pessimistic cumulative return.
    pub drawdown_paisa: i64,
    /// Worst actual pessimistic trade; zero when no trade exists.
    pub worst_trade_paisa: i64,
    /// Largest observed adverse excursion through a trade's exit minute.
    pub adverse_paisa: i64,
    /// Largest guaranteed favourable excursion, with stop-minute order unknown.
    pub favourable_paisa: i64,
    /// Summed occupied one-minute bars for actual trades.
    pub holding_minutes: u64,
    /// Complete definite signals without an immediate entry minute before the
    /// 15:10 boundary: real one-minute data holes, which refuse execution.
    pub unreachable: u64,
    /// Signals whose entry would be at/after 15:10, including every last bucket
    /// of a session, whose close has no following minute at all (D-0603).
    pub too_late: u64,
    /// Gap-invalid entries deliberately skipped before opening.
    pub gap_invalid: u64,
    /// Signals blocked by an existing position or refusal hold.
    pub while_open: u64,
    /// Refused/duplicated immediate entry rows.
    pub entry_refused: u64,
    /// Entered paths refused before a known exit.
    pub path_refused: u64,
    /// Entered paths missing the required exact closing record.
    pub closing_refused: u64,
    /// Trades exited by their frozen signal-candle stop.
    pub stopped: u64,
    /// Trades exited at the actual15:10 closing print.
    pub forced: u64,
    /// Stop exits whose prior resting order opened through its boundary.
    pub stop_gaps: u64,
}

impl Metrics {
    /// Fixed-width summary independent of Rust's in-memory layout.
    pub const BYTE_LEN: usize = 160;
    /// Exact aggregate/skip/refusal bytes, excluding no event category.
    #[must_use]
    pub fn canonical_bytes(self) -> [u8; Self::BYTE_LEN] {
        encode(
            *b"BRSCMT01",
            [
                self.trades,
                self.wins,
                bits(self.optimistic_paisa),
                bits(self.pessimistic_paisa),
                bits(self.drawdown_paisa),
                bits(self.worst_trade_paisa),
                bits(self.adverse_paisa),
                bits(self.favourable_paisa),
                self.holding_minutes,
                self.unreachable,
                self.too_late,
                self.gap_invalid,
                self.while_open,
                self.entry_refused,
                self.path_refused,
                self.closing_refused,
                self.stopped,
                self.forced,
                self.stop_gaps,
            ],
        )
    }
    /// Reopen exact counters; full row reconciliation is separate.
    /// # Errors
    /// Refuses invalid width or contradictory money/count bounds.
    pub fn decode(bytes: &[u8]) -> Result<Self, Error> {
        let [
            trades,
            wins,
            best,
            worst,
            drawdown,
            worst_trade,
            adverse,
            favourable,
            holding_minutes,
            unreachable,
            too_late,
            gap_invalid,
            while_open,
            entry_refused,
            path_refused,
            closing_refused,
            stopped,
            forced,
            stop_gaps,
        ] = decode(bytes, *b"BRSCMT01")?;
        let row = Self {
            trades,
            wins,
            optimistic_paisa: signed(best),
            pessimistic_paisa: signed(worst),
            drawdown_paisa: signed(drawdown),
            worst_trade_paisa: signed(worst_trade),
            adverse_paisa: signed(adverse),
            favourable_paisa: signed(favourable),
            holding_minutes,
            unreachable,
            too_late,
            gap_invalid,
            while_open,
            entry_refused,
            path_refused,
            closing_refused,
            stopped,
            forced,
            stop_gaps,
        };
        if wins > trades
            || stopped.checked_add(forced) != Some(trades)
            || stop_gaps > stopped
            || row.optimistic_paisa < row.pessimistic_paisa
            || row.drawdown_paisa < 0
            || row.adverse_paisa < 0
            || row.favourable_paisa < 0
            || holding_minutes < trades
            || trades == 0
                && [
                    row.optimistic_paisa,
                    row.pessimistic_paisa,
                    row.drawdown_paisa,
                    row.worst_trade_paisa,
                    row.adverse_paisa,
                    row.favourable_paisa,
                ]
                .iter()
                .any(|value| *value != 0)
        {
            return Err(Error::Encoding);
        }
        Ok(row)
    }
}

/// One sealed source/program/side evaluation, never a legacy selected-grid proof.
pub struct Evaluation {
    policy: Policy,
    family: ResearchFamilyV1,
    direction: Direction,
    timeframe: &'static str,
    first_day: i64,
    last_day: i64,
    source_id: [u8; 32],
    run_id: [u8; 32],
    program: Expression,
    truth: TruthSummary,
    metrics: Metrics,
    events: Vec<Event>,
    trades: Vec<Trade>,
    periods: Vec<Period>,
    digest: [u8; 32],
}
impl Evaluation {
    /// Exact execution policy.
    #[must_use]
    pub const fn policy(&self) -> Policy {
        self.policy
    }
    /// Exact approved index family.
    #[must_use]
    pub const fn family(&self) -> ResearchFamilyV1 {
        self.family
    }
    /// One explicit long or short setting.
    #[must_use]
    pub const fn direction(&self) -> Direction {
        self.direction
    }
    /// Canonical signal timeframe; execution always uses actual one-minute bars.
    #[must_use]
    pub const fn timeframe(&self) -> &'static str {
        self.timeframe
    }
    /// Inclusive output start day, with earlier causal source history retained.
    #[must_use]
    pub const fn first_day(&self) -> i64 {
        self.first_day
    }
    /// Inclusive output end day, bound independently of the complete source.
    #[must_use]
    pub const fn last_day(&self) -> i64 {
        self.last_day
    }
    /// Full prepared immutable source identity.
    #[must_use]
    pub const fn source_id(&self) -> [u8; 32] {
        self.source_id
    }
    /// Full program-aware run identity, including this new policy namespace.
    #[must_use]
    pub const fn run_id(&self) -> [u8; 32] {
        self.run_id
    }
    /// Exact complete expression; referenced bits are not substituted for it.
    #[must_use]
    pub fn program(&self) -> &Expression {
        &self.program
    }
    /// Definite true, false and unknown counts over original signal rows.
    #[must_use]
    pub const fn truth(&self) -> TruthSummary {
        self.truth
    }
    /// Checked gross trade metrics and explicit refusal/skip counts.
    #[must_use]
    pub const fn metrics(&self) -> Metrics {
        self.metrics
    }
    /// Ordered definite-signal dispositions, including skipped/refused rows.
    #[must_use]
    pub fn events(&self) -> &[Event] {
        &self.events
    }
    /// Exact completed round trips only.
    #[must_use]
    pub fn trades(&self) -> &[Trade] {
        &self.trades
    }
    /// Every actually observed execution date, including explicit zero trades.
    #[must_use]
    pub fn periods(&self) -> &[Period] {
        &self.periods
    }
    /// Complete immutable source/run/row/counter digest.
    #[must_use]
    pub const fn digest(&self) -> [u8; 32] {
        self.digest
    }
}

impl<'a> Prepared<'a> {
    /// Derive one execution acceptance map, exact alignment and source identity.
    /// # Errors
    /// Refuses non-index scope, invalid source identity/order/shape, a foreign
    /// execution subspan, arithmetic or requested output allocation failure.
    pub fn new(policy: Policy, source: Source<'a>) -> Result<Self, Error> {
        let family =
            ResearchFamilyV1::new(*source.series.instrument()).map_err(|_| Error::Scope)?;
        if family.legacy_index_family().is_none() {
            return Err(Error::Scope);
        }
        let signal_length = timeframe_micros(source.timeframe)?;
        validate_source(&source, signal_length)?;
        let aligned = crate::align::onto_execution(
            source.signal_bars,
            source.signal_column.sources(),
            source.series.bars(),
            signal_length,
        )
        .ok_or(Error::Source)?;
        let (execution_column, _) = source
            .signal_column
            .reproject_checked(&aligned.onto, source.series.bars())
            .ok_or(Error::Source)?;
        let facts = SliceFacts::of(source.series.bars(), &execution_column);
        if !facts.covers(source.series.bars()) || facts.step_micros() != MINUTE {
            return Err(Error::Source);
        }
        let data_digest = crate::identity::data_digest_with_daily_reference(
            source.signal_bars,
            source.minute_context,
            source.daily_reference,
        )
        .map_err(|_| Error::Source)?;
        let mut identity = Hasher::new();
        identity.update(b"brutex.signal-candle-stop.source.v1\0");
        identity.update(&policy.canonical_bytes());
        identity.update(&family.encode());
        identity.update(&data_digest);
        identity.update(&crate::identity::data_digest(source.series.bars()));
        identity.update(&source.series.calendar_digest());
        identity.update(&source.first_day.to_le_bytes());
        identity.update(&source.last_day.to_le_bytes());
        identity.update(&column_digest_v1(source.signal_column));
        identity.update(&column_digest_v1(&execution_column));
        // The older execution-column digest predates three-valued known bits.
        // Bind the exact availability column explicitly in this new namespace.
        for known in source.signal_column.known() {
            for word in known.words() {
                identity.update(&word.to_le_bytes());
            }
        }
        let run = Run {
            mask: vocab::ConditionMask::ZERO,
            direction: Direction::Undirected,
            instrument: source.series.instrument(),
            timeframe: source.timeframe,
            params: source.params,
            data_digest,
            commit: source.series.commit(),
            feed: source.series.feed(),
        };
        identity.update(&crate::identity::identity(&run).bytes());
        let duplicates = duplicates(source.series.bars())?;
        let (periods, period_of) = period_geometry(source.series.bars(), &facts, &duplicates)?;
        Ok(Self {
            policy,
            source,
            family,
            alignment: aligned.onto,
            facts,
            duplicates,
            periods,
            period_of,
            data_digest,
            source_id: identity.finalize(),
            signal_length,
        })
    }
    /// Exact sealed source identity; computing it here never rehashes prices.
    #[must_use]
    pub const fn source_id(&self) -> [u8; 32] {
        self.source_id
    }
    /// Derived original-signal to exact-entry mapping, for source audit only.
    #[must_use]
    pub fn alignment(&self) -> &[Option<usize>] {
        &self.alignment
    }
    /// Record this exact identity before evaluating a program and direction.
    /// This reads fixed metadata/program bytes; it does not rehash source bars.
    /// # Errors
    /// Refuses an undirected frequency query at the trading boundary.
    pub fn run_id(&self, program: &Expression, direction: Direction) -> Result<[u8; 32], Error> {
        let (first, last) = self.full_span();
        self.run_id_days(program, direction, first, last)
    }
    /// Record the exact fixed-period identity before evaluating its prices.
    /// Earlier bars remain causal warm-up, never counted as later observations.
    /// # Errors
    /// Refuses an invalid direction or a window outside retained source days.
    pub fn run_id_days(
        &self,
        program: &Expression,
        direction: Direction,
        first_day: i64,
        last_day: i64,
    ) -> Result<[u8; 32], Error> {
        self.check_window(first_day, last_day)?;
        if direction == Direction::Undirected {
            return Err(Error::Scope);
        }
        let run = Run {
            mask: program.referenced(),
            direction,
            instrument: self.source.series.instrument(),
            timeframe: self.source.timeframe,
            params: self.source.params,
            data_digest: self.data_digest,
            commit: self.source.series.commit(),
            feed: self.source.series.feed(),
        };
        let mut id = Hasher::new();
        id.update(b"brutex.signal-candle-stop.run.v1\0");
        id.update(&self.source_id);
        id.update(&crate::identity::identity_with_expression(&run, program).bytes());
        id.update(&first_day.to_le_bytes());
        id.update(&last_day.to_le_bytes());
        Ok(id.finalize())
    }
    const fn full_span(&self) -> (i64, i64) {
        (self.source.first_day, self.source.last_day)
    }
    fn check_window(&self, first: i64, last: i64) -> Result<(), Error> {
        let (start, end) = self.full_span();
        if first > last || first < start || last > end {
            Err(Error::Scope)
        } else {
            Ok(())
        }
    }
    /// Evaluate exactly one program and side with a finite evidence cap.
    /// # Errors
    /// Refuses undirected execution, evidence-bound/allocation failure or any
    /// exact aggregate arithmetic overflow. Such failures yield no success seal.
    pub fn evaluate(
        &self,
        program: &Expression,
        direction: Direction,
        max_events: u64,
    ) -> Result<Evaluation, Error> {
        let (first, last) = self.full_span();
        self.evaluate_days(program, direction, max_events, first, last)
    }
    /// Evaluate one fixed output window while retaining the full causal column.
    /// No indicator is reset at the window boundary and no earlier trade is
    /// carried overnight into it: this policy is intraday by construction.
    /// # Errors
    /// Same execution/evidence errors as `evaluate`, plus invalid source window.
    pub fn evaluate_days(
        &self,
        program: &Expression,
        direction: Direction,
        max_events: u64,
        first_day: i64,
        last_day: i64,
    ) -> Result<Evaluation, Error> {
        self.check_window(first_day, last_day)?;
        let side = match direction {
            Direction::Long => Side::Long,
            Direction::Short => Side::Short,
            Direction::Undirected => return Err(Error::Scope),
        };
        let mut out = Evaluation {
            policy: self.policy,
            family: self.family,
            direction,
            timeframe: canonical_timeframe(self.source.timeframe)?,
            first_day,
            last_day,
            source_id: self.source_id,
            run_id: self.run_id_days(program, direction, first_day, last_day)?,
            program: program.clone(),
            truth: TruthSummary::default(),
            metrics: Metrics::default(),
            events: Vec::new(),
            trades: Vec::new(),
            periods: Vec::new(),
            digest: [0; 32],
        };
        out.periods
            .try_reserve_exact(self.periods.len())
            .map_err(|_| Error::Allocation)?;
        let period_start = self
            .periods
            .iter()
            .position(|row| row.day >= first_day)
            .unwrap_or(self.periods.len());
        out.periods.extend(
            self.periods
                .iter()
                .filter(|row| row.day >= first_day && row.day <= last_day)
                .copied(),
        );
        let mut occupied = None;
        let mut peak_equity = 0_i64;
        for (position, ((&bits, &known), &signal)) in self
            .source
            .signal_column
            .bits()
            .iter()
            .zip(self.source.signal_column.known())
            .zip(self.source.signal_column.sources())
            .enumerate()
        {
            let signal_day = ist_day(
                self.source
                    .signal_bars
                    .get(signal)
                    .ok_or(Error::Source)?
                    .ts_micros,
            )?;
            if signal_day < first_day || signal_day > last_day {
                continue;
            }
            let truth = program.evaluate(bits, known);
            out.truth.evaluated = add(out.truth.evaluated, 1)?;
            match truth {
                Truth::False => out.truth.misses = add(out.truth.misses, 1)?,
                Truth::Unknown => out.truth.unknown = add(out.truth.unknown, 1)?,
                Truth::True => {
                    if out.events.len() as u64 >= max_events {
                        return Err(Error::Bound);
                    }
                    out.truth.hits = add(out.truth.hits, 1)?;
                    let (mut event, trade) = self.signal(signal, position, side, occupied)?;
                    if event.reason != Reason::WhileOpen && event.occupied_through_micros.is_some()
                    {
                        occupied = event.occupied_through_micros;
                    }
                    if let Some(trade) = trade {
                        record_trade(&mut out.metrics, &mut peak_equity, &trade)?;
                        event.trade_index = Some(out.trades.len() as u64);
                        out.trades.try_reserve(1).map_err(|_| Error::Allocation)?;
                        out.trades.push(trade);
                    }
                    record_event(&mut out, &event, trade, self, period_start)?;
                    out.events.try_reserve(1).map_err(|_| Error::Allocation)?;
                    out.events.push(event);
                }
            }
        }
        out.digest = evaluation_digest(&out);
        Ok(out)
    }
    fn signal(
        &self,
        signal: usize,
        position: usize,
        side: Side,
        occupied: Option<i64>,
    ) -> Result<(Event, Option<Trade>), Error> {
        let bar = self.source.signal_bars.get(signal).ok_or(Error::Source)?;
        let close = bar
            .ts_micros
            .checked_add(self.signal_length)
            .ok_or(Error::Arithmetic)?;
        let stop = match side {
            Side::Long => bar.low,
            Side::Short => bar.high,
        };
        let mut event = Event {
            signal_bar: signal as u64,
            signal_micros: bar.ts_micros,
            signal_close_micros: close,
            stop_paisa: stop,
            reason: Reason::Unreachable,
            entry_bar: None,
            entry_micros: None,
            occupied_through_micros: None,
            trade_index: None,
        };
        let Some(entry) = self.alignment.get(position).copied().flatten() else {
            // NO ENTRY MINUTE IS NOT ALWAYS A MISSING OBSERVATION -- D-0603.
            //
            // `align::onto_execution` maps a signal to the minute stamped exactly
            // at its close on the same day, or to nothing. The session's last
            // bucket always gets nothing: it closes at or after 15:30 and no
            // minute follows the market. Filed as `Unreachable`, that bucket made
            // `execution_complete` refuse every setting whose condition could fire
            // on the day's last bar -- 3,076 of 4,000 in one live 60min batch --
            // for a signal that was only too late to trade.
            //
            // So an absent minute is judged exactly as a present one would be: an
            // entry that would open after 15:09 is `TooLate` whether or not its
            // minute exists. Only an absence at or before 15:09, which could have
            // been traded, stays `Unreachable` -- a real one-minute data hole.
            if close > forced_open(bar.ts_micros)? {
                event.reason = Reason::TooLate;
            }
            return Ok((event, None));
        };
        let entry_bar = self.source.series.bars().get(entry).ok_or(Error::Source)?;
        event.entry_bar = Some(entry as u64);
        event.entry_micros = Some(entry_bar.ts_micros);
        let forced = forced_open(entry_bar.ts_micros)?;
        if entry_bar.ts_micros > forced {
            event.reason = Reason::TooLate;
            return Ok((event, None));
        }
        if occupied.is_some_and(|through| entry_bar.ts_micros <= through) {
            event.reason = Reason::WhileOpen;
            event.occupied_through_micros = occupied;
            return Ok((event, None));
        }
        if !self.accepts(entry) {
            event.reason = Reason::EntryRefused;
            event.occupied_through_micros = Some(forced);
            return Ok((event, None));
        }
        let invalid = match side {
            Side::Long => entry_bar.open <= stop,
            Side::Short => entry_bar.open >= stop,
        };
        if invalid {
            event.reason = Reason::GapInvalid;
            return Ok((event, None));
        }
        let walked = self.walk(&event, entry, side, forced)?;
        match walked {
            Ok(trade) => {
                event.reason = Reason::None;
                event.occupied_through_micros = Some(trade.exit_bar_micros);
                Ok((event, Some(trade)))
            }
            Err(reason) => {
                event.reason = reason;
                event.occupied_through_micros = Some(forced);
                Ok((event, None))
            }
        }
    }
    fn accepts(&self, index: usize) -> bool {
        self.facts.accepts(index) && !self.duplicates.get(index).copied().unwrap_or(true)
    }
    fn walk(
        &self,
        event: &Event,
        entry: usize,
        side: Side,
        forced: i64,
    ) -> Result<Result<Trade, Reason>, Error> {
        let bars = self.source.series.bars();
        let opening = bars.get(entry).ok_or(Error::Source)?;
        let mut adverse = 0_i64;
        let mut favourable = 0_i64;
        let mut expected = opening.ts_micros;
        for (offset, bar) in bars.get(entry..).ok_or(Error::Source)?.iter().enumerate() {
            let index = entry.checked_add(offset).ok_or(Error::Arithmetic)?;
            if bar.ts_micros > forced {
                break;
            }
            if bar.ts_micros != expected || !self.accepts(index) {
                return Ok(Err(Reason::PathRefused));
            }
            expected = expected.checked_add(MINUTE).ok_or(Error::Arithmetic)?;
            let hit = match side {
                Side::Long => bar.low <= event.stop_paisa,
                Side::Short => bar.high >= event.stop_paisa,
            };
            // A stop minute has no known high/low order. Only its open is
            // guaranteed to precede the exit; crediting its later favourable
            // extreme would make the institutional excursion optimistic.
            let favourable_price = if hit {
                bar.open
            } else {
                match side {
                    Side::Long => bar.high,
                    Side::Short => bar.low,
                }
            };
            let (a, f) = match side {
                Side::Long => (
                    opening.open.checked_sub(bar.low),
                    favourable_price.checked_sub(opening.open),
                ),
                Side::Short => (
                    bar.high.checked_sub(opening.open),
                    opening.open.checked_sub(favourable_price),
                ),
            };
            adverse = adverse.max(a.ok_or(Error::Arithmetic)?.max(0));
            favourable = favourable.max(f.ok_or(Error::Arithmetic)?.max(0));
            let (reason, best, worst, gapped) = if hit {
                let (best, worst, gapped) =
                    crate::grid::printed_stop_fills_v1(bar, event.stop_paisa, side, index != entry);
                (ExitReason::Stop, best, worst, gapped)
            } else if bar.ts_micros == forced {
                if !self
                    .facts
                    .exits()
                    .get(entry)
                    .copied()
                    .flatten()
                    .is_some_and(|exit| exit.real && exit.bar == index)
                {
                    return Ok(Err(Reason::ClosingMinute));
                }
                (ExitReason::Forced1510, bar.close, bar.close, false)
            } else {
                continue;
            };
            let pnl = |price: i64| {
                match side {
                    Side::Long => price.checked_sub(opening.open),
                    Side::Short => opening.open.checked_sub(price),
                }
                .ok_or(Error::Arithmetic)
            };
            let end = bar.ts_micros.checked_add(MINUTE).ok_or(Error::Arithmetic)?;
            let trade = Trade {
                signal_bar: event.signal_bar,
                entry_bar: entry as u64,
                exit_bar: index as u64,
                signal_micros: event.signal_micros,
                signal_close_micros: event.signal_close_micros,
                entry_micros: opening.ts_micros,
                exit_bar_micros: bar.ts_micros,
                exit_from_micros: if reason == ExitReason::Forced1510 {
                    end
                } else {
                    bar.ts_micros
                },
                exit_until_micros: end,
                stop_paisa: event.stop_paisa,
                entry_paisa: opening.open,
                optimistic_exit_paisa: best,
                pessimistic_exit_paisa: worst,
                optimistic_paisa: pnl(best)?,
                pessimistic_paisa: pnl(worst)?,
                adverse_paisa: adverse,
                favourable_paisa: favourable,
                holding_minutes: u64::try_from((end - opening.ts_micros) / MINUTE)
                    .map_err(|_| Error::Arithmetic)?,
                exit_reason: reason,
                gapped,
            };
            return Ok(Ok(trade));
        }
        Ok(Err(Reason::ClosingMinute))
    }
}

fn validate_source(source: &Source<'_>, length: i64) -> Result<(), Error> {
    let signals = source.signal_bars;
    let bars = source.series.bars();
    let column = source.signal_column;
    if signals.is_empty()
        || bars.is_empty()
        || source.first_day > source.last_day
        || source.first_day.checked_mul(DAY).is_none()
        || source.last_day.checked_mul(DAY).is_none()
        || source.series.feed().is_empty()
        || source.series.commit().is_empty()
        || source.series.calendar_digest() == [0; 32]
        || column.sourced() != Sourced::Signal
        || column.bits().len() != column.known().len()
        || column.len() != column.sources().len()
        || !column.acceptance_covers(signals.len())
        || column.evaluation_spec_token().is_none()
        || signals.windows(2).any(|pair| {
            pair.first()
                .zip(pair.get(1))
                .is_some_and(|(a, b)| a.ts_micros >= b.ts_micros)
        })
        || column
            .sources()
            .windows(2)
            .any(|pair| pair.first() >= pair.get(1))
    {
        return Err(Error::Source);
    }
    for &index in column.sources() {
        let row = signals.get(index).ok_or(Error::Source)?;
        if !column.accepts(index)
            || row.ts_micros.rem_euclid(MINUTE) != 0
            || row.ts_micros.checked_add(length).is_none()
            || row.low <= 0
            || row.high < row.low
        {
            return Err(Error::Source);
        }
    }
    for offered in [bars, source.minute_context] {
        if offered.windows(2).any(|pair| {
            pair.first()
                .zip(pair.get(1))
                .is_some_and(|(a, b)| a.ts_micros > b.ts_micros)
        }) || offered.iter().any(|bar| ist_day(bar.ts_micros).is_err())
        {
            return Err(Error::Source);
        }
    }
    if bars
        .first()
        .is_some_and(|bar| indicators::ist_day(bar.ts_micros) < source.first_day)
        || bars
            .last()
            .is_some_and(|bar| indicators::ist_day(bar.ts_micros) > source.last_day)
    {
        return Err(Error::Source);
    }
    let first = bars.first().ok_or(Error::Source)?.ts_micros;
    let start = source
        .minute_context
        .iter()
        .position(|bar| bar.ts_micros >= first)
        .ok_or(Error::Source)?;
    let end = start.checked_add(bars.len()).ok_or(Error::Arithmetic)?;
    if source.minute_context.get(start..end) != Some(bars) {
        return Err(Error::Source);
    }
    Ok(())
}
fn duplicates(bars: &[Candle]) -> Result<Vec<bool>, Error> {
    let mut rows = Vec::new();
    rows.try_reserve_exact(bars.len())
        .map_err(|_| Error::Allocation)?;
    for (index, bar) in bars.iter().enumerate() {
        rows.push(
            index
                .checked_sub(1)
                .and_then(|i| bars.get(i))
                .is_some_and(|before| before.ts_micros == bar.ts_micros)
                || bars
                    .get(index.saturating_add(1))
                    .is_some_and(|after| after.ts_micros == bar.ts_micros),
        );
    }
    Ok(rows)
}
fn period_geometry(
    bars: &[Candle],
    facts: &SliceFacts,
    duplicate: &[bool],
) -> Result<(Vec<Period>, Vec<usize>), Error> {
    let mut periods: Vec<Period> = Vec::new();
    let mut positions = Vec::new();
    positions
        .try_reserve_exact(bars.len())
        .map_err(|_| Error::Allocation)?;
    let mut previous = None;
    for (index, bar) in bars.iter().enumerate() {
        let day = ist_day(bar.ts_micros)?;
        if periods.last().is_none_or(|period| period.day != day) {
            periods.try_reserve(1).map_err(|_| Error::Allocation)?;
            periods.push(Period {
                day,
                ..Period::default()
            });
            previous = None;
        }
        let position = periods.len().checked_sub(1).ok_or(Error::Source)?;
        positions.push(position);
        let period = periods.get_mut(position).ok_or(Error::Source)?;
        period.offered_bars = add(period.offered_bars, 1)?;
        let accepted = facts.accepts(index) && !duplicate.get(index).copied().unwrap_or(true);
        if accepted {
            period.accepted_bars = add(period.accepted_bars, 1)?;
        } else {
            period.unavailable_minutes = add(period.unavailable_minutes, 1)?;
        }
        if let Some(prior) = previous {
            let distance = bar.ts_micros.checked_sub(prior).ok_or(Error::Arithmetic)?;
            if distance > MINUTE {
                period.unavailable_minutes = add(
                    period.unavailable_minutes,
                    u64::try_from(distance / MINUTE - 1).map_err(|_| Error::Arithmetic)?,
                )?;
            }
        }
        period.close_verified |= facts
            .exits()
            .get(index)
            .copied()
            .flatten()
            .is_some_and(|exit| exit.real);
        previous = Some(bar.ts_micros);
    }
    Ok((periods, positions))
}
fn record_trade(metrics: &mut Metrics, peak: &mut i64, row: &Trade) -> Result<(), Error> {
    metrics.trades = add(metrics.trades, 1)?;
    metrics.wins = add(metrics.wins, u64::from(row.pessimistic_paisa > 0))?;
    metrics.optimistic_paisa = metrics
        .optimistic_paisa
        .checked_add(row.optimistic_paisa)
        .ok_or(Error::Arithmetic)?;
    metrics.pessimistic_paisa = metrics
        .pessimistic_paisa
        .checked_add(row.pessimistic_paisa)
        .ok_or(Error::Arithmetic)?;
    *peak = (*peak).max(metrics.pessimistic_paisa);
    metrics.drawdown_paisa = metrics.drawdown_paisa.max(
        peak.checked_sub(metrics.pessimistic_paisa)
            .ok_or(Error::Arithmetic)?,
    );
    metrics.worst_trade_paisa = if metrics.trades == 1 {
        row.pessimistic_paisa
    } else {
        metrics.worst_trade_paisa.min(row.pessimistic_paisa)
    };
    metrics.adverse_paisa = metrics.adverse_paisa.max(row.adverse_paisa);
    metrics.favourable_paisa = metrics.favourable_paisa.max(row.favourable_paisa);
    metrics.holding_minutes = add(metrics.holding_minutes, row.holding_minutes)?;
    match row.exit_reason {
        ExitReason::Stop => metrics.stopped = add(metrics.stopped, 1)?,
        ExitReason::Forced1510 => metrics.forced = add(metrics.forced, 1)?,
    }
    metrics.stop_gaps = add(metrics.stop_gaps, u64::from(row.gapped))?;
    Ok(())
}
fn record_event(
    out: &mut Evaluation,
    event: &Event,
    trade: Option<Trade>,
    prepared: &Prepared<'_>,
    period_start: usize,
) -> Result<(), Error> {
    let count = match event.reason {
        Reason::None => None,
        Reason::Unreachable => Some(&mut out.metrics.unreachable),
        Reason::TooLate => Some(&mut out.metrics.too_late),
        Reason::GapInvalid => Some(&mut out.metrics.gap_invalid),
        Reason::WhileOpen => Some(&mut out.metrics.while_open),
        Reason::EntryRefused => Some(&mut out.metrics.entry_refused),
        Reason::PathRefused => Some(&mut out.metrics.path_refused),
        Reason::ClosingMinute => Some(&mut out.metrics.closing_refused),
    };
    if let Some(count) = count {
        *count = add(*count, 1)?;
    }
    if let Some(index) = event.entry_bar {
        let period = prepared
            .period_of
            .get(usize::try_from(index).map_err(|_| Error::Arithmetic)?)
            .and_then(|position| position.checked_sub(period_start))
            .and_then(|position| out.periods.get_mut(position))
            .ok_or(Error::Source)?;
        period.signals = add(period.signals, 1)?;
        if matches!(
            event.reason,
            Reason::EntryRefused | Reason::PathRefused | Reason::ClosingMinute
        ) {
            period.refused = add(period.refused, 1)?;
        }
        if let Some(row) = trade {
            period.trades = add(period.trades, 1)?;
            period.wins = add(period.wins, u64::from(row.pessimistic_paisa > 0))?;
            period.optimistic_paisa = period
                .optimistic_paisa
                .checked_add(row.optimistic_paisa)
                .ok_or(Error::Arithmetic)?;
            period.pessimistic_paisa = period
                .pessimistic_paisa
                .checked_add(row.pessimistic_paisa)
                .ok_or(Error::Arithmetic)?;
        }
    }
    Ok(())
}
fn evaluation_digest(out: &Evaluation) -> [u8; 32] {
    observation_digest(
        out.source_id,
        out.run_id,
        out.truth,
        out.metrics,
        &out.events,
        &out.trades,
        &out.periods,
    )
}
/// Recompute the exact persisted observation digest without minting an
/// evaluated-source capability. Readers separately reconcile source manifests,
/// direction/program/window bindings and the native decoded row references.
#[must_use]
pub fn observation_digest(
    source_id: [u8; 32],
    run_id: [u8; 32],
    truth: TruthSummary,
    metrics: Metrics,
    events: &[Event],
    trades: &[Trade],
    periods: &[Period],
) -> [u8; 32] {
    let mut hash = Hasher::new();
    hash.update(b"brutex.signal-candle-stop.evaluation.v1\0");
    hash.update(&source_id);
    hash.update(&run_id);
    hash.update(&metrics.canonical_bytes());
    for value in [truth.evaluated, truth.hits, truth.misses, truth.unknown] {
        hash.update(&value.to_le_bytes());
    }
    hash.update(&(events.len() as u64).to_le_bytes());
    for row in events {
        hash.update(&row.canonical_bytes());
    }
    hash.update(&(trades.len() as u64).to_le_bytes());
    for row in trades {
        hash.update(&row.canonical_bytes());
    }
    hash.update(&(periods.len() as u64).to_le_bytes());
    for row in periods {
        hash.update(&row.canonical_bytes());
    }
    hash.finalize()
}
fn timeframe_micros(label: &str) -> Result<i64, Error> {
    let minutes = match label {
        "1min" => 1,
        "2min" => 2,
        "3min" => 3,
        "5min" => 5,
        "10min" => 10,
        "15min" => 15,
        "30min" => 30,
        "60min" => 60,
        _ => return Err(Error::Scope),
    };
    Ok(minutes * MINUTE)
}
fn canonical_timeframe(label: &str) -> Result<&'static str, Error> {
    match label {
        "1min" => Ok("1min"),
        "2min" => Ok("2min"),
        "3min" => Ok("3min"),
        "5min" => Ok("5min"),
        "10min" => Ok("10min"),
        "15min" => Ok("15min"),
        "30min" => Ok("30min"),
        "60min" => Ok("60min"),
        _ => Err(Error::Scope),
    }
}
fn forced_open(stamp: i64) -> Result<i64, Error> {
    let day = ist_day(stamp)?;
    day.checked_mul(DAY)
        .and_then(|v| v.checked_sub(IST))
        .and_then(|v| v.checked_add((crate::outcome::FORCED_EXIT_MINUTE - 1) * MINUTE))
        .ok_or(Error::Arithmetic)
}
fn ist_day(stamp: i64) -> Result<i64, Error> {
    stamp.checked_add(IST).ok_or(Error::Arithmetic)?;
    Ok(indicators::ist_day(stamp))
}
fn ppm(distance: i64, entry: i64) -> Result<i64, Error> {
    if entry <= 0 || distance < 0 {
        return Err(Error::Arithmetic);
    }
    i64::try_from(i128::from(distance) * 1_000_000 / i128::from(entry))
        .map_err(|_| Error::Arithmetic)
}
fn add(left: u64, right: u64) -> Result<u64, Error> {
    left.checked_add(right).ok_or(Error::Arithmetic)
}
const fn bits(value: i64) -> u64 {
    u64::from_le_bytes(value.to_le_bytes())
}
const fn signed(value: u64) -> i64 {
    i64::from_le_bytes(value.to_le_bytes())
}
fn hash(bytes: &[u8]) -> [u8; 32] {
    let mut hash = Hasher::new();
    hash.update(bytes);
    hash.finalize()
}
fn encode<const N: usize, const B: usize>(magic: [u8; 8], words: [u64; N]) -> [u8; B] {
    let mut out = [0; B];
    for (target, source) in out.iter_mut().zip(
        magic
            .iter()
            .copied()
            .chain(words.iter().flat_map(|word| word.to_le_bytes())),
    ) {
        *target = source;
    }
    out
}
fn decode<const N: usize>(raw: &[u8], magic: [u8; 8]) -> Result<[u64; N], Error> {
    if raw.len() != 8 + N * 8 || raw.get(..8) != Some(&magic) {
        return Err(Error::Encoding);
    }
    let mut words = [0; N];
    for (word, chunk) in words
        .iter_mut()
        .zip(raw.get(8..).ok_or(Error::Encoding)?.chunks_exact(8))
    {
        *word = u64::from_le_bytes(chunk.try_into().map_err(|_| Error::Encoding)?);
    }
    Ok(words)
}
fn option(flag: u64, value: u64) -> Result<Option<u64>, Error> {
    match flag {
        0 if value == 0 => Ok(None),
        1 => Ok(Some(value)),
        _ => Err(Error::Encoding),
    }
}

#[cfg(test)]
#[path = "signal_candle_stop_tests.rs"]
mod tests;
