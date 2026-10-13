//! The prices themselves — the page every other page was describing.
//!
//! # Why this file exists
//!
//! `/store` counted. It said `62,978 rows across 194 committed entries` and
//! drew a grid of swatches, and **not one page in this server ever showed a
//! single price**. A store you cannot look inside is a store you have to trust,
//! and the operator's question — *is the data any good?* — had no answer here at
//! all. Every figure was a counter about the data rather than the data.
//!
//! This reads bars back out of the file the ingest path wrote and renders them:
//! timestamp, open, high, low, close, volume, open interest. It is the first
//! surface in this repository where an operator can see what was actually
//! stored.
//!
//! # The cost, and why paging is arithmetic
//!
//! [`store::file::BarFile::read_record`] is a **seek and a fixed-length read**
//! at `offset_of(index)` — `docs/07-o1-architecture.md` layer 1, the whole point
//! of a fixed-stride format. So the row at index 40,000 costs exactly what the
//! row at index 0 costs, and a page of 200 is 200 reads regardless of which page
//! it is. Nothing here scans, and nothing here holds a month in memory: a
//! request for page 5 touches the 200 records of page 5 and no others.
//!
//! `n_valid` comes off the header, so the page count is a division rather than a
//! walk — the same arithmetic `/store` and `/instruments` page by.
//!
//! # Paisa in, rupees on screen, and nothing in between
//!
//! `CLAUDE.md` §7: prices are paisa integers and never a float. That holds all
//! the way here — [`rupees`] formats an `i64` by splitting it, not by dividing
//! it, so there is no float on this path either. `2450075` renders as
//! `24,500.75` through integer arithmetic and string assembly.

use core::fmt::Write as _;

use brutex_core::vendor::Vendor;
use pull::session::Day;
use store::file::BarFile;

use crate::fault_log::FaultLog;
use store::format::{Bar, OI_NULL};
use store::path::{FileKind, PathParts, StorePath, Timeframe, YearMonth};

#[path = "bars_change.rs"]
mod change;
#[path = "bars_series.rs"]
mod series;
#[path = "bars_sort.rs"]
mod sort;
use series::SeriesPlan;
pub use series::{Filter, MAX_SERIES, SeriesKey};
pub use sort::SortReport;

/// A request's cancellation flag: set when the client that asked went away,
/// read by the blocking work between months and every few thousand rows, so
/// an abandoned request stops instead of running to its end (DB-N15).
///
/// [`Cancel::never`] for a caller with no client — a bench, a test, the CLI.
#[derive(Debug, Clone, Default)]
pub struct Cancel(Option<std::sync::Arc<std::sync::atomic::AtomicBool>>);

impl Cancel {
    /// A flag nothing ever sets.
    #[must_use]
    pub const fn never() -> Self {
        Self(None)
    }

    /// A flag, and the guard that sets it when dropped unless it was
    /// disarmed first — dropped with the request's future when the client
    /// goes away, disarmed when the answer is ready.
    #[must_use]
    pub fn pair() -> (Self, CancelOnDrop) {
        let flag = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        (
            Self(Some(std::sync::Arc::clone(&flag))),
            CancelOnDrop(Some(flag)),
        )
    }

    /// Whether the client went away.
    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.0
            .as_ref()
            .is_some_and(|flag| flag.load(std::sync::atomic::Ordering::Acquire))
    }

    /// `Err("cancelled: …")` once the client went away.
    ///
    /// # Errors
    ///
    /// The cancellation, by name.
    pub fn check(&self) -> Result<(), String> {
        if self.is_cancelled() {
            Err("cancelled: the client that asked went away, and the work stopped".to_owned())
        } else {
            Ok(())
        }
    }
}

/// Sets its [`Cancel`] flag when dropped, unless [`Self::disarm`]ed.
#[derive(Debug)]
pub struct CancelOnDrop(Option<std::sync::Arc<std::sync::atomic::AtomicBool>>);

impl CancelOnDrop {
    /// The answer is ready: dropping the guard now cancels nothing.
    pub fn disarm(mut self) {
        self.0 = None;
    }
}

impl Drop for CancelOnDrop {
    fn drop(&mut self) {
        if let Some(flag) = self.0.take() {
            flag.store(true, std::sync::atomic::Ordering::Release);
        }
    }
}

/// Whether an F&O series on `symbol` is written on an index (D-3325 rule 3,
/// DB-G66), by the sourced alias table's answer (`brutex_core::alias::
/// option_underlying`, D-3332): every F&O index symbol (NIFTY, BANKNIFTY,
/// FINNIFTY, MIDCPNIFTY, NIFTYNXT50, and the older NIFTYIT, NIFTYFPI and
/// NIFTYCPSE products the GDFL archive holds) or index store key is an index;
/// anything else is a share, which refuses rather than prints — a 1:2 bonus
/// and a 49% crash cannot be told apart by size. One authority with
/// `/spot.json`, so the chain a bar is priced against and the refusal its
/// change gets name the same underlying.
pub(crate) fn underlying_is_index(symbol: &str) -> bool {
    brutex_core::alias::option_underlying(symbol).segment == brutex_core::instrument::Segment::Index
}

/// How many bars one page shows.
///
/// The same 200 every other paged surface uses. A minute-bar month is about
/// 7,500 rows, so a full month is ~38 pages — readable, and every page costs
/// the same.
pub const PAGE_BARS: usize = 200;

/// A price in paisa, rendered as rupees **without a float**.
///
/// `CLAUDE.md` §7 says prices are `i64` paisa and never a float, and a renderer
/// that divides by 100.0 to show them walks that back at the last possible
/// moment — where nobody looks. This splits instead: the rupee part is
/// `paisa / 100`, the paise part is `paisa % 100`, and both are integers.
///
/// Grouped in the Indian convention — `24,500.75`, and `1,23,456.78` past a
/// lakh — because this is an NSE tool and a trader reading `123,456.78` has to
/// stop and re-count.
#[must_use]
pub fn rupees(paisa: i64) -> String {
    let negative = paisa < 0;
    // `unsigned_abs` rather than `abs`: `i64::MIN` has no positive counterpart
    // and `abs` would panic on it. It is also the open-interest null, which is
    // never routed here — but a formatter that panics on one input is a
    // formatter that panics.
    let whole = paisa.unsigned_abs() / 100;
    let hundredths = paisa.unsigned_abs() % 100;

    let digits = whole.to_string();
    let mut grouped = String::with_capacity(digits.len() + 6);
    let bytes = digits.as_bytes();
    // The last three digits group together; everything before them groups in
    // twos. That is the Indian system, and it is written as arithmetic on the
    // index rather than as a regular expression.
    for (i, b) in bytes.iter().enumerate() {
        let from_end = bytes.len() - i;
        if i > 0 && (from_end == 3 || (from_end > 3 && (from_end - 3).is_multiple_of(2))) {
            grouped.push(',');
        }
        grouped.push(char::from(*b));
    }
    format!(
        "{}{grouped}.{hundredths:02}",
        if negative { "-" } else { "" }
    )
}

/// A bar's timestamp as an IST wall clock, `HH:MM`.
///
/// The store holds UTC microseconds; a trader reads IST. The conversion is the
/// fixed +5:30 India has used since 1942 — `pull::session` owns that constant
/// and this is the same offset, applied for display only. Nothing is stored,
/// compared or filtered on this string.
#[must_use]
pub fn ist_clock(ts_micros: i64) -> String {
    // Integer division throughout: microseconds to seconds, then the offset,
    // then the two fields. `div_euclid` so a pre-epoch timestamp floors rather
    // than truncating toward zero — the store cannot hold one, and a formatter
    // that is wrong for an input it cannot receive is still wrong.
    let secs = ts_micros.div_euclid(1_000_000) + 5 * 3600 + 1800;
    let day_secs = secs.rem_euclid(86_400);
    let (h, m) = (day_secs / 3600, (day_secs % 3600) / 60);
    format!("{h:02}:{m:02}")
}

/// The IST calendar day a stored timestamp falls on, `YYYY-MM-DD`.
///
/// # Why the clock alone was not enough
///
/// [`ist_clock`] computes the day and throws it away. A month of one-minute
/// bars is 7,875 rows all reading `09:15` through `15:30`, so **two rows a day
/// apart are identical on the page** and no row can be named: "the 11:50 bar"
/// picks out twenty-one of them. Paging made it worse, because the row number
/// restarts at 1 on every page.
///
/// The shift is the same one [`ist_clock`] applies, taken from the same
/// expression, so the two cannot disagree about which side of midnight a bar
/// falls on. The day count is handed to [`Day::from_days`], which owns the
/// civil-calendar arithmetic and is round-tripped over all 2,932,897
/// representable days — this file does not do calendar maths of its own.
///
/// A timestamp outside the representable range renders as `—`, which is the
/// same thing the store would refuse to hold. It is not reachable from a bar
/// file, and a formatter that lies about an input it cannot receive is still a
/// formatter that lies.
#[must_use]
pub fn ist_day(ts_micros: i64) -> String {
    let secs = ts_micros.div_euclid(1_000_000) + 5 * 3600 + 1800;
    let days = secs.div_euclid(86_400);
    u32::try_from(days)
        .ok()
        .and_then(|d| Day::from_days(d).ok())
        .map_or_else(|| "—".to_owned(), |day| day.to_string())
}

/// A refused read, named on the record instead of only in the reply body.
///
/// # What was invisible
///
/// `/bars.json` answered `400` on every call and the refusal existed in exactly
/// one place: the body of that reply. A browser showed a red box, the operator
/// reported "the chart is empty", and **nothing on the machine could say which
/// of the six path segments was wrong** — or even whether the month was absent,
/// the timeframe directory was, or the path had failed validation before any
/// file was touched. Reproducing it was the only way to find out, and a
/// reproduction needs the query string nobody wrote down.
///
/// The pair that actually disagrees is `(timeframe, month)`. D-0055 made the
/// rung a control on the form, so a daily backfill lands in `1day/` while a
/// reader asking for `1min/` gets a truthful refusal about a file that was never
/// meant to exist — see [`open`]'s own note. This line carries both, so the
/// answer is a `grep` rather than a re-run.
///
/// `Warn`, not `Error`: the server answered, the page named the refusal, and
/// nothing on disk is wrong. Someone still has to know.
///
/// **Once per refused request, and never once per bar.** A read that succeeds
/// emits nothing here at all, and [`page`] below reads up to [`PAGE_BARS`]
/// records without a line per record — a month is ~7,500 rows and a per-row
/// event would roll the request's own beginning out of the sink before it
/// finished.
fn note_refused(
    vendor: Vendor,
    exchange: &str,
    segment: &str,
    symbol: &str,
    timeframe: Timeframe,
    month: YearMonth,
    why: &str,
) {
    let month_text = month.to_string();
    let _dropped_when_filtered = telemetry::emit(
        &telemetry::Event::warn("api.bars", "read refused")
            .with("vendor", telemetry::Value::Str(vendor.as_str()))
            .with("exchange", telemetry::Value::Str(exchange))
            .with("segment", telemetry::Value::Str(segment))
            .with("symbol", telemetry::Value::Str(symbol))
            .with("timeframe", telemetry::Value::Str(timeframe.as_str()))
            .with("month", telemetry::Value::Str(&month_text))
            .with("why", telemetry::Value::Str(why)),
    );
}

/// One month of one instrument, opened for reading.
///
/// # Errors
///
/// The path could not be rendered, or the file could not be opened — both
/// carried as the refusal's own words, so a missing month names the path it
/// looked for rather than answering "no data".
#[expect(
    clippy::too_many_arguments,
    reason = "these ARE a store path's segments, and `store::path::PathParts` \
              already exists to group exactly them — its own doc gives the \
              reason: `exchange` and `segment` are both short uppercase \
              strings and swapping them builds a valid-looking path to the \
              wrong place. This function deliberately does not take one, \
              because `PathParts` carries `file` and this function must fix \
              that to `FileKind::Bars` itself: a caller free to set it could \
              open a `.crc` or a `.lock` through a bar reader and have its \
              first bytes read as a header. Grouping the other seven into a \
              second near-identical struct would put two spellings of one \
              address in the crate, which is worse than the count"
)]
pub fn open(
    store_root: &std::path::Path,
    vendor: Vendor,
    exchange: &str,
    segment: &str,
    symbol: &str,
    // THE RUNG IS AN ARGUMENT, NOT A LITERAL.
    //
    // This was `Timeframe::MINUTE_1`, so every chart read looked under
    // `1min/` whatever the store actually held. A daily backfill lands in
    // `1day/` — D-0055 made the rung a control on the form and the writer
    // honours it — so the reader answered "…/1min/2021-08.bin does not exist"
    // about a file that was never meant to. The refusal was truthful and the
    // question was wrong.
    timeframe: Timeframe,
    month: YearMonth,
    // THE CONTRACT SEGMENT, AND IT USED TO BE `None` UNCONDITIONALLY.
    //
    // The store's layout is `symbol/contract/`, so an option's bars live one
    // level below the underlying's. Hardcoding `None` here meant **no F&O
    // contract was addressable through this reader at all**, and the front end
    // compensated the only way it could: by concatenating the two into the
    // symbol field.
    //
    // Measured 2026-08-20, journal seq 910–922: `GET /bars.json` with
    // `symbol=BANKNIFTY-2026-07-28-5410000-CE` refused with *"path segment
    // symbol is 31 bytes, max 24"* — thirteen 400s in three seconds, one per
    // contract the page tried to draw. The refusal was correct and the question
    // was malformed, and it had been malformed since this function was written;
    // it only surfaced when F&O bars first existed on disk to be asked for.
    //
    // `None` remains right for spot, whose path IS one level shallower — that
    // absence is the signal `StorePath` branches on, per `Contract::of`.
    contract: Option<brutex_core::instrument::Contract>,
) -> Result<BarFile, String> {
    open_classified(
        store_root,
        PathParts {
            vendor,
            exchange,
            segment,
            symbol,
            contract,
            timeframe,
            month,
            file: FileKind::Bars,
        },
    )
    .map_err(|why| why.message)
}

pub(crate) enum OpenRefusalKind {
    Invalid,
    Absent,
    Unreadable,
}

pub(crate) struct OpenRefusal {
    pub(crate) kind: OpenRefusalKind,
    pub(crate) message: String,
}

/// Preserve absence separately from invalid addresses and unreadable authority.
pub(crate) fn open_classified(
    store_root: &std::path::Path,
    parts: PathParts<'_>,
) -> Result<BarFile, OpenRefusal> {
    let PathParts {
        vendor,
        exchange,
        segment,
        symbol,
        contract,
        timeframe,
        month,
        ..
    } = parts;
    let path = StorePath::new(PathParts {
        vendor,
        exchange,
        segment,
        symbol,
        contract,
        timeframe,
        month,
        file: FileKind::Bars,
    })
    .map_err(|why| {
        // REFUSED BEFORE ANY FILE WAS TOUCHED. This arm is the path itself
        // failing validation, which reads on the page exactly like a month that
        // is not held — and they send an operator to two different places.
        let refusal = format!("{symbol} {month}: {why}");
        note_refused(
            vendor, exchange, segment, symbol, timeframe, month, &refusal,
        );
        OpenRefusal {
            kind: OpenRefusalKind::Invalid,
            message: refusal,
        }
    })?;
    // THE SYMBOL ID IS DERIVED THE WAY `pull::ingest` DERIVES IT. The store
    // stamps it into the header on create and verifies it on every reopen, so a
    // reader that folded the hash differently would be refused by the file
    // itself — which is the check working, not a bug to route around.
    // A KEY THE CURRENT DECODER REFUSES IS REFUSED BY NAME on every reader
    // (D-3331; WP-12c, DB-A18): the re-key recorded it `retired`, its files
    // are kept, and no route serves its bars. One `stat` of the lineage
    // ledger and one hash probe, for a GDFL option only.
    if vendor == Vendor::Gdfl
        && segment == "FNO"
        && let Some(contract) = contract
        && let Some(why) = change::retired_refusal(
            store_root,
            vendor,
            &format!("{exchange}-{segment}-{symbol}-{contract}"),
        )
    {
        note_refused(vendor, exchange, segment, symbol, timeframe, month, &why);
        return Err(OpenRefusal {
            kind: OpenRefusalKind::Invalid,
            message: why,
        });
    }
    #[expect(
        clippy::cast_possible_truncation,
        reason = "the id is a CROSS-CHECK the store verifies on reopen, never an \
                  index — any 32 bits serve, and the low half of a 64-bit FNV-1a \
                  is what pull::ingest stamps in. A different fold here would \
                  fail to open a file this repository wrote."
    )]
    let symbol_id = brutex_core::universe::fnv1a(symbol) as u32;
    // `open_existing`, NEVER `open_or_create`. This is a GET.
    //
    // The read-write door creates what it cannot find — six directories, a
    // 32 KiB initialised bar file and a `.lock` — so this handler used to
    // manufacture a month for any symbol a caller typed into the query string,
    // and then report that month as empty. The file it described was the file
    // the request had just made.
    //
    // The disk was the smaller cost. The larger one is that creating on read
    // collapses "there is no such month" into "this month holds nothing", which
    // are different answers to an operator, and it made the missing-month arm
    // below unreachable. A `Missing` refusal now names the path it looked for.
    BarFile::open_existing(store_root, path, symbol_id).map_err(|why| {
        // THE MONTH IS NOT HELD, or the timeframe has no directory beneath the
        // symbol, or the header cross-check refused this reader. All three
        // arrive here as one string and all three are worth a line: the first is
        // ordinary, the second means the rung on the form does not match the rung
        // on disk, and the third means a file this build cannot read.
        let kind = if matches!(&why, store::file::StoreError::Missing { path: missing, .. }
            if *missing == path.to_path_buf(store_root))
        {
            OpenRefusalKind::Absent
        } else {
            OpenRefusalKind::Unreadable
        };
        let refusal = why.to_string();
        note_refused(
            vendor, exchange, segment, symbol, timeframe, month, &refusal,
        );
        OpenRefusal {
            kind,
            message: refusal,
        }
    })
}

