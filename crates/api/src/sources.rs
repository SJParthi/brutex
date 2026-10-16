//! `/sources.json`, `/sources/bars.json`, `/sources/day.json` -- the All
//! sources view of one instrument: the rung table, one combined timeline per
//! rung, and the day reconciliation, every row keeping its feed. D-3339.
//!
//! # Why this exists
//!
//! `/db` is one feed at a time by design, and nothing listed which feeds and
//! rungs hold a symbol, so the operator reconciled Zerodha against GDFL by
//! hand (DB-079, DB-G99): NIFTY's day close differs from GDFL's last second on
//! 19 of 19 days of 2019-03 because one is the official close and the other the
//! last print (DB-035, D-3324); RELIANCE's Zerodha prices sit at GDFL ÷ 2.0981
//! because Zerodha back-adjusts shares and GDFL does not (DB-G58, DB-G75,
//! D-3323); some adjusted runs drift, carry volume on another basis, or sit on
//! a coarser grid than the exchange's (DB-G38, DB-G39, DB-G40); and a Zerodha
//! day bar is not a fold of its minutes (DB-G41). These three routes put the
//! feeds side by side and count those differences, labelled.
//!
//! # Storage stays per vendor
//!
//! Nothing is merged on disk (`CLAUDE.md` §3 rule 3, D-0225). Every row and
//! every cell names its source with one of D-3339's four tags:
//!
//! | tag | what it is |
//! |---|---|
//! | `zerodha_stored` | a rung Zerodha serves and the store holds (`1min`, `1day`) |
//! | `zerodha_derived` | a rung the pull folds from Zerodha's minutes (`2min`..`60min`) |
//! | `gdfl_1s` | GDFL's one-second bars, as stored |
//! | `gdfl_fold` | GDFL's seconds folded to the asked rung by `pull::fold`, on request |
//!
//! The tag is `<feed>_<kind>` from the descriptor (`pull::vendor`), so the
//! words above are what the two offered feeds produce, not a table this file
//! keeps.
//!
//! # The three routes
//!
//! ```text
//! GET /sources.json?exchange=NSE&segment=INDEX&symbol=NIFTY[&contract=..]
//!     every rung each offered feed holds, stored, derived or foldable, with
//!     its span, months held and absent, census rows, price and volume basis,
//!     close semantics, archive or window, and partial or behind state.
//! GET /sources/bars.json?...&rung=1min&from=YYYY-MM-DD[&to=..][&dir=desc]
//!                        [&offset=0][&limit=50][&cursor=<unix s>]
//!     one timeline at the rung: a row per bucket any source holds, each cell
//!     tagged, each field compared against D-3339's named tolerance, and a
//!     factor in place of a disagreement where the bases differ.
//! GET /sources/day.json?...&from=YYYY-MM-DD|YYYY-MM[&to=..][&page=0][&order=asc]
//!     the day reconciliation, at most 31 days a page.
//! ```
//!
//! # Cost, stated rather than claimed
//!
//! * `/sources.json`: per offered feed one `stat` per known rung directory
//!   and a listing only when the directory changed ([`held_months`]), then one
//!   census probe per held month (`census::Manifests::entry`, one hash
//!   lookup). `O(rungs × months held)`, at most 10 × 141 probes for NIFTY, and
//!   independent of how many instruments the store holds. Each stored and
//!   derived rung's owed sessions are `/calendar.json`'s answer for it, kept
//!   per feed generation (one map probe when kept; built once per generation
//!   otherwise, which is where a first read after a commit spends its time:
//!   measured on the operator's store, 10 Oct 2026, 0.9 s for NIFTY and
//!   0.7 s for RELIANCE, and about 3 ms once kept).
//!
//! # A factor is read on one instant only (D-3339 item 3)
//!
//! A share's factor is GDFL ÷ Zerodha on a pair of prices of the same
//! instant: the closes of one bucket at an intraday rung; at the day rung,
//! whose closes are official against last print (D-3324), GDFL's opening
//! print against Zerodha's official open, and only where that print is in the
//! opening minute of the calendar of record; in the day table, else the last
//! minute both feeds hold. Where no pair is the same instant the factor is
//! `null` and nothing is judged (`basis_unmeasured`), never a close set
//! against an official close it may sit 1% from.
//! * `/sources/bars.json`: `O(offset + limit)` buckets per source. A stored
//!   source reads that many records from one `.tix` seek; a fold reads the
//!   seconds of that many buckets (at most `width` each) and no more: the
//!   page is bounded by the page, not by the history. `offset` is a bounded
//!   skip ([`MAX_OFFSET`]); `cursor` continues from any bucket at the cost of
//!   one page, wherever it is. Every read counts against [`MAX_ROWS_READ`] and
//!   a page that would pass it is refused by name, never cut short silently.
//! * `/sources/day.json`: per day, the day's Zerodha `1day` record, its
//!   minutes and GDFL's seconds (at most one IST day of each, located through
//!   `.tix`), cached per (generations, instrument, day): the key holds the
//!   `stat` of each of the three month files, so an append or a new
//!   generation is read again and an unchanged day costs three `stat`s. The
//!   day marks are `/dayflags.json`'s own records for the page (WP-33b).

use std::collections::{BTreeSet, HashMap};
use std::hash::Hash;
use std::os::unix::fs::MetadataExt as _;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock, PoisonError};

use brutex_core::instrument::{Contract, Exchange, Segment};
use brutex_core::symbol::Symbol;
use brutex_core::vendor::Vendor;
use pull::fold::{Bucket, fold_from_bars};
use pull::manifest::Entry;
use pull::session::Day;
use pull::vendor::{Basis, CloseSemantics, Feed, Span};
use serde_json::{Value, json};
use store::file::BarFile;
use store::format::{Bar, OI_NULL};
use store::path::{FileKind, PathParts, StorePath, Timeframe, YearMonth};

use super::{Loaded, Site, param, vendor_census};
use crate::census;

// ══ The named thresholds of D-3339 ═══════════════════════════════════════════

/// D-3339 item 2: an index, option or same-basis share price DISAGREES when
/// it differs by more than this many basis points. Chosen, not sourced.
pub const PRICE_TOLERANCE_BPS: i128 = 1;

/// D-3339 item 2: a volume disagrees when it differs by more than this many
/// permille (0.5%). Chosen, not sourced.
pub const VOLUME_TOLERANCE_PERMILLE: i128 = 5;

/// D-3339 item 3: a share's factor that differs from 1 by more than this many
/// permille (0.2%) is `basis_differs`, and a factor that moves by more than it
/// between consecutive common days is `basis_step`. The thresholds that
/// reproduce D-3323's 101 of 208; chosen, not sourced.
pub const FACTOR_THRESHOLD_PERMILLE: i128 = 2;

/// D-3339 item 4: the most days one `/sources/day.json` page holds.
pub const PAGE_DAYS: u32 = 31;

/// The page a timeline answers when none is asked: the /db grid's own.
pub const DEFAULT_LIMIT: usize = 50;

/// The most buckets one timeline page holds below the day rung.
pub const MAX_LIMIT: usize = 200;

/// The most buckets one timeline page holds at the day rung: a day bucket
/// folds up to a session of seconds, so a day page is the day table's page.
pub const MAX_DAY_LIMIT: usize = 31;

/// The most buckets `offset` may skip. Past it, `cursor` continues from any
/// bucket at the cost of one page; a deeper offset would cost the skip.
pub const MAX_OFFSET: usize = 1_000;

/// The most records one request may read, across every source. A page that
/// would read more is refused by name: about 80 sessions of one-second bars.
pub const MAX_ROWS_READ: u64 = 2_000_000;

/// The most months or days a list in an answer names; its count is always
/// whole.
const MAX_NAMED: usize = 24;

/// Entries a process-wide memo holds before it is emptied and refilled.
const MEMO_CAP: usize = 65_536;

/// Microseconds in a second.
const MICROS: i64 = 1_000_000;

/// Seconds in a day.
const DAY_SECS: i64 = 86_400;

/// The content type every answer here carries.
const JSON: &str = "application/json; charset=utf-8";

/// One answer: status, content type, body.
type Answer = (
    axum::http::StatusCode,
    [(axum::http::HeaderName, &'static str); 1],
    String,
);

/// `body` with `status` as JSON.
fn answer(status: axum::http::StatusCode, body: &Value) -> Answer {
    (
        status,
        [(axum::http::header::CONTENT_TYPE, JSON)],
        body.to_string(),
    )
}

/// A refusal: the status and why, as `{"error": why}`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Refusal {
    /// The HTTP status.
    pub status: axum::http::StatusCode,
    /// Why, in words a page quotes.
    pub why: String,
}

impl Refusal {
    /// A `400` naming what was wrong with the question.
    fn bad(why: impl Into<String>) -> Self {
        Self {
            status: axum::http::StatusCode::BAD_REQUEST,
            why: why.into(),
        }
    }

    /// A `422`: the question was well formed and answering it would cost
    /// more than the bound this route states.
    fn over(why: impl Into<String>) -> Self {
        Self {
            status: axum::http::StatusCode::UNPROCESSABLE_ENTITY,
            why: why.into(),
        }
    }

    /// A `503`: the store could not be read. A month refused because its
    /// key was RETIRED by the re-key is no store fault: [`Self::gone`].
    fn store(why: impl Into<String>) -> Self {
        let why = why.into();
        if pull::gdfl_lineage::is_contract_refused(&why) {
            return Self::gone(why);
        }
        Self {
            status: axum::http::StatusCode::SERVICE_UNAVAILABLE,
            why,
        }
    }

    /// A `410`: a GDFL option key the re-key RETIRED, with the ledger's
    /// sentence, as every bar route answers it (D-3331; WP-12c, DB-A18).
    fn gone(why: impl Into<String>) -> Self {
        Self {
            status: axum::http::StatusCode::GONE,
            why: why.into(),
        }
    }

    fn json(&self) -> Value {
        json!({ "error": self.why })
    }
}

/// `410` for a GDFL option key the re-key RETIRED, with the ledger's
/// sentence, as every bar route answers it (D-3331; WP-12c, DB-A18); `Ok`
/// for every other instrument. Asked FIRST by each route here, before any
/// memo or walk, so nothing cached before the re-key ran serves the key. One
/// `stat` of the ledger and one hash probe.
fn refuse_retired(root: &Path, inst: &Instrument) -> Result<(), Refusal> {
    if inst.segment == Segment::Fno
        && inst.contract.is_some()
        && let Some(why) = crate::bars::retired_refusal(root, Vendor::Gdfl, &inst.key())
    {
        return Err(Refusal::gone(why));
    }
    Ok(())
}

// ══ The instrument ═══════════════════════════════════════════════════════════

/// One instrument as the store files it: its exchange, segment and symbol
/// directories and, for a future or an option, its contract directory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Instrument {
    exchange: Exchange,
    segment: Segment,
    symbol: String,
    contract: Option<Contract>,
}

impl Instrument {
    /// Reads `exchange`, `segment`, `symbol` and `contract` off a query.
    ///
    /// All three of the first are required and never defaulted: a default
    /// would answer for an instrument the reader did not pick. A contract is
    /// named exactly when the segment is `FNO`.
    ///
    /// # Errors
    ///
    /// The parameter and why, as one sentence.
    pub fn parse(query: &str) -> Result<Self, String> {
        let exchange_text = param(query, "exchange");
        let exchange = Exchange::parse(&exchange_text)
            .map_err(|_| format!("exchange={exchange_text:?} is not NSE or BSE"))?;
        let segment_text = param(query, "segment");
        let segment = Segment::parse(&segment_text)
            .map_err(|_| format!("segment={segment_text:?} is not INDEX, CASH or FNO"))?;
        let symbol = param(query, "symbol");
        if Symbol::new(&symbol).is_err() {
            return Err(format!(
                "symbol={symbol:?} is not a symbol this store files: 1 to 24 bytes"
            ));
        }
        let contract_text = param(query, "contract");
        let contract = if contract_text.is_empty() {
            None
        } else {
            Some(Contract::parse(&contract_text).ok_or_else(|| {
                format!("contract={contract_text:?} is not a contract this store can name")
            })?)
        };
        match (segment, contract) {
            (Segment::Fno, None) => Err(
                "segment=FNO names one contract with contract=YYYY-MM-DD-<strike paisa>-CE|PE \
                 or YYYY-MM-DD-FUT; a chain is not one timeline"
                    .to_owned(),
            ),
            (Segment::Index | Segment::Cash, Some(_)) => Err(format!(
                "contract= names a future or an option, and segment={} holds none",
                segment.as_str()
            )),
            _ => Ok(Self {
                exchange,
                segment,
                symbol,
                contract,
            }),
        }
    }

    /// The census's text: `NSE-INDEX-NIFTY`, then `-<contract>`.
    fn key(&self) -> String {
        let base = format!(
            "{}-{}-{}",
            self.exchange.as_str(),
            self.segment.as_str(),
            self.symbol
        );
        match self.contract {
            Some(contract) => format!("{base}-{}", contract.as_str()),
            None => base,
        }
    }

    fn json(&self) -> Value {
        json!({
            "exchange": self.exchange.as_str(),
            "segment": self.segment.as_str(),
            "symbol": self.symbol,
            "contract": self.contract.map(|c| c.as_str().to_owned()),
            "key": self.key(),
        })
    }

    /// The store path parts of this instrument's `month` at `rung` of `vendor`.
    fn parts(&self, vendor: Vendor, rung: Timeframe, month: YearMonth) -> PathParts<'_> {
        PathParts {
            vendor,
            exchange: self.exchange.as_str(),
            segment: self.segment.as_str(),
            symbol: &self.symbol,
            contract: self.contract,
            timeframe: rung,
            month,
            file: FileKind::Bars,
        }
    }

    /// The census key of this instrument's `month` at `rung`, or `None`
    /// where the symbol is not one the census can key.
    fn census_key(&self, rung: Timeframe, month: YearMonth) -> Option<pull::manifest::EntryKey> {
        let symbol = Symbol::new(&self.symbol).ok()?;
        Some(
            census::Series {
                contract: self.contract,
                exchange: self.exchange,
                segment: self.segment,
                symbol,
                timeframe: rung,
            }
            .at(month),
        )
    }

    /// The same instrument as `/dayflags.json` names it.
    fn for_day_flags(&self) -> super::dayflags::Instrument {
        super::dayflags::Instrument {
            exchange: self.exchange,
            segment: self.segment,
            symbol: self.symbol.clone(),
            contract: self.contract,
        }
    }

    /// The `/db` selection that shows this instrument's stored rows of
    /// `vendor` at `rung`, in D-3337's parameter names.
    fn db_link(&self, vendor: Vendor, rung: Timeframe, outside_window: bool) -> String {
        let mut link = format!(
            "/db?feed={}&seg={}&sym={}&key={}&tf={}",
            vendor.as_str(),
            self.segment.as_str(),
            query_value(&self.symbol),
            query_value(&self.key()),
            rung.as_str()
        );
        if outside_window {
            link.push_str("&oow=1");
        }
        link
    }
}

/// `text` as a query-string value: every byte but an unreserved one is
/// percent-encoded, so a symbol such as `M&M` stays one value.
fn query_value(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for byte in text.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
            out.push(char::from(byte));
        } else {
            out.push('%');
            for nibble in [byte >> 4, byte & 0x0F] {
                if let Some(digit) = char::from_digit(u32::from(nibble), 16) {
                    out.push(digit.to_ascii_uppercase());
                }
            }
        }
    }
    out
}

// ══ Months on disk ═══════════════════════════════════════════════════════════

/// What a file or directory looked like when it was last read: length,
/// modification time and inode. A change in any of them reads it again.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct Stamp {
    len: u64,
    secs: i64,
    nanos: i64,
    inode: u64,
}

impl Stamp {
    fn of(meta: &std::fs::Metadata) -> Self {
        Self {
            len: meta.len(),
            secs: meta.mtime(),
            nanos: meta.mtime_nsec(),
            inode: meta.ino(),
        }
    }

    /// The stamp of the file at `path`, or `None` where there is none.
    fn at(path: &Path) -> Option<Self> {
        std::fs::metadata(path).ok().map(|meta| Self::of(&meta))
    }
}

