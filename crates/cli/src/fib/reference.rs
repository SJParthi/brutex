//! The reference stamps a FIB trade carries — NIFTY, BANKNIFTY, every stored
//! index, INDIA VIX, and a reserved GIFT NIFTY slot — never a coordinate,
//! never an identity term (FIB-8; D-6305, D-6443, FM-F087).
//!
//! # A trade carries its (day, minute) key only
//!
//! Stamping every index directory on every trade would cost O(indices) per
//! trade and widen every record (FM-F087). So a trade carries only its
//! `(day, minute)` key, and the month's reference table — a fixed-stride
//! `[day][minute][index slot]` table with the index list in its header — is
//! joined in O(1) per `(trade, index)` asked for ([`ReferenceMonth::stamp`],
//! [`ReferenceFile::stamp`]). A new index is a new reference-file version; a
//! candidate's stride never changes for it.
//!
//! # Minutes from Zerodha (operator rule a)
//!
//! Each index's month is read through [`load_index_month`], the named
//! reference loader in the shape of `cli::vix_reference`: it hard-codes the
//! index kind and the one-minute rung and is never offered to the sweep
//! (FM-F099). INDIA VIX is read through `cli::vix_reference::VixReferenceMonth`
//! — `VixStamp::Exact` or `Absent`, never a fill or a zero. GIFT NIFTY's slot
//! is reserved and always absent: its source is UNVERIFIED.
//!
//! # The move and the screen
//!
//! A stamp is the minute's close and the previous TRADABLE day's last minute
//! close it is measured against ([`ReferenceCell::Exact`]). On a day PREC-9's
//! census marks foreign, rescaled or junk, the stamp is absent with that
//! reason — NIFTY and BANKNIFTY included; until PREC-9 merges every day is
//! stamped as measured and labelled `index day unscreened`. No own-past
//! magnitude test ever blanks a stamp: an unflagged −8% day is stamped as
//! measured.

use std::collections::HashMap;
use std::hash::BuildHasher;
use std::path::{Path, PathBuf};

use brutex_core::instrument::{Exchange, InstrumentKey};
use brutex_core::vendor::Vendor;
use indicators::Candle;
use store::file::{BarFile, StoreError};
use store::path::{FileKind, StorePath, Timeframe, YearMonth};

use super::authorities::{CalendarAuthority, IndexDay, IndexScreen};
use super::day::{date, ist_day_second};
use super::files::{self, FibFile, FileError, Put, ReferenceCell, Scope, Take};
use crate::vix_reference::{VIX_REFERENCE_SYMBOL, VixReferenceMonth, VixStamp};

/// The two swept indices, stamped first.
pub const SWEPT_INDICES: &[&str] = &["NIFTY", "BANKNIFTY"];
/// The reserved GIFT NIFTY slot's name.
pub const GIFT_NIFTY: &str = "GIFTNIFTY";

/// Why a stamp is absent, by its position in [`AbsentReason::ALL`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AbsentReason {
    /// The index has no minute at that instant: no value, never a fill.
    NoMinute,
    /// PREC-9 marks the index day foreign.
    Foreign,
    /// PREC-9 marks the index day rescaled.
    Rescaled,
    /// PREC-9 marks the index day junk.
    Junk,
    /// GIFT NIFTY: no source is recorded (UNVERIFIED).
    SourceUnverified,
}

impl AbsentReason {
    /// Every reason, in position order.
    pub const ALL: &[Self] = &[
        Self::NoMinute,
        Self::Foreign,
        Self::Rescaled,
        Self::Junk,
        Self::SourceUnverified,
    ];

    /// The counted name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::NoMinute => "no_minute",
            Self::Foreign => "index_day_foreign",
            Self::Rescaled => "index_day_rescaled",
            Self::Junk => "index_day_junk",
            Self::SourceUnverified => "source UNVERIFIED",
        }
    }

    /// The byte a reference cell stores.
    #[must_use]
    pub const fn byte(self) -> u8 {
        self as u8
    }

    /// The reason a byte names.
    #[must_use]
    pub fn of_byte(byte: u8) -> Option<Self> {
        Self::ALL.get(usize::from(byte)).copied()
    }

    /// The reason PREC-9's verdict gives, if it blanks the stamp.
    #[must_use]
    pub const fn of_screen(screen: IndexDay) -> Option<Self> {
        match screen {
            IndexDay::Unscreened | IndexDay::Clean => None,
            IndexDay::Foreign => Some(Self::Foreign),
            IndexDay::Rescaled => Some(Self::Rescaled),
            IndexDay::Junk => Some(Self::Junk),
        }
    }
}

