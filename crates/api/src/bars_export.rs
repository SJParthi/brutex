//! `/bars/export.csv` -- every row a `/db` bar query keeps, as ONE streamed
//! CSV answer from the server (R3-10 item 4; DB-047, D-3326).
//!
//! # Why the server writes it
//!
//! The whole-query export was the browser's own: it asked
//! `/bars/window.json` for every page of the query, a thousand rows at a time,
//! held every row in the tab's memory, and only then wrote the file. A query at
//! the ceiling was twenty requests the page waited on one after another, the
//! whole set in the heap before the first byte was saved, and a Cancel that
//! never reached the server. This route is the same query, read by the same
//! window reader one page at a time, and written as it is read.
//!
//! # The contract
//!
//! * THE QUERY IS THE WINDOW'S. Every parameter `/bars/window.json` reads to
//!   name WHICH rows -- feed, timeframe, series (`key=` or `exchange`,
//!   `segment`, `symbol`, `contract`), months, days, time of day, order,
//!   lookback -- is read here by the same parser (`bars_route::WindowAsk`), so
//!   an export and the grid cannot mean two different queries. The parameters
//!   that name a PAGE of it ([`PAGE_ONLY`]) are refused by name: an export
//!   writes the whole query or nothing.
//! * THE SAME CEILING. At most [`EXPORT_CEILING`] rows, the browser's
//!   `EXPORT_CAP` (`web/src/lib/db/bars-plan.js`; a test holds the two
//!   equal). A larger query is refused with its count BEFORE a byte is
//!   written: a file cut at the ceiling would look like the whole answer.
//! * THE SAME COLUMNS, in the same order and words as the browser's bar CSV
//!   (`BAR_CSV_COLS` in `web/src/lib/db/export.js`; a test holds them equal),
//!   each row the value the grid shows or the empty field beside its reason.
//! * STREAMED, IN CONSTANT MEMORY. The server holds one page ([`EXPORT_PAGE`]
//!   rows, about a quarter of a megabyte of text) at a time. The next page is
//!   read only when the connection has taken the last one, so a slow reader
//!   holds the server back instead of filling it.
//! * CANCELLABLE. The page being read runs under the window route's own
//!   cancellation flag (`bars_route::cancellable`, DB-N15). When the client
//!   goes away the answer's body is dropped, the page future with it, the flag
//!   is set and the read stops at its next look; no later page is asked for.
//!   One `api.bars` line says how many rows were written before it stopped.
//! * NEVER SILENTLY SHORT. `x-export-rows` names the count before the first
//!   row. When the query's total changes between two pages (an import wrote
//!   into it), when a page names a record that would not read, or when a page
//!   is refused, the stream ends in an ERROR: the chunked answer is cut
//!   without its last chunk, so the download FAILS instead of saving a short
//!   file, and one `api.bars` warning says why (`CLAUDE.md` section 4).
//!
//! # Option rows: refused until their cells are served in one request
//!
//! The browser's file carries each option row's spot, moneyness, intrinsic,
//! extrinsic, IV and greeks, which it asks of `/spot.json` and `/greeks.json`.
//! Serving those for a page in one request is the other half of R3-10 and is
//! parked for step 2 (operator, 18 Oct 2026: F&O stocks and indices first;
//! DB-B49 and SC-CLOSE-14 stay open with owner "wave 2"). Until then a query
//! naming an option contract is refused `501 option_cells_not_streamed`, so
//! the browser keeps its own path for it: a file with those twelve columns
//! empty would be the quiet degradation `CLAUDE.md` section 4 bans. A future
//! has no option cells and streams like a spot series.
//!
//! # Cost
//!
//! Bounded, not constant, and named for `docs/06-limits.md`: one export is at
//! most `EXPORT_CEILING / EXPORT_PAGE` = 20 page reads plus 2 (the first
//! page is read before the answer starts; when it is not the whole export, two
//! one-row seeks find the first and last IST day the file holds, for its name,
//! and must count the same rows the first page did). A query of one page is
//! one read. Each page read is exactly
//! the window route's read of that page -- a seek on `sort=ts`, a scan bounded
//! by `bars::MAX_SCAN_BARS` on a value sort -- admitted by the same two pools.

use super::Loaded;
use super::bars_route::{WindowAsk, cancellable};
use crate::{bars, render};
use axum::body::Bytes;
use axum::http::{HeaderName, HeaderValue, StatusCode, header};
use axum::response::IntoResponse as _;
use std::fmt::Write as _;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

/// The route this module serves.
pub(super) const ROUTE: &str = "/bars/export.csv";

/// The most rows one export writes: the browser's `EXPORT_CAP` in
/// `web/src/lib/db/bars-plan.js`, which a test reads and holds equal.
pub(super) const EXPORT_CEILING: u64 = 20_000;

/// Rows read and written per page: the window route's own page ceiling, so
/// one page of an export costs exactly what one page of the grid costs.
pub(super) const EXPORT_PAGE: usize = bars::MAX_WINDOW_LIMIT;

/// How many times one page read is asked again when its pool stays full for
/// the pool's whole admission wait (`crate::detail::ADMISSION_WAIT_MS` each).
/// Past it the page is refused by name: a 429 before the answer starts, a
/// stream cut with a warning after.
const ADMISSION_TRIES: u32 = 8;

/// The parameters that name a page of a query rather than the query.
pub(super) const PAGE_ONLY: [&str; 5] = ["offset", "limit", "at", "extremes", "coverage"];

/// The columns, in the browser's order and words (`BAR_CSV_COLS`).
pub(super) const COLUMNS: [&str; 35] = [
    "feed",
    "instrument",
    "month",
    "timeframe",
    "ts_micros",
    "ist_day",
    "ist_time",
    "open_paisa",
    "high_paisa",
    "low_paisa",
    "close_paisa",
    "volume_shares",
    "volume_why",
    "chg_bps",
    "chg_why",
    "open_interest_shares",
    "open_interest_why",
    "oi_chg_bps",
    "oi_chg_why",
    "expiry",
    "days_to_expiry",
    "side",
    "strike_paisa",
    "spot_paisa",
    "moneyness",
    "intrinsic_paisa",
    "extrinsic_paisa",
    "spot_why",
    "iv",
    "delta",
    "gamma",
    "theta",
    "vega",
    "rho",
    "greeks_why",
];

/// The option columns at the end of a row, empty on a row that is not an
/// option: `spot_paisa` to `greeks_why`.
const OPTION_COLUMNS: usize = 12;

/// The telemetry target every export line is written under: the bar routes'.
const TARGET: &str = "api.bars";

/// IST is UTC + 5:30.
const IST_OFFSET_S: i64 = 19_800;

/// One series' contract columns, read once from its key: `expiry`, its day
/// number, `side` and `strike_paisa`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct ContractColumns {
    /// `YYYY-MM-DD`, or empty for a spot series.
    pub(super) expiry: String,
    /// The expiry's days since 1970-01-01, for `days_to_expiry`.
    pub(super) expiry_day: Option<i64>,
    /// `CE`, `PE`, or empty.
    pub(super) side: String,
    /// The strike in paisa as the key spells it, or empty.
    pub(super) strike: String,
}

impl ContractColumns {
    /// The columns of `contract`, or the empty ones for a spot series.
    pub(super) fn of(contract: Option<brutex_core::instrument::Contract>) -> Self {
        let Some(contract) = contract else {
            return Self::default();
        };
        let text = contract.as_str();
        let expiry = text.get(..10).unwrap_or("").to_owned();
        let number = |range: std::ops::Range<usize>| {
            expiry
                .get(range)
                .and_then(|digits| digits.parse::<u16>().ok())
        };
        let expiry_day = match (number(0..4), number(5..7), number(8..10)) {
            (Some(year), Some(month), Some(day)) => u8::try_from(month)
                .ok()
                .zip(u8::try_from(day).ok())
                .and_then(|(month, day)| pull::session::Day::new(year, month, day).ok())
                .map(|day| i64::from(day.days_from_epoch())),
            _ => None,
        };
        let rest = text.get(11..).unwrap_or("");
        let (strike, side) = rest.split_once('-').unwrap_or(("", ""));
        Self {
            expiry,
            expiry_day,
            side: side.to_owned(),
            strike: strike.to_owned(),
        }
    }
}

/// What every row of one export shares: the feed, the rung and each series'
/// contract columns, by the series' index in the query.
#[derive(Debug, Clone)]
pub(super) struct Columns {
    /// The feed's word, `gdfl` or `zerodha`.
    pub(super) feed: &'static str,
    /// The rung's word, `1s`, `1min`, ...
    pub(super) timeframe: String,
    /// Each series' contract columns, in the order the query names them.
    pub(super) contracts: Vec<ContractColumns>,
}