/// The records this page asked for and did not get, as **one** line.
///
/// # Why a count and a single reason, and not a line per record
///
/// [`page`] skips a record the file refuses so that one damaged row does not
/// blank the other 199, and the page names the gap. That is the right behaviour
/// and it is also how a decaying month stays invisible to everyone who is not
/// looking at that exact page: nothing outside the reply ever hears about it.
///
/// A line per faulted record is not available. A minute-month is ~7,500 records
/// and a file going bad goes bad in runs, so a line per fault would cost up to
/// [`PAGE_BARS`] events for one request — enough to push the request's own
/// earlier lines out of a 64 MiB sink.
///
/// **That figure is UNVERIFIED and stays that way on purpose.** It is arithmetic
/// about a design that was rejected and therefore never existed to measure; no
/// such code was ever written, so there is nothing to run. It is recorded as the
/// reason for the shape below, not as a measurement. `CLAUDE.md` §3 rule 6, and
/// `docs/06-limits.md` §44.
///
/// **What IS bounded here, structurally:** this function has two call
/// sites — [`page`], and [`window`] once after whichever path it took (seek,
/// merge, scan or extremes fold; D-3326) — each reached once per request and
/// outside every loop over files or records, and it returns on the first line
/// when `faults` is empty and otherwise emits once. So the real cost is at most
/// one event per request regardless of how many records, or how many of a
/// window's up to [`MAX_WINDOW_MONTHS`] files, refuse, which is the property the
/// shape was chosen for.
///
/// It was called from inside [`slots`], which a window calls once per month
/// file, so a sorted or `extremes=1` window over 240 damaged months emitted up
/// to 240 lines for one request while this doc said one (Z1-slice11-F3,
/// D-1762). `file` is the FIRST file that refused a record; `faults` counts
/// every refusal across all of them.
///
/// The faults were already counted in a
/// `Vec` the caller renders; this reports its length and the first reason, which
/// is the same bargain `pull`'s member failures strike.
///
/// Silent on a clean page: a request where every record read costs one branch on
/// an empty `Vec` and emits nothing.
fn note_unreadable_records(file: &str, faults: &FaultLog, rows: usize, skip: usize) {
    if faults.is_empty() {
        return;
    }
    // THE FIRST SENTENCE OF A CLONE, NOT OF THE LOG: the caller still renders
    // the log, and a run is only a sentence once it is closed. At most
    // `MAX_NAMED_FAULTS` sentences are built, so the clone is bounded (DB-B75).
    let said = faults.clone().sentences();
    let first = said.first().map_or("", String::as_str);
    let _dropped_when_filtered = telemetry::emit(
        &telemetry::Event::warn("api.bars", "records unreadable")
            .with("file", telemetry::Value::Str(file))
            .with("faults", telemetry::Value::Uint(faults.records()))
            .with("rows", telemetry::Value::Uint(rows as u64))
            .with("skip", telemetry::Value::Uint(skip as u64))
            // THE FIRST REASON ONLY, and the count says how many there were.
            // Every reason would be a field whose size grows with the damage,
            // which is the one thing a bounded line cannot carry.
            .with("first", telemetry::Value::Str(first)),
    );
}

/// One page of bars, read by index.
///
/// **Each row is one seek and one fixed-length read.** No scan, no month held in
/// memory, and page 38 costs what page 1 costs.
///
/// A record the file refuses is skipped rather than aborting the page: one
/// unreadable row in the middle of a month should not blank the other 199, and
/// the caller reports the gap. The refusal is returned beside the rows so it can
/// be named on the page — `CLAUDE.md` §4, degrade loudly.
/// The refusals come back as a [`FaultLog`]: consecutive records refused for
/// one reason are ONE sentence naming the file once, at most
/// [`crate::fault_log::MAX_NAMED_FAULTS`] are named, and
/// [`FaultLog::records`] counts every one (DB-B75). A month whose `.crc` was
/// lost answered 188 MB of the same sentence before.
#[must_use]
pub fn page(file: &BarFile, skip: usize, take: usize) -> (Vec<Bar>, FaultLog) {
    let (read, faults) = slots(file, skip, take);
    let rows: Vec<Bar> = read.into_iter().flatten().collect();
    note_unreadable_records(
        &file.path().display().to_string(),
        &faults,
        rows.len(),
        skip,
    );
    (rows, faults)
}

/// [`page`], reading in chunks and stopping with a `cancelled` refusal once
/// `cancel` is set — `/bars.json` reads a whole one-second month, up to 2.7
/// million rows, and an abandoned request must not run to its end (DB-N15).
///
/// One `records unreadable` line for the whole read, as [`page`] writes.
///
/// # Errors
///
/// `cancelled`, by name.
pub fn page_cancellable(
    file: &BarFile,
    skip: usize,
    take: usize,
    cancel: &Cancel,
) -> Result<(Vec<Bar>, FaultLog), String> {
    /// Rows between two looks at the flag.
    const CHUNK: usize = 65_536;
    let mut rows = Vec::with_capacity(take.min(1 << 20));
    // ONE LOG FOR THE WHOLE READ, so a run of refused records that crosses a
    // chunk is still one sentence (DB-B75).
    let mut faults = FaultLog::default();
    let mut at = skip;
    let end = skip.saturating_add(take);
    while at < end {
        cancel.check()?;
        let size = CHUNK.min(end - at);
        let read = slots_into(file, at, size, &mut faults);
        let got = read.len();
        rows.extend(read.into_iter().flatten());
        if got < size {
            break;
        }
        at += size;
    }
    note_unreadable_records(
        &file.path().display().to_string(),
        &faults,
        rows.len(),
        skip,
    );
    Ok((rows, faults))
}

/// `api.bars`, INFO, once per window request whose range holds months a
/// series does not hold inside its own life — the holes: how many, the first
/// and the last (DB-N17). Never a line per month: a 240-month window over a
/// sparse store wrote one warning per absent month on every request, and
/// about 1,240 such requests would roll the whole 64 MiB log over. Months
/// before a series' first held month or after its last are expected
/// absences and write nothing. Absent months are counted from the rung
/// directory's listing and never opened.
fn note_holes(plans: &[SeriesPlan]) {
    let holes: usize = plans.iter().map(|plan| plan.holes.len()).sum();
    let first = plans
        .iter()
        .filter_map(|plan| plan.holes.first())
        .min()
        .map(ToString::to_string);
    let last = plans
        .iter()
        .filter_map(|plan| plan.holes.last())
        .max()
        .map(ToString::to_string);
    let (Some(first), Some(last), Some(plan)) = (first, last, plans.first()) else {
        return;
    };
    let _dropped_when_filtered = telemetry::emit(
        &telemetry::Event::info("api.bars", "window range holds months not stored")
            .with("months", telemetry::Value::Uint(holes as u64))
            .with("first", telemetry::Value::Str(&first))
            .with("last", telemetry::Value::Str(&last))
            .with("key", telemetry::Value::Str(&plan.key.text()))
            .with("series", telemetry::Value::Uint(plans.len() as u64)),
    );
}

/// [`page`], with every POSITION kept: `None` where the record would not read.
///
/// The window needs the gap and not only its name. A row after an unreadable
/// record has no readable predecessor, and a page that met an unreadable
/// record has still used that position up. [`page`] drops the `None`s, and
/// with them both facts; W1-api1-10 found the window paying for that (D-0730).
fn slots(file: &BarFile, skip: usize, take: usize) -> (Vec<Option<Bar>>, FaultLog) {
    let mut faults = FaultLog::default();
    let rows = slots_into(file, skip, take, &mut faults);
    (rows, faults)
}

/// [`slots`], noting what would not read in the caller's log.
fn slots_into(file: &BarFile, skip: usize, take: usize, faults: &mut FaultLog) -> Vec<Option<Bar>> {
    let n = file.header().n_valid;
    let mut rows = Vec::with_capacity(take.min(PAGE_BARS));
    for i in skip..skip.saturating_add(take) {
        let Ok(index) = u64::try_from(i) else {
            break;
        };
        if index >= n {
            break;
        }
        match file.read_record(index) {
            Ok(bar) => rows.push(Some(bar)),
            // COUNTED IN THE `Vec`, NOT EMITTED HERE. This arm is inside the
            // per-record loop, and `slots` itself runs once per month file of
            // a window; a line placed at either would be bounded by the data
            // and not by the request. The caller emits the aggregate once.
            // COALESCED, NOT FORMATTED PER RECORD (DB-B75): a run of one
            // reason over consecutive records is one sentence.
            Err(why) => {
                rows.push(None);
                faults.record(file.path(), index, why);
            }
        }
    }
    rows
}

/// The table of prices, as HTML.
///
/// Every number is right-aligned and tabular so a column of prices lines up on
/// the decimal point, which is the whole reason a price table is readable at
/// all. Rising bars are tinted green and falling ones red — the convention every
/// terminal uses, applied to `close >= open`.
#[must_use]
pub fn table(rows: &[Bar]) -> String {
    let mut out = String::with_capacity(512 + rows.len() * 220);
    out.push_str(
        "<div class=\"hscroll\"><table class=\"bars\"><thead><tr>\
         <th>#</th><th>Date IST</th><th>Time IST</th><th class=\"num\">Open</th><th class=\"num\">High</th>\
         <th class=\"num\">Low</th><th class=\"num\">Close</th><th class=\"num\">Change</th>\
         <th class=\"num\">Volume</th><th class=\"num\">Open interest</th>\
         </tr></thead><tbody>",
    );
    for (i, bar) in rows.iter().enumerate() {
        // `close - open` in paisa, then rendered by the same integer formatter.
        // A percentage would need a division and this file holds no float.
        let delta = bar.close.saturating_sub(bar.open);
        let dir = match delta.cmp(&0) {
            core::cmp::Ordering::Greater => "up",
            core::cmp::Ordering::Less => "down",
            core::cmp::Ordering::Equal => "flat",
        };
        let sign = if delta > 0 { "+" } else { "" };
        let _ = write!(
            out,
            "<tr class=\"{dir}\"><td class=\"idx\">{}</td><td class=\"day\">{}</td>\
             <td class=\"clock\">{}</td>\
             <td class=\"num\">{}</td><td class=\"num\">{}</td><td class=\"num\">{}</td>\
             <td class=\"num close\">{}</td><td class=\"num delta\">{sign}{}</td>\
             <td class=\"num\">{}</td><td class=\"num oi\">{}</td></tr>",
            i + 1,
            ist_day(bar.ts_micros),
            ist_clock(bar.ts_micros),
            rupees(bar.open),
            rupees(bar.high),
            rupees(bar.low),
            rupees(bar.close),
            rupees(delta),
            bar.volume,
            // `i64::MIN` is the null, and zero means zero — CLAUDE.md §7. A
            // dash for the null and a `0` for a real zero are different claims
            // and must not render the same.
            if bar.open_interest == OI_NULL {
                "—".to_owned()
            } else {
                bar.open_interest.to_string()
            }
        );
    }
    out.push_str("</tbody></table></div>");
    out
}

/// The stylesheet the price table needs, appended to the shared one.
pub const BARS_STYLE: &str = "\
table.bars td.num,table.bars th.num{text-align:right;font-variant-numeric:tabular-nums;\
font-family:ui-monospace,SFMono-Regular,Menlo,Consolas,monospace}\
table.bars td.idx{color:var(--dim);font-size:12px;font-variant-numeric:tabular-nums}\
table.bars td.clock{font-family:ui-monospace,SFMono-Regular,Menlo,Consolas,monospace;\
font-weight:650;font-variant-numeric:tabular-nums}\
table.bars td.close{font-weight:700}\
table.bars tr.up td.close,table.bars tr.up td.delta{color:var(--ok)}\
table.bars tr.down td.close,table.bars tr.down td.delta{color:var(--bad)}\
table.bars tr.flat td.delta{color:var(--dim)}\
table.bars td.oi{color:var(--dim)}\
table.bars tbody tr:hover{background:color-mix(in srgb,var(--acc) 6%,transparent)}\
";

/// The most months one window request may span.
///
/// `CLAUDE.md` §3 rule 4 is about per-operation cost and this is the operation:
/// a caller naming a thousand-year range must not turn one request into a
/// thousand file opens. Eighty-one months is the whole of this store today, so
/// 240 leaves two decades of headroom and still bounds the work.
pub const MAX_WINDOW_MONTHS: usize = 240;

/// The most rows one window request may return.
pub const MAX_WINDOW_LIMIT: usize = 1_000;

/// Which column a window is ordered by.
///
/// # Why `Ts` is not just another variant
///
/// The store is indexed by TIME — `docs/02-store-format.md`, the path is the
/// index — so a window ordered by `Ts` is answered by SEEKING: the prefix sum
/// over `n_valid` says which file holds row N and `page` reads it by index.
/// Nothing scans, and page 12,470 costs what page 1 costs.
///
/// Every other variant asks a question the time index does not answer. "The
/// fifty largest closes" could not be known without reading the closes, so
/// those variants scanned, once, here (D-3326) — every kept row of the range,
/// 43.9 million for one-second NIFTY, refused past [`MAX_SCAN_BARS`]
/// (DB-N14). Since D-3349 the six value columns are served from each month's
/// `.srt` sort index (`store::sort_index`), which orders the month's rows by
/// each column when they are written; only rows no index run orders are read,
/// keeping the `offset + limit` best, capped and named on the wire.
/// [`Self::TimeOfDay`] has no index and still reads its kept rows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortKey {
    /// The store's own order.
    Ts,
    /// Open, high, low, close.
    Open,
    /// The high.
    High,
    /// The low.
    Low,
    /// The close.
    Close,
    /// Traded volume.
    Volume,
    /// Open interest, nulls last IN BOTH DIRECTIONS.
    ///
    /// A null is "this feed stamps none", not a small number, so it never
    /// leads a page. Mapping it to `i64::MIN` alone put every null FIRST on an
    /// ascending page while this line said last (Z1-slice11-F5, D-1762); the
    /// window's comparator now ranks null-ness ahead of the value and does not
    /// invert it with the direction. An index's or a share's open interest is
    /// not applicable (D-3322) and sorts as a null.
    OpenInterest,
    /// The IST second of the day, so "every bar at 10:00" sorts together
    /// across days (D-3326, DB-006). Ties keep time order.
    TimeOfDay,
}

impl SortKey {
    /// Reads the wire spelling, which is the column key the grid sorts on.
    #[must_use]
    pub fn parse(word: &str) -> Option<Self> {
        match word {
            "" | "ts" => Some(Self::Ts),
            "o" => Some(Self::Open),
            "h" => Some(Self::High),
            "l" => Some(Self::Low),
            "c" => Some(Self::Close),
            "v" => Some(Self::Volume),
            "oi" => Some(Self::OpenInterest),
            "tod" => Some(Self::TimeOfDay),
            _ => None,
        }
    }

    /// Whether this order is not time order: a value or time-of-day sort.
    /// A value sort reads only the rows no `.srt` run orders (D-3349).
    #[must_use]
    pub const fn scans(self) -> bool {
        !matches!(self, Self::Ts)
    }

    /// The field this orders on, for one bar.
    ///
    /// `OI_NULL` is `i64::MIN`, and the value is returned as it is: the
    /// window's comparator puts null rows last before it compares this, so a
    /// null is never ordered as a real number. Zero means zero here exactly as
    /// `CLAUDE.md` §7 says it does.
    #[must_use]
    const fn of(self, bar: &Bar) -> i64 {
        match self {
            Self::Ts => bar.ts_micros,
            Self::Open => bar.open,
            Self::High => bar.high,
            Self::Low => bar.low,
            Self::Close => bar.close,
            Self::Volume => bar.volume,
            Self::OpenInterest => bar.open_interest,
            Self::TimeOfDay => (bar.ts_micros.div_euclid(1_000_000)
                + pull::session::IST_OFFSET_SECS)
                .rem_euclid(86_400),
        }
    }
}

/// The most rows a non-time sort may READ before it is refused
/// `scan_budget_exceeded` (D-3326): every kept row of a time-of-day sort, and
/// of a value sort the rows no `.srt` run orders (D-3349), named with the
/// command that indexes them.
///
/// Measured on the shadow store (WP-31, 2026-10-09, release build, sequential
/// GETs): a close sort over twelve months of one-second NIFTY (5,418,076
/// bars) answered p50 279 ms and p99 353 ms over 200 pages, about 52 ns a
/// bar with the blocks cached, and 529 ms on a cold first read. So this
/// bound is a little over a second of one scan-pool thread. It admits those
/// twelve months and every option chain of one expiry; it refuses the whole
/// 97-month one-second history (43,942,544 bars), which is named with the
/// count it would need.
pub const MAX_SCAN_BARS: u64 = 24_000_000;

/// The most records one request may fold for the EXTREMES that no earlier
/// request folded (D-3326).
///
/// Larger than [`MAX_SCAN_BARS`] because a whole month's extremes are folded
/// once per file stamp and served from memory after: the first request over
/// the whole 97-month one-second NIFTY history (43.9 million bars) pays about
/// three seconds on a scan-pool thread (measured 3,093 ms on the shadow store,
/// WP-31, 2026-10-09), and every later one a lookup per month (1 ms). Past
/// this the page is served and the extremes are refused by name.
pub const MAX_EXTREMES_SCAN_BARS: u64 = 64_000_000;

/// The deepest page a non-time sort serves: `offset + limit` rows are kept
/// while it streams, and past this it is refused `sort_too_deep` (D-3326).
pub const MAX_SORT_DEPTH: usize = 100_000;

/// The widest move and the heaviest volume in a window, and where the
/// window's extreme prices and volume are.
///
/// # Why this is computed here and not in the browser
///
/// The grid scales its magnitude bars against the widest range in the QUERY, so
/// that turning a page cannot change what a full bar means. Computing that in
/// the browser meant fetching every row of every month first, which is the
/// 2,187-request storm the window endpoint exists to end. One scan here, in
/// Rust, over files already on this disk, answers it in one number.
///
/// # Where, not only how much (D-3326, DB-G95)
///
/// `range` and `volume` were all it held, so the widest bar could not be
/// found from the page and no extreme close was shown at all. Each extreme
/// now carries its value, its instant and its series. A tie keeps the
/// EARLIER instant, then the earlier series, so the answer is the same
/// however the months were folded.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Extremes {
    /// The widest `high - low` in the window, in paisa.
    pub range: i64,
    /// The heaviest `volume`. Zero is a real zero — a spot index has none.
    pub volume: i64,
    /// The largest close.
    pub close_max: Option<Extreme>,
    /// The smallest close.
    pub close_min: Option<Extreme>,
    /// The largest high.
    pub high_max: Option<Extreme>,
    /// The smallest low.
    pub low_min: Option<Extreme>,
    /// The widest `high - low`.
    pub range_max: Option<Extreme>,
    /// The heaviest volume. `None` over index series only, which trade none.
    pub volume_max: Option<Extreme>,
}

/// One extreme: its value, its bar's instant and the series it is in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Extreme {
    /// The value, paisa or a count.
    pub v: i64,
    /// The bar's stamp, UTC micros.
    pub t: i64,
    /// Index into the request's series.
    pub series: usize,
}

impl Extreme {
    /// Whether `self` should replace `held` as the extreme, `larger` saying
    /// which way the extreme runs.
    fn beats(self, held: Option<Self>, larger: bool) -> bool {
        held.is_none_or(|held| {
            let better = if larger {
                self.v > held.v
            } else {
                self.v < held.v
            };
            better || (self.v == held.v && (self.t, self.series) < (held.t, held.series))
        })
    }
}

