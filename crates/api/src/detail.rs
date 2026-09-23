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
/// Maximum rows rendered in one response page.
pub const MAX_PAGE_ROWS: u64 = 256;
/// Maximum zero-based page accepted at the HTTP boundary.
pub const MAX_PAGE: u64 = MAX_RESULT_ROWS - 1;
/// Maximum query bytes parsed by a detail endpoint.
pub const MAX_QUERY_BYTES: usize = 512;
/// Maximum JSON bytes returned by a successful or partial detail response.
pub const MAX_RESPONSE_BYTES: usize = 8 * 1024 * 1024;

static ACTIVE: AtomicUsize = AtomicUsize::new(0);

/// One admitted detail request.  Dropping it always returns the slot.
pub(crate) struct Permit;

impl Permit {
    fn try_take() -> Option<Self> {
        ACTIVE
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |active| {
                (active < MAX_CONCURRENT).then_some(active.saturating_add(1))
            })
            .ok()
            .map(|_| Self)
    }
}

impl Drop for Permit {
    fn drop(&mut self) {
        let previous = ACTIVE.fetch_sub(1, Ordering::AcqRel);
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

fn integer_param(query: &str, name: &str) -> Result<Option<u64>, String> {
    let value = crate::server::param(query, name);
    if value.is_empty() {
        return Ok(None);
    }
    value
        .parse::<u64>()
        .map(Some)
        .map_err(|_| format!("`{name}` must be an unsigned decimal integer"))
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
/// A shrunken file, a torn tail, a duplicate identity, a file moved aside on a
/// format-version bump -- each is a reason the handle no longer describes what
/// is on disk. The answer is one fresh `open`, which is the single O(rows) path
/// a cache should ever take, and the refusal that caused it is never swallowed:
/// if the reopen also fails, that error is the response.
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
pub struct Cached<T> {
    inner: std::sync::Mutex<Option<(std::path::PathBuf, T)>>,
}

impl<T> Cached<T> {
    /// An empty slot. `const` so it can back a `static`.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            inner: std::sync::Mutex::new(None),
        }
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
        // READ THROUGH A POISONED LOCK, for the reason `calendar_of` gives: a
        // panic while holding it means some other request died, and the handle
        // is still a handle. Refusing to look would make one panicked request
        // cost every later one the walk this exists to remove.
        let mut held = self
            .inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some((at, handle)) = held.as_mut()
            && at.as_path() == root
            && refresh(handle).is_ok()
        {
            return Ok(f(handle));
        }
        // Absent, a different root, or a refresh that refused: open fresh. The
        // slot is overwritten rather than cleared first, so a failed `open`
        // leaves whatever was there -- which the next request refreshes and
        // judges again on its own terms.
        let (_, handle) = held.insert((root.to_path_buf(), open()?));
        Ok(f(handle))
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
        let mut held = self
            .inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some((at, handle)) = held.as_mut()
            && at.as_path() == root
        {
            if let Err(why) = refresh(handle) {
                *held = None;
                return Err(why);
            }
            return Ok(f(handle));
        }
        let (_, handle) = held.insert((root.to_path_buf(), open()?));
        Ok(f(handle))
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

/// Refreshes both parent indexes once and returns one owned receipt before
/// the caller refreshes any child. Cold open is O(history); warm refresh is
/// O(new parent rows), with each file still subject to the HTTP byte ceiling.
///
/// # Errors
/// Returns parent generation/integrity/size failures and missing committed
/// receipts; no child is exposed after an unsuccessful parent refresh.
pub fn committed_receipt(
    root: &Path,
    identity: &[u8; 32],
) -> Result<Option<cli::result_set::Receipt>, String> {
    PARENTS.with_verified(
        root,
        || cli::result_set::CommittedParents::open_read_bounded(root, MAX_SCAN_BYTES),
        cli::result_set::CommittedParents::refresh,
        |parents| parents.receipt(identity),
    )?
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
        Cached, IDENTITY_REFUSAL, MAX_PAGE_ROWS, MAX_QUERY_BYTES, MAX_SCAN_BYTES, Page, Selector,
        preflight, run, window,
    };

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
            Some("`page` must be an unsigned decimal integer".to_owned())
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