/// The minutes a reference day holds: the index session, 09:15 to 15:29
/// (indices stay `[09:15, 15:30)` in the closing-auction era, D-3320).
#[must_use]
pub fn minutes_per_day() -> usize {
    usize::from(pull::calendar::FULL_BARS)
}

/// The minute slot of an instant: minutes after 09:15, or `None` outside the
/// index session.
#[must_use]
pub fn minute_slot(ts_micros: i64) -> Option<usize> {
    let second = ist_day_second(ts_micros).1;
    let minute = second / u32::try_from(vocab::fib_cube::SECONDS_PER_MINUTE).ok()?;
    let slot = usize::try_from(minute.checked_sub(u32::from(pull::calendar::OPEN_MINUTE))?).ok()?;
    (slot < minutes_per_day()).then_some(slot)
}

/// One index month of Zerodha minutes through the named reference loader: the
/// index kind and the one-minute rung are fixed; `None` when the month file is
/// absent.
///
/// # Errors
///
/// A refusal by name for a symbol no index key takes, a month file that is
/// locked, unreadable or corrupt, or a minute that is impossible, off the
/// grid, outside the month or not strictly after the one before.
pub fn load_index_month(
    root: &Path,
    symbol: &str,
    (year, month): (u16, u8),
) -> Result<Option<Vec<Candle>>, String> {
    let key = InstrumentKey::index(Exchange::Nse, symbol)
        .map_err(|why| format!("index_key_refused: {symbol}: {why}"))?;
    let ym = YearMonth::new(year, month).map_err(|why| format!("{year}-{month:02}: {why}"))?;
    let path = StorePath::for_key(
        Vendor::Zerodha,
        &key,
        Timeframe::MINUTE_1,
        ym,
        FileKind::Bars,
    )
    .map_err(|why| format!("index_key_refused: {symbol}: {why}"))?;
    #[expect(
        clippy::cast_possible_truncation,
        reason = "the store's symbol id is the low 32 bits of FNV-1a, as every reader asks it"
    )]
    let symbol_id = brutex_core::universe::fnv1a(key.underlying.as_str()) as u32;
    let file = match BarFile::open_existing(root, path, symbol_id) {
        Ok(file) => file,
        Err(StoreError::Missing { .. }) => return Ok(None),
        Err(why) => return Err(format!("{symbol} 1min {ym}: {why}")),
    };
    let mut bars = Vec::new();
    let mut previous: Option<i64> = None;
    for record in 0..file.records() {
        let bar = file
            .read_record(record)
            .map_err(|why| format!("{symbol} 1min {ym} record {record}: {why}"))?;
        let on_grid = bar.ts_micros.rem_euclid(minute_micros()) == 0;
        let in_month = civil_month(ist_day_second(bar.ts_micros).0) == Some((year, month));
        let after = previous.is_none_or(|before| bar.ts_micros > before);
        if !bar.ohlc_is_sane() || !bar.counts_are_sane() || !on_grid || !in_month || !after {
            return Err(format!(
                "{symbol} 1min {ym} record {record} at {} is not a reference minute: impossible, \
                 off the grid, outside the month or out of order; refused",
                bar.ts_micros
            ));
        }
        previous = Some(bar.ts_micros);
        bars.push(Candle {
            ts_micros: bar.ts_micros,
            open: bar.open,
            high: bar.high,
            low: bar.low,
            close: bar.close,
            volume: bar.volume,
            open_interest: bar.open_interest,
        });
    }
    Ok(Some(bars))
}

/// A minute in microseconds.
fn minute_micros() -> i64 {
    i64::try_from(vocab::fib_cube::SECONDS_PER_MINUTE * vocab::fib_cube::MICROS_PER_SECOND)
        .unwrap_or(i64::MAX)
}

/// The civil `(year, month)` of an IST epoch day.
fn civil_month(day: i64) -> Option<(u16, u8)> {
    let civil = pull::session::Day::from_days(u32::try_from(day).ok()?).ok()?;
    Some((civil.year(), civil.month()))
}

/// The directory every Zerodha NSE index is stored under.
fn index_root(root: &Path) -> Option<PathBuf> {
    let key = InstrumentKey::index(Exchange::Nse, SWEPT_INDICES.first()?).ok()?;
    let ym = YearMonth::new(pull::session::Day::from_days(0).ok()?.year(), 1).ok()?;
    let path = StorePath::for_key(
        Vendor::Zerodha,
        &key,
        Timeframe::MINUTE_1,
        ym,
        FileKind::Bars,
    )
    .ok()?
    .to_path_buf(root);
    // `<index root>/<symbol>/<rung>/<month file>`.
    Some(path.parent()?.parent()?.parent()?.to_path_buf())
}

