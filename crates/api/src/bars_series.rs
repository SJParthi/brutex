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
//!
//! UNVERIFIED: this cost names no test or bench that proves it (docs/06-limits.md,
//! "Cost claims of the /db fix round that name no proof").

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

use super::{Cancel, OpenRefusalKind, open_classified};

/// Microseconds in one second.
pub(super) const MICROS: i64 = 1_000_000;

/// Seconds in one day.
const DAY_SECS: i64 = 86_400;

/// The most series one window request may read. More is refused
/// `too_many_series` (D-3326): one request per contract of the widest NIFTY
/// expiry measured is 566, and a cap is what keeps one request's work bounded.
pub const MAX_SERIES: usize = 1_024;

/// Entries each process-wide memo holds, across its two generations.
///
/// # Evicted by generation, never emptied whole (DB-B57)
///
/// This memo was emptied whole when it filled: after about 675 distinct
/// time-of-day windows over NIFTY's 97 one-second months, every run of every
/// month was dropped in one `clear`, and the next request paid the first
/// request's O(days) again — a p99 cliff nothing announced. It now keeps two
/// generations of at most half the cap each. A new entry goes into the young
/// one; when that is full, the old generation is dropped and the young one
/// becomes the old. A hit in the old generation moves the entry back into the
/// young one, so whatever was used since the last turn survives it. At most
/// the older half is ever dropped at once, and a lookup or an insert stays
/// O(1): two hash probes and, once per half-cap inserts, the move of one map
/// (freed after the lock is released). No telemetry line is written from
/// here: this file emits nothing, by the shape test in `bars.rs`, so a turn
/// is counted in [`Memo::turns`] instead.
///
/// UNVERIFIED: this cost names no test or bench that proves it (docs/06-limits.md,
/// "Cost claims of the /db fix round that name no proof").
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
    /// The stamp of a file's metadata.
    pub(super) fn of(meta: &std::fs::Metadata) -> Self {
        Self {
            len: meta.len(),
            modified_secs: meta.mtime(),
            modified_nanos: meta.mtime_nsec(),
            inode: meta.ino(),
        }
    }
}

/// A bounded process-wide memo of two generations. See [`MEMO_CAP`].
pub(super) struct Memo<K, V> {
    map: OnceLock<Mutex<Generations<K, V>>>,
    /// Turns taken since the process started.
    turned: std::sync::atomic::AtomicU64,
}

/// The two generations behind a [`Memo`].
struct Generations<K, V> {
    /// Entries put, or hit, since the last turn.
    young: HashMap<K, V>,
    /// The generation before; dropped whole at the next turn.
    old: HashMap<K, V>,
}

/// The generation one turn dropped, handed out of the lock so it is freed
/// after the lock is released.
struct Turned<K, V> {
    dropped: HashMap<K, V>,
}

impl<K: Eq + Hash, V> Generations<K, V> {
    /// Puts `value` into the young generation, turning the generations first
    /// when the young one is full and `key` is new to it.
    fn admit(&mut self, key: K, value: V) -> Option<Turned<K, V>> {
        let mut turned = None;
        if !self.young.contains_key(&key) && self.young.len() >= MEMO_CAP / 2 {
            let older = std::mem::replace(&mut self.young, HashMap::with_capacity(1_024));
            let dropped = std::mem::replace(&mut self.old, older);
            turned = Some(Turned { dropped });
        }
        self.young.insert(key, value);
        turned
    }
}