/// A bounded process-wide memo: emptied whole when full, so it never grows
/// past [`MEMO_CAP`] and keeps no eviction order.
struct Memo<K, V>(OnceLock<Mutex<HashMap<K, V>>>);

impl<K: Eq + Hash, V: Clone> Memo<K, V> {
    const fn new() -> Self {
        Self(OnceLock::new())
    }

    fn lock(&self) -> MutexGuard<'_, HashMap<K, V>> {
        self.0
            .get_or_init(|| Mutex::new(HashMap::with_capacity(1_024)))
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }

    fn get(&self, key: &K) -> Option<V> {
        self.lock().get(key).cloned()
    }

    fn put(&self, key: K, value: V) {
        let mut held = self.lock();
        if held.len() >= MEMO_CAP {
            held.clear();
        }
        held.insert(key, value);
    }
}

/// Every rung directory's held months, by the directory's stamp.
static DIRS: Memo<PathBuf, (Stamp, Arc<[YearMonth]>)> = Memo::new();

/// The months a rung directory holds a `.bin` file for, oldest first.
///
/// One `stat`; the listing is read again only when the directory's stamp
/// changed, which is what adding a month does. A directory that does not
/// exist holds nothing: a rung never stored is an absence, not a refusal.
///
/// # Errors
///
/// The directory exists and cannot be listed.
fn held_months(dir: &Path) -> Result<Arc<[YearMonth]>, String> {
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
        if let Some(month) = entry
            .file_name()
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

/// `YYYY-MM` as a month, or `None`: the one strict parser.
///
/// This checked the halves' LENGTHS and handed them to `str::parse`, which
/// takes a leading `+`, so a directory entry `2024-+3.bin` was listed as a
/// held March (cloud-attack-law-20261016). `crate::bars::month_of_text`
/// already refused that spelling for every route.
fn month_of_text(text: &str) -> Option<YearMonth> {
    crate::bars::month_of_text(text)
}

/// The month after `month`.
fn next_month(month: YearMonth) -> Option<YearMonth> {
    if month.month() == 12 {
        YearMonth::new(month.year().checked_add(1)?, 1).ok()
    } else {
        YearMonth::new(month.year(), month.month() + 1).ok()
    }
}

/// A series and the months it holds on disk.
type HeldSeries = (Series, Arc<[YearMonth]>);

/// A rung's held months and its first and last record stamps.
type MonthsAndSpan = (Arc<[YearMonth]>, Option<(i64, i64)>);

/// One instrument's bars of one feed at one rung.
#[derive(Debug, Clone)]
struct Series {
    inst: Instrument,
    vendor: Vendor,
    rung: Timeframe,
}

impl Series {
    /// Where this series' month files are filed.
    fn dir(&self, root: &Path) -> Result<PathBuf, String> {
        let any = YearMonth::new(2000, 1).map_err(|why| why.to_string())?;
        let path = StorePath::new(self.inst.parts(self.vendor, self.rung, any))
            .map_err(|why| format!("{}: {why}", self.inst.key()))?
            .to_path_buf(root);
        Ok(path
            .parent()
            .map_or_else(|| root.to_path_buf(), Path::to_path_buf))
    }

    /// The months this series holds on disk.
    fn held(&self, root: &Path) -> Result<Arc<[YearMonth]>, String> {
        held_months(&self.dir(root)?)
    }

    /// The bar file of `month` every reader opens: the original, or the
    /// repair revision a promotion made it (D-3350, WP-13c). The day memo is
    /// keyed by this file's stamp, so a promotion is a new key and never a
    /// stale hit on the original's unchanged stamp. `None` also where the
    /// month's generation cannot be told; the read that follows names why.
    fn path(&self, root: &Path, month: YearMonth) -> Option<PathBuf> {
        StorePath::new(self.inst.parts(self.vendor, self.rung, month))
            .ok()
            .and_then(|path| store::generation::served_bars_path(root, path).ok())
    }

    /// `month`'s file, opened for reading through the reader every bar route
    /// uses, so its header cross-check and its refusal words are theirs.
    fn open(&self, root: &Path, month: YearMonth) -> Result<BarFile, String> {
        crate::bars::open_classified(root, self.inst.parts(self.vendor, self.rung, month))
            .map_err(|why| why.message)
    }
}

// ══ Reading rows in time order ═══════════════════════════════════════════════

/// The records one request may still read, shared by every source of it.
#[derive(Debug)]
struct Budget {
    left: u64,
}

impl Budget {
    const fn new() -> Self {
        Self {
            left: MAX_ROWS_READ,
        }
    }

    const fn used(&self) -> u64 {
        MAX_ROWS_READ - self.left
    }

    fn take(&mut self) -> Result<(), Refusal> {
        if self.left == 0 {
            return Err(Refusal::over(format!(
                "over_budget: this answer would read more than {MAX_ROWS_READ} records; ask a \
                 smaller limit, a shorter range or a finer rung"
            )));
        }
        self.left -= 1;
        Ok(())
    }
}

/// One month file being read: the rows still to read are `next..stop`
/// forwards, or `stop..next` backwards.
struct Open {
    file: BarFile,
    next: u64,
    stop: u64,
}

/// A series' records in `[lo, hi)`, forwards or backwards, across its held
/// months. Each month is opened when its first row is wanted and located
/// with two `first_at_or_after` lookups (through its `.tix`).
struct Rows<'s> {
    root: &'s Path,
    series: &'s Series,
    months: Vec<YearMonth>,
    at: usize,
    open: Option<Open>,
    lo: i64,
    hi: i64,
    desc: bool,
}

impl<'s> Rows<'s> {
    fn new(
        root: &'s Path,
        series: &'s Series,
        held: &[YearMonth],
        (lo, hi): (i64, i64),
        desc: bool,
    ) -> Self {
        let mut months: Vec<YearMonth> = held
            .iter()
            .copied()
            .filter(|month| {
                let (from, to) = month.ist_bounds_micros();
                to > lo && from < hi
            })
            .collect();
        if desc {
            months.reverse();
        }
        Self {
            root,
            series,
            months,
            at: 0,
            open: None,
            lo,
            hi,
            desc,
        }
    }

    /// The next record in reading order, or `None` past the last.
    fn next(&mut self, budget: &mut Budget) -> Result<Option<Bar>, Refusal> {
        loop {
            if let Some(open) = self.open.as_mut() {
                let row = if self.desc {
                    (open.next > open.stop).then(|| {
                        open.next -= 1;
                        open.next
                    })
                } else {
                    (open.next < open.stop).then(|| {
                        open.next += 1;
                        open.next - 1
                    })
                };
                if let Some(row) = row {
                    budget.take()?;
                    return open.file.read_record(row).map(Some).map_err(|why| {
                        Refusal::store(format!(
                            "{} record {row}: {why}",
                            open.file.path().display()
                        ))
                    });
                }
                self.open = None;
            }
            let Some(&month) = self.months.get(self.at) else {
                return Ok(None);
            };
            self.at += 1;
            let file = self.series.open(self.root, month).map_err(Refusal::store)?;
            let locate = |ts: i64| {
                file.first_at_or_after(ts).map_err(|why| {
                    Refusal::store(format!(
                        "{}: cannot locate {ts}: {why}",
                        file.path().display()
                    ))
                })
            };
            let start = locate(self.lo)?;
            let end = locate(self.hi)?;
            self.open = Some(if self.desc {
                Open {
                    file,
                    next: end,
                    stop: start,
                }
            } else {
                Open {
                    file,
                    next: start,
                    stop: end,
                }
            });
        }
    }

    /// Every record left, in reading order.
    fn all(&mut self, budget: &mut Budget) -> Result<Vec<Bar>, Refusal> {
        let mut out = Vec::new();
        while let Some(bar) = self.next(budget)? {
            out.push(bar);
        }
        Ok(out)
    }
}

// ══ Folding, through pull::fold and nothing else ═════════════════════════════

/// One folded bucket, the source records it holds and the stamp of the first
/// of them (a stored cell's own stamp): what says whether a fold's open is
/// the session's open or a later print (D-3339 item 3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Folded {
    bar: Bar,
    rows: u64,
    first: i64,
}

/// `rows` (in time order, all of `source` width) folded into `bucket`s by
/// `pull::fold`, the one fold authority, each with the count of records it
/// holds and the stamp of its first.
fn fold(rows: &[Bar], bucket: Bucket, source: Bucket) -> Result<Vec<Folded>, Refusal> {
    let bars = fold_from_bars(rows, bucket, source)
        .map_err(|why| Refusal::store(format!("pull::fold refused: {why}")))?;
    let mut out = Vec::with_capacity(bars.len());
    let mut at = 0_usize;
    for (n, bar) in bars.iter().enumerate() {
        let end = bars.get(n + 1).map_or(i64::MAX, |next| next.ts_micros);
        let first = rows.get(at).map_or(bar.ts_micros, |row| row.ts_micros);
        let mut count = 0_u64;
        while rows.get(at).is_some_and(|row| row.ts_micros < end) {
            count += 1;
            at += 1;
        }
        out.push(Folded {
            bar: *bar,
            rows: count,
            first,
        });
    }
    Ok(out)
}

/// The start of the bucket `ts` falls in, on `pull::fold`'s own grid: the
/// fold of one record is stamped at its bucket's start, so the grid is never
/// spelled a second time here.
fn bucket_start(ts: i64, bucket: Bucket) -> Result<i64, Refusal> {
    let probe = Bar {
        ts_micros: ts,
        open: 1,
        high: 1,
        low: 1,
        close: 1,
        volume: 0,
        open_interest: OI_NULL,
    };
    fold_from_bars(&[probe], bucket, Bucket::SECOND)
        .map_err(|why| Refusal::bad(format!("pull::fold refused the instant {ts}: {why}")))?
        .first()
        .map(|bar| bar.ts_micros)
        .ok_or_else(|| Refusal::bad(format!("pull::fold placed no bucket for {ts}")))
}

/// The fold bucket of a rung.
fn bucket_of(rung: Timeframe) -> Result<Bucket, Refusal> {
    Bucket::of_secs(rung.secs())
        .ok_or_else(|| Refusal::bad(format!("{} is not a width pull::fold folds", rung.as_str())))
}

/// The first `need` complete buckets of `rows`' seconds folded to `bucket`,
/// in reading order, and whether the source ran out before `need`.
///
/// Reads one record past the `need`-th bucket (the first record of the next)
/// so the last bucket returned is known whole; never more. A bucket is
/// complete forwards when a later record has been read, and backwards when
/// an earlier one has.
fn fold_buckets(
    rows: &mut Rows<'_>,
    bucket: Bucket,
    need: usize,
    budget: &mut Budget,
) -> Result<(Vec<Folded>, bool), Refusal> {
    let mut done: Vec<Folded> = Vec::with_capacity(need);
    // THE OPEN BUCKET'S RECORDS, in time order.
    let mut pending: Vec<Bar> = Vec::new();
    let mut pending_start: Option<i64> = None;
    let desc = rows.desc;
    loop {
        if done.len() >= need {
            return Ok((done, false));
        }
        let Some(bar) = rows.next(budget)? else {
            if !pending.is_empty() {
                if desc {
                    pending.reverse();
                }
                done.extend(fold(&pending, bucket, Bucket::SECOND)?);
            }
            done.truncate(need);
            return Ok((done, true));
        };
        let start = bucket_start(bar.ts_micros, bucket)?;
        if pending_start.is_some_and(|open| open != start) {
            // THE RECORD OPENS ANOTHER BUCKET, SO THE OPEN ONE IS WHOLE.
            if desc {
                pending.reverse();
            }
            done.extend(fold(&pending, bucket, Bucket::SECOND)?);
            pending.clear();
        }
        pending_start = Some(start);
        pending.push(bar);
    }
}

// ══ Arithmetic on prices, integer only (CLAUDE.md §7) ════════════════════════

/// `n / d` rounded half away from zero; `d > 0`.
const fn div_round(n: i128, d: i128) -> i128 {
    let q = n / d;
    let r = n % d;
    if 2 * r.abs() >= d {
        if n < 0 { q - 1 } else { q + 1 }
    } else {
        q
    }
}

/// `scaled / 10^places` as a JSON number with exactly `places` decimals.
fn decimal(scaled: i128, places: u32) -> Value {
    let unit = 10_u128.pow(places);
    let sign = if scaled < 0 { "-" } else { "" };
    let abs = scaled.unsigned_abs();
    let text = if places == 0 {
        format!("{sign}{abs}")
    } else {
        let width = usize::try_from(places).unwrap_or(0);
        format!("{sign}{}.{:0width$}", abs / unit, abs % unit)
    };
    text.parse::<serde_json::Number>()
        .map_or(Value::Null, Value::Number)
}

/// `a` against `b` in basis points to two decimals, and whether it is more
/// than [`PRICE_TOLERANCE_BPS`] — the test is exact, on the integers, never on
/// the rounded figure. `None` where `b` is not a price.
fn price_cmp(a: i64, b: i64) -> Option<(Value, bool)> {
    if b <= 0 || a < 0 {
        return None;
    }
    let (a, b) = (i128::from(a), i128::from(b));
    let hundredths = div_round((a - b) * 1_000_000, b);
    let beyond = (a - b).abs() * 10_000 > PRICE_TOLERANCE_BPS * b;
    Some((decimal(hundredths, 2), beyond))
}

/// `a` against `b` in percent to two decimals, and whether it is more than
/// [`VOLUME_TOLERANCE_PERMILLE`]. `None` where `b` is not positive.
fn volume_cmp(a: i64, b: i64) -> Option<(Value, bool)> {
    if b <= 0 || a < 0 {
        return None;
    }
    let (a, b) = (i128::from(a), i128::from(b));
    let hundredths = div_round((a - b) * 10_000, b);
    let beyond = (a - b).abs() * 1_000 > VOLUME_TOLERANCE_PERMILLE * b;
    Some((decimal(hundredths, 2), beyond))
}

/// `num / den` to four decimals. `None` where `den` is not positive.
fn ratio4(num: i64, den: i64) -> Option<Value> {
    (den > 0 && num >= 0).then(|| decimal(div_round(i128::from(num) * 10_000, i128::from(den)), 4))
}

/// Whether the factor `g / z` differs from 1 by more than
/// [`FACTOR_THRESHOLD_PERMILLE`]: `|g − z| × 1000 > 2 × z`, exact.
fn basis_differs(g: i64, z: i64) -> bool {
    let (g, z) = (i128::from(g), i128::from(z));
    z > 0 && (g - z).abs() * 1_000 > FACTOR_THRESHOLD_PERMILLE * z
}

/// Whether the factor moved by more than [`FACTOR_THRESHOLD_PERMILLE`] from
/// `(g0 / z0)` to `(g1 / z1)`: `|g1·z0 − z1·g0| × 1000 > 2 × z1·g0`, exact.
fn factor_stepped((g0, z0): (i64, i64), (g1, z1): (i64, i64)) -> bool {
    let (g0, z0, g1, z1) = (
        i128::from(g0),
        i128::from(z0),
        i128::from(g1),
        i128::from(z1),
    );
    let base = z1 * g0;
    base > 0 && (g1 * z0 - base).abs() * 1_000 > FACTOR_THRESHOLD_PERMILLE * base
}

/// The greatest common divisor of two non-negative integers.
const fn gcd(mut a: i64, mut b: i64) -> i64 {
    while b != 0 {
        let r = a % b;
        a = b;
        b = r;
    }
    a
}

/// The grid a price series sits on: the gcd of every open, high, low and
/// close in paisa. `0` for no record.
fn grid_of(rows: &[Bar]) -> i64 {
    rows.iter().fold(0, |grid, bar| {
        [bar.open, bar.high, bar.low, bar.close]
            .into_iter()
            .filter(|price| *price > 0)
            .fold(grid, gcd)
    })
}

// ══ Time ═════════════════════════════════════════════════════════════════════

/// The IST day (days since the epoch) of a UTC microsecond stamp.
const fn day_number(ts: i64) -> i64 {
    (ts.div_euclid(MICROS) + pull::session::IST_OFFSET_SECS).div_euclid(DAY_SECS)
}

