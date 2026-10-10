//! The series one `/bars/window.json` request reads, the months each one
//! holds, and the rows its day and time-of-day bounds keep. D-3326.
//!
//! # What this replaces
//!
//! The window opened every month of its range for one series — a file open,
//! a lock, a header and its checksum, about 30 µs each — whether or not the
//! page needed a row from it, and an absent month wrote one `read refused`
//! warning per request (DB-065, DB-023). A window over several series did not
//! exist: `/db` asked each contract separately with the same offset and
//! merged the pages it held, so page 2 of a sorted chain was the wrong rows
//! and a CE+PE pair was stacked rather than merged by time (DB-002, DB-003,
//! DB-G82, DB-G92, DB-G93). A time-of-day bound read only the newest month
//! (DB-004, DB-G91).
//!
//! # What a series costs now
//!
//! * **Which months it holds**: one `stat` of its rung directory, and a
//!   directory listing only when that directory changed since the last
//!   request ([`held_months`]). An absent month is COUNTED from the listing,
//!   never opened and never logged; the months before the series' first held
//!   month or after its last are counted apart as outside its life.
//! * **How many rows each held month has**: one `stat` of the month file, and
//!   its header read only when the file changed since the last request
//!   ([`header_of`]). So the total, the prefix sums and the month a position
//!   falls in cost `O(months)` `stat`s and no file opens.
//! * **Opening**: a month file is opened only when a row of it is read, or a
//!   time is located in it.
//! * **Filters**: a day bound locates its two edges, in the two edge months
//!   only. A time-of-day bound locates each day's window edges, two `.tix`
//!   lookups per calendar day of the range: `O(days)`, cached per month file
//!   and window ([`RUNS`]).
//! * **Rank** — how many kept rows lie strictly before an instant — is one
//!   bucket lookup (the instant's month, or its day under a time-of-day bound)
//!   and at most one `first_at_or_after`: O(1) with the month's `.tix`.
//!
//! Every cache here is bounded and is keyed by the file's own stamp (length,
//! modification time and inode), so a file the import appends to, or a
//! generation installed beside it, is read again rather than served stale.

use std::collections::{BTreeSet, HashMap};
use std::hash::Hash;
use std::os::unix::fs::MetadataExt as _;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock, PoisonError};

use brutex_core::instrument::Contract;
use brutex_core::vendor::Vendor;
use store::file::BarFile;
use store::format::Bar;
use store::path::{FileKind, PathParts, StorePath, Timeframe, YearMonth};

use super::{OpenRefusalKind, open_classified};

/// Microseconds in one second.
pub(super) const MICROS: i64 = 1_000_000;

/// Seconds in one day.
const DAY_SECS: i64 = 86_400;

/// The most series one window request may read. More is refused
/// `too_many_series` (D-3326): one request per contract of the widest NIFTY
/// expiry measured is 566, and a cap is what keeps one request's work bounded.
pub const MAX_SERIES: usize = 1_024;

/// Entries each process-wide memo holds before it is emptied and refilled.
///
/// Emptied whole rather than evicted one at a time: an eviction order is a
/// second structure to keep, and a memo that forgets everything costs one
/// re-read per file on the next request, which is what it cost before it
/// existed.
const MEMO_CAP: usize = 65_536;

/// One series, as the census names it: `NSE-FNO-NIFTY-2019-03-28-1150000-CE`,
/// `NSE-INDEX-NIFTY`, `NSE-CASH-BAJAJ-AUTO`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SeriesKey {
    /// The venue directory, `NSE`.
    pub exchange: String,
    /// The segment directory, `INDEX`, `CASH` or `FNO`.
    pub segment: String,
    /// The symbol directory — for a contract, its underlying.
    pub symbol: String,
    /// The contract directory, for a future or an option.
    pub contract: Option<Contract>,
}

impl SeriesKey {
    /// A key from its four parts.
    #[must_use]
    pub fn new(exchange: &str, segment: &str, symbol: &str, contract: Option<Contract>) -> Self {
        Self {
            exchange: exchange.to_owned(),
            segment: segment.to_owned(),
            symbol: symbol.to_owned(),
            contract,
        }
    }

    /// The census's text for this key, which is what [`Self::parse`] reads.
    #[must_use]
    pub fn text(&self) -> String {
        self.contract.map_or_else(
            || format!("{}-{}-{}", self.exchange, self.segment, self.symbol),
            |contract| {
                format!(
                    "{}-{}-{}-{}",
                    self.exchange, self.segment, self.symbol, contract
                )
            },
        )
    }