impl Columns {
    /// The columns of `query`.
    pub(super) fn of(query: &bars::WindowQuery) -> Self {
        Self {
            feed: query.vendor.as_str(),
            timeframe: query.timeframe.as_str().to_owned(),
            contracts: query
                .series
                .iter()
                .map(|key| ContractColumns::of(key.contract))
                .collect(),
        }
    }
}

/// The header record.
pub(super) fn header_record() -> String {
    let mut out = COLUMNS.join(",");
    out.push_str("\r\n");
    out
}

/// RFC 4180 quoting, the browser's `csvQuote`: a field holding a quote, a
/// comma or a line break is quoted, and a quote inside is doubled.
fn push_field(out: &mut String, text: &str) {
    if text.contains(['"', ',', '\r', '\n']) {
        out.push('"');
        out.push_str(&text.replace('"', "\"\""));
        out.push('"');
    } else {
        out.push_str(text);
    }
}

/// A number or the empty field, then a comma.
fn push_number(out: &mut String, value: Option<i64>) {
    if let Some(value) = value {
        let _ = write!(out, "{value}");
    }
    out.push(',');
}

/// One window row as one CSV record, appended to `out`: the values the grid
/// shows, each unknown value empty beside its reason, never a zero.
pub(super) fn push_record(
    out: &mut String,
    columns: &Columns,
    instrument: &str,
    row: &bars::WindowBar,
) {
    let bar = &row.bar;
    let ist = bar
        .ts_micros
        .div_euclid(1_000_000)
        .saturating_add(IST_OFFSET_S);
    let day_number = ist.div_euclid(86_400);
    let clock = ist.rem_euclid(86_400);
    let day = bars::ist_day(bar.ts_micros);
    push_field(out, columns.feed);
    out.push(',');
    push_field(out, instrument);
    out.push(',');
    push_field(out, day.get(..7).unwrap_or(&day));
    out.push(',');
    push_field(out, &columns.timeframe);
    let _ = write!(
        out,
        ",{},{day},{:02}:{:02}:{:02},{},{},{},{},",
        bar.ts_micros,
        clock / 3_600,
        (clock % 3_600) / 60,
        clock % 60,
        bar.open,
        bar.high,
        bar.low,
        bar.close,
    );
    // AN INDEX'S VOLUME IS NOT A MEASUREMENT (D-3322): empty, with the reason.
    match row.v_why {
        Some(why) => {
            out.push(',');
            push_field(out, why);
        }
        None => {
            let _ = write!(out, "{},", bar.volume);
        }
    }
    out.push(',');
    // EXACTLY ONE OF EACH PAIR IS FILLED, as on the wire: an unknown change
    // is never `0`.
    push_number(out, row.chg);
    if row.chg.is_none() {
        push_field(out, row.chg_why);
    }
    out.push(',');
    // OPEN INTEREST: not applicable on an index or a share, the null sentinel
    // named, and a zero written as the zero it is.
    if !row.oi_applies {
        out.push_str(",not_a_derivative,");
    } else if bar.open_interest == store::format::OI_NULL {
        out.push_str(",oi_null,");
    } else {
        let _ = write!(out, "{},,", bar.open_interest);
    }
    push_number(out, row.oichg);
    if row.oichg.is_none() {
        push_field(out, row.oichg_why);
    }
    out.push(',');
    let contract = columns.contracts.get(row.series);
    let empty = ContractColumns::default();
    let contract = contract.unwrap_or(&empty);
    push_field(out, &contract.expiry);
    out.push(',');
    push_number(
        out,
        contract
            .expiry_day
            .map(|expiry| expiry.saturating_sub(day_number)),
    );
    push_field(out, &contract.side);
    out.push(',');
    push_field(out, &contract.strike);
    // THE OPTION CELLS: never filled here -- an option query is refused
    // before a row is read (see the module comment), so every row written is
    // one the browser writes these columns empty for too.
    for _ in 0..OPTION_COLUMNS {
        out.push(',');
    }
    out.push_str("\r\n");
}

/// One page, read and written: what the body checks and sends.
#[derive(Debug, Default)]
pub(super) struct Page {
    /// Every row the query keeps, as this read counted it.
    pub(super) total: u64,
    /// Rows written into `csv`.
    pub(super) rows: u64,
    /// The first record or month that would not read, and how many did not.
    pub(super) fault: Option<(String, u64)>,
    /// The earliest and the latest instant on the page (UTC micros), in any
    /// order the page was read in: a page that is the whole export names the
    /// file's days from its own rows, with no further read.
    pub(super) span: Option<(i64, i64)>,
    /// The records.
    pub(super) csv: Bytes,
}

impl Page {
    /// `window`'s rows as CSV records under `columns`.
    pub(super) fn of(window: &bars::Window, columns: &Columns) -> Self {
        let mut csv = String::with_capacity(window.bars.len().saturating_mul(160));
        let mut span: Option<(i64, i64)> = None;
        for row in &window.bars {
            let instrument = window
                .series
                .get(row.series)
                .map_or("", |held| held.key.as_str());
            push_record(&mut csv, columns, instrument, row);
            let at = row.bar.ts_micros;
            span = Some(span.map_or((at, at), |(lo, hi)| (lo.min(at), hi.max(at))));
        }
        let fault = window.faults.first().map(|first| {
            (
                first.clone(),
                window
                    .faulted_records
                    .max(u64::try_from(window.faults.len()).unwrap_or(u64::MAX)),
            )
        });
        Self {
            total: window.total,
            rows: u64::try_from(window.bars.len()).unwrap_or(u64::MAX),
            fault,
            span,
            csv: Bytes::from(csv),
        }
    }

    /// The first and last IST day the page holds, when it holds a row.
    pub(super) fn days(&self) -> Option<(String, String)> {
        self.span
            .map(|(lo, hi)| (bars::ist_day(lo), bars::ist_day(hi)))
    }
}

/// Why a read produced no page.
#[derive(Debug)]
pub(super) enum Refused {
    /// The pool stayed full for every try, or the task would not join.
    Pool(crate::detail::RunError, bool),
    /// The reader's own refusal sentence.
    Read(String),
}

impl Refused {
    /// The sentence a stream cut by it is logged with.
    fn sentence(&self) -> String {
        match self {
            Self::Pool(why, heavy) => format!(
                "the {} pool did not admit the next page after {ADMISSION_TRIES} tries ({why:?})",
                if *heavy { "scan" } else { "store-read" }
            ),
            Self::Read(why) => why.clone(),
        }
    }
}

/// Reads one window of `query` the way `/bars/window.json` reads it -- the
/// cheap pool first, the scan pool when the plan says it needs one -- and
/// shapes it with `shape` off the async workers, under the request's
/// cancellation flag. A full pool is asked again up to [`ADMISSION_TRIES`].
///
/// # Errors
///
/// The pool's refusal after every try, or the reader's refusal.
pub(super) async fn read_window<T, F>(
    root: std::path::PathBuf,
    query: bars::WindowQuery,
    shape: F,
) -> Result<T, Refused>
where
    T: Send + 'static,
    F: Fn(&bars::Window) -> T + Send + Sync + 'static,
{
    let (root, query, shape) = (Arc::new(root), Arc::new(query), Arc::new(shape));
    let mut heavy = query.filter.tod.is_some() || query.coverage;
    let mut tries = 0_u32;
    loop {
        let budget = (!heavy).then_some(bars::CHEAP_READ_BUDGET);
        let (root, query, shape) = (Arc::clone(&root), Arc::clone(&query), Arc::clone(&shape));
        let answered = cancellable(heavy, move |cancel: &bars::Cancel| {
            bars::window_admitted(&root, &query, cancel, budget).map(|window| shape(&window))
        })
        .await;
        match answered {
            Ok(Ok(shaped)) => return Ok(shaped),
            Ok(Err(why)) if !heavy && bars::needs_scan(&why) => heavy = true,
            Ok(Err(why)) => return Err(Refused::Read(why)),
            Err(crate::detail::RunError::Saturated)
                if tries.saturating_add(1) < ADMISSION_TRIES =>
            {
                tries = tries.saturating_add(1);
            }
            Err(why) => return Err(Refused::Pool(why, heavy)),
        }
    }
}

/// A page being read.
pub(super) type PageFuture = Pin<Box<dyn Future<Output = Result<Page, Refused>> + Send>>;

