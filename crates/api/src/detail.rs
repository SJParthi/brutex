//! Shared bounded admission for blocking sweep detail, live and status reads.
//! Cold fixed-stride indexes cost O(history); refreshed indexes consume new
//! records. Neither blocking file reads nor formatting occupies a Tokio worker.
//! Admission limits queued plus running tasks, and readers enforce byte/row caps.

use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};

#[path = "boolean_observation_budget.rs"]
mod boolean_observation_budget;
pub(crate) use boolean_observation_budget::BooleanObservationBudget;

/// Detail reads that may be queued or running at once, across both endpoints.
pub const MAX_CONCURRENT: usize = 4;
/// Maximum bytes in any one fixed-stride file a request will freshly index.
pub const MAX_SCAN_BYTES: u64 = 64 * 1024 * 1024;
/// Maximum verified rows belonging to one run that a request will hold.
pub const MAX_RESULT_ROWS: u64 = 4_096;
// The frontier writer refuses a TOP above this same count before a run, so a
// run never commits frontier rows this reader then refuses whole. CE-19, D-1981.
const _: () = assert!(cli::frontier::MAX_ROWS as u64 == MAX_RESULT_ROWS);
/// Maximum rows rendered in one response page.
pub const MAX_PAGE_ROWS: u64 = 256;
/// Maximum zero-based page accepted at the HTTP boundary.
pub const MAX_PAGE: u64 = MAX_RESULT_ROWS - 1;
/// Maximum query bytes parsed by a detail endpoint.
pub const MAX_QUERY_BYTES: usize = 512;
/// Maximum JSON bytes returned by a successful or partial detail response.
pub const MAX_RESPONSE_BYTES: usize = 8 * 1024 * 1024;

/// Calendar derivations (`/calendar.json`, `/gaps.json`'s peer vote) that may
/// be queued or running at once on the blocking pool.
///
/// A pool of its own, not [`MAX_CONCURRENT`]'s: a page polling the calendar
/// must not be refused because sweep detail reads are busy, nor the reverse.
/// Concurrent misses on one series share one derivation
/// (`calendar_of::Cache`), so this bounds blocking threads, not derivations
/// per key. W1-api2-11, D-1443.
pub const MAX_CALENDAR_CONCURRENT: usize = 8;

/// Operator routes that read a folder or the masters directory
/// (`/folder.json`, `/indexmap.json`) and may be queued or running at once on
/// the blocking pool.
///
/// A pool of its own for the reason [`MAX_CALENDAR_CONCURRENT`] has one: a
/// page asking what a folder holds must not be refused because sweep detail
/// reads or calendar derivations are busy. W1-api2-11, D-1508.
pub const MAX_STORE_READ_CONCURRENT: usize = 8;

/// Log tail reads (`/logs.json`, `/logs`) that may be queued or running at
/// once on the blocking pool.
///
/// A pool of its own for the reason [`MAX_CALENDAR_CONCURRENT`] has one: the
/// backtest page polls `/logs.json` every two seconds per running sweep, and
/// that poll must neither be refused because a folder read is busy nor fill
/// the pool a folder read needs. Each admitted read walks at most
/// `2 × crate::logs::SCAN_BYTES` of NDJSON (`docs/06-limits.md`, D-2327), so this
/// count is what bounds the log bytes being decoded at once. log-3, P1-04-02,
/// D-2327.
pub const MAX_LOG_READ_CONCURRENT: usize = 4;

#[cfg(test)]
thread_local! {
    /// Whether a reader cache's mutex was free when this thread reached its
    /// cold open, as [`note_slot_free`] last saw it. Test builds only.
    /// expr-3, cand-2, D-2576.
    pub(crate) static SLOT_FREE_AT_OPEN: std::cell::Cell<Option<bool>> =
        const { std::cell::Cell::new(None) };
}

/// Records whether `slot` can be locked right now, from the cold-open point
/// of a reader cache (expr-3, cand-2, D-2576). A cache that still held its own
/// guard there would park every other detail permit behind one cold open; the
/// probe retries briefly, so a different thread's momentary take or put-back
/// is not mistaken for this thread's own hold. Test builds only.
#[cfg(test)]
pub(crate) fn note_slot_free<T>(slot: &std::sync::Mutex<T>) {
    let mut free = false;
    for _ in 0..10_000 {
        if !matches!(slot.try_lock(), Err(std::sync::TryLockError::WouldBlock)) {
            free = true;
            break;
        }
        std::thread::yield_now();
    }
    SLOT_FREE_AT_OPEN.with(|cell| cell.set(Some(free)));
}

static ACTIVE: AtomicUsize = AtomicUsize::new(0);
static CALENDAR_ACTIVE: AtomicUsize = AtomicUsize::new(0);
static STORE_READ_ACTIVE: AtomicUsize = AtomicUsize::new(0);
static LOG_READ_ACTIVE: AtomicUsize = AtomicUsize::new(0);