    /// The key a census instrument text names, or `None`.
    ///
    /// The contract is recognised by its shape at the END of the text —
    /// `YYYY-MM-DD-<digits>-CE|PE` or `YYYY-MM-DD-FUT` — so an underlying that
    /// holds a dash (`BAJAJ-AUTO`) reads back whole, and a symbol with no
    /// contract (`ABB-III`) is not mistaken for one.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        let (exchange, rest) = text.split_once('-')?;
        let (segment, rest) = rest.split_once('-')?;
        if exchange.is_empty() || segment.is_empty() || rest.is_empty() {
            return None;
        }
        let (symbol, contract) = split_contract(rest);
        if symbol.is_empty() {
            return None;
        }
        Some(Self::new(exchange, segment, symbol, contract))
    }

    /// The store path parts of this key in `month`.
    fn parts(&self, vendor: Vendor, timeframe: Timeframe, month: YearMonth) -> PathParts<'_> {
        PathParts {
            vendor,
            exchange: &self.exchange,
            segment: &self.segment,
            symbol: &self.symbol,
            contract: self.contract,
            timeframe,
            month,
            file: FileKind::Bars,
        }
    }
}

/// `rest` split into its symbol and, when its tail has a contract's shape,
/// the contract.
fn split_contract(rest: &str) -> (&str, Option<Contract>) {
    let fields: Vec<&str> = rest.split('-').collect();
    for width in [5_usize, 4] {
        let Some(cut) = fields.len().checked_sub(width).filter(|&cut| cut > 0) else {
            continue;
        };
        if !contract_shaped(fields.get(cut..).unwrap_or(&[])) {
            continue;
        }
        let at: usize = fields
            .get(..cut)
            .map_or(0, |head| head.iter().map(|field| field.len() + 1).sum());
        let symbol = rest.get(..at.saturating_sub(1)).unwrap_or("");
        if let Some(contract) = rest.get(at..).and_then(Contract::parse) {
            return (symbol, Some(contract));
        }
    }
    (rest, None)
}

/// Whether `tail` is an option's or a future's contract fields.
fn contract_shaped(tail: &[&str]) -> bool {
    let digits = |text: &str, width: Option<usize>| {
        !text.is_empty()
            && text.bytes().all(|b| b.is_ascii_digit())
            && width.is_none_or(|width| text.len() == width)
    };
    let date =
        |y: &str, m: &str, d: &str| digits(y, Some(4)) && digits(m, Some(2)) && digits(d, Some(2));
    match tail {
        [y, m, d, strike, side] => {
            date(y, m, d) && digits(strike, None) && matches!(*side, "CE" | "PE")
        }
        [y, m, d, fut] => date(y, m, d) && *fut == "FUT",
        _ => false,
    }
}

/// What a file looked like when it was last read: length, modification time
/// and inode. A change in any of them reads the file again.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) struct Stamp {
    len: u64,
    modified_secs: i64,
    modified_nanos: i64,
    inode: u64,
}

impl Stamp {
    fn of(meta: &std::fs::Metadata) -> Self {
        Self {
            len: meta.len(),
            modified_secs: meta.mtime(),
            modified_nanos: meta.mtime_nsec(),
            inode: meta.ino(),
        }
    }
}

/// A bounded process-wide memo. See [`MEMO_CAP`].
pub(super) struct Memo<K, V> {
    map: OnceLock<Mutex<HashMap<K, V>>>,
}

