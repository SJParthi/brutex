//! The per-stock-day GDFL clock calibration (FNO-CLOCK; D-8220 to D-8226).
//!
//! # What it answers
//!
//! The F&O strategy's headline uses only latencies at or above a stock-day's
//! calibrated floor (SPEC OP-07, D-0802 phase 2). A decision is made on a
//! Zerodha candle's close and filled on a GDFL second, and the two feeds keep
//! different clocks: a latency under the floor could act in a GDFL second that
//! printed before the candle that decided it. For one stock-day,
//!
//! `floor = max(0, offset) + back_step`
//!
//! * **The offset** (D-8221) is where GDFL's stamps sit against Zerodha's
//!   minute boundaries that day, positive when GDFL stamps a trade later. Each
//!   shift `delta` sets every comparable Zerodha minute `[t, t + 60)` against
//!   GDFL's traded seconds in `[t + delta, t + delta + 60)`, folded to one
//!   open, high, low and close, and counts the prices that agree under the
//!   session's raw-to-adjusted ratio. The shift with the most agreeing prices
//!   is the offset; on a tie the larger `|delta|`, and the positive one on an
//!   equal `|delta|`: the worse reading, because the floor takes
//!   `max(0, offset)` (SPEC FIB-I5).
//! * **The back step** (D-8222) is the largest step backwards of the vendor's
//!   own clock in the day's raw tick order: the most by which a row's stamp
//!   falls below the largest stamp of the rows before it in the file.
//!
//! [`ClockRow::floor`] says which of the two terms were measured, and a run
//! labels every latency under the floor, or every latency of a stock-day with
//! no calibrated offset, "uncalibrated clock" (SPEC 4.2).
//!
//! # The comparison (D-8221)
//!
//! A Zerodha minute is compared when it traded (volume above zero) and its
//! minute of day is [`Phase::Continuous`] for the NSE cash venue by
//! `pull::calendar::phase_at`, the one hours authority, with the share's
//! closing-auction eligibility `Unknown`. So the closing-auction band from
//! 2026-08-03, a recorded halt and its reopening auction, and every minute
//! outside the session are never compared, and no time or date is typed here.
//!
//! Zerodha is back-adjusted and GDFL is raw, and inside one session they
//! differ by one ratio (SPEC FIB-I8). A matched price gives the exact ratio
//! GDFL over Zerodha, as a reduced fraction. The session's ratio at a shift is
//! the ratio the most prices give, among the ratios that prices of at least two
//! minutes give: a ratio only one minute gives is fitted to that minute and
//! proves nothing. The shift's score is the number of prices that agree under
//! it, and a day whose best score is zero has no agreement and no offset.
//!
//! A minute whose shifted window holds no traded second matches none of its
//! prices, so every shift is scored over the same minutes and an empty window
//! can never hide a disagreement.
//!
//! The search starts at `±FIRST_WINDOW_S` and doubles while the best shift
//! sits on the window's edge, until it is inside the window or the window
//! covers every shift at which some minute's window can still meet a traded
//! second (the row's `reach_s`).
//!
//! # Where the back step comes from (D-8222)
//!
//! PARTS left it UNVERIFIED whether `pull::gdfl_tickstore` exposes raw tick
//! order. It does: `TickStore`, a `pull::gdfl_cm::CmSource`, rebuilds the
//! vendor's file byte for byte, and `pull::gdfl_cm::read_listed` checks those
//! bytes against the length and CRC-32 the day's listing states before it
//! decodes them into `CmFile::rows`, every kept row in file order with its
//! stamp. [`read_back_step`] reads it there. A day the source does not hold,
//! or a stock it holds no file for, leaves the step unmeasured and flagged; a
//! refusal does too, and [`build`] names it.
//!
//! # The table (D-8220)
//!
//! `build(keys, bars, ticks, out)` writes `<out>/fno-clock-v1/<STOCK>.fcl`, one
//! file per stock, its rows at a fixed stride in day order. The key list is the
//! one input (PARTS, READINESS ADDENDUM item 8): nothing else chooses a day,
//! and a list rebuild writes exactly the bytes a full build writes for every
//! key it names, leaving every other row and every other stock's file as it
//! was (PARTS, main-session decision 5 of 1 Nov 2026). A file is
//!
//! | bytes | field |
//! |---|---|
//! | 0..8 | [`MAGIC`] |
//! | 8..10 | [`VERSION`], little-endian |
//! | 10..12 | [`ROW_LEN`], little-endian |
//! | 12..16 | rows, little-endian |
//! | 16..40 | the stock, zero-padded |
//! | 40..72 | BLAKE3 of a domain tag, bytes 0..40 and every row |
//! | 72.. | the rows |
//!
//! and a row is eleven little-endian 4-byte fields, then two 2-byte ones: the
//! day (days since 1970-01-01), the offset in milliseconds (`i32::MIN` when
//! uncalibrated), the score, the margin, the back step in seconds (`u32::MAX`
//! when unmeasured), the rows that stepped back, the comparable minutes, the
//! unmatched minutes, the traded seconds, the window and the reach in seconds;
//! the flags, and a zero pad. [`Table::digest`] names the whole table for the
//! run identity's `clock_digest` (PARTS, READINESS ADDENDUM item 2).
//!
//! # Cost
//!
//! A stock-day is a bounded batch step, not a hot path (PARTS 11;
//! `docs/06-limits.md` L-FNO-CLOCK-01). Folding the day's seconds into the
//! per-day sparse table is linear in them. Each shift then costs one fixed step
//! per comparable minute, at most 375: two sparse-table reads, two index reads
//! and four ratio tallies. C-FNO-CLOCK-01 times that step at 1x, 10x and 100x
//! a session's seconds (`crates/cli/benches/ratio.rs`). A day whose best shift
//! lies within 59 s of zero searches 121 shifts; a wider search covers at most
//! twice the reach.

use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fmt;
use std::fs;
use std::hint::black_box;
use std::io::{Read as _, Write as _};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering as Atomic};
use std::time::Instant;

use brutex_core::blake3::Hasher;
use brutex_core::instrument::{Exchange, InstrumentKey, Kind, Segment};
use brutex_core::symbol::{SYMBOL_CAPACITY, Symbol};
use pull::calendar::{CasEligibility, DayKind, Phase, exchange_kind_of, phase_at};
use pull::gdfl_cm::{CmKind, CmRefusal, CmSource, DayListing, read_listed};
use pull::session::{Day, IstMoment, SECS_PER_MINUTE};
use pull::vendor::Venue;
use rayon::prelude::*;
use store::format::Bar;

/// The folder, under the directory a build is given, that holds the table.
pub const TABLE_DIR: &str = "fno-clock-v1";

/// The extension of one stock's file.
pub const EXTENSION: &str = "fcl";

/// The first eight bytes of every file.
pub const MAGIC: [u8; 8] = *b"BRTXFCL1";

/// The only format version this build reads or writes.
pub const VERSION: u16 = 1;

/// Bytes of a file's header.
pub const HEADER_LEN: usize = 72;

/// Bytes of one row.
pub const ROW_LEN: usize = 48;

/// The half-width, in seconds, of the first shift window: PARTS FNO-CLOCK's
/// "start ±60 s".
pub const FIRST_WINDOW_S: u32 = 60;

/// How many tick-reader refusals a build names with their key; every one is
/// counted. A bound, so a day of refusals cannot make a report unbounded.
pub const NAMED_REFUSALS: usize = 16;

/// Levels of the sparse table: two blocks of the widest, 32 seconds, cover a
/// minute, which is the longest range a shifted window queries.
const LEVELS: usize = 6;

/// Microseconds in a second: a stored bar's stamp is in microseconds.
const MICROS_PER_SECOND: i64 = 1_000_000;