/// One admitted request in one pool.  Dropping it always returns the slot.
pub(crate) struct Permit(&'static AtomicUsize);

impl Permit {
    fn try_take() -> Option<Self> {
        Self::try_take_from(&ACTIVE, MAX_CONCURRENT)
    }

    fn try_take_from(pool: &'static AtomicUsize, max: usize) -> Option<Self> {
        pool.fetch_update(Ordering::AcqRel, Ordering::Acquire, |active| {
            (active < max).then_some(active.saturating_add(1))
        })
        .ok()
        .map(|_| Self(pool))
    }

    /// A slot for work an already-admitted request owes, taken past the cap.
    ///
    /// It is never refused, and it still counts: while it is held,
    /// [`Permit::try_take`] sees one more active task and refuses new work
    /// sooner. D-1445.
    fn owed() -> Self {
        ACTIVE.fetch_add(1, Ordering::AcqRel);
        Self(&ACTIVE)
    }
}

impl Drop for Permit {
    fn drop(&mut self) {
        let previous = self.0.fetch_sub(1, Ordering::AcqRel);
        debug_assert!(previous > 0, "a detail permit is released exactly once");
    }
}

/// Why a bounded blocking request did not produce its handler result.
#[derive(Debug, PartialEq, Eq)]
pub enum RunError {
    /// Every admitted blocking slot is already queued or running.
    Saturated,
    /// Tokio could not join the blocking task (normally runtime shutdown/panic).
    Join(String),
}

/// Runs blocking detail work outside Tokio's worker pool.
///
/// # Errors
///
/// Returns [`RunError::Saturated`] before queueing when all four slots are
/// occupied, or [`RunError::Join`] when Tokio cannot join the blocking task.
pub async fn run<T, F>(work: F) -> Result<T, RunError>
where
    T: Send + 'static,
    F: FnOnce() -> T + Send + 'static,
{
    let permit = Permit::try_take().ok_or(RunError::Saturated)?;
    admitted(permit, work).await
}

/// Runs a calendar derivation outside Tokio's worker pool, in the calendar
/// pool of [`MAX_CALENDAR_CONCURRENT`] slots. W1-api2-11, D-1443.
///
/// # Errors
///
/// [`RunError::Saturated`] before queueing when every calendar slot is
/// occupied, or [`RunError::Join`] when Tokio cannot join the blocking task.
pub async fn run_calendar<T, F>(work: F) -> Result<T, RunError>
where
    T: Send + 'static,
    F: FnOnce() -> T + Send + 'static,
{
    let permit = Permit::try_take_from(&CALENDAR_ACTIVE, MAX_CALENDAR_CONCURRENT)
        .ok_or(RunError::Saturated)?;
    admitted(permit, work).await
}

/// Runs an operator route's folder or masters read outside Tokio's worker
/// pool, in its own pool of [`MAX_STORE_READ_CONCURRENT`] slots. W1-api2-11,
/// D-1508.
///
/// # Errors
///
/// [`RunError::Saturated`] before queueing when every slot is occupied, or
/// [`RunError::Join`] when Tokio cannot join the blocking task.
pub async fn run_store_read<T, F>(work: F) -> Result<T, RunError>
where
    T: Send + 'static,
    F: FnOnce() -> T + Send + 'static,
{
    let permit = Permit::try_take_from(&STORE_READ_ACTIVE, MAX_STORE_READ_CONCURRENT)
        .ok_or(RunError::Saturated)?;
    admitted(permit, work).await
}

/// Runs a `/logs.json` or `/logs` tail walk outside Tokio's worker pool, in
/// its own pool of [`MAX_LOG_READ_CONCURRENT`] slots. log-3, P1-04-02, D-2327.
///
/// # Errors
///
/// [`RunError::Saturated`] before queueing when every slot is occupied, or
/// [`RunError::Join`] when Tokio cannot join the blocking task.
pub async fn run_log_read<T, F>(work: F) -> Result<T, RunError>
where
    T: Send + 'static,
    F: FnOnce() -> T + Send + 'static,
{
    let permit = Permit::try_take_from(&LOG_READ_ACTIVE, MAX_LOG_READ_CONCURRENT)
        .ok_or(RunError::Saturated)?;
    admitted(permit, work).await
}

/// The status and JSON body a route answers when its blocking work was not
/// admitted (429) or could not be joined (503), naming `what` and its bound.
/// D-1508.
#[must_use]
pub fn admission_refused(
    what: &str,
    bound: usize,
    why: &RunError,
) -> (axum::http::StatusCode, String) {
    let status = if matches!(why, RunError::Saturated) {
        axum::http::StatusCode::TOO_MANY_REQUESTS
    } else {
        axum::http::StatusCode::SERVICE_UNAVAILABLE
    };
    let body = serde_json::json!({
        "error": format!(
            "{what} not admitted ({why:?}): at most {bound} run at once, off the async \
             workers; retry"
        )
    });
    (status, body.to_string())
}

async fn admitted<T, F>(permit: Permit, work: F) -> Result<T, RunError>
where
    T: Send + 'static,
    F: FnOnce() -> T + Send + 'static,
{
    tokio::task::spawn_blocking(move || {
        let _permit = permit;
        work()
    })
    .await
    .map_err(|why| RunError::Join(why.to_string()))
}

/// Runs blocking work an already-admitted request OWES, outside Tokio's
/// worker pool, without refusing it when every slot is taken.
///
/// For the invocation journal's terminal record only: the handler has already
/// run, so refusing the write that records its outcome cannot undo it and
/// used to leave a false `Cancelled`. The slot it takes is counted against
/// [`run`]'s admission but bypasses its cap, so at most one owed task exists
/// per audited request whose handler has returned; `docs/06-limits.md`
/// (D-1445) states that this count is bounded by in-flight requests, not by
/// [`MAX_CONCURRENT`]. UNVERIFIED: no bench times this path.
///
/// # Errors
///
/// Returns [`RunError::Join`] when Tokio cannot join the blocking task. It
/// never returns [`RunError::Saturated`].
pub(crate) async fn run_owed<T, F>(work: F) -> Result<T, RunError>
where
    T: Send + 'static,
    F: FnOnce() -> T + Send + 'static,
{
    let permit = Permit::owed();
    tokio::task::spawn_blocking(move || {
        let _permit = permit;
        work()
    })
    .await
    .map_err(|why| RunError::Join(why.to_string()))
}

/// Refuses a detail query before any parser allocates from an unbounded URI.
///
/// # Errors
///
/// Returns the measured and admitted byte counts when `query` is too large.
pub fn query_is_bounded(query: &str) -> Result<(), String> {
    if query.len() > MAX_QUERY_BYTES {
        return Err(format!(
            "detail query is {} bytes; this endpoint accepts at most {MAX_QUERY_BYTES}",
            query.len()
        ));
    }
    Ok(())
}

/// One explicitly bounded response page.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Page {
    /// Zero-based page number.
    pub number: u64,
    /// Rows requested on a page.
    pub limit: u64,
}

impl Page {
    /// Parses `page` and `limit`, refusing malformed and over-limit values.
    ///
    /// # Errors
    ///
    /// Refuses an oversized query, repeated parser failure, zero/over-limit
    /// row count, over-limit page number or arithmetic overflow.
    pub fn parse(query: &str) -> Result<Self, String> {
        query_is_bounded(query)?;
        let number = integer_param(query, "page")?.unwrap_or(0);
        let limit = integer_param(query, "limit")?.unwrap_or(MAX_PAGE_ROWS);
        if number > MAX_PAGE {
            return Err(format!(
                "`page` is {number}; the hard maximum is {MAX_PAGE}"
            ));
        }
        if limit == 0 || limit > MAX_PAGE_ROWS {
            return Err(format!(
                "`limit` must be from 1 through {MAX_PAGE_ROWS}; received {limit}"
            ));
        }
        let _ = number
            .checked_mul(limit)
            .ok_or_else(|| "`page * limit` overflows u64".to_owned())?;
        Ok(Self { number, limit })
    }

    /// First row represented by this page.
    #[must_use]
    pub fn offset(self) -> u64 {
        self.number.saturating_mul(self.limit)
    }
}

/// The refusal `/trades.json` and `/frontier.json` give a missing, short, long
/// or non-hex `identity`. One copy, because both routes key on one identity.
const IDENTITY_REFUSAL: &str = "`identity` must be the 64 hex characters `/backtest.json` prints on \
     every row. This file holds many runs, so which one is not a detail \
     it can infer.";

/// One `/trades.json` or `/frontier.json` selector, parsed before admission.
///
/// Both routes used to parse this inside [`run`], so a selector that could
/// only be refused still took a detail slot, and while every slot was held it
/// was answered 429 instead of 400. Parsing it on the async path first means a
/// malformed selector never reaches admission (D-0689).
///
/// The order is the one the blocking path used -- page, then store root, then
/// identity -- so a request with more than one fault is refused for the same
/// one, in the same words, as before.
pub(crate) struct Selector {
    /// The store root the blocking read opens.
    pub(crate) root: std::path::PathBuf,
    /// The run whose detail rows are asked for.
    pub(crate) identity: [u8; 32],
    /// The one page of those rows asked for.
    pub(crate) page: Page,
}

impl Selector {
    /// Parses `page`/`limit`, then takes the store root, then decodes `identity`.
    ///
    /// # Errors
    ///
    /// The first of: [`Page::parse`]'s refusal, the store-root refusal passed
    /// in, or the identity refusal. Each is returned verbatim.
    pub(crate) fn parse(
        root: Result<std::path::PathBuf, String>,
        query: &str,
    ) -> Result<Self, String> {
        // THE SAME EXACTNESS EVERY SIBLING DETAIL ROUTE HOLDS. This read only
        // its three keys through `param`, so `offset=256` (the boolean routes'
        // spelling) or a typo like `pgae=3` was answered page 0 under 200,
        // again and again. Unknown, repeated and empty keys now refuse
        // (P1-01-04, D-1765).
        query_is_bounded(query)?;
        let mut seen = std::collections::BTreeSet::new();
        for pair in query.split('&').filter(|pair| !pair.is_empty()) {
            let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
            if value.is_empty()
                || !matches!(key, "identity" | "page" | "limit")
                || !seen.insert(key)
            {
                return Err(format!(
                    "unknown, repeated or empty detail query field {key:?}; this \
                     route reads identity, page and limit"
                ));
            }
        }
        let page = Page::parse(query)?;
        let root = root?;
        let identity = crate::trades::from_hex_public(&crate::server::param(query, "identity"))
            .ok_or_else(|| IDENTITY_REFUSAL.to_owned())?;
        Ok(Self {
            root,
            identity,
            page,
        })
    }
}