/// Reads the page starting at a row offset.
pub(super) type PageReader = Arc<dyn Fn(usize) -> PageFuture + Send + Sync>;

/// The production reader: `query` from `offset`, shaped into a [`Page`].
fn page_reader(
    root: std::path::PathBuf,
    query: bars::WindowQuery,
    columns: Arc<Columns>,
) -> PageReader {
    Arc::new(move |offset: usize| {
        let mut query = query.clone();
        query.offset = Some(offset);
        let columns = Arc::clone(&columns);
        let root = root.clone();
        let page: PageFuture = Box::pin(read_window(root, query, move |window: &bars::Window| {
            Page::of(window, &columns)
        }));
        page
    })
}

/// How an export ended, for its one `api.bars` line.
enum Ended<'a> {
    /// Every row written.
    Whole,
    /// Cut short, and why.
    Short(&'a str),
    /// The client went away; the work stopped.
    Abandoned,
}

/// The streamed body: the first page already read, then one page at a time.
pub(super) struct ExportBody {
    /// Bytes ready to send before any read: the header and the first page.
    ready: Option<Bytes>,
    /// The page being read, when one is.
    pending: Option<PageFuture>,
    /// Reads the page at an offset.
    reader: PageReader,
    /// The rows the query kept when the export began.
    total: u64,
    /// Rows written so far.
    written: u64,
    /// The stream ended: whole, short or abandoned.
    ended: bool,
    /// When the export began, for its line.
    began: std::time::Instant,
}

impl ExportBody {
    /// A body that sends `ready`, then reads from row `written` until `total`
    /// rows are written.
    pub(super) fn new(ready: Bytes, written: u64, total: u64, reader: PageReader) -> Self {
        Self {
            ready: Some(ready),
            pending: None,
            reader,
            total,
            written,
            ended: false,
            began: std::time::Instant::now(),
        }
    }

    /// Writes the export's one line and marks the stream ended.
    fn end(&mut self, how: &Ended<'_>) {
        self.ended = true;
        let ms = u64::try_from(self.began.elapsed().as_millis()).unwrap_or(u64::MAX);
        let event = match how {
            Ended::Whole => telemetry::Event::info(TARGET, "export written"),
            Ended::Short(_) => telemetry::Event::warn(TARGET, "export stopped short"),
            Ended::Abandoned => telemetry::Event::info(TARGET, "export abandoned; work stopped"),
        };
        let mut event = event
            .with("route", telemetry::Value::Str(ROUTE))
            .with("rows", telemetry::Value::Uint(self.written))
            .with("total", telemetry::Value::Uint(self.total))
            .with("ms", telemetry::Value::Uint(ms));
        if let Ended::Short(why) = how {
            event = event.with("why", telemetry::Value::Str(why));
        }
        let _dropped_when_filtered = telemetry::emit(&event);
    }

    /// The page that arrived, checked: its records, or why the stream ends.
    fn accept(&mut self, page: Result<Page, Refused>) -> Result<Bytes, String> {
        let page = page.map_err(|why| why.sentence())?;
        if let Some((first, count)) = page.fault {
            return Err(format!(
                "{count} record(s) or month(s) of this query would not read past row {}, \
                 first: {first}",
                self.written
            ));
        }
        if page.total != self.total {
            return Err(format!(
                "the query's rows changed while it was exported: {} when it began, {} at \
                 row {} (a writer added to or replaced a month it reads); nothing past row {} \
                 was written -- export again",
                self.total, page.total, self.written, self.written
            ));
        }
        let after = self.written.saturating_add(page.rows);
        if page.rows == 0 || after > self.total {
            return Err(format!(
                "the page at row {} held {} row(s) of a query keeping {}",
                self.written, page.rows, self.total
            ));
        }
        self.written = after;
        Ok(page.csv)
    }
}

impl http_body::Body for ExportBody {
    type Data = Bytes;
    type Error = axum::BoxError;

    fn poll_frame(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<http_body::Frame<Self::Data>, Self::Error>>> {
        let this = self.get_mut();
        if let Some(ready) = this.ready.take() {
            return Poll::Ready(Some(Ok(http_body::Frame::data(ready))));
        }
        if this.ended {
            return Poll::Ready(None);
        }
        if this.written >= this.total {
            this.end(&Ended::Whole);
            return Poll::Ready(None);
        }
        let offset = usize::try_from(this.written).unwrap_or(usize::MAX);
        let reader = &this.reader;
        let pending = this.pending.get_or_insert_with(|| reader(offset));
        match pending.as_mut().poll(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(page) => {
                this.pending = None;
                match this.accept(page) {
                    Ok(csv) => Poll::Ready(Some(Ok(http_body::Frame::data(csv)))),
                    Err(why) => {
                        this.end(&Ended::Short(&why));
                        Poll::Ready(Some(Err(why.into())))
                    }
                }
            }
        }
    }

    fn is_end_stream(&self) -> bool {
        self.ended && self.ready.is_none()
    }
}

impl Drop for ExportBody {
    /// The client went away before the last row: the page being read is
    /// dropped with this body, which sets its cancellation flag, and the
    /// export says how far it got.
    fn drop(&mut self) {
        if !self.ended {
            self.pending = None;
            self.end(&Ended::Abandoned);
        }
    }
}

/// A refusal: status, code and sentence, as JSON.
fn refused(status: StatusCode, code: &str, why: &str) -> axum::response::Response {
    (
        status,
        [(header::CONTENT_TYPE, "application/json; charset=utf-8")],
        format!(
            r#"{{"error":{},"code":{}}}"#,
            render::json_string(why),
            render::json_string(code)
        ),
    )
        .into_response()
}

/// The first page-naming parameter `query` sends -- with a value, with an
/// empty one or with none -- or `None`.
fn page_parameter(query: &str) -> Option<&'static str> {
    query.split('&').find_map(|pair| {
        let key = pair.split_once('=').map_or(pair, |(key, _)| key);
        PAGE_ONLY.into_iter().find(|named| *named == key)
    })
}

/// The window query an export reads, or the refusal.
///
/// # Errors
///
/// A page parameter, anything `/bars/window.json` refuses, and an option
/// contract (see the module comment).
pub(super) fn plan_of(
    query: &str,
) -> Result<bars::WindowQuery, (StatusCode, &'static str, String)> {
    if let Some(key) = page_parameter(query) {
        return Err((
            StatusCode::BAD_REQUEST,
            "page_parameter",
            format!(
                "{key:?} names a page of the query; an export writes every row the query keeps, \
                 at most {EXPORT_CEILING}, so it takes no {key}"
            ),
        ));
    }
    let asked =
        WindowAsk::parse(query).map_err(|why| (StatusCode::BAD_REQUEST, "bad_query", why))?;
    for key in &asked.series {
        if key.contract.is_some_and(|contract| contract.is_option()) {
            return Err((
                StatusCode::NOT_IMPLEMENTED,
                "option_cells_not_streamed",
                format!(
                    "{} is an option: its spot, moneyness, IV and greeks columns are not served \
                     by the streamed export yet (R3-10 step 2), and a file with them empty would \
                     not be the file the grid exports",
                    key.text()
                ),
            ));
        }
    }
    let mut query = asked.query();
    query.offset = Some(0);
    query.limit = EXPORT_PAGE;
    query.want_extremes = false;
    query.at = None;
    query.coverage = false;
    Ok(query)
}

/// The IST day of the first row `query` keeps in time order, ascending or
/// descending -- one of the file name's two ends -- and the query's total as
/// that read counted it.
async fn end_day(
    root: std::path::PathBuf,
    query: &bars::WindowQuery,
    desc: bool,
) -> Result<(String, u64), Refused> {
    let mut ends = query.clone();
    ends.sort = bars::SortKey::Ts;
    ends.desc = desc;
    ends.offset = Some(0);
    ends.limit = 1;
    read_window(root, ends, |window: &bars::Window| {
        let day = window
            .bars
            .first()
            .map_or_else(|| "none".to_owned(), |row| bars::ist_day(row.bar.ts_micros));
        (day, window.total)
    })
    .await
}

