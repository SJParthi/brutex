//! One bounded door for the two disk-backed result-detail endpoints.
//!
//! Both `/trades.json` and `/frontier.json` rebuild several fixed-stride
//! indexes before they can answer an identity lookup.  That work is blocking
//! file I/O and CPU work, so it must not occupy a Tokio worker.  It is also
//! bounded twice: a small process-wide admission counter limits queued plus
//! running blocking tasks, and callers preflight every file they will index
//! against [`MAX_SCAN_BYTES`].

use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};

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

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    reason = "tests fail through assertions"
)]
mod tests {
    use super::{MAX_PAGE_ROWS, MAX_QUERY_BYTES, MAX_SCAN_BYTES, Page, preflight, run, window};

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