/// A canonical unsigned decimal: `+3` and `003` are refused, as
/// `candidatejson::integer` refuses them, rather than read as 3 (D-1765).
fn integer_param(query: &str, name: &str) -> Result<Option<u64>, String> {
    let value = crate::server::param(query, name);
    if value.is_empty() {
        return Ok(None);
    }
    value
        .parse::<u64>()
        .ok()
        .filter(|parsed| parsed.to_string() == value)
        .map(Some)
        .ok_or_else(|| format!("`{name}` must be a canonical unsigned decimal integer"))
}

/// Bounds every file a fresh detail request will index before it opens one.
///
/// Missing files are left to the route's existing absent-vs-corrupt logic.
/// Every other metadata failure is a refusal, never an inferred empty store.
///
/// # Errors
///
/// Refuses a present file over [`MAX_SCAN_BYTES`] or any metadata error other
/// than `NotFound`. The opened-handle readers repeat the hard measurement.
pub fn preflight<'a>(paths: impl IntoIterator<Item = (&'a str, &'a Path)>) -> Result<(), String> {
    for (name, path) in paths {
        match std::fs::metadata(path) {
            Ok(metadata) if metadata.len() > MAX_SCAN_BYTES => {
                return Err(format!(
                    "{name} is {} bytes; this fresh detail request will index at most {MAX_SCAN_BYTES} bytes from any one file. No partial index was built",
                    metadata.len()
                ));
            }
            Ok(_) => {}
            Err(why) if why.kind() == std::io::ErrorKind::NotFound => {}
            Err(why) => {
                return Err(format!(
                    "{name} at {} could not be measured before its bounded read: {why}",
                    path.display()
                ));
            }
        }
    }
    Ok(())
}

/// Whether a page contains the complete result and where a later page starts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Window {
    /// Inclusive start in the verified result block.
    pub start: usize,
    /// Exclusive end in the verified result block.
    pub end: usize,
    /// Whether this one response contains the entire result.
    pub complete: bool,
    /// The next zero-based page when rows remain.
    pub next_page: Option<u64>,
}

/// Resolves one page against an already verified, hard-bounded result count.
///
/// A page beyond a non-empty result (or any non-zero page of an empty result)
/// is refused rather than returned as a plausible empty list. `complete` means
/// this response contains the whole result, not merely that its page read
/// finished.
///
/// # Errors
///
/// Refuses a result above [`MAX_RESULT_ROWS`], an offset outside the verified
/// result, or any coordinate that cannot be represented on this machine.
pub fn window(total: usize, page: Page) -> Result<Window, String> {
    let total_u64 = u64::try_from(total).unwrap_or(u64::MAX);
    if total_u64 > MAX_RESULT_ROWS {
        return Err(format!(
            "run owns {total_u64} detail rows; one request verifies at most {MAX_RESULT_ROWS}. No prefix was read or exposed"
        ));
    }
    seek_window(total, page)
}

/// [`window`] for a reader that SEEKS to one page and never holds the result.
///
/// [`MAX_RESULT_ROWS`] bounds what a request holds and verifies, and it is the
/// right refusal for a reader that loads every row of a run before it slices
/// one page. A fixed-stride reader that seeks to `offset` and reads at most
/// `page.limit` rows holds one page whatever the total, so the cap protects
/// nothing there, and applying it refused every page of a result above 4,096
/// rows, page 0 included: an audit keeps `audit_keep()` ranked rows, 10,000 by
/// default, so its ranked evidence was never readable (W1-api6-5, D-0954).
///
/// The total is still bounded, by the caller's byte cap on the file it seeks
/// in, and the page by [`MAX_PAGE_ROWS`]. Every other check is [`window`]'s:
/// a page past a non-empty result, or any non-zero page of an empty one, is
/// refused rather than returned as a plausible empty list. O(1) arithmetic,
/// pinned at every boundary by
/// `api::detail::a_seeked_window_has_no_hold_cap_and_keeps_every_other_refusal`.
///
/// ONE MORE REFUSAL, because [`Page::parse`] still caps the page NUMBER at
/// [`MAX_PAGE`]: at `limit` rows a page, only `(MAX_PAGE + 1) * limit` rows
/// are addressable. A result larger than that is refused on every page with
/// the smallest limit that reaches all of it, rather than served with a
/// `next_page` the parser would then refuse. At the default 256 that is
/// 1,048,576 rows, more than any 64 MiB file of 64-byte or wider rows holds.
///
/// # Errors
///
/// Refuses a result the requested limit cannot page to its end, an offset
/// outside the result, or a coordinate that cannot be represented on this
/// machine.
pub fn seek_window(total: usize, page: Page) -> Result<Window, String> {
    let total_u64 = u64::try_from(total).unwrap_or(u64::MAX);
    let addressable = MAX_PAGE.saturating_add(1).saturating_mul(page.limit);
    if total_u64 > addressable {
        let needed = total_u64.div_ceil(MAX_PAGE.saturating_add(1));
        return Err(format!(
            "result owns {total_u64} rows and `limit={}` addresses only the first {addressable} of them within page {MAX_PAGE}; ask `limit={needed}` or more. No prefix was read or exposed",
            page.limit
        ));
    }
    let offset = page.offset();
    if total == 0 && page.number > 0 {
        return Err(format!(
            "page {} was requested for an empty detail result; only page 0 exists",
            page.number
        ));
    }
    if total > 0 && offset >= total_u64 {
        return Err(format!(
            "page {} starts at row {offset}, past this run's {total_u64} rows",
            page.number
        ));
    }
    let start = usize::try_from(offset).unwrap_or(total);
    let take = usize::try_from(page.limit).unwrap_or(0);
    let end = start.saturating_add(take).min(total);
    let complete = start == 0 && end == total;
    Ok(Window {
        start,
        end,
        complete,
        next_page: (end < total).then_some(page.number.saturating_add(1)),
    })
}

#[cfg(test)]
static TEST_SERIAL: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// Keeps a test that sends admitted requests through these shared slots from
/// running while [`hold_every_slot`] must own all of them.
#[cfg(test)]
pub(crate) async fn apart_from_slot_owners() -> tokio::sync::MutexGuard<'static, ()> {
    TEST_SERIAL.lock().await
}

/// Takes every slot that is free right now, for a test that must saturate the
/// pool from inside a request it is already running.
///
/// Asks for the serial guard so it cannot race [`hold_every_slot`]. Other
/// tests that use a slot without that guard may still hold some, which is why
/// this takes what is free rather than exactly [`MAX_CONCURRENT`].
#[cfg(test)]
pub(crate) fn take_every_free_slot(_apart: &tokio::sync::MutexGuard<'static, ()>) -> Vec<Permit> {
    std::iter::from_fn(Permit::try_take).collect()
}

#[cfg(test)]
pub(crate) struct HeldSlots {
    _serial: tokio::sync::MutexGuard<'static, ()>,
    _permits: Vec<Permit>,
}

#[cfg(test)]
pub(crate) async fn hold_every_slot() -> Result<HeldSlots, &'static str> {
    let serial = TEST_SERIAL.lock().await;
    let permits = (0..MAX_CONCURRENT)
        .map(|_| Permit::try_take().ok_or("the test did not own every detail slot"))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(HeldSlots {
        _serial: serial,
        _permits: permits,
    })
}

/// Takes every log-read slot that is free, for a test that must see
/// `/logs.json` and `/logs` refused at admission. Asks for the serial guard,
/// and every test that sends a log read holds that guard too, so while it is
/// held this takes all [`MAX_LOG_READ_CONCURRENT`] and nothing else can.
#[cfg(test)]
pub(crate) fn take_every_log_read_slot(
    _apart: &tokio::sync::MutexGuard<'static, ()>,
) -> Vec<Permit> {
    std::iter::from_fn(|| Permit::try_take_from(&LOG_READ_ACTIVE, MAX_LOG_READ_CONCURRENT))
        .collect()
}