/// Whether an index directory's name is one the table lists by name — the
/// swept pair or INDIA VIX — rather than among the sorted others. A `match`
/// over the three names, which `the_named_indices_are_the_lists` holds equal
/// to [`SWEPT_INDICES`] and the VIX symbol.
fn is_named_index(name: &str) -> bool {
    matches!(name, "NIFTY" | "BANKNIFTY" | "INDIAVIX")
}

/// Every index the store holds Zerodha minutes for, in the order a reference
/// table lists them: NIFTY and BANKNIFTY, every other index directory sorted,
/// INDIA VIX, and the reserved GIFT NIFTY slot.
///
/// # Errors
///
/// `index_root_unreadable` when the index directory exists and cannot be listed.
pub fn reference_indices(root: &Path) -> Result<Vec<String>, String> {
    let mut others = Vec::new();
    if let Some(dir) = index_root(root).filter(|dir| dir.is_dir()) {
        let entries = std::fs::read_dir(&dir)
            .map_err(|why| format!("index_root_unreadable: {}: {why}", dir.display()))?;
        for entry in entries {
            let entry =
                entry.map_err(|why| format!("index_root_unreadable: {}: {why}", dir.display()))?;
            let name = entry.file_name().to_string_lossy().into_owned();
            let named = is_named_index(&name);
            if entry.path().is_dir() && !named {
                others.push(name);
            }
        }
    }
    others.sort_unstable();
    let mut out: Vec<String> = SWEPT_INDICES
        .iter()
        .map(|name| (*name).to_owned())
        .collect();
    out.extend(others);
    out.push(VIX_REFERENCE_SYMBOL.to_owned());
    out.push(GIFT_NIFTY.to_owned());
    Ok(out)
}

/// One index's month for a reference table.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum IndexSource {
    /// Zerodha minutes, oldest first.
    Minutes(Vec<Candle>),
    /// INDIA VIX's validated month.
    Vix,
    /// No month held: every stamp absent, `no_minute`.
    Missing,
    /// GIFT NIFTY: no source.
    Unsourced,
}

/// A month's reference table: `[day][minute][index slot]`, the index list in
/// its header.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReferenceMonth {
    /// The year.
    pub year: u16,
    /// The month.
    pub month: u8,
    /// The index list, in slot order.
    pub indices: Vec<String>,
    /// Days in the month.
    pub days: usize,
    cells: Vec<ReferenceCell>,
    /// Absent stamps per index slot, counted.
    pub holes: Vec<u64>,
    /// Each index's last tradable close carried out of the month: the next
    /// month's previous closes.
    pub carried: HashMap<String, i64>,
    /// The screen's label: `index day unscreened` until PREC-9 merges.
    pub screen_label: &'static str,
}

impl ReferenceMonth {
    /// The slot of an index by name.
    #[must_use]
    pub fn index_slot(&self, name: &str) -> Option<usize> {
        self.indices.iter().position(|index| index == name)
    }

    /// The stamp of index `slot` at the trade key `ts_micros` (a minute of
    /// this month). O(1): one multiply-add and one read.
    /// Measured: `crates/cli/benches/fib.rs` (FIB-8's rows).
    #[must_use]
    pub fn stamp(&self, ts_micros: i64, slot: usize) -> Option<ReferenceCell> {
        let at = cell_index(self.year, self.month, self.indices.len(), ts_micros, slot)?;
        self.cells.get(at).copied()
    }
}

/// The record index of `(day, minute, slot)`: `((day - 1) × minutes +
/// minute) × indices + slot`, or `None` outside the month or the session.
fn cell_index(year: u16, month: u8, indices: usize, ts_micros: i64, slot: usize) -> Option<usize> {
    if slot >= indices {
        return None;
    }
    let day = ist_day_second(ts_micros).0;
    if civil_month(day) != Some((year, month)) {
        return None;
    }
    let first = i64::from(
        pull::session::Day::new(year, month, 1)
            .ok()?
            .days_from_epoch(),
    );
    let day_index = usize::try_from(day - first).ok()?;
    let minute = minute_slot(ts_micros)?;
    Some((day_index * minutes_per_day() + minute) * indices + slot)
}

