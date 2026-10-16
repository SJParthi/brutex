//! A month's summary: what a reader needs to know about a month WITHOUT reading
//! its bars — the minutes and seconds of the day it holds, its extremes, its
//! bars per day and the facts about its input a gap audit refuses on. R3-06,
//! D-4980.
//!
//! # Why it exists
//!
//! `/gaps.json` and the gap band on `/db` read every record of every month
//! they were asked about, on every request (DB-A30): 5,167,003 records for the
//! first twelve NIFTY one-second months. A gap is a fact about WHICH minutes
//! and seconds a month holds, never about its prices, so a month's minute and
//! second index answers it exactly, and that index is a few kilobytes against
//! the month's megabytes.
//!
//! # What it holds, and why it is not "the gap result" itself
//!
//! A gap result depends on the request: the calendar it is counted against
//! (`record`, `peers`, `table`), the run length a second gap must reach to be
//! listed (`min_run`), and the contract's expiry. Storing one result would
//! answer one of those and silently mis-answer the rest. The summary stores
//! the calendar-free facts every result is computed from, so the audit reads
//! one summary a month and no bar, and is the same computation whichever
//! calendar is asked:
//!
//! * the month's identity and the committed header it describes (`generation`,
//!   `n_valid`, first and last stamp), so a summary of other bars is refused
//!   by a header comparison and never read as these;
//! * counts of input a gap audit cannot certify: stamps off the month's own
//!   grid or not strictly increasing, stamps that go backwards, negative
//!   volume, rows stamped outside the month;
//! * extremes: the highest high and the lowest low with the stamp of the
//!   EARLIEST bar holding each, the first open, the last close, the volume
//!   summed (saturating) and the last open interest;
//! * per day of the month: bars, first and last second of the day held;
//! * per day with bars: a bitmap of the minutes of the day held, from its
//!   first to its last minute;
//! * for a month finer than a minute whose held seconds are at least half the
//!   seconds its days span (the density rule, below): a bitmap of the seconds
//!   held, from each day's first to its last.
//!
//! # The density rule for seconds
//!
//! Measured on the operator's store (13 Oct, read only): NIFTY one-second
//! 2019-06 holds 404,352 bars in 21,382 runs of held seconds, so a list of runs
//! would be 171 KB and a bitmap of its spanned seconds is 53 KB. A
//! trade-driven series is the opposite: the NIFTY 2019-06-27 10,000 CE holds
//! 7,328 bars in 6,139 runs across 18 days, where a bitmap of the spanned
//! seconds (about 50 KB) is larger than its compressed bars. Seconds are kept
//! when held seconds are at least half the spanned ones, which bounds the
//! section at two bits a held second (a quarter of a byte a bar, under a tenth
//! of a sealed bar's three bytes). The rule is judged at every write: a month
//! an append takes under half drops its seconds at that write, and a month
//! whose seconds were dropped or never kept is rebuilt whole the first time an
//! append makes it dense again, so the whole build runs once per crossing of
//! the half line, never once per commit. Only the index audit needs seconds
//! (a trade-driven series owes no count, D-3336 §2), and an index publishes
//! every second, so its months are dense; a month whose seconds are not kept
//! is NAMED by the reader that needed them, which then reads the bars.
//!
//! # Where it is kept
//!
//! A sealed month carries these bytes in its `.sbar` summary section (head
//! flag bit 2, `sum_off`, `sum_len`; amendment A4): the section is this
//! module's [`VERSION`], two zero bytes, the payload and a CRC-32C, and
//! [`crate::sealed::Builder`] carries it unchanged. A month still being
//! written keeps the SAME section behind an 8-byte magic in a `<yyyy-mm>.sum`
//! sibling ([`crate::path::FileKind::MonthSummary`]), written by the writer's
//! step after every commit ([`crate::file::BarFile::append`]) and by
//! [`crate::file::BarFile::month_summary_existing`], so the sealer copies the
//! sibling's bytes after the magic and changes none. A month under
//! [`crate::time_index::INDEX_MIN_ROWS`] bars has no `.sum`, as it has no `.tix`
//! and no `.srt` (D-4933): its summary is built from its at most 1,023 bars
//! by [`MonthSummary::build`] on demand, the bound the sort-on-demand rule
//! already has.
//!
//! # Cost
//!
//! Reading: one `stat`-free open and one read of the whole file (1.5 KB for a
//! minute month, about 55 KB for a one-second index month, at most
//! [`MAX_PAYLOAD_LEN`]), one CRC and a decode linear in those bytes; no bar
//! is read. Building: one pass over the bars folded, each O(1).

use std::fmt;
use std::io::{ErrorKind, Read as _};
use std::path::{Path, PathBuf};

use crate::crc::crc32c;
use crate::format::Bar;
use crate::header::Header;
use crate::path::YearMonth;

/// The summary's own format version: the number the `.sbar` section and the
/// `.sum` sibling carry before the payload.
pub const VERSION: u16 = 1;