/// Milliseconds in a second: the row keeps the offset in milliseconds.
const MILLIS_PER_SECOND: i64 = 1_000;

/// The bytes of a header before its digest.
const HEAD_LEN: usize = 40;

/// The digest domain of one stock's file.
const FILE_DOMAIN: &[u8] = b"brutex fno-clock-v1 file\0";

/// The digest domain of the whole table.
const TABLE_DOMAIN: &[u8] = b"brutex fno-clock-v1 table\0";

/// One stock-day the clock is calibrated for: the stock as the store spells it
/// that day, and the IST day.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Key {
    /// The stock, as the store spells it on `day` (a joined stock's caller
    /// gives the spelling `JoinMap::keys_for` names for that day).
    pub stock: Symbol,
    /// The IST day.
    pub day: Day,
}

/// What one calibration met, one bit each (D-8224).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Flags(u16);

impl Flags {
    /// The calendar of record holds no session that day.
    pub const NO_SESSION: Self = Self(1);
    /// No Zerodha minute traded in the continuous session.
    pub const NO_MINUTES: Self = Self(1 << 1);
    /// No GDFL second traded.
    pub const NO_SECONDS: Self = Self(1 << 2);
    /// No shift gave a ratio that two minutes agree on.
    pub const NO_AGREEMENT: Self = Self(1 << 3);
    /// A Zerodha minute broke an input rule.
    pub const MINUTES_MALFORMED: Self = Self(1 << 4);
    /// A GDFL second broke an input rule.
    pub const SECONDS_MALFORMED: Self = Self(1 << 5);
    /// More than one shift gave the best score; the larger `|delta|` was
    /// taken, the positive one on an equal `|delta|`.
    pub const TIED: Self = Self(1 << 6);
    /// The tick source holds no file for the stock that day.
    pub const TICKS_ABSENT: Self = Self(1 << 7);
    /// The tick reader refused the day or the file.
    pub const TICKS_REFUSED: Self = Self(1 << 8);

    /// Every flag that leaves the offset uncalibrated.
    pub const UNCALIBRATED: Self = Self(
        Self::NO_SESSION.0
            | Self::NO_MINUTES.0
            | Self::NO_SECONDS.0
            | Self::NO_AGREEMENT.0
            | Self::MINUTES_MALFORMED.0
            | Self::SECONDS_MALFORMED.0,
    );

    /// Every flag that leaves the back step unmeasured.
    pub const UNMEASURED: Self = Self(Self::TICKS_ABSENT.0 | Self::TICKS_REFUSED.0);

    /// Every flag, with the word a report prints for it.
    pub const ALL: [(&'static str, Self); 9] = [
        ("no_session", Self::NO_SESSION),
        ("no_minutes", Self::NO_MINUTES),
        ("no_seconds", Self::NO_SECONDS),
        ("no_agreement", Self::NO_AGREEMENT),
        ("minutes_malformed", Self::MINUTES_MALFORMED),
        ("seconds_malformed", Self::SECONDS_MALFORMED),
        ("tied", Self::TIED),
        ("ticks_absent", Self::TICKS_ABSENT),
        ("ticks_refused", Self::TICKS_REFUSED),
    ];

    /// Every bit a flag uses.
    const KNOWN: u16 = (1 << 9) - 1;

    /// The bits.
    #[must_use]
    pub const fn bits(self) -> u16 {
        self.0
    }

    /// The flags of `bits`, or `None` when a bit is no flag's.
    #[must_use]
    pub const fn from_bits(bits: u16) -> Option<Self> {
        if bits & !Self::KNOWN == 0 {
            Some(Self(bits))
        } else {
            None
        }
    }

    /// Whether every bit of `other` is set.
    #[must_use]
    pub const fn has(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    /// Whether any bit of `other` is set.
    #[must_use]
    pub const fn any(self, other: Self) -> bool {
        self.0 & other.0 != 0
    }

    /// These flags and `other`'s.
    #[must_use]
    pub const fn with(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

/// The vendor clock's largest step backwards over one file's rows (D-8222).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BackStep {
    /// The most, in seconds, by which a row's stamp falls below the largest
    /// stamp of the rows before it in file order; 0 when none does.
    pub step_s: u32,
    /// How many rows fall below it.
    pub rows: u32,
}

/// The back step over `stamps`, seconds of the IST day in file order.
#[must_use]
pub fn back_step(stamps: impl IntoIterator<Item = u32>) -> BackStep {
    let mut highest = 0_u32;
    let mut out = BackStep::default();
    for stamp in stamps {
        let back = highest.saturating_sub(stamp);
        if back > 0 {
            out.rows = out.rows.saturating_add(1);
            out.step_s = out.step_s.max(back);
        }
        highest = highest.max(stamp);
    }
    out
}

/// What the raw tick order gave for one stock-day.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ticks {
    /// The file was read and its rows measured.
    Measured(BackStep),
    /// The tick source holds no file for the stock that day.
    Absent,
    /// The tick reader refused the day or the file.
    Refused,
}

/// The back step of `stock` on the day `listing` lists, from the kept rows of
/// the vendor's file in file order (D-8222).
///
/// `Ok(None)` when the listing holds no file for the stock.
///
/// # Errors
///
/// Every refusal of `pull::gdfl_cm::read_listed`: the stock off the listing's
/// scope, the calendar's refusals, a file whose bytes are not the length and
/// CRC-32 the listing states, and every decode refusal.
pub fn read_back_step<S: CmSource>(
    source: &S,
    listing: &DayListing<S::Locator>,
    stock: Symbol,
) -> Result<Option<BackStep>, CmRefusal> {
    let key = InstrumentKey {
        exchange: Exchange::Nse,
        segment: Segment::Cash,
        underlying: stock,
        kind: Kind::Equity,
    };
    Ok(read_listed(source, listing, &key)?
        .map(|day| back_step(day.file.rows.iter().map(|row| row.sod))))
}

/// One stock-day's bars as the store holds them.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DayBars {
    /// GDFL one-second bars in raw paisa, each stamped at its second's open
    /// (UTC microseconds), in time order. A bar with no volume did not trade.
    pub seconds: Vec<Bar>,
    /// Zerodha one-minute bars in back-adjusted paisa, each stamped at its
    /// minute's open (UTC microseconds), in time order.
    pub minutes: Vec<Bar>,
}

/// Where a build reads each stock-day's bars from (D-8220). FNO-RUN's adapter
/// implements it over the store, with FNO-SEC-WIRE's seconds reader, so this
/// module holds no second copy of a store reader.
pub trait BarSource: Sync {
    /// The bars of `key`: empty lists when the store holds none that day.
    ///
    /// # Errors
    ///
    /// The source's own refusal, in its own words. A build stops on it and
    /// writes nothing.
    fn bars(&self, key: Key) -> Result<DayBars, String>;
}

/// The calibrated floor of one stock-day (D-8223).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Floor {
    /// Both terms measured: `max(0, offset) + back_step`, in whole seconds.
    Calibrated(u32),
    /// The offset is measured and the back step is not: `max(0, offset)`
    /// alone, which a run labels "back step unmeasured" (PARTS 8.3 lever 4).
    OffsetOnly(u32),
    /// No offset: every latency of the stock-day is "uncalibrated clock".
    Uncalibrated,
}