/// Builds a month's reference table.
///
/// For each index, streamed in time order: each minute's close is stamped
/// against the previous TRADABLE day's last minute close (`previous` holds
/// each index's carried close from the month before); a day `screen` blanks
/// is absent with its reason; a minute with no bar is absent, `no_minute`.
/// INDIA VIX is read through `vix`, GIFT NIFTY is always absent.
#[must_use]
pub fn build_reference<S: BuildHasher>(
    (year, month): (u16, u8),
    sources: &[(String, IndexSource)],
    previous: &HashMap<String, i64, S>,
    screen: &dyn IndexScreen,
    calendar: &dyn CalendarAuthority,
    vix: Option<&VixReferenceMonth>,
) -> ReferenceMonth {
    let first = pull::session::Day::new(year, month, 1).ok();
    let first_day = first.map_or(0, |day| i64::from(day.days_from_epoch()));
    let days = first.map_or(0, |day| usize::from(day.end_of_month().day()));
    let indices = sources.len();
    let minutes = minutes_per_day();
    let mut cells =
        vec![ReferenceCell::Absent(AbsentReason::NoMinute.byte()); days * minutes * indices];
    let mut holes = vec![0_u64; indices];
    let mut carried = HashMap::with_capacity(sources.len());
    for (slot, (name, source)) in sources.iter().enumerate() {
        let mut last_tradable = previous.get(name).copied();
        let mut today_last: Option<i64> = None;
        // Each index's minutes are streamed once, in time order: a day takes
        // the minutes up to its end and no further.
        let mut stream = match source {
            IndexSource::Minutes(bars) => bars.as_slice(),
            IndexSource::Vix | IndexSource::Missing | IndexSource::Unsourced => &[],
        }
        .iter()
        .peekable();
        let mut minutes_of = |day: i64| -> Vec<Option<Candle>> {
            let mut row = vec![None; minutes];
            while let Some(bar) = stream.next_if(|bar| ist_day_second(bar.ts_micros).0 <= day) {
                let today = ist_day_second(bar.ts_micros).0 == day;
                if let Some(at) = minute_slot(bar.ts_micros)
                    .filter(|_| today)
                    .and_then(|at| row.get_mut(at))
                {
                    *at = Some(*bar);
                }
            }
            if matches!(source, IndexSource::Vix) {
                for (minute, at) in row.iter_mut().enumerate() {
                    let ts = super::day::instant(
                        day,
                        (u32::from(pull::calendar::OPEN_MINUTE)
                            + u32::try_from(minute).unwrap_or(0))
                            * u32::try_from(vocab::fib_cube::SECONDS_PER_MINUTE).unwrap_or(0),
                    );
                    if let Some(Ok(VixStamp::Exact(bar))) = vix.map(|month| month.stamp(ts)) {
                        *at = Some(bar);
                    }
                }
            }
            row
        };
        for day_index in 0..days {
            let day = first_day + i64::try_from(day_index).unwrap_or(0);
            let blank = match source {
                IndexSource::Unsourced => Some(AbsentReason::SourceUnverified),
                _ => AbsentReason::of_screen(screen.screen(name, day)),
            };
            let row = minutes_of(day);
            for (minute, bar) in row.iter().enumerate() {
                let at = (day_index * minutes + minute) * indices + slot;
                let cell = match (blank, bar) {
                    (Some(reason), _) => ReferenceCell::Absent(reason.byte()),
                    (None, None) => ReferenceCell::Absent(AbsentReason::NoMinute.byte()),
                    (None, Some(bar)) => match last_tradable {
                        Some(previous_close) => ReferenceCell::Exact {
                            close: bar.close,
                            previous_close,
                        },
                        None => ReferenceCell::NoPrevious { close: bar.close },
                    },
                };
                if matches!(cell, ReferenceCell::Absent(_))
                    && let Some(hole) = holes.get_mut(slot)
                {
                    *hole += 1;
                }
                if let Some(entry) = cells.get_mut(at) {
                    *entry = cell;
                }
                if let Some(bar) = bar {
                    today_last = Some(bar.close);
                }
            }
            // The day closes: its last minute close becomes the previous
            // close when the day was tradable and the screen kept it.
            if calendar.market(day).is_ok()
                && blank.is_none()
                && let Some(close) = today_last
            {
                last_tradable = Some(close);
            }
            today_last = None;
        }
        if let Some(close) = last_tradable {
            carried.insert(name.clone(), close);
        }
    }
    ReferenceMonth {
        year,
        month,
        indices: sources.iter().map(|(name, _)| name.clone()).collect(),
        days,
        cells,
        holes,
        carried,
        screen_label: screen.version().label,
    }
}

