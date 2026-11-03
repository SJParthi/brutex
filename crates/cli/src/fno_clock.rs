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

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::too_many_lines,
    reason = "the fixtures index invented feeds directly and narrow small invented counts, \
              and an assertion that cannot panic cannot fail"
)]
mod tests {
    use super::*;
    use pull::gdfl_cm::{HEADER_OPEN_INTEREST, ListedFile, day_folder_name};
    use pull::session::IST_OFFSET_SECS;
    use std::os::unix::ffi::OsStrExt as _;
    use std::os::unix::fs::MetadataExt as _;

    // ── invented feeds ───────────────────────────────────────────────────────

    fn day(year: u16, month: u8, date: u8) -> Day {
        Day::new(year, month, date).unwrap()
    }

    /// The UTC second of `hh:mm:ss` IST on `at`.
    fn utc(at: Day, hh: i64, mm: i64, ss: i64) -> i64 {
        i64::from(at.days_from_epoch()) * 86_400 - IST_OFFSET_SECS + hh * 3_600 + mm * 60 + ss
    }

    fn stock(name: &str) -> Symbol {
        Symbol::new(name).unwrap()
    }

    fn key(name: &str, at: Day) -> Key {
        Key {
            stock: stock(name),
            day: at,
        }
    }

    fn bar(second: i64, prices: Prices, volume: i64) -> Bar {
        Bar {
            ts_micros: second * MICROS_PER_SECOND,
            open: prices.open,
            high: prices.high,
            low: prices.low,
            close: prices.close,
            volume,
            open_interest: store::format::OI_NULL,
        }
    }

    const fn flat(price: i64) -> Prices {
        Prices {
            open: price,
            high: price,
            low: price,
            close: price,
        }
    }

    /// Every real second in `from..to` that traded, with its prices: an
    /// invented random walk of `tick` paisa a step, up, down or still; one
    /// second in `quiet` trades nothing and one in nine has a one-tick range.
    fn walk(from: i64, to: i64, seed: u64, tick: i64, quiet: u64) -> Vec<(i64, Prices)> {
        let mut state = seed.max(1);
        let mut price = 20_000 * tick;
        let mut out = Vec::new();
        for second in from..to {
            let draw = xorshift(&mut state);
            if draw.is_multiple_of(quiet) {
                continue;
            }
            let step = match (draw >> 8) % 3 {
                0 => -tick,
                1 => 0,
                _ => tick,
            };
            price = (price + step).max(tick * 1_000);
            let wick = if (draw >> 16).is_multiple_of(9) {
                tick
            } else {
                0
            };
            out.push((
                second,
                Prices {
                    open: price,
                    high: price + wick,
                    low: price - wick,
                    close: price,
                },
            ));
        }
        out
    }

    /// One price every second of `from..to`, rising one paisa a second.
    fn rising(from: i64, to: i64) -> Vec<(i64, Prices)> {
        (from..to)
            .map(|second| (second, flat(100_000 + second - from)))
            .collect()
    }

    /// Zerodha's minute bars of `trades` for `count` minutes from the minute
    /// opening at UTC second `first`: each the fold of its traded seconds,
    /// back-adjusted by `raw_over_adjusted`; a minute with no trade has no bar.
    fn zerodha(
        trades: &[(i64, Prices)],
        first: i64,
        count: i64,
        raw_over_adjusted: (i64, i64),
    ) -> Vec<Bar> {
        let (num, den) = raw_over_adjusted;
        let mut folds: BTreeMap<i64, (Prices, i64)> = BTreeMap::new();
        for &(second, prices) in trades {
            let start = first + (second - first).div_euclid(60) * 60;
            if start < first || start >= first + count * 60 {
                continue;
            }
            folds
                .entry(start)
                .and_modify(|(fold, volume)| {
                    fold.high = fold.high.max(prices.high);
                    fold.low = fold.low.min(prices.low);
                    fold.close = prices.close;
                    *volume += 1;
                })
                .or_insert((prices, 1));
        }
        folds
            .into_iter()
            .map(|(start, (prices, volume))| {
                let adjust = |paisa: i64| {
                    assert_eq!(paisa * den % num, 0, "an invented adjustment must be exact");
                    paisa * den / num
                };
                let adjusted = Prices {
                    open: adjust(prices.open),
                    high: adjust(prices.high),
                    low: adjust(prices.low),
                    close: adjust(prices.close),
                };
                bar(start, adjusted, volume)
            })
            .collect()
    }

    /// GDFL's second bars of `trades`, each stamped `delta` seconds after its
    /// real second, kept only when stamped inside `keep` (the store's session
    /// gate).
    fn gdfl(trades: &[(i64, Prices)], delta: i64, keep: (i64, i64)) -> Vec<Bar> {
        trades
            .iter()
            .map(|&(second, prices)| (second + delta, prices))
            .filter(|&(stamp, _)| stamp >= keep.0 && stamp < keep.1)
            .map(|(stamp, prices)| bar(stamp, prices, 3))
            .collect()
    }

    /// The continuous session of a regular day, in UTC seconds.
    fn session(at: Day) -> (i64, i64) {
        (utc(at, 9, 15, 0), utc(at, 15, 30, 0))
    }

    const MEASURED: Ticks = Ticks::Measured(BackStep { step_s: 0, rows: 0 });

    /// A random walk over `minutes` minutes from 09:15 on `at`, two minutes of
    /// trades either side, GDFL stamped `delta` late.
    fn walked_day(at: Day, minutes: i64, seed: u64, delta: i64) -> DayBars {
        let open = utc(at, 9, 15, 0);
        let trades = walk(open - 120, open + minutes * 60 + 120, seed, 5, 4);
        DayBars {
            seconds: gdfl(&trades, delta, session(at)),
            minutes: zerodha(&trades, open, minutes, (1, 1)),
        }
    }

    // ── the offset ───────────────────────────────────────────────────────────

    #[test]
    fn shifts_of_one_to_thirty_seconds_either_way_are_recovered_exactly() {
        let at = day(2024, 4, 1);
        for delta in -30_i64..=30 {
            let bars = walked_day(at, 120, 0xC10C + delta.unsigned_abs(), delta);
            let row = calibrate(key("RELIANCE", at), &bars, MEASURED);
            assert_eq!(
                row.offset_ms,
                Some(i32::try_from(delta * 1_000).unwrap()),
                "{delta}"
            );
            assert_eq!(row.flags, Flags::default(), "{delta}");
            assert!(row.margin > 0, "{delta}: the true shift beats every other");
            assert_eq!(row.minutes, 120);
            assert_eq!(row.window_s, FIRST_WINDOW_S);
            assert!(row.score <= 4 * row.minutes);
        }
    }

