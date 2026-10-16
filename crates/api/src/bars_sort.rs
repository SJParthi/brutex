//! A value-ordered window page served from the months' `.srt` sort indexes,
//! not by reading every bar. D-3349, WP-39; DB-N14.
//!
//! # What this replaces
//!
//! A window ordered by a price, the volume or the open interest streamed every
//! kept row (D-3326): 43,942,544 bars for one-second NIFTY over 97 months, 4.3
//! to 7.4 s and 8 to 12 GB a request on the measurement that found it (DB-N14),
//! and since D-3326 a refusal past [`super::MAX_SCAN_BARS`] rows. The operator
//! could not sort the series by close at all.
//!
//! # The shape of the answer
//!
//! Each held month's `.srt` holds runs, and each run orders its rows by
//! `(value, time)` per column (`store::sort_index`). Every run fully inside the
//! request's kept rows is one SORTED ARRAY here. A global page at offset `k`
//! over those arrays is:
//!
//! 1. the VALUE of the `k`-th row, by a bisection over the value domain whose
//!    every step counts, per array, the rows at or below a value — from the
//!    array's fence table in memory, and a read of one block only for the
//!    arrays the fences cannot decide;
//! 2. its INSTANT, among the rows that hold that value, by a bisection over
//!    time across those rows' slices of each array (ties in value are ordered
//!    by time, then by the series' place in the request, in both directions —
//!    the order every sort of the grid has kept since D-3326);
//! 3. the page, merged forward from that row: one cursor per array, `limit`
//!    rows out, each read from its bar file and checked against the entry
//!    that named it.
//!
//! # A day bound
//!
//! A day bound cuts only the edge months of a range. A run it cuts is served
//! from its OWN entries: those stamped inside the bound are exactly the kept
//! rows it orders, already in order, so they are read once from the run's
//! column (eight or twelve bytes a row, checked block by block), held for the
//! request — and between requests within a budget — and searched in memory.
//! No bar is read to order them, and no depth is refused (`Piece::Cut`).
//!
//! # What is not in an index, and what it costs
//!
//! Rows no run covers — a month with no `.srt` yet, rows a writer appended
//! after the last run, a month whose index failed when it was read, every row
//! under a time-of-day bound — are READ, once. Their count is capped at
//! [`super::MAX_SCAN_BARS`]; up to [`MAX_READ_HELD`] of them are ordered
//! whole, so any page of them is served; past that a page within
//! [`super::MAX_SORT_DEPTH`] of the front is served, or of the end when no row
//! is indexed, and a deeper one is refused `sort_too_deep` by name. The answer
//! names them — how many rows, in which months, why, and the `cli srt-build`
//! command that indexes them (`sort_index` on the wire). Nothing is read
//! silently.
//!
//! A month whose index fails while it is read — a block that fails its
//! checksum — is set aside until its index changes, and the page is asked
//! again with its rows read and named: never served from the damaged index,
//! never a refusal of the whole range on its account.
//!
//! # Cost, stated
//!
//! With every month indexed: `O(m)` header and stamp reads for `m` runs, the
//! fence tables (memoised by the file's stamp), `O(m · log V)` in-memory fence
//! searches for a value domain of `V`, a handful of block reads per run near
//! the answer, then `limit` row reads and their predecessors. No bar outside
//! the page is read. Measured on the shadow store (WP-39, release build,
//! sequential GETs, n = 200 a shape): one-second NIFTY over 97 months
//! (43,942,544 bars), every column, both directions, first, middle and last
//! page, `extremes=1`: p50 2.1-3.3 ms, p99 under 5 ms on a quiet machine, the
//! first request after a start up to about 100 ms. `docs/06-limits.md` is
//! WP-99's to carry.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock, PoisonError};

use store::format::{Bar, OI_NULL};
use store::path::YearMonth;
use store::sort_index::{Column, ColumnInfo, Entry, FENCE, Fences, Shape, SortIndex, Walked, Why};

use super::series::{self, Position, SeriesPlan, Stamp};
use super::{
    Behind, Cancel, Extreme, Extremes, MAX_SCAN_BARS, MAX_SORT_DEPTH, SortKey, WindowBar,
    WindowQuery, behind_of, change, window_bar,
};
use crate::fault_log::FaultLog;

/// The column a value sort orders on, or `None` for time and time of day.
#[must_use]
pub(super) const fn column_of(sort: SortKey) -> Option<Column> {
    match sort {
        SortKey::Open => Some(Column::Open),
        SortKey::High => Some(Column::High),
        SortKey::Low => Some(Column::Low),
        SortKey::Close => Some(Column::Close),
        SortKey::Volume => Some(Column::Volume),
        SortKey::OpenInterest => Some(Column::OpenInterest),
        SortKey::Ts | SortKey::TimeOfDay => None,
    }
}

/// The most reasons the answer names one by one; the counts are always whole.
const MAX_NAMED: usize = 12;

/// The most rows no index orders that one request orders WHOLE, so any page
/// of them is served however deep (D-3349): forty bytes a row, 40 MB at the
/// bound. Past it a page is served within [`MAX_SORT_DEPTH`] of the front, or
/// of the end when no row is indexed, and refused `sort_too_deep` by name
/// deeper. A five-minute time-of-day window over the 97 months of one-second
/// NIFTY keeps 590,181 rows: every page of it is served.
pub(super) const MAX_READ_HELD: u64 = 1_000_000;

#[cfg(test)]
thread_local! {
    /// [`MAX_READ_HELD`], lowered by a test on its own thread so the paths
    /// past it are driven without writing a million bars.
    pub(super) static READ_HELD: std::cell::Cell<u64> = const { std::cell::Cell::new(MAX_READ_HELD) };
}

/// [`MAX_READ_HELD`], or a test's own bound.
fn read_held() -> u64 {
    #[cfg(test)]
    {
        READ_HELD.with(std::cell::Cell::get)
    }
    #[cfg(not(test))]
    {
        MAX_READ_HELD
    }
}

/// Bytes of fence tables the process keeps between requests. Emptied whole
/// when full, as every memo here is: a forgotten table costs one read.
const FENCE_BUDGET: usize = 64 * 1024 * 1024;

/// Bytes of day-cut entries (see [`Piece::Cut`]) the process keeps between
/// requests: about six one-second months of one column. A forgotten list
/// costs one read of its run's column.
const CUT_BUDGET: usize = 48 * 1024 * 1024;

/// What one request's value sort used and read, for the wire.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SortReport {
    /// The column ordered on.
    pub column: &'static str,
    /// Kept rows served by a sort index.
    pub indexed_rows: u64,
    /// Index runs those rows lie in.
    pub indexed_runs: usize,
    /// Of those, runs a day bound cuts, served from their own entries
    /// stamped inside the bound (D-3349); their rows are among
    /// `indexed_rows`.
    pub cut_runs: usize,
    /// Kept rows read because no run orders them.
    pub scanned_rows: u64,
    /// Held months with any such row.
    pub scanned_months: usize,
    /// Up to [`MAX_NAMED`] of those months, each with its rows and why.
    pub scanned_why: Vec<String>,
    /// The command that indexes the months a reader had to read, when one
    /// would help.
    pub build: Vec<String>,
    /// Every kept row is null in this column — open interest of an index or
    /// a share (D-3322) — so the page is the rows in time order, served by
    /// seeking, with nothing read beyond it.
    pub by_time: bool,
    /// Kept rows that were to be read and were NOT: their month would not
    /// open (another process holds it), or the record would not read. They
    /// are in neither `indexed_rows` nor `scanned_rows`, the page is ordered
    /// without them, and the extremes are withheld.
    pub unread_rows: u64,
    /// Held months with any such row.
    pub unread_months: usize,
    /// Up to [`MAX_NAMED`] of those months, each with its rows and why.
    pub unread_why: Vec<String>,
}

/// What a value sort answers.
pub(super) struct Sorted {
    /// The page.
    pub(super) page: Vec<WindowBar>,
    /// The extremes, when asked for.
    pub(super) extremes: Option<Extremes>,
    /// Why the extremes were asked for and are not served.
    pub(super) extremes_why: Option<String>,
    /// Kept rows read beyond the page.
    pub(super) scanned: u64,
    /// What it used.
    pub(super) report: SortReport,
}

/// Why a month under the small-month threshold with no `.srt` is read: the
/// rule's, not a fault, and no rebuild would index it (D-4933).
const SMALL_MONTH: &str = "it holds fewer than 1,024 bars, so by the small-month rule it has \
                           no .srt and its rows are ordered here (D-4933)";

// The sentence above names the store's threshold in words; a change to the
// threshold must change the sentence, so the build refuses the two drifting.
const _: () = assert!(store::time_index::INDEX_MIN_ROWS == 1_024);

/// Why rows are read rather than served by an index.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Unindexed {
    /// A time-of-day bound keeps windows of days; no run is per window.
    TimeOfDay,
    /// The month's `.srt` is absent or refused, for this reason.
    NoIndex(String),
    /// The index orders rows `0..coverage` and these lie past it: appended
    /// after it, or — when the walk found bytes that are not a run — behind
    /// a tear, named.
    PastIndex { coverage: u64, torn: Option<String> },
    /// A day bound cuts through a run whose entries would not serve the
    /// stretch it keeps, for this reason (see [`Piece::Cut`]).
    CutUnserved(String),
}

impl Unindexed {
    fn sentence(&self) -> String {
        match self {
            Self::TimeOfDay => "a time-of-day bound keeps windows no index run is cut to".into(),
            Self::NoIndex(why) => why.clone(),
            Self::PastIndex {
                coverage,
                torn: None,
            } => {
                format!("appended after its sort index, which orders rows 0..{coverage}")
            }
            // A TORN INDEX IS SAID TO BE TORN: these rows were not appended
            // after it; the bytes that would have ordered them are not a run.
            Self::PastIndex {
                coverage,
                torn: Some(why),
            } => format!(
                "its sort index is torn ({why}) and orders only rows 0..{coverage}; a rebuild \
                 writes it whole"
            ),
            Self::CutUnserved(why) => {
                format!("a day bound cuts a sort index run whose entries would not serve it: {why}")
            }
        }
    }

    /// Whether `cli srt-build` would index these rows: every reason but a
    /// time-of-day bound, which no index run is cut to, and a month under the
    /// small-month threshold, which the rule gives no `.srt` (D-4933). A cut
    /// run that would not serve failed a checksum or disagrees with its bars,
    /// and a rebuild writes it again.
    fn buildable(&self) -> bool {
        match self {
            Self::TimeOfDay => false,
            Self::NoIndex(why) => why != SMALL_MONTH,
            Self::PastIndex { .. } | Self::CutUnserved(_) => true,
        }
    }
}

/// A memoised walk of one month's `.srt`, bound to its bar file.
struct View {
    path: PathBuf,
    shape: Shape,
    walked: Walked,
}

/// Every month's walk, by the `.srt`'s stamp and the bar file's.
static VIEWS: series::Memo<(PathBuf, Stamp, Stamp), Arc<Result<View, String>>> =
    series::Memo::new();

/// One run's fence table for one column, by the `.srt`'s path and stamp.
type FenceKey = (PathBuf, Stamp, usize, Column);

/// A month's walk as one request holds it, and the `.srt`'s stamp.
type Held = (Arc<Result<View, String>>, Stamp);

/// Every month's walk one request loaded, by series and slot.
type Views = HashMap<(usize, usize), Held>;

/// A process-wide memo held within a byte budget, emptied whole when a value
/// would pass it.
struct Weighed<K, V> {
    map: OnceLock<Mutex<(usize, HashMap<K, V>)>>,
    budget: usize,
}