impl<K: Eq + Hash, V: Clone> Memo<K, V> {
    /// An empty memo.
    pub(super) const fn new() -> Self {
        Self {
            map: OnceLock::new(),
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<K, V>> {
        self.map
            .get_or_init(|| Mutex::new(HashMap::with_capacity(1_024)))
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }

    /// The value held for `key`, cloned.
    pub(super) fn get(&self, key: &K) -> Option<V> {
        self.lock().get(key).cloned()
    }

    /// Holds `value` for `key`, emptying the memo first when it is full.
    pub(super) fn put(&self, key: K, value: V) {
        let mut map = self.lock();
        if map.len() >= MEMO_CAP {
            map.clear();
        }
        map.insert(key, value);
    }
}

/// Every rung directory's held months, by the directory's stamp.
static DIRS: Memo<PathBuf, (Stamp, Arc<[YearMonth]>)> = Memo::new();

/// Every month file's header facts, by the file's stamp.
static HEADERS: Memo<PathBuf, (Stamp, Held)> = Memo::new();

/// One month file's time-of-day runs are keyed by the file, its stamp, the
/// clamped day window and the time-of-day window.
type RunsKey = (PathBuf, Stamp, i64, i64, u32, u32);

/// Every month file's time-of-day runs, by file stamp and window.
static RUNS: Memo<RunsKey, Arc<[Run]>> = Memo::new();

/// The months a rung directory holds a bar file for, oldest first.
///
/// One `stat` of the directory; the listing is read again only when the
/// directory's stamp changed, which is what adding a month does. A directory
/// that does not exist holds nothing — every month of a symbol that was
/// never stored is an absence, not a refusal.
///
/// # Errors
///
/// The directory exists and cannot be listed.
pub(super) fn held_months(dir: &Path) -> Result<Arc<[YearMonth]>, String> {
    let meta = match std::fs::metadata(dir) {
        Ok(meta) => meta,
        Err(why) if why.kind() == std::io::ErrorKind::NotFound => return Ok(Arc::from([])),
        Err(why) => return Err(format!("cannot read {}: {why}", dir.display())),
    };
    let stamp = Stamp::of(&meta);
    if let Some((at, held)) = DIRS.get(&dir.to_path_buf())
        && at == stamp
    {
        return Ok(held);
    }
    let entries =
        std::fs::read_dir(dir).map_err(|why| format!("cannot list {}: {why}", dir.display()))?;
    let mut months = BTreeSet::new();
    for entry in entries {
        let entry = entry.map_err(|why| format!("cannot list {}: {why}", dir.display()))?;
        let name = entry.file_name();
        if let Some(month) = name
            .to_str()
            .and_then(|name| name.strip_suffix(".bin"))
            .and_then(month_of_text)
        {
            months.insert(month);
        }
    }
    let held: Arc<[YearMonth]> = months.into_iter().collect::<Vec<_>>().into();
    DIRS.put(dir.to_path_buf(), (stamp, Arc::clone(&held)));
    Ok(held)
}

/// `YYYY-MM` as a month, or `None`.
pub(super) fn month_of_text(text: &str) -> Option<YearMonth> {
    let (year, month) = text.split_once('-')?;
    if year.len() != 4 || month.len() != 2 {
        return None;
    }
    YearMonth::new(year.parse().ok()?, month.parse().ok()?).ok()
}

/// What a month file's header says, kept between requests.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Held {
    /// Committed records.
    pub(super) n_valid: u64,
    /// The first committed record's stamp.
    pub(super) first_ts: i64,
    /// The last committed record's stamp.
    pub(super) last_ts: i64,
    /// The header's generation.
    pub(super) generation: u64,
}

impl Held {
    fn of(file: &BarFile) -> Self {
        let header = file.header();
        Self {
            n_valid: header.n_valid,
            first_ts: header.first_ts_micros,
            last_ts: header.last_ts_micros,
            generation: header.generation,
        }
    }
}

/// The header facts of one month file, from the memo when the file has not
/// changed, else by opening it — in which case the opened file is returned
/// too, so the caller does not open it twice.
///
/// `Ok(None)` when the file is absent (it went between the listing and the
/// `stat`).
fn header_of(
    root: &Path,
    parts: PathParts<'_>,
    path: &Path,
) -> Result<Option<(Held, Stamp, Option<BarFile>)>, String> {
    let meta = match std::fs::metadata(path) {
        Ok(meta) => meta,
        Err(why) if why.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(why) => return Err(format!("cannot read {}: {why}", path.display())),
    };
    let stamp = Stamp::of(&meta);
    if let Some((at, held)) = HEADERS.get(&path.to_path_buf())
        && at == stamp
    {
        return Ok(Some((held, stamp, None)));
    }
    match open_classified(root, parts) {
        Ok(file) => {
            let held = Held::of(&file);
            HEADERS.put(path.to_path_buf(), (stamp, held));
            Ok(Some((held, stamp, Some(file))))
        }
        Err(why) => match why.kind {
            OpenRefusalKind::Absent => Ok(None),
            OpenRefusalKind::Invalid | OpenRefusalKind::Unreadable => Err(why.message),
        },
    }
}

/// The day and time-of-day bounds a request applies to every series.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Filter {
    /// Inclusive start of the first day asked for, UTC micros.
    pub from: Option<i64>,
    /// Exclusive end of the last day asked for, UTC micros.
    pub to: Option<i64>,
    /// Inclusive IST seconds of the day, `from..=to`.
    pub tod: Option<(u32, u32)>,
}