impl Extremes {
    /// Folds one bar of series `series` in. `volume` is false for an index.
    fn fold(&mut self, bar: &Bar, series: usize, volume: bool) {
        let range = bar.high.saturating_sub(bar.low);
        self.range = self.range.max(range);
        let at = |v: i64| Extreme {
            v,
            t: bar.ts_micros,
            series,
        };
        for (slot, value, larger) in [
            (&mut self.close_max, bar.close, true),
            (&mut self.close_min, bar.close, false),
            (&mut self.high_max, bar.high, true),
            (&mut self.low_min, bar.low, false),
            (&mut self.range_max, range, true),
        ] {
            if at(value).beats(*slot, larger) {
                *slot = Some(at(value));
            }
        }
        if volume {
            self.volume = self.volume.max(bar.volume);
            if at(bar.volume).beats(self.volume_max, true) {
                self.volume_max = Some(at(bar.volume));
            }
        }
    }

    /// Combines another fold in, renaming its series to `series` when given.
    fn merge(&mut self, other: &Self, series: Option<usize>) {
        let named = |held: Option<Extreme>| {
            held.map(|extreme| Extreme {
                series: series.unwrap_or(extreme.series),
                ..extreme
            })
        };
        self.range = self.range.max(other.range);
        self.volume = self.volume.max(other.volume);
        for (slot, theirs, larger) in [
            (&mut self.close_max, named(other.close_max), true),
            (&mut self.close_min, named(other.close_min), false),
            (&mut self.high_max, named(other.high_max), true),
            (&mut self.low_min, named(other.low_min), false),
            (&mut self.range_max, named(other.range_max), true),
            (&mut self.volume_max, named(other.volume_max), true),
        ] {
            if let Some(theirs) = theirs
                && theirs.beats(*slot, larger)
            {
                *slot = Some(theirs);
            }
        }
    }
}

/// A bar, with the change the grid draws beside it.
///
/// # Why the change travels with the bar instead of being derived on arrival
///
/// The grid's CHANGE % is against the PREVIOUS STORED BAR of the series, in
/// time. A price-ordered page has no time neighbours at all, so a page
/// fetched already-sorted cannot compute the column on arrival. The number is
/// therefore decided HERE, against the record written before it — read even
/// when it lies in a month or day the request did not ask for (D-3325) — and
/// refused by name where that record is not the comparable one.
///
/// `why` carries the grid's own vocabulary rather than a new one, so a cell that
/// cannot be computed says the same thing it said before.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WindowBar {
    /// The record as stored.
    pub bar: Bar,
    /// Basis points against the previous stored bar, when it is the
    /// comparable one and the arithmetic holds.
    pub chg: Option<i64>,
    /// Empty when `chg` is `Some`; otherwise the reason, in the grid's words.
    pub chg_why: &'static str,
    /// The same, for open interest.
    pub oichg: Option<i64>,
    /// Empty when `oichg` is `Some`; otherwise the reason.
    pub oichg_why: &'static str,
    /// Index into the request's series.
    pub series: usize,
    /// The session phase of the bar's minute, on an intraday rung (D-3320).
    pub phase: Option<&'static str>,
    /// Whether open interest applies: false for an index or a share, whose
    /// stored value is served as `null` (D-3322).
    pub oi_applies: bool,
    /// Why the volume is not a traded volume: `index_no_volume` (D-3322).
    pub v_why: Option<&'static str>,
}

/// What stands behind a row in time, for its change columns.
#[derive(Debug, Clone, Copy)]
enum Behind {
    /// No earlier bar of the series is stored at all.
    Nothing,
    /// The predecessor exists and would not read. Not a zero: the row before
    /// a gap is not this row's predecessor (D-0730).
    Unreadable,
    /// The request turned the lookback off and the predecessor is outside
    /// it: the code says where (D-3325).
    NotRead(&'static str),
    /// The record written immediately before.
    Bar(Bar),
}

/// The change columns of `bar`, behind which stands `behind`, under the
/// series' D-3325 `rules`. `crate::server::basis_points` is called rather
/// than re-implemented: it rounds half away from zero and returns a named
/// refusal, and a second spelling of that arithmetic would drift.
fn change_of(
    rules: &change::Rules,
    behind: Behind,
    bar: &Bar,
) -> (Option<i64>, &'static str, Option<i64>, &'static str) {
    let ratio = |base: i64, value: i64, zero: &'static str| match crate::server::basis_points(
        base, value,
    ) {
        Ok(bps) => (Some(bps), ""),
        Err(crate::server::Unknown::Overflow) => (None, "overflow"),
        Err(_) => (None, zero),
    };
    let absent: Option<&'static str> = match behind {
        Behind::Nothing => Some("no_earlier_stored_bar"),
        Behind::Unreadable => Some("previous_unreadable"),
        Behind::NotRead(code) => Some(code),
        Behind::Bar(_) => None,
    };
    let oi_null = bar.open_interest == OI_NULL;
    match (behind, absent) {
        (Behind::Bar(before), _) => {
            let common = rules.judge(&before, bar);
            let (chg, chg_why) = common.map_or_else(
                || ratio(before.close, bar.close, "previous_close_zero"),
                |code| (None, code),
            );
            let (oichg, oichg_why) = if rules.not_a_derivative() {
                (None, "not_a_derivative")
            } else if let Some(code) = common {
                (None, code)
            } else if rules.oi_unadjusted(bar) {
                (None, "vendor_oi_unadjusted")
            } else if oi_null {
                (None, "oi_null")
            } else if before.open_interest == OI_NULL {
                (None, "oi_null_before")
            } else if bar.open_interest == 0 && before.open_interest > 0 {
                // A FALL TO EXACTLY ZERO IS NOT SHOWN AS -100% (WP-53, DB-N10;
                // D-3325's change refusals): GDFL NIFTY 2019-03-28 11,600 CE
                // printed 8,025 / 0 / 8,325 on three consecutive bars of
                // 2019-01-15/16, a lone zero between non-zero prints. Which
                // zeros are real closes is not sourced (UNVERIFIED), so the
                // change is withheld by name; the zero itself is shown as
                // stored (`CLAUDE.md` §7: zero means zero). Only this bar and
                // the one before are read, so nothing later decides it.
                (None, "oi_to_zero")
            } else {
                ratio(before.open_interest, bar.open_interest, "previous_oi_zero")
            };
            (chg, chg_why, oichg, oichg_why)
        }
        (_, code) => {
            let code = code.unwrap_or("no_earlier_stored_bar");
            // A NULL OPEN INTEREST IS STILL NAMED FIRST when there is no
            // predecessor to measure against: it is a fact about this row.
            let oi = if rules.not_a_derivative() {
                "not_a_derivative"
            } else if oi_null {
                "oi_null"
            } else {
                code
            };
            (None, code, None, oi)
        }
    }
}

/// A bar with its change decided, labelled for the wire.
fn window_bar(rules: &change::Rules, behind: Behind, bar: Bar, series: usize) -> WindowBar {
    let (chg, chg_why, oichg, oichg_why) = change_of(rules, behind, &bar);
    WindowBar {
        bar,
        chg,
        chg_why,
        oichg,
        oichg_why,
        series,
        phase: rules.phase_word(&bar),
        oi_applies: !rules.not_a_derivative(),
        v_why: rules.index_no_volume().then_some("index_no_volume"),
    }
}

/// A series the current decoder files under another key (D-3331; WP-12c,
/// DB-A17): still served, and named with the key it continues as.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SupersededBy {
    /// The census key asked for.
    pub key: String,
    /// The census key the decoder files it under.
    pub by: String,
    /// Why its expiry moved: `closed_day` or `vendor_redated`.
    pub kind: &'static str,
    /// `true` when the re-key recorded it (`superseded` in
    /// `imports/gdfl.lineage`, the bars published under `by`); `false` when
    /// the decoder disagrees with the stored key and the re-key that moves it
    /// has not run (`cli gdfl-rekey --apply`, WP-80), so `by` holds no bar
    /// of it yet.
    pub recorded: bool,
}

/// A series some of whose bars the re-key copied in from a stored key the
/// decoder files here (D-3331; WP-12c, DB-A17): the reverse of
/// [`SupersededBy`], so the copy can be audited from the key it landed in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Supersedes {
    /// The census key asked for.
    pub key: String,
    /// The stored key the bars were copied from; still readable, and kept.
    pub of: String,
    /// Why its expiry moved: `closed_day` or `vendor_redated`.
    pub kind: &'static str,
    /// Bars copied, as the ledger line states them.
    pub bars: u64,
    /// BLAKE3 of those bars' records in write order, 64 hex digits.
    pub blake3: String,
}

/// The sentence `/provenance.json` adds about a key the re-key superseded,
/// copied into or retired (D-3331; WP-12c), or `None` for any other key:
/// one `stat` of the lineage ledger and two hash probes.
pub(crate) fn rekey_note(
    store_root: &std::path::Path,
    vendor: Vendor,
    key: &str,
) -> Option<String> {
    change::rekey_note(store_root, vendor, key)
}

/// The refusal a reader that opens a month ITSELF, or answers from a memo,
/// rather than through [`open_classified`] (`/dayflags.json`, `/sources.json`
/// and its two timelines), answers a key the re-key RETIRED
/// with (D-3331; WP-12c, DB-A18), or `None` for every other key: the same
/// sentence the bar routes answer 410 with. One `stat` of the lineage ledger
/// and one hash probe.
pub(crate) fn retired_refusal(
    store_root: &std::path::Path,
    vendor: Vendor,
    key: &str,
) -> Option<String> {
    change::retired_refusal(store_root, vendor, key)
}

/// What the re-key says of one asked series: refused, superseded, or
/// nothing. O(1): one ledger probe, and for a GDFL option the decoder's
/// settlement of its stored expiry (bounded walks over compile-time tables).
///
/// UNVERIFIED: this cost names no test or bench that proves it (docs/06-limits.md,
/// "Cost claims of the /db fix round that name no proof").
fn rekey_status(
    lineage: &change::Lineage,
    vendor: Vendor,
    key: &SeriesKey,
) -> Result<Option<SupersededBy>, String> {
    let text = key.text();
    match lineage.status_of(&text) {
        Some(change::KeyStatus::Retired(why)) => return Err(why.clone()),
        Some(change::KeyStatus::Superseded { by, kind }) => {
            return Ok(Some(SupersededBy {
                key: text,
                by: by.clone(),
                kind,
                recorded: true,
            }));
        }
        None => {}
    }
    if vendor != Vendor::Gdfl || key.exchange != "NSE" || key.segment != "FNO" {
        return Ok(None);
    }
    let decoded = key.contract.and_then(|contract| {
        let underlying = brutex_core::symbol::Symbol::new(&key.symbol).ok()?;
        pull::gdfl_rekey::decoded_key(underlying, contract)
            .map(|(to, kind)| (pull::gdfl_lineage::option_key(underlying, to), kind))
    });
    Ok(decoded.map(|(by, kind)| SupersededBy {
        key: text,
        by,
        kind: kind.as_str(),
        recorded: false,
    }))
}

/// [`rekey_status`] of every asked series, the superseded ones, and the
/// keys the re-key copied into each (one more probe per series); or the
/// first retired one's refusal.
fn rekey_statuses(
    lineage: &change::Lineage,
    query: &WindowQuery,
) -> Result<(Vec<SupersededBy>, Vec<Supersedes>), String> {
    let mut superseded = Vec::with_capacity(query.series.len());
    let mut copied_in = Vec::new();
    for key in &query.series {
        if let Some(one) = rekey_status(lineage, query.vendor, key)? {
            superseded.push(one);
        }
        let text = key.text();
        copied_in.extend(lineage.predecessors_of(&text).iter().map(|one| Supersedes {
            key: text.clone(),
            of: one.of.clone(),
            kind: one.kind,
            bars: one.bars,
            blake3: one.blake3.clone(),
        }));
    }
    Ok((superseded, copied_in))
}

/// One series' total, as the answer names it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeriesTotal {
    /// The series' census key.
    pub key: String,
    /// Its kept rows.
    pub total: u64,
}

/// One held month's coverage, when asked for (`coverage=1`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MonthCoverage {
    /// Index into the request's series.
    pub series: usize,
    /// The month.
    pub month: YearMonth,
    /// Its committed records.
    pub rows: u64,
    /// The IST days holding at least one of them (DB-G35).
    pub days_covered: u32,
}

/// One page of bars drawn from a RANGE of months, over one or more series.
#[derive(Debug, Clone, Default)]
pub struct Window {
    /// Every row the window keeps, across every series and month, after the
    /// day and time-of-day bounds.
    pub total: u64,
    /// The offset the page starts at: the one asked for, or the page that
    /// holds `at_index` when `at` was asked without an offset.
    pub offset: usize,
    /// Held months read, across every series.
    pub months_read: usize,
    /// Months of the range with no file. Not an error: a store is allowed to
    /// be sparse, and saying so is how a reader tells a gap from a refusal.
    pub months_missing: usize,
    /// Up to [`MAX_NAMED_MISSING`] of the missing months that lie INSIDE a
    /// series' held life — the holes.
    pub missing_months: Vec<String>,
    /// Missing months before a series' first held month or after its last:
    /// expected absences, counted apart.
    pub months_outside_life: usize,
    /// The rows asked for, each with its change decided.
    pub bars: Vec<WindowBar>,
    /// Records and months that would not read, named. Never silently dropped.
    /// A run of records refused for one cause is ONE sentence (DB-B75).
    pub faults: Vec<String>,
    /// Every record the request could not read, named or not (DB-B75).
    pub faulted_records: u64,
    /// Present only when asked for, because it costs a scan.
    pub extremes: Option<Extremes>,
    /// Why the extremes asked for are not given, when they are not.
    pub extremes_why: Option<String>,
    /// Each series' key and total, in the order asked.
    pub series: Vec<SeriesTotal>,
    /// The global index of the instant asked with `at`.
    pub at_index: Option<u64>,
    /// Records read beyond the page, by a scan or an extremes fold.
    pub scan_rows: u64,
    /// Days covered per held month, when asked for.
    pub coverage: Vec<MonthCoverage>,
    /// What a value sort served from sort indexes and what it read (D-3349).
    pub sort_index: Option<SortReport>,
    /// The asked series the current decoder files under another key, each
    /// with that key (D-3331; WP-12c).
    pub superseded: Vec<SupersededBy>,
    /// The stored keys whose bars the re-key copied into an asked series
    /// (D-3331; WP-12c).
    pub supersedes: Vec<Supersedes>,
}

/// The most missing months named in one answer; the count is always whole.
pub const MAX_NAMED_MISSING: usize = 24;

/// What one window request asks for. D-3326.
#[derive(Debug, Clone)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "four independent yes/no questions of the request — direction, \
              extremes, lookback, coverage — each read off its own query \
              parameter; folding them into a state enum would name sixteen \
              states nobody asks about"
)]
pub struct WindowQuery {
    /// The feed.
    pub vendor: Vendor,
    /// The rung.
    pub timeframe: Timeframe,
    /// The series, one to [`MAX_SERIES`].
    pub series: Vec<SeriesKey>,
    /// First month of the range.
    pub from: YearMonth,
    /// Last month of the range.
    pub to: YearMonth,
    /// The order.
    pub sort: SortKey,
    /// Newest (or largest) first.
    pub desc: bool,
    /// Rows to skip; `None` is 0, or the page holding `at_index`.
    pub offset: Option<usize>,
    /// Rows to return, at most [`MAX_WINDOW_LIMIT`].
    pub limit: usize,
    /// Whether to fold the extremes.
    pub want_extremes: bool,
    /// An instant (UTC micros) whose global index to report; `sort=ts` only.
    pub at: Option<i64>,
    /// The day and time-of-day bounds.
    pub filter: Filter,
    /// Whether a row's predecessor outside the request is read (D-3325).
    pub lookback: bool,
    /// Whether to report days covered per held month.
    pub coverage: bool,
}

impl WindowQuery {
    /// The one-series, unfiltered query the window route took before D-3326.
    #[must_use]
    #[expect(
        clippy::too_many_arguments,
        reason = "the pre-D-3326 window's own coordinates, kept as one \
                  constructor so its callers name the same seven they always did"
    )]
    pub fn single(
        vendor: Vendor,
        key: SeriesKey,
        timeframe: Timeframe,
        from: YearMonth,
        to: YearMonth,
        sort: SortKey,
        desc: bool,
        offset: usize,
        limit: usize,
        want_extremes: bool,
    ) -> Self {
        Self {
            vendor,
            timeframe,
            series: vec![key],
            from,
            to,
            sort,
            desc,
            offset: Some(offset),
            limit,
            want_extremes,
            at: None,
            filter: Filter::default(),
            lookback: true,
            coverage: false,
        }
    }
}

/// The months of a range, oldest first, bounded.
///
/// Returns `None` when the range is inverted or longer than
/// [`MAX_WINDOW_MONTHS`] — both are refusals rather than clamps, because a
/// silently shortened range answers a narrower question than the one asked and
/// nothing on the page would say so.
#[must_use]
pub fn months_of(from: YearMonth, to: YearMonth) -> Option<Vec<YearMonth>> {
    let (fy, fm) = (i32::from(from.year()), i32::from(from.month()));
    let (ty, tm) = (i32::from(to.year()), i32::from(to.month()));
    let span = (ty - fy) * 12 + (tm - fm);
    if span < 0 {
        return None;
    }
    let count = usize::try_from(span).ok()?.checked_add(1)?;
    if count > MAX_WINDOW_MONTHS {
        return None;
    }
    let mut out = Vec::with_capacity(count);
    let (mut y, mut m) = (fy, fm);
    for _ in 0..count {
        let year = u16::try_from(y).ok()?;
        let month = u8::try_from(m).ok()?;
        out.push(YearMonth::new(year, month).ok()?);
        m += 1;
        if m > 12 {
            m = 1;
            y += 1;
        }
    }
    Some(out)
}

/// The extremes of one whole month file, by its stamp.
static MONTH_EXTREMES: series::Memo<(std::path::PathBuf, series::Stamp), Extremes> =
    series::Memo::new();