impl<K: Eq + std::hash::Hash, V: Clone> Weighed<K, V> {
    const fn new(budget: usize) -> Self {
        Self {
            map: OnceLock::new(),
            budget,
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, (usize, HashMap<K, V>)> {
        self.map
            .get_or_init(|| Mutex::new((0, HashMap::with_capacity(1_024))))
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }

    fn get(&self, key: &K) -> Option<V> {
        self.lock().1.get(key).cloned()
    }

    /// Holds `value`, which weighs `weight` bytes; one heavier than the whole
    /// budget is not held.
    fn put(&self, key: K, value: V, weight: usize) {
        if weight > self.budget {
            return;
        }
        let mut held = self.lock();
        if held.0.saturating_add(weight) > self.budget {
            held.1.clear();
            held.0 = 0;
        }
        held.0 = held.0.saturating_add(weight);
        held.1.insert(key, value);
    }
}

/// Fence tables kept between requests, within [`FENCE_BUDGET`].
static FENCE_TABLES: Weighed<FenceKey, Arc<Fences>> = Weighed::new(FENCE_BUDGET);

/// A day-cut run's entries for one column: the `.srt`'s path and stamp, the
/// run, the column, and the bound `[lo, hi)`.
type CutKey = (PathBuf, Stamp, usize, Column, i64, i64);

/// The entries a day bound keeps of one run's column, and how many of them
/// are nulls.
type CutEntries = Arc<(Arc<[Entry]>, u64)>;

/// Day-cut entries kept between requests, within [`CUT_BUDGET`].
static CUT_ENTRIES: Weighed<CutKey, CutEntries> = Weighed::new(CUT_BUDGET);

/// Every month's `.srt` whose index failed while a page read it — a block or
/// fence table that fails its checksum — by its path and stamp, with why. It
/// is not used again until it changes (a rebuild): its rows are read, named
/// with the command that rebuilds it.
static BROKEN: series::Memo<(PathBuf, Stamp), Arc<str>> = series::Memo::new();

/// The most months one request sets aside because their index failed while
/// it was read, before it is refused instead.
const MAX_BROKEN: usize = 8;

/// The months ONE REQUEST sets aside because the HOST refused their index
/// while the page read it — the `.srt`'s path and stamp, and why. A host's
/// refusal (permission, too many open files, a file gone) says nothing about
/// the file's bytes and moves no stamp when it passes, so it is never put in
/// [`BROKEN`], where it would outlive its cause until the server restarted:
/// the month's rows are read for this request, and the next request asks the
/// host again.
type Aside = Vec<(PathBuf, Stamp, Arc<str>)>;

/// The extremes of rows `[start, end)` of a month file that is not a whole
/// month — a stretch a day bound cuts — by the file's path and stamp.
static CUT_EXTREMES: series::Memo<(PathBuf, Stamp, u64, u64), Extremes> = series::Memo::new();

/// The `.srt` of `slot` in `plan`, walked and bound to its bar file, from the
/// memo when neither file changed.
fn view(
    root: &Path,
    plan: &mut SeriesPlan,
    slot: usize,
    timeframe_secs: u32,
    aside: &Aside,
) -> Result<Held, String> {
    let Some(at) = plan.slots.get(slot) else {
        return Err(format!("slot {slot} is not a month of this window"));
    };
    let (bars_path, bars_stamp, held, month) = (at.path.clone(), at.stamp, at.held, at.month);
    let path = bars_path.with_extension("srt");
    // NEVER THROUGH A LINK, as every door of the store refuses one below the
    // root: the name itself is asked (`symlink_metadata`), and the open below
    // refuses a final link in the kernel, so a link put there between the two
    // is refused as well. A followed link would serve another file's order
    // as this month's whenever that file passed the bind.
    let meta = match std::fs::symlink_metadata(&path) {
        Ok(meta) => meta,
        // THE SMALL-MONTH RULE (D-4933): a month under 1,024 bars is given no
        // `.srt`, so its rows are ordered here and no command is offered for
        // it — `cli srt-build` would answer `small` and write nothing.
        Err(why)
            if why.kind() == std::io::ErrorKind::NotFound
                && store::time_index::below_threshold(held.n_valid) =>
        {
            return Err(SMALL_MONTH.to_owned());
        }
        Err(why) if why.kind() == std::io::ErrorKind::NotFound => {
            return Err(Why::Absent.to_string());
        }
        Err(why) => return Err(format!("{}: {why}", path.display())),
    };
    if !meta.is_file() {
        let what = if meta.file_type().is_symlink() {
            "a symbolic link; no link is followed"
        } else {
            "not a regular file"
        };
        return Err(Why::Unreadable(format!("{} is {what}", path.display())).to_string());
    }
    let stamp = Stamp::of(&meta);
    if let Some(why) = BROKEN.get(&(path.clone(), stamp)) {
        return Err(format!(
            "its .srt sort index failed when it was read and is not used until it is rebuilt \
             ({why})"
        ));
    }
    if let Some((_, _, why)) = aside
        .iter()
        .find(|(at, as_of, _)| *at == path && *as_of == stamp)
    {
        return Err(format!(
            "the host refused its .srt sort index while this request read it, so its rows are \
             read; the next request asks again ({why})"
        ));
    }
    let key = (path.clone(), stamp, bars_stamp);
    if let Some(found) = VIEWS.get(&key) {
        return Ok((found, stamp));
    }
    let symbol_id = symbol_id(&plan.key.symbol);
    let shape = Shape::of(month, timeframe_secs, symbol_id);
    let mut bar_refused = false;
    let bound = SortIndex::open(&path, &shape).and_then(|index| {
        let walked = index.walked().clone();
        store::sort_index::bind(
            &walked.runs,
            held.n_valid,
            held.first_ts,
            held.last_ts,
            |row| {
                plan.read(root, slot, row)
                    .map(|bar| bar.ts_micros)
                    .map_err(String::from)
                    .inspect_err(|_| bar_refused = true)
            },
        )?;
        Ok(View {
            path: path.clone(),
            shape,
            walked,
        })
    });
    // ONLY WHAT THE BYTES SAY IS REMEMBERED. The memo's key is the two
    // files' stamps (size, mtime, inode), so a verdict read from their bytes
    // stays true until a stamp moves. A refusal by the HOST — permission, an
    // I/O error, too many open files, a file gone since the stat — says
    // nothing about the bytes and changes no stamp when it passes: held, it
    // made the month unindexed until the server restarted, and the printed
    // `cli srt-build` answered `kept`. It is asked again by the next request.
    let passing = bar_refused || matches!(bound, Err(Why::Absent | Why::Unreadable(_)));
    let found = Arc::new(bound.map_err(|why| why.to_string()));
    if !passing {
        VIEWS.put(key, Arc::clone(&found));
    }
    Ok((found, stamp))
}

/// The header symbol id a bar file of `symbol` carries, folded as
/// `pull::ingest` folds it.
fn symbol_id(symbol: &str) -> u32 {
    let [a, b, c, d, ..] = brutex_core::universe::fnv1a(symbol).to_le_bytes();
    u32::from_le_bytes([a, b, c, d])
}

/// One stretch of a kept run: ordered by one index run, cut from one, or
/// read.
#[derive(Debug, Clone)]
enum Piece {
    /// Rows `[start, end)` of the slot, ordered by `.srt` run `run`.
    Indexed {
        slot: usize,
        run: usize,
        start: u64,
        end: u64,
    },
    /// Rows `[start, end)` of the slot: the part of `.srt` run `run` a day
    /// bound keeps, which are its entries stamped inside `[lo, hi)`. A day
    /// bound cuts only the edge months of a range, so a request holds at
    /// most two of these per series. The entries are read from the run's
    /// column whole, filtered, and are already in order (D-3349): no bar is
    /// read to order them, and no depth is refused.
    Cut {
        slot: usize,
        run: usize,
        start: u64,
        end: u64,
        lo: i64,
        hi: i64,
        /// The kept entries and their nulls, once read for the request's
        /// column.
        entries: Option<CutEntries>,
    },
    /// Rows `[start, end)` of the slot, read.
    Read {
        slot: usize,
        start: u64,
        end: u64,
        why: Unindexed,
    },
}

/// A kept run of a series, cut into pieces.
struct Cut {
    series: usize,
    /// Index into the plan's runs.
    run: usize,
    pieces: Vec<Piece>,
}

/// Cuts every kept run of every series into indexed and read pieces.
fn cut(
    root: &Path,
    plans: &mut [SeriesPlan],
    query: &WindowQuery,
    views: &mut Views,
    aside: &Aside,
) -> Vec<Cut> {
    let timeframe_secs = query.timeframe.secs();
    let mut out = Vec::new();
    for (series, plan) in plans.iter_mut().enumerate() {
        for run_at in 0..plan.runs.len() {
            let Some(run) = plan.runs.get(run_at).copied() else {
                continue;
            };
            let mut pieces = Vec::new();
            if query.filter.tod.is_some() {
                pieces.push(Piece::Read {
                    slot: run.slot,
                    start: run.start,
                    end: run.end,
                    why: Unindexed::TimeOfDay,
                });
                out.push(Cut {
                    series,
                    run: run_at,
                    pieces,
                });
                continue;
            }
            let loaded = if let Some(held) = views.get(&(series, run.slot)).cloned() {
                Ok(held)
            } else {
                let found = view(root, plan, run.slot, timeframe_secs, aside);
                if let Ok(held) = &found {
                    views.insert((series, run.slot), held.clone());
                }
                found
            };
            let walked = match &loaded {
                Ok((found, _)) => match found.as_ref() {
                    Ok(view) => Ok(&view.walked),
                    Err(why) => Err(why.clone()),
                },
                Err(why) => Err(why.clone()),
            };
            match walked {
                Err(why) => pieces.push(Piece::Read {
                    slot: run.slot,
                    start: run.start,
                    end: run.end,
                    why: Unindexed::NoIndex(why),
                }),
                Ok(walked) => {
                    let mut at = run.start;
                    for (index, srt) in walked.runs.iter().enumerate() {
                        if srt.end <= run.start || srt.start >= run.end {
                            continue;
                        }
                        // THE RUNS CHAIN FROM ROW 0 with no gap (the walk
                        // refuses one), so the first run met holds `at` and
                        // each later one starts where the last piece ended.
                        if srt.start >= run.start && srt.end <= run.end {
                            pieces.push(Piece::Indexed {
                                slot: run.slot,
                                run: index,
                                start: srt.start,
                                end: srt.end,
                            });
                            at = srt.end;
                        } else {
                            // A DAY BOUND CUTS THIS RUN: its entries stamped
                            // inside the bound are exactly the kept rows it
                            // orders, because the kept run is every row of the
                            // month inside `[lo, hi)`.
                            let from = srt.start.max(at);
                            let until = srt.end.min(run.end);
                            if until > from {
                                pieces.push(Piece::Cut {
                                    slot: run.slot,
                                    run: index,
                                    start: from,
                                    end: until,
                                    lo: run.lo,
                                    hi: run.hi,
                                    entries: None,
                                });
                                at = until;
                            }
                        }
                    }
                    if at < run.end {
                        pieces.push(Piece::Read {
                            slot: run.slot,
                            start: at,
                            end: run.end,
                            why: Unindexed::PastIndex {
                                coverage: walked.coverage(),
                                torn: walked.torn.as_ref().map(|(at, why)| {
                                    format!("the bytes from {at} are not a run: {why}")
                                }),
                            },
                        });
                    }
                }
            }
            out.push(Cut {
                series,
                run: run_at,
                pieces,
            });
        }
    }
    out
}

/// The order every value page is in: nulls last in both directions, then
/// the value (`vkey` is `!v` descending, which reverses `i64` order exactly),
/// then the instant, then the series' place in the request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct Key {
    null: bool,
    vkey: i64,
    ts: i64,
    series: u32,
}

/// The order key of value `v`.
const fn vkey(v: i64, desc: bool) -> i64 {
    if desc { !v } else { v }
}

/// A row read from outside every index run, kept while it is among the
/// `offset + limit` best.
#[derive(Debug, Clone, Copy)]
struct Kept {
    key: Key,
    run: usize,
    row: u64,
}

/// One index run as a sorted array of this request.
struct RunArray {
    series: u32,
    slot: usize,
    run: usize,
    rows: u64,
    nulls: u64,
    info: ColumnInfo,
    origin: i64,
    path: PathBuf,
    stamp: Stamp,
    view: Arc<Result<View, String>>,
    file: Option<SortIndex>,
    fences: Option<Arc<Fences>>,
    blocks: HashMap<u64, Arc<[Entry]>>,
    column: Column,
    /// A day-cut run's kept entries, in order, held whole: `entry` reads
    /// them and no fence or block is read (see [`Piece::Cut`]).
    held: Option<Arc<[Entry]>>,
    /// The series and month, and the command that rebuilds the month's
    /// index, for a refusal to name.
    named: (Arc<str>, Arc<str>),
    /// Why reading this run's index failed, once it has: the page is then
    /// asked again with the month read instead (see [`page`]). The flag is
    /// whether the HOST refused it (see [`Aside`]) rather than its bytes
    /// failing a check.
    failure: Option<(String, bool)>,
}

impl RunArray {
    /// Records that reading this run's index failed for `why`, and says so,
    /// naming the month and the command that rebuilds it.
    fn fail(&mut self, why: &Why) -> String {
        let host = matches!(why, Why::Absent | Why::Unreadable(_));
        let said = if host {
            format!("{}: {why}", self.named.0)
        } else {
            format!("{}: {why}; rebuild it with: {}", self.named.0, self.named.1)
        };
        self.failure = Some((said.clone(), host));
        said
    }

    fn view(&self) -> Result<&View, String> {
        self.view.as_ref().as_ref().map_err(Clone::clone)
    }

    fn file(&mut self) -> Result<&SortIndex, String> {
        if self.file.is_none() {
            let view = self.view()?;
            let opened = SortIndex::open_walked(&view.path, &view.shape, view.walked.clone());
            let opened = opened.map_err(|why| self.fail(&why))?;
            self.file = Some(opened);
        }
        self.file
            .as_ref()
            .ok_or_else(|| "the sort index did not open".to_owned())
    }

    fn fences(&mut self) -> Result<Arc<Fences>, String> {
        if let Some(held) = &self.fences {
            return Ok(Arc::clone(held));
        }
        let key = (self.path.clone(), self.stamp, self.run, self.column);
        let fences = if let Some(found) = FENCE_TABLES.get(&key) {
            found
        } else {
            let (run, column) = (self.run, self.column);
            let read = self.file()?.fences(run, column);
            let read = Arc::new(read.map_err(|why| self.fail(&why))?);
            FENCE_TABLES.put(key, Arc::clone(&read), read.weight());
            read
        };
        self.fences = Some(Arc::clone(&fences));
        Ok(fences)
    }

    fn entry(&mut self, at: u64) -> Result<Entry, String> {
        if let Some(held) = &self.held {
            return usize::try_from(at)
                .ok()
                .and_then(|at| held.get(at).copied())
                .ok_or_else(|| format!("entry {at} is past the cut run's {}", held.len()));
        }
        let block = at / FENCE;
        let held = if let Some(held) = self.blocks.get(&block) {
            Arc::clone(held)
        } else {
            let fences = self.fences()?;
            let (run, column) = (self.run, self.column);
            let read = self.file()?.block(run, column, block, &fences);
            let read: Arc<[Entry]> = read.map_err(|why| self.fail(&why))?.into();
            self.blocks.insert(block, Arc::clone(&read));
            read
        };
        usize::try_from(at % FENCE)
            .ok()
            .and_then(|i| held.get(i).copied())
            .ok_or_else(|| format!("entry {at} is not in its block"))
    }

    /// Where the first entry of `[lo, hi)` for which `pred` holds lies,
    /// bounded by the fence table alone: `[lower, upper]`.
    fn bounds(
        &mut self,
        lo: u64,
        hi: u64,
        pred: &dyn Fn(Entry) -> bool,
    ) -> Result<(u64, u64), String> {
        if lo >= hi {
            return Ok((hi, hi));
        }
        if self.held.is_some() {
            // IN MEMORY: the search below is over the entries themselves.
            return Ok((lo, hi));
        }
        let fences = self.fences()?;
        let (jlo, jhi) = (lo.div_ceil(FENCE), hi.div_ceil(FENCE));
        let (mut a, mut b) = (jlo, jhi);
        while a < b {
            let mid = a + (b - a) / 2;
            let fence = usize::try_from(mid)
                .ok()
                .and_then(|i| fences.entries.get(i).copied())
                .ok_or_else(|| format!("fence {mid} is missing"))?;
            if pred(fence) {
                b = mid;
            } else {
                a = mid + 1;
            }
        }
        let upper = if a < jhi { a * FENCE } else { hi };
        let lower = if a > jlo { (a - 1) * FENCE + 1 } else { lo };
        Ok((lower.min(upper), upper))
    }