/// The `.sum` sibling's magic. Not in use anywhere else in the store.
pub const MAGIC: [u8; 8] = *b"BRUTEXU1";

/// Bytes of the payload's fixed head.
pub const HEAD_LEN: usize = 160;

/// Bytes of one day's entry: bars, first second, last second.
pub const DAY_LEN: usize = 12;

/// Payload flag: the seconds bitmaps are present.
pub const FLAG_SECONDS: u32 = 0b1;

/// Every payload flag version 1 knows. Any other bit is refused.
pub const KNOWN_FLAGS: u32 = FLAG_SECONDS;

/// The longest payload: the head, 31 days, every minute and every second of
/// every day held. 31 × (180 + 10,800) + 160 + 372 = 340,912 bytes, under the
/// sealed section's [`crate::sealed::MAX_SUMMARY_LEN`].
pub const MAX_PAYLOAD_LEN: usize = HEAD_LEN + 31 * (DAY_LEN + 180 + 10_800);

/// Bytes the section wraps around a payload: version, two zero bytes, CRC.
pub const SECTION_OVERHEAD: usize = 8;

/// The IST offset from UTC in seconds. No daylight saving: a constant.
const IST_OFFSET_SECS: i64 = 19_800;

/// Seconds in a day.
const SECONDS_PER_DAY: i64 = 86_400;

/// Words of a day's minute bitmap: 1,440 bits.
const MINUTE_WORDS: usize = 23;

/// Words of a day's second bitmap: 86,400 bits.
const SECOND_WORDS: usize = 1_350;

/// A day entry's `first`/`last` when the day holds no bar.
const NO_SECOND: u32 = u32::MAX;

/// One day of a month, as a summary holds it.
#[derive(Debug, Clone, PartialEq, Eq)]
struct DayBits {
    /// Bars stamped on this IST day.
    bars: u32,
    /// The first and last second of the day held, or [`NO_SECOND`].
    first: u32,
    last: u32,
    /// Minutes of the day held.
    minutes: [u64; MINUTE_WORDS],
    /// Seconds of the day held, while they are tracked.
    seconds: Option<Box<[u64; SECOND_WORDS]>>,
}

impl DayBits {
    fn empty(track_seconds: bool) -> Self {
        Self {
            bars: 0,
            first: NO_SECOND,
            last: NO_SECOND,
            minutes: [0; MINUTE_WORDS],
            seconds: track_seconds.then(|| Box::new([0; SECOND_WORDS])),
        }
    }

    fn held(&self) -> bool {
        self.bars > 0 && self.first != NO_SECOND
    }

    /// Seconds from the first held to the last held, inclusive.
    fn span(&self) -> u32 {
        if self.held() {
            self.last.saturating_sub(self.first).saturating_add(1)
        } else {
            0
        }
    }
}

/// Sets bit `bit` of `words`; a bit past the end is ignored (cannot happen
/// for a second under 86,400 or a minute under 1,440).
fn set(words: &mut [u64], bit: u32) {
    let word = usize::try_from(bit / 64).unwrap_or(usize::MAX);
    if let Some(slot) = words.get_mut(word) {
        *slot |= 1_u64 << (bit % 64);
    }
}

/// The set bits of `words` in `first..=last`, ascending: one step a word and
/// one a set bit, never one a clear bit.
fn set_bits(words: &[u64], first: u32, last: u32) -> Vec<u32> {
    let mut out = Vec::new();
    let (from, to) = (first / 64, last / 64);
    for index in from..=to {
        let word = usize::try_from(index).unwrap_or(usize::MAX);
        let mut bits = words.get(word).copied().unwrap_or(0);
        if index == from {
            bits &= u64::MAX << (first % 64);
        }
        if index == to && last % 64 < 63 {
            bits &= (1_u64 << (last % 64 + 1)) - 1;
        }
        while bits != 0 {
            out.push(index * 64 + bits.trailing_zeros());
            bits &= bits - 1;
        }
    }
    out
}

/// Counts of input a gap audit cannot certify, and of what was folded.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Counts {
    /// Rows folded into this summary.
    pub folded: u64,
    /// Stamps off the month's own grid (`timeframe_secs`) or not strictly
    /// after the row before: what a minute audit refuses (`invalid_timestamps`).
    pub invalid: u32,
    /// Stamps before the row before: what the fold refuses (`OutOfOrder`).
    pub decreasing: u32,
    /// Rows with a negative volume: what the fold refuses.
    pub negative_volume: u32,
    /// Rows whose IST day is not a day of the month.
    pub outside_month: u32,
    /// Distinct seconds held (a stamp floored to its second, counted when it
    /// differs from the row before's; exact for a non-decreasing month).
    pub held_seconds: u32,
    /// Distinct minutes held, counted the same way, rows outside the month
    /// included: what `pull::fold` makes of a non-decreasing one-second month
    /// and what a minute month's rows number, so a gap audit's `held` is the
    /// one a full read states.
    pub held_minutes: u32,
}

