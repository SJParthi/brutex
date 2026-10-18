//! Shared bounded admission for blocking sweep detail, live and status reads.
//! Cold fixed-stride indexes cost O(history); refreshed indexes consume new
//! records. Neither blocking file reads nor formatting occupies a Tokio worker.
//! Admission limits running tasks per pool; a request that finds its pool full
//! waits at most [`ADMISSION_WAIT_MS`] in a queue of at most [`MAX_WAITING`]
//! before it is refused with 429 and `Retry-After` (DB-B51, WP-81). Readers
//! enforce byte/row caps.

use std::path::Path;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

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

/// Operations reads — `/scrub.json` and `/disk.json` — that may be queued or
/// running at once on the blocking pool (DB-B74, DB-B73; WP-81 repair 2).
///
/// A pool of its own for the reason [`MAX_CALENDAR_CONCURRENT`] has one, and
/// measured: these two routes took [`run_store_read`]'s slots, which every
/// `/bars/window.json` seek also takes. While a scrub appended to a ledger of
/// 385,419 findings, 20 clients polling `/scrub.json` had 283 of 960
/// concurrent bar seeks refused 429. A page polling what a scrub found must
/// never be what refuses `/db` its bars. Each admitted read is bounded
/// (`scrub_route`'s cost section); the one long one — the first index of a
/// long ledger after a start — holds the vendor's lock for at most one
/// 16 MiB step, and the requests behind it wait here, not in the seeks' pool.
pub const MAX_OPS_READ_CONCURRENT: usize = 4;

/// `/store.json` census reads that may be queued or running at once on the
/// blocking pool (DB-B51, WP-81; D-3327).
///
/// A pool of its own for the reason [`MAX_CALENDAR_CONCURRENT`] has one. The
/// census read used [`run`]'s four slots, which the audit journal of every
/// audited route (`/backtest/run`, `/live.json` …) also takes: ten `/db` tabs
/// opening at once answered 429 on the census to 18 of 30 requests and held
/// the slots a backtest launch needed. A first encode of the GDFL census is
/// about 1.9 s; every later read of an unchanged census is served from the
/// wire cache in milliseconds.
pub const MAX_CENSUS_CONCURRENT: usize = 4;

/// How long a request waits for a slot in a full pool before it is refused,
/// in milliseconds (DB-B51, WP-81).
///
/// A choice, not a measurement of anything outside this process. The reads
/// these pools admit are short — a seek, a facet, a page of imports answers
/// in about a millisecond on the real store — so a quarter of a second turns
/// the refusals ten tabs met (271 of 480 seeks, 279 of 480 facets) into a
/// short wait, while a pool held by genuinely long work still refuses within
/// a quarter second, naming its bound, with `Retry-After`.
pub const ADMISSION_WAIT_MS: u64 = 250;

/// The most requests that may WAIT for one pool at once (DB-B51, WP-81).
/// Past it a request is refused at once: the queue is bounded, so a burst
/// cannot park an unbounded number of tasks on one pool.
pub const MAX_WAITING: usize = 32;

/// How often a pool's clock wakes its waiters so each can read its deadline,
/// in milliseconds. A waiter is refused at most this long past
/// [`ADMISSION_WAIT_MS`].
const CLOCK_TICK_MS: u64 = 10;

/// Window scans (a non-time sort, a fold of the extremes, the O(days) work of
/// a time-of-day bound or `coverage=1`) that may run at once: the scan pool's
/// ceiling, apart from [`run_store_read`]'s so a seek never waits behind a
/// scan (D-3326, DB-066). It was a counter of its own in `bars_route` that
/// refused at once with no wait and no name; since R3-08 it is a named pool
/// here like every other.
pub const MAX_SCAN_CONCURRENT: usize = 2;

/// Whole-vendor checks (`/verify.json`) that may run at once: ONE (R3-08,
/// RA-009). A check opens one bar file per held month — 246,147 for GDFL when
/// it was timed, 158 to 172 s a request — and it used a slot of the store-read
/// pool every `/db` seek takes, so eight loads of it took every page slot for
/// minutes. Its own pool of one, so at most one runs and the bar pages keep
/// all of theirs; a second asks again after `Retry-After`.
pub const MAX_VERIFY_CONCURRENT: usize = 1;

/// The feed list and the settings reads (`/feeds.json`, `/settings.json`)
/// that may run at once: the list pool's ceiling (R3-08, NEW-04, RA-020).
///
/// R3-05 moved both bodies off the async worker onto Tokio's own blocking
/// pool, which has no name, no bound a page can see and no `Retry-After`: a
/// burst of cold feed lists (a held-span walk of 0.66 to 2 s) started that
/// many threads with nothing counted. Their own pool, apart from every page
/// read's, so the feed list — the page's first answer and the canary the
/// load test times beside 32 looping clients — never waits behind a month
/// read, a scan or the whole-vendor check.
pub const MAX_LIST_CONCURRENT: usize = 4;

/// How long a page read admitted to one of these pools waits for a WRITER
/// that holds the month it opens (an import worker, a month repair, an index
/// build) before it is answered "being written, ask again" — never
/// "unreadable" (R3-08, NEW-21, RA-117; `store::flock::with_patience`).
///
/// A choice, not a measurement of a writer: an import holds a month for the
/// length of a day's append, so a short wait catches a writer that is just
/// letting go, and a writer that is not answers within a fifth of a second
/// with `Retry-After` rather than holding a page slot for the writer's whole
/// run.
pub const READ_PATIENCE_MS: u64 = 200;

/// How many slots a pool has: from the machine's core count, never past a
/// ceiling, or fixed (R3-08: "pool sizes come from the core count").
#[derive(Debug, Clone, Copy)]
pub(crate) enum Size {
    /// One slot per core the host reports, at least one and at most `cap`.
    Cores {
        /// The ceiling; the `MAX_*` constant the pool's refusal names.
        cap: usize,
    },
    /// Exactly this many, whatever the machine.
    Fixed(usize),
}

/// The cores this process may use, as the host reports them; one when it
/// cannot say.
fn cores() -> usize {
    std::thread::available_parallelism().map_or(1, std::num::NonZeroUsize::get)
}

/// The slots a [`Size::Cores`] pool with ceiling `cap` has on this machine:
/// one per core, at least one, at most `cap`.
fn slots_under(cap: usize) -> usize {
    cores().clamp(1, cap.max(1))
}

/// One bounded, NAMED pool: the slots running, the requests waiting, the
/// wake-up a returned slot sends to one waiter, the clock that ends a wait,
/// and what it did — answered, refused busy, abandoned by its reader — so a
/// page can show it (`/pools.json`, R3-08).
pub(crate) struct Pool {
    /// The name every refusal, count and table uses.
    name: &'static str,
    active: AtomicUsize,
    waiting: AtomicUsize,
    freed: tokio::sync::Notify,
    size: Size,
    /// The slot count, decided from [`Size`] on first use.
    slots: std::sync::OnceLock<usize>,
    /// The thread that wakes this pool's waiters every [`CLOCK_TICK_MS`]
    /// while any wait; started by the first wait, parked while none does.
    /// `None` inside: the thread could not be started, and nothing waits.
    clock: std::sync::OnceLock<Option<std::thread::Thread>>,
    /// Work this pool ran to its end for a reader still there.
    answered: AtomicU64,
    /// Requests refused because every slot stayed taken.
    busy: AtomicU64,
    /// Requests whose reader left: while waiting for a slot, or while the
    /// work ran (the work saw the flag).
    cancelled: AtomicU64,
}

