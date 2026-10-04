//! The GDFL import: indices, stocks and option contracts, read from the
//! vendor's zips or the verified tick store, folded to one-second OHLCV bars
//! and filed through the one ingest door. D-2802 to D-2807.
//!
//! # One second, and nothing coarser (operator, 4 Oct 2026, D-2807)
//!
//! GDFL supplies the one-second rung ONLY. This module writes `1s` and no
//! other rung, and `crate::ingest` derives no coarser rung from a `gdfl`
//! bar on any path (`ingest::derives_for`): minutes and every higher
//! timeframe come from the Zerodha one-minute pulls, derived by the app.
//!
//! # One runtime for every kind
//!
//! A source only LISTS a day and READS its files ([`crate::gdfl_cm`] for the
//! capital-market trees, [`crate::gdfl_nfo`] for options); everything after
//! that is this module, once, for all three kinds: the calendar and venue
//! gate, the journal, the row rule, the placement of late rows, the fold and
//! the write. Two sources and three kinds meet in [`drive`], so a bar from a
//! zip and a bar from the tick store take byte for byte the same path to
//! disk, and a run over either leaves the same store
//! (`tests::every_kind_lands_at_one_second_and_both_sources_leave_the_same_store`,
//! and over random worlds DPT-13; the `CM-GI-02` this cited is no row).
//!
//! # The bar (D-2802, the agreed definition of 4 Oct 2026)
//!
//! - **Stocks and options:** rows with LTQ > 0 only; open the first such
//!   row of the second, high and low the extremes, close the last (file
//!   order), volume the sum of LTQ. A second with no traded row has no bar.
//! - **Index spot:** every row (an index row carries no quantity), volume 0.
//! - **Open interest:** the options file's `OpenInterest`, the last of the
//!   bucket; `OI_NULL` for the capital-market trees, which carry none.
//! - The second is folded by [`crate::fold`], the one fold authority.
//!
//! # A late row is deferred, never moved earlier (D-0805's rule, D-2802)
//!
//! [`crate::fold`] refuses a back-step, and the vendor's files hold them
//! (the design's §1.3 measures single late rows, replays and a forward
//! stamp). So before the fold every kept row is PLACED: a row stamped at or
//! after the running maximum of EVERY row before it, untraded rows included
//! (D-3170), is in order and keeps its stamp; a late row is
//! placed at the stamp of the next in-order row of any kind after it in the file, the
//! earliest second it is proven to have printed by. A late row with no
//! in-order row after it is dropped and counted (`late_unresolved`). A price
//! can be delayed, never advanced, so no bar holds a print from its future
//! (`CLAUDE.md` §3 rule 7). The design's UTC re-stamp is NOT applied: such a
//! row is deferred like any other late row, which is the conservative
//! direction, and is counted in `late_rows`.
//!
//! A deferral is never silent about its size (D-3172): one forward-stamped
//! row makes every row after it late until the clock catches up, and they
//! all land in one second. The largest back-step of a run is in
//! [`Report::max_back_s`], each file with a late row is logged at `Warn`
//! with its counts, and the journal's `done` line carries both. No bound
//! refuses a file for it: none is measured over the archive.
//!
//! # Incremental, idempotent, resumable (D-2803)
//!
//! The store itself refuses a duplicate: `BarFile::append` answers
//! `AlreadyPresent` for bars it holds and appends only a following suffix.
//! On top of that a journal, `<store>/imports/gdfl.journal`, records `begin`
//! before a day's first write and `done` after a clean day, keyed by kind,
//! day and instrument filter. A re-run skips every `done` day without reading
//! the source (O(1) per day), writes nothing and leaves the store byte for
//! byte as it was. A `begin` with no `done` is a crashed or incomplete run: it
//! is named (`resumed`) and the day is imported again, the store keeping what
//! it holds and appending the rest, the census re-counted. A day older than
//! what a month file already holds is refused by the store by name, never
//! inserted.
//!
//! # Refusals
//!
//! A run-level refusal (a bad range, an unreadable journal) is an `Err`. A
//! day-level one (calendar, venue hours, a listing the source refuses) and a
//! file-level one (any reader refusal, a volume past `i64`) is one named
//! [`Failure`] in the report and in the log, and the run continues. A closed
//! day is counted, a day the source does not hold is counted and named.
//!
//! # Cost
//!
//! Per day: one listing (O(entries)), then per instrument file one fetch,
//! check and decode (O(bytes)), one placement (two passes, O(rows)), two
//! folds (O(rows)), and two `from_rows` appends (O(bars + log n_valid +
//! blocks touched), `docs/06-limits.md`). The census is published once per
//! day ([`crate::ingest::record_held`], O(entries held)). None of this is a
//! constant-time claim; `docs/06-limits.md` states it. What IS constant per
//! lookup is the stored result: a bar by index is one positional read
//! (`store::file::BarFile::read_record`, C-28/C-29), a ticker in a listing is
//! one slot index (CM-13) or one hash probe (`gdfl_nfo::NfoDay::locate`,
//! DPN-04; the `C-GI-01/02` rows this cited do not exist, D-3166).

use std::collections::{HashMap, HashSet};
use std::io::Write as _;
use std::path::{Path, PathBuf};