/// One stock-day's calibration: a row of the table (D-8220).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClockRow {
    /// The stock-day.
    pub key: Key,
    /// Where GDFL's stamps sit against Zerodha's minute boundaries, in
    /// milliseconds (whole seconds), positive when GDFL stamps later; `None`
    /// when the day could not be calibrated.
    pub offset_ms: Option<i32>,
    /// Prices that agree at the chosen shift.
    pub score: u32,
    /// The chosen shift's score less the best score of any other shift
    /// searched; 0 on a tie.
    pub margin: u32,
    /// The largest backward step of the vendor's clock, in seconds; `None`
    /// when the raw tick order was not read.
    pub back_step_s: Option<u32>,
    /// Rows stamped below the largest stamp before them.
    pub back_rows: u32,
    /// Zerodha minutes compared at every shift.
    pub minutes: u32,
    /// Of them, minutes whose window held no traded second at the chosen shift.
    pub unmatched: u32,
    /// GDFL seconds that traded.
    pub seconds: u32,
    /// The half-width of the last shift window searched, in seconds.
    pub window_s: u32,
    /// The largest `|delta|` at which a minute's window can meet a traded
    /// second, in seconds.
    pub reach_s: u32,
    /// What the calibration met.
    pub flags: Flags,
}

impl ClockRow {
    /// A row for `key` that met nothing yet.
    const fn blank(key: Key) -> Self {
        Self {
            key,
            offset_ms: None,
            score: 0,
            margin: 0,
            back_step_s: None,
            back_rows: 0,
            minutes: 0,
            unmatched: 0,
            seconds: 0,
            window_s: 0,
            reach_s: 0,
            flags: Flags(0),
        }
    }

    /// The floor: `max(0, offset) + back_step` in whole seconds, and which of
    /// its terms were measured (D-8223).
    #[must_use]
    pub fn floor(&self) -> Floor {
        let Some(offset_ms) = self.offset_ms else {
            return Floor::Uncalibrated;
        };
        let lead =
            u32::try_from(i64::from(offset_ms.max(0)) / MILLIS_PER_SECOND).unwrap_or(u32::MAX);
        self.back_step_s.map_or(Floor::OffsetOnly(lead), |step| {
            Floor::Calibrated(lead.saturating_add(step))
        })
    }

    /// The row's bytes ([`ROW_LEN`] of them), appended to `out`.
    fn write(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(&self.key.day.days_from_epoch().to_le_bytes());
        out.extend_from_slice(&self.offset_ms.unwrap_or(i32::MIN).to_le_bytes());
        for field in [
            self.score,
            self.margin,
            self.back_step_s.unwrap_or(u32::MAX),
            self.back_rows,
            self.minutes,
            self.unmatched,
            self.seconds,
            self.window_s,
            self.reach_s,
        ] {
            out.extend_from_slice(&field.to_le_bytes());
        }
        out.extend_from_slice(&self.flags.bits().to_le_bytes());
        out.extend_from_slice(&0_u16.to_le_bytes());
    }

    /// The row `bytes` hold for `stock`, or `None` when they are not a row
    /// this build writes: a day that is no date, an unknown flag, a nonzero
    /// pad, or fields that disagree with the flags ([`ClockRow::agrees`]).
    fn read(stock: Symbol, bytes: &[u8]) -> Option<Self> {
        let day = Day::from_days(u32_at(bytes, 0)).ok()?;
        let offset = i32::from_le_bytes(u32_at(bytes, 4).to_le_bytes());
        let back = u32_at(bytes, 16);
        let flags = Flags::from_bits(u16_at(bytes, 44))?;
        let row = Self {
            key: Key { stock, day },
            offset_ms: (offset != i32::MIN).then_some(offset),
            score: u32_at(bytes, 8),
            margin: u32_at(bytes, 12),
            back_step_s: (back != u32::MAX).then_some(back),
            back_rows: u32_at(bytes, 20),
            minutes: u32_at(bytes, 24),
            unmatched: u32_at(bytes, 28),
            seconds: u32_at(bytes, 32),
            window_s: u32_at(bytes, 36),
            reach_s: u32_at(bytes, 40),
            flags,
        };
        (u16_at(bytes, 46) == 0 && row.agrees()).then_some(row)
    }

    /// Whether the fields say what the flags say: an offset exactly when no
    /// flag uncalibrates it, in whole seconds; a back step exactly when no
    /// flag leaves it unmeasured, and never both tick flags.
    fn agrees(&self) -> bool {
        let offset_ok = match self.offset_ms {
            Some(ms) => {
                !self.flags.any(Flags::UNCALIBRATED) && i64::from(ms) % MILLIS_PER_SECOND == 0
            }
            None => self.flags.any(Flags::UNCALIBRATED),
        };
        let back_ok = match self.back_step_s {
            Some(_) => !self.flags.any(Flags::UNMEASURED),
            None => self.flags.any(Flags::UNMEASURED) && !self.flags.has(Flags::UNMEASURED),
        };
        offset_ok && back_ok
    }
}

/// The little-endian `u32` at `at` in `bytes`, 0 past the end.
fn u32_at(bytes: &[u8], at: usize) -> u32 {
    bytes
        .get(at..at.saturating_add(4))
        .and_then(|field| <[u8; 4]>::try_from(field).ok())
        .map_or(0, u32::from_le_bytes)
}

/// The little-endian `u16` at `at` in `bytes`, 0 past the end.
fn u16_at(bytes: &[u8], at: usize) -> u16 {
    bytes
        .get(at..at.saturating_add(2))
        .and_then(|field| <[u8; 2]>::try_from(field).ok())
        .map_or(0, u16::from_le_bytes)
}

// ------------------------------------------------------------ calibration --

/// One bar's four prices, in paisa.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Prices {
    open: i64,
    high: i64,
    low: i64,
    close: i64,
}

impl Prices {
    const fn of(bar: &Bar) -> Self {
        Self {
            open: bar.open,
            high: bar.high,
            low: bar.low,
            close: bar.close,
        }
    }

    /// Each of these four prices beside the same one of `gdfl`'s, as
    /// `(zerodha, gdfl)`.
    const fn beside(self, gdfl: Self) -> [(i64, i64); 4] {
        [
            (self.open, gdfl.open),
            (self.high, gdfl.high),
            (self.low, gdfl.low),
            (self.close, gdfl.close),
        ]
    }
}

/// One Zerodha minute compared at every shift.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Minute {
    /// The UTC second the minute opens.
    start: i64,
    /// Its back-adjusted prices.
    prices: Prices,
}

/// What a day's bars held: a list that broke an input rule, none that count,
/// or the ones that do.
#[derive(Debug)]
enum Input<T> {
    Malformed,
    Empty,
    Held(T),
}

/// The UTC second `bar` opens at, and its IST moment, when it opens on a whole
/// `unit` of seconds on `day`.
fn stamp(bar: &Bar, day: Day, unit: i64) -> Option<(i64, IstMoment)> {
    let second = bar.ts_micros.div_euclid(MICROS_PER_SECOND);
    let at = IstMoment::from_epoch_secs(second).ok()?;
    let whole = bar
        .ts_micros
        .rem_euclid(MICROS_PER_SECOND.saturating_mul(unit))
        == 0;
    (whole && at.day() == day).then_some((second, at))
}

/// Whether `bar` is priced as a bar: at least one paisa, its open and close
/// inside its low and high, and no negative volume.
fn priced(bar: &Bar) -> bool {
    bar.low >= 1
        && bar.low <= bar.open.min(bar.close)
        && bar.open.max(bar.close) <= bar.high
        && bar.volume >= 0
}

/// The comparable minutes of `bars` on `day` (D-8221): every minute must be
/// stamped on a whole minute of `day`, priced, and later than the one before;
/// the ones kept traded and are continuous trading for the NSE cash venue.
fn minutes_of(day: Day, bars: &[Bar]) -> Input<Vec<Minute>> {
    let epoch_day = i64::from(day.days_from_epoch());
    let mut kept = Vec::with_capacity(bars.len());
    let mut before = None;
    for bar in bars {
        let Some((start, at)) = stamp(bar, day, SECS_PER_MINUTE) else {
            return Input::Malformed;
        };
        if !priced(bar) || before.is_some_and(|last| start <= last) {
            return Input::Malformed;
        }
        before = Some(start);
        let minute = u16::try_from(at.minute_of_day()).unwrap_or(u16::MAX);
        let phase = phase_at(Venue::NseCash, epoch_day, minute, CasEligibility::Unknown);
        if bar.volume > 0 && phase == Phase::Continuous {
            kept.push(Minute {
                start,
                prices: Prices::of(bar),
            });
        }
    }
    if kept.is_empty() {
        Input::Empty
    } else {
        Input::Held(kept)
    }
}