/// One page of bars across a range of months and one or more series, in one
/// request. D-3326.
///
/// # The paths, and what each costs
///
/// * **Ordered by time, one series** — SEEKS. The prefix sums over the held
///   months say which month holds row `offset`, and the page is read by index:
///   `O(months)` `stat`s plus `O(limit)` record reads, page 12,470 costing
///   what page 1 costs.
/// * **Ordered by time, several series** — a MERGE BY TIME BISECTION. The
///   instant of global row `offset` is found by bisecting time, each step
///   summing every series' rank through its `.tix` (`O(k · log T)`), and the
///   page is then merged from each series' own position: `O(k + limit)`
///   record reads and `O(k · limit)` comparisons. Ties in time are ordered by
///   the series' place in the request, so the order is total and stable.
/// * **Ordered by a value column** — from the months' `.srt` sort indexes
///   (D-3349): the value of row `offset` by a bisection over each run's fence
///   table, its instant by a bisection over time, then `limit` rows merged
///   from one cursor per run (`sort::page`). A run a day bound cuts is
///   served from its own entries stamped inside the bound. Rows no run
///   orders — a month without its index, rows appended after it, every row
///   under a time-of-day bound — are read and named, refused past
///   [`MAX_SCAN_BARS`]; up to a million of them are ordered whole, and past
///   that a page within [`MAX_SORT_DEPTH`] of the front is served, or of the
///   end when no row is indexed. Open interest over series that have none
///   (an index, a share) is time order and is a seek.
/// * **Ordered by time of day** — streams every kept row once, keeping only
///   the `offset + limit` best (`O(bars)` time, `O(offset + limit)` memory),
///   refused past [`MAX_SCAN_BARS`] and [`MAX_SORT_DEPTH`] by name.
/// * **`at`** — the global index of an instant: one rank per series.
/// * **Extremes** — each index run's own summary (D-3349), whole months
///   no run covers from a per-month memo keyed by the file's stamp, the rest
///   folded by a scan under the same budget.
///
/// # Errors
///
/// The range being inverted or too long, too many or no series, an invalid
/// address, an unavailable store root, or a scan past its budget.
pub fn window(store_root: &std::path::Path, query: &WindowQuery) -> Result<Window, String> {
    window_cancellable(store_root, query, &Cancel::never())
}

/// [`window`], stopping with a `cancelled` refusal once `cancel` is set —
/// looked at between the plan and the read, and every few thousand rows of
/// any read beyond the page (DB-N15).
///
/// # Errors
///
/// What [`window`] refuses, and `cancelled`.
#[expect(
    clippy::too_many_lines,
    reason = "the plan, the three ways a page is answered and the extremes, in \
              the order the doc above states them; each branch is one call"
)]
pub fn window_cancellable(
    store_root: &std::path::Path,
    query: &WindowQuery,
    cancel: &Cancel,
) -> Result<Window, String> {
    let Some(months) = months_of(query.from, query.to) else {
        return Err(format!(
            "{}-{:02} to {}-{:02} is not a month range this build will read: it \
             must run forwards and span at most {MAX_WINDOW_MONTHS} months.",
            query.from.year(),
            query.from.month(),
            query.to.year(),
            query.to.month()
        ));
    };
    if query.series.is_empty() || query.series.len() > MAX_SERIES {
        return Err(format!(
            "too_many_series: a window reads 1 to {MAX_SERIES} series and {} were named",
            query.series.len()
        ));
    }
    root_is_a_directory(store_root)?;
    let limit = query.limit.min(MAX_WINDOW_LIMIT);
    let mut plans = Vec::with_capacity(query.series.len());
    let mut rules = Vec::with_capacity(query.series.len());
    let mut out = Window::default();
    // THE LINEAGE LEDGER IS READ ONCE PER REQUEST, and its fault said once.
    let lineage = change::Lineage::read(store_root, query.vendor);
    out.faults.extend(lineage.fault.clone());
    // A RETIRED SERIES IS REFUSED BY NAME, and a superseded one is served
    // with the key it continues as (D-3331; WP-12c).
    (out.superseded, out.supersedes) = rekey_statuses(&lineage, query)?;
    for key in &query.series {
        rules.push(change::Rules::of(
            store_root,
            query.timeframe,
            key,
            &lineage,
        ));
        cancel.check()?;
        plans.push(SeriesPlan::build(
            store_root,
            query.vendor,
            query.timeframe,
            key.clone(),
            &months,
            query.filter,
            cancel,
        )?);
    }
    let unplanned = tally(&mut out, &mut plans);
    note_holes(&plans);
    if let Some(at) = query.at {
        let index = at_index(store_root, &mut plans, at, query.desc, out.total)?;
        out.at_index = Some(index);
    }
    out.offset = query.offset.unwrap_or_else(|| {
        out.at_index.map_or(0, |index| {
            let last = index.min(out.total.saturating_sub(1));
            usize::try_from(last).map_or(0, |last| last - last % limit.max(1))
        })
    });
    cancel.check()?;
    let mut record_faults = FaultLog::default();
    if query.sort == SortKey::OpenInterest && rules.iter().all(change::Rules::not_a_derivative) {
        // NO ROW HAS AN OPEN INTEREST: an index's or a share's is not
        // applicable (D-3322) and sorts as a null, and nulls are ordered by
        // time, then series, in BOTH directions. That is the ascending
        // time order, so the page is a seek — `limit` reads, not a merge of
        // null slices (D-3349).
        let ascending = WindowQuery {
            desc: false,
            ..query.clone()
        };
        out.bars = seek(
            store_root,
            &mut plans,
            &rules,
            &ascending,
            (out.offset, limit),
            &mut record_faults,
        )?;
        if query.want_extremes {
            match sort::extremes_of(
                store_root,
                &mut plans,
                &rules,
                query,
                &mut record_faults,
                cancel,
            ) {
                Ok((extremes, rows)) => {
                    out.extremes = Some(extremes);
                    out.scan_rows = rows;
                }
                Err(why) if why.starts_with("cancelled") => return Err(why),
                Err(why) => out.extremes_why = Some(why),
            }
        }
        out.sort_index = Some(SortReport {
            column: "oi",
            by_time: true,
            ..SortReport::default()
        });
    } else if sort::column_of(query.sort).is_some() {
        // A VALUE SORT IS SERVED FROM THE SORT INDEXES (D-3349): only the
        // rows no index run orders are read, capped and named.
        let sorted = sort::page(
            store_root,
            &mut plans,
            &rules,
            query,
            (out.offset, limit),
            &mut record_faults,
            cancel,
        )?;
        out.scan_rows = sorted.scanned;
        out.bars = sorted.page;
        out.extremes = sorted.extremes;
        out.extremes_why = sorted.extremes_why;
        out.sort_index = Some(sorted.report);
    } else if query.sort.scans() {
        scan_admitted(out.total, query.sort, out.offset, limit)?;
        let scanned = scan(
            store_root,
            &mut plans,
            &rules,
            query,
            (out.offset, limit),
            &mut record_faults,
            cancel,
        )?;
        out.scan_rows = scanned.rows;
        out.bars = scanned.page;
        if query.want_extremes && scanned.unread == 0 {
            out.extremes = Some(scanned.extremes);
        } else if query.want_extremes {
            // NEVER THE EXTREMES OF THE MONTHS THAT HAPPENED TO READ.
            out.extremes_why = Some(format!(
                "months_unread: {} kept rows of this window would not read, so its extremes are \
                 not known and none is served",
                scanned.unread
            ));
        }
    } else {
        out.bars = seek(
            store_root,
            &mut plans,
            &rules,
            query,
            (out.offset, limit),
            &mut record_faults,
        )?;
        if query.want_extremes {
            match sort::extremes_of(
                store_root,
                &mut plans,
                &rules,
                query,
                &mut record_faults,
                cancel,
            ) {
                Ok((extremes, rows)) => {
                    out.extremes = Some(extremes);
                    out.scan_rows = rows;
                }
                Err(why) if why.starts_with("cancelled") => return Err(why),
                Err(why) => out.extremes_why = Some(why),
            }
        }
    }
    if !record_faults.is_empty() {
        // THE FILE IS THE FAULT'S OWN PREFIX: a record and a run both name
        // their path before ` record`.
        let said = record_faults.clone().sentences();
        let first = said.first().map_or("", String::as_str);
        let file = first.split_once(" record").map_or(first, |(path, _)| path);
        note_unreadable_records(file, &record_faults, out.bars.len(), out.offset);
    }
    // COALESCED AND CAPPED (DB-B75): a scan over a month whose sidecar is
    // lost refuses every record, and each used to be its own sentence.
    out.faulted_records = record_faults.records();
    out.faults.extend(record_faults.sentences());
    // A HELD MONTH THE PLAN COULD NOT READ IS IN NO TOTAL AND NO FOLD (it
    // would not open: another process holds it, or its header is refused).
    // The extremes of the months that did read are not the window's, and
    // served beside a null `extremes_why` they would look exactly like it.
    if let Some((months, first)) = unplanned
        && out.extremes.is_some()
    {
        out.extremes = None;
        out.extremes_why = Some(format!(
            "months_unread: {months} held month(s) of this window would not open, so its \
             extremes are not known and none is served (the first: {first})"
        ));
    }
    if query.coverage {
        out.coverage = coverage_of(store_root, &mut plans, &mut out.faults);
    }
    Ok(out)
}

/// The store root is a readable directory, or the refusal naming it: an
/// unavailable root must never read as an empty history.
fn root_is_a_directory(store_root: &std::path::Path) -> Result<(), String> {
    let metadata = std::fs::metadata(store_root)
        .map_err(|why| format!("cannot read store root {}: {why}", store_root.display()))?;
    if metadata.is_dir() {
        Ok(())
    } else {
        Err(format!(
            "store root {} is not a directory",
            store_root.display()
        ))
    }
}

/// Folds every series' months, faults and total into the answer. Returns the
/// held months no plan could read — their count and the first's reason —
/// when there is one: they are in no total, and no extreme covers them.
fn tally(out: &mut Window, plans: &mut [SeriesPlan]) -> Option<(usize, String)> {
    let several = plans.len() > 1;
    let mut unplanned: Option<(usize, String)> = None;
    for plan in plans.iter_mut() {
        if let Some(first) = plan.faults.first() {
            let held = unplanned.get_or_insert_with(|| (0, first.clone()));
            held.0 += plan.faults.len();
        }
        out.months_read += plan.slots.len();
        out.months_missing += plan.holes.len() + plan.outside_life;
        out.months_outside_life += plan.outside_life;
        for hole in &plan.holes {
            if out.missing_months.len() < MAX_NAMED_MISSING {
                out.missing_months.push(if several {
                    format!("{} {hole}", plan.key.text())
                } else {
                    hole.to_string()
                });
            }
        }
        out.faults.append(&mut plan.faults);
        out.series.push(SeriesTotal {
            key: plan.key.text(),
            total: plan.total(),
        });
        out.total = out.total.saturating_add(plan.total());
    }
    unplanned
}

/// Whether a non-time sort over `total` kept rows, serving `limit` rows from
/// `offset`, is within its budgets — or the refusal naming what it would
/// need: `sort_too_deep` past [`MAX_SORT_DEPTH`] rows kept,
/// `scan_budget_exceeded` past [`MAX_SCAN_BARS`] rows read (D-3326). Decided
/// before a record is read, so a refused scan costs nothing.
///
/// # Errors
///
/// The refusal sentence, opening with its code.
fn scan_admitted(total: u64, sort: SortKey, offset: usize, limit: usize) -> Result<(), String> {
    let depth = offset.saturating_add(limit);
    if depth > MAX_SORT_DEPTH {
        return Err(format!(
            "sort_too_deep: a sorted page keeps offset + limit = {depth} rows while it \
             streams, past the {MAX_SORT_DEPTH} this route holds; page by time, or narrow \
             the range or the series"
        ));
    }
    if total > MAX_SCAN_BARS {
        return Err(format!(
            "scan_budget_exceeded: ordering by {sort:?} reads all {total} kept rows, past the \
             {MAX_SCAN_BARS} one request may stream; narrow the months, days, time of day \
             or series"
        ));
    }
    Ok(())
}

/// The global index, in the request's direction, of the first bar at or
/// after `at` (ascending) or at or before it (descending). D-3326.
///
/// Ascending, it is how many kept rows lie strictly before `at`. Descending,
/// the rows after `at` come first, so it is the total less the rows at or
/// before it. One rank per series.
fn at_index(
    root: &std::path::Path,
    plans: &mut [SeriesPlan],
    at: i64,
    desc: bool,
    total: u64,
) -> Result<u64, String> {
    let mut before = 0_u64;
    let probe = if desc { at.saturating_add(1) } else { at };
    for plan in plans.iter_mut() {
        before = before.saturating_add(plan.rank(root, probe)?.count);
    }
    Ok(if desc {
        total.saturating_sub(before)
    } else {
        before
    })
}

/// The ascending positions `[a, b)` a page covers. A descending page is the
/// ascending window read backwards, so it covers
/// `[total - offset - limit, total - offset)`, reversed after.
fn ascending_span(total: u64, offset: usize, limit: usize, desc: bool) -> (u64, u64) {
    let offset = u64::try_from(offset).unwrap_or(u64::MAX);
    let limit = u64::try_from(limit).unwrap_or(u64::MAX);
    if offset >= total {
        return (0, 0);
    }
    if desc {
        let end = total - offset;
        (end.saturating_sub(limit), end)
    } else {
        (offset, offset.saturating_add(limit).min(total))
    }
}

/// The last row a series emitted on this page: its slot, row and bar (or
/// `None` where it would not read), so the next row of the same file needs
/// no second read for its predecessor.
type Last = Option<(usize, u64, Option<Bar>)>;

/// What stands behind the kept row at `at` of `plan`.
///
/// The record before it in its file, or the last record of the held month
/// before — read even when it lies outside the request (one extra read per
/// series per page, D-3325), unless the request turned the lookback off and
/// it is not itself kept.
fn behind_of(
    root: &std::path::Path,
    plan: &mut SeriesPlan,
    at: series::Position,
    last: Last,
    lookback: bool,
) -> Behind {
    let Some(run) = plan.runs.get(at.run).copied() else {
        return Behind::Nothing;
    };
    if let Some((slot, row, bar)) = last
        && slot == run.slot
        && row.checked_add(1) == Some(at.row)
    {
        return bar.map_or(Behind::Unreadable, Behind::Bar);
    }
    let Some(month) = plan.month_of_slot(run.slot) else {
        return Behind::Nothing;
    };
    if !lookback && !plan.previous_is_kept(at) {
        if at.row > 0 {
            return Behind::NotRead("first_bar_in_window");
        }
        return match plan.previous_held(month) {
            None => Behind::Nothing,
            Some(before) if plan.slot_of(before).is_some() => {
                Behind::NotRead("first_bar_in_window")
            }
            Some(_) => Behind::NotRead("earlier_month_not_read"),
        };
    }
    if at.row > 0 {
        return plan
            .read(root, run.slot, at.row - 1)
            .map_or(Behind::Unreadable, Behind::Bar);
    }
    match plan.last_before(root, month) {
        Ok(Some(bar)) => Behind::Bar(bar),
        Ok(None) => Behind::Nothing,
        Err(_) => Behind::Unreadable,
    }
}

/// The time-ordered page: positions `[a, b)` of the ascending merge of every
/// series, read, their changes decided, reversed for a descending page.
fn seek(
    root: &std::path::Path,
    plans: &mut [SeriesPlan],
    rules: &[change::Rules],
    query: &WindowQuery,
    (offset, limit): (usize, usize),
    faults: &mut FaultLog,
) -> Result<Vec<WindowBar>, String> {
    let total: u64 = plans.iter().map(SeriesPlan::total).sum();
    let (a, b) = ascending_span(total, offset, limit, query.desc);
    if a >= b {
        return Ok(Vec::new());
    }
    let picked = if plans.len() == 1 {
        let Some(plan) = plans.first() else {
            return Ok(Vec::new());
        };
        let mut picked = Vec::with_capacity(usize::try_from(b - a).unwrap_or(0));
        let mut at = plan.nth(a);
        while let Some(position) = at.filter(|p| p.count < b) {
            picked.push((0, position));
            at = plan.after(position);
        }
        picked
    } else {
        merged(root, plans, a, b, query.timeframe)?
    };
    let mut last: Vec<Last> = vec![None; plans.len()];
    let mut out = Vec::with_capacity(picked.len());
    // A MONTH THAT WILL NOT OPEN IS ONE FAULT, however many of the page's rows
    // lie in it: the refusal is the month's, the same words for every row,
    // and a line a row was 50 copies of it on a page of 50. Counted here by
    // (series, slot), in the order first met, and said once below. A record
    // of a month that DID open and will not read is still named by itself.
    let mut unopened: Vec<(String, u64)> = Vec::new();
    let mut unopened_at: std::collections::HashMap<(usize, usize), usize> =
        std::collections::HashMap::new();
    for (series, at) in picked {
        let (Some(plan), Some(series_rules)) = (plans.get_mut(series), rules.get(series)) else {
            continue;
        };
        let Some(run) = plan.runs.get(at.run).copied() else {
            continue;
        };
        if let Err(why) = plan.opens(root, run.slot) {
            let index = *unopened_at.entry((series, run.slot)).or_insert_with(|| {
                unopened.push((why, 0));
                unopened.len() - 1
            });
            if let Some((_, rows)) = unopened.get_mut(index) {
                *rows += 1;
            }
            if let Some(slot) = last.get_mut(series) {
                *slot = Some((run.slot, at.row, None));
            }
            continue;
        }
        let held = last.get(series).copied().flatten();
        let behind = behind_of(root, plan, at, held, query.lookback);
        // POSITIONS USED, NOT ROWS RETURNED. An unreadable record still
        // occupies its place on the page: it is named in `faults` on the page
        // that covers it, and the next page does not return it again (D-0730).
        let read = plan.read(root, run.slot, at.row);
        if let Some(slot) = last.get_mut(series) {
            *slot = Some((run.slot, at.row, read.as_ref().ok().copied()));
        }
        match read {
            Ok(bar) => out.push(window_bar(series_rules, behind, bar, series)),
            Err(why) => faults.push(why),
        }
    }
    for (why, rows) in unopened {
        faults.push(format!(
            "{why}; the month did not open, so none of its {rows} rows on this page was read"
        ));
    }
    if query.desc {
        out.reverse();
    }
    Ok(out)
}

