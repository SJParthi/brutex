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
//! it is. The PAGE route scans nothing and holds no month in memory: a request
//! for page 5 touches the 200 records of page 5 and no others. The WINDOW
//! route ([`window`]) is the exception in this module: sorted by a price
//! column, or asked for extremes, it reads every bar of every month in its
//! range and holds them, up to `MAX_WINDOW_MONTHS` months —
//! `docs/06-limits.md`, D-0733. Its `ts` order does not (D-3304).
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
use store::format::{Bar, OI_NULL};
use store::path::{FileKind, PathParts, StorePath, Timeframe, YearMonth};

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
    let secs = ts_micros.div_euclid(1_000_000) + pull::session::IST_OFFSET_SECS;
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
    let secs = ts_micros.div_euclid(1_000_000) + pull::session::IST_OFFSET_SECS;
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

enum OpenRefusalKind {
    Invalid,
    Absent,
    Unreadable,
}

struct OpenRefusal {
    kind: OpenRefusalKind,
    message: String,
}

/// Preserve absence separately from invalid addresses and unreadable authority.
fn open_classified(
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
/// **What IS bounded here, structurally:** this function has three call
/// sites — [`page`], [`window`]'s seek branch, and `read_in_time`, which only
/// [`window`] calls, once — each reached once per request and outside every
/// loop over files or records, and it returns on the first line
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
fn note_unreadable_records(file: &BarFile, faults: &[String], rows: usize, skip: usize) {
    let Some(first) = faults.first() else {
        return;
    };
    let _dropped_when_filtered = telemetry::emit(
        &telemetry::Event::warn("api.bars", "records unreadable")
            .with(
                "file",
                telemetry::Value::Str(&file.path().display().to_string()),
            )
            .with("faults", telemetry::Value::Uint(faults.len() as u64))
            .with("rows", telemetry::Value::Uint(rows as u64))
            .with("skip", telemetry::Value::Uint(skip as u64))
            .with("n_valid", telemetry::Value::Uint(file.header().n_valid))
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
#[must_use]
pub fn page(file: &BarFile, skip: usize, take: usize) -> (Vec<Bar>, Vec<String>) {
    let (read, faults) = slots(file, skip, take);
    let rows: Vec<Bar> = read.into_iter().flatten().collect();
    note_unreadable_records(file, &faults, rows.len(), skip);
    (rows, faults)
}

/// [`page`], with every POSITION kept: `None` where the record would not read.
///
/// The window needs the gap and not only its name. A row after an unreadable
/// record has no readable predecessor, and a page that met an unreadable
/// record has still used that position up. [`page`] drops the `None`s, and
/// with them both facts; W1-api1-10 found the window paying for that (D-0730).
fn slots(file: &BarFile, skip: usize, take: usize) -> (Vec<Option<Bar>>, Vec<String>) {
    let n = file.header().n_valid;
    let mut rows = Vec::with_capacity(take.min(PAGE_BARS));
    let mut faults = Vec::new();
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
            Err(why) => {
                rows.push(None);
                faults.push(format!("record {index}: {why}"));
            }
        }
    }
    (rows, faults)
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

/// The most stored records a window ordered by anything but `ts` may read.
///
/// Such an order is a question no index answers, so every record of the
/// window is read, change-folded and held, and [`MAX_WINDOW_MONTHS`] alone
/// let that reach about 1.9 million one-minute bars in one request
/// (W1-api5-4). 2^20 is about ten years of one-minute bars for one series, or
/// every daily bar the range cap allows many times over. A window past it is
/// refused by name, before any record is read, rather than cut. The cost at
/// the ceiling is measured in `docs/06-limits.md`'s D-4439 row. D-4439.
pub const MAX_SCAN_WINDOW_RECORDS: u64 = 1 << 20;

/// How many months' extremes [`month_fold`] keeps before it starts over.
///
/// One entry per stored instrument-month ever asked for with `extremes=1`;
/// a full map is dropped whole rather than grown, so memory stays bounded by
/// this and a working set past it pays the cold read it paid before D-4439.
pub const MONTH_FOLDS_KEPT: usize = 4_096;

/// Which column a window is ordered by.
///
/// # Why `Ts` is not just another variant
///
/// The store is indexed by TIME — `docs/02-store-format.md`, the path is the
/// index — so a window ordered by `Ts` is answered by SEEKING: the prefix sum
/// over `n_valid` says which file holds row N and `page` reads it by index.
/// Nothing scans, and page 12,470 costs what page 1 costs.
///
/// Every other variant asks a question no index answers. "The fifty largest
/// closes" cannot be known without reading the closes, and `CLAUDE.md` §4 bans
/// a query planner precisely because there is no second path to choose. So
/// those variants scan, once, here — in Rust over local files — instead of
/// moving four million rows to a browser to be sorted there.
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
    /// invert it with the direction.
    OpenInterest,
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
            _ => None,
        }
    }

    /// Whether answering this order requires reading every row.
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
        }
    }
}

/// The widest move and the heaviest volume in a window.
///
/// # Why this is computed here and not in the browser
///
/// The grid scales its magnitude bars against the widest range in the QUERY, so
/// that turning a page cannot change what a full bar means. Computing that in
/// the browser meant fetching every row of every month first, which is the
/// 2,187-request storm the window endpoint exists to end. One scan here, in
/// Rust, over files already on this disk, answers it in one number.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Extremes {
    /// The widest `high - low` in the window, in paisa.
    pub range: i64,
    /// The heaviest `volume`. Zero is a real zero — a spot index has none.
    pub volume: i64,
}

/// A bar, with the change the grid draws beside it.
///
/// # Why the change travels with the bar instead of being derived on arrival
///
/// The grid's CHANGE % is against the PREVIOUS BAR IN TIME, and it is computed
/// per file: the first row of a month has no predecessor in that file and says
/// so. That works in the browser only because the browser holds whole months.
///
/// A price-ordered page does not: fifty rows sorted by close have no time
/// neighbours at all, so a page fetched already-sorted cannot compute the column
/// on arrival. The number is therefore folded HERE, over the records in the
/// order they were written, BEFORE anything is sorted — which is exactly what
/// the browser does today, moved to where the time order still exists.
///
/// `why` carries the grid's own vocabulary rather than a new one, so a cell that
/// cannot be computed says the same thing it said before.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WindowBar {
    /// The record as stored.
    pub bar: Bar,
    /// Basis points against the previous bar in the same month, when there is
    /// one and the arithmetic holds.
    pub chg: Option<i64>,
    /// Empty when `chg` is `Some`; otherwise the reason, in the grid's words.
    pub chg_why: &'static str,
    /// The same, for open interest.
    pub oichg: Option<i64>,
    /// Empty when `oichg` is `Some`; otherwise the reason.
    pub oichg_why: &'static str,
}

/// What stands behind a row in time, for its change columns.
#[derive(Debug, Clone, Copy)]
enum Behind {
    /// The first record of the file: there is no predecessor to measure against.
    Nothing,
    /// The predecessor exists and would not read. Not "first in file", and not a
    /// zero: the row before a gap is not this row's predecessor (D-0730).
    Unreadable,
    /// The record written immediately before.
    Bar(Bar),
}