impl<K: Eq + Hash, V: Clone> Memo<K, V> {
    /// An empty memo.
    pub(super) const fn new() -> Self {
        Self {
            map: OnceLock::new(),
            turned: std::sync::atomic::AtomicU64::new(0),
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Generations<K, V>> {
        self.map
            .get_or_init(|| {
                Mutex::new(Generations {
                    young: HashMap::with_capacity(1_024),
                    old: HashMap::new(),
                })
            })
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }

    /// The value held for `key`, cloned. A hit in the old generation moves
    /// the entry into the young one, so a key in use survives the next turn.
    pub(super) fn get(&self, key: &K) -> Option<V> {
        let mut gens = self.lock();
        if let Some(value) = gens.young.get(key) {
            return Some(value.clone());
        }
        let (key, value) = gens.old.remove_entry(key)?;
        let out = value.clone();
        let turned = gens.admit(key, value);
        drop(gens);
        self.announce(turned);
        Some(out)
    }

    /// Holds `value` for `key`, turning the generations when the young one is
    /// full: the older half is dropped, never the whole memo.
    pub(super) fn put(&self, key: K, value: V) {
        let mut gens = self.lock();
        gens.old.remove(&key);
        let turned = gens.admit(key, value);
        drop(gens);
        self.announce(turned);
    }

    /// Counts a turn and frees the generation it dropped, outside the lock.
    fn announce(&self, turned: Option<Turned<K, V>>) {
        let Some(turned) = turned else {
            return;
        };
        self.turned
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        drop(turned.dropped);
    }

    /// How many times the generations have turned: each turn dropped at most
    /// the older half, never the whole memo.
    #[cfg(test)]
    pub(super) fn turns(&self) -> u64 {
        self.turned.load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Entries held across both generations.
    #[cfg(test)]
    pub(super) fn len(&self) -> usize {
        let gens = self.lock();
        gens.young.len() + gens.old.len()
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

/// The first and last month a series is owed in, or `None` when nothing is.
///
/// # An index has no expiry to be outside of (DB-B47)
///
/// A contract lives from its listing to its expiry, so a month before its
/// first file or after its last is an expected absence, and that stays. An
/// `INDEX` never expires: a month after its newest file, inside the feed's
/// own span — the archive it was bought as, else the operator's window
/// (D-3338) — is a month not yet pulled or imported, a HOLE the feed owes.
/// Before this, GDFL INDIAVIX 1s (files to 2025-10, archive to 2026-09) read
/// its eleven newest missing months as `months_outside_life` and named none,
/// so its completeness read whole.
///
/// # What is not claimed
///
/// The START stays the first held month: an index launched inside the span
/// (NIFTYMIDSELECT's first file is 2022-03) and history never imported look
/// the same from the store, and no launch date is sourced here, so a month
/// before the first file is not called a hole. A `CASH` share keeps its held
/// life at both ends: a delisting, a merger or a rename (HDFC's last file is
/// 2023-07) ends a share's months exactly as a missing pull would, and no
/// lifecycle source tells them apart on this path. Both stay counted as
/// expected absences rather than invented as holes.
fn life_of(key: &SeriesKey, vendor: Vendor, held: &[YearMonth]) -> Option<(YearMonth, YearMonth)> {
    use brutex_core::instrument::Segment;
    let held_life = held.first().copied().zip(held.last().copied());
    let never_expires =
        key.contract.is_none() && matches!(Segment::parse(&key.segment), Ok(Segment::Index));
    let Some((born, died)) = held_life.filter(|_| never_expires) else {
        return held_life;
    };
    let span_end = pull::vendor::Feed::ALL
        .into_iter()
        .find(|feed| feed.store_vendor() == Some(vendor))
        .and_then(|feed| feed.descriptor().archive.or(feed.descriptor().window))
        .and_then(|span| span.to.year_month().ok());
    Some((born, span_end.map_or(died, |to| to.max(died))))
}

/// `YYYY-MM` as a month, or `None`: THE ONE MONTH SPELLING EVERY ROUTE READS.
///
/// Exactly seven bytes — four ASCII year digits, `-`, two ASCII month digits —
/// and then the range `YearMonth::new` already checks. Nothing else parses.
///
/// # Why the bytes are checked before any integer parse (app-7, AF-04)
///
/// Three routes split on `-` and handed each half to `str::parse`, and Rust's
/// integer `from_str` accepts a leading `+` and any number of leading zeros.
/// So `2024-3`, `2024-003`, `2024-0003`, `02024-03`, `+2024-03` and `2024-+3`
/// all answered March 2024 under HTTP 200 while the refusal sentence promised
/// `YYYY-MM`. This function was already the strict one for its own callers but
/// checked only the LENGTHS of the halves, so `2024-+3` (two bytes after the
/// dash) still passed it. One spelling of a month means one request text per
/// month in every access record and every bookmark.
///
/// O(1): a fixed seven-byte pattern, six digit tests and two small integers.
#[must_use]
pub(crate) fn month_of_text(text: &str) -> Option<YearMonth> {
    let &[y0, y1, y2, y3, b'-', m0, m1] = text.as_bytes() else {
        return None;
    };
    let digit = |byte: u8| byte.is_ascii_digit().then(|| byte - b'0');
    let year = [y0, y1, y2, y3]
        .into_iter()
        .try_fold(0u16, |year, byte| Some(year * 10 + u16::from(digit(byte)?)))?;
    let month = digit(m0)? * 10 + digit(m1)?;
    YearMonth::new(year, month).ok()
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
    /// Months a stretch read ([`Self::read_stretch`]) left rows of unread,
    /// one entry a month whatever the rows.
    unread: Vec<UnreadMonth>,
}

/// Rows between two looks at the client's cancellation flag during a read
/// (DB-N15).
pub(super) const CANCEL_STRIDE: u64 = 4_096;

/// A month a request asked to read rows of and could not: named ONCE, with
/// the rows left out, whatever their number.
///
/// A month another process holds (an import writer, `cli srt-build`) refuses
/// to open for every row alike. Named a row at a time, that was one fault
/// string per row — a 59 MB answer for six months of one-second bars, and up
/// to `MAX_SCAN_BARS` strings by the budget's own arithmetic.
#[derive(Debug, Clone, PartialEq, Eq)]
struct UnreadMonth {
    slot: usize,
    /// Rows asked for and not read.
    rows: u64,
    /// The first reason.
    why: String,
    /// The month never opened, so none of its rows was attempted.
    unopened: bool,
}

/// What [`SeriesPlan::take_unread`] drained: the rows and months left unread.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) struct Unread {
    /// Kept rows asked for and not read.
    pub(super) rows: u64,
    /// Held months with any such row.
    pub(super) months: usize,
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
        cancel: &super::Cancel,
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
            unread: Vec::new(),
        };
        let life = life_of(&plan.key, vendor, &held_all);
        let mut held = held_all.iter().copied().peekable();
        for &month in months {
            while held.next_if(|&at| at < month).is_some() {}
            if held.next_if_eq(&month).is_none() {
                if life.is_some_and(|(born, died)| born <= month && month <= died) {
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
            plan.cut_days(root, filter, (from, to), cancel)?;
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
    fn cut_days(
        &mut self,
        root: &Path,
        filter: Filter,
        tod: (u32, u32),
        cancel: &super::Cancel,
    ) -> Result<(), String> {
        let Buckets::Days { first, .. } = self.buckets else {
            return Ok(());
        };
        for slot in 0..self.slots.len() {
            // TWO LOOKUPS A DAY, so the client is asked after every month
            // (DB-N15): an abandoned request stops here, not at its end.
            cancel.check()?;
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
                // A DAY BEFORE THE EPOCH IS BEFORE EVERY MONTH, AND A DAY PAST
                // THE CALENDAR'S LAST YEAR IS AFTER EVERY MONTH. `at=` takes
                // any whole unix second and the calendar names no day past
                // 9999: both used to answer `-1`, so an instant in the year
                // 10000 ranked BEFORE every row.
                let Ok(day) = u32::try_from(day_of(ts)) else {
                    return -1;
                };
                pull::session::Day::from_days(day).map_or(i64::MAX, |day| {
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

    /// Whether the month of `slot` opens, and why not: the MONTH's refusal,
    /// the same for every row of it, which a page that reads one row at a
    /// time says once a month rather than once a row.
    ///
    /// # Errors
    ///
    /// The month's name and the reason its bar file did not open.
    pub(super) fn opens(&mut self, root: &Path, slot: usize) -> Result<(), String> {
        self.file(root, slot).map(|_| ())
    }

    /// The first row of `slot` at or after `ts`.
    pub(super) fn locate(&mut self, root: &Path, slot: usize, ts: i64) -> Result<u64, String> {
        let file = self.file(root, slot)?;
        file.first_at_or_after(ts)
            .map_err(|why| format!("{}: cannot locate {ts}: {why}", file.path().display()))
    }

    /// Row `row` of `slot`.
    ///
    /// # Errors
    ///
    /// The file would not open, or the record would not read.
    pub(super) fn read(
        &mut self,
        root: &Path,
        slot: usize,
        row: u64,
    ) -> Result<Bar, crate::fault_log::ReadFault> {
        let file = self.file(root, slot)?;
        // THE RECORD'S FAULT KEPT AS A VALUE, NOT A SENTENCE (DB-B75), so a
        // run of them over one file coalesces into one line.
        file.read_record(row)
            .map_err(|why| crate::fault_log::ReadFault::Record {
                file: file.path().display().to_string(),
                row,
                why,
            })
    }

    /// Rows `[start, end)` of `slot`, each handed to `each` with its row
    /// number; the client is asked every [`CANCEL_STRIDE`] rows. Returns the
    /// rows NOT read.
    ///
    /// **A month that will not open is one entry, and none of its rows is
    /// attempted**: the refusal is the month's, not a row's. A record of an
    /// open month that will not read is counted under the month's first such
    /// reason. Either way the month is named once by [`Self::take_unread`],
    /// so what a read leaves out costs one line a month and the answer stays
    /// bounded whatever the stretch holds.
    ///
    /// # Errors
    ///
    /// `cancelled`, by name.
    pub(super) fn read_stretch(
        &mut self,
        root: &Path,
        slot: usize,
        (start, end): (u64, u64),
        cancel: &Cancel,
        since: &mut u64,
        mut each: impl FnMut(u64, &Bar),
    ) -> Result<u64, String> {
        if end <= start {
            return Ok(0);
        }
        if let Err(why) = self.file(root, slot).map(|_| ()) {
            self.note_unread(slot, end - start, why, true);
            return Ok(end - start);
        }
        let mut missed = 0_u64;
        let mut first = None;
        for row in start..end {
            *since += 1;
            if *since >= CANCEL_STRIDE {
                *since = 0;
                cancel.check()?;
            }
            match self.read(root, slot, row) {
                Ok(bar) => each(row, &bar),
                Err(why) => {
                    missed += 1;
                    first.get_or_insert(why);
                }
            }
        }
        if let Some(why) = first {
            self.note_unread(slot, missed, why.into(), false);
        }
        Ok(missed)
    }

    /// Counts `rows` of `slot` as unread for `why`. Stretches are read in
    /// slot order, so a month's entry is the last one: one comparison.
    fn note_unread(&mut self, slot: usize, rows: u64, why: String, unopened: bool) {
        match self.unread.last_mut() {
            Some(at) if at.slot == slot => at.rows = at.rows.saturating_add(rows),
            _ => self.unread.push(UnreadMonth {
                slot,
                rows,
                why,
                unopened,
            }),
        }
    }

    /// Forgets what earlier stretch reads left unread: a request that asks a
    /// page again starts its count again.
    pub(super) fn clear_unread(&mut self) {
        self.unread.clear();
    }

    /// Empties the unread ledger into `faults` — ONE line a month, saying the
    /// reason and the rows not read — and returns what it held.
    ///
    /// The line goes to the request's [`FaultLog`] WITH its count
    /// ([`FaultLog::counted`]), so `faulted_records` is every refused record
    /// while the month stays one sentence (DB-B75); `each` sees the line too,
    /// for a caller that reports the months it could not read.
    pub(super) fn take_unread(
        &mut self,
        faults: &mut crate::fault_log::FaultLog,
        mut each: impl FnMut(&str),
    ) -> Unread {
        let mut out = Unread::default();
        for at in std::mem::take(&mut self.unread) {
            out.rows = out.rows.saturating_add(at.rows);
            out.months += 1;
            let month = self
                .slots
                .get(at.slot)
                .map_or_else(String::new, |slot| slot.month.to_string());
            let line = if at.unopened {
                format!(
                    "{}; the month did not open, so none of its {} kept rows in this window \
                     was read",
                    at.why, at.rows
                )
            } else {
                format!(
                    "{}; {} kept rows of {month} in this window would not read, this the first",
                    at.why, at.rows
                )
            };
            each(&line);
            // A MONTH THAT DID NOT OPEN REFUSED NO RECORD: none was tried.
            faults.counted(line, if at.unopened { 0 } else { at.rows });
        }
        out
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
                    return self.read(root, slot, last).map(Some).map_err(String::from);
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

#[cfg(test)]
mod generation_tests {
    #![allow(clippy::expect_used)]

    use super::*;

    /// **A FULL MEMO DROPS ITS OLDER HALF, NEVER EVERYTHING, AND A KEY IN USE
    /// SURVIVES.** DB-B57: `put` emptied the whole map at 65,536 entries, so
    /// about 675 distinct time-of-day windows over NIFTY's 97 one-second
    /// months dropped every cached run at once and the next request paid the
    /// first request's O(days) again.
    #[test]
    fn a_full_memo_turns_a_generation_and_keeps_what_is_in_use() {
        let memo: Memo<u64, u64> = Memo::new();
        let cap = MEMO_CAP as u64;
        memo.put(u64::MAX, 7);
        for key in 0..cap * 2 {
            memo.put(key, key);
            // THE KEY IN USE: read every thousand inserts, as a month asked
            // about again is.
            if key % 1_000 == 0 {
                assert_eq!(memo.get(&u64::MAX), Some(7), "lost at insert {key}");
            }
            assert!(memo.len() <= MEMO_CAP, "held {} past the cap", memo.len());
        }
        assert!(memo.turns() >= 3, "{} turns", memo.turns());
        // NEVER EMPTIED: right after a turn the newer half is still held.
        assert!(
            memo.len() >= MEMO_CAP / 2,
            "only {} held after {} turns",
            memo.len(),
            memo.turns()
        );
        // THE NEWEST ENTRIES ARE ALWAYS THERE, the oldest are not.
        assert_eq!(memo.get(&(cap * 2 - 1)), Some(cap * 2 - 1));
        assert_eq!(memo.get(&0), None);
        // A FRESHER VALUE SUPERSEDES ONE HELD IN THE OLD GENERATION.
        let held_old = cap * 2 - cap / 2 - 10;
        memo.put(held_old, 1);
        assert_eq!(memo.get(&held_old), Some(1));
    }

    fn months(text: &[&str]) -> Vec<YearMonth> {
        text.iter()
            .map(|m| month_of_text(m).expect("a month"))
            .collect()
    }

    /// **AN INDEX IS OWED ITS FEED'S SPAN AFTER ITS NEWEST FILE; A CONTRACT
    /// AND A SHARE ONLY THEIR HELD LIFE.** DB-B47.
    #[test]
    fn an_index_is_owed_the_feeds_span_after_its_newest_file() {
        let held = months(&["2019-01", "2019-03"]);
        let index = SeriesKey::parse("NSE-INDEX-INDIAVIX").expect("key");
        let share = SeriesKey::parse("NSE-CASH-BAJAJ-AUTO").expect("key");
        let option = SeriesKey::parse("NSE-FNO-NIFTY-2019-03-28-1150000-CE").expect("key");
        let span = |from: &str, to: &str| {
            Some((
                month_of_text(from).expect("m"),
                month_of_text(to).expect("m"),
            ))
        };
        // ZERODHA: the operator's window ends 2026-09 (D-3338).
        assert_eq!(
            life_of(&index, Vendor::Zerodha, &held),
            span("2019-01", "2026-09")
        );
        // GDFL: the archive it was bought as ends 2026-09-24.
        assert_eq!(
            life_of(&index, Vendor::Gdfl, &held),
            span("2019-01", "2026-09")
        );
        // A MONTH HELD PAST THE SPAN KEEPS ITS PLACE: the span never shortens.
        let late = months(&["2019-01", "2026-11"]);
        assert_eq!(
            life_of(&index, Vendor::Zerodha, &late),
            span("2019-01", "2026-11")
        );
        // A SHARE AND A CONTRACT LIVE FROM THEIR FIRST FILE TO THEIR LAST.
        assert_eq!(
            life_of(&share, Vendor::Zerodha, &held),
            span("2019-01", "2019-03")
        );
        assert_eq!(
            life_of(&option, Vendor::Gdfl, &held),
            span("2019-01", "2019-03")
        );
        // A FEED WITH NO RECORDED SPAN, AND A SERIES HOLDING NOTHING, CLAIM
        // NOTHING MORE THAN WHAT IS HELD.
        assert_eq!(
            life_of(&index, Vendor::Dhan, &held),
            span("2019-01", "2019-03")
        );
        assert_eq!(life_of(&index, Vendor::Zerodha, &[]), None);
    }
}

/// Tests written for lines and branches a cloud mutation and coverage pass
/// found unrun in this file (`reports/cloud-mut-api-a-20261017.md`). Every bar
/// is generated, in a scratch store the test removes.
#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "a fixture that cannot be planted is a test that cannot run, and \
              must fail loudly"
)]
mod cloud_mutant_tests {
    use super::*;
    use store::format::OI_NULL;

    const SYMBOL: &str = "CLOUDSERIES";

    fn key() -> SeriesKey {
        SeriesKey::new("NSE", "INDEX", SYMBOL, None)
    }

    fn month(m: u8) -> YearMonth {
        YearMonth::new(2025, m).expect("a month")
    }

    /// A scratch store root, emptied first.
    fn scratch(tag: &str) -> PathBuf {
        let root = crate::scratch::path(&format!("cloud-series-{tag}"));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("a scratch root");
        root
    }

    /// `minutes` (IST minutes of the day) on IST day `day` of `month`, as one
    /// minute bars of the fixture series. Appends; returns the file's path.
    fn plant(root: &Path, month: YearMonth, day: u8, minutes: &[i64]) -> PathBuf {
        let series = key();
        let address = StorePath::new(series.parts(Vendor::Dhan, Timeframe::MINUTE_1, month))
            .expect("a legal path");
        let id = u32::try_from(brutex_core::universe::fnv1a(SYMBOL) & u64::from(u32::MAX))
            .expect("the low 32 bits");
        let mut file = BarFile::open_or_create(root, address, id).expect("a bar file");
        let midnight = month.ist_bounds_micros().0 + i64::from(day - 1) * DAY_SECS * MICROS;
        let rows: Vec<Bar> = minutes
            .iter()
            .map(|&minute| Bar {
                ts_micros: midnight + minute * 60 * MICROS,
                open: 100,
                high: 110,
                low: 90,
                close: 105,
                volume: 1,
                open_interest: OI_NULL,
            })
            .collect();
        file.append(&rows).expect("generated records");
        address.to_path_buf(root)
    }

    fn plan(root: &Path, months: &[YearMonth], filter: Filter) -> SeriesPlan {
        SeriesPlan::build(
            root,
            Vendor::Dhan,
            Timeframe::MINUTE_1,
            key(),
            months,
            filter,
            &super::super::Cancel::never(),
        )
        .expect("a plan")
    }

    /// **ONLY A WHOLE DATE AND A STRIKE, OR A WHOLE DATE AND `FUT`, IS A
    /// CONTRACT'S TAIL.** Three fields are neither, and an empty year is no
    /// date. Runs `contract_shaped`'s `_` arm and the false branch of its
    /// `!text.is_empty()`.
    #[test]
    fn only_five_or_four_whole_fields_are_a_contract() {
        assert!(contract_shaped(&["2026", "08", "27", "2500000", "CE"]));
        assert!(contract_shaped(&["2026", "08", "27", "FUT"]));
        assert!(!contract_shaped(&["2026", "08", "27"]));
        assert!(!contract_shaped(&["", "08", "27", "2500000", "CE"]));
        assert!(!contract_shaped(&["2026", "08", "27", "", "CE"]));
    }

    /// **A WINDOW OF NO MONTHS IS REFUSED BY NAME**, not planned as empty.
    #[test]
    fn a_window_of_no_months_is_refused_by_name() {
        let got = SeriesPlan::build(
            Path::new("/nonexistent-cloud-series-root"),
            Vendor::Dhan,
            Timeframe::MINUTE_1,
            key(),
            &[],
            Filter::default(),
            &super::super::Cancel::never(),
        );
        assert_eq!(
            got.err(),
            Some("a window names at least one month".to_owned())
        );
    }

    /// **A HEADER MEMOISED UNDER AN OLDER STAMP IS READ AGAIN**: a month that
    /// grew after a request read it is planned at its new length. Takes the
    /// false branch of `at == stamp` in `header_of`.
    #[test]
    fn a_month_that_grew_is_planned_at_its_new_length() {
        let root = scratch("grew");
        plant(&root, month(5), 2, &[555, 556]);
        assert_eq!(plan(&root, &[month(5)], Filter::default()).total(), 2);
        plant(&root, month(5), 2, &[557]);
        assert_eq!(plan(&root, &[month(5)], Filter::default()).total(), 3);
        let _ = std::fs::remove_dir_all(&root);
    }

    /// **A MONTH THAT IS GONE WHEN IT IS OPENED IS NO SLOT, NOT A FAULT.**
    /// `header_of` stats the file, then opens it; another process may remove
    /// it between the two. The race is stood in for by a stat that finds a
    /// file and an open of an address that holds none. Runs `header_of`'s
    /// `Absent` arm.
    #[test]
    fn a_month_gone_at_its_open_is_absent() {
        let root = scratch("gone-at-open");
        let present = plant(&root, month(5), 2, &[555]);
        let series = key();
        let missing = series.parts(Vendor::Dhan, Timeframe::MINUTE_1, month(6));
        assert!(matches!(header_of(&root, missing, &present), Ok(None)));
        let _ = std::fs::remove_dir_all(&root);
    }

    /// **A DAY BOUND THAT LEAVES A HELD MONTH OUT CUTS NO RUN OF IT**: from
    /// 10 February, January keeps nothing and February keeps its rows. Takes
    /// the true branch of `lo >= hi` in `cut_months`.
    #[test]
    fn a_day_bound_past_a_month_cuts_no_run_of_it() {
        let root = scratch("bound-past");
        plant(&root, month(1), 2, &[555, 556]);
        plant(&root, month(2), 11, &[555, 556, 557]);
        let from = month(2).ist_bounds_micros().0 + 9 * DAY_SECS * MICROS;
        let cut = plan(
            &root,
            &[month(1), month(2)],
            Filter {
                from: Some(from),
                ..Filter::default()
            },
        );
        assert_eq!(cut.total(), 3);
        assert!(cut.runs.iter().all(|run| run.slot == 1), "February alone");
        let _ = std::fs::remove_dir_all(&root);
    }

    /// **A CUT UNDER THE OTHER BUCKET KIND IS NO CUT**: `cut_months` on a
    /// plan bucketed by day, and `cut_days` on one bucketed by month, add no
    /// run and refuse nothing.
    #[test]
    fn a_cut_under_the_other_bucket_kind_adds_no_run() {
        let root = scratch("other-kind");
        plant(&root, month(5), 2, &[600, 601]);
        let tod = Filter {
            tod: Some((36_000, 36_119)),
            ..Filter::default()
        };
        let mut by_day = plan(&root, &[month(5)], tod);
        let runs = by_day.runs.len();
        by_day.cut_months(&root, Filter::default()).expect("no cut");
        assert_eq!(by_day.runs.len(), runs);
        let mut by_month = plan(&root, &[month(5)], Filter::default());
        let runs = by_month.runs.len();
        by_month
            .cut_days(&root, tod, (36_000, 36_119), &super::super::Cancel::never())
            .expect("no cut");
        assert_eq!(by_month.runs.len(), runs);
        let _ = std::fs::remove_dir_all(&root);
    }

    /// **PAST THE LAST KEPT ROW THERE IS NO ROW, NO PREDECESSOR AND NO
    /// STRETCH TO READ, AND A SLOT THE PLAN DOES NOT HOLD COVERS NO DAY.**
    /// The edges of `nth`, `previous_is_kept`, `read_stretch` and
    /// `days_covered`, which their callers never ask.
    #[test]
    fn the_edges_past_a_plan_answer_nothing() {
        let root = scratch("edges");
        plant(&root, month(5), 2, &[555, 556]);
        let mut held = plan(&root, &[month(5)], Filter::default());
        assert_eq!(held.total(), 2);
        assert!(held.nth(1).is_some());
        assert!(held.nth(2).is_none(), "the third of two rows");
        let past = Position {
            count: 2,
            run: held.runs.len(),
            row: 0,
        };
        assert!(!held.previous_is_kept(past));
        let mut since = 0;
        assert_eq!(
            held.read_stretch(
                &root,
                0,
                (1, 1),
                &super::super::Cancel::never(),
                &mut since,
                |_, _| { unreachable!("an empty stretch reads no row") }
            ),
            Ok(0)
        );
        assert_eq!(since, 0, "and counts none toward the stride");
        assert_eq!(held.days_covered(&root, held.slots.len()), Ok(0));
        assert_eq!(held.days_covered(&root, 0), Ok(1));
        let _ = std::fs::remove_dir_all(&root);
    }

    /// **UNDER A TIME-OF-DAY BOUND, A MONTH'S FIRST KEPT ROW MEASURES
    /// AGAINST THE MONTH BEFORE ONLY WHEN IT OPENS ITS FILE, THE MONTH BEFORE
    /// CLOSED ON A KEPT ROW, AND NO HELD MONTH LIES BETWEEN.** The window is
    /// 10:00–10:59. January keeps its every row; February holds only a 09:15
    /// row, so it keeps none; March opens on a kept row; April opens on a
    /// 09:15 row it does not keep. March's first kept row does not measure
    /// against January (February is held between), and April's does not
    /// measure against March (the row before it in its file is not kept).
    /// Takes the false branches of `at.row == 0` and of
    /// `before.slot + 1 == run.slot` in `previous_is_kept`.
    #[test]
    fn a_first_kept_row_measures_back_only_across_adjacent_whole_files() {
        let root = scratch("lookback-tod");
        plant(&root, month(1), 2, &[600, 601]);
        plant(&root, month(2), 3, &[555]);
        plant(&root, month(3), 3, &[600, 601]);
        plant(&root, month(4), 1, &[555, 600]);
        let held = plan(
            &root,
            &[month(1), month(2), month(3), month(4)],
            Filter {
                tod: Some((36_000, 39_599)),
                ..Filter::default()
            },
        );
        let first_of = |slot: usize| {
            let run = held
                .runs
                .iter()
                .position(|run| run.slot == slot)
                .expect("a kept run");
            Position {
                count: 0,
                run,
                row: held.runs[run].start,
            }
        };
        assert!(
            held.runs.iter().all(|run| run.slot != 1),
            "February keeps nothing"
        );
        let march = first_of(2);
        assert_eq!(march.row, 0, "March opens on a kept row");
        assert!(!held.previous_is_kept(march), "February is held between");
        let april = first_of(3);
        assert_eq!(april.row, 1, "April's 09:15 row is not kept");
        assert!(!held.previous_is_kept(april));
        // THE SAME RULE, MET: January's second kept row follows its first.
        let january = first_of(0);
        assert!(held.previous_is_kept(Position {
            count: 1,
            run: january.run,
            row: 1
        }));
        let _ = std::fs::remove_dir_all(&root);
    }

    /// **A PAGE ASKED AGAIN STARTS ITS UNREAD COUNT AGAIN, AND A COUNT IS
    /// TAKEN ONCE.** A stretch of a slot the plan does not hold is counted
    /// unread; cleared, the ledger gives nothing; not cleared, it gives the
    /// month once. Kills `clear_unread -> ()`, which the 16 Oct pass called
    /// unobservable because every reader drains the ledger first: the
    /// method's own contract is asked here.
    #[test]
    fn clearing_the_unread_ledger_forgets_what_a_read_left_unread() {
        let root = scratch("clear-unread");
        plant(&root, month(5), 2, &[555]);
        let mut held = plan(&root, &[month(5)], Filter::default());
        let mut faults = crate::fault_log::FaultLog::default();
        let mut since = 0;
        let unread = |held: &mut SeriesPlan, since: &mut u64| {
            held.read_stretch(&root, 9, (0, 3), &Cancel::never(), since, |_, _| {})
                .expect("a refusal is counted, not raised")
        };
        assert_eq!(unread(&mut held, &mut since), 3);
        held.clear_unread();
        assert_eq!(held.take_unread(&mut faults, |_| {}), Unread::default());
        assert_eq!(unread(&mut held, &mut since), 3);
        assert_eq!(
            held.take_unread(&mut faults, |_| {}),
            Unread { rows: 3, months: 1 }
        );
        assert_eq!(held.take_unread(&mut faults, |_| {}), Unread::default());
        let _ = std::fs::remove_dir_all(&root);
    }

    /// **AN INSTANT PAST A DAY'S WINDOW, ON THAT DAY, RANKS AT THE NEXT DAY'S
    /// FIRST KEPT ROW**: under a 10:00–10:59 window over the 2nd and the 3rd,
    /// noon on the 2nd has both of the 2nd's kept rows before it and stands
    /// at the 3rd's run. Kills `run_at + 1 -> run_at * 1` in `rank`, which
    /// the 16 Oct pass called unobservable because callers read only the
    /// count: the position's run and row are the answer here.
    #[test]
    fn an_instant_past_a_days_window_ranks_at_the_next_days_first_row() {
        let root = scratch("rank-past-window");
        plant(&root, month(5), 2, &[600, 601]);
        plant(&root, month(5), 3, &[600, 601]);
        let mut held = plan(
            &root,
            &[month(5)],
            Filter {
                tod: Some((36_000, 39_599)),
                ..Filter::default()
            },
        );
        assert_eq!(held.runs.len(), 2, "one run a day");
        let noon = month(5).ist_bounds_micros().0 + (DAY_SECS + 12 * 3_600) * MICROS;
        assert_eq!(
            held.rank(&root, noon),
            Ok(Position {
                count: 2,
                run: 1,
                row: 2
            })
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    /// **A MONTH NO SLOT HOLDS HAS NO SLOT, BEFORE THE FIRST OR AFTER THE
    /// LAST.** Kills `mid + 1 -> mid * 1` in `slot_of`, which the 16 Oct pass
    /// called equivalent having asked only a month before every slot: past
    /// the last slot the search would never end.
    #[test]
    fn a_month_outside_the_slots_has_no_slot() {
        let root = scratch("slot-outside");
        plant(&root, month(5), 2, &[555]);
        let held = plan(&root, &[month(4), month(5), month(6)], Filter::default());
        assert_eq!(held.slots.len(), 1);
        assert_eq!(held.slot_of(month(4)), None);
        assert_eq!(held.slot_of(month(5)), Some(0));
        assert_eq!(held.slot_of(month(6)), None);
        let _ = std::fs::remove_dir_all(&root);
    }
}