/// The reference file's profile: the index list, framed.
fn profile(indices: &[String]) -> Vec<u8> {
    let mut out = Put::default();
    out.u32(u32::try_from(indices.len()).unwrap_or(u32::MAX));
    for name in indices {
        out.u32(u32::try_from(name.len()).unwrap_or(u32::MAX));
        out.bytes(name.as_bytes());
    }
    out.0
}

/// Writes a month's reference table as one sealed `reference` file.
///
/// # Errors
///
/// Every `files::write_records` refusal.
pub fn write_reference(
    path: &Path,
    run_identity: [u8; files::DIGEST],
    schema_digest: [u8; files::DIGEST],
    table: &ReferenceMonth,
) -> Result<files::Written, FileError> {
    files::write_records(
        path,
        run_identity,
        schema_digest,
        Scope::Month {
            year: table.year,
            month: table.month,
        },
        profile(&table.indices),
        &table.cells,
    )
}

/// An open reference file: its index list read from the header.
#[derive(Debug)]
pub struct ReferenceFile {
    file: FibFile,
    year: u16,
    month: u8,
    /// The index list, in slot order.
    pub indices: Vec<String>,
}

impl ReferenceFile {
    /// Opens a sealed reference file and reads its index list.
    ///
    /// # Errors
    ///
    /// Every `FibFile::open` refusal; `wrong_kind` for a file that is not a
    /// month's reference table; `bad_profile` for an index list that does not
    /// parse or does not match the record count.
    pub fn open(path: &Path) -> Result<Self, FileError> {
        let file = FibFile::open(path)?;
        let sealed = file.sealed().clone();
        let Scope::Month { year, month } = sealed.header.scope else {
            return Err(FileError {
                word: "wrong_kind",
                detail: format!("{} is not a month's reference table", path.display()),
            });
        };
        if sealed.header.kind != files::Kind::Reference {
            return Err(FileError {
                word: "wrong_kind",
                detail: format!("{} holds {}", path.display(), sealed.header.kind.name()),
            });
        }
        let bad = |why: String| FileError {
            word: "bad_profile",
            detail: format!("{}: {why}", path.display()),
        };
        let mut take = Take::new(&sealed.header.profile);
        let count = take.u32().map_err(bad)?;
        let mut indices = Vec::new();
        for _ in 0..count {
            let length = usize::try_from(take.u32().map_err(bad)?).unwrap_or(usize::MAX);
            let name = take.bytes(length).map_err(bad)?;
            indices.push(String::from_utf8_lossy(name).into_owned());
        }
        let days = pull::session::Day::new(year, month, 1)
            .map_or(0, |day| usize::from(day.end_of_month().day()));
        let expected = days * minutes_per_day() * indices.len();
        if take.left() != 0 || u64::try_from(expected).ok() != Some(sealed.records) {
            return Err(bad(format!(
                "{} indices over {days} days do not make its {} records",
                indices.len(),
                sealed.records
            )));
        }
        Ok(Self {
            file,
            year,
            month,
            indices,
        })
    }

    /// The stamp of index `slot` at the trade key `ts_micros`. O(1): one
    /// record read (its block verified on first touch).
    ///
    /// Measured: `crates/cli/benches/fib.rs` (FIB-8's rows).
    ///
    /// # Errors
    ///
    /// `outside_table` for a key outside the month or the session, or a slot
    /// past the list; every `FibFile::read` refusal.
    pub fn stamp(&mut self, ts_micros: i64, slot: usize) -> Result<ReferenceCell, FileError> {
        let at = cell_index(self.year, self.month, self.indices.len(), ts_micros, slot)
            .ok_or_else(|| FileError {
                word: "outside_table",
                detail: format!(
                    "{} slot {slot} is outside the table",
                    date(ist_day_second(ts_micros).0)
                ),
            })?;
        self.file.read(u64::try_from(at).unwrap_or(u64::MAX))
    }
}

/// The reference cell's move in basis points, floored, exact in `i128`;
/// `None` when the stamp is absent or has no previous close.
#[must_use]
pub fn move_bp(cell: ReferenceCell) -> Option<i128> {
    match cell {
        ReferenceCell::Exact {
            close,
            previous_close,
        } if previous_close > 0 => Some(
            ((i128::from(close) - i128::from(previous_close))
                * i128::from(vocab::fib_cube::BP_PER_UNIT))
            .div_euclid(i128::from(previous_close)),
        ),
        _ => None,
    }
}

#[cfg(test)]
#[path = "reference_tests.rs"]
mod tests;