/// Takes every store-read slot that is free, for a test that must see a
/// store-reading route refused at admission (resources-4, P1-04-01, D-2593).
/// Asks for the serial guard so it cannot race [`hold_every_slot`] or the
/// other pool-holding tests; it takes what is free rather than exactly
/// [`MAX_STORE_READ_CONCURRENT`], and the caller asserts what it got.
#[cfg(test)]
pub(crate) fn take_every_store_read_slot(
    _apart: &tokio::sync::MutexGuard<'static, ()>,
) -> Vec<Permit> {
    std::iter::from_fn(|| Permit::try_take_from(&STORE_READ_ACTIVE, MAX_STORE_READ_CONCURRENT))
        .collect()
}

/// One long-lived read handle on a results file, refreshed per request.
///
/// # Why a cached handle, and what it changes
///
/// Every results file -- `runs.bin`, `frontier.bin`, `chosen-trades.bin`,
/// `detail-sets.bin` -- builds its identity index by walking the whole file at
/// `open`. The probe on an open handle is then one hash hit. But this crate
/// opened FRESH on every request, so one `/trades.json` walked every trade row
/// ever written and verified every seal to return one run's rows: O(8,145)
/// today for at most 256, with a hard refusal at [`MAX_SCAN_BYTES`] that lands
/// near 666 runs at the current density.
///
/// `Trades::refresh` and `Frontier::refresh` absorb only the rows appended
/// since the handle last scanned -- O(delta), zero when nothing was written.
/// Holding the handle here makes the first request O(rows) and every one after
/// it O(delta): the end-to-end lookup finally has the cost its probe always had.
///
/// # The ordering guarantee is kept, and this is why `refresh` runs per request
///
/// Both handlers snapshot the committed receipt BEFORE touching the child, and
/// the writer syncs children before it writes the receipt. So a receipt seen at
/// T0 proves its rows were durable before T0, and a refresh at any T1 >= T0
/// absorbs them. The order "receipt, then child" was the whole argument for
/// opening fresh; refreshing after the receipt is the same argument with the
/// walk removed.
///
/// # Any refusal from `refresh` drops the handle
///
/// A shrunken file, a torn tail, a file replaced or moved aside under its path
/// -- each is a reason the handle no longer describes what is on disk. The
/// answer is one fresh `open`, which is the single O(rows) path a cache should
/// ever take, and the refusal that caused it is never swallowed: if the reopen
/// also fails, that error is the response.
///
/// A recorded integrity failure (a bad seal, an invalid schema, a
/// non-contiguous duplicate identity) is a refusal for `Frontier::refresh`,
/// which still refuses on it first. It is NOT one for `Trades::refresh` since
/// D-0919: that refresh records the damage the way a cold open does and keeps
/// its handle, and it refuses instead when the path names a file other than the
/// one it holds, so a reviewed repair renamed into place is reopened.
///
/// A root that differs from the cached one is treated the same way. There is
/// one store root per process, so this is a guard against a future caller
/// rather than a branch that runs today.
///
/// # What this does NOT change
///
/// [`preflight`] still refuses a file over [`MAX_SCAN_BYTES`] before any
/// handle is touched. That wall bounded a FRESH index, which no longer happens
/// on a warm cache, so it is now stricter than it needs to be -- but lifting it
/// is a separate decision about what an unbounded results file should cost,
/// and this change does not make it.
///
/// # Which lock is held, and for how long (D-2309)
///
/// The slot behind `inner` holds an `Arc` to the handle's own mutex, and the
/// slot lock is held only to read, install or clear that `Arc`: O(1), never
/// across `open`, `refresh` or `f`. A cold `open` -- O(history), up to
/// [`MAX_SCAN_BYTES`] -- runs holding no lock at all, and a fresh handle is
/// locked by its opener before it is installed, so `f` sees it before any
/// other request can refresh it. Two requests that both find no usable handle
/// both open; the later install replaces the earlier, and each serves the
/// handle it opened and checked. That duplicated open is the stated price.
///
/// `refresh` and `f` run under the HANDLE's mutex, so requests for one
/// handle still take its growth branch -- D-1560's O(indexed bytes) re-hash
/// -- one at a time. That is not removed: `refresh` mutates the handle, and a
/// second request that waits is served the already-refreshed handle, where
/// refreshing beside it would pay the same re-hash again. Proved by `api::detail::the_slot_lock_is_free_while_a_handle_opens_or_refreshes`.
pub struct Cached<T> {
    inner: std::sync::Mutex<Option<(std::path::PathBuf, Slot<T>)>>,
}

/// One handle, shared between the slot and every request using it.
type Slot<T> = std::sync::Arc<std::sync::Mutex<T>>;

/// READ THROUGH A POISONED LOCK, for the reason `calendar_of` gives: a panic
/// while holding it means some other request died, and the handle is still a
/// handle. Refusing to look would make one panicked request cost every later
/// one the walk this exists to remove.
fn lock_through_poison<V>(mutex: &std::sync::Mutex<V>) -> std::sync::MutexGuard<'_, V> {
    mutex
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

impl<T> Cached<T> {
    /// An empty slot. `const` so it can back a `static`.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            inner: std::sync::Mutex::new(None),
        }
    }

    /// The handle cached for `root`, if any: one O(1) look under the slot lock,
    /// released before the caller touches the handle. Proved by `api::detail::the_slot_lock_is_free_while_a_handle_opens_or_refreshes`.
    fn cached_for(&self, root: &std::path::Path) -> Option<Slot<T>> {
        match lock_through_poison(&self.inner).as_ref() {
            Some((at, slot)) if at.as_path() == root => Some(std::sync::Arc::clone(slot)),
            _ => None,
        }
    }

    /// Install a handle `open` has just returned and run `f` on it. The
    /// handle's own lock is taken BEFORE it is published, so no other request
    /// refreshes it before `f` has run; the slot lock is held only for the
    /// O(1) store, and never while waiting on a handle's lock. Proved by `api::detail::the_slot_lock_is_free_while_a_handle_opens_or_refreshes`.
    fn install<R>(&self, root: &std::path::Path, opened: T, f: impl FnOnce(&mut T) -> R) -> R {
        let slot: Slot<T> = std::sync::Arc::new(std::sync::Mutex::new(opened));
        let mut handle = lock_through_poison(&slot);
        // The replaced handle is dropped after the slot lock is released.
        let replaced = lock_through_poison(&self.inner)
            .replace((root.to_path_buf(), std::sync::Arc::clone(&slot)));
        drop(replaced);
        f(&mut handle)
    }

    /// Run `f` against a handle on `root`, refreshing a cached one or opening
    /// a fresh one.
    ///
    /// `Err` is an `open` failure and nothing else: a `refresh` failure is
    /// answered by reopening, and `f`'s own result is returned inside `Ok`
    /// untouched, so a caller keeps every refusal it could already make.
    ///
    /// # Errors
    ///
    /// Whatever `open` returns, when no usable handle exists and one could not
    /// be opened.
    pub fn with<R>(
        &self,
        root: &std::path::Path,
        open: impl FnOnce() -> Result<T, String>,
        refresh: impl FnOnce(&mut T) -> Result<(), String>,
        f: impl FnOnce(&mut T) -> R,
    ) -> Result<R, String> {
        if let Some(slot) = self.cached_for(root) {
            let mut handle = lock_through_poison(&slot);
            if refresh(&mut handle).is_ok() {
                return Ok(f(&mut handle));
            }
        }
        // Absent, a different root, or a refresh that refused: open fresh,
        // holding no lock. The slot is overwritten rather than cleared first,
        // so a failed `open` leaves whatever was there -- which the next
        // request refreshes and judges again on its own terms.
        let opened = open()?;
        Ok(self.install(root, opened, f))
    }

    /// Uses a refreshed handle, exposing any failed generation or integrity
    /// check on this request. A refused handle is discarded; a later request
    /// may attempt a fresh validated open after the underlying issue is fixed.
    ///
    /// # Errors
    /// Returns the first failed refresh/open rather than hiding it behind an
    /// automatic successful reopen during the same request.
    pub fn with_verified<R>(
        &self,
        root: &std::path::Path,
        open: impl FnOnce() -> Result<T, String>,
        refresh: impl FnOnce(&mut T) -> Result<(), String>,
        f: impl FnOnce(&mut T) -> R,
    ) -> Result<R, String> {
        if let Some(slot) = self.cached_for(root) {
            let mut handle = lock_through_poison(&slot);
            if let Err(why) = refresh(&mut handle) {
                drop(handle);
                // Discard the refused handle -- unless a racing open has
                // already replaced it with one that passed its own checks.
                let mut held = lock_through_poison(&self.inner);
                if held
                    .as_ref()
                    .is_some_and(|(_, cached)| std::sync::Arc::ptr_eq(cached, &slot))
                {
                    *held = None;
                }
                return Err(why);
            }
            return Ok(f(&mut handle));
        }
        let opened = open()?;
        Ok(self.install(root, opened, f))
    }
}