/// One day's traded seconds on a dense second grid, with a sparse table of
/// range highs and lows over it, so any window of a minute or less is read in
/// a fixed number of steps.
#[derive(Debug)]
struct Grid {
    /// The UTC second of index 0: the first traded second.
    origin: i64,
    /// Seconds from the first traded second to the last, both included.
    len: usize,
    /// How many seconds traded.
    traded: usize,
    /// Each traded second's open; 0 where none traded.
    open: Vec<i64>,
    /// Each traded second's close; 0 where none traded.
    close: Vec<i64>,
    /// `highs[k][i]`: the highest high in `[i, i + 2^k)`, `i64::MIN` when none
    /// traded there.
    highs: Vec<Vec<i64>>,
    /// `lows[k][i]`: the lowest low in `[i, i + 2^k)`, `i64::MAX` when none
    /// traded there.
    lows: Vec<Vec<i64>>,
    /// The first traded index at or after `i`; `len` when none.
    next: Vec<usize>,
    /// One past the last traded index at or before `i`; 0 when none.
    prev: Vec<usize>,
}

impl Grid {
    /// The grid of `day`'s traded seconds: every bar stamped on a whole second
    /// of `day`, priced, and later than the one before. A bar with no volume
    /// did not trade (SPEC 0: a traded second's volume is above zero).
    fn of_day(day: Day, bars: &[Bar]) -> Input<Self> {
        let mut traded = Vec::with_capacity(bars.len());
        let mut before = None;
        for bar in bars {
            let Some((second, _)) = stamp(bar, day, 1) else {
                return Input::Malformed;
            };
            if !priced(bar) || before.is_some_and(|last| second <= last) {
                return Input::Malformed;
            }
            before = Some(second);
            if bar.volume > 0 {
                traded.push((second, Prices::of(bar)));
            }
        }
        let (Some(&(origin, _)), Some(&(last, _))) = (traded.first(), traded.last()) else {
            return Input::Empty;
        };
        let len = usize::try_from(last - origin)
            .unwrap_or(0)
            .saturating_add(1);
        let mut cells = traded.iter().peekable();
        let dense: Vec<Option<Prices>> = (origin
            ..origin.saturating_add(i64::try_from(len).unwrap_or(0)))
            .map(|second| {
                cells
                    .next_if(|&&(at, _)| at == second)
                    .map(|&(_, prices)| prices)
            })
            .collect();
        Input::Held(Self::build(origin, &dense))
    }

    /// The grid of `cells`, one per second from `origin`.
    fn build(origin: i64, cells: &[Option<Prices>]) -> Self {
        let len = cells.len();
        let open = cells
            .iter()
            .map(|cell| cell.map_or(0, |p| p.open))
            .collect();
        let close = cells
            .iter()
            .map(|cell| cell.map_or(0, |p| p.close))
            .collect();
        let mut high: Vec<i64> = cells
            .iter()
            .map(|cell| cell.map_or(i64::MIN, |p| p.high))
            .collect();
        let mut low: Vec<i64> = cells
            .iter()
            .map(|cell| cell.map_or(i64::MAX, |p| p.low))
            .collect();
        let mut highs = Vec::with_capacity(LEVELS);
        let mut lows = Vec::with_capacity(LEVELS);
        for level in 1..LEVELS {
            let half = 1_usize << (level - 1);
            let count = len.saturating_sub(2 * half - 1);
            let wider_high = (0..count)
                .map(|at| highest(&high, at).max(highest(&high, at + half)))
                .collect();
            let wider_low = (0..count)
                .map(|at| lowest(&low, at).min(lowest(&low, at + half)))
                .collect();
            highs.push(std::mem::replace(&mut high, wider_high));
            lows.push(std::mem::replace(&mut low, wider_low));
        }
        highs.push(high);
        lows.push(low);
        let mut next = vec![len; len];
        let mut upcoming = len;
        for (at, slot) in next.iter_mut().enumerate().rev() {
            if cells.get(at).is_some_and(Option::is_some) {
                upcoming = at;
            }
            *slot = upcoming;
        }
        let mut prev = vec![0; len];
        let mut seen = 0;
        for (at, slot) in prev.iter_mut().enumerate() {
            if cells.get(at).is_some_and(Option::is_some) {
                seen = at + 1;
            }
            *slot = seen;
        }
        let traded = cells.iter().filter(|cell| cell.is_some()).count();
        Self {
            origin,
            len,
            traded,
            open,
            close,
            highs,
            lows,
            next,
            prev,
        }
    }

    /// The four prices of the traded seconds in the minute-long window that
    /// opens at UTC second `start`, folded as one bar; `None` when no second
    /// traded there.
    fn window(&self, start: i64) -> Option<Prices> {
        let from = start.saturating_sub(self.origin);
        let to = from.saturating_add(SECS_PER_MINUTE);
        let from = usize::try_from(from.max(0)).unwrap_or(usize::MAX);
        let to = usize::try_from(to.max(0)).unwrap_or(0).min(self.len);
        if from >= to {
            return None;
        }
        let first = self.next.get(from).copied().unwrap_or(self.len);
        if first >= to {
            return None;
        }
        let last = self
            .prev
            .get(to - 1)
            .copied()
            .unwrap_or(0)
            .saturating_sub(1);
        let level = (to - from).ilog2();
        let width = 1_usize << level;
        let row = usize::try_from(level).unwrap_or(0);
        let highs = self.highs.get(row).map_or(&[][..], Vec::as_slice);
        let lows = self.lows.get(row).map_or(&[][..], Vec::as_slice);
        Some(Prices {
            open: self.open.get(first).copied().unwrap_or(0),
            high: highest(highs, from).max(highest(highs, to - width)),
            low: lowest(lows, from).min(lowest(lows, to - width)),
            close: self.close.get(last).copied().unwrap_or(0),
        })
    }

    /// The largest `|delta|` at which some minute's window can still meet a
    /// traded second: past it every window is empty and scores nothing.
    fn reach(&self, minutes: &[Minute]) -> i64 {
        let first = minutes.first().map_or(0, |minute| minute.start);
        let last = minutes.last().map_or(0, |minute| minute.start);
        let end = self
            .origin
            .saturating_add(i64::try_from(self.len).unwrap_or(0))
            - 1;
        let later = end - first;
        let earlier = self.origin - (last + SECS_PER_MINUTE - 1);
        later.abs().max(earlier.abs())
    }
}

/// `levels[at]`, or the lowest high when `at` is past the end.
fn highest(levels: &[i64], at: usize) -> i64 {
    levels.get(at).copied().unwrap_or(i64::MIN)
}

/// `levels[at]`, or the highest low when `at` is past the end.
fn lowest(levels: &[i64], at: usize) -> i64 {
    levels.get(at).copied().unwrap_or(i64::MAX)
}

/// The exact ratio `gdfl / zerodha` as a reduced fraction. Both are prices of
/// at least one paisa, so the divisor is at least one.
fn ratio(gdfl: i64, zerodha: i64) -> (i64, i64) {
    let mut a = gdfl;
    let mut b = zerodha;
    while b != 0 {
        (a, b) = (b, a % b);
    }
    (gdfl / a, zerodha / a)
}