/// The month's extremes. Zero when the month holds no row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Extremes {
    /// The highest high, and the stamp of the earliest bar holding it.
    pub high: i64,
    /// Stamp of the earliest bar holding [`Self::high`].
    pub high_ts: i64,
    /// The lowest low.
    pub low: i64,
    /// Stamp of the earliest bar holding [`Self::low`].
    pub low_ts: i64,
    /// The first row's open.
    pub open: i64,
    /// The last row's close.
    pub close: i64,
    /// Every row's volume summed, saturating.
    pub volume: i64,
    /// The last row's open interest.
    pub open_interest: i64,
}

/// One month's summary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MonthSummary {
    /// The month's symbol id, from its header.
    pub symbol_id: u32,
    /// The month's rung in seconds.
    pub timeframe_secs: u32,
    /// The month.
    pub month: YearMonth,
    /// The committed header's generation this summary describes.
    pub generation: u64,
    /// The committed rows it describes.
    pub n_valid: u64,
    /// The first row's stamp, or 0 for an empty month.
    pub first_ts_micros: i64,
    /// The last row's stamp, or 0 for an empty month.
    pub last_ts_micros: i64,
    /// What was folded and what a gap audit cannot certify.
    pub counts: Counts,
    /// The extremes.
    pub extremes: Extremes,
    /// Epoch day of the month's first day.
    first_day: i64,
    /// One entry per day of the month.
    days: Vec<DayBits>,
    /// Whether seconds are tracked (and, encoded, kept).
    seconds: bool,
}

/// Why a kept summary was not used, each with its own name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refused {
    /// No summary is kept beside the month.
    Absent,
    /// The month holds fewer than `INDEX_MIN_ROWS` bars and keeps none
    /// (D-4933): its summary is built from its bars.
    BelowThreshold,
    /// The host refused the read.
    Unreadable(String),
    /// The file is shorter than its magic and section, or its section's
    /// length is not the payload's geometry.
    Torn,
    /// Another magic.
    Magic,
    /// The section fails its CRC-32C.
    Checksum,
    /// A version this build does not read, or a nonzero reserved byte.
    Version(u16),
    /// The payload's own geometry is wrong, named.
    Shape(&'static str),
    /// A summary of another symbol, rung or month.
    NotThisMonth,
    /// A summary of other bars: another generation, row count or stamps.
    Stale {
        /// The summary's generation and rows.
        summary: (u64, u64),
        /// The header's.
        header: (u64, u64),
    },
}

impl Refused {
    /// A stable one-word name.
    #[must_use]
    pub const fn reason(&self) -> &'static str {
        match self {
            Self::Absent => "absent",
            Self::BelowThreshold => "below_threshold",
            Self::Unreadable(_) => "unreadable",
            Self::Torn => "torn",
            Self::Magic => "magic",
            Self::Checksum => "checksum",
            Self::Version(_) => "version",
            Self::Shape(_) => "shape",
            Self::NotThisMonth => "not_this_month",
            Self::Stale { .. } => "stale",
        }
    }
}

impl fmt::Display for Refused {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let reason = self.reason();
        match self {
            Self::Absent => write!(f, "{reason}: no month summary is kept beside the month"),
            Self::BelowThreshold => write!(
                f,
                "{reason}: a month under 1,024 bars keeps no summary (D-4933); it is built \
                 from its bars"
            ),
            Self::Unreadable(why) => write!(f, "{reason}: {why}"),
            Self::Torn => write!(f, "{reason}: the summary is shorter than its section says"),
            Self::Magic => write!(f, "{reason}: the summary file has another magic"),
            Self::Checksum => write!(f, "{reason}: the summary section fails its CRC-32C"),
            Self::Version(version) => {
                write!(
                    f,
                    "{reason}: summary version {version} is not one this build reads"
                )
            }
            Self::Shape(why) => write!(f, "{reason}: {why}"),
            Self::NotThisMonth => {
                write!(
                    f,
                    "{reason}: the summary names another symbol, rung or month"
                )
            }
            Self::Stale { summary, header } => write!(
                f,
                "{reason}: the summary describes generation {} with {} bars; the month \
                 commits generation {} with {}",
                summary.0, summary.1, header.0, header.1
            ),
        }
    }
}

impl std::error::Error for Refused {}