/// A run of consecutive rows of one month file that the filters keep, and
/// the time window `[lo, hi)` they lie in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Run {
    /// Index into [`SeriesPlan::slots`].
    pub(super) slot: usize,
    /// First kept row.
    pub(super) start: u64,
    /// One past the last kept row.
    pub(super) end: u64,
    /// Every kept row is at or after this instant.
    pub(super) lo: i64,
    /// Every kept row is before this instant.
    pub(super) hi: i64,
    /// The month (or, under a time-of-day bound, the day) the run lies in,
    /// counted from the request's first.
    pub(super) bucket: usize,
}

impl Run {
    const fn len(&self) -> u64 {
        self.end.saturating_sub(self.start)
    }
}

/// One held month of a series within the request's range.
pub(super) struct Slot {
    /// The month.
    pub(super) month: YearMonth,
    /// Its bar file.
    pub(super) path: PathBuf,
    /// Its header facts.
    pub(super) held: Held,
    /// Its stamp.
    pub(super) stamp: Stamp,
    /// The open file, once a row was read or a time located in it.
    file: Option<BarFile>,
    /// Why it would not open, once it would not.
    failed: Option<String>,
}

/// A position among a series' kept rows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Position {
    /// Kept rows before it.
    pub(super) count: u64,
    /// The run it is in, or `runs.len()` past the last.
    pub(super) run: usize,
    /// The file row it is at.
    pub(super) row: u64,
}

/// How a series' buckets are cut: by month, or by IST day.
#[derive(Debug, Clone, Copy)]
enum Buckets {
    /// Months from `first`.
    Months { first: YearMonth, count: usize },
    /// IST days (days since the epoch) from `first`.
    Days { first: i64, count: usize },
}

/// Everything one request knows about one series.
pub(super) struct SeriesPlan {
    /// The series.
    pub(super) key: SeriesKey,
    vendor: Vendor,
    timeframe: Timeframe,
    /// Every month the series holds, inside the range or not.
    pub(super) held_all: Arc<[YearMonth]>,
    /// The held months inside the range, oldest first.
    pub(super) slots: Vec<Slot>,
    /// The kept rows, in time order.
    pub(super) runs: Vec<Run>,
    /// `prefix[i]` is the kept rows in `runs[..i]`.
    prefix: Vec<u64>,
    /// `first_run[b]` is the first run whose bucket is at or after `b`.
    first_run: Vec<usize>,
    buckets: Buckets,
    /// Months of the range with no file, between the series' first and last
    /// held month.
    pub(super) holes: Vec<YearMonth>,
    /// Months of the range with no file, before the series' first held month
    /// or after its last: an expected absence, counted apart.
    pub(super) outside_life: usize,
    /// Months that are held and would not read, named.
    pub(super) faults: Vec<String>,
}

impl SeriesPlan {
    /// Plans one series over `months` under `filter`.
    ///
    /// # Errors
    ///
    /// The address is invalid, the rung directory cannot be listed, or a
    /// month a filter needs to locate a time in cannot be addressed.
    pub(super) fn build(
        root: &Path,
        vendor: Vendor,
        timeframe: Timeframe,
        key: SeriesKey,
        months: &[YearMonth],
        filter: Filter,
    ) -> Result<Self, String> {
        let (Some(&first), Some(&last)) = (months.first(), months.last()) else {
            return Err("a window names at least one month".to_owned());
        };
        let address = StorePath::new(key.parts(vendor, timeframe, first))
            .map_err(|why| format!("{} {first}: {why}", key.symbol))?;
        let path = address.to_path_buf(root);
        let dir = path
            .parent()
            .map_or_else(|| root.to_path_buf(), Path::to_path_buf);
        let held_all = held_months(&dir)?;
        let mut plan = Self {
            key,
            vendor,
            timeframe,
            held_all: Arc::clone(&held_all),
            slots: Vec::with_capacity(months.len().min(held_all.len())),
            runs: Vec::new(),
            prefix: Vec::new(),
            first_run: Vec::new(),
            buckets: Buckets::Months {
                first,
                count: months.len(),
            },
            holes: Vec::new(),
            outside_life: 0,
            faults: Vec::new(),
        };
        let life = held_all.first().copied().zip(held_all.last().copied());
        let mut held = held_all.iter().copied().peekable();
        for &month in months {
            while held.next_if(|&at| at < month).is_some() {}
            if held.next_if_eq(&month).is_none() {
                if life.is_some_and(|(born, died)| born < month && month < died) {
                    plan.holes.push(month);
                } else {
                    plan.outside_life = plan.outside_life.saturating_add(1);
                }
                continue;
            }
            plan.admit(root, month);
        }
        if let Some((from, to)) = filter.tod {
            let first_day = day_of(filter.from.map_or_else(
                || first.ist_bounds_micros().0,
                |at| at.max(first.ist_bounds_micros().0),
            ));
            let last_day = day_of(
                filter
                    .to
                    .map_or_else(
                        || last.ist_bounds_micros().1,
                        |at| at.min(last.ist_bounds_micros().1),
                    )
                    .saturating_sub(1),
            );
            plan.buckets = Buckets::Days {
                first: first_day,
                count: usize::try_from(last_day.saturating_sub(first_day).saturating_add(1))
                    .unwrap_or(0),
            };
            plan.cut_days(root, filter, (from, to))?;
        } else {
            plan.cut_months(root, filter)?;
        }
        plan.index();
        Ok(plan)
    }