/// Positions `[a, b)` of the ascending time merge of several series.
///
/// The instant of global row `a` is found by bisecting time: `C(t)`, the
/// kept rows strictly before `t` summed over every series, is one rank per
/// series, and the smallest `t` with `C(t + 1) > a` is the instant row `a`
/// stands at. Each series is then positioned at its own rank of that instant
/// and the rows are merged forward, the earliest head first and, at one
/// instant, the earlier series first. The rows before `a` at that same
/// instant — fewer than one per series — are skipped.
///
/// An intraday rung's stamps are whole seconds (its grid starts at 09:15 IST
/// and every width is whole seconds), so the bisection walks seconds there:
/// about 22 steps across a month instead of 42 over microseconds. A day bar
/// may sit anywhere in its day, so a day rung bisects microseconds. A stamp
/// off that grid would still be merged in order — the skip below pops in the
/// merge's own order — and would only make the skip longer.
fn merged(
    root: &std::path::Path,
    plans: &mut [SeriesPlan],
    a: u64,
    b: u64,
    timeframe: Timeframe,
) -> Result<Vec<(usize, series::Position)>, String> {
    let step: i64 = if timeframe.secs() < 86_400 {
        1_000_000
    } else {
        1
    };
    let lo_bound = plans
        .iter()
        .filter_map(|plan| plan.runs.first().map(|run| run.lo))
        .min()
        .unwrap_or(0);
    let hi_bound = plans
        .iter()
        .filter_map(|plan| plan.runs.last().map(|run| run.hi))
        .max()
        .unwrap_or(0);
    let count_before = |plans: &mut [SeriesPlan], at: i64| -> Result<u64, String> {
        let mut sum = 0_u64;
        for plan in plans.iter_mut() {
            sum = sum.saturating_add(plan.rank(root, at)?.count);
        }
        Ok(sum)
    };
    // C(lo) <= a < C(hi) holds at the bounds, in units of `step`: nothing is
    // kept before the first run's start, and everything before the last
    // run's end.
    let (mut lo, mut hi) = (
        lo_bound.div_euclid(step),
        hi_bound.div_euclid(step).saturating_add(1),
    );
    while hi - lo > 1 {
        let mid = lo + (hi - lo) / 2;
        if count_before(plans, mid.saturating_mul(step))? <= a {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    let instant = lo.saturating_mul(step);
    let mut heads: Vec<Option<(i64, series::Position)>> = Vec::with_capacity(plans.len());
    let mut before = 0_u64;
    for plan in plans.iter_mut() {
        let at = plan.rank(root, instant)?;
        before = before.saturating_add(at.count);
        heads.push(head(root, plan, (at.count < plan.total()).then_some(at)));
    }
    let skip = a.saturating_sub(before);
    let want = usize::try_from(b - a).unwrap_or(0);
    let mut out = Vec::with_capacity(want);
    let mut popped = 0_u64;
    while out.len() < want {
        let mut best: Option<(i64, usize)> = None;
        for (series, held) in heads.iter().enumerate() {
            if let Some((ts, _)) = held
                && best.is_none_or(|(best_ts, _)| *ts < best_ts)
            {
                best = Some((*ts, series));
            }
        }
        let Some((_, series)) = best else {
            break;
        };
        let Some(Some((_, at))) = heads.get(series).copied() else {
            break;
        };
        if popped >= skip {
            out.push((series, at));
        }
        popped += 1;
        let next = plans.get(series).and_then(|plan| plan.after(at));
        if let (Some(plan), Some(slot)) = (plans.get_mut(series), heads.get_mut(series)) {
            *slot = head(root, plan, next);
        }
    }
    Ok(out)
}

/// A series' merge head: the instant of its row at `at`, read. A record
/// that would not read still takes its place, at its run's lower bound — its
/// fault is named when the page reads it.
fn head(
    root: &std::path::Path,
    plan: &mut SeriesPlan,
    at: Option<series::Position>,
) -> Option<(i64, series::Position)> {
    let at = at?;
    let run = plan.runs.get(at.run).copied()?;
    let ts = plan
        .read(root, run.slot, at.row)
        .map_or(run.lo, |bar| bar.ts_micros);
    Some((ts, at))
}

/// One row a non-time sort keeps while it streams: its column value, its
/// instant and where it is, and nothing else — forty bytes, so the
/// `2 × (offset + limit)` rows held at the deepest page are about 8 MB. The
/// page's rows are read again, with their predecessors, once chosen.
#[derive(Debug, Clone, Copy)]
struct Candidate {
    value: i64,
    ts: i64,
    series: u32,
    run: u32,
    row: u64,
    null: bool,
}

/// What a scan returns.
struct Scanned {
    page: Vec<WindowBar>,
    extremes: Extremes,
    rows: u64,
    /// Kept rows that would not read, each month of them one fault.
    unread: u64,
}

/// The total order a non-time page is in: the column (open interest nulls
/// last in both directions), then the instant, then the series.
fn candidate_order(desc: bool) -> impl Fn(&Candidate, &Candidate) -> std::cmp::Ordering + Copy {
    move |a: &Candidate, b: &Candidate| {
        let primary = if desc {
            b.value.cmp(&a.value)
        } else {
            a.value.cmp(&b.value)
        };
        a.null
            .cmp(&b.null)
            .then(primary)
            .then_with(|| a.ts.cmp(&b.ts))
            .then_with(|| a.series.cmp(&b.series))
    }
}

/// Streams every kept row of every series once, in time order, keeping the
/// `offset + limit` best under the order and folding the extremes.
///
/// The kept set is cut back to its depth whenever it doubles, by the same
/// partition that orders a page ([`page_of`]); after a cut its worst row is
/// the bar a later row must beat, so most rows cost one comparison and no
/// allocation. Rows are read one at a time from the file's verified block,
/// with no buffer per block. Memory is `O(offset + limit)`.
fn scan(
    root: &std::path::Path,
    plans: &mut [SeriesPlan],
    rules: &[change::Rules],
    query: &WindowQuery,
    (offset, limit): (usize, usize),
    faults: &mut FaultLog,
    cancel: &Cancel,
) -> Result<Scanned, String> {
    let order = candidate_order(query.desc);
    let depth = offset.saturating_add(limit).max(1);
    let mut kept: Vec<Candidate> = Vec::with_capacity(depth.saturating_mul(2).min(1 << 16));
    let mut bar_to_beat: Option<Candidate> = None;
    let mut extremes = Extremes::default();
    let mut rows = 0_u64;
    let mut unread = 0_u64;
    let mut since = 0_u64;
    for (series, plan) in plans.iter_mut().enumerate() {
        let Some(series_rules) = rules.get(series) else {
            continue;
        };
        plan.clear_unread();
        let volume = !series_rules.index_no_volume();
        let oi_absent = series_rules.not_a_derivative();
        let (Ok(series_at), Ok(runs)) = (u32::try_from(series), u32::try_from(plan.runs.len()))
        else {
            continue;
        };
        for run_at in 0..runs {
            let Some(run) = plan.runs.get(run_at as usize).copied() else {
                continue;
            };
            // THE CLIENT IS ASKED AFTER EVERY RUN — a month, or under a
            // time-of-day bound a day's window — and every few thousand rows
            // of one (DB-N15).
            cancel.check()?;
            // ONE FAULT A MONTH THAT WILL NOT READ, never one a row, and a
            // month that will not open is not tried row by row (see
            // `SeriesPlan::read_stretch`).
            let missed = plan.read_stretch(
                root,
                run.slot,
                (run.start, run.end),
                cancel,
                &mut since,
                |row, bar| {
                    if query.want_extremes {
                        extremes.fold(bar, series, volume);
                    }
                    let candidate = Candidate {
                        value: query.sort.of(bar),
                        ts: bar.ts_micros,
                        series: series_at,
                        run: run_at,
                        row,
                        null: matches!(query.sort, SortKey::OpenInterest)
                            && (oi_absent || bar.open_interest == OI_NULL),
                    };
                    if bar_to_beat.is_some_and(|worst| order(&candidate, &worst).is_ge()) {
                        return;
                    }
                    kept.push(candidate);
                    if kept.len() >= depth.saturating_mul(2) {
                        kept = page_of(std::mem::take(&mut kept), 0, depth, order);
                        bar_to_beat = kept.last().copied();
                    }
                },
            )?;
            rows = rows.saturating_add((run.end - run.start).saturating_sub(missed));
            unread = unread.saturating_add(missed);
        }
        let _counted_above = plan.take_unread(faults, |_| {});
    }
    let mut page = Vec::with_capacity(limit);
    for chosen in page_of(kept, offset, limit, order) {
        let series = chosen.series as usize;
        let (Some(plan), Some(series_rules)) = (plans.get_mut(series), rules.get(series)) else {
            continue;
        };
        let Some(run) = plan.runs.get(chosen.run as usize).copied() else {
            continue;
        };
        let at = series::Position {
            count: 0,
            run: chosen.run as usize,
            row: chosen.row,
        };
        // THE CHOSEN ROW AND ITS PREDECESSOR ARE READ AGAIN: two reads a row,
        // `limit` rows, in place of holding every candidate's bars.
        let behind = behind_of(root, plan, at, None, query.lookback);
        match plan.read(root, run.slot, chosen.row) {
            Ok(bar) => page.push(window_bar(series_rules, behind, bar, series)),
            Err(why) => faults.push(why),
        }
    }
    Ok(Scanned {
        page,
        extremes,
        rows,
        unread,
    })
}

/// Days covered per held month of every series (`coverage=1`), DB-G35.
fn coverage_of(
    root: &std::path::Path,
    plans: &mut [SeriesPlan],
    faults: &mut Vec<String>,
) -> Vec<MonthCoverage> {
    let mut out = Vec::new();
    for (series, plan) in plans.iter_mut().enumerate() {
        for slot in 0..plan.slots.len() {
            let Some((month, rows)) = plan.slots.get(slot).map(|at| (at.month, at.held.n_valid))
            else {
                continue;
            };
            match plan.days_covered(root, slot) {
                Ok(days_covered) => out.push(MonthCoverage {
                    series,
                    month,
                    rows,
                    days_covered,
                }),
                Err(why) => faults.push(why),
            }
        }
    }
    out
}

/// The rows `offset .. offset + limit` of `all` under `order`, in that order.
///
/// Equal to ordering all of `all` and slicing, when `order` is a total order
/// (no two rows compare equal), which is what `window`'s comparator is: its
/// timestamp tie-break is unique within a series.
///
/// # Cost
///
/// Two partitions and one ordering of the page: an `O(n)` partition at
/// `offset` (skipped when it is zero), an `O(n - offset)` partition at `limit`
/// inside what is left, then `O(limit log limit)` to order the page. An offset
/// at or past the end answers empty with no comparison at all. The ordering of
/// rows before or after the page, which an `offset + limit` partition paid
/// near the end of a window, is gone. `std`'s `select_nth_unstable_by` is the
/// partition, and the comparison count is measured, not argued, by
/// `api::bars::tests::the_page_orders_only_itself_wherever_the_offset_lands`.
/// W1-api5-4, D-1446.
fn page_of<T>(
    mut all: Vec<T>,
    offset: usize,
    limit: usize,
    mut order: impl FnMut(&T, &T) -> std::cmp::Ordering,
) -> Vec<T> {
    if limit == 0 || offset >= all.len() {
        return Vec::new();
    }
    if offset > 0 {
        all.select_nth_unstable_by(offset, &mut order);
    }
    let mut page = all.split_off(offset);
    if limit < page.len() {
        page.select_nth_unstable_by(limit - 1, &mut order);
        page.truncate(limit);
    }
    page.sort_by(&mut order);
    page
}

/// What the route tests read of this module's private parts.
#[cfg(test)]
pub(crate) mod test_access {
    /// How many lineage events and unreadable lines a ledger text holds
    /// (`<store>/imports/gdfl.lineage`, D-3331).
    pub(crate) fn lineage_ledger_counts(text: &str) -> (usize, usize) {
        super::change::for_tests::ledger_counts(text)
    }

    /// What a ledger text's re-key records say of `key` (`superseded by
    /// <key>`, `retired`, or nothing) and how many lineage events label it
    /// (D-3331; WP-12c).
    pub(crate) fn lineage_ledger_status(text: &str, key: &str) -> (String, usize) {
        let (status, events) = super::change::for_tests::ledger_status(text, key);
        let said = match status {
            Some(super::change::KeyStatus::Superseded { by, kind }) => {
                format!("superseded by {by} ({kind})")
            }
            Some(super::change::KeyStatus::Retired(why)) => format!("retired: {why}"),
            None => String::new(),
        };
        (said, events)
    }
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "a test that cannot panic cannot fail, and these lints exist to \
              keep panics out of the crate rather than out of its tests"
)]
mod tests {
    use super::*;

    /// **NO FLOAT REACHES A PRICE, INCLUDING ON THE WAY TO A SCREEN.**
    ///
    /// `CLAUDE.md` §7 fixes prices as paisa integers. A renderer that divides
    /// by 100.0 to show them walks that back at the last possible moment, where
    /// nobody looks — and `24500.75` is not exactly representable in binary
    /// floating point, so the last digit is a coin toss.
    #[test]
    fn a_price_renders_from_integers_and_keeps_every_paisa() {
        assert_eq!(rupees(2_450_075), "24,500.75");
        assert_eq!(rupees(0), "0.00", "zero is a price, not an absence");
        assert_eq!(rupees(1), "0.01", "one paisa survives");
        assert_eq!(rupees(10), "0.10", "and one tenth is TEN paise");
        assert_eq!(rupees(99), "0.99");
        assert_eq!(rupees(100), "1.00");
        assert_eq!(rupees(-2_450_075), "-24,500.75", "a fall keeps its sign");
    }

    /// Grouped the way an NSE screen groups: three, then twos.
    #[test]
    fn the_grouping_is_indian_and_not_western() {
        assert_eq!(rupees(100_000), "1,000.00");
        assert_eq!(rupees(10_000_000), "1,00,000.00", "a lakh, not 100,000");
        assert_eq!(rupees(1_000_000_000), "1,00,00,000.00", "a crore");
        assert_eq!(rupees(12_345_678), "1,23,456.78");
        assert_eq!(rupees(12_345), "123.45", "no comma below a thousand");
    }

    /// The formatter must not panic on the one value it can never divide.
    #[test]
    fn the_extreme_of_the_type_formats_rather_than_panicking() {
        // `i64::MIN.abs()` panics; `unsigned_abs` does not. This value is the
        // open-interest null and is never routed here — a formatter that
        // panics on an input it "cannot receive" is still a panic.
        let shown = rupees(i64::MIN);
        assert!(shown.starts_with('-'), "{shown}");
        assert!(!shown.is_empty());
        let _ = rupees(i64::MAX);
    }

    /// The clock is IST, from a UTC store, by integer arithmetic.
    #[test]
    fn the_clock_reads_ist_from_a_utc_timestamp() {
        // 2025-07-01 03:45:00 UTC is 09:15 IST — the session open.
        assert_eq!(ist_clock(1_751_341_500_000_000), "09:15");
        assert_eq!(ist_clock(1_751_341_560_000_000), "09:16");
        // 10:00 UTC is 15:30 IST — the close.
        assert_eq!(ist_clock(1_751_364_000_000_000), "15:30");
        // And it wraps a day rather than running past 23:59.
        assert_eq!(ist_clock(1_751_400_000_000_000), "01:30");
    }

    /// A rising bar, a falling bar and one that did not move are three
    /// different rows.
    #[test]
    fn direction_is_encoded_in_the_row_and_not_only_in_the_number() {
        let bar = |open: i64, close: i64| Bar {
            ts_micros: 1_751_341_500_000_000,
            open,
            high: close.max(open),
            low: close.min(open),
            close,
            volume: 100,
            open_interest: OI_NULL,
        };
        let html = table(&[
            bar(2_450_000, 2_451_000),
            bar(2_451_000, 2_450_000),
            bar(2_450_000, 2_450_000),
        ]);
        assert_eq!(html.matches("<tr class=\"up\">").count(), 1, "{html}");
        assert_eq!(html.matches("<tr class=\"down\">").count(), 1, "{html}");
        assert_eq!(html.matches("<tr class=\"flat\">").count(), 1, "{html}");
        assert!(html.contains(">+10.00<"), "a rise is signed: {html}");
        assert!(
            html.contains(">-10.00<"),
            "a fall carries its minus: {html}"
        );
        // The null open interest is a dash; a real zero would be `0`.
        assert_eq!(html.matches("<td class=\"num oi\">—</td>").count(), 3);
    }

    /// Zero open interest is a measurement and must not render as the null.
    #[test]
    fn a_real_zero_open_interest_is_not_a_dash() {
        let html = table(&[Bar {
            ts_micros: 1_751_341_500_000_000,
            open: 1,
            high: 1,
            low: 1,
            close: 1,
            volume: 0,
            open_interest: 0,
        }]);
        assert!(html.contains("<td class=\"num oi\">0</td>"), "{html}");
        assert!(!html.contains("oi\">—"), "zero means zero: {html}");
    }

    /// An empty month renders a table with no rows rather than nothing at all.
    #[test]
    fn no_bars_still_renders_a_table_with_its_headings() {
        let html = table(&[]);
        assert!(html.contains("<tbody></tbody>"), "{html}");
        assert!(html.contains("Open interest"), "the headings stay: {html}");
    }

    /// No page this server emits carries a script, and this one is no exception.
    /// **A `GET` NEVER CREATES A MONTH.**
    ///
    /// `open` used to call `BarFile::open_or_create`, the store's read-write
    /// door, so this handler manufactured whatever tuple arrived in the query
    /// string: six directories, a 32 KiB initialised bar file and a `.lock`
    /// beside it. It then rendered "this month is empty" about the file the
    /// request had just created. Reproduced live before the fix against a
    /// symbol that has never traded.
    ///
    /// The assertion that matters is the second one. It is not enough to check
    /// that the call refused — `open_or_create` also refuses eventually, on the
    /// symbol-id or timeframe cross-check, *after* the file is on disk. So this
    /// walks the tree and requires it to be empty, which is the only form of the
    /// claim a creating opener cannot satisfy.
    #[test]
    fn asking_for_a_month_that_was_never_written_creates_nothing() {
        fn walk(dir: &std::path::Path, into: &mut Vec<String>) {
            let Ok(entries) = std::fs::read_dir(dir) else {
                return;
            };
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    walk(&path, into);
                }
                into.push(path.display().to_string());
            }
        }

        let root = std::env::temp_dir().join(format!(
            "brutex-bars-readonly-{}-{}",
            std::process::id(),
            line!()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("a scratch root");

        let month = YearMonth::new(2025, 7).expect("a real month");
        let outcome = open(
            &root,
            Vendor::Dhan,
            "NSE",
            "INDEX",
            "NOTAREALSYMBOL",
            Timeframe::MINUTE_1,
            month,
            None,
        );

        let why = outcome.expect_err(
            "a month nobody has ever written must refuse, not be conjured into \
             existence so it can be reported empty",
        );
        assert!(
            why.contains("NOTAREALSYMBOL"),
            "the refusal must name the path it looked for, so an operator can \
             see WHICH month is absent: {why}"
        );

        let mut found = Vec::new();
        walk(&root, &mut found);
        assert!(
            found.is_empty(),
            "a read-only GET wrote {} entries to the store: {found:#?}",
            found.len()
        );

        let _ = std::fs::remove_dir_all(&root);
    }

    /// Selecting the page and then ordering it gives the SAME page as ordering
    /// everything and slicing — which is the whole licence for not sorting the
    /// universe.
    ///
    /// # Why this is a differential test and not a repeat of the implementation
    ///
    /// `window` used to sort every bar in the range and then `skip`/`take`. At a
    /// 240-month window that is roughly 1.9 million rows ordered to hand back a
    /// thousand, and `docs/07-o1-architecture.md` layer 12 asks for a bounded
    /// page and never O(universe). It now partitions with
    /// `select_nth_unstable_by`, keeps the fewer of the first `offset + limit`
    /// and the last `n - offset` rows, and sorts only those (D-0733).
    ///
    /// The risk in that trade is not speed, it is ORDER: an unstable partition
    /// may reorder anything it is free to reorder, so a page could come back
    /// with two rows swapped between otherwise identical requests, and a reader
    /// paging through would see one row twice and another never. The defence is
    /// that the comparator is TOTAL — the timestamp tie-break is unique within a
    /// series, so no two rows ever compare equal and the partition has nothing
    /// it is free to reorder.
    ///
    /// Asserting that by recomputing the new path the new way would assert
    /// nothing. This runs BOTH strategies over the same data and requires them
    /// to agree, which is a claim about the pair rather than about either one.
    /// The fixture deliberately carries HEAVY TIES on the primary key — every
    /// value appears three times — because a comparator that was not total
    /// would pass on distinct keys and fail here.
    #[test]
    fn selecting_the_page_then_ordering_it_equals_ordering_everything_then_slicing() {
        // Deterministic, and not sorted to begin with: a stride of 7 over 60
        // wraps without repeating, and `/ 3` then forces each primary value to
        // appear exactly three times.
        let rows: Vec<(i64, i64)> = (0..60_i64)
            .map(|i| {
                let scattered = (i * 7) % 60;
                (scattered / 3, scattered)
            })
            .collect();
        let order = |a: &(i64, i64), b: &(i64, i64)| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1));

        // EVERY OFFSET, including each one past the end, against the
        // unbounded page (D-0733, and since D-1446 through `page_of`).
        for offset in 0..=62 {
            for limit in [0, 1, 2, 10, 29, 30, 31, 60, 61] {
                let mut whole = rows.clone();
                whole.sort_by(order);
                let expected: Vec<(i64, i64)> =
                    whole.into_iter().skip(offset).take(limit).collect();
                let got = super::page_of(rows.clone(), offset, limit, order);
                assert_eq!(
                    got, expected,
                    "offset={offset} limit={limit}: the bounded page must be the \
                     same rows in the same order as the unbounded one"
                );
            }
        }
        // AND THE EDGES NO OFFSET ABOVE REACHES: limits and offsets at
        // `usize::MAX`, where `offset + limit` would overflow (W1-api5-4, D-1446).
        for (offset, limit) in [(usize::MAX, usize::MAX), (1, usize::MAX), (59, usize::MAX)] {
            let mut whole = rows.clone();
            whole.sort_by(order);
            let expected: Vec<(i64, i64)> = whole.into_iter().skip(offset).take(limit).collect();
            assert_eq!(
                super::page_of(rows.clone(), offset, limit, order),
                expected,
                "offset={offset} limit={limit}"
            );
        }
    }

    /// **THE SORTED WINDOW'S FULL READ IS STATED IN `docs/06-limits.md`, AND
    /// EVERY LINE IT QUOTES IS THIS FILE'S OWN.** It was stated only in comments
    /// here (W1-api1-3, D-0733). Each quoted line must appear in the limits
    /// section and in this file's code above its tests, so a line that changes
    /// here without the limit changing fails.
    #[test]
    fn the_sorted_window_read_is_stated_in_the_limits_and_quotes_this_source() {
        // D-3326 REPLACED THE WHOLE-WINDOW READ: a sorted window now streams
        // each kept row once and holds `offset + limit` of them, refused past
        // its budget. These are the lines that make that true, held to this
        // file. `docs/06-limits.md`'s D-0733 section still quotes the read it
        // replaced; WP-99 rewrites it with WP-31's measured p50/p99 (see the
        // WP-31 report), so the quotes are checked here against the code only.
        let squash = |text: &str| text.split_whitespace().collect::<Vec<_>>().join(" ");
        let source = include_str!("bars.rs");
        let code = squash(
            source
                .split_once("\n#[cfg(test)]\n")
                .expect("the tests follow the code")
                .0,
        );
        for quoted in [
            "kept = page_of(std::mem::take(&mut kept), 0, depth, order);",
            "if bar_to_beat.is_some_and(|worst| order(&candidate, &worst).is_ge()) {",
            "for chosen in page_of(kept, offset, limit, order) {",
            "page.select_nth_unstable_by(limit - 1, &mut order);",
            "page.sort_by(&mut order);",
            "pub const MAX_WINDOW_MONTHS: usize = 240;",
            "pub const MAX_WINDOW_LIMIT: usize = 1_000;",
            "pub const MAX_SCAN_BARS: u64 = ",
            "pub const MAX_SORT_DEPTH: usize = 100_000;",
        ] {
            assert!(code.contains(quoted), "bars.rs says {quoted}");
        }
        for gone in ["reserved_window(", "read_in_time(", "fn seek_page("] {
            assert!(
                !code.contains(gone),
                "the whole-window read is gone: {gone}"
            );
        }
    }

    /// **A page orders itself and nothing else, wherever its offset lands.**
    /// W1-api5-4, D-1446.
    ///
    /// `window` partitioned at `offset + limit` and ordered everything before
    /// that, so a page near the end of the window (or an offset past it, which
    /// the query string does not cap) ordered every row. Counted here with a
    /// comparator that counts: over 200,000 rows in three adversarial input
    /// orders, the page at the very end costs a linear number of comparisons,
    /// within a bound an `n log n` ordering of the whole window (some 3.5
    /// million comparisons here) cannot meet, and an offset past the end costs
    /// none.
    #[test]
    fn the_page_orders_only_itself_wherever_the_offset_lands() {
        const N: usize = 200_000;
        const N_I64: i64 = 200_000;
        const LIMIT: usize = 1_000;
        let inputs: [(&str, Vec<(i64, i64)>); 3] = [
            ("ascending", (0..N_I64).map(|i| (i, i)).collect()),
            ("descending", (0..N_I64).rev().map(|i| (i, i)).collect()),
            (
                "one primary value",
                (0..N_I64).map(|i| (7, (i * 7919) % N_I64)).collect(),
            ),
        ];
        for (name, rows) in inputs {
            for offset in [0, N / 2, N - LIMIT, N - 1, N, N + 1, usize::MAX] {
                let compared = std::cell::Cell::new(0_u64);
                let order = |a: &(i64, i64), b: &(i64, i64)| {
                    compared.set(compared.get() + 1);
                    a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1))
                };
                let got = super::page_of(rows.clone(), offset, LIMIT, order);
                let mut whole = rows.clone();
                whole.sort_unstable();
                let expected: Vec<_> = whole.into_iter().skip(offset).take(LIMIT).collect();
                assert_eq!(got, expected, "{name} offset={offset}");
                // Two linear partitions and one ordering of the page.
                let bound = 12 * N as u64 + 2 * (LIMIT as u64) * 10;
                assert!(
                    compared.get() <= bound,
                    "{name} offset={offset}: {} comparisons against a linear bound of {bound}",
                    compared.get()
                );
                if offset >= N {
                    assert_eq!(
                        compared.get(),
                        0,
                        "{name} offset={offset}: an empty page compares nothing"
                    );
                }
            }
        }
    }

    #[test]
    fn the_price_table_carries_no_script() {
        let html = table(&[Bar {
            ts_micros: 1,
            open: 1,
            high: 1,
            low: 1,
            close: 1,
            volume: 1,
            open_interest: 1,
        }]);
        for forbidden in ["<script", "javascript:", "onclick", "onload", "onerror"] {
            assert!(!html.contains(forbidden), "{forbidden} must never appear");
        }
    }
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::panic,
    reason = "a test that cannot panic cannot fail, and these lints exist to \
              keep panics out of the crate rather than out of its tests"
)]
mod ist_day_tests {
    use super::*;