    /// The first entry of `[lo, hi)` for which `pred` holds, or `hi`: the
    /// fence table, then one block at most.
    fn partition(&mut self, lo: u64, hi: u64, pred: &dyn Fn(Entry) -> bool) -> Result<u64, String> {
        let (mut a, mut b) = self.bounds(lo, hi, pred)?;
        while a < b {
            let mid = a + (b - a) / 2;
            if pred(self.entry(mid)?) {
                b = mid;
            } else {
                a = mid + 1;
            }
        }
        Ok(a)
    }

    fn ts(&self, entry: Entry) -> i64 {
        entry.ts(self.origin)
    }

    /// Non-null entries whose value is at or below `x` (`le`) or below it.
    fn count_v(&mut self, x: i64, le: bool, exact: bool) -> Result<(u64, u64), String> {
        let pred = move |entry: Entry| if le { entry.v > x } else { entry.v >= x };
        let (lo, hi) = (self.nulls, self.rows);
        if exact {
            let at = self.partition(lo, hi, &pred)?;
            Ok((at - lo, at - lo))
        } else {
            let (a, b) = self.bounds(lo, hi, &pred)?;
            Ok((a - lo, b - lo))
        }
    }

    /// Non-null entries whose ORDER KEY is at or below `x`: bounds, or exact.
    fn count_vkey_le(&mut self, x: i64, desc: bool, exact: bool) -> Result<(u64, u64), String> {
        if desc {
            // !v <= x  <=>  v >= !x
            let (a, b) = self.count_v(!x, false, exact)?;
            let valued = self.rows - self.nulls;
            Ok((valued - b, valued - a))
        } else {
            self.count_v(x, true, exact)
        }
    }

    /// The non-null rows holding value `v`: `[first, end)`.
    fn group(&mut self, v: i64) -> Result<(u64, u64), String> {
        let (lo, hi) = (self.nulls, self.rows);
        let first = self.partition(lo, hi, &|entry: Entry| entry.v >= v)?;
        let end = self.partition(first, hi, &|entry: Entry| entry.v > v)?;
        Ok((first, end))
    }

    /// The first entry of `[lo, hi)` — sorted by time there — at or after
    /// `(ts, series)` in the page's order.
    fn at_or_after(&mut self, lo: u64, hi: u64, ts: i64, series: u32) -> Result<u64, String> {
        let (origin, own) = (self.origin, self.series);
        self.partition(lo, hi, &move |entry: Entry| {
            let at = entry.ts(origin);
            at > ts || (at == ts && own >= series)
        })
    }
}

/// The kept rows of a series whose open interest is not applicable, under
/// an open-interest sort: every one is a null, ordered by time.
#[derive(Debug, Clone, Copy)]
struct TimeArray {
    series: u32,
    run: usize,
    slot: usize,
    start: u64,
    end: u64,
}

/// One slice of rows that share a value (or are all null), sorted by time,
/// that the `j`-th row of the group is chosen from.
#[derive(Debug, Clone, Copy)]
enum Slice {
    Run { array: usize, lo: u64, hi: u64 },
    Time { array: usize, lo: u64, hi: u64 },
    Kept { lo: usize, hi: usize },
}

/// Everything one request selects its page from.
struct Arrays {
    runs: Vec<RunArray>,
    times: Vec<TimeArray>,
    kept: Vec<Kept>,
    desc: bool,
}

impl Arrays {
    /// Non-null rows across every array, the read ones counted whole.
    fn valued(&self, kept_valued: u64) -> u64 {
        self.runs
            .iter()
            .map(|run| run.rows - run.nulls)
            .sum::<u64>()
            .saturating_add(kept_valued)
    }

    /// The order-key domain of the non-null rows.
    fn domain(&self) -> Option<(i64, i64)> {
        let mut lo: Option<i64> = None;
        let mut hi: Option<i64> = None;
        let mut take = |a: i64, b: i64| {
            lo = Some(lo.map_or(a, |held| held.min(a)));
            hi = Some(hi.map_or(b, |held| held.max(b)));
        };
        for run in &self.runs {
            if run.rows > run.nulls {
                let (a, b) = (
                    vkey(run.info.base, self.desc),
                    vkey(run.info.max, self.desc),
                );
                take(a.min(b), a.max(b));
            }
        }
        for kept in self.kept.iter().filter(|kept| !kept.key.null) {
            take(kept.key.vkey, kept.key.vkey);
        }
        lo.zip(hi)
    }

    /// Read rows kept with an order key at or below `x`, non-null.
    fn kept_le(&self, x: i64) -> u64 {
        let (mut lo, mut hi) = (0_usize, self.kept.len());
        while lo < hi {
            let mid = lo + (hi - lo) / 2;
            let le = self
                .kept
                .get(mid)
                .is_some_and(|kept| !kept.key.null && kept.key.vkey <= x);
            if le {
                lo = mid + 1;
            } else {
                hi = mid;
            }
        }
        lo as u64
    }

    /// Whether more than `k` non-null rows hold an order key at or below `x`.
    fn more_than(&mut self, x: i64, k: u64) -> Result<bool, String> {
        let kept = self.kept_le(x);
        let mut bounds = Vec::with_capacity(self.runs.len());
        let (mut lb, mut ub) = (kept, kept);
        for run in &mut self.runs {
            if run.rows == run.nulls {
                bounds.push((0, 0));
                continue;
            }
            let (a, b) = run.count_vkey_le(x, self.desc, false)?;
            lb += a;
            ub += b;
            bounds.push((a, b));
        }
        if lb > k {
            return Ok(true);
        }
        if ub <= k {
            return Ok(false);
        }
        let mut exact = kept;
        for (run, (a, b)) in self.runs.iter_mut().zip(bounds) {
            exact += if a == b {
                a
            } else {
                run.count_vkey_le(x, self.desc, true)?.0
            };
        }
        Ok(exact > k)
    }

    /// Exact count of non-null rows with an order key at or below `x`.
    fn count_le(&mut self, x: i64) -> Result<u64, String> {
        let mut total = self.kept_le(x);
        for run in &mut self.runs {
            if run.rows > run.nulls {
                total += run.count_vkey_le(x, self.desc, true)?.0;
            }
        }
        Ok(total)
    }

    /// The first and last instant of a slice, and how many of its rows lie at
    /// or before `t`.
    fn slice_count(
        &mut self,
        root: &Path,
        plans: &mut [SeriesPlan],
        slice: Slice,
        t: i64,
    ) -> Result<u64, String> {
        match slice {
            Slice::Run { array, lo, hi } => {
                let run = self.runs.get_mut(array).ok_or("no such sorted array")?;
                let origin = run.origin;
                let at = run.partition(lo, hi, &move |entry: Entry| entry.ts(origin) > t)?;
                Ok(at - lo)
            }
            Slice::Time { array, lo, hi } => {
                let time = *self.times.get(array).ok_or("no such time array")?;
                let plan = plans
                    .get_mut(time.series as usize)
                    .ok_or("no such series")?;
                let row = plan.locate(root, time.slot, t.saturating_add(1))?;
                Ok(row.clamp(lo, hi) - lo)
            }
            Slice::Kept { lo, hi } => {
                let (mut a, mut b) = (lo, hi);
                while a < b {
                    let mid = a + (b - a) / 2;
                    if self.kept.get(mid).is_some_and(|kept| kept.key.ts <= t) {
                        a = mid + 1;
                    } else {
                        b = mid;
                    }
                }
                Ok((a - lo) as u64)
            }
        }
    }

    /// The instant and series of a slice's row `at` (its own position).
    fn slice_at(
        &mut self,
        root: &Path,
        plans: &mut [SeriesPlan],
        slice: Slice,
        at: u64,
    ) -> Result<(i64, u32), String> {
        match slice {
            Slice::Run { array, lo, .. } => {
                let run = self.runs.get_mut(array).ok_or("no such sorted array")?;
                let entry = run.entry(lo + at)?;
                Ok((run.ts(entry), run.series))
            }
            Slice::Time { array, lo, .. } => {
                let time = *self.times.get(array).ok_or("no such time array")?;
                let plan = plans
                    .get_mut(time.series as usize)
                    .ok_or("no such series")?;
                let bar = plan.read(root, time.slot, lo + at)?;
                Ok((bar.ts_micros, time.series))
            }
            Slice::Kept { lo, .. } => {
                let kept = usize::try_from(at)
                    .ok()
                    .and_then(|at| self.kept.get(lo + at))
                    .ok_or("no such kept row")?;
                Ok((kept.key.ts, kept.key.series))
            }
        }
    }

    /// The `j`-th row, by `(instant, series)`, of the union of `slices`, or
    /// `None` when they hold `j` rows or fewer.
    fn select_in(
        &mut self,
        root: &Path,
        plans: &mut [SeriesPlan],
        slices: &[(Slice, u64)],
        j: u64,
    ) -> Result<Option<(i64, u32)>, String> {
        if slices.iter().map(|&(_, len)| len).sum::<u64>() <= j {
            return Ok(None);
        }
        let mut ends = Vec::with_capacity(slices.len());
        let (mut tlo, mut thi) = (i64::MAX, i64::MIN);
        for &(slice, len) in slices {
            if len == 0 {
                ends.push(None);
                continue;
            }
            let first = self.slice_at(root, plans, slice, 0)?.0;
            let last = self.slice_at(root, plans, slice, len - 1)?.0;
            tlo = tlo.min(first);
            thi = thi.max(last);
            ends.push(Some((first, last)));
        }
        if tlo > thi {
            return Ok(None);
        }
        let count = |this: &mut Self, plans: &mut [SeriesPlan], t: i64| -> Result<u64, String> {
            let mut total = 0;
            for (&(slice, len), end) in slices.iter().zip(&ends) {
                let Some((first, last)) = *end else {
                    continue;
                };
                total += if t < first {
                    0
                } else if t >= last {
                    len
                } else {
                    this.slice_count(root, plans, slice, t)?
                };
            }
            Ok(total)
        };
        // The smallest instant with more than `j` rows at or before it.
        let (mut lo, mut hi) = (tlo, thi);
        while lo < hi {
            let mid = lo + (hi - lo) / 2;
            if count(self, plans, mid)? > j {
                hi = mid;
            } else {
                lo = mid + 1;
            }
        }
        let before = count(self, plans, lo.saturating_sub(1))?;
        // THE ROWS AT THAT INSTANT, BY SERIES: at most one per series — a
        // run's slice holds one series, the read rows' slice several — so a
        // short list.
        let mut at: Vec<u32> = Vec::new();
        for (&(slice, _), end) in slices.iter().zip(&ends) {
            let Some((first, last)) = *end else {
                continue;
            };
            if lo < first || lo > last {
                continue;
            }
            let upto = self.slice_count(root, plans, slice, lo)?;
            let below = self.slice_count(root, plans, slice, lo.saturating_sub(1))?;
            for row in below..upto {
                let (_, series) = self.slice_at(root, plans, slice, row)?;
                at.push(series);
            }
        }
        at.sort_unstable();
        let offset = usize::try_from(j.saturating_sub(before)).unwrap_or(usize::MAX);
        let series = at
            .get(offset)
            .copied()
            .ok_or_else(|| format!("no row {offset} at instant {lo}"))?;
        Ok(Some((lo, series)))
    }
}

/// Where a chosen row lives, to read it.
#[derive(Debug, Clone, Copy)]
enum Found {
    /// A run's entry: its series' slot, its instant and the value it names.
    Run {
        series: u32,
        slot: usize,
        ts: i64,
        value: Option<i64>,
    },
    /// A time array's row, already read.
    Time {
        series: u32,
        run: usize,
        row: u64,
        bar: Bar,
    },
    /// A read row.
    Kept { series: u32, run: usize, row: u64 },
}

/// A cursor over one array, in the page's order from a starting key.
enum Cursor {
    /// A run's non-null entries, ascending: `[p, end)`.
    Up { array: usize, p: u64 },
    /// A run's non-null entries, descending by value and ascending by time
    /// within a value: the group `[g0, g1)` from `p`.
    Down {
        array: usize,
        g0: u64,
        g1: u64,
        p: u64,
    },
    /// A run's null entries, by time: `[p, nulls)`.
    Nulls { array: usize, p: u64 },
    /// A time array's rows: `[row, end)`.
    Time { array: usize, row: u64 },
    /// The read rows: from `i`.
    Kept { i: usize },
}

impl Arrays {
    /// The cursor's current row, or `None` when it is spent.
    fn head(
        &mut self,
        root: &Path,
        plans: &mut [SeriesPlan],
        cursor: &Cursor,
    ) -> Result<Option<(Key, Found)>, String> {
        let desc = self.desc;
        match *cursor {
            Cursor::Up { array, p } | Cursor::Down { array, p, .. } => {
                let run = self.runs.get_mut(array).ok_or("no such sorted array")?;
                let end = match *cursor {
                    Cursor::Down { g1, .. } => g1,
                    _ => run.rows,
                };
                if p >= end {
                    return Ok(None);
                }
                let entry = run.entry(p)?;
                let ts = run.ts(entry);
                Ok(Some((
                    Key {
                        null: false,
                        vkey: vkey(entry.v, desc),
                        ts,
                        series: run.series,
                    },
                    Found::Run {
                        series: run.series,
                        slot: run.slot,
                        ts,
                        value: Some(entry.v),
                    },
                )))
            }
            Cursor::Nulls { array, p } => {
                let run = self.runs.get_mut(array).ok_or("no such sorted array")?;
                if p >= run.nulls {
                    return Ok(None);
                }
                let entry = run.entry(p)?;
                let ts = run.ts(entry);
                Ok(Some((
                    Key {
                        null: true,
                        vkey: 0,
                        ts,
                        series: run.series,
                    },
                    Found::Run {
                        series: run.series,
                        slot: run.slot,
                        ts,
                        value: None,
                    },
                )))
            }
            Cursor::Time { array, row } => {
                let time = *self.times.get(array).ok_or("no such time array")?;
                if row >= time.end {
                    return Ok(None);
                }
                let plan = plans
                    .get_mut(time.series as usize)
                    .ok_or("no such series")?;
                let bar = plan.read(root, time.slot, row)?;
                Ok(Some((
                    Key {
                        null: true,
                        vkey: 0,
                        ts: bar.ts_micros,
                        series: time.series,
                    },
                    Found::Time {
                        series: time.series,
                        run: time.run,
                        row,
                        bar,
                    },
                )))
            }
            Cursor::Kept { i } => Ok(self.kept.get(i).map(|kept| {
                (
                    kept.key,
                    Found::Kept {
                        series: kept.key.series,
                        run: kept.run,
                        row: kept.row,
                    },
                )
            })),
        }
    }