    /// Adds `month`'s file as a slot, or names why it would not read.
    fn admit(&mut self, root: &Path, month: YearMonth) {
        let parts = self.key.parts(self.vendor, self.timeframe, month);
        let Ok(address) = StorePath::new(parts) else {
            return;
        };
        // THE FILE EVERY READER OPENS, which a repair makes a revision's
        // (D-3350): the header and runs memos are keyed by it, so a promotion
        // is a new key and never a stale hit on the original's stamp.
        let path = match store::generation::served_bars_path(root, address) {
            Ok(path) => path,
            Err(why) => {
                self.faults.push(format!("{month}: {why}"));
                return;
            }
        };
        match header_of(root, parts, &path) {
            Ok(Some((held, stamp, file))) => self.slots.push(Slot {
                month,
                path,
                held,
                stamp,
                file,
                failed: None,
            }),
            // GONE BETWEEN THE LISTING AND THE STAT: an absence, counted.
            Ok(None) => self.outside_life = self.outside_life.saturating_add(1),
            Err(why) => self.faults.push(format!("{month}: {why}")),
        }
    }

    /// Runs without a time-of-day bound: one per held month, trimmed at the
    /// day bounds in the edge months.
    fn cut_months(&mut self, root: &Path, filter: Filter) -> Result<(), String> {
        let first = match self.buckets {
            Buckets::Months { first, .. } => first,
            Buckets::Days { .. } => return Ok(()),
        };
        for slot in 0..self.slots.len() {
            let Some(at) = self.slots.get(slot) else {
                continue;
            };
            let (month, held) = (at.month, at.held);
            let (month_lo, month_hi) = month.ist_bounds_micros();
            let lo = filter.from.map_or(month_lo, |from| from.max(month_lo));
            let hi = filter.to.map_or(month_hi, |to| to.min(month_hi));
            if lo >= hi || held.n_valid == 0 {
                continue;
            }
            // THE EDGES ARE LOCATED ONLY WHERE A BOUND CUTS THE MONTH'S
            // RECORDS. A bound outside the records needs no lookup, so an
            // interior month is never opened to be counted.
            let start = if lo > held.first_ts {
                self.locate(root, slot, lo)?
            } else {
                0
            };
            let end = if hi <= held.last_ts {
                self.locate(root, slot, hi)?
            } else {
                held.n_valid
            };
            if end > start {
                self.runs.push(Run {
                    slot,
                    start,
                    end,
                    lo,
                    hi,
                    bucket: months_between(first, month),
                });
            }
        }
        Ok(())
    }

    /// Runs under a time-of-day bound: one per IST day whose window holds a
    /// row, two lookups per calendar day, cached per month file and window.
    fn cut_days(&mut self, root: &Path, filter: Filter, tod: (u32, u32)) -> Result<(), String> {
        let Buckets::Days { first, .. } = self.buckets else {
            return Ok(());
        };
        for slot in 0..self.slots.len() {
            let Some(at) = self.slots.get(slot) else {
                continue;
            };
            let (month, held, path, stamp) = (at.month, at.held, at.path.clone(), at.stamp);
            if held.n_valid == 0 {
                continue;
            }
            let (month_lo, month_hi) = month.ist_bounds_micros();
            let lo = filter.from.map_or(month_lo, |from| from.max(month_lo));
            let hi = filter.to.map_or(month_hi, |to| to.min(month_hi));
            if lo >= hi {
                continue;
            }
            let memo = (path, stamp, lo, hi, tod.0, tod.1);
            let runs = if let Some(runs) = RUNS.get(&memo) {
                runs
            } else {
                let runs: Arc<[Run]> = self.day_runs(root, slot, (lo, hi), held, tod)?.into();
                RUNS.put(memo, Arc::clone(&runs));
                runs
            };
            for run in runs.iter() {
                let bucket = usize::try_from(day_of(run.lo).saturating_sub(first)).unwrap_or(0);
                self.runs.push(Run {
                    slot,
                    bucket,
                    ..*run
                });
            }
        }
        Ok(())
    }