/// Days in `month`, Gregorian.
fn days_in(month: YearMonth) -> u8 {
    match month.month() {
        2 => {
            let year = month.year();
            if (year.is_multiple_of(4) && !year.is_multiple_of(100)) || year.is_multiple_of(400) {
                29
            } else {
                28
            }
        }
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

/// Days from 1970-01-01 to the first of `month` (Howard Hinnant's
/// `days_from_civil`).
fn first_day_of(month: YearMonth) -> i64 {
    let (m, y) = (i64::from(month.month()), i64::from(month.year()));
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// The IST epoch day and second of the day of a stamp.
fn ist(ts_micros: i64) -> (i64, u32) {
    let secs = ts_micros
        .div_euclid(1_000_000)
        .saturating_add(IST_OFFSET_SECS);
    (
        secs.div_euclid(SECONDS_PER_DAY),
        u32::try_from(secs.rem_euclid(SECONDS_PER_DAY)).unwrap_or(0),
    )
}

impl MonthSummary {
    /// An empty summary of `month`, ready to [`Self::fold`] its rows in order.
    #[must_use]
    pub fn new(symbol_id: u32, timeframe_secs: u32, month: YearMonth) -> Self {
        let seconds = timeframe_secs < 60;
        Self {
            symbol_id,
            timeframe_secs,
            month,
            generation: 0,
            n_valid: 0,
            first_ts_micros: 0,
            last_ts_micros: 0,
            counts: Counts::default(),
            extremes: Extremes::default(),
            first_day: first_day_of(month),
            days: (0..days_in(month))
                .map(|_| DayBits::empty(seconds))
                .collect(),
            seconds,
        }
    }

    /// The summary of `rows`, a month's committed rows in order, describing the
    /// header `header` — the ONE library call that builds a summary, used by
    /// the writer's step, by the rebuild door and by a reader of a month that
    /// keeps none.
    #[must_use]
    pub fn build(header: &Header, month: YearMonth, rows: &[Bar]) -> Self {
        let mut summary = Self::new(header.symbol_id, header.timeframe_secs, month);
        for bar in rows {
            summary.fold(bar);
        }
        summary.bind(header);
        summary
    }

    /// Binds the summary to the committed header it now describes.
    pub fn bind(&mut self, header: &Header) {
        self.generation = header.generation;
        self.n_valid = header.n_valid;
    }

    /// Folds the next row, in the month's order. O(1).
    pub fn fold(&mut self, bar: &Bar) {
        let ts = bar.ts_micros;
        let previous = (self.counts.folded > 0).then_some(self.last_ts_micros);
        let grid = i64::from(self.timeframe_secs.max(1)).saturating_mul(1_000_000);
        if previous.is_some_and(|before| ts < before) {
            self.counts.decreasing = self.counts.decreasing.saturating_add(1);
        }
        if ts.rem_euclid(grid) != 0 || previous.is_some_and(|before| ts <= before) {
            self.counts.invalid = self.counts.invalid.saturating_add(1);
        }
        if bar.volume < 0 {
            self.counts.negative_volume = self.counts.negative_volume.saturating_add(1);
        }
        if previous.is_none_or(|before| ts.div_euclid(1_000_000) != before.div_euclid(1_000_000)) {
            self.counts.held_seconds = self.counts.held_seconds.saturating_add(1);
        }
        if previous.is_none_or(|before| ts.div_euclid(60_000_000) != before.div_euclid(60_000_000))
        {
            self.counts.held_minutes = self.counts.held_minutes.saturating_add(1);
        }
        self.fold_extremes(bar, previous.is_none());
        if previous.is_none() {
            self.first_ts_micros = ts;
        }
        self.last_ts_micros = ts;
        self.counts.folded = self.counts.folded.saturating_add(1);
        let (day, second) = ist(ts);
        let Some(entry) = usize::try_from(day.saturating_sub(self.first_day))
            .ok()
            .and_then(|at| self.days.get_mut(at))
        else {
            self.counts.outside_month = self.counts.outside_month.saturating_add(1);
            return;
        };
        entry.bars = entry.bars.saturating_add(1);
        entry.first = if entry.first == NO_SECOND {
            second
        } else {
            entry.first.min(second)
        };
        entry.last = if entry.last == NO_SECOND {
            second
        } else {
            entry.last.max(second)
        };
        set(&mut entry.minutes, second / 60);
        if let Some(seconds) = entry.seconds.as_mut() {
            set(seconds.as_mut_slice(), second);
        }
    }

    fn fold_extremes(&mut self, bar: &Bar, first: bool) {
        let e = &mut self.extremes;
        if first {
            *e = Extremes {
                high: bar.high,
                high_ts: bar.ts_micros,
                low: bar.low,
                low_ts: bar.ts_micros,
                open: bar.open,
                close: bar.close,
                volume: bar.volume,
                open_interest: bar.open_interest,
            };
            return;
        }
        if bar.high > e.high {
            e.high = bar.high;
            e.high_ts = bar.ts_micros;
        }
        if bar.low < e.low {
            e.low = bar.low;
            e.low_ts = bar.ts_micros;
        }
        e.close = bar.close;
        e.volume = e.volume.saturating_add(bar.volume);
        e.open_interest = bar.open_interest;
    }

    /// Seconds the month's days span, first held to last held, summed.
    #[must_use]
    pub fn spanned_seconds(&self) -> u64 {
        self.days.iter().map(|day| u64::from(day.span())).sum()
    }

    /// Whether the encoded summary keeps the seconds bitmaps (the density
    /// rule in the module header).
    #[must_use]
    pub fn keeps_seconds(&self) -> bool {
        self.seconds
            && u64::from(self.counts.held_seconds).saturating_mul(2) >= self.spanned_seconds()
    }

    /// Whether this month's seconds are tracked. A summary decoded without
    /// them is not, and [`Self::wants_seconds`] says when that matters.
    #[must_use]
    pub const fn tracks_seconds(&self) -> bool {
        self.seconds
    }

    /// A finer-than-minute month that is dense now but whose seconds were not
    /// kept: its writer rebuilds it whole, once (the module header).
    #[must_use]
    pub fn wants_seconds(&self) -> bool {
        !self.seconds
            && self.timeframe_secs < 60
            && u64::from(self.counts.held_seconds).saturating_mul(2) >= self.spanned_seconds()
    }

    /// Whether the summary describes exactly the committed `header` of
    /// `month`: one comparison of six numbers, no bar read.
    ///
    /// # Errors
    ///
    /// [`Refused::NotThisMonth`] or [`Refused::Stale`].
    pub fn describes(&self, header: &Header, month: YearMonth) -> Result<(), Refused> {
        if self.symbol_id != header.symbol_id
            || self.timeframe_secs != header.timeframe_secs
            || self.month != month
        {
            return Err(Refused::NotThisMonth);
        }
        let stamps = header.n_valid == 0
            || (self.first_ts_micros == header.first_ts_micros
                && self.last_ts_micros == header.last_ts_micros);
        if self.generation != header.generation || self.n_valid != header.n_valid || !stamps {
            return Err(Refused::Stale {
                summary: (self.generation, self.n_valid),
                header: (header.generation, header.n_valid),
            });
        }
        Ok(())
    }

    /// The epoch day of the month's first day.
    #[must_use]
    pub const fn first_day(&self) -> i64 {
        self.first_day
    }

    /// The epoch day of the month's last day.
    #[must_use]
    pub fn last_day(&self) -> i64 {
        self.first_day + i64::try_from(self.days.len()).unwrap_or(0) - 1
    }

    /// Bars stamped on epoch day `day`; zero outside the month.
    #[must_use]
    pub fn bars_on(&self, day: i64) -> u32 {
        self.day(day).map_or(0, |entry| entry.bars)
    }

    fn day(&self, day: i64) -> Option<&DayBits> {
        usize::try_from(day.saturating_sub(self.first_day))
            .ok()
            .and_then(|at| self.days.get(at))
    }

    /// Every minute held, as the stamp of its open, ascending: the stamps a
    /// minute month holds, or the minutes a finer month folds to. Linear in
    /// the minutes of the month (at most 44,640), never in its bars.
    #[must_use]
    pub fn minute_stamps(&self) -> Vec<i64> {
        let mut out = Vec::new();
        for (offset, entry) in (0_i64..).zip(&self.days) {
            if !entry.held() {
                continue;
            }
            let base = (self.first_day + offset) * SECONDS_PER_DAY - IST_OFFSET_SECS;
            for minute in set_bits(&entry.minutes, entry.first / 60, entry.last / 60) {
                out.push((base + i64::from(minute) * 60) * 1_000_000);
            }
        }
        out
    }

    /// The seconds of epoch day `day` held, ascending and distinct, or `None`
    /// when this summary does not keep seconds. Empty for a day without bars.
    #[must_use]
    pub fn seconds_on(&self, day: i64) -> Option<Vec<u32>> {
        if !self.seconds {
            return None;
        }
        let Some(entry) = self.day(day) else {
            return Some(Vec::new());
        };
        let words = entry.seconds.as_deref()?;
        if !entry.held() {
            return Some(Vec::new());
        }
        Some(set_bits(words, entry.first, entry.last))
    }

    /// The payload a `.sbar` summary section and a `.sum` sibling carry with
    /// [`VERSION`]. The density rule decides, at this write, whether the
    /// seconds go in; a summary decoded with them tracks them
    /// ([`Self::decode`]), and one decoded without them is rebuilt whole by
    /// the writer when [`Self::wants_seconds`] says so.
    #[must_use]
    pub fn payload(&self) -> Vec<u8> {
        let seconds = self.keeps_seconds();
        let mut out = Vec::with_capacity(HEAD_LEN + self.days.len() * DAY_LEN + 1_024);
        out.extend_from_slice(&self.symbol_id.to_le_bytes());
        out.extend_from_slice(&self.timeframe_secs.to_le_bytes());
        out.extend_from_slice(&self.month.year().to_le_bytes());
        out.push(self.month.month());
        out.push(u8::try_from(self.days.len()).unwrap_or(0));
        out.extend_from_slice(&(if seconds { FLAG_SECONDS } else { 0 }).to_le_bytes());
        for word in [self.generation, self.n_valid] {
            out.extend_from_slice(&word.to_le_bytes());
        }
        for word in [self.first_ts_micros, self.last_ts_micros] {
            out.extend_from_slice(&word.to_le_bytes());
        }
        out.extend_from_slice(&self.counts.folded.to_le_bytes());
        let c = &self.counts;
        for word in [
            c.invalid,
            c.decreasing,
            c.negative_volume,
            c.outside_month,
            c.held_seconds,
            c.held_minutes,
        ] {
            out.extend_from_slice(&word.to_le_bytes());
        }
        let e = &self.extremes;
        for word in [
            e.high,
            e.high_ts,
            e.low,
            e.low_ts,
            e.open,
            e.close,
            e.volume,
            e.open_interest,
        ] {
            out.extend_from_slice(&word.to_le_bytes());
        }
        out.resize(HEAD_LEN, 0);
        for entry in &self.days {
            out.extend_from_slice(&entry.bars.to_le_bytes());
            out.extend_from_slice(&entry.first.to_le_bytes());
            out.extend_from_slice(&entry.last.to_le_bytes());
        }
        for entry in self.days.iter().filter(|entry| entry.held()) {
            put_bits(&mut out, &entry.minutes, entry.first / 60, entry.last / 60);
        }
        if seconds {
            for entry in self.days.iter().filter(|entry| entry.held()) {
                if let Some(words) = entry.seconds.as_deref() {
                    put_bits(&mut out, words, entry.first, entry.last);
                }
            }
        }
        out
    }

    /// The `.sbar` summary section and the `.sum` body: [`VERSION`], two zero
    /// bytes, the payload, and a CRC-32C over everything before it — byte for
    /// byte what [`crate::sealed::Builder`] writes for a summary it is handed.
    #[must_use]
    pub fn section(&self) -> Vec<u8> {
        let payload = self.payload();
        let mut out = Vec::with_capacity(payload.len() + SECTION_OVERHEAD);
        out.extend_from_slice(&VERSION.to_le_bytes());
        out.extend_from_slice(&[0, 0]);
        out.extend_from_slice(&payload);
        let crc = crc32c(&out);
        out.extend_from_slice(&crc.to_le_bytes());
        out
    }

    /// The whole `.sum` sibling: [`MAGIC`] then [`Self::section`].
    #[must_use]
    pub fn file_image(&self) -> Vec<u8> {
        let mut out = MAGIC.to_vec();
        out.extend_from_slice(&self.section());
        out
    }

    /// A payload of `version`, decoded and checked against its own geometry.
    ///
    /// # Errors
    ///
    /// [`Refused::Version`], [`Refused::Shape`] naming the field.
    pub fn decode(version: u16, payload: &[u8]) -> Result<Self, Refused> {
        if version != VERSION {
            return Err(Refused::Version(version));
        }
        if payload.len() < HEAD_LEN || payload.len() > MAX_PAYLOAD_LEN {
            return Err(Refused::Shape("the payload is not a head and its days"));
        }
        let mut at = Cursor {
            bytes: payload,
            at: 0,
            short: false,
        };
        let symbol_id = at.u32();
        let timeframe_secs = at.u32();
        let year = at.u16();
        let month_number = at.u8();
        let n_days = at.u8();
        let month = YearMonth::new(year, month_number)
            .map_err(|_| Refused::Shape("the payload names no month"))?;
        if n_days != days_in(month) {
            return Err(Refused::Shape("the day count is not the month's"));
        }
        let flags = at.u32();
        if flags & !KNOWN_FLAGS != 0 {
            return Err(Refused::Shape("a flag this version does not define"));
        }
        let mut summary = Self::new(symbol_id, timeframe_secs, month);
        summary.generation = at.u64();
        summary.n_valid = at.u64();
        summary.first_ts_micros = at.i64();
        summary.last_ts_micros = at.i64();
        summary.counts.folded = at.u64();
        summary.counts.invalid = at.u32();
        summary.counts.decreasing = at.u32();
        summary.counts.negative_volume = at.u32();
        summary.counts.outside_month = at.u32();
        summary.counts.held_seconds = at.u32();
        summary.counts.held_minutes = at.u32();
        let e = &mut summary.extremes;
        for field in [
            &mut e.high,
            &mut e.high_ts,
            &mut e.low,
            &mut e.low_ts,
            &mut e.open,
            &mut e.close,
            &mut e.volume,
            &mut e.open_interest,
        ] {
            *field = at.i64();
        }
        if payload
            .get(at.at..HEAD_LEN)
            .is_none_or(|rest| rest.iter().any(|b| *b != 0))
        {
            return Err(Refused::Shape("a reserved byte is set"));
        }
        at.at = HEAD_LEN;
        summary.seconds = flags & FLAG_SECONDS != 0;
        let tracked = summary.seconds;
        for entry in &mut summary.days {
            *entry = DayBits::empty(tracked);
            entry.bars = at.u32();
            entry.first = at.u32();
            entry.last = at.u32();
            let none = entry.first == NO_SECOND && entry.last == NO_SECOND;
            let some = entry.first <= entry.last && entry.last < 86_400 && entry.bars > 0;
            if !(some || (none && entry.bars == 0)) {
                return Err(Refused::Shape("a day's first and last second disagree"));
            }
        }
        for entry in summary.days.iter_mut().filter(|entry| entry.held()) {
            at.bits(&mut entry.minutes, entry.first / 60, entry.last / 60)?;
        }
        if tracked {
            for entry in summary.days.iter_mut().filter(|entry| entry.held()) {
                let (first, last) = (entry.first, entry.last);
                if let Some(words) = entry.seconds.as_deref_mut() {
                    at.bits(words, first, last)?;
                }
            }
        }
        if at.short || at.at != payload.len() {
            return Err(Refused::Shape(
                "the payload's length is not its days' geometry",
            ));
        }
        Ok(summary)
    }

    /// A `.sbar` summary section or a `.sum` body after its magic, checked
    /// and decoded.
    ///
    /// # Errors
    ///
    /// [`Refused::Torn`], [`Refused::Checksum`], [`Refused::Version`] and
    /// [`Self::decode`]'s refusals.
    pub fn from_section(section: &[u8]) -> Result<Self, Refused> {
        if section.len() < SECTION_OVERHEAD {
            return Err(Refused::Torn);
        }
        let (body, crc) = section.split_at(section.len() - 4);
        if crc32c(body).to_le_bytes().as_slice() != crc {
            return Err(Refused::Checksum);
        }
        let version = u16::from_le_bytes([
            body.first().copied().unwrap_or(0),
            body.get(1).copied().unwrap_or(0),
        ]);
        if body.get(2..4) != Some([0, 0].as_slice()) {
            return Err(Refused::Version(version));
        }
        Self::decode(version, body.get(4..).unwrap_or_default())
    }

    /// A whole `.sum` sibling's bytes, checked and decoded.
    ///
    /// # Errors
    ///
    /// [`Refused::Torn`], [`Refused::Magic`] and [`Self::from_section`]'s.
    pub fn from_file_image(image: &[u8]) -> Result<Self, Refused> {
        let Some((magic, section)) = image.split_at_checked(MAGIC.len()) else {
            return Err(Refused::Torn);
        };
        if magic != MAGIC {
            return Err(Refused::Magic);
        }
        Self::from_section(section)
    }
}

/// Appends bits `first..=last` of `words`, packed eight to a byte, low bit
/// first. A byte at a time, read off the words with one shift (two across a
/// word boundary), so the cost is the bytes written, not the bits.
fn put_bits(out: &mut Vec<u8>, words: &[u64], first: u32, last: u32) {
    let len = bits_len(first, last);
    let span = last.saturating_sub(first).saturating_add(1);
    out.reserve(len);
    for index in 0..len {
        let offset = u32::try_from(index).unwrap_or(u32::MAX).saturating_mul(8);
        let mut byte = byte_at(words, first.saturating_add(offset));
        // THE LAST BYTE CARRIES ONLY THE RANGE'S BITS, so one summary has one
        // encoding and `Cursor::bits` can refuse any other.
        let valid = span.saturating_sub(offset);
        if valid < 8 {
            byte &= (1_u8 << valid) - 1;
        }
        out.push(byte);
    }
}

/// The eight bits of `words` from bit `at`, low bit first; bits past the end
/// read as zero.
fn byte_at(words: &[u64], at: u32) -> u8 {
    let word = usize::try_from(at / 64).unwrap_or(usize::MAX);
    let shift = at % 64;
    let low = words.get(word).map_or(0, |w| w >> shift);
    let high = if shift > 56 {
        words
            .get(word.saturating_add(1))
            .map_or(0, |w| w << (64 - shift))
    } else {
        0
    };
    #[expect(
        clippy::cast_possible_truncation,
        reason = "the low eight bits, by intent"
    )]
    let byte = (low | high) as u8;
    byte
}

/// ORs the eight bits of `byte` into `words` from bit `at`; bits past the end
/// are dropped (a caller refuses them first).
fn or_byte(words: &mut [u64], at: u32, byte: u8) {
    let word = usize::try_from(at / 64).unwrap_or(usize::MAX);
    let shift = at % 64;
    if let Some(slot) = words.get_mut(word) {
        *slot |= u64::from(byte) << shift;
    }
    if shift > 56
        && let Some(slot) = words.get_mut(word.saturating_add(1))
    {
        *slot |= u64::from(byte) >> (64 - shift);
    }
}

/// Bytes [`put_bits`] writes for `first..=last`.
fn bits_len(first: u32, last: u32) -> usize {
    usize::try_from(last.saturating_sub(first))
        .unwrap_or(usize::MAX)
        .saturating_add(1)
        .div_ceil(8)
}

/// A forward reader over a payload. A read past the end gives zeros and sets
/// `short`, which [`MonthSummary::decode`] refuses once at the end.
struct Cursor<'a> {
    bytes: &'a [u8],
    at: usize,
    short: bool,
}