    #[test]
    fn random_walks_recover_their_shift_under_many_seeds() {
        let mut state = 0x5EED_u64;
        for _ in 0..12 {
            let at = day(2024, 4, 1);
            let delta = i64::try_from(xorshift(&mut state) % 61).unwrap() - 30;
            let tick = [1, 5, 10][usize::try_from(xorshift(&mut state) % 3).unwrap()];
            let quiet = 2 + xorshift(&mut state) % 6;
            let open = utc(at, 9, 15, 0);
            let trades = walk(
                open - 120,
                open + 90 * 60 + 120,
                xorshift(&mut state),
                tick,
                quiet,
            );
            let bars = DayBars {
                seconds: gdfl(&trades, delta, session(at)),
                minutes: zerodha(&trades, open, 90, (1, 1)),
            };
            let row = calibrate(key("TCS", at), &bars, MEASURED);
            assert_eq!(row.offset_ms, Some(i32::try_from(delta * 1_000).unwrap()));
            assert!(!row.flags.has(Flags::TIED));
        }
    }

    #[test]
    fn a_full_session_is_recovered_and_its_edges_are_unmatched_only_where_gdfl_is_cut() {
        let at = day(2024, 4, 1);
        let bars = walked_day(at, 375, 41, -17);
        let row = calibrate(key("RELIANCE", at), &bars, MEASURED);
        assert_eq!(row.offset_ms, Some(-17_000));
        assert_eq!(row.minutes, 375);
        // GDFL stamps 17 s early, and the gate cuts the first 17 s of the day:
        // the 09:15 minute still holds the rest of its seconds.
        assert_eq!(row.unmatched, 0);
        assert!(row.seconds > 0);
        assert_eq!(row.floor(), Floor::Calibrated(0));
    }

    #[test]
    fn a_split_day_is_compared_under_its_session_ratio() {
        let at = day(2024, 4, 2);
        let open = utc(at, 9, 15, 0);
        for (raw_over_adjusted, tick) in [((2, 1), 10), ((3, 2), 3), ((5, 1), 5)] {
            let trades = walk(open - 120, open + 60 * 60 + 120, 77, tick, 3);
            let bars = DayBars {
                seconds: gdfl(&trades, 11, session(at)),
                minutes: zerodha(&trades, open, 60, raw_over_adjusted),
            };
            let row = calibrate(key("RELIANCE", at), &bars, MEASURED);
            assert_eq!(row.offset_ms, Some(11_000), "{raw_over_adjusted:?}");
        }
    }

    /// The primes from `from` up, by trial division: invented prices no two of
    /// which share a ratio with one other price.
    fn primes(from: i64, count: usize) -> Vec<i64> {
        (from..)
            .filter(|&n| (2..=n.isqrt()).all(|d| n % d != 0))
            .take(count)
            .collect()
    }

    #[test]
    fn a_day_with_no_agreement_is_flagged_uncalibrated() {
        let at = day(2024, 4, 1);
        let open = utc(at, 9, 15, 0);
        let constant = 100_003;
        let seconds = (open..open + 20 * 60)
            .map(|s| bar(s, flat(constant), 1))
            .collect();
        let distinct = primes(120_000, 80);
        let minutes = (0..20)
            .map(|m| {
                let p = &distinct[m * 4..m * 4 + 4];
                let prices = Prices {
                    low: p[0],
                    open: p[1],
                    close: p[2],
                    high: p[3],
                };
                bar(open + i64::try_from(m).unwrap() * 60, prices, 9)
            })
            .collect();
        let row = calibrate(key("RELIANCE", at), &DayBars { seconds, minutes }, MEASURED);
        assert_eq!(row.offset_ms, None);
        assert!(row.flags.has(Flags::NO_AGREEMENT));
        assert_eq!(row.score, 0);
        assert_eq!(row.floor(), Floor::Uncalibrated);
        assert!(row.window_s >= row.reach_s, "the whole reach was searched");
    }

    #[test]
    fn ties_take_the_larger_shift_and_the_later_one_on_an_equal_shift() {
        let at = day(2024, 4, 1);
        let open = utc(at, 9, 15, 0);
        let minutes: Vec<Bar> = (0..30)
            .map(|m| bar(open + m * 60, flat(50_000), 4))
            .collect();
        // Every second of 09:15:00..09:45:00 trades at the one price, so every
        // shift in -59..=59 matches every minute: the larger |delta| and then
        // the later one is +59.
        let seconds = (open..open + 30 * 60)
            .map(|s| bar(s, flat(50_000), 1))
            .collect();
        let bars = DayBars {
            seconds,
            minutes: minutes.clone(),
        };
        let row = calibrate(key("RELIANCE", at), &bars, MEASURED);
        assert_eq!(row.offset_ms, Some(59_000));
        assert!(row.flags.has(Flags::TIED));
        assert_eq!(row.margin, 0);
        assert_eq!(row.floor(), Floor::Calibrated(59));
        // Cut at 09:44:30: shifts past +29 leave the last minute unmatched, so
        // the tie is -59..=29 and the larger |delta| is -59.
        let seconds = (open..open + 30 * 60 - 30)
            .map(|s| bar(s, flat(50_000), 1))
            .collect();
        let row = calibrate(key("RELIANCE", at), &DayBars { seconds, minutes }, MEASURED);
        assert_eq!(row.offset_ms, Some(-59_000));
        assert!(row.flags.has(Flags::TIED));
        assert_eq!(row.floor(), Floor::Calibrated(0));
    }

    #[test]
    fn the_window_doubles_until_the_best_shift_is_inside_it() {
        let at = day(2024, 4, 3);
        let open = utc(at, 9, 15, 0);
        // A price that rises every second agrees with no other minute's ratio
        // at any wrong shift, so a window that misses the truth scores nothing
        // and its best shift is its edge.
        let trades = rising(open - 600, open + 70 * 60);
        for (delta, window) in [(59, 60), (60, 120), (90, 120), (-200, 240), (480, 960)] {
            let bars = DayBars {
                seconds: gdfl(&trades, delta, session(at)),
                minutes: zerodha(&trades, open, 60, (1, 1)),
            };
            let row = calibrate(key("RELIANCE", at), &bars, MEASURED);
            assert_eq!(
                row.offset_ms,
                Some(i32::try_from(delta * 1_000).unwrap()),
                "{delta}"
            );
            assert_eq!(row.window_s, window, "{delta}");
            assert!(!row.flags.has(Flags::TIED), "{delta}");
            assert_eq!(
                row.margin, row.score,
                "{delta}: no other shift agrees at all"
            );
        }
    }

    #[test]
    fn a_best_shift_on_the_edge_of_the_whole_reach_stops_the_search() {
        let at = day(2024, 4, 1);
        let open = utc(at, 9, 15, 0);
        // One minute and one traded second a minute later: the reach is 60,
        // one minute makes no ratio two minutes give, every shift ties at 0
        // and the tie's larger |delta| is the window's edge.
        let bars = DayBars {
            seconds: vec![bar(open + 60, flat(40_000), 2)],
            minutes: vec![bar(open, flat(40_000), 2)],
        };
        let row = calibrate(key("RELIANCE", at), &bars, MEASURED);
        assert_eq!((row.reach_s, row.window_s), (60, 60));
        assert!(row.flags.has(Flags::NO_AGREEMENT));
        // A reach under the first window's half-width ends the search at once.
        let bars = DayBars {
            seconds: vec![bar(open + 30, flat(40_000), 2)],
            minutes: vec![bar(open, flat(40_000), 2)],
        };
        let row = calibrate(key("RELIANCE", at), &bars, MEASURED);
        assert_eq!((row.reach_s, row.window_s), (30, 60));
    }