/// How many prices, and of how many minutes, gave one ratio at one shift.
#[derive(Debug, Clone, Copy, Default)]
struct Support {
    prices: u32,
    minutes: u32,
    /// One past the index of the last minute counted.
    last: usize,
}

/// The ratios one shift's prices gave.
type Tally = HashMap<(i64, i64), Support>;

/// Tallies minute `index`'s four prices at shift `delta`, and returns the
/// most prices any ratio it touched now has from two minutes or more; 0 when
/// its window holds no traded second.
fn tally_minute(grid: &Grid, tally: &mut Tally, index: usize, minute: &Minute, delta: i64) -> u32 {
    let Some(gdfl) = grid.window(minute.start.saturating_add(delta)) else {
        return 0;
    };
    let mut top = 0;
    for (zerodha, gdfl) in minute.prices.beside(gdfl) {
        let support = tally.entry(ratio(gdfl, zerodha)).or_default();
        support.prices = support.prices.saturating_add(1);
        if support.last != index + 1 {
            support.minutes = support.minutes.saturating_add(1);
            support.last = index + 1;
        }
        if support.minutes >= 2 {
            top = top.max(support.prices);
        }
    }
    top
}

/// One shift and its score.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Shift {
    delta: i64,
    score: u32,
}

impl Shift {
    /// Whether this shift is the reading to take over `other`: the higher
    /// score; on a tie the larger `|delta|`, then the positive one (D-8221).
    fn beats(self, other: Self) -> bool {
        (self.score, self.delta.unsigned_abs(), self.delta)
            > (other.score, other.delta.unsigned_abs(), other.delta)
    }
}

/// The shifts searched so far: the one taken, the best score, how many
/// shifts reached it, and the best score below it.
#[derive(Debug, Clone, Copy)]
struct Board {
    best: Shift,
    top: u32,
    ties: u32,
    below: u32,
}

impl Board {
    const fn new() -> Self {
        Self {
            best: Shift { delta: 0, score: 0 },
            top: 0,
            ties: 0,
            below: 0,
        }
    }

    fn record(&mut self, shift: Shift) {
        if shift.beats(self.best) {
            self.best = shift;
        }
        match shift.score.cmp(&self.top) {
            Ordering::Greater => {
                self.below = self.top;
                self.top = shift.score;
                self.ties = 1;
            }
            Ordering::Equal => self.ties = self.ties.saturating_add(1),
            Ordering::Less => self.below = self.below.max(shift.score),
        }
    }

    /// The best score less the best of any other shift; 0 on a tie.
    const fn margin(self) -> u32 {
        if self.ties > 1 {
            0
        } else {
            self.top - self.below
        }
    }
}

/// The shift search over one day.
struct Search<'a> {
    grid: &'a Grid,
    minutes: &'a [Minute],
    tally: Tally,
    board: Board,
}

impl<'a> Search<'a> {
    fn new(grid: &'a Grid, minutes: &'a [Minute]) -> Self {
        Self {
            grid,
            minutes,
            tally: HashMap::with_capacity(minutes.len().saturating_mul(4)),
            board: Board::new(),
        }
    }

    /// Scores every shift in `from..=to`.
    fn sweep(&mut self, from: i64, to: i64) {
        for delta in from..=to {
            self.tally.clear();
            let mut score = 0;
            for (index, minute) in self.minutes.iter().enumerate() {
                score = score.max(tally_minute(
                    self.grid,
                    &mut self.tally,
                    index,
                    minute,
                    delta,
                ));
            }
            self.board.record(Shift { delta, score });
        }
    }
}

/// Calibrates one stock-day from its bars and its raw tick order (D-8221,
/// D-8222). Every outcome is a row: what the day lacked is in its flags.
#[must_use]
pub fn calibrate(key: Key, bars: &DayBars, ticks: Ticks) -> ClockRow {
    let mut row = ClockRow::blank(key);
    match ticks {
        Ticks::Measured(step) => {
            row.back_step_s = Some(step.step_s);
            row.back_rows = step.rows;
        }
        Ticks::Absent => row.flags = row.flags.with(Flags::TICKS_ABSENT),
        Ticks::Refused => row.flags = row.flags.with(Flags::TICKS_REFUSED),
    }
    if !matches!(
        exchange_kind_of(i64::from(key.day.days_from_epoch())),
        DayKind::Open(_)
    ) {
        row.flags = row.flags.with(Flags::NO_SESSION);
        return row;
    }
    let minutes = match minutes_of(key.day, &bars.minutes) {
        Input::Malformed => {
            row.flags = row.flags.with(Flags::MINUTES_MALFORMED);
            None
        }
        Input::Empty => {
            row.flags = row.flags.with(Flags::NO_MINUTES);
            None
        }
        Input::Held(minutes) => Some(minutes),
    };
    let grid = match Grid::of_day(key.day, &bars.seconds) {
        Input::Malformed => {
            row.flags = row.flags.with(Flags::SECONDS_MALFORMED);
            None
        }
        Input::Empty => {
            row.flags = row.flags.with(Flags::NO_SECONDS);
            None
        }
        Input::Held(grid) => Some(grid),
    };
    let (Some(minutes), Some(grid)) = (minutes, grid) else {
        return row;
    };
    find_offset(row, &grid, &minutes)
}

/// The shift search of one day with comparable minutes and traded seconds,
/// written into `row`.
fn find_offset(mut row: ClockRow, grid: &Grid, minutes: &[Minute]) -> ClockRow {
    row.minutes = u32::try_from(minutes.len()).unwrap_or(u32::MAX);
    row.seconds = u32::try_from(grid.traded).unwrap_or(u32::MAX);
    let reach = grid.reach(minutes);
    let mut half = i64::from(FIRST_WINDOW_S);
    let mut search = Search::new(grid, minutes);
    search.sweep(-half.min(reach), half.min(reach));
    loop {
        if search.board.best.delta.unsigned_abs() != half.unsigned_abs() {
            break;
        }
        if half >= reach {
            break;
        }
        let wider = half.saturating_mul(2);
        search.sweep(-wider.min(reach), -half - 1);
        search.sweep(half + 1, wider.min(reach));
        half = wider;
    }
    row.window_s = u32::try_from(half).unwrap_or(u32::MAX);
    row.reach_s = u32::try_from(reach).unwrap_or(u32::MAX);
    let board = search.board;
    if board.top == 0 {
        row.flags = row.flags.with(Flags::NO_AGREEMENT);
        return row;
    }
    if board.ties > 1 {
        row.flags = row.flags.with(Flags::TIED);
    }
    let best = board.best;
    row.offset_ms =
        Some(i32::try_from(best.delta.saturating_mul(MILLIS_PER_SECOND)).unwrap_or(i32::MAX));
    row.score = board.top;
    row.margin = board.margin();
    let unmatched = minutes
        .iter()
        .filter(|minute| {
            grid.window(minute.start.saturating_add(best.delta))
                .is_none()
        })
        .count();
    row.unmatched = u32::try_from(unmatched).unwrap_or(u32::MAX);
    row
}

// ------------------------------------------------------------------ table --

/// A file shorter than its header.
const SHORT: &str = "the file is shorter than its header";
/// A file that does not open with [`MAGIC`].
const NOT_MAGIC: &str = "the file does not start with BRTXFCL1";
/// A file of another format version.
const NOT_VERSION: &str = "the file is not format version 1";
/// A file whose header states another row length.
const NOT_STRIDE: &str = "the file's header states another row length";
/// A file whose length is not its header and its stated rows.
const NOT_LENGTH: &str = "the file's length is not its header and its stated rows";
/// A file whose header names another stock than its name.
const NOT_STOCK: &str = "the file's header names another stock than its name";
/// A file whose digest is not its bytes'.
const NOT_DIGEST: &str = "the file's digest is not its bytes'";
/// A row this build does not write.
const NOT_ROW: &str = "a row is not one this build writes";
/// Rows out of day order, or two of one day.
const NOT_ORDER: &str = "the rows are not in day order, one a day";