use brutex_core::instrument::Contract;
use brutex_core::vendor::Vendor;
use store::format::{Bar, OI_NULL};

use crate::fetch::BarRequest;
use crate::fold::{Bucket, fold};
use crate::gdfl_cm::{
    CmKind, CmRefusal, CmSource, Resolution, read_listed, resolve, session_gate, split_name,
};
use crate::gdfl_nfo::{NfoRefusal, NfoSource, decode_ticker, read_file};
use crate::ingest::{Failure, Ingested, Plan, from_rows, record_held};
use crate::manifest::Held;
use crate::session::{Day, IST_OFFSET_SECS, Window};
use crate::vendor::{Granularity, Listing, PriceScale, TimestampEncoding};

/// The telemetry target every event of this module carries.
pub const TARGET: &str = "gdfl.import";

/// Which tree a run imports.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ImportKind {
    /// NIFTY 50 and NIFTY BANK, filed as `NSE/INDEX/NIFTY` and `.../BANKNIFTY`.
    Indices,
    /// The F&O shares' cash equities, filed as `NSE/CASH/<SYM>`.
    Stocks,
    /// Option contracts, filed as `NSE/FNO/<UNDERLYING>/<contract>`.
    Options,
}

impl ImportKind {
    /// Every kind, in import order.
    pub const ALL: [Self; 3] = [Self::Indices, Self::Stocks, Self::Options];

    /// The word for this kind, as the `cli` verb takes it.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Indices => "indices",
            Self::Stocks => "stocks",
            Self::Options => "options",
        }
    }

    /// The kind `word` names.
    ///
    /// # Errors
    ///
    /// [`ImportRefusal::KindUnknown`] for any other word.
    pub fn parse(word: &str) -> Result<Self, ImportRefusal> {
        Self::ALL
            .into_iter()
            .find(|kind| kind.as_str() == word)
            .ok_or_else(|| ImportRefusal::KindUnknown {
                word: word.to_owned(),
            })
    }

    /// The store segment this kind is filed under.
    #[must_use]
    pub const fn segment(self) -> &'static str {
        match self {
            Self::Indices => "INDEX",
            Self::Stocks => "CASH",
            Self::Options => "FNO",
        }
    }

    /// The listing class, which decides the venue's hours.
    #[must_use]
    pub const fn listing(self) -> Listing {
        match self {
            Self::Indices => Listing::Index,
            Self::Stocks => Listing::Equity,
            Self::Options => Listing::Derivative,
        }
    }

    /// Whether every row is a price (an index level) or only a traded row is.
    #[must_use]
    pub const fn every_row_counts(self) -> bool {
        matches!(self, Self::Indices)
    }
}

/// Why a run, a day or a file was refused. Every refusal names itself.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ImportRefusal {
    /// The kind word is not `indices`, `stocks` or `options`.
    KindUnknown {
        /// The word.
        word: String,
    },
    /// The first day is after the last.
    RangeBackwards {
        /// The first day asked.
        from: Day,
        /// The last day asked.
        to: Day,
    },
    /// A traded quantity, or a second's sum of them, past `i64::MAX`.
    VolumeOverflow {
        /// The second of the day it happened at.
        sod: u32,
    },
    /// The fold refused, in its own words.
    Fold {
        /// Why.
        why: String,
    },
    /// The journal could not be read or written, or is not a journal.
    Journal {
        /// Why.
        why: String,
    },
    /// A filter name that is not a symbol exactly as the store files it
    /// (D-3171). The journal keys a day by the filter's names joined by `,`
    /// and split on ` `, and `*` is the whole tree: a name holding a space,
    /// a comma, a `*`, a lowercase letter or nothing would match no file
    /// and still record the day `done` under a key another filter shares.
    FilterName {
        /// The name as given.
        name: String,
    },
}

impl core::fmt::Display for ImportRefusal {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::KindUnknown { word } => write!(
                f,
                "{word:?} is not a GDFL import kind; the kinds are indices, stocks and options"
            ),
            Self::RangeBackwards { from, to } => write!(f, "the range {from}..{to} runs backwards"),
            Self::VolumeOverflow { sod } => write!(
                f,
                "the traded quantity at second {sod} of the day does not fit an i64; refused, never saturated"
            ),
            Self::Fold { why } => write!(f, "the fold refused: {why}"),
            Self::Journal { why } => write!(f, "the import journal: {why}"),
            Self::FilterName { name } => write!(
                f,
                "{name:?} is not a symbol as the store files it (A-Z, 0-9, -, _, &, at most 24); \
                 refused before the journal keys a day by it"
            ),
        }
    }
}

impl core::error::Error for ImportRefusal {}

/// One row as both readers hand it on: its second, its price in paisa, its
/// traded quantity and, for options, its open interest.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tick {
    /// Second of the IST day, as stamped.
    pub sod: u32,
    /// Last traded price, paisa.
    pub ltp: i64,
    /// Last traded quantity.
    pub ltq: u64,
    /// Open interest, `None` where the tree carries none.
    pub oi: Option<i64>,
}