/// The file's two days, from the reads that found them.
///
/// THE NAME MUST BE THE FILE'S. A page that is the whole export names them
/// from its own rows and nothing else is read. Otherwise each end read counts
/// the query again, and when either count is not the first page's a writer
/// changed the rows between the reads: the name could name a day the file
/// does not hold, so the export is refused `503 rows_changed` with
/// `Retry-After` before a byte, never named wrong.
///
/// # Errors
///
/// The `503` when the reads counted different rows.
pub(super) fn file_days(
    first: &Page,
    ends: Option<((String, u64), (String, u64))>,
) -> Result<(String, String), Box<axum::response::Response>> {
    if first.rows == first.total
        && let Some(days) = first.days()
    {
        return Ok(days);
    }
    let Some(((lo, lo_total), (hi, hi_total))) = ends else {
        return Err(Box::new(refused(
            StatusCode::INTERNAL_SERVER_ERROR,
            "export_not_whole",
            "the first page is not the whole export and its two ends were not read",
        )));
    };
    if lo_total == first.total && hi_total == first.total {
        return Ok((lo, hi));
    }
    let mut answer = refused(
        StatusCode::SERVICE_UNAVAILABLE,
        "rows_changed",
        &format!(
            "the query's rows changed while the export began: {} on its first page, {lo_total} \
             and {hi_total} on the reads of its two ends (a writer added to or replaced a month \
             it reads) -- ask again",
            first.total
        ),
    );
    answer
        .headers_mut()
        .insert(header::RETRY_AFTER, HeaderValue::from_static("1"));
    Err(Box::new(answer))
}

/// The file's name, the browser's `barCsvName`: the feed, the rung, the
/// instrument (or `selection`), the first and last IST day it holds, and how
/// many rows that is -- keys only, so two exports of the same rows share it.
pub(super) fn file_name(
    columns: &Columns,
    query: &bars::WindowQuery,
    first: &str,
    last: &str,
    rows: u64,
) -> String {
    let symbol = query.series.first().map_or("", |key| key.symbol.as_str());
    let what = if !symbol.is_empty() && query.series.iter().all(|key| key.symbol == symbol) {
        symbol
    } else {
        "selection"
    };
    let mut safe = String::with_capacity(what.len());
    let mut dashed = false;
    for c in what.chars() {
        if c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-') {
            safe.push(c);
            dashed = false;
        } else if !dashed {
            safe.push('-');
            dashed = true;
        }
    }
    format!(
        "brutex-db-bars_{}_{}_{safe}_{first}_{last}_{rows}rows.csv",
        columns.feed, columns.timeframe
    )
}

/// What a refused read answers before the stream starts: the window route's
/// statuses, so the page reads the two routes' refusals the same way.
fn refused_read(why: &Refused) -> axum::response::Response {
    match why {
        Refused::Pool(why, heavy) => {
            let (status, body) = crate::detail::admission_refused(
                if *heavy {
                    "a bar export scan"
                } else {
                    "a bar export"
                },
                if *heavy {
                    super::bars_route::MAX_SCAN_CONCURRENT
                } else {
                    crate::detail::MAX_STORE_READ_CONCURRENT
                },
                why,
            );
            (
                status,
                [(header::CONTENT_TYPE, "application/json; charset=utf-8")],
                body,
            )
                .into_response()
        }
        // A SERIES THE DECODER REFUSES IS GONE: 410, as the window says it.
        Refused::Read(why) if pull::gdfl_lineage::is_contract_refused(why) => {
            refused(StatusCode::GONE, "contract_refused", why)
        }
        Refused::Read(why) => {
            match crate::detail::being_written_answer(std::slice::from_ref(why)) {
                Some((status, body)) => (
                    status,
                    [(header::CONTENT_TYPE, "application/json; charset=utf-8")],
                    body,
                )
                    .into_response(),
                None => refused(StatusCode::BAD_REQUEST, "bad_query", why),
            }
        }
    }
}

/// Why the first page refuses the export, or `None` when it may stream.
///
/// A SET THAT IS NOT WHOLE IS NOT EXPORTED: a writer's hold is "ask again"
/// (503, `Retry-After` added by `count_route`), any other unreadable record
/// is named; a query that keeps nothing, or more than the ceiling, is refused
/// with its count before a byte of the file.
fn first_page_refusal(first: &Page) -> Option<axum::response::Response> {
    if let Some((fault, count)) = &first.fault {
        if let Some((status, body)) =
            crate::detail::being_written_answer(std::slice::from_ref(fault))
        {
            return Some(
                (
                    status,
                    [(header::CONTENT_TYPE, "application/json; charset=utf-8")],
                    body,
                )
                    .into_response(),
            );
        }
        return Some(refused(
            StatusCode::INTERNAL_SERVER_ERROR,
            "export_not_whole",
            &format!(
                "{count} record(s) or month(s) of this query would not read, so it is not \
                 exported in part; first: {fault}"
            ),
        ));
    }
    if first.total == 0 {
        return Some(refused(
            StatusCode::NOT_FOUND,
            "no_bar_kept",
            "this query keeps no bar: there is nothing to export",
        ));
    }
    if first.total > EXPORT_CEILING {
        return Some(refused(
            StatusCode::BAD_REQUEST,
            "too_many_rows",
            &format!(
                "this query keeps {} bar(s), past the {EXPORT_CEILING} one export writes -- \
                 narrow the dates, the time of day or the contracts",
                first.total
            ),
        ));
    }
    if first.rows == 0 || first.rows > first.total {
        return Some(refused(
            StatusCode::INTERNAL_SERVER_ERROR,
            "export_not_whole",
            &format!(
                "the first page held {} row(s) of a query keeping {}, so it is not exported",
                first.rows, first.total
            ),
        ));
    }
    None
}

/// The answer's headers: the CSV type, the file's name, its row count and
/// the ceiling, and the first and last IST day it holds.
///
/// # Errors
///
/// The sentence naming a value that is not a header value.
fn export_headers(
    name: &str,
    total: u64,
    first_day: &str,
    last_day: &str,
) -> Result<[(HeaderName, HeaderValue); 7], String> {
    let value = |text: &str| {
        HeaderValue::from_str(text).map_err(|_| format!("{text:?} is not a header value"))
    };
    Ok([
        (
            header::CONTENT_TYPE,
            HeaderValue::from_static("text/csv; charset=utf-8"),
        ),
        (
            header::CONTENT_DISPOSITION,
            value(&format!("attachment; filename=\"{name}\""))?,
        ),
        (header::CACHE_CONTROL, HeaderValue::from_static("no-store")),
        (
            HeaderName::from_static("x-export-rows"),
            HeaderValue::from(total),
        ),
        (
            HeaderName::from_static("x-export-ceiling"),
            HeaderValue::from(EXPORT_CEILING),
        ),
        (
            HeaderName::from_static("x-export-first-day"),
            value(first_day)?,
        ),
        (
            HeaderName::from_static("x-export-last-day"),
            value(last_day)?,
        ),
    ])
}

/// `GET /bars/export.csv?<the window query>`: every row it keeps, streamed.
pub(super) async fn bars_export_csv(
    axum::extract::State(site): axum::extract::State<Loaded>,
    method: axum::http::Method,
    uri: axum::http::Uri,
) -> axum::response::Response {
    let query = match plan_of(uri.query().unwrap_or("")) {
        Ok(query) => query,
        Err((status, code, why)) => return refused(status, code, &why),
    };
    let root = site.store_root.clone();
    let columns = Arc::new(Columns::of(&query));
    let reader = page_reader(root.clone(), query.clone(), Arc::clone(&columns));
    let first = match reader(0).await {
        Ok(first) => first,
        Err(why) => return refused_read(&why),
    };
    if let Some(refusal) = first_page_refusal(&first) {
        return refusal;
    }
    let ends = if first.rows == first.total {
        None
    } else {
        match (
            end_day(root.clone(), &query, false).await,
            end_day(root, &query, true).await,
        ) {
            (Ok(lo), Ok(hi)) => Some((lo, hi)),
            (Err(why), _) | (_, Err(why)) => return refused_read(&why),
        }
    };
    let (lo, hi) = match file_days(&first, ends) {
        Ok(days) => days,
        Err(refusal) => return *refusal,
    };
    let name = file_name(&columns, &query, &lo, &hi, first.total);
    let headers = match export_headers(&name, first.total, &lo, &hi) {
        Ok(headers) => headers,
        Err(why) => return refused(StatusCode::INTERNAL_SERVER_ERROR, "file_name", &why),
    };
    // A HEAD IS ANSWERED THE HEAD ALONE: no body to stream, so no export to
    // log as abandoned.
    if method == axum::http::Method::HEAD {
        return (StatusCode::OK, headers, axum::body::Body::empty()).into_response();
    }
    let mut ready = header_record().into_bytes();
    ready.extend_from_slice(&first.csv);
    let body = ExportBody::new(Bytes::from(ready), first.rows, first.total, reader);
    (StatusCode::OK, headers, axum::body::Body::new(body)).into_response()
}