/// `stock`'s bytes, zero-padded to a symbol's capacity.
fn padded(stock: Symbol) -> [u8; SYMBOL_CAPACITY] {
    let mut out = [0_u8; SYMBOL_CAPACITY];
    for (slot, byte) in out.iter_mut().zip(stock.as_str().bytes()) {
        *slot = byte;
    }
    out
}

/// The digest of a file: its domain, its first [`HEAD_LEN`] bytes and its rows.
fn file_digest(head: &[u8], rows: &[u8]) -> [u8; 32] {
    let mut hasher = Hasher::new();
    hasher.update(FILE_DOMAIN);
    hasher.update(head);
    hasher.update(rows);
    hasher.finalize()
}

/// The bytes of `stock`'s file holding `rows`, which are in day order.
fn encode(stock: Symbol, rows: &[ClockRow]) -> Vec<u8> {
    let mut body = Vec::with_capacity(rows.len().saturating_mul(ROW_LEN));
    for row in rows {
        row.write(&mut body);
    }
    let mut out = Vec::with_capacity(HEADER_LEN.saturating_add(body.len()));
    out.extend_from_slice(&MAGIC);
    out.extend_from_slice(&VERSION.to_le_bytes());
    out.extend_from_slice(&u16::try_from(ROW_LEN).unwrap_or(u16::MAX).to_le_bytes());
    out.extend_from_slice(&u32::try_from(rows.len()).unwrap_or(u32::MAX).to_le_bytes());
    out.extend_from_slice(&padded(stock));
    let digest = file_digest(&out, &body);
    out.extend_from_slice(&digest);
    out.extend_from_slice(&body);
    out
}

/// The rows and the digest of `stock`'s file `bytes`, or why it is not a file
/// this build writes.
fn decode(stock: Symbol, bytes: &[u8]) -> Result<(Vec<ClockRow>, [u8; 32]), &'static str> {
    if bytes.len() < HEADER_LEN {
        return Err(SHORT);
    }
    let head = bytes.get(..HEAD_LEN).unwrap_or_default();
    let stated = bytes.get(HEAD_LEN..HEADER_LEN).unwrap_or_default();
    let body = bytes.get(HEADER_LEN..).unwrap_or_default();
    if head.get(..MAGIC.len()) != Some(&MAGIC[..]) {
        return Err(NOT_MAGIC);
    }
    if u16_at(head, 8) != VERSION {
        return Err(NOT_VERSION);
    }
    if usize::from(u16_at(head, 10)) != ROW_LEN {
        return Err(NOT_STRIDE);
    }
    let count = usize::try_from(u32_at(head, 12)).unwrap_or(usize::MAX);
    if count.checked_mul(ROW_LEN) != Some(body.len()) {
        return Err(NOT_LENGTH);
    }
    if head.get(16..HEAD_LEN) != Some(&padded(stock)[..]) {
        return Err(NOT_STOCK);
    }
    let digest = file_digest(head, body);
    if stated != digest {
        return Err(NOT_DIGEST);
    }
    let mut rows: Vec<ClockRow> = Vec::with_capacity(count);
    for chunk in body.chunks_exact(ROW_LEN) {
        let row = ClockRow::read(stock, chunk).ok_or(NOT_ROW)?;
        if rows
            .last()
            .is_some_and(|before| before.key.day >= row.key.day)
        {
            return Err(NOT_ORDER);
        }
        rows.push(row);
    }
    Ok((rows, digest))
}

/// `stock`'s file name.
fn file_name(stock: Symbol) -> String {
    format!("{}.{EXTENSION}", stock.as_str())
}

/// The stock a file name names: `<STOCK>.fcl`, spelt exactly as the table
/// spells the stock.
fn stock_of(name: &str) -> Option<Symbol> {
    let stem = name.strip_suffix(EXTENSION)?.strip_suffix('.')?;
    let stock = Symbol::new(stem).ok()?;
    (stock.as_str() == stem).then_some(stock)
}

/// The bytes of the file at `path`, opened through the store's one reading
/// door (`store::safe_open`, which refuses a link, a pipe, a device or a
/// directory at the name); `Ok(None)` when no file has the name.
fn read_held(path: &Path) -> Result<Option<Vec<u8>>, String> {
    match store::safe_open::open_read_len(path) {
        Err(why) if why.is_missing() => Ok(None),
        Err(why) => Err(why.to_string()),
        Ok((mut file, len)) => {
            let mut bytes = Vec::with_capacity(usize::try_from(len).unwrap_or(0));
            file.read_to_end(&mut bytes)
                .map(|_| Some(bytes))
                .map_err(|why| why.to_string())
        }
    }
}

/// Why the table could not be read (D-8224).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TableRefusal {
    /// The folder or a file in it could not be read.
    Unreadable {
        /// What could not be read.
        path: PathBuf,
        /// The reader's words.
        why: String,
    },
    /// An entry that is neither a stock's file nor a hidden temporary.
    Stray {
        /// The entry.
        path: PathBuf,
    },
    /// A stock's file failed its checks.
    Corrupt {
        /// The file.
        path: PathBuf,
        /// The check it failed.
        why: &'static str,
    },
}

impl fmt::Display for TableRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unreadable { path, why } => {
                write!(f, "the clock table cannot read {}: {why}", path.display())
            }
            Self::Stray { path } => write!(
                f,
                "the clock table holds {}, which is neither a stock's file nor a hidden temporary",
                path.display()
            ),
            Self::Corrupt { path, why } => {
                write!(f, "the clock table's {} is refused: {why}", path.display())
            }
        }
    }
}

impl std::error::Error for TableRefusal {}

/// The table: every row of every stock's file, read and checked whole, so a
/// lookup reads no file.
#[derive(Debug)]
pub struct Table {
    rows: HashMap<Key, ClockRow>,
    files: BTreeMap<Symbol, [u8; 32]>,
    leftovers: usize,
}

impl Table {
    /// Opens the table under `out` (`<out>/fno-clock-v1/`): every stock's file
    /// is read through the store's reading door and checked whole, its digest
    /// included. Hidden entries are a killed writer's temporaries: counted,
    /// never read.
    ///
    /// # Errors
    ///
    /// [`TableRefusal::Unreadable`] when the folder or a file cannot be read
    /// (a folder that does not exist included: a run with no table is refused,
    /// never run as "uncalibrated" everywhere), [`TableRefusal::Stray`] for an
    /// entry that is no stock's file, and [`TableRefusal::Corrupt`] for a file
    /// that fails its checks.
    pub fn open(out: &Path) -> Result<Self, TableRefusal> {
        let dir = out.join(TABLE_DIR);
        let entries = fs::read_dir(&dir)
            .and_then(|walk| {
                walk.map(|entry| entry.map(|found| (found.file_name(), found.path())))
                    .collect::<std::io::Result<Vec<_>>>()
            })
            .map_err(|why| TableRefusal::Unreadable {
                path: dir.clone(),
                why: why.to_string(),
            })?;
        let mut held = BTreeMap::new();
        let mut leftovers = 0;
        for (name, path) in entries {
            match name.to_str() {
                Some(text) if text.starts_with('.') => leftovers += 1,
                Some(text) => match stock_of(text) {
                    Some(stock) => {
                        held.insert(stock, path);
                    }
                    None => return Err(TableRefusal::Stray { path }),
                },
                None => return Err(TableRefusal::Stray { path }),
            }
        }
        let mut decoded = Vec::with_capacity(held.len());
        let mut total = 0_usize;
        for (stock, path) in held {
            let bytes = read_held(&path)
                .map_err(|why| TableRefusal::Unreadable {
                    path: path.clone(),
                    why,
                })?
                .unwrap_or_default();
            let (rows, digest) =
                decode(stock, &bytes).map_err(|why| TableRefusal::Corrupt { path, why })?;
            total = total.saturating_add(rows.len());
            decoded.push((stock, rows, digest));
        }
        let mut rows = HashMap::with_capacity(total);
        let mut files = BTreeMap::new();
        for (stock, held_rows, digest) in decoded {
            for row in held_rows {
                rows.insert(row.key, row);
            }
            files.insert(stock, digest);
        }
        Ok(Self {
            rows,
            files,
            leftovers,
        })
    }