    /// Steps the cursor past its current row.
    fn advance(&mut self, cursor: &mut Cursor) -> Result<(), String> {
        match cursor {
            Cursor::Up { p, .. } | Cursor::Nulls { p, .. } => *p += 1,
            Cursor::Time { row, .. } => *row += 1,
            Cursor::Kept { i } => *i += 1,
            Cursor::Down { array, g0, g1, p } => {
                *p += 1;
                if *p >= *g1 {
                    let run = self.runs.get_mut(*array).ok_or("no such sorted array")?;
                    if *g0 <= run.nulls {
                        *p = *g1;
                        return Ok(());
                    }
                    // THE NEXT SMALLER VALUE'S GROUP, read forward by time.
                    let v = run.entry(*g0 - 1)?.v;
                    let start = run.partition(run.nulls, *g0, &move |entry: Entry| entry.v >= v)?;
                    *g1 = *g0;
                    *g0 = start;
                    *p = start;
                }
            }
        }
        Ok(())
    }
}

impl Arrays {
    /// The rows holding order key `vk` (value `v`), as time-sorted slices:
    /// one per run that holds any, and the read rows that do.
    fn value_slices(&mut self, vk: i64, v: i64) -> Result<Vec<(Slice, u64)>, String> {
        let mut slices = Vec::new();
        for array in 0..self.runs.len() {
            let Some(run) = self.runs.get_mut(array) else {
                continue;
            };
            if run.rows == run.nulls {
                continue;
            }
            let (lo, hi) = run.group(v)?;
            if hi > lo {
                slices.push((Slice::Run { array, lo, hi }, hi - lo));
            }
        }
        let (mut a, mut b) = (0_usize, self.kept.len());
        while a < b {
            let mid = a + (b - a) / 2;
            if self
                .kept
                .get(mid)
                .is_some_and(|kept| !kept.key.null && kept.key.vkey < vk)
            {
                a = mid + 1;
            } else {
                b = mid;
            }
        }
        let first = a;
        let mut end = first;
        while self
            .kept
            .get(end)
            .is_some_and(|kept| !kept.key.null && kept.key.vkey == vk)
        {
            end += 1;
        }
        if end > first {
            slices.push((Slice::Kept { lo: first, hi: end }, (end - first) as u64));
        }
        Ok(slices)
    }

    /// Every null row, as time-sorted slices.
    fn null_slices(&self) -> Vec<(Slice, u64)> {
        let mut slices = Vec::new();
        for array in 0..self.runs.len() {
            if let Some(run) = self.runs.get(array)
                && run.nulls > 0
            {
                slices.push((
                    Slice::Run {
                        array,
                        lo: 0,
                        hi: run.nulls,
                    },
                    run.nulls,
                ));
            }
        }
        for (array, time) in self.times.iter().enumerate() {
            if time.end > time.start {
                slices.push((
                    Slice::Time {
                        array,
                        lo: time.start,
                        hi: time.end,
                    },
                    time.end - time.start,
                ));
            }
        }
        let first = self.kept.iter().take_while(|kept| !kept.key.null).count();
        if self.kept.len() > first {
            slices.push((
                Slice::Kept {
                    lo: first,
                    hi: self.kept.len(),
                },
                (self.kept.len() - first) as u64,
            ));
        }
        slices
    }
}

/// The page's rows: `limit` from global position `k`, as `Found`s.
fn select(
    root: &Path,
    plans: &mut [SeriesPlan],
    arrays: &mut Arrays,
    kept_valued: u64,
    (k, limit): (u64, usize),
) -> Result<Vec<Found>, String> {
    let Some(start) = start_key(root, plans, arrays, kept_valued, k)? else {
        return Ok(Vec::new());
    };
    let mut cursors = cursors_at(root, plans, arrays, start)?;
    merge(root, plans, arrays, &mut cursors, limit)
}

/// The key of the row at global position `k`: the value by a bisection over
/// the value domain, then the instant among the rows that hold it. `None`
/// past every row the arrays hold.
fn start_key(
    root: &Path,
    plans: &mut [SeriesPlan],
    arrays: &mut Arrays,
    kept_valued: u64,
    k: u64,
) -> Result<Option<Key>, String> {
    let desc = arrays.desc;
    let valued = arrays.valued(kept_valued);
    let start = if k < valued {
        let Some((lo, hi)) = arrays.domain() else {
            return Err("no valued row to order".to_owned());
        };
        let (mut a, mut b) = (lo, hi);
        while a < b {
            let mid = i64::try_from((i128::from(a) + i128::from(b)).div_euclid(2))
                .map_err(|_| "the value bisection left i64".to_owned())?;
            if arrays.more_than(mid, k)? {
                b = mid;
            } else {
                a = mid + 1;
            }
        }
        let vk = a;
        let before = if vk == i64::MIN {
            0
        } else {
            arrays.count_le(vk - 1)?
        };
        let v = if desc { !vk } else { vk };
        let slices = arrays.value_slices(vk, v)?;
        let Some((ts, series)) = arrays.select_in(root, plans, &slices, k - before)? else {
            return Err(format!(
                "the value bisection found {vk} for row {k} and no row holds it"
            ));
        };
        Key {
            null: false,
            vkey: vk,
            ts,
            series,
        }
    } else {
        let slices = arrays.null_slices();
        // PAST EVERY ROW THE ARRAYS HOLD: rows that did not read are not in
        // a read stretch's order, so the page past them is empty.
        let Some((ts, series)) = arrays.select_in(root, plans, &slices, k - valued)? else {
            return Ok(None);
        };
        Key {
            null: true,
            vkey: 0,
            ts,
            series,
        }
    };
    Ok(Some(start))
}

/// One cursor per array, at its first row at or after `start` in the page's
/// order.
fn cursors_at(
    root: &Path,
    plans: &mut [SeriesPlan],
    arrays: &mut Arrays,
    start: Key,
) -> Result<Vec<Cursor>, String> {
    let desc = arrays.desc;
    let mut cursors = Vec::with_capacity(arrays.runs.len() * 2 + arrays.times.len() + 1);
    for array in 0..arrays.runs.len() {
        let Some(run) = arrays.runs.get_mut(array) else {
            continue;
        };
        let own = run.series;
        if !start.null && run.rows > run.nulls {
            let v = if desc { !start.vkey } else { start.vkey };
            let (ts, series) = (start.ts, start.series);
            if desc {
                let (g0, g1) = run.group(v)?;
                let p = run.at_or_after(g0, g1, ts, series)?;
                // NOTHING LEFT AT THIS VALUE IN THIS ARRAY: step from the
                // group's last place, which moves the cursor to the next
                // smaller value's group (or spends it).
                let mut cursor = Cursor::Down {
                    array,
                    g0,
                    g1,
                    p: if p >= g1 { g1.saturating_sub(1) } else { p },
                };
                if p >= g1 {
                    arrays.advance(&mut cursor)?;
                }
                cursors.push(cursor);
            } else {
                let origin = run.origin;
                let p = run.partition(run.nulls, run.rows, &move |entry: Entry| {
                    let at = entry.ts(origin);
                    entry.v > v || (entry.v == v && (at > ts || (at == ts && own >= series)))
                })?;
                cursors.push(Cursor::Up { array, p });
            }
        }
        if let Some(run) = arrays.runs.get_mut(array)
            && run.nulls > 0
        {
            let p = if start.null {
                run.at_or_after(0, run.nulls, start.ts, start.series)?
            } else {
                0
            };
            cursors.push(Cursor::Nulls { array, p });
        }
    }
    for (array, time) in arrays.times.clone().iter().enumerate() {
        let row = if start.null {
            let probe = if time.series >= start.series {
                start.ts
            } else {
                start.ts.saturating_add(1)
            };
            let plan = plans
                .get_mut(time.series as usize)
                .ok_or("no such series")?;
            plan.locate(root, time.slot, probe)?
                .clamp(time.start, time.end)
        } else {
            time.start
        };
        cursors.push(Cursor::Time { array, row });
    }
    let mut a = 0_usize;
    let mut b = arrays.kept.len();
    while a < b {
        let mid = a + (b - a) / 2;
        if arrays.kept.get(mid).is_some_and(|kept| kept.key < start) {
            a = mid + 1;
        } else {
            b = mid;
        }
    }
    cursors.push(Cursor::Kept { i: a });
    Ok(cursors)
}

/// The merge of every cursor: the smallest head, `limit` times.
fn merge(
    root: &Path,
    plans: &mut [SeriesPlan],
    arrays: &mut Arrays,
    cursors: &mut [Cursor],
    limit: usize,
) -> Result<Vec<Found>, String> {
    let mut heads: Vec<Option<(Key, Found)>> = Vec::with_capacity(cursors.len());
    for cursor in cursors.iter() {
        heads.push(arrays.head(root, plans, cursor)?);
    }
    let mut out = Vec::with_capacity(limit);
    while out.len() < limit {
        let mut best: Option<(usize, Key)> = None;
        for (at, head) in heads.iter().enumerate() {
            if let Some((key, _)) = head
                && best.is_none_or(|(_, held)| *key < held)
            {
                best = Some((at, *key));
            }
        }
        let Some((at, _)) = best else {
            break;
        };
        if let Some(Some((_, found))) = heads.get(at) {
            out.push(*found);
        }
        if let Some(cursor) = cursors.get_mut(at) {
            arrays.advance(cursor)?;
            let next = arrays.head(root, plans, cursor)?;
            if let Some(slot) = heads.get_mut(at) {
                *slot = next;
            }
        }
    }
    Ok(out)
}

/// Cuts `kept` to its `keep` best by `order`, in place and without
/// allocating, and returns the worst of those kept — the bar a later row must
/// beat to be kept. Leaves `kept` as it is when it holds no more.
fn keep_best(
    kept: &mut Vec<Kept>,
    keep: usize,
    order: impl Fn(&Kept, &Kept) -> std::cmp::Ordering,
) -> Option<Kept> {
    if keep > 0 && kept.len() > keep {
        kept.select_nth_unstable_by(keep - 1, &order);
        kept.truncate(keep);
        return kept.last().copied();
    }
    None
}

/// One series' kept-run index for a slot, under no time-of-day bound: a
/// month is one kept run.
fn run_of_slot(plan: &SeriesPlan, slot: usize) -> Option<usize> {
    let (mut lo, mut hi) = (0_usize, plan.runs.len());
    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        match plan.runs.get(mid).map(|run| run.slot.cmp(&slot)) {
            Some(std::cmp::Ordering::Less) => lo = mid + 1,
            Some(std::cmp::Ordering::Equal) => return Some(mid),
            _ => hi = mid,
        }
    }
    None
}

/// The extremes of a run, renamed to `series`, volume only where it trades.
fn run_extremes(x: &store::sort_index::RunExtremes, series: usize, volume: bool) -> Extremes {
    let at = |held: store::sort_index::Extreme| {
        Some(Extreme {
            v: held.v,
            t: held.ts,
            series,
        })
    };
    Extremes {
        range: x.range_max.v,
        volume: if volume { x.volume_max.v } else { 0 },
        close_max: at(x.close_max),
        close_min: at(x.close_min),
        high_max: at(x.high_max),
        low_min: at(x.low_min),
        range_max: at(x.range_max),
        volume_max: if volume { at(x.volume_max) } else { None },
    }
}

/// The entries a day bound keeps of `.srt` run `run` for `column` — every
/// entry stamped inside `[lo, hi)` — from the memo, or read from the run's
/// column whole and filtered (D-3349). `rows` is how many kept rows the bars
/// hold there; an index that disagrees is refused by name, never served.
///
/// # Errors
///
/// The month's index did not walk, a block or fence table failed its
/// checksum, or the count disagrees with the bars.
fn cut_entries(
    held: &Held,
    run: usize,
    column: Column,
    (lo, hi): (i64, i64),
    rows: u64,
) -> Result<CutEntries, String> {
    let (found, stamp) = held;
    let view = found.as_ref().as_ref().map_err(Clone::clone)?;
    let key = (view.path.clone(), *stamp, run, column, lo, hi);
    if let Some(found) = CUT_ENTRIES.get(&key) {
        return Ok(found);
    }
    let index = SortIndex::open_walked(&view.path, &view.shape, view.walked.clone())
        .map_err(|why| why.to_string())?;
    let fence_key = (view.path.clone(), *stamp, run, column);
    let fences = if let Some(found) = FENCE_TABLES.get(&fence_key) {
        found
    } else {
        let read = Arc::new(index.fences(run, column).map_err(|why| why.to_string())?);
        FENCE_TABLES.put(fence_key, Arc::clone(&read), read.weight());
        read
    };
    let origin = view.shape.origin_micros;
    let (entries, nulls) = index
        .entries_where(run, column, &fences, |entry| {
            let at = entry.ts(origin);
            at >= lo && at < hi
        })
        .map_err(|why| why.to_string())?;
    if entries.len() as u64 != rows {
        return Err(format!(
            "its run holds {} entries inside the day bound and the bars hold {rows} rows there",
            entries.len()
        ));
    }
    let weight = entries.len().saturating_mul(std::mem::size_of::<Entry>());
    let value: CutEntries = Arc::new((entries.into(), nulls));
    CUT_ENTRIES.put(key, Arc::clone(&value), weight);
    Ok(value)
}

/// Whether the extremes of rows `[start, end)` of `slot` are already held.
fn cut_extremes_held(plan: &SeriesPlan, slot: usize, start: u64, end: u64) -> bool {
    plan.slots.get(slot).is_some_and(|at| {
        CUT_EXTREMES
            .get(&(at.path.clone(), at.stamp, start, end))
            .is_some()
    })
}