impl Cursor<'_> {
    fn take<const N: usize>(&mut self) -> [u8; N] {
        let mut out = [0_u8; N];
        match self.bytes.get(self.at..self.at.saturating_add(N)) {
            Some(slice) => out.copy_from_slice(slice),
            None => self.short = true,
        }
        self.at = self.at.saturating_add(N);
        out
    }
    fn u8(&mut self) -> u8 {
        u8::from_le_bytes(self.take::<1>())
    }
    fn u16(&mut self) -> u16 {
        u16::from_le_bytes(self.take::<2>())
    }
    fn u32(&mut self) -> u32 {
        u32::from_le_bytes(self.take::<4>())
    }
    fn u64(&mut self) -> u64 {
        u64::from_le_bytes(self.take::<8>())
    }
    fn i64(&mut self) -> i64 {
        i64::from_le_bytes(self.take::<8>())
    }

    /// Reads bits `first..=last` into `words`, a byte at a time; a set bit
    /// past the range's last bit is refused, so one summary has one encoding.
    fn bits(&mut self, words: &mut [u64], first: u32, last: u32) -> Result<(), Refused> {
        let len = bits_len(first, last);
        let Some(bytes) = self.bytes.get(self.at..self.at.saturating_add(len)) else {
            self.short = true;
            return Err(Refused::Shape("a bitmap runs past the payload"));
        };
        let span = last.saturating_sub(first).saturating_add(1);
        for (index, byte) in (0_u32..).zip(bytes) {
            let offset = index.saturating_mul(8);
            let valid = span.saturating_sub(offset);
            if valid < 8 && byte >> valid != 0 {
                return Err(Refused::Shape("a bitmap sets a bit past its range"));
            }
            or_byte(words, first.saturating_add(offset), *byte);
        }
        self.at = self.at.saturating_add(len);
        Ok(())
    }
}