impl<T> Default for Cached<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// The process's one read handle on `chosen-trades.bin`. See [`Cached`].
pub static TRADES: Cached<cli::trades::Trades> = Cached::new();

/// The process's one read handle on `frontier.bin`. See [`Cached`].
pub static FRONTIER: Cached<cli::frontier::Frontier> = Cached::new();

static PARENTS: Cached<cli::result_set::CommittedParents> = Cached::new();

/// Refreshes both parent indexes once and returns one owned receipt, with
/// the instrument its ledger parent names, before the caller refreshes any
/// child. Cold open is O(history); warm refresh is O(new parent rows), with
/// each file still subject to the HTTP byte ceiling.
///
/// # Errors
/// Returns parent generation/integrity/size failures and missing committed
/// receipts; no child is exposed after an unsuccessful parent refresh.
pub fn committed_receipt(
    root: &Path,
    identity: &[u8; 32],
) -> Result<Option<cli::result_set::Committed>, String> {
    PARENTS.with_verified(
        root,
        || cli::result_set::CommittedParents::open_read_bounded(root, MAX_SCAN_BYTES),
        cli::result_set::CommittedParents::refresh,
        |parents| parents.committed(identity),
    )?
}

static LEDGER: Cached<cli::results::Results> = Cached::new();

/// The instrument the results ledger records for `identity`, or `None` when
/// no ledger row names it, a store with no ledger or an empty one included.
///
/// For a reader whose own file is keyed by identity and is not a receipted
/// child: `/sweep-evidence.json` and the AND-mask `/candidate-trades.json`.
/// It reads the ledger alone, so a damaged receipt sidecar cannot refuse a
/// saved attempt that never had a receipt. Cold open is O(history); warm
/// refresh is O(new rows), under the same byte ceiling. Nothing is created:
/// an absent or empty ledger is answered before any open.
///
/// AN EMPTY LEDGER IS AN ABSENCE, as `cli::results::Results::open_read` says
/// on its own read path: no run has been recorded, which is not an error. A
/// zero-byte `runs.bin` is what `Results::open` leaves when it stops between
/// creating the file and writing its header, and the next writer gives it
/// its header. The length is measured from the path's metadata, one call,
/// rather than by matching the words of the read path's refusal.
///
/// # Errors
/// Returns a damaged, changed or over-limit ledger, never a guessed `None`.
pub fn recorded_underlying(root: &Path, identity: &[u8; 32]) -> Result<Option<String>, String> {
    match std::fs::metadata(cli::results::Results::path(root)) {
        Err(why) if why.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(why) => return Err(format!("ledger path cannot be inspected: {why}")),
        Ok(ledger) if ledger.len() == 0 => return Ok(None),
        Ok(_) => {}
    }
    let parent = LEDGER.with_verified(
        root,
        || cli::results::Results::open_read_bounded(root, MAX_SCAN_BYTES),
        cli::results::Results::refresh,
        |ledger| ledger.of_identity(identity),
    )??;
    Ok(parent.map(|row| cli::results::read_field(&row.underlying)))
}

/// The `"equity_note"` member, leading comma included, that a payload over
/// one recorded run carries when that run's instrument is a swept stock, and
/// nothing for any other run.
///
/// The text is `cli::equity_note_for`, the stored banner's own: gross of
/// every charge, then corporate actions unchecked. `/backtest.json`,
/// `/frontier.json`, `/trades.json` and `/sweep-evidence.json` all take it
/// from here, so one instrument cannot be described two ways, and an index
/// run's payload gains no key and is the bytes it was. D-0694, AF-19.
#[must_use]
pub fn equity_note_member(underlying: Option<&str>) -> String {
    let note = underlying.map_or_else(String::new, cli::equity_note_for);
    if note.is_empty() {
        return note;
    }
    format!(r#","equity_note":{}"#, crate::render::json_string(&note))
}

/// [`equity_note_member`] for a payload built as a `serde_json::Value`: puts
/// `note` in as the `"equity_note"` member when it says anything, and adds
/// no key when it is empty, so an index payload is the value it was.
///
/// `note` is always one of `cli`'s own wordings, `cli::equity_note_for` for a
/// recorded run or `cli::research_equity_note` for Boolean research families,
/// and both are `runner::audit::CostScope::report_note`. D-0694, AF-19.
///
/// # Errors
/// Refuses a payload that is not a JSON object rather than dropping the note.
pub fn put_equity_note(body: &mut serde_json::Value, note: String) -> Result<(), String> {
    if note.is_empty() {
        return Ok(());
    }
    body.as_object_mut()
        .ok_or("a payload that is not a JSON object cannot carry its equity note")?
        .insert("equity_note".to_owned(), serde_json::Value::String(note));
    Ok(())
}

/// Whether a Boolean observation route must authenticate its saved body again
/// rather than project from the reader its single slot holds. W1-api1-5, D-1444.
///
/// `held` is whether the slot holds a reader for exactly this root, identity,
/// model and budget. A pinned request (`pinned`) over a held reader reuses it
/// and lets the projection refuse a changed generation, as it always did. An
/// unpinned request -- every first page -- used to authenticate afresh every
/// time; it now reuses a held reader whose `current` check passes (the same
/// generation and lease check a warm page makes) and authenticates again only
/// when that check refuses. `current` is not called when nothing is held or the
/// request is pinned. What a cold admission still costs is stated in
/// `docs/06-limits.md` under D-1444 and is UNVERIFIED as a measurement.
pub(crate) fn must_admit(held: bool, pinned: bool, current: impl FnOnce() -> bool) -> bool {
    !held || (!pinned && !current())
}

/// ONE RETAINED READER, TAKEN OUT BY A REQUEST AND PUT BACK WHEN IT ENDS
/// (locks-2, D-1912).
///
/// The five index-stop JSON caches held a process-wide `try_lock` guard across
/// the whole render, a cold `Reader::open` included. `spawn_blocking` cannot be
/// cancelled, so a request the page had just abandoned kept the slot until its
/// verification finished, and the same viewer's next click was refused 503
/// "busy". The mutex is now held only to take the entry out and to put it back:
/// a request that finds the slot empty opens its own reader, and the last one
/// to finish is the one retained. Concurrency stays bounded by `run`'s
/// admission, not by this slot.
pub(crate) struct Checkout<'slot, T> {
    slot: &'slot std::sync::Mutex<Option<T>>,
    value: Option<T>,
}

impl<'slot, T> Checkout<'slot, T> {
    /// Empties `slot` into this request. The lock is released on return.
    pub(crate) fn take(slot: &'slot std::sync::Mutex<Option<T>>) -> Self {
        // A poisoned slot holds a plain `Option`; no invariant spans the
        // panic, and the shipped binary aborts on panic regardless.
        let value = slot
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .take();
        Self { slot, value }
    }
}

impl<T> std::ops::Deref for Checkout<'_, T> {
    type Target = Option<T>;
    fn deref(&self) -> &Option<T> {
        &self.value
    }
}

impl<T> std::ops::DerefMut for Checkout<'_, T> {
    fn deref_mut(&mut self) -> &mut Option<T> {
        &mut self.value
    }
}