#[cfg(test)]
#[allow(
    clippy::indexing_slicing,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    reason = "a test that cannot panic cannot fail, and these lints exist to \
              keep panics out of the crate rather than out of its tests"
)]
mod tests {
    use super::*;
    use axum::http::{HeaderMap, Method};
    use brutex_core::vendor::Vendor;
    use http_body::Body as _;
    use pull::session::Day;
    use std::time::Duration;
    use store::format::{Bar, OI_NULL};
    use store::path::{FileKind, PathParts, StorePath, Timeframe, YearMonth};

    /// One-minute bars a fixture session holds, 09:15 to 15:29 IST.
    const SESSION: usize = 375;

    /// A store of generated NIFTY one-minute bars, nothing else.
    struct Store {
        root: std::path::PathBuf,
        site: Loaded,
    }

    impl Store {
        fn new(name: &str) -> Self {
            let root = crate::scratch::path(&format!("bars-export-{name}"));
            let masters = root.join("masters");
            std::fs::create_dir_all(&masters).expect("an owned empty master root");
            let site = Loaded::new(super::super::Site::load(&masters, &root));
            Self { root, site }
        }

        /// Appends `rows` generated bars to one 2025 month of NIFTY under
        /// `segment` (and `contract`), bar `i` of the month at minute
        /// `i % 375` of day `2 + i / 375`, from bar `from`.
        fn write(
            &self,
            segment: &str,
            contract: Option<&str>,
            month: u8,
            from: usize,
            rows: usize,
        ) {
            let path = StorePath::new(PathParts {
                vendor: Vendor::Gdfl,
                exchange: "NSE",
                segment,
                symbol: "NIFTY",
                contract: contract.map(|text| {
                    brutex_core::instrument::Contract::parse(text).expect("a generated contract")
                }),
                timeframe: Timeframe::MINUTE_1,
                month: YearMonth::new(2025, month).expect("a generated month"),
                file: FileKind::Bars,
            })
            .expect("a generated path");
            let hash = brutex_core::universe::fnv1a("NIFTY").to_le_bytes();
            let symbol = u32::from_le_bytes(hash[..4].try_into().expect("low 32 bits"));
            let mut file = store::file::BarFile::open_or_create(&self.root, path, symbol)
                .expect("an owned generated bar writer");
            let first = i64::from(
                Day::new(2025, month, 2)
                    .expect("a generated day")
                    .days_from_epoch(),
            );
            let bars: Vec<Bar> = (from..from + rows)
                .map(|i| {
                    let n = i64::try_from(i).expect("a small index");
                    let session = i64::try_from(SESSION).expect("375");
                    // INVENTED PRICES: whole rupees around 10,000.37, a value
                    // no NIFTY print is mistaken for.
                    let close = 1_000_037 + ((n * 7_919) % 5_003) * 100;
                    Bar {
                        ts_micros: (first + n / session) * 86_400_000_000
                            + 13_500_000_000
                            + (n % session) * 60_000_000,
                        open: close,
                        high: close + 50,
                        low: close - 50,
                        close,
                        volume: 10 + n,
                        open_interest: if segment == "FNO" { 1_000 + n } else { OI_NULL },
                    }
                })
                .collect();
            file.append(&bars).expect("complete generated records");
        }

        /// The export of `query` through the handler. A full pool (another
        /// test's reads) is asked again.
        async fn export(&self, query: &str) -> (StatusCode, HeaderMap, Result<String, String>) {
            self.export_as(Method::GET, query).await
        }

        async fn export_as(
            &self,
            method: Method,
            query: &str,
        ) -> (StatusCode, HeaderMap, Result<String, String>) {
            for _ in 0..400 {
                let uri = format!("{ROUTE}?{query}").parse().expect("a URI");
                let answer = bars_export_csv(
                    axum::extract::State(Loaded::clone(&self.site)),
                    method.clone(),
                    uri,
                )
                .await;
                if answer.status() == StatusCode::TOO_MANY_REQUESTS {
                    tokio::time::sleep(Duration::from_millis(20)).await;
                    continue;
                }
                let (parts, body) = answer.into_parts();
                let body = axum::body::to_bytes(body, usize::MAX)
                    .await
                    .map(|bytes| String::from_utf8(bytes.to_vec()).expect("UTF-8 text"))
                    .map_err(|why| why.to_string());
                return (parts.status, parts.headers, body);
            }
            panic!("the pools stayed full");
        }

        /// One `/bars/window.json` page, as JSON.
        async fn window(&self, query: &str) -> serde_json::Value {
            for _ in 0..400 {
                let uri = format!("/bars/window.json?{query}").parse().expect("a URI");
                let (status, _, body) = super::super::bars_window_json(
                    axum::extract::State(Loaded::clone(&self.site)),
                    uri,
                )
                .await;
                if status == StatusCode::TOO_MANY_REQUESTS {
                    tokio::time::sleep(Duration::from_millis(20)).await;
                    continue;
                }
                assert_eq!(status, StatusCode::OK, "{body}");
                return serde_json::from_str(&body).expect("window JSON");
            }
            panic!("the pools stayed full");
        }
    }

    /// The query every spot fixture is asked with.
    const SPOT: &str = "feed=gdfl&timeframe=1min&exchange=NSE&segment=INDEX&symbol=NIFTY";

    /// A record's fields (no field of these fixtures is quoted).
    fn fields(record: &str) -> Vec<&str> {
        record.split(',').collect()
    }

    /// The records after the header.
    fn records(csv: &str) -> Vec<&str> {
        let mut lines: Vec<&str> = csv.split("\r\n").collect();
        assert_eq!(lines.pop(), Some(""), "every record ends CRLF");
        assert_eq!(lines.first().copied(), Some(header_record().trim_end()));
        lines.split_off(1)
    }

    /// A file of `web/`, read as text: the browser's own statement of a
    /// fact the server must agree with.
    fn web(path: &str) -> String {
        let at = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../web")
            .join(path);
        std::fs::read_to_string(&at).unwrap_or_else(|why| panic!("{}: {why}", at.display()))
    }