impl Pool {
    /// A named pool of `size` slots, none taken. `const` so it can back a
    /// `static`.
    pub(crate) const fn new(name: &'static str, size: Size) -> Self {
        Self {
            name,
            active: AtomicUsize::new(0),
            waiting: AtomicUsize::new(0),
            freed: tokio::sync::Notify::const_new(),
            size,
            slots: std::sync::OnceLock::new(),
            clock: std::sync::OnceLock::new(),
            answered: AtomicU64::new(0),
            busy: AtomicU64::new(0),
            cancelled: AtomicU64::new(0),
        }
    }

    /// How many slots this pool has on this machine.
    pub(crate) fn max(&self) -> usize {
        *self.slots.get_or_init(|| match self.size {
            Size::Cores { cap } => slots_under(cap),
            // A FIXED BOUND IS TAKEN AS WRITTEN: zero admits nothing, as it
            // always did (`a_pool_admits_its_bound_and_a_dropped_permit_frees_its_slot`).
            Size::Fixed(slots) => slots,
        })
    }

    /// This pool's name.
    pub(crate) const fn name(&self) -> &'static str {
        self.name
    }

    /// This pool's clock, started on the first wait; `None` when the thread
    /// could not be started.
    ///
    /// THE WAIT IS TIMED BY A THREAD OF THIS PROCESS, NOT BY THE RUNTIME'S
    /// TIMER. It was `tokio::time::timeout_at`, which PANICS on a runtime
    /// built without a timer ("timers are disabled") — and only when the pool
    /// is full, so the panic came and went with the load: on the merged tree
    /// a test that drives a route on such a runtime died in two of four runs
    /// of the suite, whenever other tests held the slots at that instant.
    /// Before the wait existed a full pool refused at once on any runtime;
    /// a wait must not turn that refusal into a panic. The clock asks nothing
    /// of the runtime: it wakes the waiters, and each reads its own deadline
    /// on `std::time::Instant`.
    ///
    /// Cost: one thread per pool that has ever waited (five in the server),
    /// parked while nobody waits; while somebody does, one wake-up per waiter
    /// every [`CLOCK_TICK_MS`], each one failed `try_take` — at most
    /// [`MAX_WAITING`] of them, for at most [`ADMISSION_WAIT_MS`].
    fn clock(&'static self) -> Option<&'static std::thread::Thread> {
        self.clock
            .get_or_init(|| {
                std::thread::Builder::new()
                    .name("admission-clock".to_owned())
                    .spawn(move || {
                        self.tick();
                    })
                    .ok()
                    .map(|handle| handle.thread().clone())
            })
            .as_ref()
    }

    /// The clock thread's whole life: parked while nobody waits, and while
    /// somebody does, every waiter woken once per [`CLOCK_TICK_MS`].
    fn tick(&'static self) -> ! {
        loop {
            if self.waiting.load(Ordering::Acquire) == 0 {
                // Nobody waits: sleep until a waiter unparks this thread. An
                // `unpark` sent just before this `park` makes it return at
                // once, so a waiter that arrives in between is not missed.
                std::thread::park();
                continue;
            }
            std::thread::sleep(std::time::Duration::from_millis(CLOCK_TICK_MS));
            self.freed.notify_waiters();
        }
    }

    /// A slot now, or `None` when every slot is taken.
    fn try_take(&'static self) -> Option<Permit> {
        let max = self.max();
        self.active
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |active| {
                (active < max).then_some(active.saturating_add(1))
            })
            .ok()
            .map(|_| Permit(self))
    }

    /// A slot now, or within [`ADMISSION_WAIT_MS`] when one is returned, or
    /// `None`: at once when [`MAX_WAITING`] requests already wait, else when
    /// the wait ends with every slot still taken (DB-B51).
    ///
    /// O(1) per wake-up: a returned slot wakes one waiter, which takes it or
    /// waits again until its deadline. A newcomer may take a returned slot
    /// before the waiter it woke; the waiter then waits on, never past its
    /// deadline by more than one tick of the pool's clock, so nothing waits
    /// unboundedly. It needs no timer of the runtime it is polled on
    /// ([`Pool::clock`]).
    ///
    /// UNVERIFIED: this cost names no test or bench that proves it (docs/06-limits.md,
    /// "Cost claims of the /db fix round that name no proof").
    async fn take(&'static self) -> Option<Permit> {
        let taken = self.queue().await;
        if taken.is_none() {
            self.busy.fetch_add(1, Ordering::Relaxed);
        }
        taken
    }

    /// [`Self::take`]'s wait, counted: a reader that leaves while it waits —
    /// the request's future dropped with this one — is counted cancelled.
    async fn queue(&'static self) -> Option<Permit> {
        if let Some(permit) = self.try_take() {
            return Some(permit);
        }
        self.waiting
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |waiting| {
                (waiting < MAX_WAITING).then_some(waiting.saturating_add(1))
            })
            .ok()?;
        let _queued = Queued(&self.waiting);
        let left = Leaving(Some(&self.cancelled));
        // NO CLOCK, NO WAIT: a wait nothing can end is refused at once, as a
        // full pool was before the wait existed.
        let Some(clock) = self.clock() else {
            left.stay();
            return None;
        };
        let deadline =
            std::time::Instant::now() + std::time::Duration::from_millis(ADMISSION_WAIT_MS);
        // Counted in `waiting` above, so the clock ticks until this request
        // leaves the queue; one `unpark` starts it if it was parked.
        clock.unpark();
        loop {
            // REGISTERED BEFORE THE CHECK, so a slot returned, or a tick
            // sent, between the check and the wait still wakes this waiter.
            let freed = self.freed.notified();
            let mut freed = std::pin::pin!(freed);
            freed.as_mut().enable();
            if let Some(permit) = self.try_take() {
                left.stay();
                return Some(permit);
            }
            if std::time::Instant::now() >= deadline {
                left.stay();
                return None;
            }
            // Woken by a returned slot or by the clock's next tick.
            freed.await;
        }
    }
}

/// A waiter that has not been answered yet: dropped before [`Self::stay`],
/// its reader left while it waited, and the pool counts it cancelled.
struct Leaving(Option<&'static AtomicU64>);

impl Leaving {
    /// The wait ended with an answer (a slot or a refusal): nothing to count.
    fn stay(mut self) {
        self.0 = None;
    }
}

impl Drop for Leaving {
    fn drop(&mut self) {
        if let Some(cancelled) = self.0.take() {
            cancelled.fetch_add(1, Ordering::Relaxed);
        }
    }
}

/// One waiting request; dropping it leaves the queue.
struct Queued(&'static AtomicUsize);

impl Drop for Queued {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::AcqRel);
    }
}