/// 00:00 IST of the IST day `day`, UTC micros.
const fn day_start(day: i64) -> i64 {
    (day * DAY_SECS - pull::session::IST_OFFSET_SECS) * MICROS
}

/// The IST minute-of-day the first window of `day`'s session opens, from the
/// calendar of record (`pull::calendar::exchange_kind_of`: a Muhurat or a
/// disaster-recovery Saturday has its own window), and whether the record
/// said so. A day the record holds no window for (closed, unmeasured, or a
/// session of unmeasured length) is answered with the regular 09:15 open,
/// `pull::calendar::OPEN_MINUTE`, and `false`, which the answer names.
fn opening_minute(day: i64) -> (u16, bool) {
    match pull::calendar::exchange_kind_of(day) {
        pull::calendar::DayKind::Open(session) if session.count > 0 => session
            .windows
            .first()
            .map_or((pull::calendar::OPEN_MINUTE, false), |window| {
                (window.from, true)
            }),
        _ => (pull::calendar::OPEN_MINUTE, false),
    }
}

/// Whether a print stamped `ts` is the session's opening print: in or before
/// the opening minute of its IST day (a pre-open print at 09:07:59 is; a
/// first print at 11:57:48, the day GDFL's file starts late, is not).
fn at_open(ts: i64) -> bool {
    let secs = ts.div_euclid(MICROS) + pull::session::IST_OFFSET_SECS;
    let minute = secs.rem_euclid(DAY_SECS) / 60;
    let (open, _) = opening_minute(secs.div_euclid(DAY_SECS));
    minute <= i64::from(open)
}

/// `[00:00 IST, next 00:00 IST)` of `day`, UTC micros.
fn day_bounds(day: Day) -> (i64, i64) {
    let n = i64::from(day.days_from_epoch());
    (day_start(n), day_start(n + 1))
}

/// `YYYY-MM-DD HH:MM:SS` in IST.
fn ist_stamp(ts: i64) -> String {
    let secs = ts.div_euclid(MICROS) + pull::session::IST_OFFSET_SECS;
    let of_day = secs.rem_euclid(DAY_SECS);
    let day = u32::try_from(secs.div_euclid(DAY_SECS))
        .ok()
        .and_then(|days| Day::from_days(days).ok())
        .map_or_else(|| "?".to_owned(), |day| day.to_string());
    format!(
        "{day} {:02}:{:02}:{:02}",
        of_day / 3_600,
        (of_day % 3_600) / 60,
        of_day % 60
    )
}

/// `HH:MM:SS` in IST.
fn ist_clock(ts: i64) -> String {
    let of_day = (ts.div_euclid(MICROS) + pull::session::IST_OFFSET_SECS).rem_euclid(DAY_SECS);
    format!(
        "{:02}:{:02}:{:02}",
        of_day / 3_600,
        (of_day % 3_600) / 60,
        of_day % 60
    )
}

/// A day, `YYYY-MM-DD`, or a month, `YYYY-MM` (its first day, or its last
/// when `end`).
fn day_param(text: &str, name: &str, end: bool) -> Result<Day, String> {
    let bad = || format!("{name}={text:?} is not a day YYYY-MM-DD or a month YYYY-MM");
    let fields: Vec<&str> = text.split('-').collect();
    let number = |at: usize, width: usize| {
        fields
            .get(at)
            .filter(|field| field.len() == width && field.bytes().all(|b| b.is_ascii_digit()))
            .and_then(|field| field.parse::<u16>().ok())
    };
    let year = number(0, 4).ok_or_else(bad)?;
    let month = number(1, 2)
        .and_then(|m| u8::try_from(m).ok())
        .ok_or_else(bad)?;
    match fields.len() {
        2 => {
            let first = Day::new(year, month, 1).map_err(|_| bad())?;
            Ok(if end { first.end_of_month() } else { first })
        }
        3 => {
            let day = number(2, 2)
                .and_then(|d| u8::try_from(d).ok())
                .ok_or_else(bad)?;
            Day::new(year, month, day).map_err(|_| bad())
        }
        _ => Err(bad()),
    }
}

// ══ Wire words from the descriptor ═══════════════════════════════════════════

/// The offered feeds with a store prefix, in the descriptor table's order,
/// with their store vendor (D-3338: Zerodha and GDFL).
fn offered() -> impl Iterator<Item = (Feed, Vendor)> {
    Feed::ALL.into_iter().filter_map(|feed| {
        feed.descriptor()
            .offer
            .is_offered()
            .then_some(feed)
            .and_then(|feed| feed.store_vendor().map(|vendor| (feed, vendor)))
    })
}

/// What a rung of a feed is to its store.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    /// A rung the feed serves and the store files as served.
    Stored,
    /// A rung the pull folds from the feed's minutes and files.
    Derived,
    /// The feed's finest stored rung folded to a wider one on request.
    Fold,
    /// A rung directory the descriptor neither serves nor derives: named,
    /// never read as either.
    Unexpected,
}

impl Kind {
    const fn word(self) -> &'static str {
        match self {
            Self::Stored => "stored",
            Self::Derived => "derived",
            Self::Fold => "fold",
            Self::Unexpected => "unexpected",
        }
    }
}

/// What the store's rung `rung` of `feed` is: served, derived from the
/// minutes, or neither — `feeds_route::rungs_of`, the rule `/feeds.json`
/// states, and no second copy of it.
fn kind_of(feed: Feed, rung: Timeframe) -> Kind {
    let (served, derived) = super::feeds_route::rungs_of(feed);
    if served.iter().any(|word| *word == rung.as_str()) {
        Kind::Stored
    } else if !derived.iter().all(|tf| *tf != rung) {
        Kind::Derived
    } else {
        Kind::Unexpected
    }
}

/// D-3339's row tag: `<feed>_<kind>`, with a stored one-second rung named
/// `<feed>_1s`.
fn tag(vendor: Vendor, kind: Kind, rung: Timeframe) -> String {
    if kind == Kind::Stored && rung == Timeframe::SECOND_1 {
        format!("{}_1s", vendor.as_str())
    } else {
        format!("{}_{}", vendor.as_str(), kind.word())
    }
}

/// What a bar's close is on `feed` at `rung` (D-3324): the descriptor's
/// word, and a fold's close is the last print it folded.
fn close_of(feed: Feed, rung: Timeframe, kind: Kind) -> Option<CloseSemantics> {
    if kind == Kind::Fold {
        return Some(CloseSemantics::LastPrint);
    }
    feed.descriptor().basis.close_of(rung == Timeframe::DAY_1)
}

/// `close_of` as its wire word.
const fn close_word(close: CloseSemantics) -> &'static str {
    match close {
        CloseSemantics::Official => "official",
        CloseSemantics::LastPrint => "last_print",
    }
}

/// The price, volume and close basis of `feed`'s `segment` at `rung`, as
/// `/feeds.json` words it (D-3323, D-3324).
fn basis_json(feed: Feed, segment: Segment, rung: Timeframe, kind: Kind) -> Value {
    let close = close_of(feed, rung, kind);
    let close_word = close.map(close_word);
    let close_label = close.map(CloseSemantics::label);
    match feed.descriptor().basis {
        Basis::Unverified(why) => json!({
            "price": null, "volume": null, "adjusted_as_of": null,
            "close": null, "close_label": null, "unverified": why,
        }),
        Basis::Measured { .. } => match feed.descriptor().basis.segment(segment) {
            Some(row) => json!({
                "price": super::feeds_route::price_word(row.price),
                "price_label": row.price.label(),
                "volume": super::feeds_route::volume_word(row.volume),
                "volume_label": row.volume.label(),
                "adjusted_as_of": row.price.adjusted_as_of_pull_date().then_some("pull_date"),
                "close": close_word,
                "close_label": close_label,
                "source": row.source,
            }),
            None => json!({
                "price": null, "volume": null, "adjusted_as_of": null,
                "close": close_word, "close_label": close_label,
                "unverified": format!(
                    "{} records no basis for {}", feed.display(), segment.as_str()
                ),
            }),
        },
    }
}

/// Whether two feeds' prices for `segment` are on different bases: one
/// vendor-adjusted and the other not (D-3323). Read off the descriptor, so a
/// segment nobody measured is `None`, never assumed equal.
fn bases_differ(segment: Segment, one: Feed, other: Feed) -> Option<bool> {
    let price = |feed: Feed| {
        feed.descriptor()
            .basis
            .segment(segment)
            .map(|row| row.price)
    };
    Some(price(one)? != price(other)?)
}

/// A dated span as JSON, or `null`.
fn span_json(span: Option<Span>) -> Value {
    span.map_or(Value::Null, |span| {
        json!({
            "from": span.from.to_string(),
            "to": span.to.to_string(),
            "source": span.source,
        })
    })
}

/// Whether `month` lies wholly outside the span.
fn month_outside(month: YearMonth, span: Span) -> bool {
    Day::new(month.year(), month.month(), 1).is_ok_and(|first| {
        let last = first.end_of_month();
        last < span.from || first > span.to
    })
}

/// The label D-3338 gives a month outside the operator's window.
const OUTSIDE_WINDOW: &str = "outside your window";

/// The tolerances every comparison here is made against, as one object.
fn tolerances_json() -> Value {
    json!({
        "price_bps": decimal(PRICE_TOLERANCE_BPS, 0),
        "volume_percent": decimal(VOLUME_TOLERANCE_PERMILLE, 1),
        "factor_percent": decimal(FACTOR_THRESHOLD_PERMILLE, 1),
        "rule": "D-3339: a price disagrees past 1 bp, a volume past 0.5%; a share whose \
                 factor (GDFL ÷ Zerodha) differs from 1 by more than 0.2% is basis_differs and \
                 shows the factor instead of a disagreement, and a factor that moves by more \
                 than 0.2% between common days is basis_step. A day close is official and an \
                 intraday close the last print: the two are labelled, their difference shown, \
                 and never counted as a disagreement (D-3324). Chosen, not sourced.",
    })
}

// ══ /sources.json — the rung table ═══════════════════════════════════════════

/// `GET /sources.json`.
pub(super) async fn sources_json(
    axum::extract::State(site): axum::extract::State<Loaded>,
    uri: axum::http::Uri,
) -> Answer {
    let query = uri.query().unwrap_or("").to_owned();
    let inst = match Instrument::parse(&query) {
        Ok(inst) => inst,
        Err(why) => {
            return answer(
                axum::http::StatusCode::BAD_REQUEST,
                &json!({ "error": why }),
            );
        }
    };
    let site = Loaded::clone(&site);
    match crate::detail::run_store_read(move || rung_table(&site, &inst)).await {
        Ok(Ok(body)) => answer(axum::http::StatusCode::OK, &body),
        Ok(Err(refusal)) => answer(refusal.status, &refusal.json()),
        Err(why) => refused_admission("all sources rung table", &why),
    }
}

/// The 429 or 503 a blocking read that was not admitted answers.
fn refused_admission(what: &str, why: &crate::detail::RunError) -> Answer {
    let (status, body) =
        crate::detail::admission_refused(what, crate::detail::MAX_STORE_READ_CONCURRENT, why);
    (status, [(axum::http::header::CONTENT_TYPE, JSON)], body)
}

/// One rung of one feed as the store holds it.
struct Held {
    feed: Feed,
    vendor: Vendor,
    rung: Timeframe,
    kind: Kind,
    months: Arc<[YearMonth]>,
    /// The first and last record's stamps.
    span: Option<(i64, i64)>,
}

/// Every rung `feed` holds for `inst`, finest first, with the faults met.
fn held_rungs(
    root: &Path,
    feed: Feed,
    vendor: Vendor,
    inst: &Instrument,
) -> (Vec<Held>, Vec<String>) {
    let mut out = Vec::with_capacity(Timeframe::KNOWN.len());
    let mut faults = Vec::new();
    let mut rungs: Vec<Timeframe> = Timeframe::KNOWN.to_vec();
    // FINEST FIRST, by insertion: `KNOWN` puts the day first.
    rungs.rotate_left(1);
    for rung in rungs {
        let series = Series {
            inst: inst.clone(),
            vendor,
            rung,
        };
        match series.held(root) {
            Ok(months) if months.is_empty() => {}
            Ok(months) => out.push(Held {
                feed,
                vendor,
                rung,
                kind: kind_of(feed, rung),
                months,
                span: None,
            }),
            Err(why) => faults.push(why),
        }
    }
    (out, faults)
}

/// The rung table: every rung each offered feed holds for `inst`, stored,
/// derived or foldable. See the module header for what it costs.
///
/// # Errors
///
/// The store root is not a readable directory.
pub(super) fn rung_table(site: &Site, inst: &Instrument) -> Result<Value, Refusal> {
    let root = site.store_root.as_path();
    let meta = std::fs::metadata(root).map_err(|why| {
        Refusal::store(format!("cannot read store root {}: {why}", root.display()))
    })?;
    if !meta.is_dir() {
        return Err(Refusal::store(format!(
            "store root {} is not a directory",
            root.display()
        )));
    }
    // A KEY THE RE-KEY RETIRED IS GONE: its files are kept, so the walk
    // below would find them and call the month uncounted.
    refuse_retired(root, inst)?;
    let mut feeds = Vec::new();
    let mut rows = Vec::new();
    let mut notes: Vec<String> = Vec::new();
    // EACH OFFERED FEED THAT HOLDS NOTHING OF THE INSTRUMENT, named unless
    // none holds anything, which one note says instead.
    let mut holds_nothing: Vec<String> = Vec::new();
    for (feed, vendor) in offered() {
        let (_stamps, now) = vendor_census(site, vendor);
        let census = now.census();
        let (held, faults) = held_rungs(root, feed, vendor, inst);
        if held.is_empty() && faults.is_empty() {
            holds_nothing.push(format!(
                "{} holds no rung of {} in this store",
                feed.display(),
                inst.key()
            ));
        }
        feeds.push(feed_json(site, feed, vendor, census, held.len()));
        notes.extend(faults);
        rows.extend(feed_rows(site, inst, census, held));
    }
    if rows.is_empty() {
        notes.push(format!(
            "no offered feed holds any rung of {} in this store",
            inst.key()
        ));
    } else {
        notes.extend(holds_nothing);
    }
    Ok(json!({
        "schema": 1,
        "instrument": inst.json(),
        "feeds": feeds,
        "rungs": rows,
        "tolerances": tolerances_json(),
        "notes": notes,
    }))
}

/// One offered feed as the rung table heads it: its archive, the operator's
/// window, its census state and how many rungs of the instrument it holds.
fn feed_json(
    site: &Site,
    feed: Feed,
    vendor: Vendor,
    census: Option<&census::VendorCensus>,
    rungs_held: usize,
) -> Value {
    let behind = super::store_route::behind(site, vendor);
    let state = census.map_or("absent", |census| census.state_word(behind.days));
    json!({
        "feed": vendor.as_str(),
        "display": feed.display(),
        "archive": span_json(feed.descriptor().archive),
        "window": span_json(feed.descriptor().window).as_object().map(|object| {
            let mut object = object.clone();
            object.insert("label_outside".to_owned(), json!(OUTSIDE_WINDOW));
            Value::Object(object)
        }),
        "census": {
            "state": state,
            "generation": census.and_then(census::VendorCensus::generation),
            "note": census.map(census::VendorCensus::note),
            "degraded": census.and_then(census::VendorCensus::degraded),
            "behind_days": behind.kept.then_some(behind.days),
            "behind_from": behind.from.map(|day| day.to_string()),
        },
        "rungs_held": rungs_held,
    })
}