/// What the row rule and the placement did to one file's rows.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Placement {
    /// Rows the file held.
    pub rows: usize,
    /// Rows dropped because they record no trade (LTQ = 0, stocks and options).
    pub ltq_zero_dropped: usize,
    /// Kept rows stamped below the running maximum, each deferred.
    pub late_rows: usize,
    /// Late rows with no in-order row after them, dropped.
    pub late_unresolved: usize,
    /// The largest back-step seen, in seconds.
    pub max_back_s: u32,
}

/// One file's bars.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Converted {
    /// One-second bars, one per second holding a kept row.
    pub seconds: Vec<Bar>,
    /// What the rule and the placement did.
    pub placement: Placement,
}

/// The UTC microsecond of second `sod` of IST day `day`.
fn micros_at(day: Day, sod: u32) -> i64 {
    (i64::from(day.days_from_epoch()) * 86_400 + i64::from(sod) - IST_OFFSET_SECS) * 1_000_000
}

/// The rows of one file, filtered by the row rule and placed (module doc).
/// Returns each kept row with its placed second, in file order.
///
/// The running maximum and the next in-order stamp are taken over EVERY row
/// of the file, the untraded ones included, and only then is the row rule
/// applied (D-3170). An untraded row's stamp is the file's evidence of time
/// exactly as a traded row's is: a trade written after a row stamped
/// 10:00:05 was known no earlier than 10:00:05, whatever that row's LTQ.
/// Filtering first discarded that evidence and filed such a trade at its own
/// earlier stamp, a print in a bar before the file had shown it
/// (`CLAUDE.md` §3 rule 7). The counts stay about kept rows: `late_rows`,
/// `late_unresolved` and `max_back_s` count only rows the rule keeps.
fn place(kind: ImportKind, ticks: &[Tick], placement: &mut Placement) -> Vec<(u32, Tick)> {
    placement.rows = ticks.len();
    let counts = |tick: &Tick| kind.every_row_counts() || tick.ltq > 0;
    // Forward: in order when at or above the running maximum of every row.
    let mut high: Option<u32> = None;
    let mut in_order = Vec::with_capacity(ticks.len());
    for tick in ticks {
        match high {
            Some(max) if tick.sod < max => {
                if counts(tick) {
                    placement.late_rows += 1;
                    placement.max_back_s = placement.max_back_s.max(max - tick.sod);
                }
                in_order.push(false);
            }
            _ => {
                high = Some(tick.sod);
                in_order.push(true);
            }
        }
    }
    // Backward: a late row lands at the next in-order row's stamp.
    let mut next: Option<u32> = None;
    let mut placed: Vec<Option<u32>> = vec![None; ticks.len()];
    for (at, tick) in ticks.iter().enumerate().rev() {
        if in_order.get(at).copied().unwrap_or(true) {
            next = Some(tick.sod);
        }
        if let Some(slot) = placed.get_mut(at) {
            *slot = next;
        }
    }
    let mut out = Vec::with_capacity(ticks.len());
    for (tick, second) in ticks.iter().copied().zip(placed) {
        if !counts(&tick) {
            placement.ltq_zero_dropped += 1;
            continue;
        }
        match second {
            Some(second) => out.push((second, tick)),
            None => placement.late_unresolved += 1,
        }
    }
    out
}

/// One file's rows as one-second bars under the agreed
/// definition (module doc).
///
/// # Errors
///
/// [`ImportRefusal::VolumeOverflow`] for a quantity or a second's sum past
/// `i64::MAX`, and [`ImportRefusal::Fold`] for anything else the fold refuses.
pub fn convert(kind: ImportKind, day: Day, ticks: &[Tick]) -> Result<Converted, ImportRefusal> {
    let mut placement = Placement::default();
    let placed = place(kind, ticks, &mut placement);
    let mut rows = Vec::with_capacity(placed.len());
    for (second, tick) in placed {
        let volume = if kind.every_row_counts() {
            0
        } else {
            i64::try_from(tick.ltq).map_err(|_| ImportRefusal::VolumeOverflow { sod: second })?
        };
        rows.push(Bar {
            ts_micros: micros_at(day, second),
            open: tick.ltp,
            high: tick.ltp,
            low: tick.ltp,
            close: tick.ltp,
            volume,
            open_interest: tick.oi.unwrap_or(OI_NULL),
        });
    }
    let refused = |why: crate::fold::FoldError| match why {
        crate::fold::FoldError::VolumeOverflow { bucket, .. } => ImportRefusal::VolumeOverflow {
            sod: second_of(bucket, day),
        },
        other => ImportRefusal::Fold {
            why: format!("{other:?}"),
        },
    };
    let seconds = fold(&rows, Bucket::SECOND).map_err(refused)?;
    Ok(Converted { seconds, placement })
}

/// The second of `day` a bucket starting at `ts_micros` names.
fn second_of(ts_micros: i64, day: Day) -> u32 {
    let secs = ts_micros.div_euclid(1_000_000) + IST_OFFSET_SECS
        - i64::from(day.days_from_epoch()) * 86_400;
    u32::try_from(secs).unwrap_or(0)
}

/// One instrument's file of one day, as a source hands it on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TickFile {
    /// The symbol the bars are filed under: `NIFTY`, `RELIANCE`, the option's
    /// underlying.
    pub symbol: String,
    /// The option contract, `None` for spot.
    pub contract: Option<Contract>,
    /// The vendor's file name, for every refusal and event about it.
    pub name: String,
    /// Every row, in file order.
    pub ticks: Vec<Tick>,
}