static ACTIVE: Pool = Pool::new(
    "detail",
    Size::Cores {
        cap: MAX_CONCURRENT,
    },
);
static CALENDAR: Pool = Pool::new(
    "calendar",
    Size::Cores {
        cap: MAX_CALENDAR_CONCURRENT,
    },
);
static STORE_READ: Pool = Pool::new(
    "store_read",
    Size::Cores {
        cap: MAX_STORE_READ_CONCURRENT,
    },
);
static LOG_READ: Pool = Pool::new(
    "log_read",
    Size::Cores {
        cap: MAX_LOG_READ_CONCURRENT,
    },
);
static OPS_READ: Pool = Pool::new(
    "ops_read",
    Size::Cores {
        cap: MAX_OPS_READ_CONCURRENT,
    },
);
static CENSUS: Pool = Pool::new(
    "census",
    Size::Cores {
        cap: MAX_CENSUS_CONCURRENT,
    },
);
static SCAN: Pool = Pool::new(
    "scan",
    Size::Cores {
        cap: MAX_SCAN_CONCURRENT,
    },
);
static VERIFY: Pool = Pool::new("verify", Size::Fixed(MAX_VERIFY_CONCURRENT));
static LIST: Pool = Pool::new(
    "list",
    Size::Cores {
        cap: MAX_LIST_CONCURRENT,
    },
);