/// The extremes of rows `[start, end)` of `slot` — a stretch a day bound cuts,
/// whose run summary covers more than it keeps — from the memo, or read and
/// then held when every row read. Returns them, renamed to `series`, and the
/// rows read (none from the memo). Rows that would not read are left in the
/// plan's unread ledger, one entry a month.
///
/// # Errors
///
/// `cancelled`, by name.
fn cut_extremes(
    root: &Path,
    plan: &mut SeriesPlan,
    slot: usize,
    (start, end): (u64, u64),
    (series, volume): (usize, bool),
    cancel: &Cancel,
    since: &mut u64,
) -> Result<(Extremes, u64), String> {
    let key = plan
        .slots
        .get(slot)
        .map(|at| (at.path.clone(), at.stamp, start, end));
    if let Some(held) = key.as_ref().and_then(|key| CUT_EXTREMES.get(key)) {
        let mut out = Extremes::default();
        out.merge(&held, Some(series));
        return Ok((out, 0));
    }
    let mut month = Extremes::default();
    // A ROW THAT WILL NOT READ IS THE PLAN'S TO NAME, once a month (see
    // `SeriesPlan::read_stretch`); a fold that missed one is never held.
    let missed = plan.read_stretch(root, slot, (start, end), cancel, since, |_, bar| {
        month.fold(bar, 0, volume);
    })?;
    if missed == 0
        && let Some(key) = key
    {
        CUT_EXTREMES.put(key, month);
    }
    let mut out = Extremes::default();
    out.merge(&month, Some(series));
    Ok((out, (end - start).saturating_sub(missed)))
}

/// The build command for months that had no usable index.
fn build_command(
    query: &WindowQuery,
    key: &super::SeriesKey,
    from: YearMonth,
    to: YearMonth,
) -> String {
    let contract = key
        .contract
        .map_or_else(String::new, |contract| format!("/{contract}"));
    format!(
        "cli srt-build {} {}/{}/{}{contract} {} {from} {to}",
        query.vendor.as_str(),
        key.exchange,
        key.segment,
        key.symbol,
        query.timeframe.as_str(),
    )
}

/// One value-ordered page. See the module header.
///
/// # Errors
///
/// `scan_budget_exceeded` or `sort_too_deep` for rows no index orders past
/// the caps, each naming the months and the command that indexes them;
/// `cancelled` when the client went away; more than [`MAX_BROKEN`] months
/// whose index failed while it was read.
///
/// # A month whose index fails while it is read
///
/// A block or fence table that fails its checksum stops the attempt; the
/// month is set aside ([`BROKEN`], until its index changes) and the page is
/// asked again with that month's rows read — counted, named, and given the
/// command that rebuilds it, like a month with no index. Never served from
/// the damaged index, and never refused for the whole range on its account.
pub(super) fn page(
    root: &Path,
    plans: &mut [SeriesPlan],
    rules: &[change::Rules],
    query: &WindowQuery,
    (offset, limit): (usize, usize),
    faults: &mut FaultLog,
    cancel: &Cancel,
) -> Result<Sorted, String> {
    let mut set_aside = Vec::new();
    let mut aside = Aside::new();
    for _ in 0..=MAX_BROKEN {
        let mut tried = FaultLog::default();
        match page_once(
            root,
            plans,
            rules,
            query,
            (offset, limit),
            &mut tried,
            (cancel, &aside),
        ) {
            Ok(sorted) => {
                faults.absorb(tried);
                return Ok(sorted);
            }
            Err(Attempt::Refused(why)) => return Err(why),
            Err(Attempt::Broken {
                path,
                stamp,
                why,
                host,
            }) => {
                // BYTES THAT FAIL A CHECK are remembered until the file
                // changes; a refusal by the host is this request's alone.
                if host {
                    aside.push((path, stamp, Arc::from(why.as_str())));
                } else {
                    BROKEN.put((path, stamp), Arc::from(why.as_str()));
                }
                set_aside.push(why);
            }
        }
    }
    Err(format!(
        "more than {MAX_BROKEN} months' sort indexes failed while this page read them; each is \
         set aside until it is rebuilt: {}",
        set_aside.join("; ")
    ))
}

/// Why one attempt at a value page did not answer.
enum Attempt {
    /// Refused, in these words: the request's answer.
    Refused(String),
    /// The month whose index is at `path` (as it was at `stamp`) failed while
    /// it was read, for `why`.
    Broken {
        path: PathBuf,
        stamp: Stamp,
        why: String,
        /// The host refused the file, rather than its bytes failing a check.
        host: bool,
    },
}

impl From<String> for Attempt {
    fn from(why: String) -> Self {
        Self::Refused(why)
    }
}

/// One attempt at [`page`]. See there.
#[expect(
    clippy::too_many_lines,
    reason = "one request's plan, budget, read and selection, in the order the \
              module header states them; split, each part would carry the \
              same eight arguments"
)]
fn page_once(
    root: &Path,
    plans: &mut [SeriesPlan],
    rules: &[change::Rules],
    query: &WindowQuery,
    (offset, limit): (usize, usize),
    faults: &mut FaultLog,
    (cancel, aside): (&Cancel, &Aside),
) -> Result<Sorted, Attempt> {
    let Some(column) = column_of(query.sort) else {
        return Err(Attempt::Refused(
            "a value page needs a value column".to_owned(),
        ));
    };
    let desc = query.desc;
    // ONE ATTEMPT, ONE COUNT: a page asked again (see `page`) starts its
    // unread ledger again.
    for plan in plans.iter_mut() {
        plan.clear_unread();
    }
    let mut views = HashMap::with_capacity(plans.iter().map(|plan| plan.slots.len()).sum());
    let mut cuts = cut(root, plans, query, &mut views, aside);
    cancel.check()?;
    let null_series = |series: usize| {
        column == Column::OpenInterest
            && rules
                .get(series)
                .is_none_or(change::Rules::not_a_derivative)
    };
    // THE RUNS A DAY BOUND CUTS, from their own entries (D-3349). One that
    // would not serve is read instead, and the answer says why.
    for cut in &mut cuts {
        if null_series(cut.series) {
            continue;
        }
        for piece in &mut cut.pieces {
            let Piece::Cut {
                slot,
                run,
                start,
                end,
                lo,
                hi,
                ..
            } = *piece
            else {
                continue;
            };
            cancel.check()?;
            let loaded = views
                .get(&(cut.series, slot))
                .ok_or_else(|| "the month's sort index was not walked".to_owned())
                .and_then(|held| cut_entries(held, run, column, (lo, hi), end - start));
            match loaded {
                Ok(found) => {
                    if let Piece::Cut { entries, .. } = piece {
                        *entries = Some(found);
                    }
                }
                Err(why) => {
                    *piece = Piece::Read {
                        slot,
                        start,
                        end,
                        why: Unindexed::CutUnserved(why),
                    };
                }
            }
        }
    }
    // THE BUDGET, BEFORE ANY ROW IS READ.
    let mut report = SortReport {
        column: column.as_str(),
        ..SortReport::default()
    };
    let mut read_rows = 0_u64;
    let mut sort_read = 0_u64;
    let mut named: Vec<(usize, YearMonth, u64, String, bool)> = Vec::new();
    for cut in &cuts {
        for piece in &cut.pieces {
            match piece {
                Piece::Indexed { start, end, .. } => {
                    report.indexed_rows += end - start;
                    report.indexed_runs += 1;
                }
                Piece::Cut { start, end, .. } => {
                    report.indexed_rows += end - start;
                    report.indexed_runs += 1;
                    report.cut_runs += 1;
                }
                Piece::Read {
                    slot,
                    start,
                    end,
                    why,
                } => {
                    read_rows += end - start;
                    if !null_series(cut.series) {
                        sort_read += end - start;
                    }
                    let month = plans
                        .get(cut.series)
                        .and_then(|plan| plan.month_of_slot(*slot));
                    if let Some(month) = month {
                        match named
                            .iter_mut()
                            .find(|held| held.0 == cut.series && held.1 == month)
                        {
                            Some(held) => held.2 += end - start,
                            None => named.push((
                                cut.series,
                                month,
                                end - start,
                                why.sentence(),
                                why.buildable(),
                            )),
                        }
                    }
                }
            }
        }
    }
    report.scanned_rows = read_rows;
    report.scanned_months = named.len();
    for (series, month, rows, why, _) in named.iter().take(MAX_NAMED) {
        let key = plans
            .get(*series)
            .map_or_else(String::new, |plan| plan.key.text());
        report
            .scanned_why
            .push(format!("{key} {month}: {rows} rows read: {why}"));
    }
    for (series, plan) in plans.iter().enumerate() {
        let months: Vec<YearMonth> = named
            .iter()
            .filter(|held| held.0 == series && held.4)
            .map(|held| held.1)
            .collect();
        if let (Some(&from), Some(&to)) = (months.iter().min(), months.iter().max())
            && report.build.len() < 4
        {
            report.build.push(build_command(query, &plan.key, from, to));
        }
    }
    let command = || {
        if report.build.is_empty() {
            String::new()
        } else {
            format!("; index them with: {}", report.build.join(" ; "))
        }
    };
    if read_rows > MAX_SCAN_BARS {
        return Err(Attempt::Refused(format!(
            "scan_budget_exceeded: ordering by {} serves {} kept rows from sort indexes and \
             would read the other {read_rows}, past the {MAX_SCAN_BARS} one request may read \
             ({}){}",
            column.as_str(),
            report.indexed_rows,
            report.scanned_why.join("; "),
            command()
        )));
    }
    let depth = offset.saturating_add(limit);
    let total: u64 = plans.iter().map(SeriesPlan::total).sum();
    // A PAGE NEAR THE END OF AN ORDER NO INDEX SERVES (D-3349). When every
    // kept row is read — a time-of-day bound, or no month indexed — the page
    // at `offset` is the `total - offset`-th from the end, so a page within
    // `MAX_SORT_DEPTH` of either end keeps that many rows, not `offset +
    // limit`. Mixed with index runs, a read row's place among the read ones
    // does not fix its place in the page, so only the front is served.
    let pure = report.indexed_rows == 0 && cuts.iter().all(|cut| !null_series(cut.series));
    let tail = u64::try_from(offset).map_or(0, |k| total.saturating_sub(k));
    let held_whole = sort_read <= read_held();
    let from_end = !held_whole
        && pure
        && depth > MAX_SORT_DEPTH
        && usize::try_from(tail).is_ok_and(|tail| tail <= MAX_SORT_DEPTH);
    if !held_whole && depth > MAX_SORT_DEPTH && !from_end {
        return Err(Attempt::Refused(format!(
            "sort_too_deep: {sort_read} kept rows no sort index orders are read for this page, \
             more than the {MAX_READ_HELD} one request orders whole, and it keeps offset + limit \
             = {depth} of them, past the {MAX_SORT_DEPTH} this route holds from either end ({}){}",
            report.scanned_why.join("; "),
            command()
        )));
    }
    // THE READ ROWS: kept while among the `offset + limit` best, or, for a
    // page near the end, the `total - offset` last.
    let keep = if from_end {
        usize::try_from(tail).unwrap_or(MAX_SORT_DEPTH)
    } else {
        depth
    }
    .max(1);
    let order = |a: &Kept, b: &Kept| {
        if from_end {
            b.key.cmp(&a.key)
        } else {
            a.key.cmp(&b.key)
        }
    };
    // ONE ALLOCATION, NEVER GROWN: room for twice what is kept, or for every
    // row read when that is fewer; past it the worst half is cut in place.
    let capacity = keep
        .saturating_mul(2)
        .min(usize::try_from(sort_read).unwrap_or(usize::MAX))
        .max(1);
    let mut kept: Vec<Kept> = Vec::with_capacity(capacity);
    let mut worst: Option<Kept> = None;
    let mut joined = 0_u64;
    let mut kept_valued = 0_u64;
    let mut scanned_extremes = Extremes::default();
    let mut since = 0_u64;
    let mut unread_sorted = 0_u64;
    for cut in &cuts {
        let Some(series_rules) = rules.get(cut.series) else {
            continue;
        };
        let volume = !series_rules.index_no_volume();
        let joins = !null_series(cut.series);
        let Ok(series) = u32::try_from(cut.series) else {
            continue;
        };
        for piece in &cut.pieces {
            let Piece::Read {
                slot, start, end, ..
            } = *piece
            else {
                continue;
            };
            let Some(plan) = plans.get_mut(cut.series) else {
                continue;
            };
            // A MONTH THAT WILL NOT OPEN IS NOT TRIED ROW BY ROW, and a row
            // that will not read is counted, not named: one fault a month
            // (see `SeriesPlan::read_stretch`).
            unread_sorted +=
                plan.read_stretch(root, slot, (start, end), cancel, &mut since, |row, bar| {
                    if query.want_extremes {
                        scanned_extremes.fold(bar, cut.series, volume);
                    }
                    if !joins {
                        return;
                    }
                    let v = column.of(bar);
                    let null = column.nullable() && v == OI_NULL;
                    if !null {
                        kept_valued += 1;
                    }
                    joined += 1;
                    let candidate = Kept {
                        key: Key {
                            null,
                            vkey: if null { 0 } else { vkey(v, desc) },
                            ts: bar.ts_micros,
                            series,
                        },
                        run: cut.run,
                        row,
                    };
                    if worst
                        .as_ref()
                        .is_some_and(|worst| order(&candidate, worst).is_ge())
                    {
                        return;
                    }
                    if kept.len() == capacity {
                        worst = keep_best(&mut kept, keep, order);
                    }
                    kept.push(candidate);
                })?;
        }
    }
    keep_best(&mut kept, keep, order);
    // IN PLACE: the keys are unique (an instant holds one row per series), so
    // an unstable sort is the order.
    kept.sort_unstable_by(order);
    if from_end {
        kept.reverse();
    }
    cancel.check()?;
    // THE ARRAYS.
    let mut arrays = Arrays {
        runs: Vec::new(),
        times: Vec::new(),
        kept,
        desc,
    };
    let mut extremes = query.want_extremes.then_some(scanned_extremes);
    for cut in &cuts {
        let Ok(series) = u32::try_from(cut.series) else {
            continue;
        };
        let volume = rules
            .get(cut.series)
            .is_none_or(|series_rules| !series_rules.index_no_volume());
        if null_series(cut.series)
            && let Some(run) = plans
                .get(cut.series)
                .and_then(|plan| plan.runs.get(cut.run))
        {
            arrays.times.push(TimeArray {
                series,
                run: cut.run,
                slot: run.slot,
                start: run.start,
                end: run.end,
            });
        }
        for piece in &cut.pieces {
            let (slot, run, held) = match piece {
                Piece::Indexed { slot, run, .. } => (*slot, *run, None),
                Piece::Cut {
                    slot, run, entries, ..
                } => (*slot, *run, Some(entries)),
                Piece::Read { .. } => continue,
            };
            let Some((found, stamp)) = views.get(&(cut.series, slot)).cloned() else {
                continue;
            };
            let Ok(view) = found.as_ref() else {
                continue;
            };
            let Some(info) = view.walked.runs.get(run).copied() else {
                continue;
            };
            // A WHOLE RUN'S EXTREMES ARE ITS SUMMARY; a cut run's are folded
            // below, since its summary covers rows the bound leaves out.
            if held.is_none()
                && let Some(kept) = extremes.as_mut()
            {
                kept.merge(&run_extremes(&info.extremes, cut.series, volume), None);
            }
            if null_series(cut.series) {
                continue;
            }
            let array = match held {
                None => {
                    let column_info = *info.column(column);
                    (info.rows(), column_info.nulls, column_info, None)
                }
                Some(Some(cut_held)) => {
                    let (entries, nulls) = (&cut_held.0, cut_held.1);
                    let valued = entries.get(usize::try_from(nulls).unwrap_or(usize::MAX)..);
                    let (base, max) = valued
                        .and_then(|valued| valued.first().zip(valued.last()))
                        .map_or((0, 0), |(lo, hi)| (lo.v, hi.v));
                    let column_info = ColumnInfo {
                        nulls,
                        base,
                        max,
                        ..ColumnInfo::default()
                    };
                    (
                        entries.len() as u64,
                        nulls,
                        column_info,
                        Some(Arc::clone(entries)),
                    )
                }
                // NOT LOADED: only a null series' cut runs are left so, and
                // they never reach here.
                Some(None) => continue,
            };
            let (rows, nulls, column_info, held) = array;
            let named = plans.get(cut.series).map_or_else(
                || (Arc::from(""), Arc::from("")),
                |plan| {
                    let month = plan.month_of_slot(slot);
                    let at = month.map_or_else(String::new, |month| month.to_string());
                    let command = month.map_or_else(String::new, |month| {
                        build_command(query, &plan.key, month, month)
                    });
                    (
                        Arc::from(format!("{} {at}", plan.key.text())),
                        Arc::from(command),
                    )
                },
            );
            arrays.runs.push(RunArray {
                series,
                slot,
                run,
                rows,
                nulls,
                info: column_info,
                origin: view.shape.origin_micros,
                path: view.path.clone(),
                stamp,
                view: Arc::clone(&found),
                file: None,
                fences: None,
                blocks: HashMap::with_capacity(4),
                column,
                held,
                named,
                failure: None,
            });
        }
    }
    // THE EXTREMES OF THE CUT RUNS: from the memo, or their kept rows read,
    // under the extremes' own budget; past it the page is served without
    // them, named.
    let mut extremes_why = None;
    let mut extremes_read = 0_u64;
    if extremes.is_some() {
        let mut owed = 0_u64;
        for cut in &cuts {
            for piece in &cut.pieces {
                if let Piece::Cut {
                    slot, start, end, ..
                } = *piece
                    && plans
                        .get(cut.series)
                        .is_some_and(|plan| !cut_extremes_held(plan, slot, start, end))
                {
                    owed = owed.saturating_add(end - start);
                }
            }
        }
        if owed > super::MAX_EXTREMES_SCAN_BARS {
            extremes = None;
            extremes_why = Some(format!(
                "scan_budget_exceeded: the extremes need {owed} records read that no sort index \
                 summarises and no earlier request folded, past the {} one request may fold; the \
                 page is served without them",
                super::MAX_EXTREMES_SCAN_BARS
            ));
        } else {
            let mut since = 0_u64;
            for cut in &cuts {
                let volume = rules
                    .get(cut.series)
                    .is_none_or(|series_rules| !series_rules.index_no_volume());
                for piece in &cut.pieces {
                    let Piece::Cut {
                        slot, start, end, ..
                    } = *piece
                    else {
                        continue;
                    };
                    let Some(plan) = plans.get_mut(cut.series) else {
                        continue;
                    };
                    let (folded, read) = cut_extremes(
                        root,
                        plan,
                        slot,
                        (start, end),
                        (cut.series, volume),
                        cancel,
                        &mut since,
                    )?;
                    extremes_read += read;
                    if let Some(kept) = extremes.as_mut() {
                        kept.merge(&folded, None);
                    }
                }
            }
        }
    }
    // WHAT WOULD NOT READ: one fault a month, the months named on the wire as
    // NOT read, and no extreme served — an extreme folded from the months
    // that did read is not the window's, and would look exactly like it.
    let unread = unread_of(plans, faults, &mut report);
    report.scanned_rows = read_rows.saturating_sub(unread_sorted);
    if let Some(why) = unread.withheld()
        && extremes.is_some()
    {
        extremes = None;
        extremes_why = Some(why);
    }
    let k = u64::try_from(offset).unwrap_or(u64::MAX);
    let chosen = if k >= total {
        Vec::new()
    } else if from_end {
        // THE KEPT ROWS ARE THE ORDER'S LAST: every joined row not kept
        // comes before them.
        let before = usize::try_from(joined)
            .unwrap_or(usize::MAX)
            .saturating_sub(arrays.kept.len());
        arrays
            .kept
            .iter()
            .skip(offset.saturating_sub(before))
            .take(limit)
            .map(|kept| Found::Kept {
                series: kept.key.series,
                run: kept.run,
                row: kept.row,
            })
            .collect()
    } else {
        match select(root, plans, &mut arrays, kept_valued, (k, limit)) {
            Ok(chosen) => chosen,
            Err(why) => {
                // A MONTH'S INDEX FAILED WHILE IT WAS READ: that month is set
                // aside and the page asked again (see `page`).
                return Err(arrays
                    .runs
                    .iter()
                    .find_map(|run| {
                        run.failure.as_ref().map(|(failed, host)| Attempt::Broken {
                            path: run.path.clone(),
                            stamp: run.stamp,
                            why: failed.clone(),
                            host: *host,
                        })
                    })
                    .unwrap_or(Attempt::Refused(why)));
            }
        }
    };
    cancel.check()?;
    // THE PAGE, READ AND CHECKED AGAINST WHAT NAMED IT.
    let mut page = Vec::with_capacity(chosen.len());
    for found in chosen {
        let (series, run, row, bar) = match found {
            Found::Kept { series, run, row } => {
                let Some(plan) = plans.get_mut(series as usize) else {
                    continue;
                };
                let Some(slot) = plan.runs.get(run).map(|at| at.slot) else {
                    continue;
                };
                match plan.read(root, slot, row) {
                    Ok(bar) => (series, run, row, bar),
                    Err(why) => {
                        faults.push_once(why);
                        continue;
                    }
                }
            }
            Found::Time {
                series,
                run,
                row,
                bar,
            } => (series, run, row, bar),
            Found::Run {
                series,
                slot,
                ts,
                value,
            } => {
                let Some(plan) = plans.get_mut(series as usize) else {
                    continue;
                };
                let Some(run) = run_of_slot(plan, slot) else {
                    faults.push(format!("{}: slot {slot} has no kept run", plan.key.text()));
                    continue;
                };
                let row = match plan.locate(root, slot, ts) {
                    Ok(row) => row,
                    Err(why) => {
                        faults.push_once(why);
                        continue;
                    }
                };
                let bar = match plan.read(root, slot, row) {
                    Ok(bar) => bar,
                    Err(why) => {
                        faults.push_once(why);
                        continue;
                    }
                };
                let held = column.of(&bar);
                let agrees = bar.ts_micros == ts
                    && match value {
                        Some(v) => held == v,
                        None => held == OI_NULL,
                    };
                if !agrees {
                    // NEVER SERVED: a sort entry that does not name the bar
                    // it points at is a stale index, and the row it would
                    // have put on the page is named instead.
                    faults.push(format!(
                        "{} {}: the sort index names a {} of {value:?} at {ts} and the bar \
                         there is stamped {} with {held}; rebuild it with {}",
                        plan.key.text(),
                        plan.month_of_slot(slot)
                            .map_or_else(String::new, |month| month.to_string()),
                        column.as_str(),
                        bar.ts_micros,
                        plan.month_of_slot(slot).map_or_else(String::new, |month| {
                            build_command(query, &plan.key, month, month)
                        })
                    ));
                    continue;
                }
                (series, run, row, bar)
            }
        };
        let series = series as usize;
        let (Some(plan), Some(series_rules)) = (plans.get_mut(series), rules.get(series)) else {
            continue;
        };
        let at = Position { count: 0, run, row };
        let behind: Behind = behind_of(root, plan, at, None, query.lookback);
        page.push(window_bar(series_rules, behind, bar, series));
    }
    Ok(Sorted {
        page,
        extremes,
        extremes_why,
        scanned: read_rows.saturating_sub(unread_sorted) + extremes_read,
        report,
    })
}