    /// The row of `key`, or `None` when the table holds none: one hash probe.
    #[must_use]
    pub fn get(&self, key: Key) -> Option<&ClockRow> {
        self.rows.get(&key)
    }

    /// Rows in the table.
    #[must_use]
    pub fn len(&self) -> usize {
        self.rows.len()
    }

    /// Whether the table holds no row.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    /// Hidden temporaries a killed writer left in the folder.
    #[must_use]
    pub const fn leftovers(&self) -> usize {
        self.leftovers
    }

    /// The table's digest: BLAKE3 of a domain tag and, for every stock in
    /// order, its padded name and its file's digest. The run identity's
    /// `clock_digest` (PARTS, READINESS ADDENDUM item 2).
    #[must_use]
    pub fn digest(&self) -> [u8; 32] {
        let mut hasher = Hasher::new();
        hasher.update(TABLE_DOMAIN);
        for (stock, digest) in &self.files {
            hasher.update(&padded(*stock));
            hasher.update(digest);
        }
        hasher.finalize()
    }
}

// ------------------------------------------------------------------ build --

/// Why a build stopped (D-8224).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BuildRefusal {
    /// The bar source refused a key. Nothing was written.
    Bars {
        /// The first key refused, in (day, stock) order.
        key: Key,
        /// The source's words.
        why: String,
    },
    /// A stock's file already in the table failed its checks, so its other
    /// rows cannot be carried into the rebuilt file. Nothing was written.
    Corrupt {
        /// The file.
        path: PathBuf,
        /// The check it failed.
        why: &'static str,
    },
    /// A folder or a file could not be made, read, written, flushed or renamed.
    /// A file written before it is whole and stays; none after it was written.
    Io {
        /// What failed.
        path: PathBuf,
        /// The system's words.
        why: String,
    },
    /// The table, read again after the writes, refused.
    Table(TableRefusal),
}

impl fmt::Display for BuildRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Bars { key, why } => write!(
                f,
                "the clock build is refused: the bars of {} on {} could not be read: {why}",
                key.stock.as_str(),
                key.day
            ),
            Self::Corrupt { path, why } => write!(
                f,
                "the clock build is refused: {} is already in the table and fails its checks: {why}",
                path.display()
            ),
            Self::Io { path, why } => {
                write!(f, "the clock build stopped at {}: {why}", path.display())
            }
            Self::Table(why) => write!(f, "the clock build wrote its files, then: {why}"),
        }
    }
}

impl std::error::Error for BuildRefusal {}

/// What a build did. FNO-RUN's `fno clock build` verb prints and logs it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Built {
    /// Distinct keys calibrated.
    pub keys: usize,
    /// Stock files written.
    pub written: usize,
    /// Stock files whose bytes were already the build's, left untouched.
    pub unchanged: usize,
    /// Keys whose floor is [`Floor::Calibrated`].
    pub calibrated: usize,
    /// Keys whose floor is [`Floor::OffsetOnly`].
    pub offset_only: usize,
    /// Keys whose floor is [`Floor::Uncalibrated`].
    pub uncalibrated: usize,
    /// Keys carrying each flag, in [`Flags::ALL`]'s order.
    pub flagged: [usize; 9],
    /// Keys whose raw tick order the tick reader refused.
    pub tick_refusals: usize,
    /// The first [`NAMED_REFUSALS`] of them, in (day, stock) order, with the
    /// reader's words.
    pub named: Vec<(Key, String)>,
    /// Hidden temporaries in the folder after the build.
    pub leftovers: usize,
    /// Rows in the whole table after the build.
    pub rows: usize,
    /// The whole table's digest after the build ([`Table::digest`]).
    pub digest: [u8; 32],
}

impl Built {
    /// Counts `row` and, when the tick reader refused it, its words.
    fn count(&mut self, row: &ClockRow, said: Option<String>) {
        match row.floor() {
            Floor::Calibrated(_) => self.calibrated += 1,
            Floor::OffsetOnly(_) => self.offset_only += 1,
            Floor::Uncalibrated => self.uncalibrated += 1,
        }
        for (slot, (_, flag)) in self.flagged.iter_mut().zip(Flags::ALL) {
            if row.flags.has(flag) {
                *slot += 1;
            }
        }
        if let Some(why) = said {
            self.tick_refusals += 1;
            if self.named.len() < NAMED_REFUSALS {
                self.named.push((row.key, why));
            }
        }
    }
}

/// Calibrates every key of `keys` from `bars` and the raw tick order in
/// `ticks`, and writes each stock's rows into `<out>/fno-clock-v1/` (D-8220).
///
/// The key list is the build's one input (PARTS, READINESS ADDENDUM item 8):
/// a full build passes every key and a list rebuild passes the keys to redo.
/// Each listed key's row is replaced or added, every other row of its stock's
/// file is carried byte for byte, and no other stock's file is opened for
/// writing, so a list rebuild leaves exactly the bytes a full build over the
/// same data would. A file whose bytes would not change is not rewritten.
/// Keys are calibrated in parallel by day, each day's tick listing read once;
/// rows are pure functions of their key's data, so the bytes do not depend on
/// the order work finished in. Every file is written to a hidden temporary
/// through `store::staging::create`, flushed and renamed into place.
///
/// # Errors
///
/// [`BuildRefusal::Bars`] for the first key, in (day, stock) order, whose bars
/// the source refused, and [`BuildRefusal::Corrupt`] for a stock file already
/// there that fails its checks: both before anything is written.
/// [`BuildRefusal::Io`] for a folder or file that cannot be made, read,
/// written, flushed or renamed, and [`BuildRefusal::Table`] when the table,
/// read again after the writes, refuses.
pub fn build<B: BarSource, S: CmSource + Sync>(
    keys: &[Key],
    bars: &B,
    ticks: &S,
    out: &Path,
) -> Result<Built, BuildRefusal> {
    build_with(keys, bars, ticks, out, &staging_name)
}

/// Temporaries this process has named.
static STAGED: AtomicU64 = AtomicU64::new(0);

/// The hidden temporary a writer fills before renaming it over `stock`'s file:
/// named for this process and this call, so two writers never share one.
fn staging_name(dir: &Path, stock: Symbol) -> PathBuf {
    dir.join(format!(
        ".{}.{EXTENSION}.tmp.{}.{}",
        stock.as_str(),
        std::process::id(),
        STAGED.fetch_add(1, Atomic::Relaxed)
    ))
}

/// One key's outcome: its row and the tick reader's refusal, if any; or the
/// bar source's refusal.
type Outcome = Result<(ClockRow, Option<String>), BuildRefusal>;