/// What a source did with one day.
#[derive(Debug, Default)]
pub struct DayRead {
    /// Whether the source holds the day at all.
    pub held: bool,
    /// Entries of the day that are not wanted files (off the surface, outside
    /// the filter, not a file of the day).
    pub skipped: usize,
    /// Files the source or the reader refused, named.
    pub refused: Vec<Failure>,
}

/// What one run did.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Report {
    /// Days that were imported this run (clean or not).
    pub days_imported: usize,
    /// Days the journal already records as done, skipped unread.
    pub days_skipped: usize,
    /// Days the calendar records as closed.
    pub days_closed: usize,
    /// Trading days the source does not hold, named.
    pub days_missing: Vec<Day>,
    /// Days refused before any file was read, named in `failures`.
    pub days_refused: usize,
    /// Days a crashed or incomplete earlier run had begun, imported again.
    pub resumed: Vec<Day>,
    /// Instrument files read and folded; a file the fold refuses is in
    /// `files_refused` instead, never in both (D-3168).
    pub files: usize,
    /// Entries not wanted.
    pub files_skipped: usize,
    /// Files refused, named in `failures`.
    pub files_refused: usize,
    /// Rows those files held.
    pub rows: usize,
    /// Rows dropped as recording no trade.
    pub ltq_zero_dropped: usize,
    /// Late rows deferred.
    pub late_rows: usize,
    /// Late rows dropped with nothing after them.
    pub late_unresolved: usize,
    /// The largest back-step of any kept row of any file, in seconds: how far
    /// the worst deferral moved a print (D-3172).
    pub max_back_s: u32,
    /// One-second bars offered to the store.
    pub seconds: usize,
    /// One-second bars the store wrote this run.
    pub seconds_committed: usize,
    /// Bars the venue's session declined (pre-open, post-close).
    pub outside_session: usize,
    /// Every failure, named.
    pub failures: Vec<Failure>,
}

/// What one run is asked to do.
#[derive(Debug, Clone, Copy)]
pub struct Run<'a> {
    /// Which tree.
    pub kind: ImportKind,
    /// The first day, inclusive.
    pub from: Day,
    /// The last day, inclusive.
    pub to: Day,
    /// Only these symbols (spot) or underlyings (options); empty is every one.
    pub only: &'a [String],
    /// The bar store's root.
    pub store_root: &'a Path,
}

impl Run<'_> {
    /// Refuses a filter name that is not a symbol as the store files it
    /// (D-3171): the journal key and the match are both by exact bytes.
    fn check_filter(&self) -> Result<(), ImportRefusal> {
        for name in self.only {
            let canonical =
                brutex_core::symbol::Symbol::new(name).is_ok_and(|symbol| symbol.as_str() == name);
            if !canonical {
                return Err(ImportRefusal::FilterName { name: name.clone() });
            }
        }
        Ok(())
    }

    /// ONE REQUEST FOR THE RUN: its window is the run's range, which is also
    /// the range check, its rung the second, its listing the kind's venue. A
    /// file, not a vendor request, is the source, so it carries no vendor id.
    /// Every venue table holds verified hours from the epoch (`crate::vendor`
    /// const-asserts their shipping shape), so the session filter in
    /// `from_rows` cannot meet a day with no hours.
    fn request(&self) -> Result<BarRequest, ImportRefusal> {
        let window =
            Window::new(self.from, self.to).map_err(|_| ImportRefusal::RangeBackwards {
                from: self.from,
                to: self.to,
            })?;
        Ok(BarRequest {
            instrument_id: String::new(),
            listing: self.kind.listing(),
            window,
            granularity: Granularity::Second1,
        })
    }

    /// Whether `symbol` passes the filter.
    fn wants(&self, symbol: &str) -> bool {
        self.only.is_empty() || self.only.iter().any(|only| only == symbol)
    }

    /// The journal key's filter part: the sorted names, or `*`.
    fn filter_key(&self) -> String {
        if self.only.is_empty() {
            return "*".to_owned();
        }
        // Ordered and deduplicated by the set (D-3178), bounded by the filter's own
        // length, never by the data.
        let names: std::collections::BTreeSet<&str> =
            self.only.iter().map(String::as_str).collect();
        names.into_iter().collect::<Vec<&str>>().join(",")
    }
}

/// The import journal's path under a store root.
#[must_use]
pub fn journal_path(store_root: &Path) -> PathBuf {
    store_root.join("imports").join("gdfl.journal")
}

/// What closes a torn journal line, so every later load knows it for one
/// (D-3173). A line ending in it is skipped; no line the journal writes
/// whole can end in it, because a key is a kind, a day and symbol names.
const TORN: &str = " (torn)";

/// Whether `fragment`, a line cut short by a crash, can be the start of a
/// line the journal writes: a prefix of one of its three verbs and the space
/// after it, or text that starts with one (D-3169). Anything else at the end
/// of the file is not a torn journal line but a foreign one.
fn torn_fragment(fragment: &str) -> bool {
    ["begin ", "done ", "incomplete "]
        .into_iter()
        .any(|verb| verb.starts_with(fragment) || fragment.starts_with(verb))
}