/// One feed's rows of the rung table: every rung it holds, then the wider
/// rungs its stored seconds fold to.
fn feed_rows(
    site: &Site,
    inst: &Instrument,
    census: Option<&census::VendorCensus>,
    mut held: Vec<Held>,
) -> Vec<Value> {
    let root = site.store_root.as_path();
    let mut rows = Vec::with_capacity(held.len() + Timeframe::KNOWN.len());
    // THE MINUTES A DERIVED RUNG IS MEASURED AGAINST, AND THE SECONDS A
    // FOLD IS MADE FROM, noted on the one walk that spans each rung.
    let mut minute: Option<MonthsAndSpan> = None;
    let mut second: Option<usize> = None;
    for (at, rung) in held.iter_mut().enumerate() {
        rung.span = span_of(root, inst, rung, census);
        if rung.rung == Timeframe::MINUTE_1 {
            minute = Some((Arc::clone(&rung.months), rung.span));
        }
        if rung.rung == Timeframe::SECOND_1 && rung.kind == Kind::Stored {
            second = Some(at);
        }
    }
    // EVERY STORED AND DERIVED RUNG'S SESSIONS, the minutes' among them: a
    // derived rung is partial where it lacks a session its minutes hold.
    let sessions: Vec<Sessions> = held
        .iter()
        .map(|rung| {
            if matches!(rung.kind, Kind::Stored | Kind::Derived) {
                sessions_of(site, rung.vendor, inst, rung.rung)
            } else {
                Sessions::default()
            }
        })
        .collect();
    let minute_not_held = held
        .iter()
        .zip(&sessions)
        .find(|(rung, _)| rung.rung == Timeframe::MINUTE_1)
        .and_then(|(_, sessions)| sessions.not_held.as_ref());
    let source = MinuteSource {
        months_and_span: minute.as_ref(),
        not_held: minute_not_held,
    };
    for (rung, sessions) in held.iter().zip(&sessions) {
        rows.push(rung_row(inst, rung, census, &source, sessions));
    }
    // A FEED WHOSE FINEST STORED RUNG IS THE SECOND IS FOLDABLE TO EVERY
    // WIDER RUNG ON REQUEST (D-3339 item 1): listed, never filed.
    if let Some(second) = second.and_then(|at| held.get(at)) {
        for &wider in Timeframe::KNOWN {
            if wider.secs() > Timeframe::SECOND_1.secs() {
                rows.push(fold_row(inst, second, wider));
            }
        }
    }
    rows
}

/// What `/calendar.json` says of one rung of a series: its answer as the
/// rung row carries it, and the owed sessions it holds no bar on.
#[derive(Debug, Clone, Default)]
struct Sessions {
    json: Value,
    not_held: Option<NotHeld>,
}

/// The owed sessions a rung holds no bar on.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct NotHeld {
    /// How many, whole.
    count: u64,
    /// The days named, `YYYY-MM-DD`.
    days: BTreeSet<String>,
    /// Whether `days` is every one of them (the calendar names them all and
    /// did not cut the list short).
    whole: bool,
}

/// The sessions the calendar of record owes a stored or derived rung of this
/// series, and the owed sessions it holds no bar on — the day-level holes a
/// month count cannot show (Zerodha's minutes around Diwali, DB-079).
///
/// `/calendar.json`'s own answer for the series (WP-35, D-3336), kept per
/// feed generation: one map probe when kept, and built once per generation
/// otherwise (measured for that route: about 0.1 s for a Zerodha series and
/// up to about 0.7 s for a GDFL share). Never derived a second time here. A
/// contract is traded on demand, so no calendar owes it a session and none is
/// asked.
fn sessions_of(site: &Site, vendor: Vendor, inst: &Instrument, rung: Timeframe) -> Sessions {
    if inst.contract.is_some() {
        return Sessions {
            json: json!({ "why": "trade_driven: a contract owes no session; its bars are its trades" }),
            not_held: None,
        };
    }
    let link = format!(
        "/calendar.json?feed={}&symbol={}&rung={}",
        vendor.as_str(),
        query_value(&inst.symbol),
        rung.as_str()
    );
    let Ok(uri) = link.parse::<axum::http::Uri>() else {
        return Sessions {
            json: json!({ "calendar": link, "why": "the calendar link would not parse" }),
            not_held: None,
        };
    };
    let (status, _, body) =
        super::calendar_route::calendar_json_reading(site, &uri, super::census_now_stamped);
    let answer: Value = serde_json::from_str(&body).unwrap_or(Value::Null);
    let field = |key: &str| answer.get(key).cloned().unwrap_or(Value::Null);
    if !status.is_success() || answer.get("status").and_then(Value::as_str) != Some("ok") {
        return Sessions {
            json: json!({
                "calendar": link,
                "status": status.as_u16(),
                "why": answer
                    .get("reason")
                    .or_else(|| answer.get("refusal"))
                    .or_else(|| answer.get("error"))
                    .cloned()
                    .unwrap_or_else(|| json!("the calendar answered no sessions")),
            }),
            not_held: None,
        };
    }
    let listed = answer.get("sessionsNotHeld");
    let days: BTreeSet<String> = listed
        .and_then(|held| held.get("days"))
        .and_then(Value::as_array)
        .map(|days| {
            days.iter()
                .filter_map(|day| day.get("date").and_then(Value::as_str).map(str::to_owned))
                .collect()
        })
        .unwrap_or_default();
    let count = listed
        .and_then(|held| held.get("count"))
        .and_then(Value::as_u64);
    let truncated = listed
        .and_then(|held| held.get("truncated"))
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let not_held = count.map(|count| NotHeld {
        count,
        whole: !truncated && u64::try_from(days.len()).is_ok_and(|named| named == count),
        days,
    });
    let named: Vec<&String> = not_held
        .as_ref()
        .map(|held| held.days.iter().take(MAX_NAMED).collect())
        .unwrap_or_default();
    Sessions {
        json: json!({
            "owed": field("sessions"),
            "not_held": count,
            "not_held_named": named,
            "partly_held": answer
                .get("sessionsPartlyHeld")
                .and_then(|held| held.get("count"))
                .cloned(),
            "from": field("fromDate"),
            "to": field("toDate"),
            "calendar": link,
        }),
        not_held,
    }
}

/// The owed sessions a derived rung lacks that its minutes hold: the
/// sessions the fold has not reached. The count, the days named (at most
/// [`MAX_NAMED`]) and whether the count is exact; where either calendar cut
/// its list short, the count is the difference of the two counts, a lower
/// bound, and says so.
fn sessions_behind(derived: &NotHeld, minutes: &NotHeld) -> (u64, Vec<String>, bool) {
    if derived.whole && minutes.whole {
        let missing: Vec<String> = derived.days.difference(&minutes.days).cloned().collect();
        let count = u64::try_from(missing.len()).unwrap_or(u64::MAX);
        (count, missing.into_iter().take(MAX_NAMED).collect(), true)
    } else {
        (
            derived.count.saturating_sub(minutes.count),
            Vec::new(),
            false,
        )
    }
}

/// The census entry of one month of a rung, when the census holds it.
fn census_entry(
    census: Option<&census::VendorCensus>,
    inst: &Instrument,
    rung: Timeframe,
    month: YearMonth,
) -> Option<Entry> {
    let census::Census::Held { ref manifest } = census?.state else {
        return None;
    };
    manifest.entry(&inst.census_key(rung, month)?)
}

/// A rung's first and last record stamps: the census's, where it counts the
/// edge months, else each edge month's own header.
fn span_of(
    root: &Path,
    inst: &Instrument,
    held: &Held,
    census: Option<&census::VendorCensus>,
) -> Option<(i64, i64)> {
    let series = Series {
        inst: inst.clone(),
        vendor: held.vendor,
        rung: held.rung,
    };
    let edge = |month: YearMonth| {
        census_entry(census, inst, held.rung, month)
            .map(|entry| (entry.first_ts_micros, entry.last_ts_micros))
            .or_else(|| {
                let file = series.open(root, month).ok()?;
                let header = file.header();
                (header.n_valid > 0).then_some((header.first_ts_micros, header.last_ts_micros))
            })
    };
    let first = edge(*held.months.first()?)?.0;
    let last = edge(*held.months.last()?)?.1;
    Some((first, last))
}

/// The months strictly between a series' first and last held month that it
/// does not hold: the holes, counted, the first [`MAX_NAMED`] named.
fn holes_of(months: &[YearMonth]) -> (usize, Vec<String>) {
    let mut count = 0_usize;
    let mut named = Vec::new();
    for pair in months.windows(2) {
        let (Some(&before), Some(&after)) = (pair.first(), pair.get(1)) else {
            continue;
        };
        let mut month = next_month(before);
        while let Some(at) = month.filter(|at| *at < after) {
            count += 1;
            if named.len() < MAX_NAMED {
                named.push(at.to_string());
            }
            month = next_month(at);
        }
    }
    (count, named)
}

/// The months of `source` that `held` lacks. Both are sorted, so this is
/// one walk of each: no search.
fn missing_from(source: &[YearMonth], held: &[YearMonth]) -> Vec<YearMonth> {
    let mut out = Vec::new();
    let mut theirs = held.iter().peekable();
    for &month in source {
        while theirs.next_if(|at| **at < month).is_some() {}
        if theirs.next_if(|at| **at == month).is_none() {
            out.push(month);
        }
    }
    out
}

/// What a derived rung is measured against: its minutes' held months and
/// span, and the owed sessions the minutes hold no bar on.
struct MinuteSource<'a> {
    months_and_span: Option<&'a MonthsAndSpan>,
    not_held: Option<&'a NotHeld>,
}

/// One stored or derived rung as a row of the table.
#[expect(
    clippy::too_many_lines,
    reason = "one rung's row written in the order the wire carries it, field by field"
)]
fn rung_row(
    inst: &Instrument,
    held: &Held,
    census: Option<&census::VendorCensus>,
    minute: &MinuteSource<'_>,
    sessions: &Sessions,
) -> Value {
    let months = &held.months;
    let (holes, named_holes) = holes_of(months);
    // THE CENSUS, ONE PROBE A HELD MONTH: what it counts, and the months on
    // disk it does not (a recount owed, D-3317, D-3327).
    let mut census_rows = 0_u64;
    let mut counted = 0_usize;
    let mut uncounted = Vec::new();
    let mut uncounted_count = 0_usize;
    let census_held = census.is_some_and(|c| matches!(c.state, census::Census::Held { .. }));
    for &month in months.iter() {
        if let Some(entry) = census_entry(census, inst, held.rung, month) {
            census_rows = census_rows.saturating_add(entry.rows);
            counted += 1;
        } else {
            uncounted_count += 1;
            if uncounted.len() < MAX_NAMED {
                uncounted.push(month.to_string());
            }
        }
    }
    // OUTSIDE THE OPERATOR'S WINDOW (D-3338): counted and labelled, never
    // hidden from a listing and never deleted.
    let window = held.feed.descriptor().window;
    let outside: Vec<YearMonth> = window.map_or_else(Vec::new, |span| {
        months
            .iter()
            .copied()
            .filter(|month| month_outside(*month, span))
            .collect()
    });
    let outside_json = (!outside.is_empty()).then(|| {
        json!({
            "months": outside.len(),
            "first": outside.first().map(ToString::to_string),
            "last": outside.last().map(ToString::to_string),
            "label": OUTSIDE_WINDOW,
            "db": inst.db_link(held.vendor, held.rung, true),
        })
    });
    // A DERIVED RUNG BEHIND ITS SOURCE (DB-G41's derived half): the months
    // the minutes hold that it does not, how many days its last record
    // trails the minutes' last, and the owed sessions the minutes hold that
    // it lacks (NIFTY's derived rungs lacked four on 10 Oct 2026 while every
    // month and the last day matched).
    let behind = (held.kind == Kind::Derived)
        .then(|| {
            let (source_months, source_span) = minute.months_and_span?;
            let missing = missing_from(source_months, months);
            let source_last = source_span.map(|(_, last)| day_number(last));
            let last = held.span.map(|(_, last)| day_number(last));
            let days = source_last.zip(last).map(|(s, l)| (s - l).max(0));
            let sessions_missing = sessions
                .not_held
                .as_ref()
                .zip(minute.not_held)
                .map(|(derived, minutes)| sessions_behind(derived, minutes));
            let sessions_short = sessions_missing
                .as_ref()
                .is_some_and(|(count, _, _)| *count > 0);
            (!missing.is_empty() || days.is_some_and(|d| d > 0) || sessions_short).then(|| {
                json!({
                    "source_rung": Timeframe::MINUTE_1.as_str(),
                    "source_last_day": source_span.map(|(_, last)| ist_day_text(last)),
                    "last_day": held.span.map(|(_, last)| ist_day_text(last)),
                    "days_behind": days,
                    "months_missing": missing.len(),
                    "months_missing_named": missing
                        .iter()
                        .take(MAX_NAMED)
                        .map(ToString::to_string)
                        .collect::<Vec<_>>(),
                    "sessions_missing": sessions_missing.as_ref().map(|(count, _, _)| *count),
                    "sessions_missing_named": sessions_missing
                        .as_ref()
                        .map(|(_, named, _)| named.clone()),
                    "sessions_missing_exact": sessions_missing
                        .as_ref()
                        .map(|(_, _, exact)| *exact),
                })
            })
        })
        .flatten();
    // A DERIVED RUNG WHOSE SESSIONS COULD NOT BE SET AGAINST ITS MINUTES'
    // (either calendar refused) is not called whole: it says so.
    let sessions_unchecked =
        held.kind == Kind::Derived && (sessions.not_held.is_none() || minute.not_held.is_none());
    let state = if held.kind == Kind::Unexpected {
        "unexpected"
    } else if behind.is_some() {
        "partial"
    } else if !census_held || uncounted_count > 0 {
        "uncounted"
    } else if sessions_unchecked {
        "sessions_unchecked"
    } else {
        "held"
    };
    json!({
        "tag": tag(held.vendor, held.kind, held.rung),
        "feed": held.vendor.as_str(),
        "rung": held.rung.as_str(),
        "kind": held.kind.word(),
        "stored": held.kind != Kind::Fold,
        "derived_from": (held.kind == Kind::Derived).then_some(Timeframe::MINUTE_1.as_str()),
        "first_day": held.span.map(|(first, _)| ist_day_text(first)),
        "last_day": held.span.map(|(_, last)| ist_day_text(last)),
        "first_month": months.first().map(ToString::to_string),
        "last_month": months.last().map(ToString::to_string),
        "months_held": months.len(),
        "months_absent": holes,
        "holes": named_holes,
        "census": {
            "rows": census_rows,
            "months_counted": counted,
            "months_uncounted": uncounted_count,
            "uncounted": uncounted,
        },
        "basis": basis_json(held.feed, inst.segment, held.rung, held.kind),
        "archive": span_json(held.feed.descriptor().archive),
        "outside_window": outside_json,
        "behind_source": behind,
        "state": state,
        "sessions": sessions.json.clone(),
        "db": inst.db_link(held.vendor, held.rung, false),
    })
}

/// One wider rung the feed's seconds fold to, on request (`gdfl_fold`).
fn fold_row(inst: &Instrument, second: &Held, wider: Timeframe) -> Value {
    json!({
        "tag": tag(second.vendor, Kind::Fold, wider),
        "feed": second.vendor.as_str(),
        "rung": wider.as_str(),
        "kind": Kind::Fold.word(),
        "stored": false,
        "derived_from": Timeframe::SECOND_1.as_str(),
        "computed": "on_request",
        "fold": "pull::fold: open first, high max, low min, close last, volume sum, open interest last carried",
        "first_day": second.span.map(|(first, _)| ist_day_text(first)),
        "last_day": second.span.map(|(_, last)| ist_day_text(last)),
        "first_month": second.months.first().map(ToString::to_string),
        "last_month": second.months.last().map(ToString::to_string),
        "months_held": second.months.len(),
        "basis": basis_json(second.feed, inst.segment, wider, Kind::Fold),
        "archive": span_json(second.feed.descriptor().archive),
        "state": "on_request",
        "bars": format!(
            "/sources/bars.json?exchange={}&segment={}&symbol={}{}&rung={}",
            inst.exchange.as_str(),
            inst.segment.as_str(),
            query_value(&inst.symbol),
            inst.contract
                .map_or_else(String::new, |c| format!("&contract={}", c.as_str())),
            wider.as_str()
        ),
    })
}

/// The IST day of a stamp, `YYYY-MM-DD`.
fn ist_day_text(ts: i64) -> String {
    u32::try_from(day_number(ts))
        .ok()
        .and_then(|days| Day::from_days(days).ok())
        .map_or_else(|| "?".to_owned(), |day| day.to_string())
}