/// [`build`], with the temporary of each stock's file named by `temporary`, so
/// a test can plant what a killed writer leaves.
fn build_with<B: BarSource, S: CmSource + Sync>(
    keys: &[Key],
    bars: &B,
    ticks: &S,
    out: &Path,
    temporary: &dyn Fn(&Path, Symbol) -> PathBuf,
) -> Result<Built, BuildRefusal> {
    let mut distinct = BTreeSet::new();
    for &key in keys {
        distinct.insert(key);
    }
    let mut by_day: BTreeMap<Day, Vec<Key>> = BTreeMap::new();
    for &key in &distinct {
        by_day.entry(key.day).or_default().push(key);
    }
    let days: Vec<(Day, Vec<Key>)> = by_day.into_iter().collect();
    let outcomes: Vec<Vec<Outcome>> = days
        .par_iter()
        .map(|(day, day_keys)| calibrate_day(*day, day_keys, bars, ticks))
        .collect();
    let mut built = Built {
        keys: distinct.len(),
        ..Built::default()
    };
    let mut fresh: BTreeMap<Symbol, Vec<ClockRow>> = BTreeMap::new();
    for outcome in outcomes.into_iter().flatten() {
        let (row, said) = outcome?;
        built.count(&row, said);
        fresh.entry(row.key.stock).or_default().push(row);
    }
    let dir = out.join(TABLE_DIR);
    fs::create_dir_all(&dir).map_err(|why| BuildRefusal::Io {
        path: dir.clone(),
        why: why.to_string(),
    })?;
    let mut staged = Vec::with_capacity(fresh.len());
    for (stock, rows) in fresh {
        let path = dir.join(file_name(stock));
        let held = read_held(&path).map_err(|why| BuildRefusal::Io {
            path: path.clone(),
            why,
        })?;
        let mut merged: BTreeMap<Day, ClockRow> = BTreeMap::new();
        if let Some(bytes) = &held {
            let (carried, _) = decode(stock, bytes).map_err(|why| BuildRefusal::Corrupt {
                path: path.clone(),
                why,
            })?;
            for row in carried {
                merged.insert(row.key.day, row);
            }
        }
        for row in rows {
            merged.insert(row.key.day, row);
        }
        let merged: Vec<ClockRow> = merged.into_values().collect();
        let bytes = encode(stock, &merged);
        if held.as_ref() == Some(&bytes) {
            built.unchanged += 1;
        } else {
            staged.push((temporary(&dir, stock), path, bytes));
        }
    }
    for (staging, path, bytes) in staged {
        publish(&dir, &path, &staging, &bytes).map_err(|why| BuildRefusal::Io {
            path,
            why: why.to_string(),
        })?;
        built.written += 1;
    }
    let table = Table::open(out).map_err(BuildRefusal::Table)?;
    built.leftovers = table.leftovers();
    built.rows = table.len();
    built.digest = table.digest();
    Ok(built)
}

/// The outcomes of `keys`, all of `day`: the day's tick listing is read once.
fn calibrate_day<B: BarSource, S: CmSource>(
    day: Day,
    keys: &[Key],
    bars: &B,
    ticks: &S,
) -> Vec<Outcome> {
    let listing = ticks.day(CmKind::Stocks, day);
    keys.iter()
        .map(|&key| {
            let held = bars
                .bars(key)
                .map_err(|why| BuildRefusal::Bars { key, why })?;
            let (read, said) = match &listing {
                Ok(Some(listing)) => match read_back_step(ticks, listing, key.stock) {
                    Ok(Some(step)) => (Ticks::Measured(step), None),
                    Ok(None) => (Ticks::Absent, None),
                    Err(why) => (Ticks::Refused, Some(why.to_string())),
                },
                Ok(None) => (Ticks::Absent, None),
                Err(why) => (Ticks::Refused, Some(why.to_string())),
            };
            Ok((calibrate(key, &held, read), said))
        })
        .collect()
}

/// Writes `bytes` to `temporary` made by `store::staging::create` (gate 28's
/// one door), flushes it, renames it over `path` and flushes `dir`. A failure
/// after the temporary was made removes it, and the old file, if any, stays
/// whole; a temporary the door refused is left as it found it.
fn publish(dir: &Path, path: &Path, temporary: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let staged = store::staging::create(temporary).map_err(std::io::Error::other)?;
    let mut file = staged.file;
    let done = file
        .write_all(bytes)
        .and_then(|()| file.sync_all())
        .and_then(|()| fs::rename(temporary, path))
        .and_then(|()| fs::File::open(dir))
        .and_then(|handle| handle.sync_all());
    if done.is_err() {
        let _gone = fs::remove_file(temporary);
    }
    done
}

// ------------------------------------------------------------------ bench --

/// The p50, the p99 and the largest of a run's samples, by nearest rank, and
/// how many there were.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Spread {
    /// The median.
    pub p50: u128,
    /// The 99th percentile.
    pub p99: u128,
    /// The largest.
    pub max: u128,
    /// How many samples.
    pub n: usize,
}

impl Spread {
    /// The spread of `samples`; all zero for none.
    #[must_use]
    pub fn of(samples: &[u128]) -> Self {
        let mut counts: BTreeMap<u128, usize> = BTreeMap::new();
        for &sample in samples {
            *counts.entry(sample).or_insert(0) += 1;
        }
        let n = samples.len();
        let rank = |percent: usize| n.saturating_mul(percent).div_ceil(100).max(1);
        let at = |wanted: usize| {
            let mut seen = 0;
            for (&value, &count) in &counts {
                seen += count;
                if seen >= wanted {
                    return value;
                }
            }
            0
        };
        Self {
            p50: at(rank(50)),
            p99: at(rank(99)),
            max: counts.last_key_value().map_or(0, |(&value, _)| value),
            n,
        }
    }
}

/// One run of C-FNO-CLOCK-01 (D-8225): `samples` timings, in picoseconds per
/// step, of `steps` minute-boundary steps each, on a grid of `seconds`
/// invented seconds, one second in seven untraded. A step is the calibration's
/// own: one minute's shifted window read and its four prices tallied
/// ([`tally_minute`]). The windows open at invented seconds drawn before each
/// timing, some before the grid and some across its end.
#[must_use]
pub fn measure_minute_boundary(seconds: usize, samples: usize, steps: usize) -> Vec<u128> {
    let mut state = 0x9E37_79B9_7F4A_7C15_u64;
    let cells: Vec<Option<Prices>> = (0..seconds)
        .map(|at| {
            let draw = i64::try_from(xorshift(&mut state) % 64).unwrap_or(0);
            (at % 7 != 3).then_some(Prices {
                open: 100_000 + draw,
                high: 100_064 + draw,
                low: 99_936 + draw,
                close: 100_032 - draw,
            })
        })
        .collect();
    let grid = Grid::build(0, &cells);
    let prices = Prices {
        open: 100_000,
        high: 100_100,
        low: 99_900,
        close: 100_032,
    };
    let span = u64::try_from(seconds)
        .unwrap_or(u64::MAX)
        .saturating_add(SPAN_PAD);
    let per = u128::try_from(steps.max(1)).unwrap_or(1);
    let mut tally: Tally = HashMap::with_capacity(steps.saturating_mul(4).max(4));
    let mut out = Vec::with_capacity(samples);
    for _ in 0..samples {
        let starts: Vec<Minute> = (0..steps)
            .map(|_| Minute {
                start: i64::try_from(xorshift(&mut state) % span).unwrap_or(0) - SECS_PER_MINUTE,
                prices,
            })
            .collect();
        tally.clear();
        let clock = Instant::now();
        for (index, minute) in starts.iter().enumerate() {
            black_box(tally_minute(&grid, &mut tally, index, minute, 0));
        }
        out.push(clock.elapsed().as_nanos().saturating_mul(1_000) / per);
    }
    out
}

/// Seconds past the grid's two ends the bench's windows may open in: one
/// minute before it and one across its end.
const SPAN_PAD: u64 = 120;

/// The next value of a xorshift64 sequence.
fn xorshift(state: &mut u64) -> u64 {
    let mut x = *state;
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    *state = x;
    x
}