    /// **EVERY ROW ON THE PAGE SAYS WHICH DAY IT IS.**
    ///
    /// `ist_clock` computes the calendar day and discards it, so a month of
    /// one-minute bars rendered 7,875 rows all reading `09:15`..`15:30` and two
    /// rows a day apart were identical. "The 11:50 bar" picked out twenty-one
    /// of them, and the row number restarts at 1 on every page, so nothing on
    /// the page identified a row at all.
    #[test]
    fn a_stamp_renders_the_ist_day_it_falls_on() {
        // 2025-07-01 09:15:00 IST — the session open of the fixture month.
        assert_eq!(ist_day(1_751_341_500_000_000), "2025-07-01");
        assert_eq!(ist_clock(1_751_341_500_000_000), "09:15");
        // …and the close of the same day stays on it.
        assert_eq!(ist_day(1_751_364_000_000_000), "2025-07-01");
        assert_eq!(ist_clock(1_751_364_000_000_000), "15:30");
    }

    /// The day and the clock agree about midnight, because both derive it from
    /// the same shifted seconds rather than each doing their own arithmetic.
    #[test]
    fn the_day_rolls_exactly_when_the_clock_does() {
        // Walk a whole IST day in one-minute steps across a midnight boundary
        // and require the day to change exactly once, at 00:00.
        let start = 1_751_341_500_000_000_i64 - 9 * 3600 * 1_000_000 - 15 * 60 * 1_000_000;
        let mut rolls = 0;
        let mut previous = ist_day(start);
        for step in 1..=1_440_i64 {
            let at = start + step * 60 * 1_000_000;
            let day = ist_day(at);
            if day != previous {
                rolls += 1;
                assert_eq!(
                    ist_clock(at),
                    "00:00",
                    "the day changed at {} — the two disagree about midnight",
                    ist_clock(at)
                );
                previous = day;
            }
        }
        assert_eq!(rolls, 1, "exactly one midnight in 1,440 minutes");
    }

    /// A stamp the store could not hold renders as an em dash rather than a
    /// plausible wrong date. `CLAUDE.md` §4 — never a fallback that hides.
    #[test]
    fn an_unrepresentable_stamp_says_so_instead_of_inventing_a_day() {
        assert_eq!(ist_day(i64::MIN), "—");
        assert_eq!(ist_day(i64::MAX), "—");
    }
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "a test that cannot panic cannot fail, and these lints exist to \
              keep panics out of the crate rather than out of its tests"
)]
mod window_tests {
    use super::*;
    use store::path::PathParts;

    const SYMBOL: &str = "WINDOWTEST";

    /// A segment no NSE venue answers to, so these tests read the fold, the
    /// seek and the scan alone. The D-3325 rules a real segment brings —
    /// the calendar's sessions, open interest that does not apply — have
    /// their own tests in `change_tests` and the route's.
    const SEGMENT: &str = "TESTSEG";