fn with_change(behind: Behind, rows: Vec<Option<Bar>>) -> Vec<WindowBar> {
    let mut out = Vec::with_capacity(rows.len());
    let mut previous = behind;
    for slot in rows {
        let Some(bar) = slot else {
            previous = Behind::Unreadable;
            continue;
        };
        let (chg, chg_why) = match previous {
            Behind::Nothing => (None, "first_bar_in_file"),
            Behind::Unreadable => (None, "previous_unreadable"),
            Behind::Bar(before) => match crate::server::basis_points(before.close, bar.close) {
                Ok(bps) => (Some(bps), ""),
                Err(crate::server::Unknown::Overflow) => (None, "overflow"),
                // ZERO AND NEGATIVE ARE DIFFERENT FACTS (gap-audit #13, D-3685):
                // a zero close is a real zero, a negative one a corrupt stored
                // value, and both were called "is zero".
                Err(_) if before.close == 0 => (None, "previous_close_zero"),
                Err(_) => (None, "previous_close_negative"),
            },
        };
        // OPEN INTEREST HAS A NULL AND A REAL ZERO, and they are not the same
        // cell. `OI_NULL` is the sentinel `CLAUDE.md` §7 names; a stored zero is
        // a stored zero and must not be erased into "unknown".
        let oi_null = bar.open_interest == OI_NULL;
        let (oichg, oichg_why) = if oi_null {
            (None, "oi_null")
        } else {
            match previous {
                Behind::Nothing => (None, "first_bar_in_file"),
                Behind::Unreadable => (None, "previous_unreadable"),
                Behind::Bar(before) if before.open_interest == OI_NULL => (None, "oi_null_before"),
                Behind::Bar(before) => {
                    match crate::server::basis_points(before.open_interest, bar.open_interest) {
                        Ok(bps) => (Some(bps), ""),
                        Err(crate::server::Unknown::Overflow) => (None, "overflow"),
                        Err(_) if before.open_interest == 0 => (None, "previous_oi_zero"),
                        Err(_) => (None, "previous_oi_negative"),
                    }
                }
            }
        };
        previous = Behind::Bar(bar);
        out.push(WindowBar {
            bar,
            chg,
            chg_why,
            oichg,
            oichg_why,
        });
    }
    out
}