// ══ /sources/bars.json — one timeline ════════════════════════════════════════

/// What one timeline request asks for, read and checked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimelineAsk {
    inst: Instrument,
    rung: Timeframe,
    from: Day,
    to: Day,
    desc: bool,
    offset: usize,
    limit: usize,
    /// A bucket start, UTC micros: the page starts at this bucket.
    cursor: Option<i64>,
}

impl TimelineAsk {
    /// Reads one, or names the first thing wrong with it.
    ///
    /// # Errors
    ///
    /// The parameter and why, as one sentence.
    pub fn parse(query: &str) -> Result<Self, String> {
        let inst = Instrument::parse(query)?;
        let rung_text = param(query, "rung");
        let rung = Timeframe::KNOWN
            .iter()
            .copied()
            .find(|tf| tf.as_str() == rung_text)
            .ok_or_else(|| {
                format!(
                    "rung={rung_text:?} is not a rung this store has. It holds {}.",
                    Timeframe::KNOWN
                        .iter()
                        .map(|tf| tf.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            })?;
        let from_text = param(query, "from");
        if from_text.is_empty() {
            return Err(
                "from=YYYY-MM-DD is required: a timeline never guesses its range".to_owned(),
            );
        }
        let from = day_param(&from_text, "from", false)?;
        let to_text = param(query, "to");
        let to = if to_text.is_empty() {
            if from_text.len() == 7 {
                from.end_of_month()
            } else {
                from
            }
        } else {
            day_param(&to_text, "to", true)?
        };
        if from > to {
            return Err(format!("from={from} is after to={to}"));
        }
        let desc = match param(query, "dir").as_str() {
            "" | "desc" => true,
            "asc" => false,
            other => return Err(format!("dir={other:?} is not asc or desc")),
        };
        let number = |key: &str| -> Result<Option<usize>, String> {
            let raw = param(query, key);
            if raw.is_empty() {
                return Ok(None);
            }
            raw.parse::<usize>()
                .ok()
                .filter(|n| n.to_string() == raw)
                .map(Some)
                .ok_or_else(|| format!("{key}={raw:?} is not a whole number"))
        };
        let most = if rung == Timeframe::DAY_1 {
            MAX_DAY_LIMIT
        } else {
            MAX_LIMIT
        };
        let limit = number("limit")?.unwrap_or(DEFAULT_LIMIT.min(most));
        if limit == 0 || limit > most {
            return Err(format!(
                "limit={limit} is not a page this timeline serves at {}: 1 to {most}",
                rung.as_str()
            ));
        }
        let offset = number("offset")?.unwrap_or(0);
        if offset > MAX_OFFSET {
            return Err(format!(
                "offset={offset} is past {MAX_OFFSET}: continue with cursor=<unix seconds> from \
                 the answer's next_cursor, which costs one page wherever it is"
            ));
        }
        let cursor_text = param(query, "cursor");
        let cursor = if cursor_text.is_empty() {
            None
        } else {
            Some(
                cursor_text
                    .parse::<i64>()
                    .ok()
                    .and_then(|secs| secs.checked_mul(MICROS))
                    .ok_or_else(|| {
                        format!("cursor={cursor_text:?} is not a whole number of unix seconds")
                    })?,
            )
        };
        Ok(Self {
            inst,
            rung,
            from,
            to,
            desc,
            offset,
            limit,
            cursor,
        })
    }
}

/// `GET /sources/bars.json`.
pub(super) async fn sources_bars_json(
    axum::extract::State(site): axum::extract::State<Loaded>,
    uri: axum::http::Uri,
) -> Answer {
    let query = uri.query().unwrap_or("").to_owned();
    let asked = match TimelineAsk::parse(&query) {
        Ok(asked) => asked,
        Err(why) => {
            return answer(
                axum::http::StatusCode::BAD_REQUEST,
                &json!({ "error": why }),
            );
        }
    };
    let root = site.store_root.clone();
    match crate::detail::run_store_read(move || timeline(&root, &asked)).await {
        Ok(Ok(body)) => answer(axum::http::StatusCode::OK, &body),
        Ok(Err(refusal)) => answer(refusal.status, &refusal.json()),
        Err(why) => refused_admission("all sources timeline", &why),
    }
}

/// One source of a timeline.
struct Source {
    feed: Feed,
    kind: Kind,
    series: Series,
    held: Arc<[YearMonth]>,
}

impl Source {
    fn tag(&self, rung: Timeframe) -> String {
        tag(self.series.vendor, self.kind, rung)
    }
}

/// The sources of `asked`'s rung: each offered feed's stored or derived
/// rung, and a fold of each feed's stored seconds where the rung is wider;
/// and, for each offered feed that gives none, why — never a silent absence.
fn sources_of(root: &Path, asked: &TimelineAsk) -> Result<(Vec<Source>, Vec<String>), Refusal> {
    let mut out = Vec::with_capacity(4);
    let mut absent = Vec::new();
    for (feed, vendor) in offered() {
        let at_rung = Series {
            inst: asked.inst.clone(),
            vendor,
            rung: asked.rung,
        };
        let held = at_rung.held(root).map_err(Refusal::store)?;
        let kind = kind_of(feed, asked.rung);
        if !held.is_empty() && matches!(kind, Kind::Stored | Kind::Derived) {
            out.push(Source {
                feed,
                kind,
                series: at_rung,
                held,
            });
            continue;
        }
        if asked.rung.secs() > Timeframe::SECOND_1.secs()
            && kind_of(feed, Timeframe::SECOND_1) == Kind::Stored
        {
            let seconds = Series {
                inst: asked.inst.clone(),
                vendor,
                rung: Timeframe::SECOND_1,
            };
            let held = seconds.held(root).map_err(Refusal::store)?;
            if !held.is_empty() {
                out.push(Source {
                    feed,
                    kind: Kind::Fold,
                    series: seconds,
                    held,
                });
                continue;
            }
        }
        absent.push(match kind {
            Kind::Stored | Kind::Derived => format!(
                "{} holds no {} month of {}, and no finer stored rung of it folds to {}",
                feed.display(),
                asked.rung.as_str(),
                asked.inst.key(),
                asked.rung.as_str()
            ),
            Kind::Fold | Kind::Unexpected => format!(
                "{} neither serves nor derives a {} rung, and holds no one-second month of {} \
                 to fold to it",
                feed.display(),
                asked.rung.as_str(),
                asked.inst.key()
            ),
        });
    }
    Ok((out, absent))
}

/// One source's buckets of a page, in reading order.
struct Read {
    cells: Vec<Folded>,
    exhausted: bool,
}

/// The first `need` buckets of `source` in `range`, in `desc` order.
fn read_source(
    root: &Path,
    source: &Source,
    rung: Timeframe,
    range: (i64, i64),
    desc: bool,
    need: usize,
    budget: &mut Budget,
) -> Result<Read, Refusal> {
    let mut rows = Rows::new(root, &source.series, &source.held, range, desc);
    if source.kind == Kind::Fold {
        let (cells, exhausted) = fold_buckets(&mut rows, bucket_of(rung)?, need, budget)?;
        return Ok(Read { cells, exhausted });
    }
    let mut cells = Vec::with_capacity(need);
    while cells.len() < need {
        let Some(bar) = rows.next(budget)? else {
            return Ok(Read {
                cells,
                exhausted: true,
            });
        };
        cells.push(Folded {
            bar,
            rows: 1,
            first: bar.ts_micros,
        });
    }
    Ok(Read {
        cells,
        exhausted: false,
    })
}

/// The bucket starts every source holds, merged in reading order, each once.
///
/// The sources' lists are already in reading order, so this is a k-way merge
/// of at most four short lists: no sort.
fn union_of(reads: &[Read], desc: bool) -> Vec<i64> {
    let mut at = vec![0_usize; reads.len()];
    let mut out = Vec::with_capacity(reads.iter().map(|read| read.cells.len()).sum());
    loop {
        let mut best: Option<i64> = None;
        for (n, read) in reads.iter().enumerate() {
            if let Some(cell) = at.get(n).and_then(|&i| read.cells.get(i)) {
                let ts = cell.bar.ts_micros;
                let better = best.is_none_or(|b| if desc { ts > b } else { ts < b });
                if better {
                    best = Some(ts);
                }
            }
        }
        let Some(ts) = best else {
            return out;
        };
        for (n, read) in reads.iter().enumerate() {
            if let Some(i) = at.get_mut(n)
                && read
                    .cells
                    .get(*i)
                    .is_some_and(|cell| cell.bar.ts_micros == ts)
            {
                *i += 1;
            }
        }
        out.push(ts);
    }
}

/// One timeline page. See the module header for its cost.
///
/// # Errors
///
/// The store could not be read, or the page would read more than
/// [`MAX_ROWS_READ`] records.
#[expect(
    clippy::too_many_lines,
    reason = "one page read, merged and written in the order the wire carries it"
)]
pub fn timeline(root: &Path, asked: &TimelineAsk) -> Result<Value, Refusal> {
    refuse_retired(root, &asked.inst)?;
    let (sources, absent) = sources_of(root, asked)?;
    let (range_lo, range_hi) = (day_bounds(asked.from).0, day_bounds(asked.to).1);
    let bucket = bucket_of(asked.rung)?;
    // THE CURSOR IS A BUCKET: an instant inside one is moved to its start on
    // `pull::fold`'s grid, so a page never splits a bucket.
    let cursor = asked
        .cursor
        .map(|at| bucket_start(at, bucket))
        .transpose()?;
    let range = match cursor {
        None => (range_lo, range_hi),
        Some(start) if asked.desc => {
            let end = start.saturating_add(i64::from(asked.rung.secs()) * MICROS);
            (range_lo, end.min(range_hi))
        }
        Some(start) => (start.max(range_lo), range_hi),
    };
    let need = asked.offset + asked.limit + 1;
    let mut budget = Budget::new();
    let mut reads = Vec::with_capacity(sources.len());
    for source in &sources {
        reads.push(read_source(
            root,
            source,
            asked.rung,
            range,
            asked.desc,
            need,
            &mut budget,
        )?);
    }
    let union = union_of(&reads, asked.desc);
    let page: Vec<i64> = union
        .iter()
        .copied()
        .skip(asked.offset)
        .take(asked.limit)
        .collect();
    let next_cursor = union
        .get(asked.offset + asked.limit)
        .map(|ts| ts.div_euclid(MICROS));
    // WHERE EACH SOURCE'S CELLS ARE, by bucket start: a hash probe a cell.
    let mut cells: Vec<HashMap<i64, Folded>> = Vec::with_capacity(reads.len());
    for read in &reads {
        let mut by_ts = HashMap::with_capacity(read.cells.len());
        for cell in &read.cells {
            by_ts.insert(cell.bar.ts_micros, *cell);
        }
        cells.push(by_ts);
    }
    let segment = asked.inst.segment;
    let mut rows = Vec::with_capacity(page.len());
    let mut tally = Tally::default();
    for ts in &page {
        let mut row_cells = serde_json::Map::with_capacity(sources.len());
        let mut present: Vec<(&Source, Folded)> = Vec::with_capacity(sources.len());
        for (source, by_ts) in sources.iter().zip(&cells) {
            if let Some(cell) = by_ts.get(ts) {
                present.push((source, *cell));
            }
            row_cells.insert(
                source.series.vendor.as_str().to_owned(),
                by_ts.get(ts).map_or(Value::Null, |cell| {
                    cell_json(source, asked.rung, segment, cell)
                }),
            );
        }
        let compared = compare_row(&present, asked.rung, segment);
        tally.count(&compared, &present);
        rows.push(json!({
            "t": ts.div_euclid(MICROS),
            "ist": ist_stamp(*ts),
            "cells": row_cells,
            "only": (present.len() == 1 && sources.len() > 1)
                .then(|| present.first().map(|(source, _)| source.series.vendor.as_str())),
            "cmp": compared.json,
            "flags": compared.flags,
        }));
    }
    let complete = reads.iter().all(|read| read.exhausted) && next_cursor.is_none();
    Ok(json!({
        "schema": 1,
        "instrument": asked.inst.json(),
        "rung": asked.rung.as_str(),
        "range": { "from": asked.from.to_string(), "to": asked.to.to_string() },
        "dir": if asked.desc { "desc" } else { "asc" },
        "offset": asked.offset,
        "limit": asked.limit,
        "cursor": asked.cursor.map(|at| at.div_euclid(MICROS)),
        "sources": sources.iter().zip(&reads).map(|(source, read)| json!({
            "tag": source.tag(asked.rung),
            "feed": source.series.vendor.as_str(),
            "kind": source.kind.word(),
            "read_rung": source.series.rung.as_str(),
            "buckets_read": read.cells.len(),
            "exhausted": read.exhausted,
            "basis": basis_json(source.feed, segment, asked.rung, source.kind),
        })).collect::<Vec<_>>(),
        "rows": rows,
        "next_cursor": next_cursor,
        "complete": complete,
        "summary": tally.json(),
        "records_read": budget.used(),
        "tolerances": tolerances_json(),
        "notes": timeline_notes(&sources, absent, asked),
    }))
}

/// What a timeline page says about itself: each offered feed that gives no
/// source and why, first.
fn timeline_notes(sources: &[Source], absent: Vec<String>, asked: &TimelineAsk) -> Vec<String> {
    let mut notes = absent;
    if sources.is_empty() {
        notes.push(format!(
            "no offered feed holds {} at {} or a finer rung that folds to it",
            asked.inst.key(),
            asked.rung.as_str()
        ));
    } else if sources.len() == 1 {
        notes.push(
            "one source holds this rung, so nothing is compared: each cell is that feed's alone"
                .to_owned(),
        );
    }
    if asked.inst.segment == Segment::Index {
        notes.push(
            "an index has no traded volume (D-2802, D-3322): volume is never compared".to_owned(),
        );
    }
    notes
}

/// One cell: a source's bar for one bucket.
fn cell_json(source: &Source, rung: Timeframe, segment: Segment, cell: &Folded) -> Value {
    let bar = cell.bar;
    let oi = (segment == Segment::Fno && bar.open_interest != OI_NULL).then_some(bar.open_interest);
    json!({
        "tag": source.tag(rung),
        "o": bar.open, "h": bar.high, "l": bar.low, "c": bar.close, "v": bar.volume,
        "oi": oi,
        "oi_why": if segment == Segment::Fno {
            (bar.open_interest == OI_NULL).then_some("not_recorded")
        } else {
            Some("not_a_derivative")
        },
        "close": close_of(source.feed, rung, source.kind).map(close_word),
        "seconds": (source.kind == Kind::Fold).then_some(cell.rows),
        "first_at": (source.kind == Kind::Fold).then(|| ist_clock(cell.first)),
    })
}

/// A row's comparison and its flags.
struct Compared {
    json: Value,
    flags: Vec<&'static str>,
    /// Price fields beyond tolerance.
    disagree: usize,
    /// How the two bases stood on this row.
    basis: RowBasis,
    /// Whether open, high, low and close are exactly equal, where the two
    /// cells were compared on one basis.
    equal: Option<[bool; 4]>,
}

/// How two cells' price bases stood on one row (D-3323, D-3339 item 3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RowBasis {
    /// Nothing was compared: one cell, or one feed.
    NotCompared,
    /// Compared on one basis: the same basis, or a factor within 0.2% of 1.
    One,
    /// The factor differs from 1 by more than 0.2%: the factor replaces the
    /// disagreement flags.
    Differs,
    /// The bases may differ and no same-instant pair measures the factor on
    /// this row: nothing is judged, and the row says why.
    Unmeasured,
}

/// What a row's factor was measured on, and its two prices (GDFL's first).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FactorPair {
    /// GDFL's close ÷ Zerodha's close of the same bucket, both last prints.
    Close(i64, i64),
    /// GDFL's first print ÷ Zerodha's official open, at the day rung, where
    /// the first print is the session's opening print: D-3323's measure.
    Open(i64, i64),
    /// No same-instant pair: GDFL's first print of the day is after the
    /// opening minute, and an official close is not a last print (D-3324).
    None,
}