impl<T> Drop for Checkout<'_, T> {
    /// Puts a still-admitted entry back. An entry this request evicted stays
    /// out.
    fn drop(&mut self) {
        if let Some(value) = self.value.take() {
            *self
                .slot
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(value);
        }
    }
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    reason = "tests fail through assertions"
)]
mod tests {
    use super::{
        Cached, IDENTITY_REFUSAL, LOG_READ_ACTIVE, MAX_LOG_READ_CONCURRENT, MAX_PAGE,
        MAX_PAGE_ROWS, MAX_QUERY_BYTES, MAX_RESULT_ROWS, MAX_SCAN_BYTES, MAX_STORE_READ_CONCURRENT,
        Ordering, Page, Permit, RunError, STORE_READ_ACTIVE, Selector, admission_refused,
        must_admit, preflight, run, run_calendar, run_log_read, run_store_read, seek_window,
        window,
    };

    /// locks-2, D-1912: a request that finds the retained reader taken out is
    /// not refused; it starts empty, and the entry is back once the first
    /// request ends. An evicted entry is not put back.
    #[test]
    fn a_checked_out_reader_never_refuses_the_next_request() {
        let slot = std::sync::Mutex::new(Some(7_u8));
        let first = super::Checkout::take(&slot);
        assert_eq!(*first, Some(7));
        let second = super::Checkout::take(&slot);
        assert_eq!(*second, None);
        drop(second);
        drop(first);
        let mut third = super::Checkout::take(&slot);
        assert_eq!(*third, Some(7));
        *third = None;
        drop(third);
        assert_eq!(*slot.lock().unwrap(), None);
    }

    /// locks-2, D-1912: no index-stop cache holds its slot across the render.
    #[test]
    fn no_index_stop_cache_holds_its_slot_across_the_render() {
        for source in [
            include_str!("indexstopvixjson.rs"),
            include_str!("indexstopcandlesjson.rs"),
            include_str!("indexstopqualificationjson.rs"),
            include_str!("indexstopjson.rs"),
            include_str!("indexstoprankingjson.rs"),
        ] {
            assert!(!source.contains(".try_lock()"));
            assert!(source.contains("crate::detail::Checkout::take(CACHE.get_or_init("));
        }
    }

    /// THE ADMISSION DECISION, EVERY INPUT. W1-api1-5, D-1444.
    ///
    /// Nothing held: admit, and the currency check is never asked (there is
    /// no reader to ask). Held and pinned: reuse without asking, so the
    /// projection's own check is what refuses a changed generation. Held and
    /// unpinned: reuse exactly when the reader is current.
    #[test]
    fn a_held_reader_is_reused_when_pinned_or_current_and_admitted_again_otherwise() {
        for pinned in [false, true] {
            for current in [false, true] {
                let mut asked = 0;
                assert!(
                    must_admit(false, pinned, || {
                        asked += 1;
                        current
                    }),
                    "nothing held: pinned={pinned} current={current}"
                );
                assert_eq!(asked, 0, "nothing held is never asked about currency");
                let mut asked = 0;
                let admit = must_admit(true, pinned, || {
                    asked += 1;
                    current
                });
                assert_eq!(
                    admit,
                    !pinned && !current,
                    "pinned={pinned} current={current}"
                );
                assert_eq!(
                    asked,
                    usize::from(!pinned),
                    "pinned={pinned}: asked once if unpinned"
                );
            }
        }
    }