/// One admitted request in one pool.  Dropping it always returns the slot,
/// and wakes one request waiting for it.
pub(crate) struct Permit(&'static Pool);

impl Permit {
    /// A slot for work an already-admitted request owes, taken past the cap.
    ///
    /// It is never refused, and it still counts: while it is held,
    /// [`run`] sees one more active task and refuses new work sooner. D-1445.
    fn owed() -> Self {
        ACTIVE.active.fetch_add(1, Ordering::AcqRel);
        Self(&ACTIVE)
    }
}

impl Drop for Permit {
    fn drop(&mut self) {
        let previous = self.0.active.fetch_sub(1, Ordering::AcqRel);
        debug_assert!(previous > 0, "a detail permit is released exactly once");
        self.0.freed.notify_one();
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
/// Returns [`RunError::Saturated`] when all four slots stay occupied for
/// [`ADMISSION_WAIT_MS`] (or [`MAX_WAITING`] requests already wait), or
/// [`RunError::Join`] when Tokio cannot join the blocking task.
pub async fn run<T, F>(work: F) -> Result<T, RunError>
where
    T: Send + 'static,
    F: FnOnce() -> T + Send + 'static,
{
    let permit = ACTIVE.take().await.ok_or(RunError::Saturated)?;
    admitted(permit, work).await
}

/// Runs a `/store.json` census read outside Tokio's worker pool, in the
/// census pool of [`MAX_CENSUS_CONCURRENT`] slots, apart from [`run`]'s
/// (DB-B51, WP-81).
///
/// # Errors
///
/// [`RunError::Saturated`] when every census slot stays occupied for
/// [`ADMISSION_WAIT_MS`], or [`RunError::Join`] when Tokio cannot join the
/// blocking task.
pub async fn run_census<T, F>(work: F) -> Result<T, RunError>
where
    T: Send + 'static,
    F: FnOnce() -> T + Send + 'static,
{
    let permit = CENSUS.take().await.ok_or(RunError::Saturated)?;
    admitted(permit, work).await
}

/// Runs a calendar derivation outside Tokio's worker pool, in the calendar
/// pool of [`MAX_CALENDAR_CONCURRENT`] slots. W1-api2-11, D-1443.
///
/// # Errors
///
/// [`RunError::Saturated`] when every calendar slot stays occupied for
/// [`ADMISSION_WAIT_MS`], or [`RunError::Join`] when Tokio cannot join the blocking task.
pub async fn run_calendar<T, F>(work: F) -> Result<T, RunError>
where
    T: Send + 'static,
    F: FnOnce() -> T + Send + 'static,
{
    let permit = CALENDAR.take().await.ok_or(RunError::Saturated)?;
    admitted(permit, work).await
}

/// Runs an operator route's folder or masters read outside Tokio's worker
/// pool, in its own pool of [`MAX_STORE_READ_CONCURRENT`] slots. W1-api2-11,
/// D-1508.
///
/// # Errors
///
/// [`RunError::Saturated`] when every slot stays occupied for
/// [`ADMISSION_WAIT_MS`], or
/// [`RunError::Join`] when Tokio cannot join the blocking task.
pub async fn run_store_read<T, F>(work: F) -> Result<T, RunError>
where
    T: Send + 'static,
    F: FnOnce() -> T + Send + 'static,
{
    let permit = STORE_READ.take().await.ok_or(RunError::Saturated)?;
    admitted(permit, work).await
}

/// Runs a `/scrub.json` or `/disk.json` read outside Tokio's worker pool, in
/// its own pool of [`MAX_OPS_READ_CONCURRENT`] slots, apart from the bar
/// seeks' [`run_store_read`] (WP-81 repair 2).
///
/// # Errors
///
/// [`RunError::Saturated`] when every slot stays occupied for
/// [`ADMISSION_WAIT_MS`], or
/// [`RunError::Join`] when Tokio cannot join the blocking task.
pub async fn run_ops_read<T, F>(work: F) -> Result<T, RunError>
where
    T: Send + 'static,
    F: FnOnce() -> T + Send + 'static,
{
    let permit = OPS_READ.take().await.ok_or(RunError::Saturated)?;
    admitted(permit, work).await
}

/// Runs a `/logs.json` or `/logs` tail walk outside Tokio's worker pool, in
/// its own pool of [`MAX_LOG_READ_CONCURRENT`] slots. log-3, P1-04-02, D-2327.
///
/// # Errors
///
/// [`RunError::Saturated`] when every slot stays occupied for
/// [`ADMISSION_WAIT_MS`], or
/// [`RunError::Join`] when Tokio cannot join the blocking task.
pub async fn run_log_read<T, F>(work: F) -> Result<T, RunError>
where
    T: Send + 'static,
    F: FnOnce() -> T + Send + 'static,
{
    let permit = LOG_READ.take().await.ok_or(RunError::Saturated)?;
    admitted(permit, work).await
}

/// The status and JSON body a route answers when its blocking work was not
/// admitted (429) or could not be joined (503), naming `what` and its bound.
/// D-1508.
///
/// `bound` is the pool's ceiling (a `MAX_*` constant); the sentence names the
/// slots the pool has ON THIS MACHINE (R3-08: sizes come from the core
/// count), which is the same number for every pool sized by
/// [`Size::Cores`] and for the verify pool of one — so a smaller host is
/// never told a ceiling it does not have.
#[must_use]
pub fn admission_refused(
    what: &str,
    bound: usize,
    why: &RunError,
) -> (axum::http::StatusCode, String) {
    let bound = slots_under(bound);
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

/// Runs admitted work on the blocking pool, its slot held until it ends,
/// PATIENT for a writer on the months it opens ([`READ_PATIENCE_MS`]), and
/// counted: answered when its reader is still there, cancelled when the
/// reader left while it ran (the request's future dropped with this one).
async fn admitted<T, F>(permit: Permit, work: F) -> Result<T, RunError>
where
    T: Send + 'static,
    F: FnOnce() -> T + Send + 'static,
{
    let pool = permit.0;
    let left = Leaving(Some(&pool.cancelled));
    let joined = tokio::task::spawn_blocking(move || {
        let _permit = permit;
        store::flock::with_patience(std::time::Duration::from_millis(READ_PATIENCE_MS), work)
    })
    .await
    .map_err(|why| RunError::Join(why.to_string()));
    left.stay();
    if joined.is_ok() {
        pool.answered.fetch_add(1, Ordering::Relaxed);
    }
    joined
}

/// The pools by name, for a caller that chooses one at run time and for the
/// route table that names each route's pool (`pool_table_tests`, R3-08).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Named {
    /// [`run`]: sweep detail, live and status reads, the audit journal.
    Detail,
    /// [`run_calendar`]: calendar derivations and the gaps audit.
    Calendar,
    /// [`run_store_read`]: bar seeks, facets, chains, folder and source reads.
    StoreRead,
    /// [`run_log_read`]: log tails.
    LogRead,
    /// [`run_ops_read`]: scrub and disk reads.
    OpsRead,
    /// [`run_census`]: `/store.json`.
    Census,
    /// [`run_scan`]: window scans.
    Scan,
    /// [`run_verify`]: the whole-vendor check.
    Verify,
    /// [`run_list`]: the feed list and the settings reads.
    List,
}

impl Named {
    /// Every pool by name, in the order `/pools.json` lists them: one list,
    /// so a pool added to [`Named`] is served and counted by the same change.
    pub(crate) const ALL: [Self; 9] = [
        Self::Detail,
        Self::Calendar,
        Self::StoreRead,
        Self::LogRead,
        Self::OpsRead,
        Self::Census,
        Self::Scan,
        Self::Verify,
        Self::List,
    ];

    /// The pool this name stands for.
    pub(crate) fn pool(self) -> &'static Pool {
        match self {
            Self::Detail => &ACTIVE,
            Self::Calendar => &CALENDAR,
            Self::StoreRead => &STORE_READ,
            Self::LogRead => &LOG_READ,
            Self::OpsRead => &OPS_READ,
            Self::Census => &CENSUS,
            Self::Scan => &SCAN,
            Self::Verify => &VERIFY,
            Self::List => &LIST,
        }
    }
}

/// Runs `work` in the named pool with a cancellation flag the request's
/// future owns: when the reader leaves, the future and the guard drop, the
/// flag is set, and `work` — which looks at it between months and every few
/// thousand rows — stops by name (DB-N15; R3-08 brings it to every pool). An
/// answer that arrives disarms the guard first, so a finished request
/// cancels nothing.
///
/// # Errors
///
/// [`RunError::Saturated`] when every slot stays occupied for
/// [`ADMISSION_WAIT_MS`], or [`RunError::Join`].
pub(crate) async fn cancellable_in<T, F>(named: Named, work: F) -> Result<T, RunError>
where
    T: Send + 'static,
    F: FnOnce(&crate::bars::Cancel) -> T + Send + 'static,
{
    let (cancel, guard) = crate::bars::Cancel::pair();
    let permit = named.pool().take().await.ok_or(RunError::Saturated)?;
    let answered = admitted(permit, move || work(&cancel)).await;
    guard.disarm();
    answered
}

/// Runs a window scan in the scan pool of at most [`MAX_SCAN_CONCURRENT`]
/// slots, apart from [`run_store_read`]'s (D-3326, DB-066; named and given
/// the bounded wait by R3-08).
///
/// # Errors
///
/// [`RunError::Saturated`] when every scan slot stays occupied for
/// [`ADMISSION_WAIT_MS`], or [`RunError::Join`].
pub async fn run_scan<T, F>(work: F) -> Result<T, RunError>
where
    T: Send + 'static,
    F: FnOnce() -> T + Send + 'static,
{
    let permit = SCAN.take().await.ok_or(RunError::Saturated)?;
    admitted(permit, work).await
}

/// Runs the whole-vendor check (`/verify.json`) in its own pool of ONE
/// ([`MAX_VERIFY_CONCURRENT`]), with the cancellation flag (R3-08, RA-009,
/// RA-130).
///
/// # Errors
///
/// [`RunError::Saturated`] while another check runs past
/// [`ADMISSION_WAIT_MS`], or [`RunError::Join`].
pub(crate) async fn run_verify<T, F>(work: F) -> Result<T, RunError>
where
    T: Send + 'static,
    F: FnOnce(&crate::bars::Cancel) -> T + Send + 'static,
{
    cancellable_in(Named::Verify, work).await
}

/// Runs a feed-list or settings read (`/feeds.json`, `/settings.json`) in the
/// list pool of at most [`MAX_LIST_CONCURRENT`] slots, apart from every page
/// read's pool, with the bounded wait and its `busy` answer (R3-08, NEW-04).
///
/// # Errors
///
/// [`RunError::Saturated`] when every list slot stays occupied for
/// [`ADMISSION_WAIT_MS`], or [`RunError::Join`].
pub(crate) async fn run_list<T, F>(work: F) -> Result<T, RunError>
where
    T: Send + 'static,
    F: FnOnce() -> T + Send + 'static,
{
    let permit = LIST.take().await.ok_or(RunError::Saturated)?;
    admitted(permit, work).await
}

/// What one route did since this process started (R3-08): answered;
/// asked to come back because every slot of its pool stayed taken (`busy`,
/// 429); asked to come back because a writer held the month it reads
/// (`being_written`, 503 with [`BEING_WRITTEN_FIELD`]); abandoned by its
/// reader before the answer (`cancelled`); refused as asked (other 4xx);
/// failed (every other 5xx, a 503 that is not "being written" among them).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct RouteTally {
    pub(crate) answered: u64,
    pub(crate) busy: u64,
    pub(crate) being_written: u64,
    pub(crate) cancelled: u64,
    pub(crate) refused: u64,
    pub(crate) failed: u64,
}

/// What a JSON answer carries when the read met a writer on its month and
/// waited [`READ_PATIENCE_MS`] in vain (R3-08, NEW-21). [`count_route`] gives
/// a 503 that carries it `Retry-After`, and only such a 503: a generic 503 (a
/// task that could not be joined, a census that does not read) is not a
/// "come back" and the page's retry (`fetchWithBusyRetry`) must not wait on
/// it.
pub const BEING_WRITTEN_FIELD: &str = r#""being_written":true"#;

/// The longest 503 body [`count_route`] reads to tell "being written" from
/// any other 503. Every being-written answer is made by
/// [`being_written_answer`], which keeps it under this bound by construction
/// ([`BEING_WRITTEN_CHARS`]): the store's sentences and the holders' names,
/// never the rows a read got before it met the writer. A longer body is not
/// one, and is left as it is without being read.
pub(crate) const BEING_WRITTEN_BODY_MAX: usize = 16 * 1024;

/// Every route's tally, keyed by the route as registered (`MatchedPath`), so
/// the map holds at most one row per registered route plus [`FRONT_END`]:
/// bounded by the route table, never by what clients type.
///
/// **An ordered map, so `/pools.json` walks it in route order and sorts
/// nothing** (R3-08 repair 5, verify-R3-08-r5 blocking 2). It was a hash map
/// that [`pools_json`] copied and sorted on every read, a sort gate 11 rule 4
/// refuses on a request path; the order is now kept where the rows live, once
/// per route ever, rather than redone per read.
static ROUTES: std::sync::Mutex<std::collections::BTreeMap<String, RouteTally>> =
    std::sync::Mutex::new(std::collections::BTreeMap::new());

/// The row requests that matched no registered route are counted under: the
/// front end's pages and assets, served by the fallback.
pub(crate) const FRONT_END: &str = "(front end)";

/// Changes one route's tally. One lock and one probe of the ordered map:
/// O(log R) comparisons, where R is at most one row per registered route
/// plus [`FRONT_END`] (96 today) and is fixed when the binary is built, so a
/// constant per request that never grows with the store or with what clients
/// type; the route's name is copied once, the first time it is seen.
/// UNVERIFIED on its own: no bench times the tally apart. It runs inside every
/// request `crates/api/tests/pool_load.rs` times (p99 per request at 16, 64
/// and 256 clients), and its map is bounded by the route table
/// (`crate::server::pool_table_tests::every_store_route_names_its_pool`);
/// `docs/06-limits.md` L-R308-01.
fn tally(route: &str, change: impl FnOnce(&mut RouteTally)) {
    let mut routes = lock_through_poison(&ROUTES);
    if let Some(row) = routes.get_mut(route) {
        change(row);
    } else {
        let mut row = RouteTally::default();
        change(&mut row);
        routes.insert(route.to_owned(), row);
    }
}

/// One route's tally now; a route never asked is all zeros.
#[cfg(test)]
#[must_use]
pub(crate) fn route_tally(route: &str) -> RouteTally {
    lock_through_poison(&ROUTES)
        .get(route)
        .copied()
        .unwrap_or_default()
}

/// A request in flight: dropped before its answer — its reader left and the
/// server dropped the request's future — it is counted cancelled.
struct InFlight {
    route: Option<axum::extract::MatchedPath>,
    armed: bool,
}

impl InFlight {
    fn route(&self) -> &str {
        self.route
            .as_ref()
            .map_or(FRONT_END, axum::extract::MatchedPath::as_str)
    }
}

impl Drop for InFlight {
    fn drop(&mut self) {
        if self.armed {
            tally(self.route(), |row| {
                row.cancelled = row.cancelled.saturating_add(1);
            });
        }
    }
}

/// The middleware that counts every route's answers (R3-08): what the Speed
/// page shows as answered, busy, being written and cancelled per route
/// (`/pools.json`). A 503 that says the month is being written
/// ([`BEING_WRITTEN_FIELD`]) leaves with `Retry-After: 1`, so the page asks
/// again by itself (NEW-21); no other 503 gains one.
pub(crate) async fn count_route(
    request: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    let route = request
        .extensions()
        .get::<axum::extract::MatchedPath>()
        .cloned();
    let mut in_flight = InFlight { route, armed: true };
    let response = next.run(request).await;
    in_flight.armed = false;
    let (response, being_written) = ask_again_when_being_written(response).await;
    let status = response.status().as_u16();
    tally(in_flight.route(), |row| {
        let counter = match status {
            _ if being_written => &mut row.being_written,
            429 => &mut row.busy,
            0..400 => &mut row.answered,
            400..500 => &mut row.refused,
            _ => &mut row.failed,
        };
        *counter = counter.saturating_add(1);
    });
    response
}

/// `response`, with `Retry-After: 1` when it is a 503 whose body says the
/// month is being written, and whether it is one.
///
/// Only a 503 is looked at, and only a body whose exact length is known and
/// at most [`BEING_WRITTEN_BODY_MAX`]: one bounded read of a few hundred
/// bytes on an answer that is already a refusal, nothing on any other
/// answer. A body that fails to read is replaced by a sentence that says so,
/// never by an empty one.
async fn ask_again_when_being_written(
    response: axum::response::Response,
) -> (axum::response::Response, bool) {
    use axum::body::HttpBody as _;
    if response.status() != axum::http::StatusCode::SERVICE_UNAVAILABLE {
        return (response, false);
    }
    let small = response
        .body()
        .size_hint()
        .exact()
        .is_some_and(|bytes| bytes <= BEING_WRITTEN_BODY_MAX as u64);
    if !small {
        return (response, false);
    }
    let (mut parts, body) = response.into_parts();
    let bytes = match axum::body::to_bytes(body, BEING_WRITTEN_BODY_MAX).await {
        Ok(bytes) => bytes,
        Err(why) => {
            let said = serde_json::json!({
                "error": format!("this 503's own body did not read: {why}"),
            });
            return (
                axum::response::Response::from_parts(
                    parts,
                    axum::body::Body::from(said.to_string()),
                ),
                false,
            );
        }
    };
    let being_written =
        std::str::from_utf8(&bytes).is_ok_and(|text| text.contains(BEING_WRITTEN_FIELD));
    if being_written && !parts.headers.contains_key(axum::http::header::RETRY_AFTER) {
        parts.headers.insert(
            axum::http::header::RETRY_AFTER,
            axum::http::HeaderValue::from_static("1"),
        );
    }
    (
        axum::response::Response::from_parts(parts, axum::body::Body::from(bytes)),
        being_written,
    )
}

/// `GET /pools.json`: every named pool — its slots on this machine, what runs
/// and waits now, what it answered, refused busy and saw abandoned — and every
/// route's tally (R3-08; the Speed page reads it). Memory only: it touches no
/// file, so it runs on the async worker and is in no pool.
pub(crate) async fn pools_json() -> (
    axum::http::StatusCode,
    [(axum::http::HeaderName, &'static str); 1],
    String,
) {
    let pools: Vec<serde_json::Value> = Named::ALL
        .iter()
        .map(|named| {
            let pool = named.pool();
            serde_json::json!({
                "pool": pool.name(),
                "slots": pool.max(),
                "running": pool.active.load(Ordering::Acquire),
                "waiting": pool.waiting.load(Ordering::Acquire),
                "answered": pool.answered.load(Ordering::Relaxed),
                "busy": pool.busy.load(Ordering::Relaxed),
                "cancelled": pool.cancelled.load(Ordering::Relaxed),
            })
        })
        .collect();
    // In route order as kept: the map is ordered, so nothing is sorted here
    // (repair 5). The rows are copied out under the lock and rendered after
    // it, so no request's tally waits on the JSON.
    let routes: Vec<(String, RouteTally)> = lock_through_poison(&ROUTES)
        .iter()
        .map(|(route, row)| (route.clone(), *row))
        .collect();
    let routes: Vec<serde_json::Value> = routes
        .into_iter()
        .map(|(route, row)| {
            serde_json::json!({
                "route": route,
                "answered": row.answered,
                "busy": row.busy,
                "being_written": row.being_written,
                "cancelled": row.cancelled,
                "refused": row.refused,
                "failed": row.failed,
            })
        })
        .collect();
    let body = serde_json::json!({
        "cores": cores(),
        "wait_ms": ADMISSION_WAIT_MS,
        "max_waiting": MAX_WAITING,
        "read_patience_ms": READ_PATIENCE_MS,
        // THE ONE ROUTE ROW THAT IS NO PATH, NAMED BY THE SERVER THAT COUNTS
        // IT (R3-08 repair 2): every page and asset the fallback serves is
        // tallied under [`FRONT_END`], so the row exists as soon as a browser
        // has loaded the page that asks. The reader takes the name from here
        // rather than keeping a copy of it.
        "front_end": FRONT_END,
        "pools": pools,
        "routes": routes,
        "say": "since this process started; busy is a 429 (every slot of the pool stayed taken \
                for the wait) and being_written a 503 (a writer held the month past the read's \
                patience), each with Retry-After; cancelled is a reader that left before its \
                answer; failed is every other 5xx. A pool's answered is work it ran to its end for a \
                reader still there, WHATEVER the route then said: a page, a refusal or being \
                written. The route rows split that one count by what was said, so the two \
                answered columns count different things",
    });
    (
        axum::http::StatusCode::OK,
        [(
            axum::http::header::CONTENT_TYPE,
            "application/json; charset=utf-8",
        )],
        body.to_string(),
    )
}

/// Whether a refusal says the month is being written (a writer held it past
/// [`READ_PATIENCE_MS`]): the route answers 503 "being written, ask again"
/// with `Retry-After`, never "unreadable" (R3-08, NEW-21).
#[must_use]
pub fn is_being_written(why: &str) -> bool {
    why.contains(store::flock::BEING_WRITTEN)
}

/// The most of a read's being-written sentences [`being_written_answer`]
/// names; past it the rest are counted. A per-instant read opens at most two
/// months (the option's and its underlying's), so this is never reached by
/// the routes that call it today; it keeps the body under
/// [`BEING_WRITTEN_BODY_MAX`], which [`count_route`] reads to add
/// `Retry-After`, whatever a later caller passes.
const BEING_WRITTEN_NAMED: usize = 4;

/// The most characters of the holders' sentences one being-written body
/// quotes: six bytes each at worst once escaped, with 128 bytes left for the
/// envelope and the count of the rest, so the body is never over
/// [`BEING_WRITTEN_BODY_MAX`].
pub(crate) const BEING_WRITTEN_CHARS: usize = (BEING_WRITTEN_BODY_MAX - 128) / 6;

/// A read that answers per instant (`/spot.json`, `/greeks.json`) and whose
/// EVERY fault is a writer holding a month it needed: the "being written, ask
/// again" 503, its body the store's sentences (each names the holder) and
/// [`BEING_WRITTEN_FIELD`], so [`count_route`] gives it `Retry-After` and the
/// page asks again by itself (R3-08, NEW-21). Before this such a read
/// answered 206 with every instant `"unreadable"` — the word a damaged file
/// earns — and no `Retry-After`, so nothing asked again (measured on a copy
/// of gdfl NIFTY 1s 2019-11 held by `lockf(1)`).
///
/// `None` when there is no fault, or when ANY fault is something else: real
/// damage beside a busy month is answered as damage, never dressed as
/// "ask again".
#[must_use]
pub(crate) fn being_written_answer(faults: &[String]) -> Option<(axum::http::StatusCode, String)> {
    if faults.is_empty() || !faults.iter().all(|why| is_being_written(why)) {
        return None;
    }
    // BOUNDED BY CONSTRUCTION, WHATEVER THE SENTENCES HOLD: the JSON escape
    // writes at most six bytes for one character (`<`, `\u001f`), so
    // [`BEING_WRITTEN_CHARS`] characters and the envelope stay under
    // [`BEING_WRITTEN_BODY_MAX`]. The verifier's case: a window page that
    // spanned a held month and an unheld one answered its being-written 503
    // with every row it read — 126,649 bytes, no `Retry-After`, counted
    // "failed". Every being-written body is made here now, and this cap is
    // what lets `count_route` read it.
    let named = faults
        .iter()
        .take(BEING_WRITTEN_NAMED)
        .map(String::as_str)
        .collect::<Vec<_>>()
        .join("; ")
        .chars()
        .take(BEING_WRITTEN_CHARS)
        .collect::<String>();
    let more = faults.len().saturating_sub(BEING_WRITTEN_NAMED);
    let said = if more == 0 {
        named
    } else {
        format!("{named}; and {more} more being written")
    };
    Some((
        axum::http::StatusCode::SERVICE_UNAVAILABLE,
        format!(
            r#"{{"error":{},{BEING_WRITTEN_FIELD}}}"#,
            crate::render::json_string(&said)
        ),
    ))
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

/// [`apart_from_slot_owners`] for a test that is not async and drives its
/// requests on runtimes of its own: the same guard, taken on this thread.
/// Never call it inside a runtime (`blocking_lock` panics there).
#[cfg(test)]
pub(crate) fn apart_from_slot_owners_blocking() -> tokio::sync::MutexGuard<'static, ()> {
    TEST_SERIAL.blocking_lock()
}

/// Takes every slot that is free right now, for a test that must saturate the
/// pool from inside a request it is already running.
///
/// Asks for the serial guard so it cannot race [`hold_every_slot`]. Other
/// tests that use a slot without that guard may still hold some, which is why
/// this takes what is free rather than exactly [`MAX_CONCURRENT`].
#[cfg(test)]
pub(crate) fn take_every_free_slot(_apart: &tokio::sync::MutexGuard<'static, ()>) -> Vec<Permit> {
    std::iter::from_fn(|| ACTIVE.try_take()).collect()
}

#[cfg(test)]
pub(crate) struct HeldSlots {
    _serial: tokio::sync::MutexGuard<'static, ()>,
    _permits: Vec<Permit>,
}

#[cfg(test)]
pub(crate) async fn hold_every_slot() -> Result<HeldSlots, &'static str> {
    let serial = TEST_SERIAL.lock().await;
    let permits = (0..ACTIVE.max())
        .map(|_| {
            ACTIVE
                .try_take()
                .ok_or("the test did not own every detail slot")
        })
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
    std::iter::from_fn(|| LOG_READ.try_take()).collect()
}

/// Every slot of one pool counted taken, whatever other tests hold, until
/// dropped: a test that must see a route refused at that pool (R3-08).
#[cfg(test)]
pub(crate) struct Crowd(&'static Pool, usize);

/// Crowds the named pool: adds its whole slot count to what runs.
#[cfg(test)]
pub(crate) fn crowd(named: Named) -> Crowd {
    let pool = named.pool();
    let slots = pool.max();
    pool.active.fetch_add(slots, Ordering::AcqRel);
    Crowd(pool, slots)
}

#[cfg(test)]
impl Drop for Crowd {
    fn drop(&mut self) {
        self.0.active.fetch_sub(self.1, Ordering::AcqRel);
        self.0.freed.notify_waiters();
    }
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
        ACTIVE, ADMISSION_WAIT_MS, CENSUS, Cached, IDENTITY_REFUSAL, LOG_READ,
        MAX_CENSUS_CONCURRENT, MAX_LOG_READ_CONCURRENT, MAX_PAGE, MAX_PAGE_ROWS, MAX_QUERY_BYTES,
        MAX_RESULT_ROWS, MAX_SCAN_BYTES, MAX_WAITING, Ordering, Page, Permit, Pool, RunError,
        STORE_READ, Selector, Size, admission_refused, must_admit, preflight, run, run_calendar,
        run_census, run_log_read, run_store_read, seek_window, window,
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

    /// **A REFUSAL NAMES THE SLOTS THIS MACHINE HAS, NEVER ONLY THE
    /// CEILING** (R3-08: pool sizes come from the core count). A ceiling past
    /// the cores is told as the cores; one under them as itself; the verify
    /// pool's one as one; and a ceiling of zero still names one slot, the
    /// floor every `Size::Cores` pool keeps.
    #[test]
    fn a_refusal_names_the_slots_this_machine_has() {
        let cores = std::thread::available_parallelism().map_or(1, std::num::NonZeroUsize::get);
        for (ceiling, slots) in [
            (cores.saturating_mul(4).max(2), cores),
            (1, 1),
            (0, 1),
            (super::MAX_VERIFY_CONCURRENT, 1),
        ] {
            let (status, body) = admission_refused("a probe", ceiling, &RunError::Saturated);
            assert_eq!(status, axum::http::StatusCode::TOO_MANY_REQUESTS);
            assert!(
                body.contains(&format!("at most {slots} run at once")),
                "ceiling {ceiling} on {cores} cores: {body}"
            );
        }
        for pool in super::Named::ALL {
            let pool = pool.pool();
            assert!(
                pool.max() >= 1 && pool.max() <= cores.max(1),
                "{}",
                pool.name()
            );
        }
    }

    /// **A POOL ADMITS EXACTLY ITS BOUND, AND A DROPPED PERMIT RETURNS ITS
    /// SLOT.** W1-api2-11, D-1443. Driven on a pool of the test's own, so no
    /// route test sharing the real pools can see it.
    #[test]
    fn a_pool_admits_its_bound_and_a_dropped_permit_frees_its_slot() {
        static POOL: Pool = Pool::new("test", Size::Fixed(2));
        static NONE: Pool = Pool::new("test", Size::Fixed(0));
        let first = POOL.try_take().expect("one of two");
        let second = POOL.try_take().expect("two of two");
        assert!(POOL.try_take().is_none(), "a third is refused");
        assert_eq!(POOL.active.load(Ordering::Acquire), 2);
        drop(first);
        assert_eq!(POOL.active.load(Ordering::Acquire), 1);
        let third = POOL.try_take().expect("the freed slot");
        drop((second, third));
        assert_eq!(
            POOL.active.load(Ordering::Acquire),
            0,
            "every slot returned"
        );
        assert!(NONE.try_take().is_none(), "a zero bound admits nothing");
        assert_eq!(
            NONE.active.load(Ordering::Acquire),
            0,
            "and a refusal takes nothing"
        );
    }

    /// **A FULL POOL QUEUES A SHORT READ INSTEAD OF REFUSING IT (DB-B51).**
    /// Ten tabs met 429 on 271 of 480 seeks, each a read of about a
    /// millisecond, because a full pool refused before it queued. A request
    /// that finds every slot taken now waits, and is admitted the moment a
    /// slot is returned.
    #[tokio::test(flavor = "current_thread")]
    async fn a_full_pool_admits_a_waiter_when_a_slot_is_returned() {
        static POOL: Pool = Pool::new("test", Size::Fixed(1));
        let held = POOL.try_take().expect("the only slot");
        let started = tokio::time::Instant::now();
        let waiter = tokio::spawn(POOL.take());
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        assert_eq!(POOL.waiting.load(Ordering::Acquire), 1, "it is queued");
        drop(held);
        let admitted = waiter.await.expect("joined");
        assert!(admitted.is_some(), "the returned slot is the waiter's");
        assert!(
            started.elapsed() < std::time::Duration::from_millis(ADMISSION_WAIT_MS),
            "admitted on the return, not at the deadline"
        );
        assert_eq!(POOL.waiting.load(Ordering::Acquire), 0, "the queue is left");
        drop(admitted);
        assert_eq!(POOL.active.load(Ordering::Acquire), 0);
    }

    /// **THE WAIT IS BOUNDED IN TIME AND IN LENGTH (DB-B51).** A pool held
    /// past [`ADMISSION_WAIT_MS`] refuses at the deadline, never later; and
    /// past [`MAX_WAITING`] waiters a request is refused at once, without
    /// waiting at all.
    #[tokio::test(flavor = "current_thread")]
    async fn a_waiter_is_refused_at_its_deadline_and_a_full_queue_refuses_at_once() {
        static POOL: Pool = Pool::new("test", Size::Fixed(1));
        let held = POOL.try_take().expect("the only slot");
        let started = tokio::time::Instant::now();
        assert!(POOL.take().await.is_none(), "still held at the deadline");
        let waited = started.elapsed();
        assert!(
            waited >= std::time::Duration::from_millis(ADMISSION_WAIT_MS),
            "{waited:?}"
        );
        assert!(
            waited < std::time::Duration::from_millis(ADMISSION_WAIT_MS * 4),
            "{waited:?}"
        );

        let waiters: Vec<_> = (0..MAX_WAITING)
            .map(|_| tokio::spawn(POOL.take()))
            .collect();
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        assert_eq!(POOL.waiting.load(Ordering::Acquire), MAX_WAITING);
        let at_once = tokio::time::Instant::now();
        assert!(POOL.take().await.is_none(), "the queue is full");
        assert!(
            at_once.elapsed() < std::time::Duration::from_millis(ADMISSION_WAIT_MS / 2),
            "refused without waiting"
        );
        drop(held);
        let admitted = futures_count(waiters).await;
        assert_eq!(admitted, 1, "one slot returned admits exactly one waiter");
        assert_eq!(POOL.waiting.load(Ordering::Acquire), 0);
        assert_eq!(POOL.active.load(Ordering::Acquire), 0);
    }

    /// How many of `waiters` were admitted. Every permit is kept until all
    /// are counted, so no waiter can take a slot an admitted one returned.
    async fn futures_count(waiters: Vec<tokio::task::JoinHandle<Option<Permit>>>) -> usize {
        let mut admitted = 0;
        let mut kept = Vec::new();
        for waiter in waiters {
            if let Some(permit) = waiter.await.expect("joined") {
                admitted += 1;
                kept.push(permit);
            }
        }
        admitted
    }

    /// **A WAIT ASKS NOTHING OF THE RUNTIME'S TIMER.** On a runtime built with
    /// no timer a full pool refuses its waiter at the deadline, and admits it
    /// when a slot is returned sooner. The wait was `tokio::time::timeout_at`,
    /// which panics there ("timers are disabled") — only when the pool was
    /// full, so a test driving a route on such a runtime died whenever other
    /// tests held the slots.
    #[test]
    fn a_wait_is_timed_on_a_runtime_that_has_no_timer() {
        static POOL: Pool = Pool::new("test", Size::Fixed(1));
        let runtime = tokio::runtime::Builder::new_current_thread()
            .build()
            .expect("a runtime with no timer and no I/O driver");
        let held = POOL.try_take().expect("the one slot");
        let started = std::time::Instant::now();
        assert!(
            runtime.block_on(POOL.take()).is_none(),
            "still held at the deadline"
        );
        let waited = started.elapsed();
        assert!(
            waited >= std::time::Duration::from_millis(ADMISSION_WAIT_MS),
            "refused after {waited:?}, before the wait was over"
        );
        assert!(
            waited < std::time::Duration::from_millis(ADMISSION_WAIT_MS * 4),
            "refused only after {waited:?}"
        );
        assert_eq!(POOL.waiting.load(Ordering::Acquire), 0, "it left the queue");

        // A slot returned during the wait admits the waiter there too.
        let returner = std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(20));
            drop(held);
        });
        let started = std::time::Instant::now();
        let admitted = runtime.block_on(POOL.take());
        assert!(admitted.is_some(), "the returned slot was not taken");
        assert!(
            started.elapsed() < std::time::Duration::from_millis(ADMISSION_WAIT_MS),
            "admitted only at the deadline: {:?}",
            started.elapsed()
        );
        returner.join().expect("the returning thread");
        assert_eq!(POOL.active.load(Ordering::Acquire), 1);
        drop(admitted);
        assert_eq!(POOL.active.load(Ordering::Acquire), 0);
    }

    /// **THE CENSUS HAS ITS OWN POOL (DB-B51).** With every detail slot taken
    /// — the slots the audit journal of a backtest launch needs — a census
    /// read is still admitted, and the reverse.
    #[tokio::test(flavor = "current_thread")]
    async fn the_census_pool_is_apart_from_the_detail_pool() {
        let apart = super::apart_from_slot_owners().await;
        let detail = super::take_every_free_slot(&apart);
        assert_eq!(run_census(|| 5).await, Ok(5), "detail full, census free");
        drop(detail);
        let census: Vec<Permit> = std::iter::from_fn(|| CENSUS.try_take()).collect();
        assert_eq!(
            census.len(),
            CENSUS.max(),
            "every census slot this machine has"
        );
        assert!(
            CENSUS.max() <= MAX_CENSUS_CONCURRENT,
            "never past its ceiling"
        );
        assert_eq!(run(|| 6).await, Ok(6), "census full, detail free");
        assert_eq!(run_census(|| ()).await, Err(RunError::Saturated));
        drop(census);
        assert_eq!(run_census(|| 7).await, Ok(7));
        assert_eq!(ACTIVE.active.load(Ordering::Acquire), 0);
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
        // Hold every slot; the next is refused once its wait ends (DB-B51),
        // and a released slot admits again. D-1508.
        let held: Vec<Permit> = (0..STORE_READ.max())
            .map(|_| STORE_READ.try_take().expect("a free slot"))
            .collect();
        assert_eq!(run_store_read(|| ()).await, Err(RunError::Saturated));
        // The other pools are not this one's.
        assert_eq!(run_calendar(|| 7).await, Ok(7));
        drop(held);
        assert_eq!(run_store_read(|| 9).await, Ok(9));
        assert_eq!(STORE_READ.active.load(Ordering::Acquire), 0);
    }

    /// **A FULL OPERATIONS POOL REFUSES NO BAR SEEK, AND A FULL SEEK POOL
    /// REFUSES NO OPERATIONS READ (WP-81 repair 2).** `/scrub.json` and
    /// `/disk.json` took the store-read slots every bar seek takes, and a
    /// page polling a scrub refused `/db` its bars. Every slot of one pool
    /// held: that pool refuses once its wait ends, and the other admits.
    #[tokio::test(flavor = "current_thread")]
    async fn an_ops_read_and_a_bar_seek_never_wait_on_each_other() {
        let _apart = super::apart_from_slot_owners().await;
        let worker = std::thread::current().id();
        let blocking = super::run_ops_read(|| std::thread::current().id())
            .await
            .expect("admitted ops read");
        assert_ne!(blocking, worker, "spawn_blocking owns a different thread");
        let held: Vec<Permit> = std::iter::from_fn(|| super::OPS_READ.try_take()).collect();
        assert!(!held.is_empty() && held.len() <= super::MAX_OPS_READ_CONCURRENT);
        assert_eq!(super::run_ops_read(|| ()).await, Err(RunError::Saturated));
        assert_eq!(
            run_store_read(|| 7).await,
            Ok(7),
            "a bar seek is admitted while every operations slot is held"
        );
        drop(held);
        assert_eq!(super::run_ops_read(|| 9).await, Ok(9));
        let seeks: Vec<Permit> = std::iter::from_fn(|| STORE_READ.try_take()).collect();
        assert!(!seeks.is_empty());
        assert_eq!(
            super::run_ops_read(|| 11).await,
            Ok(11),
            "an operations read is admitted while every seek slot is held"
        );
        drop(seeks);
    }

    /// A log read runs on a blocking thread, and past
    /// [`MAX_LOG_READ_CONCURRENT`] it is refused once its wait ends, without
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
        assert_eq!(
            held.len(),
            LOG_READ.max(),
            "this test owns every log-read slot"
        );
        assert_eq!(MAX_LOG_READ_CONCURRENT, 4);
        assert_eq!(run_log_read(|| ()).await, Err(RunError::Saturated));
        // The other pools are not this one's.
        assert_eq!(run_calendar(|| 7).await, Ok(7));
        drop(held);
        assert_eq!(run_log_read(|| 9).await, Ok(9));
        assert_eq!(LOG_READ.active.load(Ordering::Acquire), 0);
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

    /// **EVERY POOL'S WORK SEES ITS FLAG ONCE ITS READER LEAVES, AND THE POOL
    /// COUNTS IT** (R3-08, RA-130). The gaps walk (calendar pool), the
    /// whole-vendor check (verify pool), spot and greeks (store-read pool)
    /// and window scans (scan pool) reach their work through
    /// [`super::cancellable_in`]. For each named pool: a request starts work
    /// that looks at its flag every millisecond, the request is dropped while
    /// the work runs (what the server does when the client closes its
    /// connection), and the work must see the flag within its next look — not
    /// run to its ten-second deadline — and the pool must count one reader
    /// that left. Before R3-08 only the bar routes had a flag at all.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn every_pools_work_sees_its_flag_once_its_reader_leaves() {
        let _apart = super::apart_from_slot_owners().await;
        for named in super::Named::ALL {
            let pool = named.pool();
            let before = pool.cancelled.load(Ordering::Relaxed);
            let (started, has_started) = std::sync::mpsc::channel::<()>();
            let (ended, has_ended) = std::sync::mpsc::channel::<Option<u64>>();
            let request = tokio::spawn(super::cancellable_in(named, move |cancel| {
                let _ = started.send(());
                let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
                let mut looks = 0_u64;
                while !cancel.is_cancelled() && std::time::Instant::now() < deadline {
                    looks += 1;
                    std::thread::sleep(std::time::Duration::from_millis(1));
                }
                let _ = ended.send(cancel.is_cancelled().then_some(looks));
            }));
            tokio::task::spawn_blocking(move || {
                has_started.recv_timeout(std::time::Duration::from_secs(5))
            })
            .await
            .expect("joined")
            .unwrap_or_else(|_| panic!("{named:?}: the work never started"));
            let left = std::time::Instant::now();
            request.abort();
            let seen = tokio::task::spawn_blocking(move || {
                has_ended.recv_timeout(std::time::Duration::from_secs(12))
            })
            .await
            .expect("joined")
            .unwrap_or_else(|_| panic!("{named:?}: the work never ended"));
            assert!(
                seen.is_some(),
                "{named:?}: the work ran to its deadline; the flag never reached it"
            );
            assert!(
                left.elapsed() < std::time::Duration::from_secs(2),
                "{named:?}: the work took {:?} to see its reader leave",
                left.elapsed()
            );
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
            while (pool.cancelled.load(Ordering::Relaxed) == before
                || pool.active.load(Ordering::Acquire) != 0)
                && std::time::Instant::now() < deadline
            {
                tokio::time::sleep(std::time::Duration::from_millis(2)).await;
            }
            assert!(
                pool.cancelled.load(Ordering::Relaxed) > before,
                "{named:?}: the pool counts the reader that left"
            );
            assert_eq!(
                pool.active.load(Ordering::Acquire),
                0,
                "{named:?}: the slot is free"
            );
        }
    }
}