    /// The day runs of one month: each IST day in `[lo, hi)` and inside the
    /// file's records, its `[day + from, day + to + 1 s)` window located.
    fn day_runs(
        &mut self,
        root: &Path,
        slot: usize,
        (lo, hi): (i64, i64),
        held: Held,
        (from, to): (u32, u32),
    ) -> Result<Vec<Run>, String> {
        let mut out = Vec::with_capacity(31);
        let mut day = day_of(lo.max(held.first_ts));
        let last = day_of(hi.saturating_sub(1).min(held.last_ts));
        while day <= last {
            let start_of_day = day_start(day);
            let window_lo = (start_of_day + i64::from(from) * MICROS).max(lo);
            let window_hi = (start_of_day + (i64::from(to) + 1) * MICROS).min(hi);
            if window_lo < window_hi {
                let start = self.locate(root, slot, window_lo)?;
                let end = self.locate(root, slot, window_hi)?;
                if end > start {
                    out.push(Run {
                        slot,
                        start,
                        end,
                        lo: window_lo,
                        hi: window_hi,
                        bucket: 0,
                    });
                }
            }
            day += 1;
        }
        Ok(out)
    }

    /// The prefix sums and the bucket table, once the runs are cut.
    fn index(&mut self) {
        self.prefix = Vec::with_capacity(self.runs.len() + 1);
        let mut sum = 0_u64;
        self.prefix.push(0);
        for run in &self.runs {
            sum = sum.saturating_add(run.len());
            self.prefix.push(sum);
        }
        let count = match self.buckets {
            Buckets::Months { count, .. } | Buckets::Days { count, .. } => count,
        };
        self.first_run = Vec::with_capacity(count + 1);
        let mut next = 0_usize;
        for bucket in 0..=count {
            while self.runs.get(next).is_some_and(|run| run.bucket < bucket) {
                next += 1;
            }
            self.first_run.push(next);
        }
    }

    /// Kept rows in total.
    pub(super) fn total(&self) -> u64 {
        self.prefix.last().copied().unwrap_or(0)
    }

    /// The bucket `ts` falls in, counted from the first; negative before it.
    fn bucket_of(&self, ts: i64) -> i64 {
        match self.buckets {
            Buckets::Months { first, .. } => {
                let day = day_of(ts);
                u32::try_from(day)
                    .ok()
                    .and_then(|day| pull::session::Day::from_days(day).ok())
                    .map_or(-1, |day| {
                        (i64::from(day.year()) * 12 + i64::from(day.month()))
                            - (i64::from(first.year()) * 12 + i64::from(first.month()))
                    })
            }
            Buckets::Days { first, .. } => day_of(ts) - first,
        }
    }

    /// How many kept rows lie strictly before `ts`, and where the first kept
    /// row at or after it is.
    ///
    /// One bucket lookup and at most one `first_at_or_after`.
    ///
    /// # Errors
    ///
    /// The month the instant falls in could not be opened or located in.
    pub(super) fn rank(&mut self, root: &Path, ts: i64) -> Result<Position, String> {
        let buckets = self.first_run.len().saturating_sub(1);
        let bucket = self.bucket_of(ts);
        let Ok(bucket) = usize::try_from(bucket) else {
            return Ok(self.at_run(0, 0));
        };
        if bucket >= buckets {
            return Ok(self.at_run(self.runs.len(), self.total()));
        }
        let run_at = self
            .first_run
            .get(bucket)
            .copied()
            .unwrap_or(self.runs.len());
        let before = self.prefix.get(run_at).copied().unwrap_or(0);
        let Some(run) = self.runs.get(run_at).copied() else {
            return Ok(self.at_run(run_at, before));
        };
        if run.bucket != bucket || ts <= run.lo {
            return Ok(self.at_run(run_at, before));
        }
        // ONE BUCKET MAY HOLD SEVERAL RUNS only when a day bound and the
        // month cut one month twice, which they cannot: a month bucket holds
        // one run and a day bucket one run. So the run found is the run.
        if ts >= run.hi {
            return Ok(self.at_run(run_at + 1, before + run.len()));
        }
        // THE HEADER ANSWERS AN INSTANT OUTSIDE THE FILE'S RECORDS with no
        // read: every record is at or after its first stamp and at or before
        // its last. A contract that stopped trading early in its month is
        // answered this way on most steps of a merge's bisection.
        if let Some(held) = self.slots.get(run.slot).map(|slot| slot.held) {
            if ts <= held.first_ts {
                return Ok(self.at_run(run_at, before));
            }
            if ts > held.last_ts {
                return Ok(self.at_run(run_at + 1, before + run.len()));
            }
        }
        let row = self.locate(root, run.slot, ts)?.clamp(run.start, run.end);
        if row >= run.end {
            return Ok(self.at_run(run_at + 1, before + run.len()));
        }
        Ok(Position {
            count: before + (row - run.start),
            run: run_at,
            row,
        })
    }