    /// The pre-D-3326 window over the one fixture series.
    #[expect(
        clippy::too_many_arguments,
        reason = "the old window's own coordinates"
    )]
    fn legacy(
        root: &std::path::Path,
        from: YearMonth,
        to: YearMonth,
        sort: SortKey,
        desc: bool,
        offset: usize,
        limit: usize,
        extremes: bool,
    ) -> Result<Window, String> {
        window(
            root,
            &WindowQuery::single(
                Vendor::Dhan,
                SeriesKey::new("NSE", SEGMENT, SYMBOL, None),
                Timeframe::MINUTE_1,
                from,
                to,
                sort,
                desc,
                offset,
                limit,
                extremes,
            ),
        )
    }

    /// **CE-61. A WINDOW TOTAL NO ALLOCATION CAN HOLD IS REFUSED BY NAME.**
    ///
    /// The reading path reserved `total` rows with `Vec::with_capacity`, so a
    /// counter no machine can hold aborted the server (`panic = "abort"`,
    /// and an allocation failure aborts in any profile). The store now refuses
    /// a counter past the month at open (D-2685); this is the second wall, for
    /// a sum over many months that is legal per month and still too large.
    #[test]
    fn a_window_total_no_allocation_can_hold_is_refused_by_name() {
        // A SORTED WINDOW NO LONGER RESERVES ITS TOTAL (D-3326): it keeps
        // `offset + limit` rows while it streams. A total past the scan
        // budget — `u64::MAX` included — is refused by name before a record
        // is read, so nothing is allocated for it and nothing aborts.
        let why = scan_admitted(u64::MAX, SortKey::Close, 0, 50).expect_err("refused, not aborted");
        assert!(why.starts_with("scan_budget_exceeded"), "{why}");
        assert!(
            why.contains(&u64::MAX.to_string()),
            "names the rows it would need: {why}"
        );
        assert!(scan_admitted(MAX_SCAN_BARS, SortKey::Close, 0, 50).is_ok());
        assert!(scan_admitted(MAX_SCAN_BARS + 1, SortKey::Volume, 0, 50).is_err());
        let deep = scan_admitted(10, SortKey::Close, MAX_SORT_DEPTH, 1).expect_err("too deep");
        assert!(deep.starts_with("sort_too_deep"), "{deep}");
        assert!(scan_admitted(10, SortKey::Close, MAX_SORT_DEPTH - 1, 1).is_ok());
    }

    /// A scratch store nobody else in this process shares, emptied first.
    fn scratch(tag: &str) -> std::path::PathBuf {
        let root = std::env::temp_dir().join(format!(
            "brutex-window-{}-{}-{tag}",
            std::process::id(),
            line!()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("a scratch root");
        root
    }

    /// Writes `n` one-minute bars into one month, closing at `base + i`.
    ///
    /// The close CARRIES THE INDEX so a sorted page can be checked against the
    /// value rather than against a position: a test that only counted rows
    /// would pass on a sort that returned the right number of the wrong ones.
    fn write_month(root: &std::path::Path, month: YearMonth, n: usize, base: i64) {
        let parts = PathParts {
            vendor: Vendor::Dhan,
            exchange: "NSE",
            segment: SEGMENT,
            symbol: SYMBOL,
            contract: None,
            timeframe: Timeframe::MINUTE_1,
            month,
            file: FileKind::Bars,
        };
        let path = StorePath::new(parts).expect("a legal path");
        #[expect(
            clippy::cast_possible_truncation,
            reason = "the id is the same cross-check `open` folds, and any 32 \
                      bits serve — a different fold would fail to reopen."
        )]
        let symbol_id = brutex_core::universe::fnv1a(SYMBOL) as u32;
        let mut file =
            store::file::BarFile::open_or_create(root, path, symbol_id).expect("a bar file");
        // ONE MINUTE APART, INSIDE THE MONTH THE PATH NAMES. The store resolves
        // a record's slot from its timestamp, so a bar stamped outside its own
        // month is not a smaller test, it is a different one.
        let start = month_start_micros(month);
        let rows: Vec<Bar> = (0..n)
            .map(|i| {
                let nth = i64::try_from(i).expect("a test month is far short of i64");
                let close = base + nth;
                Bar {
                    ts_micros: start + nth * 60_000_000,
                    open: close,
                    high: close + 10,
                    low: close - 10,
                    close,
                    volume: nth * 2,
                    open_interest: OI_NULL,
                }
            })
            .collect();
        file.append(&rows).expect("the batch appends");
    }

    /// Midnight UTC on the first of a month, in micros.
    fn month_start_micros(month: YearMonth) -> i64 {
        let (y, m) = (i64::from(month.year()), i64::from(month.month()));
        // Days since the epoch, by the civil-from-days algorithm the store uses.
        let (y2, m2) = if m <= 2 { (y - 1, m + 9) } else { (y, m - 3) };
        let era = y2.div_euclid(400);
        let yoe = y2 - era * 400;
        let doy = (153 * m2 + 2) / 5;
        let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
        let days = era * 146_097 + doe - 719_468;
        days * 86_400 * 1_000_000
    }

    #[test]
    fn a_range_that_runs_backwards_is_refused_rather_than_swapped() {
        let from = YearMonth::new(2026, 8).expect("a month");
        let to = YearMonth::new(2026, 1).expect("a month");
        assert!(
            months_of(from, to).is_none(),
            "swapping the ends would answer a range nobody asked for"
        );
    }

    #[test]
    fn a_range_longer_than_the_ceiling_is_refused_rather_than_clamped() {
        // 1970 IS THE FLOOR AND NOT AN ARBITRARY ONE: a timestamp is micros
        // since the epoch, so below it no bar can exist. `store::path` refuses
        // the year before this function ever sees it, which is why the
        // over-long case is spelled from a year the store admits.
        let from = YearMonth::new(1970, 1).expect("the earliest month a bar can have");
        let to = YearMonth::new(2026, 8).expect("a month");
        assert!(
            months_of(from, to).is_none(),
            "a silently shortened range answers a narrower question and nothing \
             on the page would say so"
        );
        // And exactly at the ceiling it is allowed.
        let edge = months_of(
            YearMonth::new(2006, 9).expect("a month"),
            YearMonth::new(2026, 8).expect("a month"),
        )
        .expect("240 months is the ceiling, not past it");
        assert_eq!(edge.len(), MAX_WINDOW_MONTHS);
    }

    #[test]
    fn the_months_of_a_range_roll_the_year_and_include_both_ends() {
        let got = months_of(
            YearMonth::new(2025, 11).expect("a month"),
            YearMonth::new(2026, 2).expect("a month"),
        )
        .expect("a forwards range");
        let spelled: Vec<String> = got
            .iter()
            .map(|m| format!("{}-{:02}", m.year(), m.month()))
            .collect();
        assert_eq!(spelled, ["2025-11", "2025-12", "2026-01", "2026-02"]);

        let one = months_of(
            YearMonth::new(2026, 3).expect("a month"),
            YearMonth::new(2026, 3).expect("a month"),
        )
        .expect("one month is a legal range");
        assert_eq!(one.len(), 1, "from == to is ONE month, never zero");
    }

    /// The seek path returns the same rows the whole-window read would, and
    /// **costs the same at the far end as at the near one**.
    #[test]
    fn a_time_ordered_page_seeks_and_straddles_a_file_boundary() {
        let root = scratch("seek");
        write_month(&root, YearMonth::new(2026, 1).expect("m"), 100, 1_000);
        write_month(&root, YearMonth::new(2026, 2).expect("m"), 100, 2_000);

        // ASCENDING: rows 95..105 straddle the January/February boundary.
        let got = legacy(
            &root,
            YearMonth::new(2026, 1).expect("m"),
            YearMonth::new(2026, 2).expect("m"),
            SortKey::Ts,
            false,
            95,
            10,
            false,
        )
        .expect("a legal window");

        assert_eq!(got.total, 200, "both months count toward the total");
        assert_eq!(got.months_read, 2);
        assert_eq!(got.months_missing, 0);
        assert_eq!(got.bars.len(), 10, "the page is the size asked for");
        assert!(got.extremes.is_none(), "not asked for, so not paid for");
        let closes: Vec<i64> = got.bars.iter().map(|r| r.bar.close).collect();
        assert_eq!(
            closes,
            [
                1_095, 1_096, 1_097, 1_098, 1_099, 2_000, 2_001, 2_002, 2_003, 2_004
            ],
            "five rows from January then five from February, contiguous across \
             the file boundary and in order"
        );

        let _ = std::fs::remove_dir_all(&root);
    }

    /// Descending is the window read backwards — **each file reversed as well
    /// as the files reversed**, which is the half a single-file test cannot
    /// catch.
    #[test]
    fn a_descending_page_reverses_within_the_file_and_across_them() {
        let root = scratch("desc");
        write_month(&root, YearMonth::new(2026, 1).expect("m"), 100, 1_000);
        write_month(&root, YearMonth::new(2026, 2).expect("m"), 100, 2_000);

        let got = legacy(
            &root,
            YearMonth::new(2026, 1).expect("m"),
            YearMonth::new(2026, 2).expect("m"),
            SortKey::Ts,
            true,
            95,
            10,
            false,
        )
        .expect("a legal window");

        let closes: Vec<i64> = got.bars.iter().map(|r| r.bar.close).collect();
        assert_eq!(
            closes,
            [
                2_004, 2_003, 2_002, 2_001, 2_000, 1_099, 1_098, 1_097, 1_096, 1_095
            ],
            "newest first is February counting down, then January counting down"
        );

        let _ = std::fs::remove_dir_all(&root);
    }

    /// A page past the end is empty rather than wrapping or refusing.
    #[test]
    fn an_offset_past_the_last_row_is_an_empty_page_and_a_true_total() {
        let root = scratch("past");
        write_month(&root, YearMonth::new(2026, 1).expect("m"), 50, 1_000);

        let got = legacy(
            &root,
            YearMonth::new(2026, 1).expect("m"),
            YearMonth::new(2026, 1).expect("m"),
            SortKey::Ts,
            false,
            10_000,
            50,
            false,
        )
        .expect("a legal window");
        assert!(got.bars.is_empty(), "no rows out there");
        assert_eq!(
            got.total, 50,
            "and the total still names every row, so a pager can walk BACK"
        );

        let _ = std::fs::remove_dir_all(&root);
    }

    /// A month the store never wrote is COUNTED, not an error and not silence.
    #[test]
    fn a_month_with_no_file_is_counted_rather_than_failing_the_window() {
        let root = scratch("sparse");
        write_month(&root, YearMonth::new(2026, 1).expect("m"), 10, 1_000);
        // 2026-02 is never written.
        write_month(&root, YearMonth::new(2026, 3).expect("m"), 10, 3_000);

        let got = legacy(
            &root,
            YearMonth::new(2026, 1).expect("m"),
            YearMonth::new(2026, 3).expect("m"),
            SortKey::Ts,
            false,
            0,
            100,
            false,
        )
        .expect("a sparse store is legal");
        assert_eq!(got.months_read, 2);
        assert_eq!(
            got.months_missing, 1,
            "the gap is REPORTED — a reader must be able to tell a hole from a \
             refusal, which is `CLAUDE.md` §4"
        );
        assert_eq!(got.total, 20);
        assert_eq!(got.bars.len(), 20);

        let _ = std::fs::remove_dir_all(&root);
    }

    /// A window over a store that holds nothing at all answers empty, with the
    /// extremes still present when asked for.
    #[test]
    fn a_window_over_nothing_is_empty_and_says_how_many_months_were_absent() {
        let root = scratch("empty");
        let got = legacy(
            &root,
            YearMonth::new(2026, 1).expect("m"),
            YearMonth::new(2026, 3).expect("m"),
            SortKey::Ts,
            false,
            0,
            50,
            true,
        )
        .expect("an empty store is not an error");
        assert_eq!(got.total, 0);
        assert_eq!(got.months_read, 0);
        assert_eq!(got.months_missing, 3);
        assert!(got.bars.is_empty());
        assert_eq!(
            got.extremes,
            Some(Extremes::default()),
            "asked for, so answered — and zero is the honest answer over no rows"
        );

        let _ = std::fs::remove_dir_all(&root);
    }

    /// **THE SCAN PATH RETURNS THE LARGEST, NOT MERELY FIFTY OF THEM.**
    #[test]
    fn a_price_ordered_page_is_the_top_of_the_whole_window() {
        let root = scratch("sort");
        write_month(&root, YearMonth::new(2026, 1).expect("m"), 100, 1_000);
        write_month(&root, YearMonth::new(2026, 2).expect("m"), 100, 5_000);

        let got = legacy(
            &root,
            YearMonth::new(2026, 1).expect("m"),
            YearMonth::new(2026, 2).expect("m"),
            SortKey::Close,
            true,
            0,
            5,
            false,
        )
        .expect("a legal window");
        let closes: Vec<i64> = got.bars.iter().map(|r| r.bar.close).collect();
        assert_eq!(
            closes,
            [5_099, 5_098, 5_097, 5_096, 5_095],
            "the five largest closes IN THE WINDOW, which live in the second \
             month — a page that returned January's five largest would be the \
             right count of the wrong rows"
        );
        assert_eq!(got.total, 200, "the total is every row, not the page");

        let _ = std::fs::remove_dir_all(&root);
    }

    /// The extremes are the widest range and heaviest volume in the WINDOW.
    #[test]
    fn the_extremes_are_folded_over_every_month_the_window_names() {
        let root = scratch("extremes");
        write_month(&root, YearMonth::new(2026, 1).expect("m"), 10, 1_000);
        write_month(&root, YearMonth::new(2026, 2).expect("m"), 40, 2_000);

        let got = legacy(
            &root,
            YearMonth::new(2026, 1).expect("m"),
            YearMonth::new(2026, 2).expect("m"),
            SortKey::Ts,
            false,
            0,
            5,
            true,
        )
        .expect("a legal window");
        let top = got.extremes.expect("asked for");
        assert_eq!(top.range, 20, "high is close+10 and low is close-10");
        assert_eq!(
            top.volume, 78,
            "the heaviest volume is the 40th bar of the SECOND month — 39*2 — \
             so this is folded over the whole window and not over the page, \
             which is only five rows long"
        );

        let _ = std::fs::remove_dir_all(&root);
    }

    /// **THE CHANGE IS AGAINST THE PREVIOUS BAR IN TIME, EVEN WHEN THE PAGE IS
    /// SORTED BY PRICE.**
    ///
    /// This is the whole reason the fold happens here. Fifty rows sorted by
    /// close have no time neighbours, so a page folded after the sort would
    /// compute each row against whichever row happened to land above it — a
    /// column of real-looking numbers, every one of them meaningless.
    #[test]
    fn the_change_is_folded_in_time_order_and_survives_a_price_sort() {
        let root = scratch("chg");
        // Closes run 1_000, 1_001, … so each bar is +1 paisa on the one before.
        write_month(&root, YearMonth::new(2026, 1).expect("m"), 20, 1_000);

        let sorted = legacy(
            &root,
            YearMonth::new(2026, 1).expect("m"),
            YearMonth::new(2026, 1).expect("m"),
            SortKey::Close,
            true,
            0,
            3,
            false,
        )
        .expect("a legal window");

        let closes: Vec<i64> = sorted.bars.iter().map(|r| r.bar.close).collect();
        assert_eq!(closes, [1_019, 1_018, 1_017], "the three largest closes");
        for row in &sorted.bars {
            let before = row.bar.close - 1;
            let want = crate::server::basis_points(before, row.bar.close).expect("a real ratio");
            assert_eq!(
                row.chg,
                Some(want),
                "every row's change is against the bar one minute BEFORE it, \
                 not against the row above it in the sorted page"
            );
            assert_eq!(row.chg_why, "");
        }

        // The very first record of the file has nothing behind it, and says so
        // rather than reporting a change of zero.
        let first = legacy(
            &root,
            YearMonth::new(2026, 1).expect("m"),
            YearMonth::new(2026, 1).expect("m"),
            SortKey::Ts,
            false,
            0,
            1,
            false,
        )
        .expect("a legal window");
        assert_eq!(first.bars[0].chg, None);
        assert_eq!(
            first.bars[0].chg_why, "no_earlier_stored_bar",
            "never 0 — zero is a real bar that closed where the last one did"
        );

        let _ = std::fs::remove_dir_all(&root);
    }

    /// **THE ROW BEFORE THE PAGE IS READ AND NOT RETURNED.**
    ///
    /// A page starting mid-file must still fill its first change cell, which
    /// needs the record behind it. Reading one back costs a seek and is what
    /// makes a deep page's first row indistinguishable from any other.
    #[test]
    fn a_page_starting_mid_file_still_fills_its_first_change_cell() {
        let root = scratch("lookback");
        write_month(&root, YearMonth::new(2026, 1).expect("m"), 50, 1_000);

        let got = legacy(
            &root,
            YearMonth::new(2026, 1).expect("m"),
            YearMonth::new(2026, 1).expect("m"),
            SortKey::Ts,
            false,
            10,
            5,
            false,
        )
        .expect("a legal window");

        assert_eq!(
            got.bars.len(),
            5,
            "the lookback row is NOT part of the page"
        );
        assert_eq!(
            got.bars[0].bar.close, 1_010,
            "the page still starts where it was asked to"
        );
        assert!(
            got.bars[0].chg.is_some(),
            "and its change is filled, because the row behind it was read: {:?}",
            got.bars[0].chg_why
        );

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn the_sort_key_reads_the_wire_and_refuses_a_column_it_has_no_index_for() {
        assert_eq!(SortKey::parse(""), Some(SortKey::Ts));
        assert_eq!(SortKey::parse("ts"), Some(SortKey::Ts));
        assert_eq!(SortKey::parse("c"), Some(SortKey::Close));
        assert_eq!(SortKey::parse("oi"), Some(SortKey::OpenInterest));
        assert_eq!(SortKey::parse("close"), None, "the wire spells it `c`");
        assert_eq!(SortKey::parse("; DROP"), None);

        assert!(!SortKey::Ts.scans(), "the store IS this index");
        for key in [
            SortKey::Open,
            SortKey::High,
            SortKey::Low,
            SortKey::Close,
            SortKey::Volume,
            SortKey::OpenInterest,
        ] {
            assert!(key.scans(), "{key:?} has no index and must say so");
        }
    }

    /// A limit past the ceiling is CLAMPED, and that is deliberate: unlike a
    /// range, a shorter page is still an answer to the question asked — the
    /// pager simply asks again — whereas a shorter RANGE silently answers a
    /// different question.
    #[test]
    fn a_limit_past_the_ceiling_is_clamped_to_it() {
        let root = scratch("limit");
        write_month(&root, YearMonth::new(2026, 1).expect("m"), 100, 1_000);
        let got = legacy(
            &root,
            YearMonth::new(2026, 1).expect("m"),
            YearMonth::new(2026, 1).expect("m"),
            SortKey::Ts,
            false,
            0,
            usize::MAX,
            false,
        )
        .expect("a legal window");
        assert!(
            got.bars.len() <= MAX_WINDOW_LIMIT,
            "one request cannot be asked for an unbounded page"
        );
        assert_eq!(got.bars.len(), 100, "and it still returns what exists");

        let _ = std::fs::remove_dir_all(&root);
    }

    /// Flips one byte inside record `index` of the test month, so the block
    /// checksum that covers it refuses every record in that 73-record block.
    fn damage_record(root: &std::path::Path, month: YearMonth, index: u64) {
        let file = open_classified(
            root,
            PathParts {
                vendor: Vendor::Dhan,
                exchange: "NSE",
                segment: SEGMENT,
                symbol: SYMBOL,
                contract: None,
                timeframe: Timeframe::MINUTE_1,
                month,
                file: FileKind::Bars,
            },
        )
        .map_err(|why| why.message)
        .expect("the month opens");
        let at = file.layout().offset_of(index).expect("a committed record");
        let path = file.path().to_path_buf();
        drop(file);
        let mut bytes = std::fs::read(&path).expect("the bar file reads");
        let at = usize::try_from(at).expect("a test file offset fits usize");
        bytes[at + 8] ^= 0xff;
        std::fs::write(&path, bytes).expect("the damaged file writes");
    }

    fn ts_page(
        root: &std::path::Path,
        to: YearMonth,
        desc: bool,
        offset: usize,
        limit: usize,
    ) -> Window {
        legacy(
            root,
            YearMonth::new(2026, 1).expect("m"),
            to,
            SortKey::Ts,
            desc,
            offset,
            limit,
            false,
        )
        .expect("a legal window")
    }

    fn window_over(
        root: &std::path::Path,
        to: YearMonth,
        sort: SortKey,
        desc: bool,
        offset: usize,
        limit: usize,
        extremes: bool,
    ) -> Window {
        legacy(
            root,
            YearMonth::new(2026, 1).expect("m"),
            to,
            sort,
            desc,
            offset,
            limit,
            extremes,
        )
        .expect("a legal window")
    }

    /// **A ROW THAT OPENS A MONTH IS MEASURED AGAINST THE MONTH BEFORE IT.**
    ///
    /// `LOOKBACK`'s doc said only the window's very first record has nothing
    /// behind it; the code gave `first_bar_in_file` to the first bar of EVERY
    /// month, mid-window, on the seek path and the reading path alike
    /// (Z1-slice11-F4, D-1762). Both directions, both paths, and a missing
    /// month in between, which must not reset the lookback.
    #[test]
    fn a_row_opening_a_month_is_measured_against_the_last_bar_of_the_month_before() {
        let root = scratch("cross-month");
        write_month(&root, YearMonth::new(2026, 1).expect("m"), 10, 1_000);
        write_month(&root, YearMonth::new(2026, 2).expect("m"), 10, 2_000);
        // March has no file at all; April's first row stands behind February.
        write_month(&root, YearMonth::new(2026, 4).expect("m"), 10, 4_000);
        let april = YearMonth::new(2026, 4).expect("m");
        let jan_to_feb = crate::server::basis_points(1_009, 2_000).expect("a change");
        let feb_to_apr = crate::server::basis_points(2_009, 4_000).expect("a change");
        let change_at = |bars: &[WindowBar], close: i64| {
            let row = bars
                .iter()
                .filter(|row| row.bar.close == close)
                .collect::<Vec<_>>();
            assert_eq!(row.len(), 1, "close {close} is on the page once");
            (row[0].chg, row[0].chg_why)
        };

        // SEEK, ascending: 1008, 1009, 2000, 2001.
        let page = window_over(&root, april, SortKey::Ts, false, 8, 4, false);
        assert_eq!(page.bars.len(), 4);
        assert_eq!(change_at(&page.bars, 2_000), (Some(jan_to_feb), ""));
        assert_eq!(change_at(&page.bars, 2_001).0, Some(5));
        // SEEK, ascending, a page that OPENS on April: offset 20 is April's first.
        let page = window_over(&root, april, SortKey::Ts, false, 20, 2, false);
        assert_eq!(change_at(&page.bars, 4_000), (Some(feb_to_apr), ""));
        // SEEK, descending: April 4009..4000 are offsets 0..9, then Feb.
        let page = window_over(&root, april, SortKey::Ts, true, 8, 4, false);
        assert_eq!(
            page.bars
                .iter()
                .map(|row| row.bar.close)
                .collect::<Vec<_>>(),
            vec![4_001, 4_000, 2_009, 2_008]
        );
        assert_eq!(change_at(&page.bars, 4_000), (Some(feb_to_apr), ""));
        let page = window_over(&root, april, SortKey::Ts, true, 18, 4, false);
        assert_eq!(change_at(&page.bars, 2_000), (Some(jan_to_feb), ""));

        // READING path, by close, both directions, and the time-ordered
        // extremes read in both directions.
        for (sort, extremes) in [(SortKey::Close, false), (SortKey::Ts, true)] {
            for desc in [false, true] {
                let page = window_over(&root, april, sort, desc, 0, 30, extremes);
                assert_eq!(page.bars.len(), 30);
                assert_eq!(change_at(&page.bars, 2_000), (Some(jan_to_feb), ""));
                assert_eq!(change_at(&page.bars, 4_000), (Some(feb_to_apr), ""));
                // The window's earliest record is the one row with nothing behind it.
                assert_eq!(
                    change_at(&page.bars, 1_000),
                    (None, "no_earlier_stored_bar")
                );
                assert_eq!(
                    page.bars
                        .iter()
                        .filter(|row| row.chg_why == "no_earlier_stored_bar")
                        .count(),
                    1
                );
            }
        }
        // And the seek path agrees that January's first row has none.
        let page = window_over(&root, april, SortKey::Ts, false, 0, 1, false);
        assert_eq!(
            change_at(&page.bars, 1_000),
            (None, "no_earlier_stored_bar")
        );

        let _ = std::fs::remove_dir_all(&root);
    }

    /// An unreadable LAST record of the month before is a named gap for the
    /// next month's first row, not "first in file" and not a number measured
    /// across the gap (Z1-slice11-F4).
    #[test]
    fn an_unreadable_last_record_of_the_month_before_is_named_not_measured_across() {
        let root = scratch("cross-month-damaged");
        let jan = YearMonth::new(2026, 1).expect("m");
        let feb = YearMonth::new(2026, 2).expect("m");
        write_month(&root, jan, 200, 1_000);
        write_month(&root, feb, 10, 2_000);
        // Records 146..200 are January's last checksum block.
        damage_record(&root, jan, 199);
        let seek = window_over(&root, feb, SortKey::Ts, false, 200, 1, false);
        assert_eq!(seek.bars[0].bar.close, 2_000);
        assert_eq!(seek.bars[0].chg, None);
        assert_eq!(seek.bars[0].chg_why, "previous_unreadable");
        let scan = window_over(&root, feb, SortKey::Close, true, 9, 1, false);
        assert_eq!(scan.bars[0].bar.close, 2_000);
        assert_eq!(scan.bars[0].chg_why, "previous_unreadable");
        let _ = std::fs::remove_dir_all(&root);
    }

    /// `records unreadable` is emitted once per REQUEST, never per month file:
    /// `slots` runs once per file of a window and must not emit
    /// (Z1-slice11-F3, D-1762). Shape test, because the sink is process-global.
    #[test]
    fn unreadable_records_are_reported_once_per_request_not_once_per_file() {
        let source = include_str!("bars.rs");
        let code = source
            .split_once("\n#[cfg(test)]\n")
            .expect("the tests follow the code")
            .0;
        let slots = code
            .split_once("fn slots(")
            .expect("slots exists")
            .1
            .split_once("\n}\n")
            .expect("slots ends")
            .0;
        assert!(!slots.contains("note_unreadable_records"));
        // `page`, `page_cancellable` (D-3349) and `window` once after its
        // paths: three call sites, plus the definition (D-3326).
        assert_eq!(code.matches("note_unreadable_records(").count(), 4);
        let window = code
            .split_once("pub fn window_cancellable(")
            .expect("window exists")
            .1
            .split_once("\n}\n")
            .expect("window ends")
            .0;
        assert_eq!(window.matches("note_unreadable_records(").count(), 1);
        for path in ["fn seek(", "fn merged(", "fn scan(", "fn behind_of("] {
            let body = code
                .split_once(path)
                .expect("the path exists")
                .1
                .split_once("\n}\n")
                .expect("it ends")
                .0;
            assert!(
                !body.contains("note_unreadable_records") && !body.contains("telemetry::emit"),
                "{path} emits nothing inside its loops"
            );
        }
        let series = include_str!("bars_series.rs");
        let change = include_str!("bars_change.rs");
        // THE SORT INDEX'S READER EMITS NOTHING EITHER (D-3349): what it
        // could not serve is on the wire, named, once per request.
        let sorted = include_str!("bars_sort.rs");
        for (name, text) in [
            ("bars_series.rs", series),
            ("bars_change.rs", change),
            ("bars_sort.rs", sorted),
        ] {
            assert!(
                !text.contains(concat!("telemetry", "::emit")),
                "{name} emits nothing"
            );
        }
    }

    /// Null open interest sorts LAST in both directions, as `SortKey`
    /// documents, and real values, zero included, keep their order ahead of it
    /// (Z1-slice11-F5, D-1762).
    #[test]
    fn null_open_interest_sorts_last_in_both_directions() {
        let root = scratch("oi-nulls");
        // `write_month` stamps no open interest, so every January row is null.
        write_month(&root, YearMonth::new(2026, 1).expect("m"), 3, 1_000);
        let parts = PathParts {
            vendor: Vendor::Dhan,
            exchange: "NSE",
            segment: SEGMENT,
            symbol: SYMBOL,
            contract: None,
            timeframe: Timeframe::MINUTE_1,
            month: YearMonth::new(2026, 2).expect("m"),
            file: FileKind::Bars,
        };
        #[expect(
            clippy::cast_possible_truncation,
            reason = "the same 32-bit id `write_month` folds"
        )]
        let symbol_id = brutex_core::universe::fnv1a(SYMBOL) as u32;
        let mut file = store::file::BarFile::open_or_create(
            &root,
            StorePath::new(parts).expect("a legal path"),
            symbol_id,
        )
        .expect("a bar file");
        let start = month_start_micros(YearMonth::new(2026, 2).expect("m"));
        let rows: Vec<Bar> = [7, 0, 3]
            .into_iter()
            .zip(0i64..)
            .map(|(oi, nth)| Bar {
                ts_micros: start + nth * 60_000_000,
                open: 2_000,
                high: 2_010,
                low: 1_990,
                close: 2_000,
                volume: 1,
                open_interest: oi,
            })
            .collect();
        file.append(&rows).expect("the batch appends");
        drop(file);
        let feb = YearMonth::new(2026, 2).expect("m");
        let interest = |desc| {
            window_over(&root, feb, SortKey::OpenInterest, desc, 0, 6, false)
                .bars
                .iter()
                .map(|row| row.bar.open_interest)
                .collect::<Vec<_>>()
        };
        assert_eq!(interest(false), vec![0, 3, 7, OI_NULL, OI_NULL, OI_NULL]);
        assert_eq!(interest(true), vec![7, 3, 0, OI_NULL, OI_NULL, OI_NULL]);
        // The nulls keep the timestamp tie-break among themselves.
        let nulls = window_over(&root, feb, SortKey::OpenInterest, false, 3, 3, false);
        assert_eq!(
            nulls
                .bars
                .iter()
                .map(|row| row.bar.close)
                .collect::<Vec<_>>(),
            vec![1_000, 1_001, 1_002]
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    /// **AN UNREADABLE LOOKBACK RECORD MUST NOT EAT THE PAGE'S FIRST ROW.**
    ///
    /// The lookback used to be read as part of the page block and then dropped
    /// by POSITION, but `page` returns only the records that read. With the
    /// lookback unreadable, the dropped row was the page's own first row, and
    /// no fault named it (W1-api1-10, D-0730).
    #[test]
    fn an_unreadable_lookback_record_does_not_eat_the_first_row_of_the_page() {
        let root = scratch("lookback-damaged");
        let jan = YearMonth::new(2026, 1).expect("m");
        write_month(&root, jan, 200, 1_000);
        // Block 1 holds records 73..146; record 145 is the lookback of offset 146.
        damage_record(&root, jan, 100);

        let got = ts_page(&root, jan, false, 146, 5);
        let closes: Vec<i64> = got.bars.iter().map(|r| r.bar.close).collect();
        assert_eq!(
            closes,
            [1_146, 1_147, 1_148, 1_149, 1_150],
            "the page starts at the offset asked for: {:?}",
            got.faults
        );
        assert_eq!(got.bars[0].chg, None, "its predecessor did not read");
        assert_eq!(got.bars[0].chg_why, "previous_unreadable");
        assert_eq!(
            got.bars[0].oichg_why, "oi_null",
            "a null OI is still named first"
        );
        assert!(
            got.bars[1].chg.is_some(),
            "the second row has a read predecessor"
        );
        assert!(
            got.faults.is_empty(),
            "no record on this page failed, so none is named: {:?}",
            got.faults
        );

        // A readable lookback still fills the first cell.
        let ok = ts_page(&root, jan, false, 147, 2);
        assert_eq!(ok.bars[0].bar.close, 1_147);
        assert!(ok.bars[0].chg.is_some());

        let _ = std::fs::remove_dir_all(&root);
    }

    /// **A DAMAGED RECORD USES UP ITS PAGE POSITION.** A page that met an
    /// unreadable record used to fill the gap from the next file, and the next
    /// offset page returned those same rows again. Walking every page, both
    /// directions, must return each readable row exactly once and name each
    /// unreadable one exactly once (W1-api1-10, D-0730).
    #[test]
    fn paging_across_a_damaged_block_returns_every_readable_row_exactly_once() {
        let root = scratch("paging-damaged");
        let jan = YearMonth::new(2026, 1).expect("m");
        let feb = YearMonth::new(2026, 2).expect("m");
        write_month(&root, jan, 100, 1_000);
        write_month(&root, feb, 100, 2_000);
        // Records 73..100 of January are one checksum block.
        damage_record(&root, jan, 80);

        let first = ts_page(&root, feb, false, 70, 10);
        let closes: Vec<i64> = first.bars.iter().map(|r| r.bar.close).collect();
        assert_eq!(
            closes,
            [1_070, 1_071, 1_072],
            "no row is borrowed from February"
        );
        assert_eq!(
            (first.faults.len(), first.faulted_records),
            (1, 7),
            "records 73..80 are one named run of seven (DB-B75): {:?}",
            first.faults
        );
        assert!(
            first.faults[0].contains("records 73–79 (7)"),
            "{:?}",
            first.faults
        );

        for desc in [false, true] {
            let mut seen: Vec<i64> = Vec::new();
            let mut faults = 0usize;
            for offset in (0..200).step_by(10) {
                let got = ts_page(&root, feb, desc, offset, 10);
                assert!(got.bars.len() <= 10);
                seen.extend(got.bars.iter().map(|r| r.bar.close));
                faults += usize::try_from(got.faulted_records).expect("small");
            }
            let mut expected: Vec<i64> = (1_000..1_073).chain(2_000..2_100).collect();
            if desc {
                expected.reverse();
            }
            assert_eq!(
                seen, expected,
                "desc={desc}: each readable row once, in order"
            );
            assert_eq!(
                faults, 27,
                "desc={desc}: each unreadable record counted once"
            );
        }

        let _ = std::fs::remove_dir_all(&root);
    }

    /// **A SORTED PAGE DOES NOT MEASURE A ROW ACROSS AN UNREADABLE BLOCK.**
    ///
    /// The reading path folded the change over the records that read, so the
    /// first row after a damaged block was measured against the last row BEFORE
    /// it — a real-looking number against a bar that is not its predecessor
    /// (W1-api1-10, D-0730).
    ///
    /// Two paths, one rule. WITH a sort index (D-3349) the order was written
    /// when the bars were, so a damaged record keeps its place: the page that
    /// covers it names it in `faults` and the next page does not shift — the
    /// rule the time-ordered page keeps (D-0730). WITHOUT one, the rows are
    /// read and only those that read are ordered, as before.
    #[test]
    fn a_sorted_page_names_the_row_after_a_damaged_block_rather_than_measuring_across_it() {
        let root = scratch("scan-damaged");
        let jan = YearMonth::new(2026, 1).expect("m");
        write_month(&root, jan, 200, 1_000);
        // Records 73..146 are one checksum block; 0..73 and 146..200 read.
        damage_record(&root, jan, 100);

        // INDEXED: rows 73 and 74 hold their places, unreadable and named.
        let got =
            legacy(&root, jan, jan, SortKey::Close, false, 73, 2, false).expect("a legal window");
        assert!(got.bars.is_empty(), "{:?}", got.bars);
        // BOTH NAMED AND BOTH COUNTED — as ONE run since the merge with
        // DB-B75: consecutive records refused for one reason are one sentence
        // naming the file once (this asserted two sentences, one a record).
        assert_eq!(got.faulted_records, 2, "{:?}", got.faults);
        assert_eq!(got.faults.len(), 1, "{:?}", got.faults);
        assert!(
            got.faults[0].contains("2026-01.bin records 73–74 (2): "),
            "{:?}",
            got.faults
        );
        let indexed = got.sort_index.expect("a value page reports its index");
        assert_eq!((indexed.indexed_rows, indexed.scanned_rows), (200, 0));
        let got =
            legacy(&root, jan, jan, SortKey::Close, false, 146, 2, false).expect("a legal window");
        let closes: Vec<i64> = got.bars.iter().map(|r| r.bar.close).collect();
        assert_eq!(closes, [1_146, 1_147], "the 147th smallest close");
        assert_eq!(
            got.bars[0].chg, None,
            "its predecessor, record 145, did not read"
        );
        assert_eq!(got.bars[0].chg_why, "previous_unreadable");
        assert!(
            got.bars[1].chg.is_some(),
            "record 146 read, so 147 is measured"
        );

        // READ: no index, so the rows that read are the order.
        let srt = open_classified(
            &root,
            PathParts {
                vendor: Vendor::Dhan,
                exchange: "NSE",
                segment: SEGMENT,
                symbol: SYMBOL,
                contract: None,
                timeframe: Timeframe::MINUTE_1,
                month: jan,
                file: FileKind::Bars,
            },
        )
        .map_err(|why| why.message)
        .expect("the month opens")
        .path()
        .with_extension("srt");
        std::fs::remove_file(&srt).expect("the index is removed");
        let got =
            legacy(&root, jan, jan, SortKey::Close, false, 73, 2, false).expect("a legal window");
        let closes: Vec<i64> = got.bars.iter().map(|r| r.bar.close).collect();
        assert_eq!(closes, [1_146, 1_147], "the 74th smallest readable close");
        assert_eq!(got.bars[0].chg_why, "previous_unreadable");
        assert!(got.bars[1].chg.is_some());
        // ONE FAULT FOR THE MONTH, WITH THE COUNT (WP-39 repair 1). This
        // asserted 73 faults, one string a record that would not read: the
        // same shape answered 59 MB of faults for a one-second month another
        // process held. The 73 records are still all accounted for — the
        // first by name, the rest by number — and the report says they were
        // NOT read.
        assert_eq!(got.faults.len(), 1, "{:?}", got.faults);
        assert_eq!(
            got.faulted_records, 73,
            "and the count of every refused record is whole (DB-B75)"
        );
        assert!(
            got.faults[0].contains(" record 73: ")
                && got.faults[0].contains("73 kept rows of 2026-01 in this window would not read"),
            "every unreadable record is counted, the first named: {:?}",
            got.faults
        );
        let read = got.sort_index.expect("a value page reports what it read");
        assert_eq!(
            (read.indexed_rows, read.scanned_rows, read.unread_rows),
            (0, 127, 73)
        );
        assert_eq!(read.unread_months, 1);
        assert!(
            read.scanned_why[0].contains("no .srt sort index"),
            "{:?}",
            read.scanned_why
        );
        assert_eq!(
            read.build,
            ["cli srt-build dhan NSE/TESTSEG/WINDOWTEST 1min 2026-01 2026-01"]
        );

        let _ = std::fs::remove_dir_all(&root);
    }

    /// The open-interest column says the same thing for the same gap, and a
    /// predecessor that read is still measured.
    #[test]
    fn the_fold_names_an_unreadable_predecessor_in_both_change_columns() {
        let bar = |close: i64, open_interest: i64| Bar {
            ts_micros: 0,
            open: close,
            high: close,
            low: close,
            close,
            volume: 0,
            open_interest,
        };
        let root = std::path::Path::new("/nonexistent-root");
        let rules = change::Rules::of(
            root,
            Timeframe::MINUTE_1,
            &SeriesKey::new("NSE", SEGMENT, SYMBOL, None),
            &change::Lineage::read(root, Vendor::Dhan),
        );
        let row = |behind: Behind, now: Bar| window_bar(&rules, behind, now, 0);
        let gap = row(Behind::Unreadable, bar(100, 50));
        assert_eq!((gap.chg, gap.chg_why), (None, "previous_unreadable"));
        assert_eq!((gap.oichg, gap.oichg_why), (None, "previous_unreadable"));
        let measured = row(Behind::Bar(bar(100, 50)), bar(110, 55));
        assert_eq!(measured.chg, Some(1_000), "110 against 100 is +10%");
        assert_eq!(measured.oichg, Some(1_000), "55 against 50 is +10%");

        // THE FIRST-BAR CODE SPLITS (D-3325 item 10): no earlier stored bar
        // at all, or one the request chose not to read.
        let first = row(Behind::Nothing, bar(100, 50));
        assert_eq!(first.chg_why, "no_earlier_stored_bar");
        assert_eq!(first.oichg_why, "no_earlier_stored_bar");
        for code in ["earlier_month_not_read", "first_bar_in_window"] {
            let skipped = row(Behind::NotRead(code), bar(100, 50));
            assert_eq!((skipped.chg, skipped.chg_why), (None, code));
            assert_eq!((skipped.oichg, skipped.oichg_why), (None, code));
        }
        // A NULL OPEN INTEREST IS STILL NAMED FIRST with no predecessor.
        let null = row(Behind::Nothing, bar(100, OI_NULL));
        assert_eq!(null.oichg_why, "oi_null");
        assert_eq!(null.chg_why, "no_earlier_stored_bar");
        // AND THE ARITHMETIC REFUSALS ARE UNCHANGED.
        let zero = row(Behind::Bar(bar(0, 0)), bar(10, 10));
        assert_eq!(
            (zero.chg_why, zero.oichg_why),
            ("previous_close_zero", "previous_oi_zero")
        );
        let before_null = row(Behind::Bar(bar(10, OI_NULL)), bar(10, 10));
        assert_eq!(before_null.oichg_why, "oi_null_before");
        // A FALL TO EXACTLY ZERO IS WITHHELD BY NAME, NOT -100% (DB-N10), and
        // the bar after it is the existing zero-base refusal; a zero after a
        // zero, and a fall to a non-zero value, are measured as before.
        let to_zero = row(Behind::Bar(bar(10, 8_025)), bar(10, 0));
        assert_eq!((to_zero.oichg, to_zero.oichg_why), (None, "oi_to_zero"));
        assert_eq!(to_zero.chg, Some(0), "the price change is unaffected");
        let from_zero = row(Behind::Bar(bar(10, 0)), bar(10, 8_325));
        assert_eq!(
            (from_zero.oichg, from_zero.oichg_why),
            (None, "previous_oi_zero")
        );
        let zero_zero = row(Behind::Bar(bar(10, 0)), bar(10, 0));
        assert_eq!(zero_zero.oichg_why, "previous_oi_zero");
        let fall = row(Behind::Bar(bar(10, 8_000)), bar(10, 4_000));
        assert_eq!(
            fall.oichg,
            Some(-5_000),
            "a fall that stops above zero is measured"
        );
        let wide = row(Behind::Bar(bar(1, 1)), bar(i64::MAX, i64::MAX));
        assert_eq!((wide.chg_why, wide.oichg_why), ("overflow", "overflow"));
        assert!(
            first.phase.is_none(),
            "a segment with no venue has no phase"
        );
        assert!(first.oi_applies && first.v_why.is_none());
    }

    /// **THE `records unreadable` LINE COUNTS THE ROWS THAT READ, NOT THE
    /// POSITIONS WALKED.** `slots` keeps a `None` for every unreadable record,
    /// so the count it hands `note_unreadable_records` is kept apart from the
    /// `Vec`'s length. A page over 200 records with one 73-record block damaged
    /// must say 127 rows and 73 faults (W1-api1-10, D-0730).
    #[test]
    fn the_unreadable_records_line_counts_the_rows_that_read() {
        let root = scratch("telemetry-rows");
        let jan = YearMonth::new(2026, 1).expect("m");
        write_month(&root, jan, 200, 1_000);
        damage_record(&root, jan, 100);
        let file = open_classified(
            &root,
            PathParts {
                vendor: Vendor::Dhan,
                exchange: "NSE",
                segment: SEGMENT,
                symbol: SYMBOL,
                contract: None,
                timeframe: Timeframe::MINUTE_1,
                month: jan,
                file: FileKind::Bars,
            },
        )
        .map_err(|why| why.message)
        .expect("the month opens");

        let from = crate::emitted::mark();
        let (rows, faults) = page(&file, 0, 200);
        assert_eq!((rows.len(), faults.records()), (127, 73));
        let said = faults.sentences();
        assert_eq!(said.len(), 1, "one damaged block is one run: {said:?}");
        assert!(said[0].contains("records 73–145 (73)"), "{said:?}");

        // FILTERED ON THIS FIXTURE'S OWN DIRECTORY: a sibling test damages the
        // same block of a month the same size, in parallel.
        let mine: Vec<telemetry::Record> =
            crate::emitted::landed(from, "api.bars", "records unreadable")
                .into_iter()
                .filter(|record| crate::emitted::says(record, "file", "-telemetry-rows"))
                .collect();
        assert_eq!(mine.len(), 1, "one line for the page: {mine:?}");
        assert!(
            crate::emitted::counts(&mine[0], "rows", 127),
            "the rows that read: {mine:?}"
        );
        assert!(
            crate::emitted::counts(&mine[0], "faults", 73),
            "the records that did not: {mine:?}"
        );

        drop(file);
        let _ = std::fs::remove_dir_all(&root);
    }
}