/// One page of bars drawn from a RANGE of months.
#[derive(Debug, Clone)]
pub struct Window {
    /// Every row the window holds, across every month that opened.
    pub total: u64,
    /// Months that opened.
    pub months_read: usize,
    /// Months named by the range that hold no file. Not an error: a store is
    /// allowed to be sparse, and saying so is how a reader tells a gap from a
    /// refusal.
    pub months_missing: usize,
    /// The rows asked for, each with the change folded in time order.
    pub bars: Vec<WindowBar>,
    /// Records that would not read, named. Never silently dropped.
    pub faults: Vec<String>,
    /// Present only when asked for, because it costs a scan.
    pub extremes: Option<Extremes>,
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

/// The widest range and heaviest volume over a set of bars.
///
/// One pass, no allocation. Separate from [`window`] because it is the whole of
/// what `extremes=1` buys and reads as one sentence on its own.
#[must_use]
fn extremes_of(bars: &[WindowBar]) -> Extremes {
    bars.iter().fold(Extremes::default(), |mut top, row| {
        let bar = &row.bar;
        let range = bar.high.saturating_sub(bar.low);
        if range > top.range {
            top.range = range;
        }
        if bar.volume > top.volume {
            top.volume = bar.volume;
        }
        top
    })
}

/// One month's extremes and unreadable records, as [`extremes_of`] and
/// [`slots`] find them over every record, with the header the open handle
/// read them under.
#[derive(Debug, Clone)]
struct MonthFold {
    header: store::header::Header,
    extremes: Extremes,
    faults: Vec<String>,
}

/// The kept folds, by bar-file path, each under the stamp its file had
/// BEFORE it was opened. See [`MONTH_FOLDS_KEPT`].
type KeptFolds =
    std::collections::HashMap<std::path::PathBuf, (crate::answer_memo::FileStamp, MonthFold)>;

/// See [`KeptFolds`].
static MONTH_FOLDS: std::sync::LazyLock<std::sync::Mutex<KeptFolds>> =
    std::sync::LazyLock::new(|| {
        std::sync::Mutex::new(std::collections::HashMap::with_capacity(MONTH_FOLDS_KEPT))
    });

/// The extremes and unreadable records of one month, read whole only when no
/// kept fold answers for the file as it now stands. W1-api5-4, D-4439.
///
/// # When a kept fold answers
///
/// `before` is one `stat` of the path taken BEFORE the window opened it. A
/// kept fold answers only when that stamp (device, inode, length, both
/// clocks) and the open handle's whole header (generation, `n_valid`, both
/// stamps) equal the ones it was kept under. Any write moves the
/// status-change time and any commit moves the generation, so a month that
/// reads differently cannot match.
///
/// # When a fold is kept
///
/// Only when a second `stat`, taken after every record was read, equals
/// `before`: then nothing replaced or wrote the file between the stamp and
/// the read, so the handle read the stamped file. A fold whose stamps differ,
/// or whose stamp could not be taken, still answers this request and is not
/// kept, so the next request reads the month again. Unreadable records are
/// kept with it and named on every answer, exactly as a full read names
/// them, so a kept fold never hides a fault.
fn month_fold(file: &BarFile, before: Option<crate::answer_memo::FileStamp>) -> MonthFold {
    let header = file.header();
    let path = file.path();
    if let Some(stamp) = before {
        let kept = MONTH_FOLDS
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(path)
            .filter(|(kept, fold)| *kept == stamp && fold.header == header)
            .map(|(_, fold)| fold.clone());
        if let Some(fold) = kept {
            return fold;
        }
    }
    #[cfg(test)]
    tests::COLD_FOLDS.with(|count| count.set(count.get() + 1));
    let mut extremes = Extremes::default();
    let mut faults = Vec::new();
    for slot in 0..header.n_valid {
        match file.read_record(slot) {
            Ok(bar) => {
                extremes.range = extremes.range.max(bar.high.saturating_sub(bar.low));
                extremes.volume = extremes.volume.max(bar.volume);
            }
            Err(why) => faults.push(format!("record {slot}: {why}")),
        }
    }
    let fold = MonthFold {
        header,
        extremes,
        faults,
    };
    if let Some(stamp) = before
        && crate::answer_memo::FileStamp::of(path).ok() == Some(stamp)
    {
        let mut kept = MONTH_FOLDS
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if kept.len() >= MONTH_FOLDS_KEPT && !kept.contains_key(path) {
            kept.clear();
        }
        kept.insert(path.to_path_buf(), (stamp, fold.clone()));
    }
    fold
}

/// The rows at `offset..offset+limit` of a window, WITHOUT reading the rest.
///
/// The files arrive in the order their rows come out in, so the prefix sum over
/// each header's `n_valid` finds the file holding `offset` without touching a
/// record. Only the records returned are read: this is `O(months)` header reads
/// plus `O(limit)` record reads, and the four million rows a deep page sits past
/// cost nothing.
///
/// DESCENDING IS THE FILE READ BACKWARDS, and that is the half a single-file
/// test cannot catch. A month's records are written oldest-first, so newest-first
/// over a whole window is each file reversed **as well as** the files reversed.
/// Reading a file forwards and reversing afterwards is right for one file and
/// wrong the moment a page straddles two.
/// ONE RECORD IS READ BEHIND THE PAGE, and it is never returned.
///
/// The change column is against the previous bar IN TIME, so the first row of a
/// page needs the record before it or the cell is blank. Reading one back costs
/// a single extra seek and fills it — which is strictly better than the browser
/// managed while holding whole months, because there the first row of EVERY
/// month had nothing behind it.
///
/// A ROW THAT OPENS A MONTH FILE READS THE PREVIOUS MONTH'S LAST RECORD. This
/// doc said that already and the code did not: `start.checked_sub(LOOKBACK)`
/// is `None` at the start of EVERY file, so the first bar of every month in a
/// window answered `first_bar_in_file` while the month before it sat open in
/// the same request (Z1-slice11-F4, D-1762). [`earlier_in_time`] names, for
/// each file, the nearest earlier non-empty file of the window, and both paths
/// measure against its last record. The only row that still has none is the
/// first record of the window's earliest non-empty file, where none was read.
const LOOKBACK: usize = 1;

/// For each file of a window, the nearest EARLIER-IN-TIME file holding at
/// least one record, or `None`, in the order of `files`.
///
/// `files` are in time order, newest first when `newest_first`. One pass, so
/// it is `O(months)` header reads however the empty months fall — a lookback
/// that walked back per file would be `O(months²)` over a sparse window.
fn earlier_in_time(files: &[BarFile], newest_first: bool) -> Vec<Option<&BarFile>> {
    let mut last: Option<&BarFile> = None;
    let mut out: Vec<Option<&BarFile>> = Vec::with_capacity(files.len());
    if newest_first {
        for file in files.iter().rev() {
            out.push(last);
            if file.header().n_valid > 0 {
                last = Some(file);
            }
        }
        out.reverse();
    } else {
        for file in files {
            out.push(last);
            if file.header().n_valid > 0 {
                last = Some(file);
            }
        }
    }
    out
}

/// The last record of `file` as a lookback: a bar, or a named gap.
fn last_of(file: &BarFile) -> Behind {
    file.header()
        .n_valid
        .checked_sub(1)
        .map_or(Behind::Nothing, |last| {
            file.read_record(last)
                .map_or(Behind::Unreadable, Behind::Bar)
        })
}

fn seek_page(
    files: &[BarFile],
    desc: bool,
    offset: usize,
    limit: usize,
) -> (Vec<WindowBar>, Vec<String>, Option<&BarFile>) {
    let mut bars: Vec<WindowBar> = Vec::with_capacity(limit.min(PAGE_BARS));
    let mut first_faulted = None;
    let mut faults = Vec::new();
    let mut seen = 0usize;
    /* POSITIONS USED, NOT ROWS RETURNED. An unreadable record still occupies
    its place in the window: it is named in `faults` on the page that covers
    it. Counting only the rows that read made a page that met damage fill the
    gap from the next file, and the next offset page returned those same rows
    a second time (W1-api1-10, D-0730). */
    let mut filled = 0usize;
    let earlier = earlier_in_time(files, desc);
    for (at, file) in files.iter().enumerate() {
        if filled >= limit {
            break;
        }
        let held = usize::try_from(file.header().n_valid).unwrap_or(usize::MAX);
        let lo = seen;
        seen = seen.saturating_add(held);
        if seen <= offset {
            continue;
        }
        let skip_in_file = offset.saturating_sub(lo);
        let take = limit - filled;
        /* THE BLOCK IN FILE ORDER, WHICHEVER DIRECTION THE PAGE RUNS.
        `start..end` is always ascending because the change fold needs the
        order the records were WRITTEN; the reversal for a descending page
        happens after, on the folded rows. */
        let (start, end) = if desc {
            let end = held.saturating_sub(skip_in_file);
            (end.saturating_sub(take), end)
        } else {
            (skip_in_file, skip_in_file.saturating_add(take).min(held))
        };
        /* THE LOOKBACK IS READ ON ITS OWN, AND ITS FAILURE IS NOT THE PAGE'S.
        It used to be the first record of the block and was dropped by
        POSITION after the fold; with it unreadable, `page` had already left
        it out, and the row dropped was the page's own first row, unnamed.
        Read apart, an unreadable lookback costs the first row its change
        cell (`previous_unreadable`) and nothing else. It is not added to
        `faults`: it is not a row of this page, and the page that covers it
        names it, so a reader walking every page meets each fault once. */
        let behind = match start.checked_sub(LOOKBACK) {
            None => earlier
                .get(at)
                .copied()
                .flatten()
                .map_or(Behind::Nothing, last_of),
            Some(before) => file
                .read_record(u64::try_from(before).unwrap_or(u64::MAX))
                .map_or(Behind::Unreadable, Behind::Bar),
        };
        let (block, mut bad) = slots(file, start, end - start);
        filled += end - start;
        let mut folded = with_change(behind, block);
        if desc {
            folded.reverse();
        }
        bars.append(&mut folded);
        if !bad.is_empty() {
            first_faulted = first_faulted.or(Some(file));
        }
        faults.append(&mut bad);
    }
    (bars, faults, first_faulted)
}

/// Every record of a window's files, change-folded IN TIME ORDER, for the
/// reading path. `files` are newest first when `newest_first`. Faults are
/// returned, and reported once for the request (Z1-slice11-F3).
///
/// # Errors
///
/// [`reserved_window`]'s refusal, when `total` rows cannot be held.
fn read_in_time(
    files: &[BarFile],
    newest_first: bool,
    total: u64,
) -> Result<(Vec<WindowBar>, Vec<String>), String> {
    let mut all: Vec<WindowBar> = reserved_window(total)?;
    let mut record_faults = Vec::new();
    let mut first_faulted = None;
    /* IN TIME ORDER, so each file's first row is measured against the last
    record of the month before it (Z1-slice11-F4). `ordered` was reversed
    above only when the order is newest-first by time. */
    let mut in_time: Vec<&BarFile> = files.iter().collect();
    if newest_first {
        in_time.reverse();
    }
    let mut behind = Behind::Nothing;
    for file in in_time {
        let held = usize::try_from(file.header().n_valid).unwrap_or(usize::MAX);
        let (rows, mut bad) = slots(file, 0, held);
        // AN EMPTY MONTH LEAVES THE LOOKBACK WHERE IT WAS: the bar before the
        // next month's first row is still the last one read before it.
        let next = match rows.last() {
            None => behind,
            Some(Some(bar)) => Behind::Bar(*bar),
            Some(None) => Behind::Unreadable,
        };
        all.append(&mut with_change(behind, rows));
        behind = next;
        if !bad.is_empty() {
            first_faulted = first_faulted.or(Some(file));
        }
        record_faults.append(&mut bad);
    }
    if let Some(file) = first_faulted {
        note_unreadable_records(file, &record_faults, all.len(), 0);
    }
    Ok((all, record_faults))
}

/// Room for a whole window's `total` rows, or a refusal naming the number.
///
/// `Vec::with_capacity` aborts the process on a size no allocation can hold
/// (`handle_alloc_error`; the release profile also aborts on panic), and a
/// window sums `n_valid` over up to [`MAX_WINDOW_MONTHS`] months. The store
/// bounds each month's counter by its grid (D-2685), and that still leaves a
/// sum the machine may not have. `try_reserve_exact` turns it into an answer
/// (CE-61, D-2685).
///
/// # Errors
///
/// A sentence naming `total`, when it does not fit `usize` or the reservation
/// fails.
fn reserved_window(total: u64) -> Result<Vec<WindowBar>, String> {
    let refused = || {
        format!(
            "the window holds {total} stored records, more than this process \
             can reserve at once; ask for fewer months"
        )
    };
    let rows = usize::try_from(total).map_err(|_| refused())?;
    let mut all = Vec::new();
    all.try_reserve_exact(rows).map_err(|_| refused())?;
    Ok(all)
}

/// The window's extremes from each month's kept fold, every month's unreadable
/// records named in time order, and the first month that had one (W1-api5-4,
/// D-4439).
///
/// The window's widest range and heaviest volume are the widest and heaviest
/// of its months, and a month's are kept by [`month_fold`] until its file
/// moves, so a `ts` page with `extremes=1` costs the seek page plus one probe a
/// month instead of every record of every month. `stamps` are the files'
/// stamps taken before they were opened; a month without one is folded again.
fn kept_extremes<'f>(
    files: &'f [BarFile],
    desc: bool,
    stamps: Option<&std::collections::HashMap<std::path::PathBuf, crate::answer_memo::FileStamp>>,
) -> (Extremes, Vec<String>, Option<&'f BarFile>) {
    let mut top = Extremes::default();
    let mut named = Vec::new();
    let mut first_faulted = None;
    let in_time: Vec<&BarFile> = if desc {
        files.iter().rev().collect()
    } else {
        files.iter().collect()
    };
    for file in in_time {
        let before = stamps.and_then(|stamps| stamps.get(file.path())).copied();
        let fold = month_fold(file, before);
        top.range = top.range.max(fold.extremes.range);
        top.volume = top.volume.max(fold.extremes.volume);
        if !fold.faults.is_empty() {
            first_faulted = first_faulted.or(Some(file));
        }
        named.extend(fold.faults);
    }
    (top, named, first_faulted)
}