/// The extremes of every kept row, for a time-ordered page: the summaries of
/// the index runs that cover whole stretches, and the rows no whole run
/// covers — a stretch a day bound cuts, or rows no run orders — read, or
/// answered by the month memo or the cut-stretch memo, under
/// [`super::MAX_EXTREMES_SCAN_BARS`]. Returns the extremes and the rows read.
///
/// # Errors
///
/// The `scan_budget_exceeded` sentence, or `cancelled`.
#[expect(
    clippy::too_many_lines,
    reason = "the cut, the budget over what must be read, then the reads, in \
              the order the doc above states them; each step is the input of \
              the next"
)]
pub(super) fn extremes_of(
    root: &Path,
    plans: &mut [SeriesPlan],
    rules: &[change::Rules],
    query: &WindowQuery,
    faults: &mut FaultLog,
    cancel: &Cancel,
) -> Result<(Extremes, u64), String> {
    for plan in plans.iter_mut() {
        plan.clear_unread();
    }
    let mut views = HashMap::with_capacity(plans.iter().map(|plan| plan.slots.len()).sum());
    let cuts = cut(root, plans, query, &mut views, &Aside::new());
    let mut extremes = Extremes::default();
    let mut owed = 0_u64;
    let mut read: Vec<(usize, usize, u64, u64)> = Vec::new();
    for cut in &cuts {
        let volume = rules
            .get(cut.series)
            .is_none_or(|series_rules| !series_rules.index_no_volume());
        for piece in &cut.pieces {
            let (slot, start, end) = match *piece {
                Piece::Indexed { slot, run, .. } => {
                    let summary = views
                        .get(&(cut.series, slot))
                        .and_then(|(found, _)| found.as_ref().as_ref().ok())
                        .and_then(|view| view.walked.runs.get(run))
                        .map(|info| info.extremes);
                    if let Some(summary) = summary {
                        extremes.merge(&run_extremes(&summary, cut.series, volume), None);
                    }
                    continue;
                }
                // A CUT RUN'S SUMMARY COVERS ROWS THE BOUND LEAVES OUT: its
                // kept rows are folded, or answered by the memo, like a
                // stretch no run orders.
                Piece::Cut {
                    slot, start, end, ..
                }
                | Piece::Read {
                    slot, start, end, ..
                } => (slot, start, end),
            };
            let Some(plan) = plans.get(cut.series) else {
                continue;
            };
            let whole = plan
                .slots
                .get(slot)
                .filter(|at| start == 0 && end == at.held.n_valid);
            let memo = whole.and_then(|at| super::MONTH_EXTREMES.get(&(at.path.clone(), at.stamp)));
            if let Some(held) = memo {
                extremes.merge(&held, Some(cut.series));
            } else if whole.is_none() && cut_extremes_held(plan, slot, start, end) {
                read.push((cut.series, slot, start, end));
            } else {
                owed = owed.saturating_add(end - start);
                read.push((cut.series, slot, start, end));
            }
        }
    }
    if owed > super::MAX_EXTREMES_SCAN_BARS {
        return Err(format!(
            "scan_budget_exceeded: the extremes need {owed} records read that no sort index \
             summarises and no earlier request folded, past the {} one request may fold; the \
             page is served without them",
            super::MAX_EXTREMES_SCAN_BARS
        ));
    }
    let mut since = 0_u64;
    let mut rows = 0_u64;
    for (series, slot, start, end) in read {
        let volume = rules
            .get(series)
            .is_none_or(|series_rules| !series_rules.index_no_volume());
        let Some(plan) = plans.get_mut(series) else {
            continue;
        };
        let whole = plan
            .slots
            .get(slot)
            .is_some_and(|at| start == 0 && end == at.held.n_valid);
        if !whole {
            let (folded, read) = cut_extremes(
                root,
                plan,
                slot,
                (start, end),
                (series, volume),
                cancel,
                &mut since,
            )?;
            rows += read;
            extremes.merge(&folded, None);
            continue;
        }
        let mut month = Extremes::default();
        let missed =
            plan.read_stretch(root, slot, (start, end), cancel, &mut since, |_, bar| {
                month.fold(bar, 0, volume);
            })?;
        rows += (end - start).saturating_sub(missed);
        if missed == 0
            && let Some(at) = plan.slots.get(slot)
        {
            super::MONTH_EXTREMES.put((at.path.clone(), at.stamp), month);
        }
        extremes.merge(&month, Some(series));
    }
    // A MONTH THAT WOULD NOT READ WITHHOLDS THE EXTREMES, by name: what the
    // other months fold to is not the window's extreme.
    let unread = unread_of(plans, faults, &mut SortReport::default());
    match unread.withheld() {
        Some(why) => Err(why),
        None => Ok((extremes, rows)),
    }
}

/// What a request's stretch reads left unread, drained from every plan: one
/// fault a month into `faults`, the first [`MAX_NAMED`] months and the counts
/// into `report`.
fn unread_of(plans: &mut [SeriesPlan], faults: &mut FaultLog, report: &mut SortReport) -> Unread {
    let mut out = Unread::default();
    for plan in plans.iter_mut() {
        let key = plan.key.text();
        let left = plan.take_unread(faults, |line| {
            if out.first.is_none() {
                out.first = Some(line.to_owned());
            }
            if report.unread_why.len() < MAX_NAMED {
                report.unread_why.push(format!("{key} {line}"));
            }
        });
        out.rows = out.rows.saturating_add(left.rows);
        out.months += left.months;
    }
    report.unread_rows = out.rows;
    report.unread_months = out.months;
    out
}

/// What [`unread_of`] drained.
#[derive(Debug, Default)]
struct Unread {
    rows: u64,
    months: usize,
    first: Option<String>,
}