    #[test]
    fn kite_hole_minutes_and_gdfl_gaps_leave_the_shift_exact() {
        let at = day(2024, 4, 1);
        let open = utc(at, 9, 15, 0);
        let trades = walk(open - 120, open + 120 * 60 + 120, 9, 5, 4);
        let delta = 7;
        let mut minutes = zerodha(&trades, open, 120, (1, 1));
        let held = minutes.len();
        // Fifteen Kite-hole minutes: missing from Zerodha, not untraded.
        minutes.retain(|m| (m.ts_micros / MICROS_PER_SECOND - open) / 60 % 8 != 5);
        let holes = held - minutes.len();
        assert_eq!(holes, 15);
        // Ten minutes in which GDFL holds no traded second: as absent bars and
        // as bars with no volume, which are the same thing.
        let gap = (utc(at, 10, 0, 0), utc(at, 10, 10, 0));
        for zero_volume in [false, true] {
            let seconds: Vec<Bar> = gdfl(&trades, delta, session(at))
                .into_iter()
                .filter_map(|b| {
                    let s = b.ts_micros / MICROS_PER_SECOND;
                    if s < gap.0 || s >= gap.1 {
                        Some(b)
                    } else if zero_volume {
                        Some(Bar { volume: 0, ..b })
                    } else {
                        None
                    }
                })
                .collect();
            let bars = DayBars {
                seconds,
                minutes: minutes.clone(),
            };
            let row = calibrate(key("RELIANCE", at), &bars, MEASURED);
            assert_eq!(row.offset_ms, Some(7_000));
            assert_eq!(row.minutes, u32::try_from(minutes.len()).unwrap());
            let empty = minutes
                .iter()
                .filter(|m| {
                    let start = m.ts_micros / MICROS_PER_SECOND + delta;
                    start >= gap.0 && start + 60 <= gap.1
                })
                .count();
            // 10:00..10:08 are fully inside the gap at +7 s; two of them are
            // also Kite holes, so seven are compared and unmatched.
            assert_eq!(row.unmatched, u32::try_from(empty).unwrap());
            assert_eq!(row.unmatched, 7);
        }
    }

    #[test]
    fn the_auction_era_compares_only_the_continuous_minutes() {
        for (at, compared) in [(day(2026, 9, 15), 360), (day(2024, 4, 2), 375)] {
            let open = utc(at, 9, 15, 0);
            let mut trades = walk(open - 120, open + 375 * 60 + 120, 5, 5, 4);
            // From 15:15 the band holds prints of another kind in both feeds:
            // an invented auction price, far from the walk.
            let band = utc(at, 15, 15, 0);
            for (second, prices) in &mut trades {
                if *second >= band {
                    *prices = flat(prices.close * 3 + (*second % 7));
                }
            }
            let bars = DayBars {
                seconds: gdfl(&trades, -4, session(at)),
                minutes: zerodha(&trades, open, 375, (1, 1)),
            };
            let row = calibrate(key("TCS", at), &bars, MEASURED);
            assert_eq!(row.offset_ms, Some(-4_000), "{at}");
            assert_eq!(row.minutes, compared, "{at}");
        }
    }

    #[test]
    fn the_partial_gdfl_day_of_2019_05_30_is_calibrated_from_its_afternoon() {
        let at = day(2019, 5, 30);
        let open = utc(at, 9, 15, 0);
        let trades = walk(open - 120, open + 375 * 60 + 120, 3, 5, 3);
        let gdfl_from = utc(at, 11, 57, 47);
        let delta = 9;
        let seconds = gdfl(&trades, delta, (gdfl_from, session(at).1));
        let minutes = zerodha(&trades, open, 375, (1, 1));
        let bars = DayBars {
            seconds,
            minutes: minutes.clone(),
        };
        let row = calibrate(key("RELIANCE", at), &bars, MEASURED);
        assert_eq!(row.offset_ms, Some(9_000));
        let before = minutes
            .iter()
            .filter(|m| m.ts_micros / MICROS_PER_SECOND + delta + 60 <= gdfl_from)
            .count();
        assert_eq!(row.unmatched, u32::try_from(before).unwrap());
        assert!(row.unmatched > 150, "the morning is unmatched, never moved");
    }

    #[test]
    fn a_recorded_halt_is_never_compared() {
        let at = day(2020, 3, 13);
        let open = utc(at, 9, 15, 0);
        let trades = walk(open - 120, open + 375 * 60 + 120, 13, 5, 3);
        let bars = DayBars {
            seconds: gdfl(&trades, 2, session(at)),
            minutes: zerodha(&trades, open, 375, (1, 1)),
        };
        let row = calibrate(key("RELIANCE", at), &bars, MEASURED);
        assert_eq!(row.offset_ms, Some(2_000));
        // 09:22 to 10:20, the halt and its reopening auction, are 59 minutes.
        assert_eq!(row.minutes, 375 - 59);
    }

    // ── what a day lacked ────────────────────────────────────────────────────

    #[test]
    fn a_day_that_is_no_session_is_flagged_and_nothing_else_is_read() {
        let sunday = day(2024, 4, 14);
        let bars = walked_day(sunday, 10, 1, 0);
        let row = calibrate(key("RELIANCE", sunday), &bars, Ticks::Absent);
        assert_eq!(row.flags, Flags::NO_SESSION.with(Flags::TICKS_ABSENT));
        assert_eq!(row.offset_ms, None);
        assert_eq!(row.back_step_s, None);
        assert_eq!(row.floor(), Floor::Uncalibrated);
    }

    #[test]
    fn days_with_no_comparable_minute_or_no_traded_second_are_flagged() {
        let at = day(2024, 4, 1);
        let full = walked_day(at, 10, 2, 0);
        let pre_open = vec![bar(utc(at, 9, 10, 0), flat(10_000), 5)];
        let untraded: Vec<Bar> = full
            .minutes
            .iter()
            .map(|b| Bar { volume: 0, ..*b })
            .collect();
        for minutes in [Vec::new(), pre_open, untraded] {
            let bars = DayBars {
                seconds: full.seconds.clone(),
                minutes,
            };
            let row = calibrate(key("RELIANCE", at), &bars, MEASURED);
            assert_eq!(row.flags, Flags::NO_MINUTES);
        }
        let quiet: Vec<Bar> = full
            .seconds
            .iter()
            .map(|b| Bar { volume: 0, ..*b })
            .collect();
        for seconds in [Vec::new(), quiet] {
            let bars = DayBars {
                seconds,
                minutes: full.minutes.clone(),
            };
            let row = calibrate(key("RELIANCE", at), &bars, MEASURED);
            assert_eq!(row.flags, Flags::NO_SECONDS);
            assert_eq!(row.minutes, 0, "nothing was searched");
        }
        let row = calibrate(key("RELIANCE", at), &DayBars::default(), Ticks::Refused);
        assert_eq!(
            row.flags,
            Flags::NO_MINUTES
                .with(Flags::NO_SECONDS)
                .with(Flags::TICKS_REFUSED)
        );
        assert_eq!(row.floor(), Floor::Uncalibrated);
    }