impl FactorPair {
    const fn prices(self) -> Option<(i64, i64)> {
        match self {
            Self::Close(g, z) | Self::Open(g, z) => Some((g, z)),
            Self::None => None,
        }
    }

    const fn words(self) -> &'static str {
        match self {
            Self::Close(..) => "GDFL close ÷ Zerodha close of the same bucket (D-3339 item 3)",
            Self::Open(..) => {
                "GDFL first print ÷ Zerodha official open: the day rung's closes are official \
                 against last print (D-3324), so the opening print is the same instant \
                 (D-3323's measure)"
            }
            Self::None => {
                "unmeasured: GDFL's first print of the day is after the opening minute and an \
                 official close is not a last print, so no pair is the same instant"
            }
        }
    }
}

/// Compares the two feeds' cells of one bucket field by field (D-3339).
#[expect(
    clippy::too_many_lines,
    reason = "one bucket's comparison, field by field in the order the wire carries them"
)]
fn compare_row(present: &[(&Source, Folded)], rung: Timeframe, segment: Segment) -> Compared {
    let none = Compared {
        json: Value::Null,
        flags: Vec::new(),
        disagree: 0,
        basis: RowBasis::NotCompared,
        equal: None,
    };
    // TWO CELLS FROM TWO FEEDS, or nothing to compare.
    let (Some(&(one, a)), Some(&(other, b))) = (present.first(), present.get(1)) else {
        return none;
    };
    if one.series.vendor == other.series.vendor {
        return none;
    }
    // THE BASE AND THE OTHER, BY THEIR ROLE (D-3339 item 3: the factor is
    // GDFL ÷ Zerodha): the base is the feed whose price is vendor-adjusted in
    // this segment, else the one that holds an official day close.
    let is_base = |source: &Source| {
        let basis = source.feed.descriptor().basis;
        basis
            .segment(segment)
            .is_some_and(|row| row.price.adjusted_as_of_pull_date())
            || basis.close_of(true).is_some()
    };
    let ((z_source, z_cell), (g_source, g_cell)) = if !is_base(one) && is_base(other) {
        ((other, b), (one, a))
    } else {
        ((one, a), (other, b))
    };
    let (z, g) = (z_cell.bar, g_cell.bar);
    let z_close = close_of(z_source.feed, rung, z_source.kind);
    let g_close = close_of(g_source.feed, rung, g_source.kind);
    let semantics_differ = z_close.zip(g_close).is_some_and(|(x, y)| x != y);
    let mut flags = Vec::new();
    let mut fields = serde_json::Map::with_capacity(9);
    // THE FACTOR, FOR A SHARE WHOSE BASES MAY DIFFER (D-3323, D-3339 item 3),
    // ALWAYS ON A SAME-INSTANT PAIR: closes of one bucket, or at the day rung
    // (official close against last print) the opening print against the
    // official open — and only where GDFL's first print IS the opening one.
    let bases = bases_differ(segment, z_source.feed, g_source.feed);
    let shares = segment == Segment::Cash;
    // AT THE DAY RUNG, WHETHER GDFL'S FOLD OPENS ON THE SESSION'S OPENING
    // PRINT: where it does not (its file starts late), its open is a later
    // print and is not set against the official open as one.
    let late_open = semantics_differ && !at_open(g_cell.first);
    let pair = if semantics_differ {
        if late_open {
            FactorPair::None
        } else {
            FactorPair::Open(g.open, z.open)
        }
    } else {
        FactorPair::Close(g.close, z.close)
    };
    let factor = shares
        .then(|| pair.prices().and_then(|(gp, zp)| ratio4(gp, zp)))
        .flatten();
    let row_basis = if !shares || bases != Some(true) {
        RowBasis::One
    } else {
        match pair.prices() {
            Some((gp, zp)) if basis_differs(gp, zp) => RowBasis::Differs,
            Some(_) => RowBasis::One,
            None => RowBasis::Unmeasured,
        }
    };
    match row_basis {
        RowBasis::Differs => flags.push("basis_differs"),
        RowBasis::Unmeasured => flags.push("basis_unmeasured"),
        RowBasis::One | RowBasis::NotCompared => {}
    }
    let mut disagree = 0_usize;
    let mut equal = [false; 4];
    for (at, (name, gv, zv)) in [
        ("o", g.open, z.open),
        ("h", g.high, z.high),
        ("l", g.low, z.low),
        ("c", g.close, z.close),
    ]
    .into_iter()
    .enumerate()
    {
        if let Some(slot) = equal.get_mut(at) {
            *slot = gv == zv;
        }
        let Some((bps, beyond)) = price_cmp(gv, zv) else {
            fields.insert(name.to_owned(), json!({ "why": "not_a_price" }));
            continue;
        };
        match row_basis {
            RowBasis::Differs => {
                fields.insert(
                    name.to_owned(),
                    json!({ "bps": bps, "why": "basis_differs" }),
                );
                continue;
            }
            RowBasis::Unmeasured => {
                fields.insert(
                    name.to_owned(),
                    json!({ "bps": bps, "flag": null, "why": "basis_unmeasured" }),
                );
                continue;
            }
            RowBasis::One | RowBasis::NotCompared => {}
        }
        if name == "o" && late_open {
            fields.insert(
                name.to_owned(),
                json!({ "bps": bps, "flag": null, "why": "gdfl_first_print_after_open" }),
            );
            flags.push("gdfl_first_print_after_open");
            continue;
        }
        if name == "c" && semantics_differ {
            // AN OFFICIAL CLOSE IS NEVER A DISAGREEMENT WITH A LAST PRINT
            // (D-3324): labelled, its difference shown.
            fields.insert(
                name.to_owned(),
                json!({
                    "bps": bps, "flag": false, "equal": gv == zv, "paisa": gv - zv,
                    "semantics": "official_vs_last_print",
                }),
            );
            if gv != zv {
                flags.push("official_vs_last_print");
            }
            continue;
        }
        if beyond {
            disagree += 1;
            flags.push(match name {
                "o" => "o",
                "h" => "h",
                "l" => "l",
                _ => "c",
            });
        }
        fields.insert(
            name.to_owned(),
            json!({ "bps": bps, "flag": beyond, "equal": gv == zv, "paisa": gv - zv }),
        );
    }
    let volume = if segment == Segment::Index {
        json!({ "why": "index_no_volume" })
    } else {
        match row_basis {
            RowBasis::Differs => json!({
                "factor": ratio4(z.volume, g.volume),
                "why": "basis_differs",
            }),
            RowBasis::Unmeasured => json!({
                "factor": ratio4(z.volume, g.volume),
                "flag": null,
                "why": "basis_unmeasured",
            }),
            RowBasis::One | RowBasis::NotCompared => match volume_cmp(g.volume, z.volume) {
                Some((pct, beyond)) => {
                    if beyond {
                        flags.push("v");
                    }
                    json!({ "percent": pct, "flag": beyond, "equal": g.volume == z.volume })
                }
                None => json!({ "why": "no_volume_to_compare" }),
            },
        }
    };
    fields.insert("v".to_owned(), volume);
    fields.insert("factor".to_owned(), factor.unwrap_or(Value::Null));
    fields.insert(
        "factor_of".to_owned(),
        if shares {
            json!(pair.words())
        } else {
            Value::Null
        },
    );
    fields.insert(
        "basis".to_owned(),
        json!(match (shares, bases, row_basis) {
            (true, Some(true), RowBasis::Differs) => "basis_differs",
            (true, Some(true), RowBasis::Unmeasured) => "basis_unmeasured",
            (true, Some(true), _) => "adjusted_but_equal_here",
            (_, Some(false), _) => "same",
            _ => "unverified",
        }),
    );
    fields.insert(
        "base".to_owned(),
        json!({ "of": z_source.series.vendor.as_str(), "against": g_source.series.vendor.as_str() }),
    );
    let compared_on_one = row_basis == RowBasis::One;
    Compared {
        json: Value::Object(fields),
        flags,
        disagree,
        basis: row_basis,
        equal: compared_on_one.then_some(equal),
    }
}

/// What one page's rows add up to.
#[derive(Debug, Default)]
struct Tally {
    rows: usize,
    both: usize,
    only: usize,
    agree: usize,
    disagree: usize,
    basis_differs: usize,
    basis_unmeasured: usize,
    /// Rows compared on one basis.
    compared: usize,
    /// Of those, open, high, low and close exactly equal.
    exact: [usize; 4],
}

impl Tally {
    fn count(&mut self, compared: &Compared, present: &[(&Source, Folded)]) {
        self.rows += 1;
        if present.len() > 1 {
            self.both += 1;
            match compared.basis {
                RowBasis::Differs => self.basis_differs += 1,
                RowBasis::Unmeasured => self.basis_unmeasured += 1,
                RowBasis::One | RowBasis::NotCompared if compared.disagree > 0 => {
                    self.disagree += 1;
                }
                RowBasis::One | RowBasis::NotCompared => self.agree += 1,
            }
            if let Some(equal) = compared.equal {
                self.compared += 1;
                for (count, same) in self.exact.iter_mut().zip(equal) {
                    *count += usize::from(same);
                }
            }
        } else {
            self.only += 1;
        }
    }

    fn json(&self) -> Value {
        let [o, h, l, c] = self.exact;
        json!({
            "rows": self.rows,
            "both_feeds": self.both,
            "one_feed": self.only,
            "prices_within_tolerance": self.agree,
            "prices_disagree": self.disagree,
            "basis_differs": self.basis_differs,
            "basis_unmeasured": self.basis_unmeasured,
            "compared_on_one_basis": self.compared,
            "exactly_equal": { "o": o, "h": h, "l": l, "c": c },
        })
    }
}

// ══ /sources/day.json — the day reconciliation ═══════════════════════════════

/// What one day-table request asks for, read and checked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DayAsk {
    inst: Instrument,
    from: Day,
    to: Day,
    newest_first: bool,
    page: u32,
}

impl DayAsk {
    /// Reads one, or names the first thing wrong with it.
    ///
    /// # Errors
    ///
    /// The parameter and why, as one sentence.
    pub fn parse(query: &str) -> Result<Self, String> {
        let inst = Instrument::parse(query)?;
        let from_text = param(query, "from");
        if from_text.is_empty() {
            return Err(
                "from=YYYY-MM-DD or from=YYYY-MM is required: the day table never guesses a range"
                    .to_owned(),
            );
        }
        let from = day_param(&from_text, "from", false)?;
        let to_text = param(query, "to");
        let to = if to_text.is_empty() {
            if from_text.len() == 7 {
                from.end_of_month()
            } else {
                from
            }
        } else {
            day_param(&to_text, "to", true)?
        };
        if from > to {
            return Err(format!("from={from} is after to={to}"));
        }
        let newest_first = match param(query, "order").as_str() {
            "" | "asc" => false,
            "desc" => true,
            other => return Err(format!("order={other:?} is not asc or desc")),
        };
        let page_text = param(query, "page");
        let page = if page_text.is_empty() {
            0
        } else {
            page_text
                .parse::<u32>()
                .ok()
                .filter(|page| page.to_string() == page_text)
                .ok_or_else(|| format!("page={page_text:?} is not a whole number"))?
        };
        Ok(Self {
            inst,
            from,
            to,
            newest_first,
            page,
        })
    }

    /// The page's first and last day, the page (clamped) and the count of
    /// pages over the range.
    fn span(&self) -> (Day, Day, u32, u32) {
        let from = self.from.days_from_epoch();
        let to = self.to.days_from_epoch();
        let total = to.saturating_sub(from).saturating_add(1);
        let pages = total.div_ceil(PAGE_DAYS).max(1);
        let page = self.page.min(pages.saturating_sub(1));
        let skip = page.saturating_mul(PAGE_DAYS);
        let (first, last) = if self.newest_first {
            let last = to.saturating_sub(skip);
            (last.saturating_sub(PAGE_DAYS - 1).max(from), last)
        } else {
            let first = from.saturating_add(skip);
            (first, first.saturating_add(PAGE_DAYS - 1).min(to))
        };
        let day = |days: u32, fallback: Day| Day::from_days(days).unwrap_or(fallback);
        (day(first, self.from), day(last, self.to), page, pages)
    }
}

/// `GET /sources/day.json`.
pub(super) async fn sources_day_json(
    axum::extract::State(site): axum::extract::State<Loaded>,
    uri: axum::http::Uri,
) -> Answer {
    let query = uri.query().unwrap_or("").to_owned();
    let asked = match DayAsk::parse(&query) {
        Ok(asked) => asked,
        Err(why) => {
            return answer(
                axum::http::StatusCode::BAD_REQUEST,
                &json!({ "error": why }),
            );
        }
    };
    let root = site.store_root.clone();
    match crate::detail::run_store_read(move || day_table(&root, &asked)).await {
        Ok(Ok(body)) => answer(axum::http::StatusCode::OK, &body),
        Ok(Err(refusal)) => answer(refusal.status, &refusal.json()),
        Err(why) => refused_admission("all sources day table", &why),
    }
}

/// What one source holds of one day: its records folded to the day by
/// `pull::fold`, its first and last record, its count and its price grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DayFold {
    bar: Bar,
    records: u64,
    first: Bar,
    last: Bar,
    grid: i64,
}

impl DayFold {
    fn of(rows: &[Bar], source: Bucket) -> Result<Option<Self>, Refusal> {
        let (Some(&first), Some(&last)) = (rows.first(), rows.last()) else {
            return Ok(None);
        };
        let folded = fold(rows, Bucket::DAY, source)?;
        // ONE IST DAY IN, ONE BAR OUT: more is a grid this reader does not
        // understand, said so rather than summed.
        let [only] = folded.as_slice() else {
            return Err(Refusal::store(format!(
                "pull::fold made {} day bars of one IST day's records",
                folded.len()
            )));
        };
        Ok(Some(Self {
            bar: only.bar,
            records: only.rows,
            first,
            last,
            grid: grid_of(rows),
        }))
    }
}

/// Everything the store holds of one day for the comparison: Zerodha's day
/// record, Zerodha's minutes folded, GDFL's seconds folded, and the closing
/// auction's prints. Cached per (generations, instrument, day).
#[derive(Debug, Clone, PartialEq, Eq)]
struct DayFacts {
    zerodha_day: Option<Bar>,
    zerodha_minutes: Option<DayFold>,
    gdfl_seconds: Option<DayFold>,
    /// The last minute both feeds hold a print in: its start, Zerodha's
    /// close of it and GDFL's last print in it — the same rung and the same
    /// time (D-3339 item 3).
    common_minute: Option<(i64, i64, i64)>,
    auction: Auction,
    records_read: u64,
}

/// The last minute both feeds hold a print in, walking both back from the
/// day's end: its start, Zerodha's close of it and GDFL's last second in it.
///
/// One walk of each list, backwards: a CAS share's Zerodha minutes end at
/// 15:14 while GDFL's seconds run to the auction (D-3320), so the day's two
/// last prints are not the same time, and this is the pair that is.
fn common_minute(minutes: &[Bar], seconds: &[Bar]) -> Option<(i64, i64, i64)> {
    let mut end = seconds.len();
    for minute in minutes.iter().rev() {
        let close = minute.ts_micros.saturating_add(60 * MICROS);
        while let Some(at) = end.checked_sub(1)
            && seconds
                .get(at)
                .is_some_and(|second| second.ts_micros >= close)
        {
            end = at;
        }
        if let Some(last) = end
            .checked_sub(1)
            .and_then(|at| seconds.get(at))
            .filter(|second| second.ts_micros >= minute.ts_micros)
        {
            return Some((minute.ts_micros, minute.close, last.close));
        }
    }
    None
}

/// The closing auction's prints of a day, from the `.auc` sidecar beside the
/// GDFL second month (D-3320).
#[derive(Debug, Clone, PartialEq, Eq)]
enum Auction {
    /// Not a share, or a day before the closing auction runs.
    NotApplicable,
    /// The month has no sidecar.
    NoSidecar,
    /// The sidecar's prints of the day.
    Prints(Vec<(i64, i64, u64)>),
    /// The sidecar would not read, and why.
    Unreadable(String),
}