    /// The position at the start of `run`, with `count` kept rows before it.
    fn at_run(&self, run: usize, count: u64) -> Position {
        Position {
            count,
            run,
            row: self.runs.get(run).map_or(0, |at| at.start),
        }
    }

    /// The position of the `nth` kept row (0-based), or `None` past the end.
    ///
    /// A bisection over the run prefix sums: `O(log runs)`, at most thirteen
    /// steps at the widest time-of-day range.
    pub(super) fn nth(&self, nth: u64) -> Option<Position> {
        if nth >= self.total() {
            return None;
        }
        let (mut lo, mut hi) = (0_usize, self.runs.len());
        while hi - lo > 1 {
            let mid = lo + (hi - lo) / 2;
            if self.prefix.get(mid).copied().unwrap_or(u64::MAX) <= nth {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        let run = self.runs.get(lo)?;
        let before = self.prefix.get(lo).copied().unwrap_or(0);
        Some(Position {
            count: nth,
            run: lo,
            row: run.start + (nth - before),
        })
    }

    /// The kept position after `at`, or `None` past the last.
    pub(super) fn after(&self, at: Position) -> Option<Position> {
        let run = self.runs.get(at.run)?;
        if at.row + 1 < run.end {
            return Some(Position {
                count: at.count + 1,
                run: at.run,
                row: at.row + 1,
            });
        }
        let next = self.runs.get(at.run + 1)?;
        Some(Position {
            count: at.count + 1,
            run: at.run + 1,
            row: next.start,
        })
    }

    /// Whether the row before `at` in its file is itself a kept row — so a
    /// request that turned the lookback off still measures against it.
    pub(super) fn previous_is_kept(&self, at: Position) -> bool {
        let Some(run) = self.runs.get(at.run) else {
            return false;
        };
        if at.row > run.start {
            return true;
        }
        let Some(before) = at.run.checked_sub(1).and_then(|i| self.runs.get(i)) else {
            return false;
        };
        if before.slot == run.slot {
            return before.end == at.row;
        }
        // THE RUN BEFORE IS IN THE MONTH BEFORE: kept only when this row
        // opens its file, the earlier run closes its file, and no held month
        // lies between them.
        at.row == 0
            && self
                .slots
                .get(before.slot)
                .is_some_and(|slot| slot.held.n_valid == before.end)
            && before.slot + 1 == run.slot
            && self
                .month_of_slot(run.slot)
                .and_then(|month| self.previous_held(month))
                .is_some_and(|month| Some(month) == self.month_of_slot(before.slot))
    }

    /// The month of slot `slot`.
    pub(super) fn month_of_slot(&self, slot: usize) -> Option<YearMonth> {
        self.slots.get(slot).map(|at| at.month)
    }

    /// The held month before `month`, inside the range or not.
    ///
    /// A bisection over the directory's sorted listing: `O(log held)`.
    pub(super) fn previous_held(&self, month: YearMonth) -> Option<YearMonth> {
        let (mut lo, mut hi) = (0_usize, self.held_all.len());
        while lo < hi {
            let mid = lo + (hi - lo) / 2;
            if self.held_all.get(mid).is_some_and(|&at| at < month) {
                lo = mid + 1;
            } else {
                hi = mid;
            }
        }
        lo.checked_sub(1)
            .and_then(|at| self.held_all.get(at))
            .copied()
    }

    /// The slot holding `month`, when it is inside the range.
    pub(super) fn slot_of(&self, month: YearMonth) -> Option<usize> {
        let (mut lo, mut hi) = (0_usize, self.slots.len());
        while lo < hi {
            let mid = lo + (hi - lo) / 2;
            match self.slots.get(mid).map(|slot| slot.month.cmp(&month)) {
                Some(std::cmp::Ordering::Less) => lo = mid + 1,
                Some(std::cmp::Ordering::Equal) => return Some(mid),
                _ => hi = mid,
            }
        }
        None
    }

    /// The open file of `slot`, opening it on first use.
    fn file(&mut self, root: &Path, slot: usize) -> Result<&BarFile, String> {
        let (vendor, timeframe) = (self.vendor, self.timeframe);
        let key = &self.key;
        let at = self
            .slots
            .get_mut(slot)
            .ok_or_else(|| format!("slot {slot} is not a month of this window"))?;
        if let Some(why) = &at.failed {
            return Err(why.clone());
        }
        if at.file.is_none() {
            match open_classified(root, key.parts(vendor, timeframe, at.month)) {
                Ok(file) => at.file = Some(file),
                Err(why) => {
                    let named = format!("{}: {}", at.month, why.message);
                    at.failed = Some(named.clone());
                    return Err(named);
                }
            }
        }
        at.file
            .as_ref()
            .ok_or_else(|| format!("{} did not open", at.month))
    }

    /// The first row of `slot` at or after `ts`.
    fn locate(&mut self, root: &Path, slot: usize, ts: i64) -> Result<u64, String> {
        let file = self.file(root, slot)?;
        file.first_at_or_after(ts)
            .map_err(|why| format!("{}: cannot locate {ts}: {why}", file.path().display()))
    }

    /// Row `row` of `slot`.
    ///
    /// # Errors
    ///
    /// The file would not open, or the record would not read.
    pub(super) fn read(&mut self, root: &Path, slot: usize, row: u64) -> Result<Bar, String> {
        let file = self.file(root, slot)?;
        file.read_record(row)
            .map_err(|why| format!("{} record {row}: {why}", file.path().display()))
    }

    /// The last record of the held month `month`, which may lie outside the
    /// range, walking back past any held month with no records.
    ///
    /// `Ok(None)` when no earlier record is stored at all.
    pub(super) fn last_before(
        &mut self,
        root: &Path,
        month: YearMonth,
    ) -> Result<Option<Bar>, String> {
        let mut at = self.previous_held(month);
        while let Some(month) = at {
            if let Some(slot) = self.slot_of(month) {
                let n_valid = self.slots.get(slot).map_or(0, |s| s.held.n_valid);
                if let Some(last) = n_valid.checked_sub(1) {
                    return self.read(root, slot, last).map(Some);
                }
            } else {
                let file =
                    open_classified(root, self.key.parts(self.vendor, self.timeframe, month))
                        .map_err(|why| format!("{month}: {}", why.message))?;
                if let Some(last) = file.header().n_valid.checked_sub(1) {
                    return file
                        .read_record(last)
                        .map(Some)
                        .map_err(|why| format!("{month} record {last}: {why}"));
                }
            }
            at = self.previous_held(month);
        }
        Ok(None)
    }

    /// The IST days of `slot` that hold at least one record: one `.tix`
    /// lookup per calendar day of the month.
    ///
    /// # Errors
    ///
    /// The month would not open or locate.
    pub(super) fn days_covered(&mut self, root: &Path, slot: usize) -> Result<u32, String> {
        let Some(at) = self.slots.get(slot) else {
            return Ok(0);
        };
        let memo = (at.path.clone(), at.stamp);
        if let Some(days) = DAYS.get(&memo) {
            return Ok(days);
        }
        let held = at.held;
        if held.n_valid == 0 {
            DAYS.put(memo, 0);
            return Ok(0);
        }
        let mut covered = 0_u32;
        let mut day = day_of(held.first_ts);
        let last = day_of(held.last_ts);
        // ONE LOOKUP PER DAY: a day's end is the next day's start.
        let mut from = self.locate(root, slot, day_start(day))?;
        while day <= last {
            let to = self.locate(root, slot, day_start(day + 1))?;
            if to > from {
                covered += 1;
            }
            from = to;
            day += 1;
        }
        DAYS.put(memo, covered);
        Ok(covered)
    }
}

/// Days covered per month file, by stamp.
static DAYS: Memo<(PathBuf, Stamp), u32> = Memo::new();

/// The IST day (days since the epoch) of a UTC microsecond stamp.
pub(super) const fn day_of(ts: i64) -> i64 {
    (ts.div_euclid(MICROS) + pull::session::IST_OFFSET_SECS).div_euclid(DAY_SECS)
}

/// Months from `first` to `month`, `0` when `month` is not after it.
fn months_between(first: YearMonth, month: YearMonth) -> usize {
    let span = (i64::from(month.year()) * 12 + i64::from(month.month()))
        - (i64::from(first.year()) * 12 + i64::from(first.month()));
    usize::try_from(span).unwrap_or(0)
}

/// 00:00 IST of the IST day `day`, UTC micros.
pub(super) const fn day_start(day: i64) -> i64 {
    (day * DAY_SECS - pull::session::IST_OFFSET_SECS) * MICROS
}