    #[test]
    fn every_broken_input_rule_flags_its_feed_malformed() {
        let at = day(2024, 4, 1);
        let open = utc(at, 9, 15, 0);
        let good = walked_day(at, 10, 4, 0);
        let fine = flat(10_000);
        let broken = [
            // Not on a whole unit.
            Bar {
                ts_micros: open * MICROS_PER_SECOND + 500_000,
                ..bar(open, fine, 1)
            },
            // Another IST day.
            bar(open + 86_400, fine, 1),
            // An instant no calendar names.
            Bar {
                ts_micros: i64::MIN,
                ..bar(open, fine, 1)
            },
            // Under one paisa.
            bar(open, flat(0), 1),
            // A low above the open.
            bar(
                open,
                Prices {
                    low: 10_001,
                    ..fine
                },
                1,
            ),
            // A close above the high.
            bar(
                open,
                Prices {
                    close: 10_001,
                    ..fine
                },
                1,
            ),
            // A negative volume.
            bar(open, fine, -1),
        ];
        for wrong in broken {
            let seconds = vec![bar(open - 1, fine, 1), wrong];
            let bars = DayBars {
                seconds,
                minutes: good.minutes.clone(),
            };
            let row = calibrate(key("RELIANCE", at), &bars, MEASURED);
            assert_eq!(row.flags, Flags::SECONDS_MALFORMED, "{wrong:?}");
            let minute_wrong = Bar {
                ts_micros: if wrong.ts_micros == open * MICROS_PER_SECOND + 500_000 {
                    (open + 30) * MICROS_PER_SECOND
                } else {
                    wrong.ts_micros
                },
                ..wrong
            };
            let bars = DayBars {
                seconds: good.seconds.clone(),
                minutes: vec![bar(open - 60, fine, 1), minute_wrong],
            };
            let row = calibrate(key("RELIANCE", at), &bars, MEASURED);
            assert_eq!(row.flags, Flags::MINUTES_MALFORMED, "{minute_wrong:?}");
        }
        // Out of order, and twice the same stamp.
        for second in [open - 2, open - 1] {
            let bars = DayBars {
                seconds: vec![bar(open - 1, fine, 1), bar(second, fine, 1)],
                minutes: vec![bar(open - 60, fine, 1), bar(open - 60, fine, 1)],
            };
            let row = calibrate(key("RELIANCE", at), &bars, MEASURED);
            assert_eq!(
                row.flags,
                Flags::SECONDS_MALFORMED.with(Flags::MINUTES_MALFORMED)
            );
        }
    }

    // ── the back step ───────────────────────────────────────────────────────

    #[test]
    fn the_back_step_is_the_largest_fall_below_the_running_high() {
        assert_eq!(back_step([]), BackStep::default());
        assert_eq!(back_step([5, 5, 6, 9]), BackStep::default());
        assert_eq!(
            back_step([10, 12, 9, 13, 5, 13, 13, 11]),
            BackStep { step_s: 8, rows: 3 }
        );
    }

    /// One file an in-memory tick source holds.
    #[derive(Clone)]
    struct Held {
        entry: String,
        bytes: Vec<u8>,
        crc32: u32,
    }

    /// An in-memory tick source: each day it holds, its files or its refusal.
    struct MemTicks {
        days: Vec<(Day, Result<Vec<Held>, CmRefusal>)>,
    }

    impl MemTicks {
        fn files(&self, at: Day) -> Option<&Result<Vec<Held>, CmRefusal>> {
            self.days
                .iter()
                .find(|(held, _)| *held == at)
                .map(|(_, files)| files)
        }
    }

    impl CmSource for MemTicks {
        type Locator = usize;

        fn day(&self, kind: CmKind, at: Day) -> Result<Option<DayListing<usize>>, CmRefusal> {
            let Some(files) = self.files(at) else {
                return Ok(None);
            };
            let files = files.as_ref().map_err(Clone::clone)?;
            let mut listing = DayListing::new(kind, at);
            for (index, file) in files.iter().enumerate() {
                listing.push(&file.entry, file.bytes.len() as u64, file.crc32, index);
            }
            Ok(Some(listing))
        }

        fn fetch(
            &self,
            listing: &DayListing<usize>,
            file: &ListedFile<usize>,
        ) -> Result<Vec<u8>, CmRefusal> {
            let files = self.files(listing.day()).unwrap().as_ref().unwrap();
            Ok(files[file.locator].bytes.clone())
        }
    }

    /// A share's vendor file on `at`, invented: one row per `(second of the
    /// IST day, paisa)`, in the order given.
    fn tick_file(name: &str, at: Day, rows: &[(u32, i64)]) -> Held {
        let stem = format!("{name}.NSE");
        let date = format!("{:02}/{:02}/{:04}", at.day(), at.month(), at.year());
        let mut text = String::from(HEADER_OPEN_INTEREST);
        for &(sod, paisa) in rows {
            let time = format!("{:02}:{:02}:{:02}", sod / 3_600, sod / 60 % 60, sod % 60);
            let price = format!("{}.{:02}", paisa / 100, paisa % 100);
            let fields = [
                stem.as_str(),
                date.as_str(),
                time.as_str(),
                price.as_str(),
                price.as_str(),
                "10",
                price.as_str(),
                "10",
                "5",
                "0",
            ];
            text.push_str("\r\n");
            text.push_str(&fields.join(","));
        }
        text.push_str("\r\n");
        let bytes = text.into_bytes();
        Held {
            entry: format!("{}/{stem}.csv", day_folder_name(CmKind::Stocks, at)),
            crc32: pull::gdfl_archive::crc32(&bytes),
            bytes,
        }
    }

    /// Seconds of the IST day of `hh:mm:ss`.
    const fn sod(hh: u32, mm: u32, ss: u32) -> u32 {
        hh * 3_600 + mm * 60 + ss
    }

    #[test]
    fn the_raw_tick_order_gives_the_back_step_through_the_tick_reader() {
        let at = day(2024, 4, 1);
        let rows = [
            (sod(9, 15, 0), 73_140),
            (sod(9, 15, 4), 73_145),
            (sod(9, 15, 1), 73_150),
            (sod(9, 15, 6), 73_150),
            (sod(9, 14, 59), 73_155),
        ];
        let mut refused = tick_file("TCS", at, &rows);
        refused.crc32 ^= 1;
        let source = MemTicks {
            days: vec![(at, Ok(vec![tick_file("RELIANCE", at, &rows), refused]))],
        };
        let listing = source.day(CmKind::Stocks, at).unwrap().unwrap();
        assert_eq!(
            read_back_step(&source, &listing, stock("RELIANCE")),
            Ok(Some(BackStep { step_s: 7, rows: 2 }))
        );
        assert!(matches!(
            read_back_step(&source, &listing, stock("TCS")),
            Err(CmRefusal::SourceCrcMismatch { .. })
        ));
        assert_eq!(read_back_step(&source, &listing, stock("INFY")), Ok(None));
        assert_eq!(
            read_back_step(&source, &listing, stock("ZZQAB")),
            Err(CmRefusal::NotSwept),
            "a stock off the listing's scope is refused, not absent"
        );
    }