impl Unread {
    /// Why the extremes are withheld, when any row would not read.
    fn withheld(&self) -> Option<String> {
        (self.rows > 0).then(|| {
            format!(
                "months_unread: {} kept rows in {} held month(s) of this window would not read, \
                 so its extremes are not known and none is served (the first: {})",
                self.rows,
                self.months,
                self.first.as_deref().unwrap_or("")
            )
        })
    }
}

#[cfg(test)]
#[path = "bars_sort_tests.rs"]
mod tests;

/// Tests written for mutants and unrun lines a cloud mutation pass found in
/// this file (`reports/cloud-mut-api-a-20261016.md`). Each names its target.
/// Every bar is generated, in a scratch store the test removes.
#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "a fixture that cannot be planted is a test that cannot run, and \
              must fail loudly"
)]
mod cloud_mutant_tests {
    use std::path::PathBuf;

    use brutex_core::instrument::Contract;
    use brutex_core::vendor::Vendor;
    use store::format::{Bar, OI_NULL};
    use store::path::{FileKind, PathParts, StorePath, Timeframe, YearMonth};
    use store::sort_index::{Column, Shape, SortIndex};

    use super::super::{Filter, SeriesKey, SortKey, WindowQuery, window};

    const MICROS: i64 = 1_000_000;
    const CE: &str = "2031-03-27-1000000-CE";

    /// A scratch store root, emptied first.
    fn scratch(tag: &str) -> PathBuf {
        let root = crate::scratch::path(&format!("cloud-sort-{tag}"));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("a scratch root");
        root
    }

    fn option() -> SeriesKey {
        SeriesKey::new("NSE", "FNO", "NIFTY", Contract::parse(CE))
    }

    fn index() -> SeriesKey {
        SeriesKey::new("NSE", "INDEX", "NIFTY", None)
    }

    fn month(m: u8) -> YearMonth {
        YearMonth::new(2031, m).expect("a month")
    }

    /// Bar `i` of a month: three sessions of 120 minutes from 09:15 IST on
    /// the 2nd, 3rd and 4th, a close that wanders, and the open interest
    /// given.
    fn bar(month: YearMonth, i: i64, open_interest: i64) -> Bar {
        let day = 1 + i / 120;
        let ts = month.ist_bounds_micros().0 + (day * 86_400 + 555 * 60 + (i % 120) * 60) * MICROS;
        let close = 10_000 + (i * 7_919 + i64::from(month.month()) * 13) % 997;
        Bar {
            ts_micros: ts,
            open: close,
            high: close + 5,
            low: close - 5,
            close,
            volume: 1,
            open_interest,
        }
    }

    /// Writes `rows` as `key`'s month on GDFL, and plants a sort index of
    /// runs of 130 built from `indexed` — the same rows, or rows a test
    /// altered. Returns the `.srt`'s path.
    fn write(
        root: &std::path::Path,
        key: &SeriesKey,
        month: YearMonth,
        rows: &[Bar],
        indexed: &[Bar],
    ) -> PathBuf {
        let path = StorePath::new(PathParts {
            vendor: Vendor::Gdfl,
            exchange: &key.exchange,
            segment: &key.segment,
            symbol: &key.symbol,
            contract: key.contract,
            timeframe: Timeframe::MINUTE_1,
            month,
            file: FileKind::Bars,
        })
        .expect("a generated path");
        let symbol_id = super::symbol_id(&key.symbol);
        let mut file = store::file::BarFile::open_or_create(root, path, symbol_id)
            .expect("a generated bar file");
        file.append(rows).expect("generated records");
        let bars = path.to_path_buf(root);
        crate::scratch::plant_pre_rule_sort_index(
            &bars,
            &Shape::of(month, 60, symbol_id),
            indexed,
            130,
        );
        bars.with_extension("srt")
    }

    fn query(series: SeriesKey, from: YearMonth, to: YearMonth, sort: SortKey) -> WindowQuery {
        WindowQuery {
            vendor: Vendor::Gdfl,
            timeframe: Timeframe::MINUTE_1,
            series: vec![series],
            from,
            to,
            sort,
            desc: false,
            offset: Some(0),
            limit: 37,
            want_extremes: false,
            at: None,
            filter: Filter::default(),
            lookback: true,
            coverage: false,
        }
    }

    /// **A RUN WHOSE EVERY ROW IS NULL IN THE COLUMN HOLDS NO VALUE TO
    /// SLICE.** February's open interest is null on every bar; January's is a
    /// number. The open-interest page from the first row is January's
    /// smallest, from the indexes, and February's runs are passed over when
    /// the rows holding the chosen value are gathered. Runs `value_slices`'
    /// all-null run, which no test reached.
    #[test]
    fn a_run_whose_every_row_is_null_holds_no_value() {
        let root = scratch("all-null-run");
        let jan: Vec<Bar> = (0..360)
            .map(|i| bar(month(1), i, 40_000 + (i * 31) % 360))
            .collect();
        let feb: Vec<Bar> = (0..360).map(|i| bar(month(2), i, OI_NULL)).collect();
        write(&root, &option(), month(1), &jan, &jan);
        write(&root, &option(), month(2), &feb, &feb);
        let got = window(
            &root,
            &query(option(), month(1), month(2), SortKey::OpenInterest),
        )
        .expect("answers");
        let served: Vec<i64> = got.bars.iter().map(|row| row.bar.open_interest).collect();
        let mut want: Vec<i64> = jan.iter().map(|bar| bar.open_interest).collect();
        want.sort_unstable();
        want.truncate(37);
        assert_eq!(served, want);
        let report = got.sort_index.expect("a value sort says what it used");
        assert_eq!((report.indexed_rows, report.scanned_rows), (720, 0));
        let _ = std::fs::remove_dir_all(&root);
    }