    /// **A POOL ADMITS EXACTLY ITS BOUND, AND A DROPPED PERMIT RETURNS ITS
    /// SLOT.** W1-api2-11, D-1443. Driven on a pool of the test's own, so no
    /// route test sharing the real pools can see it.
    #[test]
    fn a_pool_admits_its_bound_and_a_dropped_permit_frees_its_slot() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        static POOL: AtomicUsize = AtomicUsize::new(0);
        let first = super::Permit::try_take_from(&POOL, 2).expect("one of two");
        let second = super::Permit::try_take_from(&POOL, 2).expect("two of two");
        assert!(
            super::Permit::try_take_from(&POOL, 2).is_none(),
            "a third is refused"
        );
        assert_eq!(POOL.load(Ordering::Acquire), 2);
        drop(first);
        assert_eq!(POOL.load(Ordering::Acquire), 1);
        let third = super::Permit::try_take_from(&POOL, 2).expect("the freed slot");
        drop((second, third));
        assert_eq!(POOL.load(Ordering::Acquire), 0, "every slot returned");
        assert!(
            super::Permit::try_take_from(&POOL, 0).is_none(),
            "a zero bound admits nothing"
        );
        assert_eq!(
            POOL.load(Ordering::Acquire),
            0,
            "and a refusal takes nothing"
        );
    }

    /// **A CALENDAR DERIVATION RUNS OFF THE ASYNC WORKER.** W1-api2-11,
    /// D-1443. On a current-thread runtime the worker is the test's own
    /// thread, so work that reports another thread ran on the blocking pool.
    #[tokio::test(flavor = "current_thread")]
    async fn a_calendar_derivation_runs_off_the_async_worker() {
        let worker = std::thread::current().id();
        let ran_on = super::run_calendar(|| std::thread::current().id())
            .await
            .expect("admitted");
        assert_ne!(ran_on, worker, "the blocking pool, not the async worker");
    }

    /// ONLY AN ABSENT LEDGER IS AN ABSENCE.
    ///
    /// `recorded_underlying` answers `None` for a ledger path that does not
    /// exist and refuses one that cannot be inspected at all. A store root
    /// that is a file makes the ledger's path fail with "not a directory",
    /// not "not found": that is a broken store, not an empty one. Nothing held
    /// the difference: a mutant that took every metadata error for `NotFound`
    /// survived the whole suite (Gate 18 on PR #19).
    #[cfg(unix)]
    #[test]
    fn only_an_absent_ledger_answers_no_underlying() {
        let root = crate::scratch::path("detail-ledger-not-a-directory");
        let _ = std::fs::remove_dir_all(&root);
        let _ = std::fs::remove_file(&root);
        assert_eq!(
            super::recorded_underlying(&root, &[0; 32]),
            Ok(None),
            "an absent store holds no ledger"
        );
        std::fs::write(&root, b"a file where the store root belongs")
            .expect("a file at the store root");
        let refused = super::recorded_underlying(&root, &[0; 32]);
        std::fs::remove_file(&root).expect("remove the fixture file");
        let why = refused.expect_err("a ledger path under a file cannot be inspected");
        assert!(
            why.starts_with("ledger path cannot be inspected: "),
            "{why}"
        );
    }

    /// THE BLOCKING TASK'S ORDER, KEPT: PAGE, THEN STORE ROOT, THEN IDENTITY.
    ///
    /// A request with several faults is refused for the one the blocking task
    /// named, so moving the parse ahead of admission changed no refusal text.
    /// The valid case pins the parsed values, upper-case hex included: the
    /// detail grammar is unchanged (D-0689).
    #[test]
    fn a_detail_selector_refuses_page_then_root_then_identity() {
        let unset = || Err::<std::path::PathBuf, _>("no store root".to_owned());
        let set = || Ok(std::path::PathBuf::from("/selector-fixture"));
        assert_eq!(
            Selector::parse(unset(), "identity=x&page=banana").err(),
            Some("`page` must be a canonical unsigned decimal integer".to_owned())
        );
        assert_eq!(
            Selector::parse(unset(), "identity=x").err(),
            Some("no store root".to_owned())
        );
        assert_eq!(
            Selector::parse(set(), "identity=x").err(),
            Some(IDENTITY_REFUSAL.to_owned())
        );
        let asked = Selector::parse(
            set(),
            &format!("identity={}&page=2&limit=3", "Ab".repeat(32)),
        )
        .expect("a valid selector");
        assert_eq!(asked.root, std::path::PathBuf::from("/selector-fixture"));
        assert_eq!(asked.identity, [0xab; 32]);
        assert_eq!(
            asked.page,
            Page {
                number: 2,
                limit: 3
            }
        );
    }

    /// The selector is exact the way every sibling detail route is: an
    /// unknown key (the boolean routes' `offset`, a typo), a repeated key, an
    /// empty value and a non-canonical integer all refuse instead of answering
    /// page 0 (P1-01-04, D-1765).
    #[test]
    fn a_detail_selector_refuses_unknown_keys_and_non_canonical_integers() {
        let set = || Ok(std::path::PathBuf::from("/selector-fixture"));
        let id = "ab".repeat(32);
        for query in [
            format!("identity={id}&offset=256"),
            format!("identity={id}&pgae=3"),
            format!("identity={id}&page=1&page=2"),
            format!("identity={id}&page="),
            format!("identity={id}&limit"),
        ] {
            let why = Selector::parse(set(), &query).err().unwrap_or_default();
            assert!(why.contains("detail query field"), "{query}: {why}");
        }
        for (query, field) in [
            (format!("identity={id}&page=%2B1"), "`page`"),
            (format!("identity={id}&page=003"), "`page`"),
            (format!("identity={id}&limit=016"), "`limit`"),
        ] {
            let why = Selector::parse(set(), &query).err().unwrap_or_default();
            assert!(
                why.contains(field) && why.contains("canonical"),
                "{query}: {why}"
            );
        }
        assert!(Selector::parse(set(), &format!("identity={id}&page=0&limit=16")).is_ok());
        assert!(Page::parse("page=%2B1").is_err());
    }

    #[test]
    fn verified_cache_exposes_refresh_refusal_before_any_later_reopen() {
        let cache = Cached::new();
        let root = std::path::Path::new("/verified-fixture");
        assert_eq!(
            cache.with_verified(root, || Ok(7), |_| Ok(()), |v| *v),
            Ok(7)
        );
        let mut reopened = false;
        let result = cache.with_verified(
            root,
            || {
                reopened = true;
                Ok(9)
            },
            |_| Err("corrupted generation".to_owned()),
            |v| *v,
        );
        assert_eq!(result, Err("corrupted generation".to_owned()));
        assert!(
            !reopened,
            "a successful fallback must not hide the failed validation"
        );
        assert_eq!(
            cache.with_verified(root, || Ok(9), |_| Err("must open".to_owned()), |v| *v),
            Ok(9)
        );
    }

    /// THE SLOT LOCK IS NOT HELD ACROSS AN OPEN, A REFRESH OR `f`. D-2309.
    ///
    /// Every closure below takes the slot's own mutex with `try_lock`, which
    /// fails while the lock is held: before D-2309 the cold `open` and the
    /// O(indexed bytes) `refresh` ran under it, so a request for the handle
    /// waited behind both. And a cold `open` can use the same cache for
    /// another root while it runs -- the call would deadlock if the open
    /// held the slot -- after which its own install still lands and is served.
    #[test]
    fn the_slot_lock_is_free_while_a_handle_opens_or_refreshes() {
        let cache: Cached<u32> = Cached::new();
        let root = std::path::Path::new("/slot-lock-root");
        let free = |cache: &Cached<u32>| cache.inner.try_lock().is_ok();

        let opened = cache.with_verified(
            root,
            || {
                assert!(free(&cache), "a cold open runs outside the slot lock");
                let other = cache.with(
                    std::path::Path::new("/slot-lock-other"),
                    || Ok(5),
                    |_| Ok(()),
                    |h| *h,
                );
                assert_eq!(other, Ok(5), "another root is served during the open");
                Ok(7)
            },
            |_| Err("nothing was cached for this root".to_owned()),
            |h| {
                assert!(free(&cache), "`f` on a fresh handle runs outside it");
                *h
            },
        );
        assert_eq!(opened, Ok(7), "the open that ran second is installed");

        let refreshed = cache.with_verified(
            root,
            || Err("the handle is cached".to_owned()),
            |h| {
                assert!(free(&cache), "a verified refresh runs outside it");
                *h += 1;
                Ok(())
            },
            |h| {
                assert!(free(&cache), "`f` on a cached handle runs outside it");
                *h
            },
        );
        assert_eq!(refreshed, Ok(8));

        let reopened = cache.with(
            root,
            || {
                assert!(free(&cache), "a reopen after a refusal runs outside it");
                Ok(11)
            },
            |_| {
                assert!(free(&cache), "a refresh runs outside it");
                Err("the file shrank".to_owned())
            },
            |h| *h,
        );
        assert_eq!(reopened, Ok(11));
        assert!(free(&cache), "nothing is left holding the slot");
    }

    /// A CACHED HANDLE IS OPENED ONCE, REFRESHED AFTER, AND REOPENED ON A REFUSAL.
    ///
    /// Three facts, each a counter: `open` runs on the first call and not the
    /// second; `refresh` runs on the second and not the first; and a refresh
    /// that refuses is answered by exactly one more `open` and never by an
    /// error, while an `open` that fails IS the error and leaves the slot for
    /// the next call to judge.
    #[test]
    #[expect(
        clippy::too_many_lines,
        reason = "five facts about ONE cache's accumulated state: each later call is \
                  only meaningful because of what the earlier calls left in the slot, \
                  so splitting them would test five fresh caches and none of the \
                  carry-over this exists to pin"
    )]
    fn a_cached_handle_opens_once_refreshes_after_and_reopens_on_refusal() {
        let cache: Cached<u32> = Cached::new();
        let root = std::path::Path::new("/one-root");
        let mut opens = 0_u32;
        let mut refreshes = 0_u32;

        let first = cache.with(
            root,
            || {
                opens += 1;
                Ok(7)
            },
            |_| {
                refreshes += 1;
                Ok(())
            },
            |h| *h,
        );
        assert_eq!(
            first,
            Ok(7),
            "the first call opens and hands the handle through"
        );
        assert_eq!((opens, refreshes), (1, 0), "opened once, refreshed never");

        let second = cache.with(
            root,
            || {
                opens += 1;
                Ok(99)
            },
            |_| {
                refreshes += 1;
                Ok(())
            },
            |h| *h,
        );
        assert_eq!(second, Ok(7), "the second call reuses the handle it opened");
        assert_eq!((opens, refreshes), (1, 1), "refreshed once, NOT reopened");

        let after_refusal = cache.with(
            root,
            || {
                opens += 1;
                Ok(11)
            },
            |_| {
                refreshes += 1;
                Err("the file shrank".to_owned())
            },
            |h| *h,
        );
        assert_eq!(
            after_refusal,
            Ok(11),
            "a refresh that refuses is answered by one fresh open, not an error"
        );
        assert_eq!((opens, refreshes), (2, 2));

        let other_root = cache.with(
            std::path::Path::new("/another-root"),
            || {
                opens += 1;
                Ok(13)
            },
            |_| {
                refreshes += 1;
                Ok(())
            },
            |h| *h,
        );
        assert_eq!(other_root, Ok(13), "a different root is a different handle");
        assert_eq!(
            (opens, refreshes),
            (3, 2),
            "reopened without refreshing the other root"
        );

        let failed_open = cache.with(
            std::path::Path::new("/a-third-root"),
            || {
                opens += 1;
                Err("no such file".to_owned())
            },
            |_| {
                refreshes += 1;
                Ok(())
            },
            |h| *h,
        );
        assert_eq!(
            failed_open,
            Err("no such file".to_owned()),
            "an open that fails is the error, verbatim"
        );
        assert_eq!((opens, refreshes), (4, 2));

        let still_previous = cache.with(
            std::path::Path::new("/another-root"),
            || {
                opens += 1;
                Ok(0)
            },
            |_| {
                refreshes += 1;
                Ok(())
            },
            |h| *h,
        );
        assert_eq!(
            still_previous,
            Ok(13),
            "a failed open left the prior handle in place for the next request to judge"
        );
        assert_eq!((opens, refreshes), (4, 3));
    }

    #[test]
    fn pagination_refuses_malformed_zero_and_over_limit_values() {
        assert_eq!(Page::parse("").expect("defaults").limit, MAX_PAGE_ROWS);
        assert!(Page::parse("limit=0").is_err());
        assert!(Page::parse("limit=257").is_err());
        assert!(Page::parse("page=-1").is_err());
        assert!(Page::parse("page=banana").is_err());
        assert!(Page::parse(&"x".repeat(MAX_QUERY_BYTES + 1)).is_err());
    }

    #[test]
    fn preflight_refuses_an_oversized_file_from_metadata_without_reading_it() {
        let path = std::env::temp_dir().join(format!(
            "brutex-detail-preflight-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_file(&path);
        let file = std::fs::File::create(&path).expect("sparse fixture");
        file.set_len(MAX_SCAN_BYTES + 1).expect("sparse length");
        let why = preflight([("fixture", path.as_path())]).expect_err("one byte over");
        assert!(why.contains(&(MAX_SCAN_BYTES + 1).to_string()), "{why}");
        assert!(why.contains(&MAX_SCAN_BYTES.to_string()), "{why}");
        assert!(why.contains("No partial index"), "{why}");
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn windows_name_partial_complete_and_out_of_range_pages() {
        let first = window(300, Page::parse("limit=256").expect("page")).expect("first");
        assert_eq!((first.start, first.end), (0, 256));
        assert!(!first.complete);
        assert_eq!(first.next_page, Some(1));
        let whole = window(2, Page::parse("").expect("page")).expect("whole");
        assert!(whole.complete);
        assert!(window(2, Page::parse("page=1&limit=2").expect("page")).is_err());
    }

    /// A seeked page is bounded by its own limit, not by the hold cap, and it
    /// still refuses every coordinate `window` refuses. W1-api6-5, D-0954.
    #[test]
    fn a_seeked_window_has_no_hold_cap_and_keeps_every_other_refusal() {
        let page = |query: &str| Page::parse(query).expect("a valid page");
        let held = usize::try_from(MAX_RESULT_ROWS).expect("fits");
        let pages = usize::try_from(MAX_PAGE + 1).expect("fits");
        // The hold cap: `window` still refuses one row past it, at page 0.
        assert!(window(held, page("")).is_ok());
        let refused = window(held + 1, page("")).expect_err("held cap");
        assert!(
            refused.contains("one request verifies at most"),
            "{refused}"
        );
        // A seeked window over the same totals is a page, not a refusal.
        for total in [held, held + 1, 10_000] {
            let first = seek_window(total, page("")).expect("page 0");
            assert_eq!((first.start, first.end, first.complete), (0, 256, false));
            assert_eq!(first.next_page, Some(1));
            let last_page = (total - 1) / 256;
            let last = seek_window(total, page(&format!("page={last_page}"))).expect("last");
            assert_eq!(
                (last.start, last.end, last.next_page),
                (last_page * 256, total, None)
            );
            assert!(seek_window(total, page(&format!("page={}", last_page + 1))).is_err());
        }
        // The empty result: page 0 only, and it is complete.
        let empty = seek_window(0, page("")).expect("empty page 0");
        assert_eq!(
            (empty.start, empty.end, empty.complete, empty.next_page),
            (0, 0, true, None)
        );
        assert!(seek_window(0, page("page=1")).is_err());
        // Addressability: exactly (MAX_PAGE + 1) * limit rows is pageable to
        // its last row at the largest page number; one more is refused on
        // page 0 with the smallest limit that would reach it.
        let reach = pages * usize::try_from(MAX_PAGE_ROWS).expect("fits");
        let deepest = seek_window(reach, page(&format!("page={MAX_PAGE}"))).expect("deepest");
        assert_eq!((deepest.end, deepest.next_page), (reach, None));
        let over = seek_window(reach + 1, page("")).expect_err("past the deepest page");
        assert!(
            over.contains(&format!("ask `limit={}`", MAX_PAGE_ROWS + 1)),
            "{over}"
        );
        let narrow = seek_window(pages + 1, page("limit=1")).expect_err("limit 1");
        assert!(narrow.contains("ask `limit=2`"), "{narrow}");
        assert!(seek_window(pages, page(&format!("page={MAX_PAGE}&limit=1"))).is_ok());
        // usize::MAX is a total no page can reach, and refusing it cannot overflow.
        assert!(seek_window(usize::MAX, page("")).is_err());
    }

    #[tokio::test(flavor = "current_thread")]
    async fn a_store_read_runs_off_the_worker_and_refuses_past_its_bound() {
        let _serial = super::TEST_SERIAL.lock().await;
        let worker = std::thread::current().id();
        let blocking = run_store_read(|| std::thread::current().id())
            .await
            .expect("admitted store read");
        assert_ne!(blocking, worker, "spawn_blocking owns a different thread");
        // Hold every slot; the next is refused before it queues, and a
        // released slot admits again. D-1508.
        let held: Vec<Permit> = (0..MAX_STORE_READ_CONCURRENT)
            .map(|_| {
                Permit::try_take_from(&STORE_READ_ACTIVE, MAX_STORE_READ_CONCURRENT)
                    .expect("a free slot")
            })
            .collect();
        assert_eq!(run_store_read(|| ()).await, Err(RunError::Saturated));
        // The other pools are not this one's.
        assert_eq!(run_calendar(|| 7).await, Ok(7));
        drop(held);
        assert_eq!(run_store_read(|| 9).await, Ok(9));
        assert_eq!(STORE_READ_ACTIVE.load(Ordering::Acquire), 0);
    }

    /// A log read runs on a blocking thread, and past
    /// [`MAX_LOG_READ_CONCURRENT`] it is refused before it queues, without
    /// touching the other pools. log-3, P1-04-02, D-2327.
    #[tokio::test(flavor = "current_thread")]
    async fn a_log_read_runs_off_the_worker_and_refuses_past_its_bound() {
        let apart = super::apart_from_slot_owners().await;
        let worker = std::thread::current().id();
        let blocking = run_log_read(|| std::thread::current().id())
            .await
            .expect("admitted log read");
        assert_ne!(blocking, worker, "spawn_blocking owns a different thread");
        let held = super::take_every_log_read_slot(&apart);
        assert_eq!(held.len(), 4, "this test owns every log-read slot");
        assert_eq!(MAX_LOG_READ_CONCURRENT, 4);
        assert_eq!(run_log_read(|| ()).await, Err(RunError::Saturated));
        // The other pools are not this one's.
        assert_eq!(run_calendar(|| 7).await, Ok(7));
        drop(held);
        assert_eq!(run_log_read(|| 9).await, Ok(9));
        assert_eq!(LOG_READ_ACTIVE.load(Ordering::Acquire), 0);
    }

    #[test]
    fn a_refused_admission_names_what_its_bound_and_why() {
        let (status, body) = admission_refused("folder read", 8, &RunError::Saturated);
        assert_eq!(status, axum::http::StatusCode::TOO_MANY_REQUESTS);
        assert!(
            body.contains("folder read not admitted (Saturated): at most 8"),
            "{body}"
        );
        let (status, body) =
            admission_refused("folder read", 8, &RunError::Join("gone".to_owned()));
        assert_eq!(status, axum::http::StatusCode::SERVICE_UNAVAILABLE);
        assert!(body.contains("gone"), "{body}");
        let parsed: serde_json::Value = serde_json::from_str(&body).expect("JSON");
        assert!(
            parsed
                .get("error")
                .is_some_and(serde_json::Value::is_string)
        );
    }

    #[tokio::test(flavor = "current_thread")]
    async fn admitted_work_runs_off_the_tokio_worker() {
        let _serial = super::TEST_SERIAL.lock().await;
        let worker = std::thread::current().id();
        let blocking = run(|| std::thread::current().id())
            .await
            .expect("admitted blocking task");
        assert_ne!(blocking, worker, "spawn_blocking owns a different thread");
    }
}