    // ── the floor and the row ───────────────────────────────────────────────

    fn row_of(at: Day, offset_ms: Option<i32>, back: Option<u32>, flags: Flags) -> ClockRow {
        ClockRow {
            offset_ms,
            back_step_s: back,
            flags,
            score: 400,
            margin: 37,
            back_rows: 3,
            minutes: 120,
            unmatched: 2,
            seconds: 5_400,
            window_s: 60,
            reach_s: 7_259,
            ..ClockRow::blank(key("RELIANCE", at))
        }
    }

    #[test]
    fn the_floor_is_the_positive_offset_plus_the_back_step() {
        let at = day(2024, 4, 1);
        let none = Flags::default();
        assert_eq!(
            row_of(at, Some(7_000), Some(3), none).floor(),
            Floor::Calibrated(10)
        );
        assert_eq!(
            row_of(at, Some(-5_000), Some(2), none).floor(),
            Floor::Calibrated(2)
        );
        assert_eq!(
            row_of(at, Some(4_000), None, Flags::TICKS_ABSENT).floor(),
            Floor::OffsetOnly(4)
        );
        assert_eq!(
            row_of(at, Some(-4_000), None, Flags::TICKS_REFUSED).floor(),
            Floor::OffsetOnly(0)
        );
        assert_eq!(
            row_of(at, None, Some(9), Flags::NO_AGREEMENT).floor(),
            Floor::Uncalibrated
        );
    }

    #[test]
    fn a_row_agrees_with_its_flags_only_as_a_build_writes_it() {
        let at = day(2024, 4, 1);
        let none = Flags::default();
        for (row, agrees) in [
            (row_of(at, Some(5_000), Some(1), none), true),
            (row_of(at, Some(5_000), Some(1), Flags::NO_AGREEMENT), false),
            (row_of(at, Some(5_500), Some(1), none), false),
            (row_of(at, None, Some(1), Flags::NO_SECONDS), true),
            (row_of(at, None, Some(1), none), false),
            (row_of(at, Some(5_000), Some(1), Flags::TICKS_ABSENT), false),
            (row_of(at, Some(5_000), None, Flags::TICKS_REFUSED), true),
            (row_of(at, Some(5_000), None, none), false),
            (row_of(at, Some(5_000), None, Flags::UNMEASURED), false),
        ] {
            assert_eq!(row.agrees(), agrees, "{row:?}");
        }
    }

    #[test]
    fn the_flags_name_nine_distinct_bits_and_refuse_any_other() {
        let mut seen = 0_u16;
        for (word, flag) in Flags::ALL {
            assert_eq!(flag.bits().count_ones(), 1, "{word}");
            assert_eq!(seen & flag.bits(), 0, "{word}");
            seen |= flag.bits();
        }
        assert_eq!(Flags::from_bits(seen), Some(Flags(seen)));
        assert_eq!(Flags::from_bits(1 << 9), None);
        let both = Flags::TIED.with(Flags::TICKS_ABSENT);
        assert!(both.has(Flags::TIED) && both.any(Flags::UNMEASURED));
        assert!(!both.has(Flags::UNMEASURED) && !both.any(Flags::UNCALIBRATED));
    }

    #[test]
    fn two_levels_of_the_sparse_table_cover_one_minute() {
        let widest = 1_i64 << (LEVELS - 1);
        assert!(widest <= SECS_PER_MINUTE && SECS_PER_MINUTE < 2 * widest);
    }

    // ── the grid and the search ─────────────────────────────────────────────

    /// The window `[start, start + 60)` folded by walking every second.
    fn scanned(cells: &[Option<Prices>], origin: i64, start: i64) -> Option<Prices> {
        let traded: Vec<Prices> = cells
            .iter()
            .enumerate()
            .filter(|(at, _)| {
                let second = origin + *at as i64;
                second >= start && second < start + 60
            })
            .filter_map(|(_, cell)| *cell)
            .collect();
        Some(Prices {
            open: traded.first()?.open,
            high: traded.iter().map(|p| p.high).max()?,
            low: traded.iter().map(|p| p.low).min()?,
            close: traded.last()?.close,
        })
    }

    #[test]
    fn the_sparse_table_folds_every_window_as_a_scan_does() {
        let mut state = 0xFEED_u64;
        for _ in 0..40 {
            let len = usize::try_from(xorshift(&mut state) % 300).unwrap();
            let cells: Vec<Option<Prices>> = (0..len)
                .map(|_| {
                    let draw = xorshift(&mut state);
                    (!draw.is_multiple_of(4)).then(|| {
                        let price = 1_000 + (draw % 50) as i64;
                        Prices {
                            open: price,
                            high: price + (draw % 3) as i64,
                            low: price - (draw % 2) as i64,
                            close: price + 1,
                        }
                    })
                })
                .collect();
            let grid = Grid::build(10_000, &cells);
            for start in 10_000 - 70..10_000 + len as i64 + 10 {
                assert_eq!(
                    grid.window(start),
                    scanned(&cells, 10_000, start),
                    "{len} {start}"
                );
            }
        }
    }

    #[test]
    fn a_ratio_is_reduced_and_a_shift_is_taken_by_score_then_size_then_sign() {
        assert_eq!(ratio(10, 5), (2, 1));
        assert_eq!(ratio(6, 4), (3, 2));
        assert_eq!(ratio(7, 7), (1, 1));
        assert_eq!(ratio(4, 6), (2, 3));
        let shift = |delta, score| Shift { delta, score };
        assert!(shift(0, 9).beats(shift(40, 8)));
        assert!(shift(-40, 8).beats(shift(39, 8)));
        assert!(shift(40, 8).beats(shift(-40, 8)));
        assert!(!shift(-40, 8).beats(shift(40, 8)));
        let mut board = Board::new();
        for (delta, score) in [(0, 3), (1, 5), (2, 5), (3, 4), (4, 1)] {
            board.record(shift(delta, score));
        }
        assert_eq!((board.best, board.top, board.ties), (shift(2, 5), 5, 2));
        assert_eq!(board.margin(), 0);
        board.record(shift(5, 6));
        assert_eq!((board.top, board.below, board.margin()), (6, 5, 1));
    }

    // ── the table ───────────────────────────────────────────────────────────