/// The journal, read once per run: the keys done, and the keys begun and not
/// finished.
#[derive(Debug, Default)]
struct Journal {
    path: PathBuf,
    done: HashSet<String>,
    open: HashSet<String>,
}

impl Journal {
    /// Reads the journal at `path`; a missing one is empty. A torn last line
    /// (no newline: a crash mid-write) is ignored and closed before the next
    /// write. Any other line that is not `begin <key>` or `done <key> ...`
    /// refuses the run.
    fn load(path: PathBuf) -> Result<Self, ImportRefusal> {
        let text = match std::fs::read_to_string(&path) {
            Ok(text) => text,
            Err(why) if why.kind() == std::io::ErrorKind::NotFound => String::new(),
            Err(why) => {
                return Err(ImportRefusal::Journal {
                    why: format!("{}: {why}", path.display()),
                });
            }
        };
        let mut journal = Self {
            path,
            ..Self::default()
        };
        let (whole, tail) = text.rsplit_once('\n').unwrap_or(("", text.as_str()));
        let shown = journal.path.display().to_string();
        let foreign = |line: &str| ImportRefusal::Journal {
            why: format!("{shown}: not a journal line: {line:?}"),
        };
        // Every whole line is judged BEFORE anything is written, and a torn
        // tail must be the start of a line the journal writes: a foreign file
        // is refused on every run and never touched. Closing first let a
        // one-line foreign file be refused once and then, its line now
        // "torn", read as an empty journal by every later run (D-3169).
        if !tail.is_empty() && !torn_fragment(tail) {
            return Err(foreign(tail));
        }
        for line in whole.split('\n').filter(|line| !line.is_empty()) {
            if let Some(fragment) = line.strip_suffix(TORN) {
                if torn_fragment(fragment) {
                    continue;
                }
                return Err(foreign(line));
            }
            let fields: Vec<&str> = line.split(' ').collect();
            match fields.as_slice() {
                ["begin", kind, day, only] => {
                    journal.open.insert(format!("{kind} {day} {only}"));
                }
                ["done", kind, day, only, ..] => {
                    let key = format!("{kind} {day} {only}");
                    journal.open.remove(&key);
                    journal.done.insert(key);
                }
                ["incomplete", kind, day, only, ..] => {
                    journal.open.remove(&format!("{kind} {day} {only}"));
                }
                _ => return Err(foreign(line)),
            }
        }
        if !tail.is_empty() {
            // Closed with a mark, never a bare newline: a bare one made the
            // torn fragment a whole line that the NEXT load refused as
            // foreign, so one crash mid-write stopped every later run
            // (D-3173).
            journal.append(TORN)?;
        }
        Ok(journal)
    }

    /// Appends `line` and a newline, durably.
    fn append(&self, line: &str) -> Result<(), ImportRefusal> {
        let refused = |why: std::io::Error| ImportRefusal::Journal {
            why: format!("{}: {why}", self.path.display()),
        };
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent).map_err(refused)?;
        }
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .map_err(refused)?;
        file.write_all(format!("{line}\n").as_bytes())
            .map_err(refused)?;
        file.sync_all().map_err(refused)
    }
}

/// Every day of `from..=to`.
fn days(from: Day, to: Day) -> Vec<Day> {
    let mut out = Vec::new();
    let mut day = from;
    while day.days_from_epoch() <= to.days_from_epoch() {
        out.push(day);
        match day.succ() {
            Ok(next) => day = next,
            Err(_) => break,
        }
    }
    out
}

/// Emits one event, discarding whether the sink kept it (a filtered level is
/// not a failure; see `crate::ingest`'s note on the same choice).
fn note(event: &telemetry::Event<'_>) {
    let _dropped_when_filtered = telemetry::emit(event);
}

/// A failure about `about`, logged at `Error` and returned for the report.
/// Named `note_*` so gate 19 sees the event beside every failure it makes
/// (D-3179).
fn note_failure(kind: ImportKind, day: Day, about: &str, why: &str) -> Failure {
    let day_text = day.to_string();
    note(
        &telemetry::Event::error(TARGET, "refused")
            .with("kind", kind.as_str())
            .with("day", day_text.as_str())
            .with("about", about)
            .with("why", why),
    );
    Failure {
        instrument: format!("{day} {about}"),
        why: why.to_owned(),
    }
}

/// The plan one rung of one file is filed under.
fn plan(kind: ImportKind, request: &BarRequest, contract: Option<Contract>) -> Plan<'_> {
    Plan {
        calendar: crate::calendar::Runtime::default(),
        cash_schedule: None,
        // Unused by `from_rows`, which takes bars already decoded; named for
        // the record. GDFL bytes never pass `csv::decode` (D-2802).
        columns: crate::csv::Columns::Gdfl,
        request,
        encoding: TimestampEncoding::EpochSecondsUtc,
        scale: PriceScale::Paisa,
        vendor: Vendor::Gdfl,
        exchange: "NSE",
        segment: kind.segment(),
        contract,
    }
}