/// The `.sum` sibling beside the bar file at `bars_path`.
#[must_use]
pub fn path_beside(bars_path: &Path) -> PathBuf {
    bars_path.with_extension(
        crate::path::FileKind::MonthSummary
            .extension()
            .trim_start_matches('.'),
    )
}

/// The summary kept beside the bar file at `bars_path`, if it describes
/// exactly the committed `header` of `month`. One open and one read; no bar
/// is read. A month under [`crate::time_index::INDEX_MIN_ROWS`] bars with no
/// summary is [`Refused::BelowThreshold`], not [`Refused::Absent`] (D-4933).
///
/// # Errors
///
/// Every [`Refused`], named.
pub fn read_beside(
    bars_path: &Path,
    header: &Header,
    month: YearMonth,
) -> Result<MonthSummary, Refused> {
    let summary = read_kept(bars_path, header.n_valid)?;
    summary.describes(header, month)?;
    Ok(summary)
}

/// The summary kept beside the bar file at `bars_path`, decoded and checked
/// (magic, length, CRC, geometry) but NOT compared with a header: the writer's
/// step reads it once and decides from it whether to extend or rebuild, where
/// a reader asks [`read_beside`]. `n_valid` is the month's committed rows, for
/// the small-month rule only.
///
/// Opened as every store sidecar is (`crate::open_flags`): a symbolic link at
/// the summary's own name is refused and never followed, the open never blocks
/// (a FIFO planted at the name cannot hang a request), and anything that is
/// not a regular file is refused by name before a byte is read. At most
/// [`MAX_PAYLOAD_LEN`] plus the section and magic is read, so a planted large
/// file costs one bounded read.
///
/// # Errors
///
/// Every [`Refused`] but [`Refused::NotThisMonth`] and [`Refused::Stale`].
pub fn read_kept(bars_path: &Path, n_valid: u64) -> Result<MonthSummary, Refused> {
    let path = path_beside(bars_path);
    let mut file = match crate::open_flags::open_read_no_follow(&path) {
        Ok(file) => file,
        Err(why) if why.kind() == ErrorKind::NotFound => {
            return Err(if crate::time_index::below_threshold(n_valid) {
                Refused::BelowThreshold
            } else {
                Refused::Absent
            });
        }
        Err(why) => {
            #[cfg(any(
                target_os = "macos",
                all(
                    any(target_os = "linux", target_os = "android"),
                    any(target_arch = "x86_64", target_arch = "aarch64")
                )
            ))]
            if why.raw_os_error() == Some(crate::open_flags::ELOOP) {
                return Err(Refused::Unreadable(format!(
                    "{} is a symbolic link; no link is followed",
                    path.display()
                )));
            }
            return Err(Refused::Unreadable(format!("{}: {why}", path.display())));
        }
    };
    let regular = file
        .metadata()
        .map_err(|why| Refused::Unreadable(format!("{}: {why}", path.display())))?
        .is_file();
    if !regular {
        return Err(Refused::Unreadable(format!(
            "{} is not a regular file",
            path.display()
        )));
    }
    let limit =
        u64::try_from(MAGIC.len() + SECTION_OVERHEAD + MAX_PAYLOAD_LEN + 1).unwrap_or(u64::MAX);
    let mut image = Vec::new();
    file.by_ref()
        .take(limit)
        .read_to_end(&mut image)
        .map_err(|why| Refused::Unreadable(format!("{}: {why}", path.display())))?;
    MonthSummary::from_file_image(&image)
}

#[cfg(test)]
#[path = "month_summary_tests.rs"]
mod tests;