    /// A private directory, named for its test and this process.
    fn scratch(tag: &str) -> PathBuf {
        let root =
            std::env::temp_dir().join(format!("brutex-fno-clock-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        root
    }

    fn file_of(stock_name: &str, rows: &[ClockRow]) -> Vec<u8> {
        encode(stock(stock_name), rows)
    }

    /// Sets bytes `at..` of `bytes` and seals the digest again, so only the
    /// check under test can refuse.
    fn reseal(mut bytes: Vec<u8>, at: usize, with: &[u8]) -> Vec<u8> {
        bytes[at..at + with.len()].copy_from_slice(with);
        let digest = file_digest(&bytes[..HEAD_LEN], &bytes[HEADER_LEN..]);
        bytes[HEAD_LEN..HEADER_LEN].copy_from_slice(&digest);
        bytes
    }

    #[test]
    fn a_file_round_trips_and_every_broken_check_is_refused_by_name() {
        let at = day(2024, 4, 1);
        let rows = [
            row_of(at, Some(3_000), Some(2), Flags::default()),
            row_of(
                day(2024, 4, 2),
                None,
                None,
                Flags::NO_AGREEMENT.with(Flags::TICKS_ABSENT),
            ),
        ];
        let good = file_of("RELIANCE", &rows);
        assert_eq!(good.len(), HEADER_LEN + 2 * ROW_LEN);
        let (read, digest) = decode(stock("RELIANCE"), &good).unwrap();
        assert_eq!(read, rows);
        assert_eq!(digest[..], good[HEAD_LEN..HEADER_LEN]);
        let row = HEADER_LEN;
        let cases: [(Vec<u8>, &str); 16] = [
            (good[..50].to_vec(), SHORT),
            (reseal(good.clone(), 0, b"BRTXFCL2"), NOT_MAGIC),
            (reseal(good.clone(), 8, &2_u16.to_le_bytes()), NOT_VERSION),
            (reseal(good.clone(), 10, &44_u16.to_le_bytes()), NOT_STRIDE),
            (reseal(good.clone(), 12, &3_u32.to_le_bytes()), NOT_LENGTH),
            (
                reseal(good.clone(), 12, &u32::MAX.to_le_bytes()),
                NOT_LENGTH,
            ),
            (good[..good.len() - 1].to_vec(), NOT_LENGTH),
            (reseal(good.clone(), 16, b"TCS\0\0\0\0\0"), NOT_STOCK),
            (
                {
                    let mut bent = good.clone();
                    bent[row + 8] ^= 1;
                    bent
                },
                NOT_DIGEST,
            ),
            (reseal(good.clone(), row, &u32::MAX.to_le_bytes()), NOT_ROW),
            (
                reseal(good.clone(), row + 44, &(1_u16 << 12).to_le_bytes()),
                NOT_ROW,
            ),
            (
                reseal(good.clone(), row + 46, &1_u16.to_le_bytes()),
                NOT_ROW,
            ),
            (
                reseal(
                    good.clone(),
                    row + 44,
                    &Flags::NO_AGREEMENT.bits().to_le_bytes(),
                ),
                NOT_ROW,
            ),
            (
                reseal(good.clone(), row + 4, &3_500_i32.to_le_bytes()),
                NOT_ROW,
            ),
            (
                reseal(
                    good.clone(),
                    row + ROW_LEN,
                    &at.days_from_epoch().to_le_bytes(),
                ),
                NOT_ORDER,
            ),
            (
                reseal(
                    good.clone(),
                    row + ROW_LEN,
                    &day(2024, 3, 28).days_from_epoch().to_le_bytes(),
                ),
                NOT_ORDER,
            ),
        ];
        for (bytes, why) in cases {
            assert_eq!(decode(stock("RELIANCE"), &bytes), Err(why));
        }
    }

    fn plant(dir: &Path, name: &str, bytes: &[u8]) {
        fs::create_dir_all(dir).unwrap();
        fs::write(dir.join(name), bytes).unwrap();
    }

    #[test]
    fn the_table_reads_every_stock_file_and_refuses_what_is_not_one() {
        let out = scratch("table");
        let dir = out.join(TABLE_DIR);
        assert!(matches!(
            Table::open(&out),
            Err(TableRefusal::Unreadable { .. })
        ));
        let at = day(2024, 4, 1);
        let reliance = [row_of(at, Some(1_000), Some(0), Flags::default())];
        let tcs = [ClockRow {
            key: key("TCS", at),
            ..row_of(at, None, None, Flags::NO_SECONDS.with(Flags::TICKS_REFUSED))
        }];
        plant(&dir, "RELIANCE.fcl", &file_of("RELIANCE", &reliance));
        plant(&dir, "TCS.fcl", &file_of("TCS", &tcs));
        plant(&dir, ".TCS.fcl.tmp.1.1", b"a killed writer's leftover");
        let table = Table::open(&out).unwrap();
        assert_eq!(
            (table.len(), table.is_empty(), table.leftovers()),
            (2, false, 1)
        );
        assert_eq!(table.get(key("RELIANCE", at)), Some(&reliance[0]));
        assert_eq!(table.get(key("TCS", at)), Some(&tcs[0]));
        assert_eq!(table.get(key("INFY", at)), None);
        let digest = table.digest();
        // The digest moves with any stock's bytes, and not with a leftover.
        plant(&dir, ".x", b"");
        assert_eq!(Table::open(&out).unwrap().digest(), digest);
        plant(&dir, "TCS.fcl", &file_of("TCS", &[]));
        assert_ne!(Table::open(&out).unwrap().digest(), digest);
        for stray in ["notes.txt", "RELIANCEfcl", "BAD NAME.fcl", "reliance.fcl"] {
            plant(&dir, stray, b"");
            assert_eq!(
                Table::open(&out).unwrap_err(),
                TableRefusal::Stray {
                    path: dir.join(stray)
                },
                "{stray}"
            );
            fs::remove_file(dir.join(stray)).unwrap();
        }
        let odd = std::ffi::OsStr::from_bytes(b"\xff.fcl");
        fs::write(dir.join(odd), b"").unwrap();
        assert!(matches!(Table::open(&out), Err(TableRefusal::Stray { .. })));
        fs::remove_file(dir.join(odd)).unwrap();
        std::os::unix::fs::symlink(dir.join("TCS.fcl"), dir.join("INFY.fcl")).unwrap();
        assert!(matches!(
            Table::open(&out),
            Err(TableRefusal::Unreadable { .. })
        ));
        fs::remove_file(dir.join("INFY.fcl")).unwrap();
        plant(&dir, "INFY.fcl", b"short");
        assert_eq!(
            Table::open(&out).unwrap_err(),
            TableRefusal::Corrupt {
                path: dir.join("INFY.fcl"),
                why: SHORT
            }
        );
        let _ = fs::remove_dir_all(&out);
    }

    #[test]
    fn an_empty_table_has_a_digest_of_its_own() {
        let out = scratch("empty");
        fs::create_dir_all(out.join(TABLE_DIR)).unwrap();
        let table = Table::open(&out).unwrap();
        assert!(table.is_empty());
        let mut hasher = Hasher::new();
        hasher.update(TABLE_DOMAIN);
        assert_eq!(table.digest(), hasher.finalize());
        let _ = fs::remove_dir_all(&out);
    }

    // ── the build ───────────────────────────────────────────────────────────

    /// An in-memory bar source: each key's bars, and one key it refuses.
    struct Feeds {
        days: HashMap<Key, DayBars>,
        refuse: Option<Key>,
    }

    impl BarSource for Feeds {
        fn bars(&self, key: Key) -> Result<DayBars, String> {
            if self.refuse == Some(key) {
                return Err("the month is locked".to_owned());
            }
            Ok(self.days.get(&key).cloned().unwrap_or_default())
        }
    }

    const DAYS: [(u16, u8, u8); 3] = [(2024, 4, 1), (2024, 4, 2), (2024, 4, 3)];

    /// Two stocks over three days, a short walk each, GDFL stamped `shift`
    /// seconds late for each key's own seed.
    fn feeds(shift: impl Fn(Key) -> i64) -> (Vec<Key>, Feeds) {
        let mut keys = Vec::new();
        let mut days = HashMap::new();
        for (index, name) in ["RELIANCE", "TCS"].into_iter().enumerate() {
            for (count, &(y, m, d)) in DAYS.iter().enumerate() {
                let k = key(name, day(y, m, d));
                let seed = 100 + (index * 10 + count) as u64;
                days.insert(k, walked_day(k.day, 15, seed, shift(k)));
                keys.push(k);
            }
        }
        (keys, Feeds { days, refuse: None })
    }

    /// The tick source of the build tests: RELIANCE's file steps back on the
    /// first day, TCS has none; on the second day RELIANCE's file is refused
    /// and TCS's is in order; the third day's listing is refused.
    fn ticks() -> MemTicks {
        let one = day(2024, 4, 1);
        let two = day(2024, 4, 2);
        let stepped = [(sod(9, 15, 3), 73_100), (sod(9, 15, 0), 73_105)];
        let ordered = [(sod(9, 15, 0), 73_100), (sod(9, 15, 3), 73_105)];
        let mut bent = tick_file("RELIANCE", two, &ordered);
        bent.crc32 ^= 1;
        MemTicks {
            days: vec![
                (one, Ok(vec![tick_file("RELIANCE", one, &stepped)])),
                (two, Ok(vec![bent, tick_file("TCS", two, &ordered)])),
                (
                    day(2024, 4, 3),
                    Err(CmRefusal::TickStoreMalformed {
                        what: "an invented refusal",
                    }),
                ),
            ],
        }
    }

    /// Every file of the table under `out`, by name.
    fn files(out: &Path) -> BTreeMap<String, (Vec<u8>, u64)> {
        fs::read_dir(out.join(TABLE_DIR))
            .unwrap()
            .map(|entry| {
                let entry = entry.unwrap();
                let name = entry.file_name().to_string_lossy().into_owned();
                let inode = entry.metadata().unwrap().ino();
                (name, (fs::read(entry.path()).unwrap(), inode))
            })
            .collect()
    }

    fn bytes_of(files: &BTreeMap<String, (Vec<u8>, u64)>) -> BTreeMap<String, Vec<u8>> {
        files
            .iter()
            .map(|(name, (bytes, _))| (name.clone(), bytes.clone()))
            .collect()
    }

    #[test]
    fn a_build_calibrates_each_key_and_reports_what_it_met() {
        let out = scratch("report");
        let (keys, bars) = feeds(|k| i64::from(k.day.day()) * 3);
        let built = build(&keys, &bars, &ticks(), &out).unwrap();
        assert_eq!(
            (built.keys, built.written, built.unchanged, built.rows),
            (6, 2, 0, 6)
        );
        // RELIANCE day 1 measured; TCS day 2 measured; the rest unmeasured.
        assert_eq!(
            (built.calibrated, built.offset_only, built.uncalibrated),
            (2, 4, 0)
        );
        let count =
            |flag: Flags| built.flagged[Flags::ALL.iter().position(|&(_, f)| f == flag).unwrap()];
        assert_eq!(count(Flags::TICKS_ABSENT), 1);
        assert_eq!(count(Flags::TICKS_REFUSED), 3);
        assert_eq!(count(Flags::NO_AGREEMENT), 0);
        assert_eq!(built.tick_refusals, 3);
        assert_eq!(built.named.len(), 3);
        assert_eq!(built.named[0].0, key("RELIANCE", day(2024, 4, 2)));
        assert!(built.named[0].1.contains("CRC"));
        assert!(built.named[1].1.contains("an invented refusal"));
        let table = Table::open(&out).unwrap();
        assert_eq!(built.digest, table.digest());
        for k in &keys {
            let row = table.get(*k).unwrap();
            assert_eq!(row.offset_ms, Some(i32::from(k.day.day()) * 3_000), "{k:?}");
        }
        let reliance = table.get(key("RELIANCE", day(2024, 4, 1))).unwrap();
        assert_eq!((reliance.back_step_s, reliance.back_rows), (Some(3), 1));
        assert_eq!(reliance.floor(), Floor::Calibrated(6));
        let _ = fs::remove_dir_all(&out);
    }

    #[test]
    fn a_build_names_only_the_first_refusals_and_counts_every_one() {
        let out = scratch("named");
        let at = day(2024, 4, 3);
        let names: Vec<String> = (0..20)
            .map(|i| format!("ZZ{}{}", char::from(b'A' + i / 5), char::from(b'A' + i % 5)))
            .collect();
        let keys: Vec<Key> = names.iter().map(|n| key(n, at)).collect();
        let bars = Feeds {
            days: HashMap::new(),
            refuse: None,
        };
        let built = build(&keys, &bars, &ticks(), &out).unwrap();
        assert_eq!(built.tick_refusals, 20);
        assert_eq!(built.named.len(), NAMED_REFUSALS);
        assert_eq!(built.uncalibrated, 20);
        let _ = fs::remove_dir_all(&out);
    }

    #[test]
    fn a_full_build_and_a_list_rebuild_over_the_same_keys_give_equal_bytes() {
        let full = scratch("full");
        let listed = scratch("listed");
        let (keys, bars) = feeds(|k| i64::from(k.day.day()) - 2);
        let whole = build(&keys, &bars, &ticks(), &full).unwrap();
        // The same keys in three lists, out of order and with a repeat.
        let (first, rest) = keys.split_at(2);
        build(rest, &bars, &ticks(), &listed).unwrap();
        build(&[first[1], first[0], first[1]], &bars, &ticks(), &listed).unwrap();
        let again = build(&keys[2..4], &bars, &ticks(), &listed).unwrap();
        assert_eq!(bytes_of(&files(&full)), bytes_of(&files(&listed)));
        assert_eq!(again.digest, whole.digest);
        assert_eq!((again.written, again.unchanged), (0, 2));
        let _ = fs::remove_dir_all(&full);
        let _ = fs::remove_dir_all(&listed);
    }

    #[test]
    fn keys_outside_the_list_are_untouched() {
        let out = scratch("untouched");
        let fresh = scratch("fresh");
        let (keys, bars) = feeds(|_| 4);
        build(&keys, &bars, &ticks(), &out).unwrap();
        let before = files(&out);
        // The data of one stock-day changes (a repaired month), and only that
        // key is rebuilt.
        let changed = key("TCS", day(2024, 4, 2));
        let (_, mut repaired) = feeds(|k| if k == changed { -6 } else { 4 });
        repaired
            .days
            .insert(changed, walked_day(changed.day, 15, 999, -6));
        let built = build(&[changed], &repaired, &ticks(), &out).unwrap();
        assert_eq!((built.keys, built.written, built.unchanged), (1, 1, 0));
        let after = files(&out);
        assert_eq!(
            after["RELIANCE.fcl"], before["RELIANCE.fcl"],
            "bytes and inode"
        );
        assert_ne!(after["TCS.fcl"].0, before["TCS.fcl"].0);
        let (old, _) = decode(stock("TCS"), &before["TCS.fcl"].0).unwrap();
        let (new, _) = decode(stock("TCS"), &after["TCS.fcl"].0).unwrap();
        for (was, is) in old.iter().zip(&new) {
            if was.key == changed {
                assert_eq!(is.offset_ms, Some(-6_000));
            } else {
                assert_eq!(was, is);
            }
        }
        // And the table is the one a full build over the repaired data writes.
        build(&keys, &repaired, &ticks(), &fresh).unwrap();
        assert_eq!(bytes_of(&after), bytes_of(&files(&fresh)));
        let _ = fs::remove_dir_all(&out);
        let _ = fs::remove_dir_all(&fresh);
    }

    #[test]
    fn a_bar_source_refusal_refuses_the_build_and_writes_nothing() {
        let out = scratch("bars");
        let (keys, mut bars) = feeds(|_| 0);
        bars.refuse = Some(key("TCS", day(2024, 4, 1)));
        let refused = build(&keys, &bars, &ticks(), &out).unwrap_err();
        assert_eq!(
            refused,
            BuildRefusal::Bars {
                key: key("TCS", day(2024, 4, 1)),
                why: "the month is locked".to_owned()
            }
        );
        assert!(!out.join(TABLE_DIR).exists(), "nothing was written");
        let _ = fs::remove_dir_all(&out);
    }

    #[test]
    fn a_corrupt_or_unreadable_file_already_there_refuses_before_any_write() {
        let out = scratch("corrupt");
        let dir = out.join(TABLE_DIR);
        let (keys, bars) = feeds(|_| 0);
        plant(&dir, "TCS.fcl", b"not a table file");
        assert_eq!(
            build(&keys, &bars, &ticks(), &out).unwrap_err(),
            BuildRefusal::Corrupt {
                path: dir.join("TCS.fcl"),
                why: SHORT
            }
        );
        assert!(!dir.join("RELIANCE.fcl").exists(), "nothing was written");
        fs::remove_file(dir.join("TCS.fcl")).unwrap();
        fs::create_dir_all(dir.join("TCS.fcl")).unwrap();
        assert!(matches!(
            build(&keys, &bars, &ticks(), &out),
            Err(BuildRefusal::Io { .. })
        ));
        assert!(!dir.join("RELIANCE.fcl").exists(), "nothing was written");
        let _ = fs::remove_dir_all(&out);
    }

    #[test]
    fn a_folder_that_cannot_be_made_refuses_the_build() {
        let out = scratch("folder").join("a-file");
        fs::write(&out, b"").unwrap();
        let (keys, bars) = feeds(|_| 0);
        assert!(matches!(
            build(&keys, &bars, &ticks(), &out),
            Err(BuildRefusal::Io { path, .. }) if path == out.join(TABLE_DIR)
        ));
    }

    #[test]
    fn a_temporary_the_staging_door_refuses_stops_the_write_and_is_left_alone() {
        let out = scratch("staging");
        let dir = out.join(TABLE_DIR);
        let blocked = dir.join(".blocked");
        fs::create_dir_all(blocked.join("inside")).unwrap();
        let (keys, bars) = feeds(|_| 0);
        let refused = build_with(&keys, &bars, &ticks(), &out, &|_, _| blocked.clone());
        assert!(
            matches!(refused, Err(BuildRefusal::Io { .. })),
            "{refused:?}"
        );
        assert!(
            blocked.join("inside").exists(),
            "the door leaves what it found"
        );
        assert!(!dir.join("RELIANCE.fcl").exists());
        let _ = fs::remove_dir_all(&out);
    }

    #[test]
    fn a_failed_rename_removes_its_temporary_and_keeps_the_target() {
        let dir = scratch("rename");
        let target = dir.join("RELIANCE.fcl");
        fs::create_dir_all(target.join("inside")).unwrap();
        let temporary = dir.join(".RELIANCE.fcl.tmp.test");
        assert!(publish(&dir, &target, &temporary, b"bytes").is_err());
        assert!(!temporary.exists());
        assert!(target.join("inside").exists());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_stray_entry_refuses_the_table_read_after_the_writes() {
        let out = scratch("stray");
        plant(&out.join(TABLE_DIR), "notes.txt", b"");
        let (keys, bars) = feeds(|_| 0);
        assert!(matches!(
            build(&keys, &bars, &ticks(), &out),
            Err(BuildRefusal::Table(TableRefusal::Stray { .. }))
        ));
        let _ = fs::remove_dir_all(&out);
    }

    #[test]
    fn an_empty_key_list_writes_no_file_and_names_the_table() {
        let out = scratch("none");
        let bars = Feeds {
            days: HashMap::new(),
            refuse: None,
        };
        let built = build(&[], &bars, &ticks(), &out).unwrap();
        assert_eq!((built.keys, built.written, built.rows), (0, 0, 0));
        assert_eq!(built.digest, Table::open(&out).unwrap().digest());
        let _ = fs::remove_dir_all(&out);
    }

    #[test]
    fn every_refusal_says_what_it_refused() {
        let path = PathBuf::from("/x/fno-clock-v1/TCS.fcl");
        let k = key("TCS", day(2024, 4, 2));
        let texts = [
            TableRefusal::Unreadable {
                path: path.clone(),
                why: "denied".to_owned(),
            }
            .to_string(),
            TableRefusal::Stray { path: path.clone() }.to_string(),
            TableRefusal::Corrupt {
                path: path.clone(),
                why: NOT_DIGEST,
            }
            .to_string(),
            BuildRefusal::Bars {
                key: k,
                why: "locked".to_owned(),
            }
            .to_string(),
            BuildRefusal::Corrupt {
                path: path.clone(),
                why: SHORT,
            }
            .to_string(),
            BuildRefusal::Io {
                path: path.clone(),
                why: "full".to_owned(),
            }
            .to_string(),
            BuildRefusal::Table(TableRefusal::Stray { path }).to_string(),
        ];
        let words = [
            "denied",
            "neither a stock's file",
            NOT_DIGEST,
            "TCS on 2024-04-02",
            SHORT,
            "full",
            "wrote its files, then",
        ];
        for (text, word) in texts.iter().zip(words) {
            assert!(text.contains(word), "{text}");
            assert!(text.contains("clock"), "{text}");
        }
    }

    // ── the bench's measure ─────────────────────────────────────────────────

    #[test]
    fn the_minute_boundary_measure_runs_at_tiny_sizes() {
        for seconds in [0, 300] {
            let samples = measure_minute_boundary(seconds, 3, 5);
            assert_eq!(samples.len(), 3);
        }
        assert!(measure_minute_boundary(300, 0, 5).is_empty());
    }

    #[test]
    fn a_spread_takes_the_nearest_rank() {
        assert_eq!(Spread::of(&[]), Spread::default());
        assert_eq!(
            Spread::of(&[5]),
            Spread {
                p50: 5,
                p99: 5,
                max: 5,
                n: 1
            }
        );
        let hundred: Vec<u128> = (1..=100).rev().collect();
        assert_eq!(
            Spread::of(&hundred),
            Spread {
                p50: 50,
                p99: 99,
                max: 100,
                n: 100
            }
        );
        assert_eq!(Spread::of(&[7, 7, 7, 9]).p50, 7);
    }
}