/// The day's per-file work and the census rows it collects.
struct DayWork<'a> {
    run: &'a Run<'a>,
    day: Day,
    request: &'a BarRequest,
    journal: &'a Journal,
    key: String,
    begun: bool,
    held: Vec<Held>,
    report: &'a mut Report,
    failures: Vec<Failure>,
}

impl DayWork<'_> {
    /// Converts and files one instrument file. A file the fold refuses is
    /// counted as refused and only as refused, so every entry of a day is
    /// exactly one file, skip or refusal (D-3168).
    fn file(&mut self, file: &TickFile) {
        let converted = match convert(self.run.kind, self.day, &file.ticks) {
            Ok(converted) => converted,
            Err(why) => {
                self.report.files_refused += 1;
                let failure = note_failure(self.run.kind, self.day, &file.name, &why.to_string());
                self.failures.push(failure);
                return;
            }
        };
        self.report.files += 1;
        let p = converted.placement;
        self.report.rows += p.rows;
        self.report.ltq_zero_dropped += p.ltq_zero_dropped;
        self.report.late_rows += p.late_rows;
        self.report.late_unresolved += p.late_unresolved;
        self.report.max_back_s = self.report.max_back_s.max(p.max_back_s);
        if p.late_rows > 0 {
            note(
                &telemetry::Event::warn(TARGET, "late rows deferred to a later second")
                    .with("file", file.name.as_str())
                    .with("late_rows", p.late_rows)
                    .with("late_unresolved", p.late_unresolved)
                    .with("max_back_s", p.max_back_s),
            );
        }
        if converted.seconds.is_empty() {
            return;
        }
        if !self.begun {
            if let Err(why) = self.journal.append(&format!("begin {}", self.key)) {
                let failure = note_failure(self.run.kind, self.day, "journal", &why.to_string());
                self.failures.push(failure);
                return;
            }
            self.begun = true;
        }
        let origin = format!("gdfl {} {} {}", self.run.kind.as_str(), self.day, file.name);
        let done: Ingested = from_rows(
            &converted.seconds,
            &[],
            &file.symbol,
            &origin,
            self.run.store_root,
            plan(self.run.kind, self.request, file.contract),
        );
        self.report.outside_session += usize::try_from(done.census.total()).unwrap_or(0);
        self.report.seconds += done.bars_stored;
        self.report.seconds_committed += done.bars_committed;
        self.held.extend(done.pending);
        for failure in done.failures {
            let failure = note_failure(self.run.kind, self.day, &file.name, &failure.why);
            self.failures.push(failure);
        }
        note(
            &telemetry::Event::debug(TARGET, "filed")
                .with("file", file.name.as_str())
                .with("rows", p.rows)
                .with("seconds", converted.seconds.len()),
        );
    }
}

/// Whether the calendar lets `day` be imported: a closed day is counted, a
/// day it does not call a regular full session is a named failure.
fn calendar_admits(kind: ImportKind, day: Day, report: &mut Report) -> bool {
    match session_gate(day) {
        Ok(()) => true,
        Err(CmRefusal::CalendarNotRegular {
            kind: crate::calendar::DayKind::Closed,
        }) => {
            report.days_closed += 1;
            false
        }
        Err(why) => {
            report.days_refused += 1;
            let failure = note_failure(kind, day, "calendar", &why.to_string());
            report.failures.push(failure);
            false
        }
    }
}

/// A trading day the source does not hold: named, warned, its failures kept.
fn missing_day(kind: ImportKind, day: Day, report: &mut Report, failures: Vec<Failure>) {
    report.days_missing.push(day);
    let day_text = day.to_string();
    note(
        &telemetry::Event::warn(TARGET, "the source holds no file of this trading day")
            .with("kind", kind.as_str())
            .with("day", day_text.as_str()),
    );
    report.failures.extend(failures);
}

/// The `day imported` event: what the day added to the report.
fn note_day(kind: ImportKind, day: Day, report: &Report, before: &Report, failures: usize) {
    let day_text = day.to_string();
    note(
        &telemetry::Event::info(TARGET, "day imported")
            .with("kind", kind.as_str())
            .with("day", day_text.as_str())
            .with("files", report.files - before.files)
            .with("rows", report.rows - before.rows)
            .with("seconds", report.seconds - before.seconds)
            .with(
                "committed",
                report.seconds_committed - before.seconds_committed,
            )
            .with("failures", failures),
    );
}

