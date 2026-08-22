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
        refusal
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
        let refusal = why.to_string();
        note_refused(
            vendor, exchange, segment, symbol, timeframe, month, &refusal,
        );
        refusal
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
/// **What IS bounded here, structurally:** this function has exactly one call
/// site — [`read_page`], outside every loop — and it returns on the first line
/// when `faults` is empty and otherwise emits once. So the real cost is at most
/// one event per request regardless of how many records a file refuses, which is
/// the property the shape was chosen for.
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
            Ok(bar) => rows.push(bar),
            // COUNTED IN THE `Vec`, NOT EMITTED HERE. This arm is inside the
            // per-record loop; a line placed at it would be bounded by the data
            // and not by the request. The aggregate is one call below.
            Err(why) => faults.push(format!("record {index}: {why}")),
        }
    }
    note_unreadable_records(file, &faults, rows.len(), skip);
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
    /// Open interest, nulls last.
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
    /// `OI_NULL` is mapped to `i64::MIN` — which it already is — so a null open
    /// interest sorts as the smallest value rather than as a real number. Zero
    /// means zero here exactly as `CLAUDE.md` §7 says it does.
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
    /// The rows asked for.
    pub bars: Vec<Bar>,
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
fn extremes_of(bars: &[Bar]) -> Extremes {
    bars.iter().fold(Extremes::default(), |mut top, bar| {
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
fn seek_page(
    files: &[BarFile],
    desc: bool,
    offset: usize,
    limit: usize,
) -> (Vec<Bar>, Vec<String>) {
    let mut bars = Vec::with_capacity(limit.min(PAGE_BARS));
    let mut faults = Vec::new();
    let mut seen = 0usize;
    for file in files {
        if bars.len() >= limit {
            break;
        }
        let held = usize::try_from(file.header().n_valid).unwrap_or(usize::MAX);
        let lo = seen;
        seen = seen.saturating_add(held);
        if seen <= offset {
            continue;
        }
        let skip_in_file = offset.saturating_sub(lo);
        let take = limit.saturating_sub(bars.len());
        let (mut got, mut bad) = if desc {
            let end = held.saturating_sub(skip_in_file);
            let start = end.saturating_sub(take);
            let (mut rows, bad) = page(file, start, end.saturating_sub(start));
            rows.reverse();
            (rows, bad)
        } else {
            page(file, skip_in_file, take)
        };
        bars.append(&mut got);
        faults.append(&mut bad);
    }
    (bars, faults)
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
/// here rather than by moving every row to a browser.
///
/// `want_extremes` forces the reading path either way, because the widest range
/// in a window is not knowable from a header.
///
/// # Errors
///
/// The range being inverted or too long, or every named month failing to open
/// for a reason other than absence.
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

    /* OPENED ONCE, HELD FOR THE REQUEST. A month with no file is counted and
    skipped: a sparse store is legal and the count is what tells a reader a
    gap from a refusal. */
    let mut files = Vec::with_capacity(ordered.len());
    let mut missing = 0usize;
    for month in ordered {
        match open(
            store_root, vendor, exchange, segment, symbol, timeframe, month, contract,
        ) {
            Ok(file) => files.push(file),
            Err(_) => missing = missing.saturating_add(1),
        }
    }
    if files.is_empty() {
        return Ok(Window {
            total: 0,
            months_read: 0,
            months_missing: missing,
            bars: Vec::new(),
            faults: Vec::new(),
            extremes: want_extremes.then(Extremes::default),
        });
    }

    let total: u64 = files
        .iter()
        .map(|f| f.header().n_valid)
        .fold(0u64, u64::saturating_add);

    if !sort.scans() && !want_extremes {
        let (bars, faults) = seek_page(&files, desc, offset, limit);
        return Ok(Window {
            total,
            months_read: files.len(),
            months_missing: missing,
            bars,
            faults,
            extremes: None,
        });
    }

    /* ---- THE READING PATH. One pass, then order, then slice. ---- */
    let mut all = Vec::with_capacity(usize::try_from(total).unwrap_or(0));
    let mut faults = Vec::new();
    for file in &files {
        let held = usize::try_from(file.header().n_valid).unwrap_or(usize::MAX);
        let (mut rows, mut bad) = page(file, 0, held);
        all.append(&mut rows);
        faults.append(&mut bad);
    }

    let extremes = want_extremes.then(|| extremes_of(&all));

    /* A TOTAL ORDER, SO THE PAGE BOUNDARY IS STABLE. Two bars with the same
    close must not swap between one request and the next, or a reader paging
    through them would see one row twice and another never. The timestamp is
    unique within a series, so it is the tie-break and it is NOT inverted
    with the direction. */
    all.sort_by(|a, b| {
        let (x, y) = (sort.of(a), sort.of(b));
        let primary = if desc { y.cmp(&x) } else { x.cmp(&y) };
        primary.then_with(|| a.ts_micros.cmp(&b.ts_micros))
    });

    let bars = all
        .into_iter()
        .skip(offset)
        .take(limit)
        .collect::<Vec<Bar>>();

    Ok(Window {
        total,
        months_read: files.len(),
        months_missing: missing,
        bars,
        faults,
        extremes,
    })
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
        let closes: Vec<i64> = got.bars.iter().map(|b| b.close).collect();
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

        let closes: Vec<i64> = got.bars.iter().map(|b| b.close).collect();
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
        let closes: Vec<i64> = got.bars.iter().map(|b| b.close).collect();
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
}