    /// **A CUT RUN WHOSE ENTRIES INSIDE THE BOUND DO NOT COUNT ITS ROWS
    /// THERE IS READ, AND SAYS SO IN NUMBERS.** The index was built from
    /// rows one of which — row 50, inside the first run — is stamped on the
    /// 3rd rather than the 2nd, so the run's first and last stamps still bind
    /// it to the bars. A day bound from the 3rd cuts that run: its entries
    /// inside the bound are 11 and the bars there are 10, so the stretch is
    /// read and named, and the page is the brute force. Runs `cut_entries`'
    /// count refusal, which no test reached.
    #[test]
    fn a_cut_run_whose_entries_miscount_the_bound_is_read() {
        let root = scratch("cut-miscount");
        let rows: Vec<Bar> = (0..360).map(|i| bar(month(1), i, OI_NULL)).collect();
        let mut indexed = rows.clone();
        indexed[50].ts_micros = rows[120].ts_micros + 30 * MICROS;
        write(&root, &index(), month(1), &rows, &indexed);
        let third = month(1).ist_bounds_micros().0 + 2 * 86_400 * MICROS;
        let mut asked = query(index(), month(1), month(1), SortKey::Close);
        asked.filter.from = Some(third);
        let got = window(&root, &asked).expect("answers");
        let mut want: Vec<(i64, i64)> = rows
            .iter()
            .filter(|bar| bar.ts_micros >= third)
            .map(|bar| (bar.close, bar.ts_micros))
            .collect();
        want.sort_unstable();
        want.truncate(37);
        let served: Vec<(i64, i64)> = got
            .bars
            .iter()
            .map(|row| (row.bar.close, row.bar.ts_micros))
            .collect();
        assert_eq!(served, want);
        let report = got.sort_index.expect("a value sort says what it used");
        assert!(
            report.scanned_why.iter().any(|why| why.contains(
                "its run holds 11 entries inside the day bound and the bars hold 10 rows there"
            )),
            "{report:?}"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    /// **MORE THAN `MAX_BROKEN` MONTHS WHOSE INDEXES FAIL WHILE THEY ARE READ
    /// ARE REFUSED BY NAME**, never asked again without end. Ten months, every
    /// run of each with a close block that fails its checksum: each attempt
    /// sets one month aside and asks again, and past the ninth the page is
    /// refused naming the cap. Runs `page`'s refusal, which no test reached.
    #[test]
    fn past_max_broken_failing_indexes_the_page_is_refused() {
        let root = scratch("max-broken");
        for m in 1..=10 {
            let rows: Vec<Bar> = (0..360).map(|i| bar(month(m), i, OI_NULL)).collect();
            let srt = write(&root, &index(), month(m), &rows, &rows);
            let shape = Shape::of(month(m), 60, super::symbol_id("NIFTY"));
            let at: Vec<u64> = SortIndex::open(&srt, &shape)
                .expect("the planted index opens")
                .runs()
                .iter()
                .map(|run| run.offset + run.column(Column::Close).entries_at() + 9)
                .collect();
            let file = std::fs::OpenOptions::new()
                .read(true)
                .write(true)
                .open(&srt)
                .expect("the planted index");
            for at in at {
                let mut byte = [0u8; 1];
                std::os::unix::fs::FileExt::read_exact_at(&file, &mut byte, at).expect("a byte");
                byte[0] ^= 0x04;
                std::os::unix::fs::FileExt::write_all_at(&file, &byte, at).expect("a byte");
            }
        }
        let why = window(&root, &query(index(), month(1), month(10), SortKey::Close))
            .expect_err("past the cap the page is refused");
        assert!(
            why.starts_with(&format!(
                "more than {} months' sort indexes failed while this page read them",
                super::MAX_BROKEN
            )),
            "{why}"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    /// **THE WEIGHED MEMO HOLDS WHAT FITS ITS BUDGET, EMPTIES WHOLE WHEN A
    /// VALUE WOULD PASS IT, AND NEVER HOLDS ONE HEAVIER THAN THE BUDGET.**
    /// A value weighing exactly the budget is held; the next, which would
    /// pass it, empties the memo first; one heavier than the budget is not
    /// held and empties nothing. Kills `Weighed::get` → `None`,
    /// `Weighed::put` → `()`, and every mutant of `weight > budget`.
    #[test]
    fn the_weighed_memo_holds_within_its_budget() {
        let memo: super::Weighed<u32, &str> = super::Weighed::new(100);
        assert_eq!(memo.get(&1), None);
        memo.put(1, "whole budget", 100);
        assert_eq!(
            memo.get(&1),
            Some("whole budget"),
            "exactly the budget is held"
        );
        memo.put(2, "small", 10);
        assert_eq!(
            (memo.get(&1), memo.get(&2)),
            (None, Some("small")),
            "a value that would pass the budget empties the memo first"
        );
        memo.put(3, "too heavy", 101);
        assert_eq!(
            (memo.get(&2), memo.get(&3)),
            (Some("small"), None),
            "a value heavier than the budget is not held and empties nothing"
        );
        memo.put(4, "fits", 90);
        assert_eq!(
            (memo.get(&2), memo.get(&4)),
            (Some("small"), Some("fits")),
            "10 + 90 is the budget, held together"
        );
    }

    /// **A PAGE ASKED AGAIN COUNTS ITS UNREAD ROWS ONCE.** February has no
    /// index, so its rows are read, and another writer holds it, so they are
    /// not; January's index fails its checksum when the page reads it, so
    /// the page is asked again with January read. The answer counts
    /// February's 360 unread rows once — the attempt that failed does not
    /// add its own count. (It was written against `SeriesPlan::clear_unread`
    /// → `()` and does NOT kill it: the failed attempt had already drained
    /// its count with `take_unread` before its index failed, so there is
    /// nothing left to clear. The report lists that mutant as equivalent.)
    #[test]
    fn a_page_asked_again_counts_its_unread_rows_once() {
        let root = scratch("retry-unread");
        let srts: Vec<PathBuf> = (1..=3)
            .map(|m| {
                let rows: Vec<Bar> = (0..360).map(|i| bar(month(m), i, OI_NULL)).collect();
                write(&root, &index(), month(m), &rows, &rows)
            })
            .collect();
        std::fs::remove_file(&srts[1]).expect("February is left with no index");
        let ask = query(index(), month(1), month(3), SortKey::Close);
        let free = window(&root, &ask)
            .expect("answers")
            .sort_index
            .expect("a value sort says what it used");
        assert_eq!(
            (free.indexed_rows, free.scanned_rows, free.unread_rows),
            (720, 360, 0)
        );
        let shape = Shape::of(month(1), 60, super::symbol_id("NIFTY"));
        let at: Vec<u64> = SortIndex::open(&srts[0], &shape)
            .expect("January's index opens")
            .runs()
            .iter()
            .map(|run| run.offset + run.column(Column::Close).entries_at() + 9)
            .collect();
        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(&srts[0])
            .expect("January's index");
        for at in at {
            let mut byte = [0u8; 1];
            std::os::unix::fs::FileExt::read_exact_at(&file, &mut byte, at).expect("a byte");
            byte[0] ^= 0x04;
            std::os::unix::fs::FileExt::write_all_at(&file, &byte, at).expect("a byte");
        }
        drop(file);
        let lock = StorePath::new(PathParts {
            vendor: Vendor::Gdfl,
            exchange: "NSE",
            segment: "INDEX",
            symbol: "NIFTY",
            contract: None,
            timeframe: Timeframe::MINUTE_1,
            month: month(2),
            file: FileKind::Lock,
        })
        .expect("a generated path")
        .to_path_buf(&root);
        let held = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&lock)
            .expect("the lock file");
        let _held = store::flock::Flock::try_lock(held, lock).expect("February's lock is free");
        let report = window(&root, &ask)
            .expect("answers")
            .sort_index
            .expect("a value sort says what it used");
        assert!(
            report
                .scanned_why
                .iter()
                .any(|why| why.contains("2031-01") && why.contains("failed when it was read")),
            "January was set aside and the page asked again: {report:?}"
        );
        assert_eq!(
            (report.unread_rows, report.unread_months),
            (360, 1),
            "{report:?}"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    /// **A HOST'S REFUSAL OF AN INDEX IS THIS REQUEST'S ALONE, AND NEVER
    /// REMEMBERED** — run in a process the mode bits bind, since root may
    /// read a `0o000` file and the tests beside this one stand down there.
    /// February's `.srt` is made unreadable twice: once before any request
    /// walked it (its walk is refused, its rows are read, and the refusal is
    /// not kept), and once after a request walked it while it was free (the
    /// page meets the refusal while it reads the index, sets February aside
    /// for this request and reads its rows). Each time the mode is restored,
    /// the next request is served from all three indexes again. Kills
    /// `view`'s `== with !=` on both halves of the set-aside match (the page
    /// then never finds the month it set aside and is asked again past the
    /// cap), its `|| with &&` on what passes, and its `delete !` on what is
    /// remembered.
    #[test]
    fn a_host_refusal_of_an_index_is_this_requests_alone() {
        crate::isolated::where_permission_binds(
            "bars::sort::cloud_mutant_tests::a_host_refusal_of_an_index_is_this_requests_alone",
            || {
                use std::os::unix::fs::PermissionsExt as _;
                let root = scratch("host-refusal");
                let srts: Vec<PathBuf> = (1..=3)
                    .map(|m| {
                        let rows: Vec<Bar> = (0..360).map(|i| bar(month(m), i, OI_NULL)).collect();
                        write(&root, &index(), month(m), &rows, &rows)
                    })
                    .collect();
                let ask = query(index(), month(1), month(3), SortKey::Close);
                let mode = |mode: u32| {
                    std::fs::set_permissions(&srts[1], std::fs::Permissions::from_mode(mode))
                        .expect("February's index takes the mode");
                };
                let read = || {
                    let answer = window(&root, &ask).expect("a refused index is read, never fatal");
                    answer.sort_index.expect("a value sort says what it used")
                };
                mode(0o000);
                let refused = read();
                assert_eq!((refused.indexed_rows, refused.scanned_rows), (720, 360));
                mode(0o644);
                let restored = read();
                assert_eq!(
                    (restored.indexed_rows, restored.scanned_rows),
                    (1_080, 0),
                    "a refused walk is not remembered"
                );
                mode(0o000);
                let aside = read();
                assert_eq!((aside.indexed_rows, aside.scanned_rows), (720, 360));
                assert!(
                    aside.scanned_why.iter().any(|why| why.contains(
                        "the host refused its .srt sort index while this request read it"
                    )),
                    "{aside:?}"
                );
                mode(0o644);
                let again = read();
                assert_eq!(
                    (again.indexed_rows, again.scanned_rows),
                    (1_080, 0),
                    "a refusal met while reading is not remembered"
                );
                let _ = std::fs::remove_dir_all(&root);
            },
        );
    }

    /// **A PAGE INSIDE AN INDEX RUN OF SEVERAL BLOCKS IS THE BRUTE FORCE.**
    /// Every other fixture's index runs are at most 130 rows, one fence block
    /// (`FENCE` entries) each. One run of 1,000 rows is four blocks: the
    /// bounds the value bisection takes from the fences are a block wide,
    /// and the exact count is read between them. Pages at the start, inside
    /// and at the end, in both directions, are the brute force.
    #[test]
    fn a_page_inside_a_run_of_several_blocks_is_the_brute_force() {
        let root = scratch("multi-block");
        let rows: Vec<Bar> = (0..1_000).map(|i| bar(month(1), i, OI_NULL)).collect();
        let symbol_id = super::symbol_id("NIFTY");
        let path = StorePath::new(PathParts {
            vendor: Vendor::Gdfl,
            exchange: "NSE",
            segment: "INDEX",
            symbol: "NIFTY",
            contract: None,
            timeframe: Timeframe::MINUTE_1,
            month: month(1),
            file: FileKind::Bars,
        })
        .expect("a generated path");
        store::file::BarFile::open_or_create(&root, path, symbol_id)
            .expect("a generated bar file")
            .append(&rows)
            .expect("generated records");
        crate::scratch::plant_pre_rule_sort_index(
            &path.to_path_buf(&root),
            &Shape::of(month(1), 60, symbol_id),
            &rows,
            1_000,
        );
        for desc in [false, true] {
            let key = |close: i64| if desc { -close } else { close };
            let mut want: Vec<(i64, i64)> = rows
                .iter()
                .map(|bar| (key(bar.close), bar.ts_micros))
                .collect();
            want.sort_unstable();
            for offset in [0, 300, 517, 963] {
                let mut asked = query(index(), month(1), month(1), SortKey::Close);
                asked.desc = desc;
                asked.offset = Some(offset);
                let got = window(&root, &asked).expect("answers");
                let served: Vec<(i64, i64)> = got
                    .bars
                    .iter()
                    .map(|row| (key(row.bar.close), row.bar.ts_micros))
                    .collect();
                let page: Vec<(i64, i64)> = want.iter().skip(offset).take(37).copied().collect();
                assert_eq!(served, page, "desc {desc} offset {offset}");
                let report = got.sort_index.expect("a value sort says what it used");
                assert_eq!((report.indexed_runs, report.scanned_rows), (1, 0));
            }
        }
        let _ = std::fs::remove_dir_all(&root);
    }

    /// **A PAGE OVER READ ROWS AND A WIDE INDEX RUN IS THE BRUTE FORCE.**
    /// January's 1,000 rows are one index run of four blocks. February has
    /// no index (a month under the small-month threshold), so its 360 rows
    /// are read and the page's best of them held. The value bisection adds
    /// the held rows at or below a value to the run's fence-bounded counts.
    /// Pages at four offsets, in both directions, are the brute force over
    /// all 1,360 rows. Kills `Arrays::more_than`'s `lb += a` → `*=`: it
    /// multiplied the held rows by the run's lower count, said "more than k"
    /// for a value too small, and the page started in the wrong place.
    #[test]
    fn a_page_over_read_rows_and_a_wide_run_is_the_brute_force() {
        let root = scratch("read-and-wide");
        let jan: Vec<Bar> = (0..1_000).map(|i| bar(month(1), i, OI_NULL)).collect();
        let feb: Vec<Bar> = (0..360).map(|i| bar(month(2), i, OI_NULL)).collect();
        let symbol_id = super::symbol_id("NIFTY");
        let path = StorePath::new(PathParts {
            vendor: Vendor::Gdfl,
            exchange: "NSE",
            segment: "INDEX",
            symbol: "NIFTY",
            contract: None,
            timeframe: Timeframe::MINUTE_1,
            month: month(1),
            file: FileKind::Bars,
        })
        .expect("a generated path");
        store::file::BarFile::open_or_create(&root, path, symbol_id)
            .expect("a generated bar file")
            .append(&jan)
            .expect("generated records");
        crate::scratch::plant_pre_rule_sort_index(
            &path.to_path_buf(&root),
            &Shape::of(month(1), 60, symbol_id),
            &jan,
            1_000,
        );
        let feb_srt = write(&root, &index(), month(2), &feb, &feb);
        std::fs::remove_file(feb_srt).expect("February is left with no index");
        for desc in [false, true] {
            let key = |close: i64| if desc { -close } else { close };
            let mut want: Vec<(i64, i64)> = jan
                .iter()
                .chain(&feb)
                .map(|bar| (key(bar.close), bar.ts_micros))
                .collect();
            want.sort_unstable();
            for offset in [0, 150, 400, 900] {
                let mut asked = query(index(), month(1), month(2), SortKey::Close);
                asked.desc = desc;
                asked.offset = Some(offset);
                let got = window(&root, &asked).expect("answers");
                let served: Vec<(i64, i64)> = got
                    .bars
                    .iter()
                    .map(|row| (key(row.bar.close), row.bar.ts_micros))
                    .collect();
                let page: Vec<(i64, i64)> = want.iter().skip(offset).take(37).copied().collect();
                assert_eq!(served, page, "desc {desc} offset {offset}");
                let report = got.sort_index.expect("a value sort says what it used");
                assert_eq!((report.indexed_rows, report.scanned_rows), (1_000, 360));
            }
        }
        let _ = std::fs::remove_dir_all(&root);
    }

    /// **A NULL PAGE THAT STARTS ON THE LATER SERIES' ROW OF AN INSTANT
    /// SKIPS THE EARLIER SERIES' ROW OF THAT INSTANT.** Open interest over
    /// the index (asked first, every row null) and a contract (null on every
    /// sixth bar): past the valued rows the order is null rows by instant,
    /// then series. A page that starts on the contract's null row of an
    /// instant must not serve the index's row of the same instant, which
    /// sorts before it. Every such page is the brute force. Runs
    /// `cursors_at`'s probe one microsecond past the start for a time-ordered
    /// series asked before the start's, which no test reached.
    #[test]
    fn a_null_page_on_the_later_series_skips_the_earlier_ones_row() {
        let root = scratch("null-start-series");
        let oi = |i: i64| {
            if i % 6 == 0 {
                OI_NULL
            } else {
                40_000 + (i * 17) % 50
            }
        };
        let mut rows: Vec<(bool, i64, i64, usize)> = Vec::new();
        for m in 1..=2 {
            let index_rows: Vec<Bar> = (0..360).map(|i| bar(month(m), i, OI_NULL)).collect();
            let option_rows: Vec<Bar> = (0..360).map(|i| bar(month(m), i, oi(i))).collect();
            write(&root, &index(), month(m), &index_rows, &index_rows);
            write(&root, &option(), month(m), &option_rows, &option_rows);
            rows.extend(index_rows.iter().map(|bar| (true, 0, bar.ts_micros, 0)));
            rows.extend(option_rows.iter().map(|bar| {
                let null = bar.open_interest == OI_NULL;
                (
                    null,
                    if null { 0 } else { bar.open_interest },
                    bar.ts_micros,
                    1,
                )
            }));
        }
        rows.sort_unstable();
        let starts: Vec<usize> = rows
            .iter()
            .enumerate()
            .filter(|(_, row)| row.0 && row.3 == 1)
            .map(|(at, _)| at)
            .take(3)
            .collect();
        assert_eq!(starts.len(), 3, "the fixture holds null contract rows");
        for start in starts {
            let mut asked = query(index(), month(1), month(2), SortKey::OpenInterest);
            asked.series.push(option());
            asked.offset = Some(start);
            asked.limit = 5;
            let got = window(&root, &asked).expect("answers");
            let served: Vec<(i64, usize)> = got
                .bars
                .iter()
                .map(|row| (row.bar.ts_micros, row.series))
                .collect();
            let want: Vec<(i64, usize)> = rows
                .iter()
                .skip(start)
                .take(5)
                .map(|row| (row.2, row.3))
                .collect();
            assert_eq!(served, want, "offset {start}");
        }
        let _ = std::fs::remove_dir_all(&root);
    }

    /// **A PAGE SERVED FROM CUT RUNS ALONE IS THE BRUTE FORCE.** A day bound
    /// keeps the 3rd alone, which cuts the month's first two index runs and
    /// leaves the third out, so every kept row is ordered from cut runs'
    /// entries and the value domain the page bisects comes from their lowest
    /// and highest values. Pages at the start, the middle and the end are
    /// the brute force, and the report names its column. Kills `page_once`'s
    /// `delete field max` on a cut run's `ColumnInfo` (it made `0` the top of
    /// the domain, which leaves out every value above the lowest) and
    /// `delete field column` on the report.
    #[test]
    fn a_page_from_cut_runs_alone_is_the_brute_force() {
        let root = scratch("cut-runs-alone");
        {
            let m = 1;
            let rows: Vec<Bar> = (0..360).map(|i| bar(month(m), i, OI_NULL)).collect();
            write(&root, &index(), month(m), &rows, &rows);
            let (lo, _) = month(m).ist_bounds_micros();
            let (from, to) = (lo + 2 * 86_400 * MICROS, lo + 3 * 86_400 * MICROS);
            let mut want: Vec<(i64, i64)> = rows
                .iter()
                .filter(|bar| bar.ts_micros >= from && bar.ts_micros < to)
                .map(|bar| (bar.close, bar.ts_micros))
                .collect();
            want.sort_unstable();
            assert_eq!(want.len(), 120, "the 3rd holds one session");
            for offset in [0, 41, 83] {
                let mut asked = query(index(), month(m), month(m), SortKey::Close);
                asked.filter.from = Some(from);
                asked.filter.to = Some(to);
                asked.offset = Some(offset);
                let got = window(&root, &asked).expect("answers");
                let served: Vec<(i64, i64)> = got
                    .bars
                    .iter()
                    .map(|row| (row.bar.close, row.bar.ts_micros))
                    .collect();
                let page: Vec<(i64, i64)> = want.iter().skip(offset).take(37).copied().collect();
                assert_eq!(served, page, "month {m} offset {offset}");
                let report = got.sort_index.expect("a value sort says what it used");
                assert_eq!(report.column, Column::Close.as_str());
                assert_eq!(
                    (report.cut_runs, report.indexed_rows, report.scanned_rows),
                    (2, 120, 0),
                    "every kept row is ordered from the two cut runs"
                );
            }
        }
        let _ = std::fs::remove_dir_all(&root);
    }

    /// **A PAGE PAST EVERY ROW THAT COULD BE READ IS EMPTY, NOT AN ERROR.**
    /// February has no index and another writer holds it, so its 360 rows
    /// are counted in the total and read by no one. A page at offset 1,000
    /// of 1,080 lies past the 720 rows January's and March's indexes order:
    /// it is empty, and the unread rows are said. Runs `start_key`'s and
    /// `select`'s past-every-row answers, which no test reached.
    #[test]
    fn a_page_past_every_readable_row_is_empty() {
        let root = scratch("past-readable");
        let srts: Vec<PathBuf> = (1..=3)
            .map(|m| {
                let rows: Vec<Bar> = (0..360).map(|i| bar(month(m), i, OI_NULL)).collect();
                write(&root, &index(), month(m), &rows, &rows)
            })
            .collect();
        std::fs::remove_file(&srts[1]).expect("February is left with no index");
        let mut ask = query(index(), month(1), month(3), SortKey::Close);
        let free = window(&root, &ask).expect("answers");
        assert_eq!(free.total, 1_080);
        let lock = StorePath::new(PathParts {
            vendor: Vendor::Gdfl,
            exchange: "NSE",
            segment: "INDEX",
            symbol: "NIFTY",
            contract: None,
            timeframe: Timeframe::MINUTE_1,
            month: month(2),
            file: FileKind::Lock,
        })
        .expect("a generated path")
        .to_path_buf(&root);
        let held = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&lock)
            .expect("the lock file");
        let _held = store::flock::Flock::try_lock(held, lock).expect("February's lock is free");
        ask.offset = Some(1_000);
        let got = window(&root, &ask).expect("a page past the readable rows answers");
        assert!(got.bars.is_empty(), "{:?}", got.bars.len());
        let report = got.sort_index.expect("a value sort says what it used");
        assert_eq!((report.unread_rows, report.unread_months), (360, 1));
        let _ = std::fs::remove_dir_all(&root);
    }
}
