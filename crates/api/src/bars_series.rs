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
//!   only. A time-of-day bound cuts each day's window from the month's
//!   TIME-OF-DAY INDEX ([`TodIndex`], R3-07, DB-B57): built once per month
//!   file stamp from the month summary R3-06 keeps beside it, it answers "rows
//!   before this instant" with one array index and one popcount, so a NEW
//!   window costs two in-memory lookups a day and no file is opened — what a
//!   kept window costs, give or take that arithmetic. A month that keeps no
//!   usable summary (none written yet, stale, or one whose counts do not
//!   add up) is located the old way, two `.tix` lookups a day, and is
//!   COUNTED and NAMED on the answer (`tod_index`), never silently slower.
//!   Either way the runs are cached per month file and window ([`RUNS`]).
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

/// A process-wide memo held within a byte budget, in two generations of at
/// most half the budget each — the [`Memo`] rule by weight rather than by
/// count (DB-B57): when the young generation would pass its half, the old one
/// is dropped and the young one becomes old, so at most the older half is
/// ever dropped at once and a value in use (hit since the last turn) survives
/// the turn. One value heavier than half the budget is not held. A lookup or
/// an insert is two hash probes; a turn moves one map, freed outside the
/// lock.
pub(super) struct Weighed<K, V> {
    map: OnceLock<Mutex<WeighedGenerations<K, V>>>,
    budget: usize,
    turned: std::sync::atomic::AtomicU64,
}

/// The two generations behind a [`Weighed`], each value with its weight.
struct WeighedGenerations<K, V> {
    young: HashMap<K, (V, usize)>,
    young_bytes: usize,
    old: HashMap<K, (V, usize)>,
}

impl<K: Eq + Hash, V> WeighedGenerations<K, V> {
    /// Puts `value` into the young generation, turning first when it would
    /// pass `half`. Returns the dropped generation, to be freed unlocked.
    fn admit(
        &mut self,
        key: K,
        value: V,
        weight: usize,
        half: usize,
    ) -> Option<Turned<K, (V, usize)>> {
        let mut turned = None;
        if let Some((_, was)) = self.young.remove(&key) {
            self.young_bytes = self.young_bytes.saturating_sub(was);
        }
        if self.young_bytes.saturating_add(weight) > half {
            let older = std::mem::replace(&mut self.young, HashMap::with_capacity(64));
            let dropped = std::mem::replace(&mut self.old, older);
            self.young_bytes = 0;
            turned = Some(Turned { dropped });
        }
        self.young_bytes = self.young_bytes.saturating_add(weight);
        self.young.insert(key, (value, weight));
        turned
    }
}