/// The one runtime: every day of the run through the calendar, the venue,
/// the journal, the source's `read_day` and the common filing path.
///
/// # Errors
///
/// [`ImportRefusal::RangeBackwards`] and [`ImportRefusal::Journal`] for the
/// run; everything else is a named failure in the report.
pub fn drive<F>(run: &Run<'_>, mut read_day: F) -> Result<Report, ImportRefusal>
where
    F: FnMut(Day, &mut dyn FnMut(TickFile)) -> DayRead,
{
    let request = run.request()?;
    run.check_filter()?;
    let journal = Journal::load(journal_path(run.store_root))?;
    let filter = run.filter_key();
    let (from_text, to_text) = (run.from.to_string(), run.to.to_string());
    note(
        &telemetry::Event::info(TARGET, "run started")
            .with("kind", run.kind.as_str())
            .with("from", from_text.as_str())
            .with("to", to_text.as_str())
            .with("only", filter.as_str()),
    );
    let mut report = Report::default();
    for day in days(run.from, run.to) {
        let key = format!("{} {day} {filter}", run.kind.as_str());
        if journal.done.contains(key.as_str()) {
            report.days_skipped += 1;
            continue;
        }
        if !calendar_admits(run.kind, day, &mut report) {
            continue;
        }
        if journal.open.contains(key.as_str()) {
            report.resumed.push(day);
            let day_text = day.to_string();
            note(
                &telemetry::Event::warn(
                    TARGET,
                    "resuming a day an earlier run began and did not finish",
                )
                .with("kind", run.kind.as_str())
                .with("day", day_text.as_str()),
            );
        }
        let before = report.clone();
        // The day's own largest back-step, for its journal line: the run's
        // maximum is reset for the day and restored after it.
        report.max_back_s = 0;
        let mut work = DayWork {
            run,
            day,
            request: &request,
            journal: &journal,
            key: key.clone(),
            begun: false,
            held: Vec::new(),
            report: &mut report,
            failures: Vec::new(),
        };
        let read = read_day(day, &mut |file| work.file(&file));
        let (begun, held, mut failures) = (work.begun, work.held, work.failures);
        let day_max_back_s = report.max_back_s;
        report.max_back_s = report.max_back_s.max(before.max_back_s);
        report.files_skipped += read.skipped;
        report.files_refused += read.refused.len();
        for refusal in read.refused {
            failures.push(note_failure(
                run.kind,
                day,
                &refusal.instrument,
                &refusal.why,
            ));
        }
        if !read.held {
            missing_day(run.kind, day, &mut report, failures);
            continue;
        }
        report.days_imported += 1;
        if let Some(why) = record_held(run.store_root, Vendor::Gdfl, &held) {
            failures.push(note_failure(run.kind, day, "census", &why));
        }
        let clean = failures.is_empty();
        if begun || clean {
            let stats = format!(
                "files={} seconds={} failures={} late={} late_unresolved={} max_back_s={}",
                report.files - before.files,
                report.seconds - before.seconds,
                failures.len(),
                report.late_rows - before.late_rows,
                report.late_unresolved - before.late_unresolved,
                day_max_back_s
            );
            let verdict = if clean { "done" } else { "incomplete" };
            if let Err(why) = journal.append(&format!("{verdict} {key} {stats}")) {
                failures.push(note_failure(run.kind, day, "journal", &why.to_string()));
            }
        }
        note_day(run.kind, day, &report, &before, failures.len());
        report.failures.extend(failures);
    }
    note(
        &telemetry::Event::info(TARGET, "run finished")
            .with("kind", run.kind.as_str())
            .with("days", report.days_imported)
            .with("skipped", report.days_skipped)
            .with("missing", report.days_missing.len())
            .with("files", report.files)
            .with("seconds", report.seconds)
            .with("failures", report.failures.len()),
    );
    Ok(report)
}

/// The run over a capital-market source (indices or stocks).
///
/// # Errors
///
/// As [`drive`].
pub fn run_cm<S: CmSource>(source: &S, run: &Run<'_>) -> Result<Report, ImportRefusal> {
    let tree = match run.kind {
        ImportKind::Indices => CmKind::Indices,
        ImportKind::Stocks => CmKind::Stocks,
        ImportKind::Options => {
            return Err(ImportRefusal::KindUnknown {
                word: "options read from a capital-market source".to_owned(),
            });
        }
    };
    drive(run, |day, sink| cm_day(source, run, tree, day, sink))
}

/// One capital-market day: every swept ticker's file, once, in listing order.
fn cm_day<S: CmSource>(
    source: &S,
    run: &Run<'_>,
    tree: CmKind,
    day: Day,
    sink: &mut dyn FnMut(TickFile),
) -> DayRead {
    let mut read = DayRead::default();
    let listing = match source.day(tree, day) {
        Ok(Some(listing)) => listing,
        Ok(None) => return read,
        Err(why) => {
            read.held = true;
            read.refused.push(Failure {
                instrument: "listing".to_owned(),
                why: why.to_string(),
            });
            return read;
        }
    };
    read.held = true;
    let mut seen = HashSet::with_capacity(listing.entries().len());
    let folder = format!("{}/", listing.folder());
    for file in listing.entries() {
        let key = file
            .entry
            .strip_prefix(&folder)
            .and_then(|name| split_name(name).ok())
            .and_then(|(stem, _)| match resolve(tree, stem) {
                Resolution::Swept(key) => Some(key),
                _ => None,
            });
        let Some(key) = key else {
            read.skipped += 1;
            continue;
        };
        if !seen.insert(key) {
            // A second name for a ticker already offered (its refusal, if
            // any, is named once): counted, so every entry of the day is
            // a file, a skip or a refusal (D-3174).
            read.skipped += 1;
            continue;
        }
        let symbol = key.underlying.as_str().to_owned();
        if !run.wants(&symbol) {
            read.skipped += 1;
            continue;
        }
        match read_listed(source, &listing, &key) {
            Ok(Some(day_file)) => sink(TickFile {
                symbol,
                contract: None,
                name: day_file.name,
                ticks: day_file
                    .file
                    .rows
                    .iter()
                    .map(|row| Tick {
                        sod: row.sod,
                        ltp: row.ltp,
                        ltq: row.ltq,
                        oi: None,
                    })
                    .collect(),
            }),
            Ok(None) => read.skipped += 1,
            Err(why) => read.refused.push(Failure {
                instrument: file.name().to_owned(),
                why: why.to_string(),
            }),
        }
    }
    read
}

/// The run over an options source.
///
/// # Errors
///
/// As [`drive`], and [`ImportRefusal::KindUnknown`] for a spot kind.
pub fn run_nfo<S: NfoSource>(source: &S, run: &Run<'_>) -> Result<Report, ImportRefusal> {
    if run.kind != ImportKind::Options {
        return Err(ImportRefusal::KindUnknown {
            word: format!("{} read from an options source", run.kind.as_str()),
        });
    }
    drive(run, |day, sink| nfo_day(source, run, day, sink))
}

/// The longest name of `names` that `ticker` starts with.
fn longest_prefix<'a>(ticker: &str, names: impl IntoIterator<Item = &'a str>) -> Option<&'a str> {
    let mut best: Option<&'a str> = None;
    for name in names {
        if ticker.starts_with(name) && best.is_none_or(|b| name.len() > b.len()) {
            best = Some(name);
        }
    }
    best
}