/// One page of bars across a range of months, in one request.
///
/// # The two paths, and why only one of them scans
///
/// Ordered by `ts`, this SEEKS. `n_valid` is in each file's header, so the
/// prefix sum over the opened months says which file holds row `offset` without
/// reading a single record, and `page` then reads `limit` of them by index. The
/// cost is `O(months)` header reads plus `O(limit)` record reads — page 12,470
/// costs what page 1 costs, and neither costs the four million rows between
/// them.
///
/// Ordered by anything else, it reads every row once, sorts, and slices. That
/// is the honest price of a question the store has no index for, and it is paid
/// here rather than by moving every row to a browser. Since D-4439 a window
/// of more than [`MAX_SCAN_WINDOW_RECORDS`] stored records is refused by name
/// on that path before a record is read.
///
/// `want_extremes` used to force the reading path either way, because the
/// widest range in a window is not knowable from a header. Since D-4439 a
/// `ts` window keeps its seek page and takes the extremes from each month's
/// kept fold ([`month_fold`]): the first request after a month's file moves
/// reads that month whole, and every other request costs one `stat` and one
/// probe per month (W1-api5-4).
///
/// # Errors
///
/// The range being inverted or too long, an invalid address, or an unavailable
/// store root. Unreadable months remain named faults alongside readable rows.
#[expect(
    clippy::too_many_arguments,
    reason = "every argument names one coordinate of the same address — feed, \
              exchange, segment, symbol, contract, rung, month range — and the \
              alternative is a struct whose only caller builds it inline at the \
              one call site, which hides nothing and names the same seven."
)]
pub fn window(
    store_root: &std::path::Path,
    vendor: Vendor,
    exchange: &str,
    segment: &str,
    symbol: &str,
    timeframe: Timeframe,
    contract: Option<brutex_core::instrument::Contract>,
    from: YearMonth,
    to: YearMonth,
    sort: SortKey,
    desc: bool,
    offset: usize,
    limit: usize,
    want_extremes: bool,
) -> Result<Window, String> {
    let Some(months) = months_of(from, to) else {
        return Err(format!(
            "{}-{:02} to {}-{:02} is not a month range this build will read: it \
             must run forwards and span at most {MAX_WINDOW_MONTHS} months.",
            from.year(),
            from.month(),
            to.year(),
            to.month()
        ));
    };
    let limit = limit.min(MAX_WINDOW_LIMIT);

    /* NEWEST FIRST WHEN THE ORDER IS NEWEST FIRST. The seek path walks the
    files in the order the rows come out in, so descending time reads the
    months backwards and the prefix sum needs no second thought. */
    let mut ordered = months;
    if desc && !sort.scans() {
        ordered.reverse();
    }

    // STAMPED BEFORE THE OPEN, for the kept month folds (D-4439): a fold is
    // served only for a file whose stamp before this open is the one it was
    // kept under. Only the seek path with extremes asks; one `stat` a month.
    let parts = PathParts {
        vendor,
        exchange,
        segment,
        symbol,
        contract,
        timeframe,
        month: from,
        file: FileKind::Bars,
    };
    let stamps =
        (want_extremes && !sort.scans()).then(|| stamp_months(store_root, parts, &ordered));

    /* OPENED ONCE, HELD FOR THE REQUEST. A month with no file is counted and
    skipped: a sparse store is legal and the count is what tells a reader a
    gap from a refusal. */
    let OpenedWindow {
        files,
        missing,
        mut faults,
    } = open_window_months(store_root, parts, &ordered)?;
    if files.is_empty() {
        return Ok(Window {
            total: 0,
            months_read: 0,
            months_missing: missing,
            bars: Vec::new(),
            faults,
            extremes: want_extremes.then(Extremes::default),
        });
    }

    let total: u64 = files
        .iter()
        .map(|f| f.header().n_valid)
        .fold(0u64, u64::saturating_add);

    if !sort.scans() {
        let (bars, mut record_faults, mut first_faulted) = seek_page(&files, desc, offset, limit);
        // EXTREMES WITHOUT THE READING PATH (W1-api5-4, D-4439). The window's
        // widest range and heaviest volume are the widest and heaviest of its
        // months, and a month's are kept by `month_fold` until its file moves,
        // so a `ts` page with `extremes=1` is the seek page plus one probe a
        // month instead of every record of every month. Every unreadable
        // record of every month is named, as the full read named them, in
        // time order; the page's own are among them.
        let extremes = want_extremes.then(|| {
            let (top, named, first) = kept_extremes(&files, desc, stamps.as_ref());
            record_faults = named;
            first_faulted = first;
            top
        });
        if let Some(file) = first_faulted {
            note_unreadable_records(file, &record_faults, bars.len(), offset);
        }
        faults.append(&mut record_faults);
        return Ok(Window {
            total,
            months_read: files.len(),
            months_missing: missing,
            bars,
            faults,
            extremes,
        });
    }

    // A SCAN PAST ITS CEILING IS REFUSED BY NAME, before a record is read
    // (W1-api5-4, D-4439): ordering by a price column reads and holds every
    // record of the window.
    scan_admitted(total)?;

    /* ---- THE READING PATH. One pass, then order, then slice. ----
    THE CHANGE IS FOLDED PER FILE, BEFORE ANYTHING IS SORTED, because it is
    against the previous bar in TIME and a sorted page has no time
    neighbours. Folding after the sort would compute each row against
    whichever row happened to land above it. */
    let (all, mut record_faults) = read_in_time(&files, desc && !sort.scans(), total)?;
    faults.append(&mut record_faults);

    let extremes = want_extremes.then(|| extremes_of(&all));

    /* A TOTAL ORDER, SO THE PAGE BOUNDARY IS STABLE. Two bars with the same
    close must not swap between one request and the next, or a reader paging
    through them would see one row twice and another never. The timestamp is
    unique within a series, so it is the tie-break and it is NOT inverted
    with the direction. */
    let null_last =
        |row: &WindowBar| matches!(sort, SortKey::OpenInterest) && row.bar.open_interest == OI_NULL;
    let order = |a: &WindowBar, b: &WindowBar| {
        let (x, y) = (sort.of(&a.bar), sort.of(&b.bar));
        let primary = if desc { y.cmp(&x) } else { x.cmp(&y) };
        // NULL OPEN INTEREST LAST, whichever way the page runs (F5).
        null_last(a)
            .cmp(&null_last(b))
            .then(primary)
            .then_with(|| a.bar.ts_micros.cmp(&b.bar.ts_micros))
    };

    // THE PAGE IS BOUNDED, SO THE ORDERING IS TOO -- AND IT WAS NOT.
    //
    // LINE COMMENTS, NOT A BLOCK. Gate 11 strips lines opening with `//` or `*`
    // before it counts, and the `/* */` blocks elsewhere in this file have
    // neither, so naming the two constructs below in prose would count as two
    // more uses of them. A comment that trips the gate it is explaining is the
    // same shape as an invariant row whose note names a test that does not
    // exist -- see docs/04-invariants.md V-01.
    //
    // This ordered EVERY bar in the window and then took `.skip(offset)
    // .take(limit)`. A 240-month window is roughly 1.9 million bars, ordered in
    // full to hand back a thousand rows. `docs/07-o1-architecture.md` layer 12
    // asks for a bounded page and never O(universe); an audit found this route
    // breaching it.
    //
    // `page_of` partitions in O(n) and orders only the page's own rows, so the
    // cost goes from `O(n log n)` to `O(n) + O(limit log limit)`. D-0733 first
    // bounded it by the nearer end of the window; D-1446 bounds it by the page.
    //
    // THE OUTPUT IS UNCHANGED, and that is what a TOTAL comparator buys: with
    // the timestamp as tie-break no two rows ever compare equal, so the set of
    // the page's rows is unique and ordering it is byte-identical to
    // ordering everything and slicing -- §3 rule 5. An unstable partition may
    // reorder only what it is free to reorder, and here there is nothing.
    // `selecting_the_page_then_ordering_it_equals_ordering_everything_then_slicing`
    // runs both strategies over a fixture with every primary value repeated
    // three times, which is where a non-total comparator would part company.
    //
    // THE BARS ARE STILL ALL READ, and that is not what this changes. `total`,
    // `extremes_of` and the per-file change fold each need every row; the read
    // is O(bars) because of the question being asked. What is removed is the
    // ordering of rows nobody will see.
    //
    // AND THE ROWS BEFORE THE PAGE ARE NOT ORDERED EITHER. `want` was
    // `offset + limit`, and `offset` comes off the query string uncapped, so a
    // page near the end of a 1.9-million-bar window ordered all of them again,
    // and an offset past the end ordered every row to answer none. `page_of`
    // partitions twice instead: once at `offset`, once at `limit` inside what
    // is left, and orders only the page. W1-api5-4, D-1446.
    let bars = page_of(all, offset, limit, order);

    Ok(Window {
        total,
        months_read: files.len(),
        months_missing: missing,
        bars,
        faults,
        extremes,
    })
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

/// Whether a window of `total` stored records may be read whole to order it
/// by a price column: at most [`MAX_SCAN_WINDOW_RECORDS`], refused by name
/// past it. W1-api5-4, D-4439.
///
/// # Errors
///
/// The sentence naming `total` and the ceiling.
fn scan_admitted(total: u64) -> Result<(), String> {
    if total > MAX_SCAN_WINDOW_RECORDS {
        return Err(format!(
            "ordering by a column other than ts reads every stored record of the \
             window, {total} here, and one request reads at most \
             {MAX_SCAN_WINDOW_RECORDS} (MAX_SCAN_WINDOW_RECORDS); ask for fewer \
             months, or order by ts, which seeks"
        ));
    }
    Ok(())
}

/// One `stat` of each month's bar file, by the path [`open_window_months`]
/// opens it at, for every month whose path is valid and whose stamp could be
/// taken. A month missing here is simply never served from a kept fold.
fn stamp_months(
    root: &std::path::Path,
    parts: PathParts<'_>,
    months: &[YearMonth],
) -> std::collections::HashMap<std::path::PathBuf, crate::answer_memo::FileStamp> {
    let mut stamps = std::collections::HashMap::with_capacity(months.len());
    for month in months {
        let Ok(path) = StorePath::new(PathParts {
            month: *month,
            ..parts
        }) else {
            continue;
        };
        let path = path.to_path_buf(root);
        if let Ok(stamp) = crate::answer_memo::FileStamp::of(&path) {
            stamps.insert(path, stamp);
        }
    }
    stamps
}

struct OpenedWindow {
    files: Vec<BarFile>,
    missing: usize,
    faults: Vec<String>,
}

fn open_window_months(
    root: &std::path::Path,
    parts: PathParts<'_>,
    months: &[YearMonth],
) -> Result<OpenedWindow, String> {
    let metadata = std::fs::metadata(root)
        .map_err(|why| format!("cannot read store root {}: {why}", root.display()))?;
    if !metadata.is_dir() {
        return Err(format!("store root {} is not a directory", root.display()));
    }
    let mut opened = OpenedWindow {
        files: Vec::with_capacity(months.len()),
        missing: 0,
        faults: Vec::new(),
    };
    for month in months {
        match open_classified(
            root,
            PathParts {
                month: *month,
                ..parts
            },
        ) {
            Ok(file) => opened.files.push(file),
            Err(why) => match why.kind {
                OpenRefusalKind::Invalid => return Err(why.message),
                OpenRefusalKind::Absent => opened.missing = opened.missing.saturating_add(1),
                OpenRefusalKind::Unreadable => {
                    opened.faults.push(format!("{month}: {}", why.message));
                }
            },
        }
    }
    Ok(opened)
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

    thread_local! {
        /// Months [`month_fold`] read whole on this thread.
        pub(super) static COLD_FOLDS: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
    }

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
        let squash = |text: &str| text.split_whitespace().collect::<Vec<_>>().join(" ");
        let limits = include_str!("../../../docs/06-limits.md");
        let section = limits
            .split_once("## A sorted or extremes window reads every bar in its range — D-0733")
            .expect("docs/06-limits.md states the sorted window's read")
            .1;
        let section = squash(section.split_once("\n## ").map_or(section, |(own, _)| own));
        let source = include_str!("bars.rs");
        let code = squash(
            source
                .split_once("\n#[cfg(test)]\n")
                .expect("the tests follow the code")
                .0,
        );
        for quoted in [
            "let mut all: Vec<WindowBar> = reserved_window(total)?;",
            "let (rows, mut bad) = slots(file, 0, held);",
            "match file.read_record(index)",
            "let extremes = want_extremes.then(|| extremes_of(&all));",
            "page.select_nth_unstable_by(limit - 1, &mut order);",
            "page.sort_by(&mut order);",
            "pub const MAX_WINDOW_MONTHS: usize = 240;",
            "pub const MAX_WINDOW_LIMIT: usize = 1_000;",
        ] {
            assert!(
                section.contains(&format!("`{quoted}`")),
                "the limit quotes {quoted}"
            );
            assert!(code.contains(quoted), "bars.rs still says {quoted}");
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

    /// **A PAGE THAT NEEDS NO PARTITION PAYS FOR NONE.** At offset zero there
    /// is nothing before the page to cut away, and a limit that reaches the end
    /// of what is left has nothing after it: either partition there is a full
    /// linear pass that changes no row. Results alone cannot see it, so the
    /// comparison count is held EXACTLY to that of ordering the rows alone.
    /// G18-api-04.
    #[test]
    fn a_page_needing_no_partition_compares_only_to_order_itself() {
        let rows: Vec<i64> = (0..64).collect();
        let plain = std::cell::Cell::new(0_u64);
        let mut alone = rows.clone();
        alone.sort_by(|a, b| {
            plain.set(plain.get() + 1);
            a.cmp(b)
        });
        assert!(plain.get() > 0);
        // limit == len (no cut after) and limit > len (none either).
        for limit in [rows.len(), rows.len() + 1] {
            let compared = std::cell::Cell::new(0_u64);
            let page = super::page_of(rows.clone(), 0, limit, |a, b| {
                compared.set(compared.get() + 1);
                a.cmp(b)
            });
            assert_eq!(page, rows, "limit={limit}");
            assert_eq!(compared.get(), plain.get(), "limit={limit}");
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

    /// **CE-61. A WINDOW TOTAL NO ALLOCATION CAN HOLD IS REFUSED BY NAME.**
    ///
    /// The reading path reserved `total` rows with `Vec::with_capacity`, so a
    /// counter no machine can hold aborted the server (`panic = "abort"`,
    /// and an allocation failure aborts in any profile). The store now refuses
    /// a counter past the month at open (D-2685); this is the second wall, for
    /// a sum over many months that is legal per month and still too large.
    #[test]
    fn a_window_total_no_allocation_can_hold_is_refused_by_name() {
        let why = read_in_time(&[], false, u64::MAX)
            .map(|(all, _)| all.len())
            .expect_err("refused, not aborted");
        assert!(why.contains(&u64::MAX.to_string()), "{why}");
        // A total that fits is reserved, and nothing is read for it.
        let (all, faults) = read_in_time(&[], false, 3).expect("three rows fit");
        assert!(all.is_empty() && all.capacity() >= 3 && faults.is_empty());
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
            segment: "INDEX",
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
        // n = 0 is a month whose file EXISTS and holds nothing: created, never
        // appended to, since the store refuses an empty batch.
        if !rows.is_empty() {
            file.append(&rows).expect("the batch appends");
        }
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
        let got = window(
            &root,
            Vendor::Dhan,
            "NSE",
            "INDEX",
            SYMBOL,
            Timeframe::MINUTE_1,
            None,
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

        let got = window(
            &root,
            Vendor::Dhan,
            "NSE",
            "INDEX",
            SYMBOL,
            Timeframe::MINUTE_1,
            None,
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

        let got = window(
            &root,
            Vendor::Dhan,
            "NSE",
            "INDEX",
            SYMBOL,
            Timeframe::MINUTE_1,
            None,
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

        let got = window(
            &root,
            Vendor::Dhan,
            "NSE",
            "INDEX",
            SYMBOL,
            Timeframe::MINUTE_1,
            None,
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
        let got = window(
            &root,
            Vendor::Dhan,
            "NSE",
            "INDEX",
            SYMBOL,
            Timeframe::MINUTE_1,
            None,
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

        let got = window(
            &root,
            Vendor::Dhan,
            "NSE",
            "INDEX",
            SYMBOL,
            Timeframe::MINUTE_1,
            None,
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

        let got = window(
            &root,
            Vendor::Dhan,
            "NSE",
            "INDEX",
            SYMBOL,
            Timeframe::MINUTE_1,
            None,
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

    /// **A `ts` window with extremes reads each month whole once, until that
    /// month's file moves; its pages are the seek pages, its extremes are the
    /// full read's, and a kept fold still names every unreadable record.**
    /// W1-api5-4, D-4439.
    #[test]
    fn a_ts_window_with_extremes_reads_each_month_once_until_it_moves() {
        let root = scratch("kept-folds");
        let (jan, feb, mar) = (
            YearMonth::new(2026, 1).expect("m"),
            YearMonth::new(2026, 2).expect("m"),
            YearMonth::new(2026, 3).expect("m"),
        );
        write_month(&root, jan, 10, 1_000);
        write_month(&root, feb, 40, 2_000);
        write_month(&root, mar, 25, 3_000);
        let cold = || super::tests::COLD_FOLDS.with(std::cell::Cell::get);
        let start = cold();
        let ask = |desc, offset, extremes| {
            window_over(&root, mar, SortKey::Ts, desc, offset, 7, extremes)
        };

        let first = ask(false, 3, true);
        assert_eq!(cold() - start, 3, "each month is read whole once");
        let top = first.extremes.expect("asked for");
        assert_eq!(
            (top.range, top.volume),
            (20, 78),
            "over every month, not the page"
        );
        assert_eq!(
            first.bars,
            ask(false, 3, false).bars,
            "the page is the seek page"
        );
        assert_eq!(first.total, 75);
        // The full read's extremes, from the reading path a price order takes.
        let full = window_over(&root, mar, SortKey::Close, false, 0, 1, true);
        assert_eq!(full.extremes, Some(top));

        for (desc, offset) in [(false, 0), (true, 0), (true, 40), (false, 74)] {
            let again = ask(desc, offset, true);
            assert_eq!(again.extremes, Some(top));
            assert_eq!(again.bars, ask(desc, offset, false).bars);
        }
        assert_eq!(cold() - start, 3, "every later page is warm");

        // A month rewritten under the same name is read again, and only it.
        let path = |month| {
            StorePath::new(PathParts {
                vendor: Vendor::Dhan,
                exchange: "NSE",
                segment: "INDEX",
                symbol: SYMBOL,
                contract: None,
                timeframe: Timeframe::MINUTE_1,
                month,
                file: FileKind::Bars,
            })
            .expect("a legal path")
            .to_path_buf(&root)
        };
        std::fs::remove_file(path(feb)).expect("owned fixture");
        write_month(&root, feb, 60, 2_000);
        let grown = ask(false, 0, true);
        assert_eq!(cold() - start, 4, "only the moved month is read again");
        assert_eq!(
            grown.extremes.expect("asked").volume,
            118,
            "59*2, the new month's"
        );
        assert_eq!(grown.total, 95);

        // A damaged record is named on the cold read AND on every warm one.
        damage_record(&root, mar, 4);
        let damaged = ask(false, 0, true);
        assert_eq!(cold() - start, 5);
        // A sealed block refuses every record it holds, so the damage is named
        // once per record of that block, record 4 among them.
        assert!(
            damaged
                .faults
                .iter()
                .any(|fault| fault.starts_with("record 4:")),
            "{:?}",
            damaged.faults
        );
        let warm = ask(true, 0, true);
        assert_eq!(cold() - start, 5, "warm");
        assert_eq!(
            warm.faults, damaged.faults,
            "a kept fold never hides a fault"
        );
        assert_eq!(
            full_faults(&root, mar),
            damaged.faults,
            "and names exactly what the full read names"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    /// The record faults the reading path names for the window to `to`.
    fn full_faults(root: &std::path::Path, to: YearMonth) -> Vec<String> {
        window_over(root, to, SortKey::Close, false, 0, 1, true).faults
    }

    /// **A price order past `MAX_SCAN_WINDOW_RECORDS` is refused by name, and
    /// the ceiling itself is admitted.** W1-api5-4, D-4439.
    #[test]
    fn a_scan_past_its_ceiling_is_refused_by_name() {
        assert_eq!(scan_admitted(MAX_SCAN_WINDOW_RECORDS), Ok(()));
        assert_eq!(scan_admitted(0), Ok(()));
        let why = scan_admitted(MAX_SCAN_WINDOW_RECORDS + 1).expect_err("past it");
        assert!(why.contains("1048577 here"), "{why}");
        assert!(why.contains("MAX_SCAN_WINDOW_RECORDS"), "{why}");
        let source = include_str!("bars.rs");
        let window_body = source
            .split_once("\npub fn window(")
            .expect("window")
            .1
            .split_once("\n}\n")
            .expect("its end")
            .0;
        let at = |needle: &str| window_body.find(needle).expect(needle);
        assert!(
            at("scan_admitted(total)?;") < at("read_in_time("),
            "refused before any record is read"
        );
    }

    /// What a `ts` page with extremes and a price-ordered page cost over long
    /// windows of full one-minute months: the first request (every month read)
    /// against a later one (kept folds), and a price order at about the scan
    /// ceiling. A measurement, run on purpose; the numbers are in
    /// `docs/06-limits.md`'s D-4439 row. W1-api5-4.
    #[test]
    #[ignore = "a latency measurement, run on purpose: see crate::latency"]
    fn latency_window_extremes_and_scan() {
        let root = scratch("latency");
        let months = months_of(
            YearMonth::new(2006, 1).expect("m"),
            YearMonth::new(2025, 12).expect("m"),
        )
        .expect("240 months");
        for month in &months {
            write_month(&root, *month, 8_250, 1_000);
        }
        let ask = |to: YearMonth, sort, extremes| {
            window(
                &root,
                Vendor::Dhan,
                "NSE",
                "INDEX",
                SYMBOL,
                Timeframe::MINUTE_1,
                None,
                YearMonth::new(2006, 1).expect("m"),
                to,
                sort,
                true,
                0,
                200,
                extremes,
            )
        };
        let last = YearMonth::new(2025, 12).expect("m");
        let cold =
            crate::latency::Timed::run(1, || ask(last, SortKey::Ts, true).map(drop)).expect("cold");
        let warm = crate::latency::Timed::run(200, || ask(last, SortKey::Ts, true).map(drop))
            .expect("warm");
        let seek = crate::latency::Timed::run(200, || ask(last, SortKey::Ts, false).map(drop))
            .expect("seek");
        // 127 months of 8,250 is 1,047,750 records, just under 2^20.
        let scan_to = months[126];
        let scan = crate::latency::Timed::run(10, || ask(scan_to, SortKey::Close, false).map(drop))
            .expect("scan");
        let refused = ask(months[127], SortKey::Close, false).expect_err("past the ceiling");
        println!("{}", cold.line("ts + extremes, 240 x 8,250, first request"));
        println!("{}", warm.line("ts + extremes, 240 x 8,250, kept folds"));
        println!("{}", seek.line("ts, no extremes, 240 x 8,250"));
        println!(
            "{}",
            scan.line("close order, 127 x 8,250 = 1,047,750 records")
        );
        println!("128 months refused: {refused}");
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

        let sorted = window(
            &root,
            Vendor::Dhan,
            "NSE",
            "INDEX",
            SYMBOL,
            Timeframe::MINUTE_1,
            None,
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
        let first = window(
            &root,
            Vendor::Dhan,
            "NSE",
            "INDEX",
            SYMBOL,
            Timeframe::MINUTE_1,
            None,
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
            first.bars[0].chg_why, "first_bar_in_file",
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

        let got = window(
            &root,
            Vendor::Dhan,
            "NSE",
            "INDEX",
            SYMBOL,
            Timeframe::MINUTE_1,
            None,
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
        let got = window(
            &root,
            Vendor::Dhan,
            "NSE",
            "INDEX",
            SYMBOL,
            Timeframe::MINUTE_1,
            None,
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
                segment: "INDEX",
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
        window(
            root,
            Vendor::Dhan,
            "NSE",
            "INDEX",
            SYMBOL,
            Timeframe::MINUTE_1,
            None,
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
        window(
            root,
            Vendor::Dhan,
            "NSE",
            "INDEX",
            SYMBOL,
            Timeframe::MINUTE_1,
            None,
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
                assert_eq!(change_at(&page.bars, 1_000), (None, "first_bar_in_file"));
                assert_eq!(
                    page.bars
                        .iter()
                        .filter(|row| row.chg_why == "first_bar_in_file")
                        .count(),
                    1
                );
            }
        }
        // And the seek path agrees that January's first row has none.
        let page = window_over(&root, april, SortKey::Ts, false, 0, 1, false);
        assert_eq!(change_at(&page.bars, 1_000), (None, "first_bar_in_file"));

        let _ = std::fs::remove_dir_all(&root);
    }

    /// **AN EMPTY MONTH FILE IS SKIPPED BY THE LOOKBACK, IN BOTH DIRECTIONS.**
    ///
    /// A month whose file exists and holds no record has no last bar, so the
    /// next month's first row stands behind the month before the empty one.
    /// `earlier_in_time` keeps a file only when `n_valid > 0`; counting an
    /// empty file as "earlier" gave April's first row `first_bar_in_file`
    /// mid-window. Ascending and descending seek pages both reach it, one per
    /// branch of `earlier_in_time`. G18-api-03.
    #[test]
    fn an_empty_month_file_between_two_months_does_not_reset_the_lookback() {
        let root = scratch("cross-month-empty");
        write_month(&root, YearMonth::new(2026, 2).expect("m"), 10, 2_000);
        // March's file EXISTS and is empty: zero valid records.
        write_month(&root, YearMonth::new(2026, 3).expect("m"), 0, 3_000);
        write_month(&root, YearMonth::new(2026, 4).expect("m"), 10, 4_000);
        let april = YearMonth::new(2026, 4).expect("m");
        let feb_to_apr = crate::server::basis_points(2_009, 4_000).expect("a change");
        let opening = |bars: &[WindowBar]| {
            let rows = bars
                .iter()
                .filter(|row| row.bar.close == 4_000)
                .map(|row| (row.chg, row.chg_why))
                .collect::<Vec<_>>();
            assert_eq!(rows.len(), 1, "April's first row is on the page once");
            rows[0]
        };
        // Ascending: offset 10 is April's first row, the page opens on it.
        let up = window_over(&root, april, SortKey::Ts, false, 10, 2, false);
        assert_eq!(up.bars[0].bar.close, 4_000);
        assert_eq!(opening(&up.bars), (Some(feb_to_apr), ""));
        // Descending: offsets 8 and 9 are 4001 and 4000.
        let down = window_over(&root, april, SortKey::Ts, true, 8, 2, false);
        assert_eq!(
            down.bars
                .iter()
                .map(|row| row.bar.close)
                .collect::<Vec<_>>(),
            vec![4_001, 4_000]
        );
        assert_eq!(opening(&down.bars), (Some(feb_to_apr), ""));
        let _ = std::fs::remove_dir_all(&root);
    }

    /// **THE `records unreadable` LINE NAMES THE FIRST FILE THAT REFUSED A
    /// RECORD, NOT THE FIRST FILE READ.** January reads clean and February is
    /// damaged: on the seek path the line names February's file. G18-api-27.
    #[test]
    fn the_unreadable_line_names_the_first_damaged_file_not_the_first_file() {
        let root = scratch("first-faulted");
        let jan = YearMonth::new(2026, 1).expect("m");
        let feb = YearMonth::new(2026, 2).expect("m");
        write_month(&root, jan, 10, 1_000);
        write_month(&root, feb, 10, 2_000);
        damage_record(&root, feb, 5);
        let path_of = |month| {
            open_classified(
                &root,
                PathParts {
                    vendor: Vendor::Dhan,
                    exchange: "NSE",
                    segment: "INDEX",
                    symbol: SYMBOL,
                    contract: None,
                    timeframe: Timeframe::MINUTE_1,
                    month,
                    file: FileKind::Bars,
                },
            )
            .map_err(|why| why.message)
            .expect("the month opens")
            .path()
            .display()
            .to_string()
        };
        let (jan_path, feb_path) = (path_of(jan), path_of(feb));
        let from = crate::emitted::mark();
        let page = window_over(&root, feb, SortKey::Ts, false, 0, 20, false);
        assert!(
            !page.faults.is_empty(),
            "February's damage is named on the page"
        );
        let mine: Vec<telemetry::Record> =
            crate::emitted::landed(from, "api.bars", "records unreadable")
                .into_iter()
                .filter(|record| {
                    crate::emitted::says(record, "file", &feb_path)
                        || crate::emitted::says(record, "file", &jan_path)
                })
                .collect();
        assert_eq!(mine.len(), 1, "one line for the request: {mine:?}");
        assert!(
            crate::emitted::says(&mine[0], "file", &feb_path),
            "{mine:?}"
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
        // `page`, the seek branch of `window`, and `read_in_time`: three call
        // sites, each after its loop, plus the definition.
        assert_eq!(code.matches("note_unreadable_records(").count(), 4);
        // `read_in_time` is reached only from `window`, once, so its call is
        // still one per request (P2-01-04, D-1766).
        assert_eq!(code.matches("read_in_time(").count(), 2);
        assert!(!code.contains("[`read_page`]"));
        let limits = include_str!("../../../docs/06-limits.md");
        assert!(
            limits.contains(
                "has three call sites — `page`, `window`'s seek branch, and `read_in_time`"
            )
        );
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
            segment: "INDEX",
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
            first.faults.len(),
            7,
            "records 73..80 are named: {:?}",
            first.faults
        );

        for desc in [false, true] {
            let mut seen: Vec<i64> = Vec::new();
            let mut faults = 0usize;
            for offset in (0..200).step_by(10) {
                let got = ts_page(&root, feb, desc, offset, 10);
                assert!(got.bars.len() <= 10);
                seen.extend(got.bars.iter().map(|r| r.bar.close));
                faults += got.faults.len();
            }
            let mut expected: Vec<i64> = (1_000..1_073).chain(2_000..2_100).collect();
            if desc {
                expected.reverse();
            }
            assert_eq!(
                seen, expected,
                "desc={desc}: each readable row once, in order"
            );
            assert_eq!(faults, 27, "desc={desc}: each unreadable record named once");
        }

        let _ = std::fs::remove_dir_all(&root);
    }

    /// **A SORTED PAGE DOES NOT MEASURE A ROW ACROSS AN UNREADABLE BLOCK.**
    ///
    /// The reading path folded the change over the records that read, so the
    /// first row after a damaged block was measured against the last row BEFORE
    /// it — a real-looking number against a bar that is not its predecessor
    /// (W1-api1-10, D-0730).
    #[test]
    fn a_sorted_page_names_the_row_after_a_damaged_block_rather_than_measuring_across_it() {
        let root = scratch("scan-damaged");
        let jan = YearMonth::new(2026, 1).expect("m");
        write_month(&root, jan, 200, 1_000);
        // Records 73..146 are one checksum block; 0..73 and 146..200 read.
        damage_record(&root, jan, 100);

        let got = window(
            &root,
            Vendor::Dhan,
            "NSE",
            "INDEX",
            SYMBOL,
            Timeframe::MINUTE_1,
            None,
            jan,
            jan,
            SortKey::Close,
            false,
            73,
            2,
            false,
        )
        .expect("a legal window");
        let closes: Vec<i64> = got.bars.iter().map(|r| r.bar.close).collect();
        assert_eq!(closes, [1_146, 1_147], "the 74th smallest readable close");
        assert_eq!(
            got.bars[0].chg, None,
            "its predecessor, record 145, did not read"
        );
        assert_eq!(got.bars[0].chg_why, "previous_unreadable");
        assert!(
            got.bars[1].chg.is_some(),
            "record 146 read, so 147 is measured"
        );
        assert_eq!(got.faults.len(), 73, "every unreadable record is named");

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
        let rows = with_change(
            Behind::Unreadable,
            vec![
                Some(bar(100, 50)),
                Some(bar(110, 55)),
                None,
                Some(bar(120, 60)),
            ],
        );
        assert_eq!(rows.len(), 3, "an unreadable slot yields no row");
        for row in [&rows[0], &rows[2]] {
            assert_eq!((row.chg, row.chg_why), (None, "previous_unreadable"));
            assert_eq!((row.oichg, row.oichg_why), (None, "previous_unreadable"));
        }
        assert_eq!(rows[1].chg, Some(1_000), "110 against 100 is +10%");
        assert_eq!(rows[1].oichg, Some(1_000), "55 against 50 is +10%");

        let first = with_change(Behind::Nothing, vec![Some(bar(100, 50))]);
        assert_eq!(first[0].chg_why, "first_bar_in_file");
        assert_eq!(first[0].oichg_why, "first_bar_in_file");
    }

    /// **A zero base and a negative base are named apart.** Gap-audit #13,
    /// D-3685: a corrupt negative previous close or open interest was reported
    /// as "is zero".
    #[test]
    fn a_negative_base_is_not_called_zero() {
        let bar = |close: i64, open_interest: i64| Bar {
            ts_micros: 0,
            open: close,
            high: close,
            low: close,
            close,
            volume: 0,
            open_interest,
        };
        let rows = with_change(
            Behind::Nothing,
            vec![
                Some(bar(0, 0)),
                Some(bar(-100, -1)),
                Some(bar(i64::MIN + 1, i64::MIN + 1)),
                Some(bar(100, 10)),
            ],
        );
        assert_eq!(rows[1].chg_why, "previous_close_zero");
        assert_eq!(rows[1].oichg_why, "previous_oi_zero");
        assert_eq!(rows[2].chg_why, "previous_close_negative");
        assert_eq!(rows[2].oichg_why, "previous_oi_negative");
        assert_eq!(rows[3].chg_why, "previous_close_negative");
        assert_eq!(rows[3].oichg_why, "previous_oi_negative");
        assert!(rows.iter().all(|row| row.chg.is_none()));
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
                segment: "INDEX",
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
        assert_eq!((rows.len(), faults.len()), (127, 73));

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