/// The day facts, by instrument, day and the stamps of the three month files.
type DayKey = (String, u32, [Option<Stamp>; 3]);

/// Every day's facts, by [`DayKey`].
static DAYS: Memo<DayKey, Arc<DayFacts>> = Memo::new();

/// The three series a day reads: Zerodha's day and minute rungs, and the
/// finest stored rung of every other offered feed (GDFL's seconds).
struct DaySeries {
    zerodha_day: Option<HeldSeries>,
    zerodha_minutes: Option<HeldSeries>,
    gdfl_seconds: Option<HeldSeries>,
}

/// The offered feed that stores `rung` as served.
fn feed_holding(rung: Timeframe) -> Option<(Feed, Vendor)> {
    offered()
        .find_map(|(feed, vendor)| (kind_of(feed, rung) == Kind::Stored).then_some((feed, vendor)))
}

impl DaySeries {
    fn of(root: &Path, inst: &Instrument) -> Result<Self, Refusal> {
        let series = |rung: Timeframe| -> Result<Option<HeldSeries>, Refusal> {
            let Some((_, vendor)) = feed_holding(rung) else {
                return Ok(None);
            };
            let series = Series {
                inst: inst.clone(),
                vendor,
                rung,
            };
            let held = series.held(root).map_err(Refusal::store)?;
            Ok(Some((series, held)))
        };
        Ok(Self {
            zerodha_day: series(Timeframe::DAY_1)?,
            zerodha_minutes: series(Timeframe::MINUTE_1)?,
            gdfl_seconds: series(Timeframe::SECOND_1)?,
        })
    }
}

/// The stamp of `series`' month file holding `day`, or `None` where there
/// is no such file: one `stat`.
fn day_stamp(root: &Path, held: Option<&HeldSeries>, day: Day) -> Option<Stamp> {
    let (series, _) = held?;
    let month = YearMonth::new(day.year(), day.month()).ok()?;
    Stamp::at(&series.path(root, month)?)
}

/// One series' records of one IST day, in time order.
fn day_rows(
    root: &Path,
    held: Option<&HeldSeries>,
    day: Day,
    budget: &mut Budget,
) -> Result<Vec<Bar>, Refusal> {
    let Some((series, months)) = held else {
        return Ok(Vec::new());
    };
    Rows::new(root, series, months, day_bounds(day), false).all(budget)
}

/// The facts of `day`, from the cache when no month file changed.
fn day_facts(
    root: &Path,
    inst: &Instrument,
    series: &DaySeries,
    day: Day,
    budget: &mut Budget,
) -> Result<(Arc<DayFacts>, bool), Refusal> {
    let key: DayKey = (
        inst.key(),
        day.days_from_epoch(),
        [
            day_stamp(root, series.zerodha_day.as_ref(), day),
            day_stamp(root, series.zerodha_minutes.as_ref(), day),
            day_stamp(root, series.gdfl_seconds.as_ref(), day),
        ],
    );
    if let Some(facts) = DAYS.get(&key) {
        return Ok((facts, true));
    }
    let before = budget.used();
    let zerodha_day = day_rows(root, series.zerodha_day.as_ref(), day, budget)?;
    let zerodha_minutes = day_rows(root, series.zerodha_minutes.as_ref(), day, budget)?;
    let gdfl_seconds = day_rows(root, series.gdfl_seconds.as_ref(), day, budget)?;
    if zerodha_day.len() > 1 {
        return Err(Refusal::store(format!(
            "{} holds {} day records on {day}; a day rung holds one",
            inst.key(),
            zerodha_day.len()
        )));
    }
    let facts = Arc::new(DayFacts {
        zerodha_day: zerodha_day.first().copied(),
        zerodha_minutes: DayFold::of(&zerodha_minutes, Bucket::MINUTE)?,
        gdfl_seconds: DayFold::of(&gdfl_seconds, Bucket::SECOND)?,
        common_minute: common_minute(&zerodha_minutes, &gdfl_seconds),
        auction: auction_of(root, inst, day),
        records_read: budget.used() - before,
    });
    DAYS.put(key, Arc::clone(&facts));
    Ok((facts, false))
}

/// The closing auction's prints of `day` for a share, from the `.auc`
/// sidecar (D-3320), or why there are none.
fn auction_of(root: &Path, inst: &Instrument, day: Day) -> Auction {
    let note = Feed::Gdfl.descriptor().closing_auction;
    let applies = inst.segment == Segment::Cash && note.is_some_and(|note| day >= note.from);
    if !applies {
        return Auction::NotApplicable;
    }
    let Ok(symbol) = Symbol::new(&inst.symbol) else {
        return Auction::Unreadable(format!("{} is not a symbol the store keys", inst.symbol));
    };
    let key = brutex_core::instrument::InstrumentKey {
        exchange: inst.exchange,
        segment: inst.segment,
        underlying: symbol,
        kind: brutex_core::instrument::Kind::Equity,
    };
    let path = match pull::gdfl_cm::auction::path(root, &key, day) {
        Ok(path) => path,
        Err(why) => return Auction::Unreadable(why),
    };
    let bytes = match std::fs::read(&path) {
        Ok(bytes) => bytes,
        Err(why) if why.kind() == std::io::ErrorKind::NotFound => return Auction::NoSidecar,
        Err(why) => return Auction::Unreadable(format!("{}: {why}", path.display())),
    };
    let (lo, hi) = day_bounds(day);
    match pull::gdfl_cm::auction::parse(&bytes) {
        Ok(records) => Auction::Prints(
            records
                .iter()
                .filter(|record| record.t_micros >= lo && record.t_micros < hi)
                .map(|record| (record.t_micros, record.price_paisa, record.quantity))
                .collect(),
        ),
        Err(why) => Auction::Unreadable(format!("{}: {why}", path.display())),
    }
}

impl Auction {
    fn json(&self) -> Value {
        match self {
            Self::NotApplicable => Value::Null,
            Self::NoSidecar => json!({
                "state": "no_sidecar",
                "says": "no auction sidecar for this month: GDFL CASH months imported before the \
                         D-3320 rule hold the auction print as an ordinary bar until re-imported, \
                         so the last print may be the auction's",
            }),
            Self::Prints(prints) => json!({
                "state": "kept_apart",
                "prints": prints.iter().map(|(at, price, quantity)| json!({
                    "at": ist_clock(*at), "price": price, "quantity": quantity,
                })).collect::<Vec<_>>(),
            }),
            Self::Unreadable(why) => json!({ "state": "unreadable", "why": why }),
        }
    }
}

/// One price against another, as `{"of", "against", "bps", "flag"}`, or
/// `null` where either is missing.
fn versus(of: Option<i64>, against: Option<i64>) -> Value {
    match of.zip(against).and_then(|(a, b)| price_cmp(a, b)) {
        Some((bps, beyond)) => json!({ "bps": bps, "flag": beyond }),
        None => Value::Null,
    }
}

/// What one day's comparisons add up to, for the page.
#[derive(Debug, Default)]
struct DayTally {
    days: usize,
    both: usize,
    open_equal: usize,
    high_equal: usize,
    low_equal: usize,
    close_equal: usize,
    close_differs: usize,
    volume_within: usize,
    volume_compared: usize,
    basis_differs: usize,
    basis_steps: usize,
    minute_extremes_outside: usize,
    minute_volume_off: usize,
    gdfl_late: usize,
    zerodha_only: usize,
    gdfl_only: usize,
    neither: usize,
    cached: usize,
}

/// The day table. See the module header for its cost.
///
/// # Errors
///
/// The store could not be read, or the page would read more than
/// [`MAX_ROWS_READ`] records.
pub fn day_table(root: &Path, asked: &DayAsk) -> Result<Value, Refusal> {
    // BEFORE THE DAY MEMO: a day table cached before the re-key retired the
    // key is keyed on its month files, which the re-key never touches.
    refuse_retired(root, &asked.inst)?;
    let (first, last, page, pages) = asked.span();
    let inst = &asked.inst;
    let series = DaySeries::of(root, inst)?;
    let mut budget = Budget::new();
    let mut days: Vec<Day> = (first.days_from_epoch()..=last.days_from_epoch())
        .filter_map(|n| Day::from_days(n).ok())
        .collect();
    // THE MARKS ARE /dayflags.json's OWN RECORDS (WP-33b), read for GDFL,
    // whose record carries the calendar, the import, the quality ledger and
    // Zerodha's gap line together.
    let flags = super::dayflags::answer(
        root,
        &super::dayflags::Asked {
            feed: Vendor::Gdfl,
            instrument: inst.for_day_flags(),
            from: first,
            to: last,
            newest_first: false,
            page: 0,
        },
    );
    let mut marks: HashMap<String, Value> = HashMap::with_capacity(days.len());
    if let Some(records) = flags.get("days").and_then(Value::as_array) {
        for record in records {
            if let Some(day) = record.get("day").and_then(Value::as_str) {
                marks.insert(day.to_owned(), marks_of(record));
            }
        }
    }
    let context = DayContext {
        segment: inst.segment,
        bases: feed_holding(Timeframe::DAY_1)
            .zip(feed_holding(Timeframe::SECOND_1))
            .and_then(|((z, _), (g, _))| bases_differ(inst.segment, z, g)),
        series: &series,
    };
    let mut tally = DayTally::default();
    let mut rows = Vec::with_capacity(days.len());
    let mut previous = PreviousFactor::default();
    for day in &days {
        let (facts, cached) = day_facts(root, inst, &series, *day, &mut budget)?;
        if cached {
            tally.cached += 1;
        }
        let mut row = day_row(*day, &facts, &context, &mut previous, &mut tally);
        if let Some(object) = row.as_object_mut() {
            object.insert(
                "marks".to_owned(),
                marks.remove(&day.to_string()).unwrap_or(Value::Null),
            );
        }
        rows.push(row);
    }
    if asked.newest_first {
        rows.reverse();
        days.reverse();
    }
    Ok(json!({
        "schema": 1,
        "instrument": inst.json(),
        "asked": {
            "from": asked.from.to_string(),
            "to": asked.to.to_string(),
            "order": if asked.newest_first { "desc" } else { "asc" },
        },
        "page": page,
        "pages": pages,
        "page_days": PAGE_DAYS,
        "page_from": first.to_string(),
        "page_to": last.to_string(),
        "sources": {
            "zerodha_day": series.zerodha_day.as_ref().map(|(s, _)| day_source(s, false)),
            "zerodha_minutes": series.zerodha_minutes.as_ref().map(|(s, _)| day_source(s, true)),
            "gdfl_seconds": series.gdfl_seconds.as_ref().map(|(s, _)| day_source(s, true)),
        },
        "days": rows,
        "summary": day_summary(&tally),
        "records_read": budget.used(),
        "tolerances": tolerances_json(),
        "notes": day_notes(inst, flags.get("notes").and_then(Value::as_array)),
    }))
}

/// A day source: its feed, the stored rung read, its tag, whether the
/// table folds it to the day, and its basis at that rung.
fn day_source(series: &Series, folded: bool) -> Value {
    let feed = feed_of_vendor(series.vendor);
    json!({
        "feed": series.vendor.as_str(),
        "rung": series.rung.as_str(),
        "tag": tag(series.vendor, Kind::Stored, series.rung),
        "folded_to_day": folded,
        "basis": feed.map(|feed| basis_json(feed, series.inst.segment, series.rung, Kind::Stored)),
    })
}

/// The offered feed whose store prefix is `vendor`.
fn feed_of_vendor(vendor: Vendor) -> Option<Feed> {
    offered().find_map(|(feed, at)| (at == vendor).then_some(feed))
}

/// The day marks a page needs, out of one `/dayflags.json` record.
fn marks_of(record: &Value) -> Value {
    let field = |key: &str| record.get(key).cloned().unwrap_or(Value::Null);
    json!({
        "calendar": field("calendar"),
        "events": field("events"),
        "import": field("import"),
        "degraded": field("degraded"),
        "zerodha_gaps": field("gaps"),
        "says": field("says"),
    })
}

/// What a day page says about itself.
fn day_notes(inst: &Instrument, flag_notes: Option<&Vec<Value>>) -> Vec<Value> {
    let mut notes: Vec<Value> = vec![json!(
        "Zerodha's 1day close is the official close and every other close here is a last print \
         (D-3324): the day bar is not a fold of its minutes (D-3001, D-3347), and the two are \
         shown side by side, never as one"
    )];
    if inst.segment == Segment::Index {
        notes.push(json!(
            "an index has no traded volume (D-2802, D-3322): volume is never compared"
        ));
    }
    if inst.segment == Segment::Cash {
        notes.push(json!(
            "Zerodha's share prices are back-adjusted by the vendor as of the pull date and \
             GDFL's are raw (D-3323): where the factor differs from 1 by more than 0.2% the \
             price comparisons across feeds give way to the factor"
        ));
    }
    if let Some(flag_notes) = flag_notes {
        notes.extend(flag_notes.iter().cloned());
    }
    notes
}

/// What a day's factor was measured on (D-3339 item 3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DayMeasure {
    /// GDFL's opening print ÷ Zerodha's official day open: D-3323's measure,
    /// the one D-3339's thresholds were chosen against.
    Open,
    /// GDFL's last print ÷ Zerodha's minute close, in the last minute both
    /// feeds hold a print in: the same rung and the same time.
    Minute,
}

impl DayMeasure {
    const fn words(self) -> &'static str {
        match self {
            Self::Open => "GDFL first print ÷ Zerodha day open (D-3323's measure)",
            Self::Minute => "GDFL ÷ Zerodha close at the last minute both hold",
        }
    }
}

/// The page's last factor pair of each measure, GDFL's price first.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct PreviousFactor {
    open: Option<(i64, i64)>,
    minute: Option<(i64, i64)>,
}

impl PreviousFactor {
    const fn of(self, measure: DayMeasure) -> Option<(i64, i64)> {
        match measure {
            DayMeasure::Open => self.open,
            DayMeasure::Minute => self.minute,
        }
    }

    const fn keep(&mut self, measure: DayMeasure, pair: (i64, i64)) {
        match measure {
            DayMeasure::Open => self.open = Some(pair),
            DayMeasure::Minute => self.minute = Some(pair),
        }
    }
}

/// What every day of one page is read against: the segment, whether the
/// feeds' price bases differ there, and the three series.
struct DayContext<'a> {
    segment: Segment,
    bases: Option<bool>,
    series: &'a DaySeries,
}

/// Why `held` holds no record of `day`, as a code a page words: no offered
/// feed stores the rung, the feed holds none of this instrument, the day is
/// outside the feed's archive or the operator's window, the calendar of
/// record says the exchange was closed, or none of those.
fn absent_why(held: Option<&HeldSeries>, day: Day) -> &'static str {
    let Some((series, months)) = held else {
        return "no_feed_stores_this_rung";
    };
    if months.is_empty() {
        return "none_of_this_instrument";
    }
    let descriptor = feed_of_vendor(series.vendor).map(Feed::descriptor);
    if let Some(span) = descriptor.and_then(|d| d.archive) {
        if day < span.from {
            return "before_archive";
        }
        if day > span.to {
            return "after_archive";
        }
    }
    if descriptor
        .and_then(|d| d.window)
        .is_some_and(|span| day < span.from || day > span.to)
    {
        return "outside_your_window";
    }
    if pull::calendar::exchange_kind_of(i64::from(day.days_from_epoch()))
        == pull::calendar::DayKind::Closed
    {
        return "closed";
    }
    "not_held"
}