/// Whether a filtered run wants a name that does not decode: when the
/// longest underlying it starts with, among the F&O underlyings AND the
/// filter's own names, is a filter name. `NIFTYNXT50…` is `NIFTYNXT50`'s,
/// never `NIFTY`'s (D-3175); `TV18BRDCST…`, a share outside today's F&O
/// list, is a `TV18BRDCST` filter's, which asked for it by name (D-3167).
/// O(213 + filter names) per undecodable name, on the refusal path only.
fn claims_undecodable(run: &Run<'_>, ticker: &str) -> bool {
    let fno = longest_prefix(ticker, brutex_core::universe::FNO_UNDERLYINGS);
    let asked = longest_prefix(ticker, run.only.iter().map(String::as_str));
    match (asked, fno) {
        (Some(asked), Some(fno)) => asked.len() >= fno.len(),
        (Some(_), None) => true,
        (None, _) => false,
    }
}

/// One options day: every option file whose underlying passes the filter.
fn nfo_day<S: NfoSource>(
    source: &S,
    run: &Run<'_>,
    day: Day,
    sink: &mut dyn FnMut(TickFile),
) -> DayRead {
    let mut read = DayRead::default();
    let listing = match source.day(day) {
        Ok(Some(listing)) => listing,
        Ok(None) => return read,
        Err(why) => {
            read.held = true;
            read.refused.push(Failure {
                instrument: "listing".to_owned(),
                why: why.to_string(),
            });
            return read;
        }
    };
    read.held = true;
    // Two tickers of the day naming one contract would file into one path:
    // counted first, and every one of them refused by name.
    let mut named: HashMap<(String, Contract), usize> =
        HashMap::with_capacity(listing.entries().len());
    for file in listing.entries() {
        if let Some(Ok(decoded)) = listing.ticker_of(file).map(|t| decode_ticker(t, day)) {
            *named
                .entry((decoded.underlying.as_str().to_owned(), decoded.contract))
                .or_default() += 1;
        }
    }
    for file in listing.entries() {
        let Some(ticker) = listing.ticker_of(file) else {
            read.skipped += 1;
            continue;
        };
        let wanted = match decode_ticker(ticker, day) {
            Ok(decoded)
                if run.wants(decoded.underlying.as_str())
                    && named.get(&(decoded.underlying.as_str().to_owned(), decoded.contract))
                        > Some(&1) =>
            {
                read.refused.push(Failure {
                    instrument: ticker.to_owned(),
                    why: NfoRefusal::TickerAmbiguous {
                        ticker: ticker.to_owned(),
                    }
                    .to_string(),
                });
                continue;
            }
            Ok(decoded) => run.wants(decoded.underlying.as_str()),
            // A name that does not decode is refused when it could be wanted:
            // by its WHOLE underlying, never by a filter name a longer F&O
            // underlying outspells (D-3175, D-3167).
            Err(_) => run.only.is_empty() || claims_undecodable(run, ticker),
        };
        if !wanted {
            read.skipped += 1;
            continue;
        }
        match read_file(source, &listing, file) {
            Ok(day_file) => sink(TickFile {
                symbol: day_file.ticker.underlying.as_str().to_owned(),
                contract: Some(day_file.ticker.contract),
                name: day_file.name,
                ticks: day_file
                    .file
                    .rows
                    .iter()
                    .map(|row| Tick {
                        sod: row.sod,
                        ltp: row.ltp,
                        ltq: row.ltq,
                        oi: Some(row.oi),
                    })
                    .collect(),
            }),
            Err(why) => read.refused.push(Failure {
                instrument: ticker.to_owned(),
                why: why.to_string(),
            }),
        }
    }
    read
}

#[cfg(test)]
#[path = "gdfl_import_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "gdfl_seconds_attack_tests.rs"]
mod attack_gdfl_seconds;

#[cfg(test)]
#[path = "gdfl_r2_attack_tests.rs"]
mod r2_attack_tests;