    /// The next frame of an export body.
    fn next_frame(
        body: &mut ExportBody,
    ) -> impl Future<Output = Option<Result<http_body::Frame<Bytes>, axum::BoxError>>> + '_ {
        std::future::poll_fn(move |cx| Pin::new(&mut *body).poll_frame(cx))
    }

    /// The export's one line of `message` for `total` rows, from `from`.
    fn line(from: u64, message: &str, total: u64) -> Vec<telemetry::Record> {
        crate::emitted::landed(from, TARGET, message)
            .into_iter()
            .filter(|record| {
                crate::emitted::says(record, "route", ROUTE)
                    && crate::emitted::counts(record, "total", total)
            })
            .collect()
    }

    /// THE NAMED TEST. The page being read when the client goes away is
    /// stopped by its cancellation flag, no later page is asked for, and one
    /// line says how far the export got. The reader is the window route's own
    /// `cancellable`, so the flag is the one every bar read honours.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_streamed_export_stops_when_cancelled() {
        let asked = Arc::new(std::sync::Mutex::new(Vec::<usize>::new()));
        let (started_tx, started_rx) = std::sync::mpsc::channel::<()>();
        let (saw_tx, saw_rx) = std::sync::mpsc::channel::<bool>();
        let reader: PageReader = {
            let asked = Arc::clone(&asked);
            Arc::new(move |offset: usize| {
                asked.lock().expect("the offsets asked").push(offset);
                let (started, saw) = (started_tx.clone(), saw_tx.clone());
                let page: PageFuture = Box::pin(async move {
                    loop {
                        let (started, saw) = (started.clone(), saw.clone());
                        let read = cancellable(false, move |cancel: &bars::Cancel| {
                            let _ = started.send(());
                            let began = std::time::Instant::now();
                            while !cancel.is_cancelled()
                                && began.elapsed() < Duration::from_secs(30)
                            {
                                std::thread::sleep(Duration::from_millis(2));
                            }
                            let _ = saw.send(cancel.is_cancelled());
                            Page::default()
                        })
                        .await;
                        match read {
                            Ok(page) => return Ok(page),
                            Err(crate::detail::RunError::Saturated) => {
                                tokio::time::sleep(Duration::from_millis(20)).await;
                            }
                            Err(why) => return Err(Refused::Pool(why, false)),
                        }
                    }
                });
                page
            })
        };
        let from = crate::emitted::mark();
        let mut body = ExportBody::new(Bytes::from_static(b"page one\r\n"), 1_000, 3_000, reader);
        let first = next_frame(&mut body).await.expect("a frame").expect("data");
        assert_eq!(
            first.into_data().expect("bytes"),
            Bytes::from_static(b"page one\r\n")
        );
        // PAGE TWO BEGINS, AND IS STILL BEING READ WHEN THE CLIENT GOES.
        let began = std::time::Instant::now();
        loop {
            let polled =
                tokio::time::timeout(Duration::from_millis(20), next_frame(&mut body)).await;
            assert!(polled.is_err(), "page two answered while its read was held");
            if started_rx.try_recv().is_ok() {
                break;
            }
            assert!(
                began.elapsed() < Duration::from_secs(45),
                "page two never began"
            );
        }
        drop(body);
        assert_eq!(
            saw_rx.recv_timeout(Duration::from_secs(10)),
            Ok(true),
            "the read in flight saw the client go"
        );
        assert_eq!(
            *asked.lock().expect("the offsets asked"),
            vec![1_000],
            "page three was asked for"
        );
        let lines = line(from, "export abandoned; work stopped", 3_000);
        assert_eq!(lines.len(), 1, "{lines:?}");
        assert!(crate::emitted::counts(&lines[0], "rows", 1_000));
        assert_eq!(lines[0].level, telemetry::Level::Info);
    }

    /// The export is the window's pages, in the window's order, row for row
    /// -- across a month boundary, on a time order, a value order and a time
    /// of day -- and a whole one says so once.
    #[tokio::test]
    async fn the_export_is_every_page_of_the_window_in_its_order() {
        let store = Store::new("pages");
        store.write("INDEX", None, 5, 0, 1_300);
        store.write("INDEX", None, 6, 0, 1_200);
        for (order, total) in [
            ("from=2025-05&to=2025-06", 2_500_u64),
            ("from=2025-05&to=2025-06&dir=asc", 2_500),
            ("from=2025-05&to=2025-06&sort=c&dir=asc", 2_500),
            ("from=2025-05&to=2025-06&sort=v", 2_500),
            ("from=2025-05&to=2025-06&tod_from=09:15&tod_to=09:20", 6 * 8),
        ] {
            let from = crate::emitted::mark();
            let query = format!("{SPOT}&{order}");
            let (status, headers, body) = store.export(&query).await;
            assert_eq!(status, StatusCode::OK, "{order}: {body:?}");
            assert_eq!(headers["x-export-rows"], total.to_string().as_str());
            assert_eq!(headers["content-type"], "text/csv; charset=utf-8");
            let body = body.expect("a whole stream");
            let got = records(&body);
            assert_eq!(got.len() as u64, total, "{order}");
            let mut want = Vec::new();
            for offset in (0..total).step_by(EXPORT_PAGE) {
                let page = store
                    .window(&format!("{query}&offset={offset}&limit={EXPORT_PAGE}"))
                    .await;
                assert_eq!(page["total"], total);
                want.extend(page["bars"].as_array().expect("rows").iter().cloned());
            }
            assert_eq!(want.len(), got.len());
            for (record, row) in got.iter().zip(&want) {
                let f = fields(record);
                assert_eq!(f.len(), COLUMNS.len(), "{record}");
                assert_eq!(f[4], (row["t"].as_i64().unwrap() * 1_000_000).to_string());
                assert_eq!(f[10], row["c"].to_string());
                assert_eq!(f[11], "", "an index's volume is not a measurement");
                assert_eq!(f[12], "index_no_volume");
                match row["chg"].as_i64() {
                    Some(bps) => assert_eq!((f[13], f[14]), (bps.to_string().as_str(), "")),
                    None => assert_eq!((f[13], f[14]), ("", row["chg_why"].as_str().unwrap())),
                }
            }
            let written = line(from, "export written", total);
            assert_eq!(written.len(), 1, "{order}: {written:?}");
            assert!(crate::emitted::counts(&written[0], "rows", total));
        }
    }

    /// THE CEILING IS THE BROWSER'S: a query of exactly that many rows is
    /// written whole, one more is refused with its count before a byte.
    #[tokio::test]
    async fn the_ceiling_is_the_browsers_and_a_larger_query_is_refused_before_a_byte() {
        let plan = web("src/lib/db/bars-plan.js");
        let cap = plan
            .lines()
            .find_map(|line| line.strip_prefix("export const EXPORT_CAP = "))
            .expect("the browser's EXPORT_CAP");
        assert_eq!(
            cap.trim_end_matches(';').replace('_', ""),
            EXPORT_CEILING.to_string()
        );
        let store = Store::new("ceiling");
        store.write("INDEX", None, 5, 0, 10_000);
        store.write("INDEX", None, 6, 0, 10_000);
        store.write("INDEX", None, 7, 0, 1);
        let (status, headers, body) = store
            .export(&format!("{SPOT}&from=2025-05&to=2025-06"))
            .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(headers["x-export-ceiling"], "20000");
        assert_eq!(records(&body.expect("a whole stream")).len(), 20_000);
        let (status, headers, body) = store
            .export(&format!("{SPOT}&from=2025-05&to=2025-07"))
            .await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert!(headers.get("x-export-rows").is_none());
        let body: serde_json::Value = serde_json::from_str(&body.expect("a body")).expect("JSON");
        assert_eq!(body["code"], "too_many_rows");
        assert!(
            body["error"]
                .as_str()
                .unwrap()
                .contains("keeps 20001 bar(s), past the 20000")
        );
    }

    /// The header is the browser's bar CSV header, word for word.
    #[test]
    fn the_columns_are_the_browsers_bar_csv_columns() {
        let export = web("src/lib/db/export.js");
        let (_, cols) = export
            .split_once("export const BAR_CSV_COLS = [")
            .expect("the browser's BAR_CSV_COLS");
        let (cols, _) = cols.split_once("];").expect("its end");
        let labels: Vec<&str> = cols
            .split("label: '")
            .skip(1)
            .map(|rest| rest.split_once('\'').expect("a quoted label").0)
            .collect();
        assert_eq!(labels, COLUMNS);
        assert_eq!(header_record(), format!("{}\r\n", COLUMNS.join(",")));
        assert_eq!(COLUMNS.len() - OPTION_COLUMNS, 23);
        assert_eq!(COLUMNS[23], "spot_paisa");
    }

    /// A page parameter, a garbage value and a repeated key are refused by
    /// name, before any read.
    #[tokio::test]
    async fn a_page_parameter_or_a_garbage_one_is_refused_by_name() {
        let store = Store::new("refusals");
        store.write("INDEX", None, 5, 0, 3);
        for (extra, code, needle) in [
            ("&offset=0", "page_parameter", "\"offset\" names a page"),
            ("&limit=5", "page_parameter", "\"limit\""),
            ("&limit", "page_parameter", "\"limit\""),
            ("&at=1746157500", "page_parameter", "\"at\""),
            ("&extremes=1", "page_parameter", "\"extremes\""),
            ("&coverage=0", "page_parameter", "\"coverage\""),
            ("&dir=ASC", "bad_query", "is not a direction"),
            ("&sort=banana", "bad_query", "is not a column"),
            ("&tod_from=25:00", "bad_query", "IST time of day"),
            (
                "&from_day=2025-05-09&to_day=2025-05-02",
                "bad_query",
                "from_day must be",
            ),
        ] {
            let (status, _, body) = store
                .export(&format!("{SPOT}&from=2025-05&to=2025-05{extra}"))
                .await;
            assert_eq!(status, StatusCode::BAD_REQUEST, "{extra}");
            let body: serde_json::Value =
                serde_json::from_str(&body.expect("a body")).expect("JSON");
            assert_eq!(body["code"], code, "{extra}: {body}");
            assert!(
                body["error"].as_str().unwrap().contains(needle),
                "{extra}: {body}"
            );
        }
        let (status, _, body) = store
            .export("feed=banana&timeframe=1min&exchange=NSE&segment=INDEX&symbol=NIFTY&from=2025-05&to=2025-05")
            .await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert!(body.unwrap().contains("is not a feed"));
        let (status, _, body) = store
            .export("feed=gdfl&timeframe=1min&key=NSE-INDEX-NIFTY&key=NSE-INDEX-NIFTY&from=2025-05&to=2025-05")
            .await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert!(body.unwrap().contains("is named twice"));
    }

    /// An option is refused until its cells stream; a future streams with its
    /// expiry and its days to it, and its open interest.
    #[tokio::test]
    async fn an_option_is_refused_until_its_cells_stream_and_a_future_streams_its_expiry() {
        let store = Store::new("contracts");
        store.write("FNO", Some("2025-06-26-FUT"), 6, 0, 2);
        store.write("FNO", Some("2025-06-26-2450000-CE"), 6, 0, 2);
        let (status, _, body) = store
            .export("feed=gdfl&timeframe=1min&exchange=NSE&segment=FNO&symbol=NIFTY&contract=2025-06-26-2450000-CE&from=2025-06&to=2025-06")
            .await;
        assert_eq!(status, StatusCode::NOT_IMPLEMENTED);
        assert!(body.unwrap().contains("option_cells_not_streamed"));
        let (status, _, body) = store
            .export("feed=gdfl&timeframe=1min&key=NSE-FNO-NIFTY-2025-06-26-FUT&key=NSE-FNO-NIFTY-2025-06-26-2450000-CE&from=2025-06&to=2025-06")
            .await;
        assert_eq!(
            status,
            StatusCode::NOT_IMPLEMENTED,
            "one option among the keys"
        );
        assert!(body.unwrap().contains("option_cells_not_streamed"));
        let (status, headers, body) = store
            .export("feed=gdfl&timeframe=1min&exchange=NSE&segment=FNO&symbol=NIFTY&contract=2025-06-26-FUT&from=2025-06&to=2025-06&dir=asc")
            .await;
        assert_eq!(status, StatusCode::OK, "{body:?}");
        assert_eq!(
            headers["content-disposition"],
            "attachment; filename=\"brutex-db-bars_gdfl_1min_NIFTY_2025-06-02_2025-06-02_2rows.csv\""
        );
        let body = body.unwrap();
        let rows = records(&body);
        let first = fields(rows[0]);
        assert_eq!(first[1], "NSE-FNO-NIFTY-2025-06-26-FUT");
        assert_eq!(&first[11..13], ["10", ""]);
        assert_eq!(&first[15..17], ["1000", ""], "a future's open interest");
        assert_eq!(&first[19..23], ["2025-06-26", "24", "", ""]);
        assert!(first[23..].iter().all(|cell| cell.is_empty()));
        let second = fields(rows[1]);
        assert_eq!(
            (second[17], second[18]),
            ("10", ""),
            "1,001 against 1,000 is 10 bps"
        );
    }

    /// A query that keeps nothing is refused by name, never an empty file.
    #[tokio::test]
    async fn an_empty_query_is_refused_not_written_as_an_empty_file() {
        let store = Store::new("empty");
        store.write("INDEX", None, 5, 0, 3);
        for query in [
            format!("{SPOT}&from=2025-05&to=2025-05&from_day=2025-05-20&to_day=2025-05-21"),
            format!("{SPOT}&from=2025-08&to=2025-08"),
        ] {
            let (status, headers, body) = store.export(&query).await;
            assert_eq!(status, StatusCode::NOT_FOUND, "{query}: {body:?}");
            assert!(headers.get("content-disposition").is_none());
            assert!(body.unwrap().contains("no_bar_kept"));
        }
    }

    /// One bar is one record, every unknown cell empty beside its reason,
    /// the file named for the day it holds.
    #[tokio::test]
    async fn a_single_bar_is_one_record_with_its_reasons() {
        let store = Store::new("single");
        store.write("INDEX", None, 5, 0, 1);
        let (status, headers, body) = store
            .export(&format!("{SPOT}&from=2025-05&to=2025-05"))
            .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(headers["x-export-first-day"], "2025-05-02");
        assert_eq!(headers["x-export-last-day"], "2025-05-02");
        assert_eq!(headers["cache-control"], "no-store");
        assert_eq!(
            headers["content-disposition"],
            "attachment; filename=\"brutex-db-bars_gdfl_1min_NIFTY_2025-05-02_2025-05-02_1rows.csv\""
        );
        let body = body.unwrap();
        let rows = records(&body);
        assert_eq!(rows.len(), 1);
        let f = fields(rows[0]);
        assert_eq!(
            &f[..7],
            [
                "gdfl",
                "NSE-INDEX-NIFTY",
                "2025-05",
                "1min",
                "1746157500000000",
                "2025-05-02",
                "09:15:00"
            ]
        );
        assert_eq!(&f[7..11], ["1000037", "1000087", "999987", "1000037"]);
        assert_eq!(&f[11..13], ["", "index_no_volume"]);
        assert_eq!(&f[13..15], ["", "no_earlier_stored_bar"]);
        assert_eq!(&f[15..19], ["", "not_a_derivative", "", "not_a_derivative"]);
        assert_eq!(&f[19..23], ["", "", "", ""]);
    }

    /// THE NAME IS THE FILE'S DAYS IN ANY ORDER: a value order puts neither
    /// end first, on one page (named from its own rows) and on several
    /// (named from the two end reads).
    #[tokio::test]
    async fn the_file_names_its_first_and_last_day_in_a_value_order() {
        let store = Store::new("ends");
        store.write("INDEX", None, 5, 0, 1_300);
        for (query, rows, last) in [
            (
                "from_day=2025-05-02&to_day=2025-05-03&sort=c",
                750_u64,
                "2025-05-03",
            ),
            ("sort=c&dir=asc", 1_300, "2025-05-05"),
        ] {
            let (status, headers, body) = store
                .export(&format!("{SPOT}&from=2025-05&to=2025-05&{query}"))
                .await;
            assert_eq!(status, StatusCode::OK, "{query}: {body:?}");
            assert_eq!(headers["x-export-rows"], rows.to_string().as_str());
            assert_eq!(headers["x-export-first-day"], "2025-05-02", "{query}");
            assert_eq!(headers["x-export-last-day"], last, "{query}");
            assert_eq!(
                headers["content-disposition"],
                format!(
                    "attachment; filename=\"brutex-db-bars_gdfl_1min_NIFTY_2025-05-02_{last}_{rows}rows.csv\""
                )
                .as_str()
            );
            assert_eq!(records(&body.unwrap()).len() as u64, rows);
        }
    }

    /// A WRITER BETWEEN THE FIRST PAGE AND THE END READS: the name could name
    /// a day the file does not hold, so the export is refused before a byte,
    /// `503 rows_changed` with `Retry-After`; agreeing counts name the file
    /// from the end reads, and a page that is the whole export from its rows.
    #[test]
    fn the_name_is_refused_when_the_rows_change_between_the_first_page_and_its_ends() {
        let day = |text: &str| {
            let (y, rest) = text.split_at(4);
            let (m, d) = rest[1..].split_at(2);
            Day::new(
                y.parse().unwrap(),
                m.parse().unwrap(),
                d[1..].parse().unwrap(),
            )
            .unwrap()
            .days_from_epoch()
        };
        let at = |text: &str| i64::from(day(text)) * 86_400_000_000 + 13_500_000_000;
        let part = Page {
            total: 2_500,
            rows: 1_000,
            span: Some((at("2025-05-02"), at("2025-05-04"))),
            ..Page::default()
        };
        let ends =
            |lo: u64, hi: u64| Some((("2025-05-02".to_owned(), lo), ("2025-06-07".to_owned(), hi)));
        for (lo, hi) in [(2_500, 2_501), (2_499, 2_500), (0, 0)] {
            let refused = file_days(&part, ends(lo, hi)).expect_err("counts that disagree");
            assert_eq!(refused.status(), StatusCode::SERVICE_UNAVAILABLE);
            assert_eq!(refused.headers()["retry-after"], "1");
        }
        assert_eq!(
            file_days(&part, ends(2_500, 2_500)).expect("counts that agree"),
            ("2025-05-02".to_owned(), "2025-06-07".to_owned()),
            "a page that is not the whole export is named from its end reads"
        );
        assert_eq!(
            file_days(&part, None).expect_err("no end reads").status(),
            StatusCode::INTERNAL_SERVER_ERROR
        );
        let whole = Page {
            total: 1_000,
            rows: 1_000,
            ..part
        };
        assert_eq!(
            file_days(&whole, None).expect("a whole page"),
            ("2025-05-02".to_owned(), "2025-05-04".to_owned()),
            "a page that is the whole export is named from its own rows"
        );
    }

    /// Rows that change between two pages cut the stream with an error and
    /// one warning: the download fails instead of saving a file that is
    /// neither the old query nor the new one.
    #[tokio::test]
    async fn rows_that_change_mid_export_cut_the_stream_with_a_warning() {
        let store = Store::new("changed");
        store.write("INDEX", None, 5, 0, 1_300);
        store.write("INDEX", None, 6, 0, 1_200);
        let query = format!("{SPOT}&from=2025-05&to=2025-06&dir=asc");
        let from = crate::emitted::mark();
        let answer = loop {
            let uri = format!("{ROUTE}?{query}").parse().expect("a URI");
            let answer = bars_export_csv(
                axum::extract::State(Loaded::clone(&store.site)),
                Method::GET,
                uri,
            )
            .await;
            if answer.status() != StatusCode::TOO_MANY_REQUESTS {
                break answer;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        };
        assert_eq!(answer.status(), StatusCode::OK);
        let mut body = answer.into_body();
        let first = std::future::poll_fn(|cx| Pin::new(&mut body).poll_frame(cx))
            .await
            .expect("a frame")
            .expect("the first page");
        assert_eq!(
            first
                .into_data()
                .expect("bytes")
                .split(|b| *b == b'\n')
                .count(),
            1_002
        );
        store.write("INDEX", None, 6, 1_200, 5);
        let mut cut = None;
        while let Some(frame) = std::future::poll_fn(|cx| Pin::new(&mut body).poll_frame(cx)).await
        {
            if let Err(why) = frame {
                cut = Some(why.to_string());
                break;
            }
        }
        let cut = cut.expect("the stream ended in an error");
        assert!(
            cut.contains("2500 when it began, 2505 at row 1000"),
            "{cut}"
        );
        let warned = line(from, "export stopped short", 2_500);
        assert_eq!(warned.len(), 1, "{warned:?}");
        assert_eq!(warned[0].level, telemetry::Level::Warn);
        assert!(crate::emitted::counts(&warned[0], "rows", 1_000));
    }

    /// A month that would not read refuses the export by name: a set that is
    /// not whole is not written in part.
    #[tokio::test]
    async fn a_damaged_month_refuses_the_export() {
        let store = Store::new("damaged");
        store.write("INDEX", None, 5, 0, 3);
        store.write("INDEX", None, 6, 0, 3);
        let june = StorePath::new(PathParts {
            vendor: Vendor::Gdfl,
            exchange: "NSE",
            segment: "INDEX",
            symbol: "NIFTY",
            contract: None,
            timeframe: Timeframe::MINUTE_1,
            month: YearMonth::new(2025, 6).expect("June"),
            file: FileKind::Bars,
        })
        .expect("a path")
        .to_path_buf(&store.root);
        std::fs::write(&june, b"unreadable generated month").expect("an owned month, damaged");
        let (status, headers, body) = store
            .export(&format!("{SPOT}&from=2025-05&to=2025-06"))
            .await;
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR, "{body:?}");
        assert!(headers.get("x-export-rows").is_none());
        let body = body.unwrap();
        assert!(
            body.contains("export_not_whole") && body.contains("2025-06"),
            "{body}"
        );
    }

    /// A MONTH MISSING IN THE MIDDLE is skipped exactly as the window skips
    /// it -- an absent month is an expected answer, not damage -- so the
    /// file is May's rows then July's, in order, and its count says so.
    #[tokio::test]
    async fn a_month_missing_in_the_middle_is_skipped_as_the_window_skips_it() {
        let store = Store::new("gap");
        store.write("INDEX", None, 5, 0, 4);
        store.write("INDEX", None, 7, 0, 3);
        let query = format!("{SPOT}&from=2025-05&to=2025-07&dir=asc");
        let (status, headers, body) = store.export(&query).await;
        assert_eq!(status, StatusCode::OK, "{body:?}");
        assert_eq!(headers["x-export-rows"], "7");
        assert_eq!(headers["x-export-first-day"], "2025-05-02");
        assert_eq!(headers["x-export-last-day"], "2025-07-02");
        let body = body.expect("a whole stream");
        let got = records(&body);
        let months: Vec<&str> = got.iter().map(|record| fields(record)[2]).collect();
        assert_eq!(
            months,
            [
                "2025-05", "2025-05", "2025-05", "2025-05", "2025-07", "2025-07", "2025-07"
            ]
        );
        let window = store.window(&format!("{query}&offset=0&limit=10")).await;
        assert_eq!(window["total"], 7);
        for (record, row) in got.iter().zip(window["bars"].as_array().expect("rows")) {
            assert_eq!(
                fields(record)[4],
                (row["t"].as_i64().unwrap() * 1_000_000).to_string()
            );
        }
    }

    /// A MONTH A WRITER HOLDS IS "ASK AGAIN", never a file in part: the
    /// export answers the window's being-written refusal before a byte of
    /// the file, and the same query is written whole once the writer lets go.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_month_being_written_is_ask_again_and_never_a_file_in_part() {
        let store = Store::new("held");
        store.write("INDEX", None, 5, 0, 1_100);
        store.write("INDEX", None, 6, 0, 5);
        let june = StorePath::new(PathParts {
            vendor: Vendor::Gdfl,
            exchange: "NSE",
            segment: "INDEX",
            symbol: "NIFTY",
            contract: None,
            timeframe: Timeframe::MINUTE_1,
            month: YearMonth::new(2025, 6).expect("June"),
            file: FileKind::Bars,
        })
        .expect("a path");
        let hash = brutex_core::universe::fnv1a("NIFTY").to_le_bytes();
        let symbol = u32::from_le_bytes(hash[..4].try_into().expect("low 32 bits"));
        let (holding_tx, holding_rx) = std::sync::mpsc::channel::<()>();
        let (release_tx, release_rx) = std::sync::mpsc::channel::<()>();
        let writer_root = store.root.clone();
        let writer = std::thread::Builder::new()
            .name("r3-10-export-writer".to_owned())
            .spawn(move || {
                let held = store::file::BarFile::open_or_create(&writer_root, june, symbol)
                    .expect("the writer takes the month");
                holding_tx.send(()).expect("the test waits for the writer");
                let _ = release_rx.recv();
                drop(held);
            })
            .expect("a writer thread");
        holding_rx.recv().expect("the writer holds June");
        let query = format!("{SPOT}&from=2025-05&to=2025-06&dir=asc");
        let (status, headers, body) = store.export(&query).await;
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE, "{body:?}");
        assert!(headers.get("content-disposition").is_none());
        assert!(headers.get("x-export-rows").is_none());
        let body = body.expect("a refusal body");
        assert!(
            body.contains("\"being_written\":true") && body.contains("r3-10-export-writer"),
            "{body}"
        );
        release_tx.send(()).expect("the writer lets go");
        writer.join().expect("the writer thread");
        let (status, headers, body) = store.export(&query).await;
        assert_eq!(status, StatusCode::OK, "{body:?}");
        assert_eq!(headers["x-export-rows"], "1105");
        assert_eq!(records(&body.expect("a whole stream")).len(), 1_105);
    }

    /// A HEAD is the head alone: the count and the name, no body, and no
    /// export logged as abandoned.
    #[tokio::test]
    async fn a_head_is_the_head_alone() {
        let store = Store::new("head");
        store.write("INDEX", None, 5, 0, 1_200);
        let from = crate::emitted::mark();
        let (status, headers, body) = store
            .export_as(Method::HEAD, &format!("{SPOT}&from=2025-05&to=2025-05"))
            .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(headers["x-export-rows"], "1200");
        assert_eq!(body.unwrap(), "");
        assert!(line(from, "export abandoned; work stopped", 1_200).is_empty());
    }

    /// The contract columns of each key shape, and a field that needs quoting.
    #[test]
    fn contract_columns_and_quoting() {
        let parse =
            |text: &str| brutex_core::instrument::Contract::parse(text).expect("a contract");
        assert_eq!(ContractColumns::of(None), ContractColumns::default());
        let future = ContractColumns::of(Some(parse("2025-06-26-FUT")));
        assert_eq!(
            (
                future.expiry.as_str(),
                future.side.as_str(),
                future.strike.as_str()
            ),
            ("2025-06-26", "", "")
        );
        assert_eq!(future.expiry_day, Some(20_265));
        let option = ContractColumns::of(Some(parse("2019-03-28-1160000-PE")));
        assert_eq!(
            (
                option.expiry.as_str(),
                option.side.as_str(),
                option.strike.as_str()
            ),
            ("2019-03-28", "PE", "1160000")
        );
        let mut out = String::new();
        push_field(&mut out, "a,\"b\"");
        assert_eq!(out, "\"a,\"\"b\"\"\"");
        let mut out = String::new();
        push_field(&mut out, "BAJAJ-AUTO");
        assert_eq!(out, "BAJAJ-AUTO");
    }
}