impl<K: Eq + Hash, V: Clone> Weighed<K, V> {
    /// An empty memo of `budget` bytes across both generations.
    pub(super) const fn new(budget: usize) -> Self {
        Self {
            map: OnceLock::new(),
            budget,
            turned: std::sync::atomic::AtomicU64::new(0),
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, WeighedGenerations<K, V>> {
        self.map
            .get_or_init(|| {
                Mutex::new(WeighedGenerations {
                    young: HashMap::with_capacity(64),
                    young_bytes: 0,
                    old: HashMap::new(),
                })
            })
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }

    /// The value held for `key`, cloned; a hit in the old generation moves
    /// it into the young one.
    pub(super) fn get(&self, key: &K) -> Option<V> {
        let mut gens = self.lock();
        if let Some((value, _)) = gens.young.get(key) {
            return Some(value.clone());
        }
        let (key, (value, weight)) = gens.old.remove_entry(key)?;
        let out = value.clone();
        let turned = gens.admit(key, value, weight, self.budget / 2);
        drop(gens);
        self.announce(turned);
        Some(out)
    }

    /// Holds `value`, which weighs `weight` bytes. One heavier than half the
    /// budget is not held: it would turn a whole generation out for itself.
    pub(super) fn put(&self, key: K, value: V, weight: usize) {
        if weight > self.budget / 2 {
            return;
        }
        let mut gens = self.lock();
        gens.old.remove(&key);
        let turned = gens.admit(key, value, weight, self.budget / 2);
        drop(gens);
        self.announce(turned);
    }

    fn announce<T>(&self, turned: Option<Turned<K, T>>) {
        let Some(turned) = turned else {
            return;
        };
        self.turned
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        drop(turned.dropped);
    }

    /// Turns taken since the process started.
    #[cfg(test)]
    pub(super) fn turns(&self) -> u64 {
        self.turned.load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Bytes held across both generations.
    #[cfg(test)]
    pub(super) fn bytes(&self) -> usize {
        let gens = self.lock();
        gens.young_bytes + gens.old.values().map(|(_, weight)| *weight).sum::<usize>()
    }
}

/// Bytes the time-of-day indexes may hold, both generations together.
///
/// A one-second index month of 22 trading days holds about 4.2 KB a day (the
/// held seconds of the day as a bitmap, 352 words, and a 32-bit rank a word):
/// about 92 KB a month, so 64 MiB holds about 700 such months — NIFTY's,
/// BANKNIFTY's and five more 97-month one-second histories at once. A minute
/// month is under 5 KB.
const TOD_INDEX_BUDGET: usize = 64 * 1024 * 1024;

/// A month's time-of-day index: for every IST day of the month, the rows of
/// the month before it, and a rank over the day's held units (seconds, or
/// minutes for a rung of a minute or coarser), so the number of rows stamped
/// before any instant of the month is one array index and one popcount.
/// Built once per month file stamp from the month summary (R3-06) and kept
/// in [`TOD_INDEXES`]. R3-07, DB-B57.
#[derive(Debug)]
pub(super) struct TodIndex {
    /// Epoch day of the month's first day.
    first_day: i64,
    /// Seconds one held unit spans: 1 below a minute, 60 at a minute or more.
    unit: i64,
    /// Rows of the month.
    n_valid: u64,
    /// One entry per day of the month.
    days: Vec<TodDay>,
}

/// One day of a [`TodIndex`].
#[derive(Debug, Default)]
struct TodDay {
    /// Rows of the month stamped before this day.
    before: u64,
    /// Rows stamped on it.
    bars: u64,
    /// The word of the day's bitmap `words[0]` is.
    first_word: u32,
    /// The held units of the day, one bit each, from `first_word * 64`.
    words: Box<[u64]>,
    /// `rank[i]` is the set bits of `words[..i]`.
    rank: Box<[u32]>,
}

impl TodDay {
    /// Rows of this day stamped at a unit below `unit`.
    fn below(&self, unit: u64) -> u64 {
        let word = unit / 64;
        let Some(local) = word.checked_sub(u64::from(self.first_word)) else {
            return 0;
        };
        let Ok(local) = usize::try_from(local) else {
            return self.bars;
        };
        match (self.words.get(local), self.rank.get(local)) {
            (Some(&bits), Some(&ranked)) => {
                let mask = (1_u64 << (unit % 64)) - 1;
                u64::from(ranked) + u64::from((bits & mask).count_ones())
            }
            _ => self.bars,
        }
    }

    /// A day from its held units, ascending and distinct.
    fn of(before: u64, held: &[u32]) -> Self {
        let (Some(&first), Some(&last)) = (held.first(), held.last()) else {
            return Self {
                before,
                ..Self::default()
            };
        };
        let (first_word, last_word) = (first / 64, last / 64);
        let span = usize::try_from(last_word - first_word + 1).unwrap_or(0);
        let mut words = vec![0_u64; span];
        for unit in held {
            if let Some(word) = usize::try_from(unit / 64 - first_word)
                .ok()
                .and_then(|at| words.get_mut(at))
            {
                *word |= 1_u64 << (unit % 64);
            }
        }
        let mut rank = Vec::with_capacity(span);
        let mut sum = 0_u32;
        for word in &words {
            rank.push(sum);
            sum = sum.saturating_add(word.count_ones());
        }
        Self {
            before,
            bars: u64::try_from(held.len()).unwrap_or(u64::MAX),
            first_word,
            words: words.into(),
            rank: rank.into(),
        }
    }

    /// Bytes this day holds.
    fn weight(&self) -> usize {
        self.words.len() * 8 + self.rank.len() * 4 + 48
    }
}

impl TodIndex {
    /// The index of a month from its kept summary, or why it cannot be one.
    ///
    /// Every count the rank arithmetic leans on is checked against the
    /// header the summary must describe and against itself: the same
    /// generation, rows and first and last stamp; no row off the rung's grid,
    /// out of order or outside the month; exactly one held unit per row on
    /// every day. A summary that fails any of them is refused by name and the
    /// month is located the old way — never indexed from counts that do not
    /// add up.
    ///
    /// # Errors
    ///
    /// The reason, in words.
    pub(super) fn from_summary(
        summary: &store::month_summary::MonthSummary,
        month: YearMonth,
        timeframe: Timeframe,
        held: Held,
    ) -> Result<Self, String> {
        if summary.month != month || summary.timeframe_secs != timeframe.secs() {
            return Err("its summary describes another month or rung".to_owned());
        }
        let stamps = held.n_valid == 0
            || (summary.first_ts_micros == held.first_ts && summary.last_ts_micros == held.last_ts);
        if summary.generation != held.generation || summary.n_valid != held.n_valid || !stamps {
            return Err(format!(
                "its summary describes generation {} with {} rows, the file is at generation {} \
                 with {} rows",
                summary.generation, summary.n_valid, held.generation, held.n_valid
            ));
        }
        let counts = summary.counts;
        if counts.folded != held.n_valid
            || counts.invalid != 0
            || counts.decreasing != 0
            || counts.outside_month != 0
        {
            return Err(format!(
                "its summary folded {} rows with {} off the grid or repeated, {} out of order \
                 and {} outside the month, so its days cannot be counted into rows",
                counts.folded, counts.invalid, counts.decreasing, counts.outside_month
            ));
        }
        let seconds = timeframe.secs() < 60;
        let unit = if seconds { 1 } else { 60 };
        let first_day = summary.first_day();
        let last_day = summary.last_day();
        let mut minutes: Vec<Vec<u32>> = Vec::new();
        if !seconds {
            minutes = vec![Vec::new(); usize::try_from(last_day - first_day + 1).unwrap_or(0)];
            for stamp in summary.minute_stamps() {
                let day = day_of(stamp);
                let minute = (stamp - day_start(day)) / (60 * MICROS);
                if let (Some(slot), Ok(minute)) = (
                    usize::try_from(day - first_day)
                        .ok()
                        .and_then(|at| minutes.get_mut(at)),
                    u32::try_from(minute),
                ) {
                    slot.push(minute);
                }
            }
        }
        let mut days = Vec::with_capacity(minutes.len().max(31));
        let mut before = 0_u64;
        for (at, day) in (first_day..=last_day).enumerate() {
            let held_units = if seconds {
                summary.seconds_on(day).ok_or_else(|| {
                    "its summary keeps no seconds (the density rule leaves them out of a \
                     sparse month)"
                        .to_owned()
                })?
            } else {
                minutes.get_mut(at).map(std::mem::take).unwrap_or_default()
            };
            let entry = TodDay::of(before, &held_units);
            if entry.bars != u64::from(summary.bars_on(day)) {
                return Err(format!(
                    "its summary counts {} bars on day {day} and {} held {}",
                    summary.bars_on(day),
                    entry.bars,
                    if seconds { "seconds" } else { "minutes" }
                ));
            }
            before = before.saturating_add(entry.bars);
            days.push(entry);
        }
        if before != held.n_valid {
            return Err(format!(
                "its summary's days hold {before} rows, the file {}",
                held.n_valid
            ));
        }
        Ok(Self {
            first_day,
            unit,
            n_valid: held.n_valid,
            days,
        })
    }

    /// Rows of the month stamped strictly before `ts`: what
    /// `BarFile::first_at_or_after(ts)` answers, from memory. One day lookup,
    /// one rank lookup and one popcount.
    pub(super) fn rows_before(&self, ts: i64) -> u64 {
        let day = day_of(ts);
        let Ok(at) = usize::try_from(day - self.first_day) else {
            return 0;
        };
        let Some(entry) = self.days.get(at) else {
            return self.n_valid;
        };
        // A UNIT u HOLDS A ROW STAMPED u × unit SECONDS INTO THE DAY, which is
        // before `ts` exactly when u < ceil(into / (unit × 1 s)).
        let into = ts - day_start(day);
        let span = self.unit * MICROS;
        let unit = u64::try_from((into + span - 1).div_euclid(span)).unwrap_or(0);
        entry.before + entry.below(unit)
    }

    /// Bytes this index holds.
    fn weight(&self) -> usize {
        self.days.iter().map(TodDay::weight).sum::<usize>() + 64
    }
}

/// Every month file's time-of-day index, or why it has none, by the bar
/// file's stamp and the summary's (a summary rewritten beside an unchanged
/// month is read again).
static TOD_INDEXES: Weighed<TodKey, TodEntry> = Weighed::new(TOD_INDEX_BUDGET);

/// A month file, its stamp and its summary's stamp (`None` when it has none).
type TodKey = (PathBuf, Stamp, Option<Stamp>);

/// A month's time-of-day index, or why it has none.
type TodEntry = Arc<Result<TodIndex, String>>;

/// The time-of-day index of one held month, from the memo when neither the
/// bar file nor its summary changed, else built from the summary. One `stat`
/// of the summary when the memo holds it; one read of it (a few kilobytes to
/// about 55 KB for a one-second month) and a pass over its held units when
/// it does not.
fn tod_index_of(
    path: &Path,
    stamp: Stamp,
    month: YearMonth,
    timeframe: Timeframe,
    held: Held,
) -> TodEntry {
    let beside = store::month_summary::path_beside(path);
    let summary_stamp = std::fs::symlink_metadata(&beside)
        .ok()
        .map(|meta| Stamp::of(&meta));
    let key = (path.to_path_buf(), stamp, summary_stamp);
    if let Some(index) = TOD_INDEXES.get(&key) {
        return index;
    }
    let built = match store::month_summary::read_kept(path, held.n_valid) {
        Ok(summary) => TodIndex::from_summary(&summary, month, timeframe, held),
        Err(why) => Err(why.to_string()),
    };
    let weight = built.as_ref().map_or(256, TodIndex::weight);
    let built = Arc::new(built);
    TOD_INDEXES.put(key, Arc::clone(&built), weight);
    built
}

/// How one request's time-of-day bound was cut, month by month (R3-07): from
/// the runs memo, from the month's time-of-day index, or located day by day
/// because the month keeps no usable summary — counted, and the first
/// reason named, so a slow first window is never silent.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TodReport {
    /// Months whose runs this window was already cut for.
    pub kept: usize,
    /// Months cut from their time-of-day index, no bar file opened.
    pub indexed: usize,
    /// Months located day by day through their `.tix`.
    pub located: usize,
    /// Why the first located month was not indexed.
    pub why: Option<String>,
}

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
fn life_of(
    root: &Path,
    key: &SeriesKey,
    vendor: Vendor,
    held: &[YearMonth],
) -> Option<(YearMonth, YearMonth)> {
    use brutex_core::instrument::Segment;
    let held_life = held.first().copied().zip(held.last().copied());
    let never_expires =
        key.contract.is_none() && matches!(Segment::parse(&key.segment), Ok(Segment::Index));
    let Some((born, died)) = held_life.filter(|_| never_expires) else {
        return held_life;
    };
    let span_end = served_end(root, vendor);
    Some((born, span_end.map_or(died, |to| to.max(died))))
}

/// The last month a feed SERVES, from what the store and its sources hold
/// rather than a date typed into the program (R3-05, R3-07): the widest of
/// the descriptor's dated measurement of the archive, the newest day the
/// import journal names (not after today: a line stamped later is no
/// evidence of a delivery), and the operator's window as the store's
/// settings hold it now. So importing one day past the old end, or saving a
/// later window, moves an index's owed months with no rebuild.
///
/// `None` for a feed with no recorded span (an index's life is then what it
/// holds). Cost: one `stat` of the settings and, for an archive feed, one of
/// the journal while neither changed — both readers keep what they read —
/// and only an `INDEX` series asks.
fn served_end(root: &Path, vendor: Vendor) -> Option<YearMonth> {
    let feed = pull::vendor::Feed::ALL
        .into_iter()
        .find(|feed| feed.store_vendor() == Some(vendor))?;
    let descriptor = feed.descriptor();
    let day_month = |day: pull::session::Day| day.year_month().ok();
    let mut end: Option<YearMonth> = None;
    let mut widen = |month: Option<YearMonth>| {
        end = match (end, month) {
            (Some(held), Some(more)) => Some(held.max(more)),
            (held, more) => held.or(more),
        };
    };
    if let Some(archive) = descriptor.archive {
        widen(day_month(archive.to));
        let today = crate::ingest::today_ist().ok();
        if let Ok(journal) = crate::server::import_journal::snapshot(root)
            && let Some((_, last)) = journal.parsed.span()
        {
            let last = today.map_or(last, |today| last.min(today));
            widen(day_month(last));
        }
    }
    if let Some(window) = descriptor.window {
        let settings = crate::server::settings_route::snapshot(root);
        let to =
            crate::server::settings_route::effective(&settings, store::settings::Key::WindowTo);
        let saved = to
            .value
            .as_deref()
            .and_then(|text| text.get(..7))
            .and_then(month_of_text);
        // THE SETTING WHEN ONE IS IN FORCE, else the descriptor's own end:
        // a window the operator narrowed narrows what is owed.
        widen(saved.or_else(|| day_month(window.to)));
    }
    end
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
/// O(1): a fixed seven-byte pattern, six digit tests and two small integers
/// (AF4-01:
/// `api::server::bars_page_tests::the_one_month_parser_takes_exactly_four_digits_a_dash_and_two`).
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
    /// How a time-of-day bound was cut, month by month (R3-07).
    pub(super) tod: TodReport,
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
            tod: TodReport::default(),
        };
        let life = life_of(root, &plan.key, vendor, &held_all);
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
    /// row, cached per month file and window. A month whose runs are not
    /// cached yet is cut from its time-of-day index ([`TodIndex`]) when its
    /// summary is usable, with no bar file opened, else located day by day
    /// through its `.tix` — counted in [`Self::tod`] and named either way.
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
            let memo = (path.clone(), stamp, lo, hi, tod.0, tod.1);
            let runs = if let Some(runs) = RUNS.get(&memo) {
                self.tod.kept = self.tod.kept.saturating_add(1);
                runs
            } else {
                // THE MONTH'S TIME-OF-DAY INDEX FIRST (R3-07, DB-B57): a new
                // window is two in-memory lookups a day, not two `.tix` reads.
                let index = tod_index_of(&path, stamp, month, self.timeframe, held);
                let runs: Vec<Run> = match index.as_ref() {
                    Ok(index) => {
                        self.tod.indexed = self.tod.indexed.saturating_add(1);
                        Self::day_runs(slot, (lo, hi), held, tod, |ts| Ok(index.rows_before(ts)))?
                    }
                    Err(why) => {
                        // LOCATED, AND SAID: the answer is the same, the cost
                        // is two `.tix` lookups a day, and the reason is on
                        // the wire (`tod_index.why`).
                        self.tod.located = self.tod.located.saturating_add(1);
                        if self.tod.why.is_none() {
                            self.tod.why = Some(format!("{} {month}: {why}", self.key.text()));
                        }
                        let mut locate = |ts: i64| self.locate(root, slot, ts);
                        Self::day_runs(slot, (lo, hi), held, tod, &mut locate)?
                    }
                };
                let runs: Arc<[Run]> = runs.into();
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
    /// file's records, its `[day + from, day + to + 1 s)` window located by
    /// `locate` (rows stamped strictly before an instant). ONE loop for both
    /// ways a month is located, so the index and the `.tix` cannot cut a
    /// different window.
    fn day_runs(
        slot: usize,
        (lo, hi): (i64, i64),
        held: Held,
        (from, to): (u32, u32),
        mut locate: impl FnMut(i64) -> Result<u64, String>,
    ) -> Result<Vec<Run>, String> {
        let mut out = Vec::with_capacity(31);
        let mut day = day_of(lo.max(held.first_ts));
        let last = day_of(hi.saturating_sub(1).min(held.last_ts));
        while day <= last {
            let start_of_day = day_start(day);
            let window_lo = (start_of_day + i64::from(from) * MICROS).max(lo);
            let window_hi = (start_of_day + (i64::from(to) + 1) * MICROS).min(hi);
            if window_lo < window_hi {
                let start = locate(window_lo)?;
                let end = locate(window_hi)?;
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
        // A STORE WITH NO JOURNAL AND NO SAVED SETTINGS: the descriptor and
        // the built-in window decide.
        let root = crate::scratch::path("lifeof-bare");
        std::fs::create_dir_all(&root).expect("a root");
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
            life_of(&root, &index, Vendor::Zerodha, &held),
            span("2019-01", "2026-09")
        );
        // GDFL: the archive it was bought as ends 2026-09-24.
        assert_eq!(
            life_of(&root, &index, Vendor::Gdfl, &held),
            span("2019-01", "2026-09")
        );
        // A MONTH HELD PAST THE SPAN KEEPS ITS PLACE: the span never shortens.
        let late = months(&["2019-01", "2026-11"]);
        assert_eq!(
            life_of(&root, &index, Vendor::Zerodha, &late),
            span("2019-01", "2026-11")
        );
        // A SHARE AND A CONTRACT LIVE FROM THEIR FIRST FILE TO THEIR LAST.
        assert_eq!(
            life_of(&root, &share, Vendor::Zerodha, &held),
            span("2019-01", "2019-03")
        );
        assert_eq!(
            life_of(&root, &option, Vendor::Gdfl, &held),
            span("2019-01", "2019-03")
        );
        // A FEED WITH NO RECORDED SPAN, AND A SERIES HOLDING NOTHING, CLAIM
        // NOTHING MORE THAN WHAT IS HELD.
        assert_eq!(
            life_of(&root, &index, Vendor::Dhan, &held),
            span("2019-01", "2019-03")
        );
        assert_eq!(life_of(&root, &index, Vendor::Zerodha, &[]), None);
    }

    /// **AN INDEX'S LIFE ENDS WHERE ITS FEED IS SERVED TO, NOT AT A TYPED
    /// DATE** (R3-07 item 7, after R3-05). A journal line for a GDFL day past
    /// the descriptor's archive end moves the owed life of every index with
    /// no rebuild; a line stamped after today is no evidence and is clamped.
    #[test]
    fn an_index_life_ends_at_the_served_span_not_a_fixed_date() {
        let held = months(&["2019-01", "2019-03"]);
        let index = SeriesKey::parse("NSE-INDEX-INDIAVIX").expect("key");
        let root = crate::server::import_journal::tests::journal(
            "r307-life",
            "begin indices 2026-10-05 *\ndone indices 2026-10-05 * files=2 seconds=44891 \
             failures=0 late=0 late_unresolved=0 max_back_s=0\n",
        );
        let life = life_of(&root, &index, Vendor::Gdfl, &held).expect("a life");
        assert_eq!(life.0, month_of_text("2019-01").expect("m"));
        let today = crate::ingest::today_ist().expect("today");
        let owed_to = today
            .year_month()
            .expect("a month")
            .min(month_of_text("2026-10").expect("m"))
            .max(month_of_text("2026-09").expect("m"));
        assert_eq!(
            life.1, owed_to,
            "the journal's 2026-10-05, never past today"
        );
        // THE SAME INDEX ON A STORE WITH NO JOURNAL ENDS AT THE DESCRIPTOR.
        let bare = crate::scratch::path("lifeof-nojournal");
        std::fs::create_dir_all(&bare).expect("a root");
        assert_eq!(
            life_of(&bare, &index, Vendor::Gdfl, &held).map(|(_, to)| to),
            month_of_text("2026-09")
        );
    }
}

#[cfg(test)]
mod tod_index_tests {
    #![allow(clippy::expect_used, clippy::indexing_slicing)]

    use super::*;

    /// UTC micros of an IST wall clock on 2024-01-`date`.
    fn ist(date: i64, hour: i64, minute: i64, second: i64) -> i64 {
        // 2024-01-01 is epoch day 19,723.
        ((19_722 + date) * 86_400 + hour * 3_600 + minute * 60 + second - 19_800) * MICROS
    }

    /// An invented bar at `ts`: prices that cannot be mistaken for a market's.
    fn bar(ts: i64, n: i64) -> Bar {
        Bar {
            ts_micros: ts,
            open: 1_000 + n % 7,
            high: 1_010 + n % 7,
            low: 990 + n % 7,
            close: 1_005 + n % 7,
            volume: 0,
            open_interest: store::format::OI_NULL,
        }
    }

    /// Writes `rows` as NIFTY's January 2024 at `timeframe` under a fresh
    /// scratch root, and returns the root and the bar file's path.
    fn month_of(name: &str, timeframe: Timeframe, rows: &[Bar]) -> (PathBuf, PathBuf) {
        let root = crate::scratch::path(name);
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("a root");
        let path = StorePath::new(PathParts {
            vendor: Vendor::Gdfl,
            exchange: "NSE",
            segment: "INDEX",
            symbol: "NIFTY",
            contract: None,
            timeframe,
            month: YearMonth::new(2024, 1).expect("a month"),
            file: FileKind::Bars,
        })
        .expect("a path");
        #[expect(clippy::cast_possible_truncation, reason = "the cross-check id")]
        let id = brutex_core::universe::fnv1a("NIFTY") as u32;
        let mut file = BarFile::open_or_create(&root, path, id).expect("a writer");
        file.append(rows).expect("ordered rows");
        drop(file);
        let bars = path.to_path_buf(&root);
        (root, bars)
    }

    /// The index the month's kept summary gives, and the file reopened.
    fn index_of(bars: &Path, timeframe: Timeframe) -> (Result<TodIndex, String>, BarFile, Held) {
        let root = bars
            .ancestors()
            .nth(7)
            .expect("the store root above vendor/exchange/segment/symbol/rung/month")
            .to_path_buf();
        let path = StorePath::new(PathParts {
            vendor: Vendor::Gdfl,
            exchange: "NSE",
            segment: "INDEX",
            symbol: "NIFTY",
            contract: None,
            timeframe,
            month: YearMonth::new(2024, 1).expect("a month"),
            file: FileKind::Bars,
        })
        .expect("a path");
        #[expect(clippy::cast_possible_truncation, reason = "the cross-check id")]
        let id = brutex_core::universe::fnv1a("NIFTY") as u32;
        let file = BarFile::open_existing(&root, path, id).expect("the month opens");
        let held = Held::of(&file);
        let index = match store::month_summary::read_kept(bars, held.n_valid) {
            Ok(summary) => TodIndex::from_summary(
                &summary,
                YearMonth::new(2024, 1).expect("a month"),
                timeframe,
                held,
            ),
            Err(why) => Err(why.to_string()),
        };
        (index, file, held)
    }

    /// Every window of `windows` cut from the index equals the one the
    /// month's `.tix` cuts, and the rank of every second between `first` and
    /// `last` on each day of `days` equals `first_at_or_after`.
    fn agree(index: &TodIndex, file: &BarFile, held: Held, days: &[i64], windows: &[(u32, u32)]) {
        let (lo, hi) = YearMonth::new(2024, 1).expect("m").ist_bounds_micros();
        for &tod in windows {
            let from_index =
                SeriesPlan::day_runs(0, (lo, hi), held, tod, |ts| Ok(index.rows_before(ts)))
                    .expect("cut from the index");
            let from_tix = SeriesPlan::day_runs(0, (lo, hi), held, tod, |ts| {
                file.first_at_or_after(ts).map_err(|why| why.to_string())
            })
            .expect("cut from the .tix");
            assert_eq!(from_index, from_tix, "window {tod:?}");
        }
        for &date in days {
            for second in (8 * 3_600)..(11 * 3_600) {
                let ts = ist(date, 0, 0, second);
                assert_eq!(
                    index.rows_before(ts),
                    file.first_at_or_after(ts).expect("located"),
                    "2024-01-{date} second {second}"
                );
            }
            for ts in [
                ist(date, 0, 0, 0),
                ist(date, 23, 59, 59),
                ist(date + 1, 0, 0, 0),
            ] {
                assert_eq!(
                    index.rows_before(ts),
                    file.first_at_or_after(ts).expect("located")
                );
            }
        }
    }

    /// The windows every test cuts: a whole day, one second, the session's
    /// first and last instants, windows that open or close inside a gap,
    /// windows wholly inside a gap or outside every bar, and a reversed
    /// clock's neighbours.
    const WINDOWS: [(u32, u32); 12] = [
        (0, 86_399),
        (33_300, 33_300),
        (33_299, 33_300),
        (33_305, 33_425),
        (33_600, 33_720),
        (33_610, 33_611),
        (33_720, 33_900),
        (34_000, 36_000),
        (36_001, 86_399),
        (0, 33_299),
        (50_000, 60_000),
        (33_301, 34_501),
    ];

    /// **THE TIME-OF-DAY INDEX CUTS EXACTLY THE WINDOW THE `.tix` CUTS, ON A
    /// ONE-SECOND MONTH WITH GAPS** (R3-07, DB-B57). Rows before any instant
    /// of the month, from the summary alone, equal `first_at_or_after`;
    /// every window of [`WINDOWS`] is the same runs both ways.
    #[test]
    fn the_time_of_day_index_cuts_every_window_the_tix_cuts() {
        let mut rows = Vec::new();
        // 2 Jan: 09:15:00 to 09:24:59, every seventh second missing.
        for second in 0..600 {
            if second % 7 != 3 {
                rows.push(bar(ist(2, 9, 15, second), second));
            }
        }
        // 3 Jan: nothing. 4 Jan: 09:15:00 to 09:40:00, with 09:20 to 09:22
        // missing whole.
        for second in 0..1_501 {
            if !(300..420).contains(&second) {
                rows.push(bar(ist(4, 9, 15, second), second));
            }
        }
        assert!(rows.len() >= 1_024, "a month the writer summarises");
        let (_root, bars) = month_of("r307-tod-1s", Timeframe::SECOND_1, &rows);
        let (index, file, held) = index_of(&bars, Timeframe::SECOND_1);
        let index = index.expect("a dense one-second month is indexed");
        assert_eq!(index.unit, 1);
        agree(&index, &file, held, &[1, 2, 3, 4, 5, 31], &WINDOWS);
    }

    /// **AND ON A MINUTE MONTH**, whose units are minutes.
    #[test]
    fn the_time_of_day_index_cuts_a_minute_month_the_way_the_tix_does() {
        let mut rows = Vec::new();
        for date in [2_i64, 3, 5, 8, 9, 10, 11] {
            for minute in 0..375 {
                if minute % 11 != 5 && !(date == 5 && (100..160).contains(&minute)) {
                    rows.push(bar(ist(date, 9, 15 + minute, 0), minute));
                }
            }
        }
        assert!(rows.len() >= 1_024);
        let (_root, bars) = month_of("r307-tod-1m", Timeframe::MINUTE_1, &rows);
        let (index, file, held) = index_of(&bars, Timeframe::MINUTE_1);
        let index = index.expect("a minute month is indexed");
        assert_eq!(index.unit, 60);
        agree(&index, &file, held, &[2, 4, 5, 11], &WINDOWS);
    }

    /// **A MONTH WITHOUT A USABLE SUMMARY IS NAMED, NEVER INDEXED FROM COUNTS
    /// THAT DO NOT ADD UP.** A sparse one-second month (one bar in three
    /// seconds) keeps no seconds by the density rule; a month under 1,024
    /// bars keeps no summary at all. Both refuse with the reason, and the
    /// window locates them through the `.tix`.
    #[test]
    fn a_month_without_a_usable_summary_is_refused_by_name() {
        let sparse: Vec<Bar> = (0..1_200)
            .map(|n| bar(ist(2, 9, 15, 0) + n * 3 * MICROS, n))
            .collect();
        let (_root, bars) = month_of("r307-tod-sparse", Timeframe::SECOND_1, &sparse);
        let why = index_of(&bars, Timeframe::SECOND_1)
            .0
            .expect_err("no seconds kept");
        assert!(why.contains("keeps no seconds"), "{why}");
        let small: Vec<Bar> = (0..50).map(|n| bar(ist(2, 9, 15, n), n)).collect();
        let (_root, bars) = month_of("r307-tod-small", Timeframe::SECOND_1, &small);
        let why = index_of(&bars, Timeframe::SECOND_1)
            .0
            .expect_err("no summary under 1,024 bars");
        assert!(!why.is_empty(), "the reason is named");
    }

    /// **A FULL WEIGHED MEMO DROPS ITS OLDER HALF, NEVER EVERYTHING, AND A
    /// VALUE IN USE SURVIVES** (DB-B57). The sort indexes' fence and cut
    /// memos, and the time-of-day indexes, are held by weight: before, one
    /// value past the budget emptied the whole map.
    #[test]
    fn a_full_weighed_memo_turns_a_generation_and_keeps_what_is_in_use() {
        let memo: Weighed<u64, u64> = Weighed::new(1_000);
        for key in 0..10 {
            memo.put(key, key, 100);
        }
        assert_eq!(memo.turns(), 1, "the young half filled once");
        assert!(
            memo.bytes() <= 1_000,
            "held {} past the budget",
            memo.bytes()
        );
        // A KEY USED SINCE THE TURN SURVIVES THE NEXT ONE.
        assert_eq!(memo.get(&0), Some(0));
        for key in 10..15 {
            memo.put(key, key, 100);
        }
        assert_eq!(memo.get(&0), Some(0), "a key in use survives a turn");
        assert!(
            memo.bytes() <= 1_000,
            "held {} past the budget",
            memo.bytes()
        );
        // NEVER EMPTIED WHOLE: the newest entries are always held.
        assert_eq!(memo.get(&14), Some(14));
        // ONE VALUE HEAVIER THAN HALF THE BUDGET IS NOT HELD.
        memo.put(99, 99, 600);
        assert_eq!(memo.get(&99), None);
        assert_eq!(memo.get(&14), Some(14), "and turns nothing out for itself");
    }
}