/// One day of the table.
#[expect(
    clippy::too_many_lines,
    reason = "one day's comparisons written in the order the wire carries them, field by field"
)]
fn day_row(
    day: Day,
    facts: &DayFacts,
    context: &DayContext<'_>,
    previous: &mut PreviousFactor,
    tally: &mut DayTally,
) -> Value {
    let segment = context.segment;
    let bases = context.bases;
    let shares = segment == Segment::Cash;
    tally.days += 1;
    let day_bar = facts.zerodha_day;
    let minutes = facts.zerodha_minutes;
    let seconds = facts.gdfl_seconds;
    let zerodha_held = day_bar.is_some() || minutes.is_some();
    match (zerodha_held, seconds.is_some()) {
        (true, true) => tally.both += 1,
        (true, false) => tally.zerodha_only += 1,
        (false, true) => tally.gdfl_only += 1,
        (false, false) => tally.neither += 1,
    }
    let index = segment == Segment::Index;
    // WHETHER GDFL'S FIRST PRINT IS THE SESSION'S OPENING PRINT: a file that
    // starts at 11:57:48 (RELIANCE 2019-05-30) holds no open to compare, and
    // comparing its first print against the official open would set two
    // instants side by side as one.
    let gdfl_open_minute = seconds.map(|g| at_open(g.first.ts_micros));
    let gdfl_late = gdfl_open_minute == Some(false);
    if gdfl_late {
        tally.gdfl_late += 1;
    }
    // THE DAY'S FACTOR, ON A SAME-INSTANT PAIR ONLY (D-3339 item 3): D-3323's
    // measure, GDFL's opening print over Zerodha's official open (GDFL's
    // first print equals the day open where the minutes' open does not,
    // DB-G41); else the last minute both feeds hold, named.
    let open_pair = seconds
        .zip(day_bar)
        .filter(|_| !gdfl_late)
        .map(|(g, d)| (g.first.open, d.open));
    let minute_pair = facts.common_minute.map(|(_, z, g)| (g, z));
    let measured = open_pair
        .map(|pair| (DayMeasure::Open, pair))
        .or_else(|| minute_pair.map(|pair| (DayMeasure::Minute, pair)));
    let factor_pair = measured.map(|(_, pair)| pair);
    let differs =
        shares && bases == Some(true) && factor_pair.is_some_and(|(g, z)| basis_differs(g, z));
    // A STEP IS A MOVE BETWEEN FACTORS OF ONE MEASURE, against the page's
    // last day measured the same way: a day measured off the open is never
    // set against one measured off a minute.
    let before = measured.and_then(|(measure, _)| previous.of(measure));
    let step: Option<bool> = before
        .zip(factor_pair)
        .map(|(before, now)| shares && bases == Some(true) && factor_stepped(before, now));
    let step_why = match (measured, before) {
        (None, _) => Some("no factor on this day"),
        (Some(_), None) => Some("no earlier day on this page measured the same way"),
        (Some(_), Some(_)) => None,
    };
    let stepped = step == Some(true);
    if differs {
        tally.basis_differs += 1;
    }
    if stepped {
        tally.basis_steps += 1;
    }
    if let Some((measure, pair)) = measured {
        previous.keep(measure, pair);
    }
    // CROSS-FEED PRICES, unless the bases differ on this day.
    let across = |of: Option<i64>, against: Option<i64>| {
        if differs {
            json!({ "why": "basis_differs" })
        } else {
            versus(of, against)
        }
    };
    let day_open = day_bar.map(|bar| bar.open);
    let first_print = seconds.map(|g| g.first.open);
    let open = json!({
        "zerodha_day": day_open,
        "zerodha_first_minute": minutes.map(|m| m.first.open),
        "zerodha_first_minute_at": minutes.map(|m| ist_clock(m.first.ts_micros)),
        "gdfl_first_print": first_print,
        "gdfl_first_print_at": seconds.map(|g| ist_clock(g.first.ts_micros)),
        "gdfl_first_print_is_open": gdfl_open_minute,
        "gdfl_vs_zerodha_day": if gdfl_late {
            json!({ "why": "gdfl_first_print_after_open" })
        } else {
            across(first_print, day_open)
        },
        "zerodha_minute_vs_day": versus(minutes.map(|m| m.first.open), day_open),
    });
    let open_equal = !differs && !gdfl_late && first_print.is_some() && first_print == day_open;
    let open_mismatch = !differs
        && !gdfl_late
        && first_print.is_some()
        && day_open.is_some()
        && first_print != day_open;
    let minute_open_mismatch = minutes
        .zip(day_bar)
        .is_some_and(|(m, d)| m.first.open != d.open);
    // HIGH AND LOW: the day bar's, the minutes' and the seconds' extremes.
    let day_high = day_bar.map(|bar| bar.high);
    let day_low = day_bar.map(|bar| bar.low);
    let minute_high = minutes.map(|m| m.bar.high);
    let minute_low = minutes.map(|m| m.bar.low);
    let second_high = seconds.map(|g| g.bar.high);
    let second_low = seconds.map(|g| g.bar.low);
    let high_equal = !differs && second_high.is_some() && second_high == day_high;
    let low_equal = !differs && second_low.is_some() && second_low == day_low;
    let minute_outside = day_bar
        .zip(minutes)
        .is_some_and(|(d, m)| m.bar.high > d.high || m.bar.low < d.low);
    let gdfl_outside = !differs
        && day_bar
            .zip(seconds)
            .is_some_and(|(d, g)| g.bar.high > d.high || g.bar.low < d.low);
    if minute_outside {
        tally.minute_extremes_outside += 1;
    }
    // THE CLOSE: official against last prints, labelled (D-3324).
    let official = day_bar.map(|bar| bar.close);
    let minute_close = minutes.map(|m| m.last.close);
    let last_print = seconds.map(|g| g.last.close);
    let close_equal = !differs && last_print.is_some() && last_print == official;
    if !differs && last_print.is_some() && official.is_some() {
        if close_equal {
            tally.close_equal += 1;
        } else {
            tally.close_differs += 1;
        }
    }
    if open_equal {
        tally.open_equal += 1;
    }
    if high_equal {
        tally.high_equal += 1;
    }
    if low_equal {
        tally.low_equal += 1;
    }
    // VOLUME: the minutes' sum against the day's, and GDFL's against both.
    let day_volume = day_bar.map(|bar| bar.volume);
    let minute_volume = minutes.map(|m| m.bar.volume);
    let second_volume = seconds.map(|g| g.bar.volume);
    let minute_share = minute_volume
        .zip(day_volume)
        .and_then(|(m, d)| ratio4(m, d));
    let minute_off = minute_volume
        .zip(day_volume)
        .and_then(|(m, d)| volume_cmp(m, d))
        .is_some_and(|(_, beyond)| beyond);
    let volume = if index {
        json!({ "why": "index_no_volume", "zerodha_day": day_volume, "gdfl_sum": second_volume })
    } else {
        if minute_off {
            tally.minute_volume_off += 1;
        }
        let gdfl_vs_day = if differs {
            json!({ "factor": day_volume.zip(second_volume).and_then(|(z, g)| ratio4(z, g)), "why": "basis_differs" })
        } else {
            match second_volume
                .zip(day_volume)
                .and_then(|(g, d)| volume_cmp(g, d))
            {
                Some((pct, beyond)) => {
                    tally.volume_compared += 1;
                    if !beyond {
                        tally.volume_within += 1;
                    }
                    json!({ "percent": pct, "flag": beyond })
                }
                None => Value::Null,
            }
        };
        json!({
            "zerodha_day": day_volume,
            "zerodha_minutes_sum": minute_volume,
            "gdfl_sum": second_volume,
            "minute_over_day": minute_share,
            "minute_vs_day": minute_volume
                .zip(day_volume)
                .and_then(|(m, d)| volume_cmp(m, d))
                .map(|(pct, beyond)| json!({ "percent": pct, "flag": beyond })),
            "gdfl_vs_zerodha_day": gdfl_vs_day,
        })
    };
    // THE SHARE'S BASIS: the factor, its step, the volume factor and the
    // price grids (DB-G38, DB-G39, DB-G40, DB-G75).
    // ZERODHA'S MINUTES ON ANOTHER BASIS THAN ITS DAY (DB-G38, DB-G41): the
    // factor at the last common minute against the day's, past the same 0.2%.
    // `null` where either pair is missing: not measured, not equal.
    let minute_basis = open_pair
        .zip(minute_pair)
        .map(|(day, minute)| factor_stepped(day, minute));
    let minute_basis_differs = shares && minute_basis == Some(true);
    let share_basis = shares.then(|| {
        let factor = factor_pair.and_then(|(g, z)| ratio4(g, z));
        let volume_factor = day_volume.zip(second_volume).and_then(|(z, g)| ratio4(z, g));
        // VOLUME SCALED WITH PRICE would make Zerodha's day volume equal
        // GDFL's times the price factor: (z_v / g_v) against (g_c / z_c).
        let volume_unscaled = differs
            && factor_pair.zip(day_volume.zip(second_volume)).is_some_and(|((gc, zc), (zv, gv))| {
                let (gc, zc, zv, gv) = (
                    i128::from(gc),
                    i128::from(zc),
                    i128::from(zv),
                    i128::from(gv),
                );
                let scaled = gv * gc;
                scaled > 0 && (zv * zc - scaled).abs() * 1_000 > VOLUME_TOLERANCE_PERMILLE * scaled
            });
        let zerodha_grid = minutes.map(|m| m.grid);
        let gdfl_grid = seconds.map(|g| g.grid);
        // QUANTISED: Zerodha's grid coarser than GDFL's raw grid divided by
        // the factor, rounded up — what an exact adjustment would leave.
        let quantised = zerodha_grid
            .zip(gdfl_grid)
            .zip(factor_pair)
            .is_some_and(|((zg, gg), (gc, zc))| {
                let (zg, gg, gc, zc) = (
                    i128::from(zg),
                    i128::from(gg),
                    i128::from(gc),
                    i128::from(zc),
                );
                gc > 0 && zg > (gg * zc + gc - 1) / gc
            });
        json!({
            "factor": factor,
            "factor_of": measured.map(|(measure, _)| measure.words()),
            "minute_factor": minute_pair.and_then(|(g, z)| ratio4(g, z)),
            "minute_factor_at": facts.common_minute.map(|(at, _, _)| ist_clock(at)),
            "minute_basis_differs_from_day": minute_basis,
            "bases": match bases { Some(true) => "differ", Some(false) => "same", None => "unverified" },
            "basis_differs": differs,
            "basis_step": step,
            "basis_step_why": step_why,
            "volume_factor": volume_factor,
            "volume_not_scaled_with_price": volume_unscaled,
            "grid_paisa": { "zerodha_minutes": zerodha_grid, "gdfl_seconds": gdfl_grid },
            "quantised": quantised,
        })
    });
    let mut flags: Vec<&str> = Vec::new();
    if open_mismatch {
        flags.push("open_mismatch");
    }
    if minute_open_mismatch {
        flags.push("minute_open_mismatch");
    }
    if minute_outside {
        flags.push("minute_extremes_outside_day");
    }
    if gdfl_outside {
        flags.push("gdfl_extremes_outside_day");
    }
    // LABELLED, NEVER A DISAGREEMENT (D-3324), AND NAMED ONLY WHERE THEY
    // DIFFER: from 2026-08-03 the two are equal and there is nothing to name.
    if official.is_some_and(|official| {
        minute_close.is_some_and(|close| close != official)
            || (!differs && last_print.is_some_and(|close| close != official))
    }) {
        flags.push("official_vs_last_print");
    }
    if gdfl_late {
        flags.push("gdfl_first_print_after_open");
    }
    if minute_off && !index {
        flags.push("minute_volume_off_day");
    }
    if differs {
        flags.push("basis_differs");
    }
    if stepped {
        flags.push("basis_step");
    }
    if minute_basis_differs {
        flags.push("minute_basis_differs_from_day");
    }
    let bar_json = |bar: Option<Bar>| {
        bar.map(|bar| json!({ "o": bar.open, "h": bar.high, "l": bar.low, "c": bar.close, "v": bar.volume }))
    };
    let fold_json = |fold: Option<DayFold>, what: &str| {
        fold.map(|f| {
            json!({
                "o": f.bar.open, "h": f.bar.high, "l": f.bar.low, "c": f.bar.close, "v": f.bar.volume,
                what: f.records,
                "first_at": ist_clock(f.first.ts_micros),
                "last_at": ist_clock(f.last.ts_micros),
            })
        })
    };
    // WHY A SOURCE HOLDS NOTHING OF THE DAY, never a silent gap.
    let mut absent = serde_json::Map::new();
    for (name, held, here) in [
        (
            "zerodha_day",
            context.series.zerodha_day.as_ref(),
            day_bar.is_some(),
        ),
        (
            "zerodha_minutes",
            context.series.zerodha_minutes.as_ref(),
            minutes.is_some(),
        ),
        (
            "gdfl_seconds",
            context.series.gdfl_seconds.as_ref(),
            seconds.is_some(),
        ),
    ] {
        if !here {
            absent.insert(name.to_owned(), json!(absent_why(held, day)));
        }
    }
    json!({
        "day": day.to_string(),
        "absent": absent,
        "zerodha_day": bar_json(day_bar),
        "zerodha_day_close": day_bar.map(|_| "official"),
        "zerodha_minutes_fold": fold_json(minutes, "minutes"),
        "gdfl_seconds_fold": fold_json(seconds, "seconds"),
        "last_print": {
            "zerodha_last_minute": minute_close,
            "zerodha_last_minute_at": minutes.map(|m| ist_clock(m.last.ts_micros)),
            "gdfl_last_print": last_print,
            "gdfl_last_print_at": seconds.map(|g| ist_clock(g.last.ts_micros)),
        },
        "open": open,
        "high": {
            "zerodha_day": day_high, "zerodha_minutes": minute_high, "gdfl_seconds": second_high,
            "gdfl_vs_zerodha_day": across(second_high, day_high),
            "minute_above_day": day_high.zip(minute_high).map(|(d, m)| m > d),
        },
        "low": {
            "zerodha_day": day_low, "zerodha_minutes": minute_low, "gdfl_seconds": second_low,
            "gdfl_vs_zerodha_day": across(second_low, day_low),
            "minute_below_day": day_low.zip(minute_low).map(|(d, m)| m < d),
        },
        "close": {
            "official": official,
            "zerodha_last_minute": minute_close,
            "gdfl_last_print": last_print,
            "official_vs_gdfl_last_print": if differs { json!({ "why": "basis_differs" }) } else {
                versus(last_print, official).as_object().map_or(Value::Null, |object| {
                    let mut object = object.clone();
                    object.insert("flag".to_owned(), json!(false));
                    object.insert("semantics".to_owned(), json!("official_vs_last_print"));
                    Value::Object(object)
                })
            },
            "official_vs_zerodha_last_minute": versus(minute_close, official).as_object().map_or(Value::Null, |object| {
                let mut object = object.clone();
                object.insert("flag".to_owned(), json!(false));
                object.insert("semantics".to_owned(), json!("official_vs_last_print"));
                Value::Object(object)
            }),
            "last_common_minute": facts.common_minute.map(|(at, z, g)| json!({
                "at": ist_clock(at),
                "zerodha": z,
                "gdfl": g,
                "gdfl_vs_zerodha": across(Some(g), Some(z)),
            })),
            "equal": (last_print.is_some() && official.is_some() && !differs).then_some(close_equal),
        },
        "volume": volume,
        "basis": share_basis,
        "auction": facts.auction.json(),
        "flags": flags,
        "records_read": facts.records_read,
    })
}

/// The page's tally as JSON.
fn day_summary(tally: &DayTally) -> Value {
    json!({
        "days": tally.days,
        "both_feeds": tally.both,
        "zerodha_only": tally.zerodha_only,
        "gdfl_only": tally.gdfl_only,
        "neither": tally.neither,
        "open_equal": tally.open_equal,
        "high_equal": tally.high_equal,
        "low_equal": tally.low_equal,
        "close_equal": tally.close_equal,
        "close_differs": tally.close_differs,
        "close_compared_as": "official (Zerodha 1day) against last print (GDFL seconds): \
                              labelled, never a disagreement (D-3324)",
        "volume_compared": tally.volume_compared,
        "volume_within_tolerance": tally.volume_within,
        "basis_differs": tally.basis_differs,
        "basis_steps": tally.basis_steps,
        "minute_extremes_outside_day": tally.minute_extremes_outside,
        "minute_volume_off_day": tally.minute_volume_off,
        "gdfl_first_print_after_open": tally.gdfl_late,
        "days_from_cache": tally.cached,
    })
}

#[cfg(test)]
#[path = "sources_tests.rs"]
mod tests;
