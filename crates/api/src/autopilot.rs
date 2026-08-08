//! The backfill drives itself: one process, no shell loop, no operator.
//!
//! # What this replaces
//!
//! Three manual things — `cargo run -p api`, a front-end dev server, and a
//! shell script looping `curl` at `/pull/spot`. The shell script is the one
//! this module deletes: the served binary now decides what is missing, asks
//! for it, and says out loud what it is doing while it does.
//!
//! # The four rules it is built on, each measured rather than assumed
//!
//! **Oldest first, always.** The store is append-only with monotonic
//! timestamps and one file per month. `store::file::BarFile::append` accepts a
//! batch whose overlap with what is held is a verified *suffix* of it; a batch
//! whose overlap is LONGER than everything held underflows and the whole batch
//! is refused. A real file on the operator's disk proves it —
//! `bars/groww/NSE/INDEX/NIFTY/1min/2026-08.bin` holds 375 bars, all of
//! 2026-08-06, so offering 08-03..=08-07 refuses even 08-07, which strictly
//! follows. A later month fetched first permanently blocks the earlier days in
//! that month file. So the ladder is climbed upward and never downward.
//!
//! **The resume point comes from the store, not from a cursor.** A cursor file
//! is a second answer to "where am I", and the one that is wrong is the one
//! that skips a month in silence. [`next_window`] probes the manifest —
//! `pull::manifest::Manifest::entry`, one hash probe — and asks from the day
//! after the last day that key already holds. That is also the only request
//! shape that can extend a partially-written month file.
//!
//! **Never today.** A session still running yields a partial day the
//! append-only store can never correct. `crates/api/src/server.rs`'s
//! `finished_day_only` refuses it as a backstop; this module clamps to
//! yesterday so the backstop is never reached.
//!
//! **Never below the vendor's floor.** Groww's history begins 2020-01-01 and
//! Dhan's rolls ~5 years back from today. The floor is taken from
//! `pull::vendor::HistoryFloor` through the same `clamp_to_floor` the manual
//! path uses — recomputed every tick, because a stored rolling floor is a
//! frozen one.
//!
//! # One pull implementation, not two
//!
//! Every fetch goes through `crate::server::broker_run` — the same loop
//! `/pull/spot` runs, so the month-boundary split, the history floor, the AIMD
//! waiting governor, the retry-with-backoff, the credential read and the store
//! append are all the ones already tested. This module decides *what* to ask
//! for and *when*; it does not know how to talk to a vendor.
//!
//! # Cost
//!
//! | operation | cost |
//! |---|---|
//! | decide one (series, month) cell | **one hash probe + integer arithmetic** |
//! | decide one month for the universe | one probe per tracked series |
//! | advance the frontier | monotone — each month is scanned once per process |
//! | the pause check inside a run | one relaxed atomic load per instrument |
//! | publish the status | in-memory, never `census_now` |
//!
//! Nothing here lists a directory and nothing scans a bar file. The census is
//! re-read once per tick — O(entries), against a tick that takes minutes — for
//! the same reason D-0039 reads it once per process.
//!
//! # What is NOT persisted, deliberately
//!
//! The pause flag, the frontier hint, the attempt counters and the stall list
//! all die with the process. A persisted pause is a second source of truth
//! about intent that outlives the machine it was set on, and it would make
//! "click Run" not mean run. A restart re-derives everything from the store,
//! which is what makes killing the process safe.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use pull::manifest::EntryKey;
use pull::session::{Day, IstMoment, Window};
use store::path::{Timeframe, YearMonth};

use crate::census::{Census, Series, VendorCensus};
use crate::server::{Broker, Loaded, Site};
use crate::{audit, census, ingest, render};

/// How long the operator has to press Pause before the first socket opens.
///
/// This is not a fallback and it hides nothing: it is the one chance to say no
/// before a vendor is contacted, counted down on the page. Zero clicks in the
/// intended case, one in the exception.
pub const GRACE_SECS: u64 = 20;

/// How many times one month is attempted before it is stalled and passed.
///
/// Three. Blocking forever on 2020-05 means 2020-06 onward are never pulled —
/// one bad month must not cost seventy good ones. The stall is not hidden: it
/// sits in a permanent list on the status with the reason verbatim.
pub const MAX_MONTH_ATTEMPTS: u8 = 3;

/// How many consecutive dry rounds retire a month.
///
/// Two, from `docs/07-plan.md` §9.2: *"a round that finds nothing may have
/// found nothing because a vendor was down, not because nothing is missing"*.
/// This is what stands in for the trading calendar this build does not have —
/// a month whose last calendar days are a weekend can never satisfy "held
/// through the last day", and inventing a holiday table would be the vendor
/// fact `CLAUDE.md` §3 rule 1 forbids.
pub const DRY_ROUNDS: u8 = 2;

/// The first backoff after the transport fails outright, in seconds.
pub const BACKOFF_FLOOR_SECS: u64 = 30;

/// The longest backoff, in seconds. Fifteen minutes.
pub const BACKOFF_CEILING_SECS: u64 = 900;

/// How often a complete autopilot re-reads the store to see if a day has
/// arrived.
pub const IDLE_POLL_SECS: u64 = 60;

/// How long a paused autopilot sleeps between checks of the flag.
///
/// One second, and it must be a real sleep: see the paused arm of [`fly`] for
/// the 99%-of-a-core spin that measuring this taught.
pub const PAUSED_POLL_SECS: u64 = 1;

/// How long to stand off when a hand-made pull holds the pull seat.
///
/// One second: long enough that waiting out a ten-minute manual pull costs six
/// hundred atomic loads rather than a pegged core, and short enough that the
/// backfill resumes promptly once the operator's own pull finishes.
pub const SEAT_WAIT_SECS: u64 = 1;

/// How many credential failures are re-read before the feed halts.
///
/// One, and `CLAUDE.md` §8 is where the number comes from: *"A stale token is
/// re-read; if the re-read returns the same dead value, the pull halts
/// loudly."* The re-read is automatic — `broker_window` reads Parameter Store
/// fresh on every instrument and caches nothing — so the retry IS the re-read.
pub const CREDENTIAL_REREADS: u8 = 1;

/// The refusal one instrument reports when a run is stopped mid-sweep.
pub const CANCELLED: &str = "stopped by the operator";

/// What the autopilot is doing, as one word.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    /// Counting down the grace window before the first fetch.
    Starting,
    /// Fetching, or about to.
    Running,
    /// Waiting out a transport failure before retrying the same month.
    Backoff,
    /// Stopped by the operator. Nothing is contacted.
    Paused,
    /// Terminal, with a reason. Only the operator clears it.
    Halted,
    /// Nothing is missing that this feed can still be asked for.
    Idle,
}

impl Phase {
    /// The word the JSON carries, and it is **D-0057's vocabulary, not one of
    /// this module's own**.
    ///
    /// `docs/05-decisions.md` D-0057 fixes the set to `starting`, `running`,
    /// `waiting`, `paused`, `halted`, `complete`, and
    /// `web/src/routes/autopilot/+page.svelte` refuses anything outside it by
    /// name rather than rendering a calm blank. So the two internal phases
    /// whose names differ map onto the published ones here — `Backoff` is a
    /// wait and `Idle` is completion — and the mapping lives in one function
    /// instead of at every call site.
    #[must_use]
    pub const fn state(self) -> &'static str {
        match self {
            Self::Starting => "starting",
            Self::Running => "running",
            Self::Backoff => "waiting",
            Self::Paused => "paused",
            Self::Halted => "halted",
            Self::Idle => "complete",
        }
    }
}

/// The whole span the backfill is aiming at, as the page's `target` object.
///
/// `docs/07-plan.md` R-2 states it: 2020-01-01 → yesterday, never today. One
/// stated target with two views of it, rather than two targets.
#[derive(Debug, Clone, Default)]
pub struct Target {
    /// The oldest day this feed can be asked for.
    pub from: String,
    /// The newest finished day.
    pub to: String,
    /// How many tracked series the sweep covers.
    pub instruments: usize,
    /// The bar length.
    pub timeframe: String,
    /// Which feed.
    pub feed: String,
}

/// The one cell actually in flight, as the page's `now` object.
///
/// `None` whenever nothing is on the wire, which is a legal answer for every
/// state except `running` — a "running" autopilot with nothing in flight is the
/// hang the page exists to expose, so [`Status::json`] reports `waiting`
/// instead of claiming to run.
#[derive(Debug, Clone)]
pub struct InFlight {
    /// The instrument being fetched right now.
    pub instrument: String,
    /// The month it is being fetched for.
    pub month: String,
    /// The bar length.
    pub timeframe: String,
    /// Which feed.
    pub feed: String,
    /// Its position in the sweep.
    pub index: usize,
    /// How many the sweep covers.
    pub of: usize,
    /// When it started.
    ///
    /// **Kept as an `Instant` and published as a DURATION.** D-0057 is explicit
    /// about why: a start instant would have to be compared against the
    /// browser's clock, and two clocks a minute apart render a cell that has
    /// been running for minus forty-seven seconds. The server measures the
    /// elapsed time itself, at the moment it serialises.
    pub since: std::time::Instant,
}

/// One instrument that did not answer, as the page's `failures` entries.
#[derive(Debug, Clone, Default)]
pub struct Failed {
    /// Which instrument.
    pub instrument: String,
    /// Which month it was being fetched for.
    pub month: String,
    /// Why, verbatim.
    pub why: String,
    /// When, as an IST day.
    pub at: String,
}

/// Which kind of trouble a reason names, and therefore what to do about it.
///
/// Three kinds and not one, because they send an operator to three different
/// places: their AWS role, their network, and their disk. A single "it failed"
/// would retry a dead credential sixty-four times and hammer a full disk.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Trouble {
    /// The credential could not be read, or the token is dead.
    ///
    /// Re-read once, then **halt**. `CLAUDE.md` §8. Never minted, never
    /// prompted for, never read from a file or an environment variable.
    Credential,
    /// The write boundary refused: a full disk, a denied path, a short write.
    ///
    /// Not retryable — hammering a full disk is pure waste — so a repeat of the
    /// same reason halts.
    Store,
    /// A timeout, a reset, a 5xx, a throttle the governor could not absorb.
    /// Worth retrying with backoff.
    Transport,
}

/// Which kind of trouble a reason names.
///
/// A pure function over the reason text, so every arm is drivable from a test
/// without a vendor, a disk or a clock. The markers are the exact phrases the
/// pull path produces — `broker_window`'s credential errors, `with_retry`'s
/// expired-token message, and `store::file::StoreError`'s host refusals.
///
/// Unrecognised is [`Trouble::Transport`], which is the retryable answer. That
/// is deliberate and it is not a fallback that hides anything: the reason is
/// carried verbatim onto the status either way, and the attempt bound stops it
/// spinning. Guessing `Halted` for text nobody has classified would stop a
/// backfill for a blip.
#[must_use]
pub fn classify(reason: &str) -> Trouble {
    // CREDENTIAL FIRST. `with_retry`'s expired-token message contains the word
    // "store" (Parameter Store), so testing for the disk first would read a
    // dead token as a full disk and halt for the wrong reason.
    const CREDENTIAL: [&str; 6] = [
        "credential",
        "access token expired",
        "aws identity",
        "parameter path",
        "status 401",
        "invalid_authentication",
    ];
    const STORE: [&str; 5] = [
        "disk full",
        "no space",
        "short write",
        "permission denied",
        "read-only filesystem",
    ];
    let lower = reason.to_ascii_lowercase();
    if CREDENTIAL.iter().any(|m| lower.contains(m)) {
        return Trouble::Credential;
    }
    if STORE.iter().any(|m| lower.contains(m)) {
        return Trouble::Store;
    }
    Trouble::Transport
}

/// The day a stored microsecond timestamp falls on, in IST.
///
/// `None` only for a timestamp no calendar can hold, which is a corrupt entry
/// rather than a state this module can act on — the caller then treats the key
/// as unheld, which re-offers the month and lets the store's own verification
/// refuse it by name.
#[must_use]
pub fn day_of(ts_micros: i64) -> Option<Day> {
    IstMoment::from_epoch_secs(ts_micros.div_euclid(1_000_000))
        .ok()
        .map(IstMoment::day)
}

/// The span of `month` a feed may still be asked for.
///
/// Clamped below by the feed's history floor and above by `yesterday` — never
/// today, for the reason `finished_day_only` gives. `None` when the two
/// clamps cross, which is a month entirely below the floor or entirely in the
/// future.
///
/// Integer comparison on three-field dates. No calendar is walked.
#[must_use]
pub fn month_span(month: YearMonth, floor: Day, yesterday: Day) -> Option<(Day, Day)> {
    let first = Day::new(month.year(), month.month(), 1).ok()?;
    let last = first.end_of_month();
    let from = if first < floor { floor } else { first };
    let to = if last < yesterday { last } else { yesterday };
    if to < from { None } else { Some((from, to)) }
}

/// What one month-tick will ask for, derived from the store alone.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unit {
    /// Which month.
    pub month: YearMonth,
    /// The days to ask for. Starts the day after the earliest resume point.
    pub window: Window,
    /// How many tracked series are short of the month's last askable day.
    pub behind: usize,
    /// How many already hold it.
    pub done: usize,
}

/// The window this month still needs, or `None` when the store already holds
/// it.
///
/// `held` answers "what is the last microsecond timestamp this key holds" in
/// one hash probe — `pull::manifest::Manifest::entry`. It is a closure rather
/// than a `&Manifest` so that every arm below is drivable from a test with no
/// manifest on disk, and so the caller decides which vendor's census answers.
///
/// # The rule, and why the window starts where it does
///
/// The start is the **earliest** resume point across the series that are
/// behind: a series holding nothing resumes at the span's first day, one
/// holding through day D resumes at D+1. Taking the earliest rather than the
/// latest is what stops a laggard losing days it could still have been given.
/// A series that is *ahead* of that start is offered an overlap, and
/// `BarFile::append` verifies the overlap byte for byte and appends only the
/// suffix — so an overlap costs vendor budget and never corrupts. A start
/// later than a laggard's resume point would cost it those days permanently,
/// and the store could never be told about them afterwards.
///
/// # Cost
///
/// One hash probe and a handful of integer comparisons per series.
/// **O(series)** for a whole month, never O(store), and no directory is
/// listed.
#[must_use]
pub fn next_window<F>(
    held: F,
    series: &[Series],
    month: YearMonth,
    span: (Day, Day),
) -> Option<Unit>
where
    F: Fn(&EntryKey) -> Option<i64>,
{
    let (from, to) = span;
    let mut start: Option<Day> = None;
    let mut behind = 0usize;
    let mut done = 0usize;
    for one in series {
        // ONE PROBE. Nothing here opens a file or lists a directory.
        let resume = match held(&one.at(month)).and_then(day_of) {
            // Nothing held for this key in this month: ask for the whole span.
            None => from,
            Some(day) if day < to => {
                // 9999-12-31 has no successor. Counted as held rather than
                // asked for again, because there is no day after it to ask for.
                let Ok(next) = day.succ() else {
                    done = done.saturating_add(1);
                    continue;
                };
                next
            }
            // Held through the last day this feed can be asked for.
            Some(_) => {
                done = done.saturating_add(1);
                continue;
            }
        };
        behind = behind.saturating_add(1);
        let resume = if resume < from { from } else { resume };
        start = Some(match start {
            Some(current) if current <= resume => current,
            _ => resume,
        });
    }
    let start = start?;
    Some(Unit {
        month,
        window: Window::new(start, to).ok()?,
        behind,
        done,
    })
}

/// What one tick did, as the state machine reads it.
///
/// Derived from the run's own counters and from a fresh probe of the store —
/// never from a clock. A tick that took an hour and stored nothing is not
/// progress, and a tick that took a second and completed the month is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TickOutcome {
    /// Instruments the sweep attempted.
    pub attempted: usize,
    /// Instruments that answered.
    pub reached: usize,
    /// Bars that reached the store.
    pub stored: usize,
    /// The first thing that went wrong, verbatim. `None` when nothing did.
    pub reason: Option<String>,
    /// Whether the store now holds this month through its last askable day.
    ///
    /// **Re-probed after the run**, not inferred from the counters: the store
    /// is the authority on what the store holds.
    pub complete: bool,
    /// Set when the operator stopped the sweep part-way. Not a failure.
    pub stopped: bool,
}

/// What to do after a tick.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Next {
    /// The month is done. Move to the next one.
    Advance,
    /// Ask again for the same month, straight away.
    Retry,
    /// Ask again for the same month after this many seconds.
    Wait {
        /// How long to wait.
        secs: u64,
    },
    /// Give up on this month, name why, and move past it.
    Stall {
        /// The reason, verbatim, kept on the page for the life of the process.
        reason: String,
    },
    /// Terminal for this feed, with the reason on every page.
    Halt {
        /// The reason, verbatim.
        reason: String,
    },
}

/// One month this feed gave up on, and why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stall {
    /// Which month.
    pub month: YearMonth,
    /// How many times it was attempted before it was passed.
    pub attempts: u8,
    /// The reason, verbatim.
    pub reason: String,
}

/// One feed's place in the backfill. In memory, never on disk.
#[derive(Debug)]
pub struct FeedState {
    /// Which feed.
    pub feed: pull::vendor::Feed,
    /// The store prefix its bars are filed under.
    pub vendor: brutex_core::vendor::Vendor,
    /// A monotone hint at the oldest month that might still need work.
    ///
    /// **A hint, never authority.** It is only ever moved forward, and every
    /// month it names is re-probed against the store before anything is
    /// fetched. On restart it begins at the feed's floor and re-derives, so it
    /// cannot disagree with the store because it is never read as truth.
    pub frontier: YearMonth,
    /// Consecutive attempts at the frontier month that made no progress.
    pub attempts: u8,
    /// Consecutive rounds at the frontier month that stored nothing.
    pub dry: u8,
    /// How many backoffs in a row, which is the exponent.
    pub backoff: u32,
    /// How many credential re-reads are still owed before halting.
    pub rereads: u8,
    /// Terminal, with the reason.
    pub halted: Option<String>,
    /// Months passed with a reason. Permanent for the life of the process.
    pub stalls: Vec<Stall>,
    /// The last reason this feed reported, verbatim.
    pub last_reason: Option<String>,
    /// How many months this feed has retired since it started.
    pub months_done: u32,
}

impl FeedState {
    /// A feed at the oldest month it can be asked for.
    #[must_use]
    pub fn new(
        feed: pull::vendor::Feed,
        vendor: brutex_core::vendor::Vendor,
        frontier: YearMonth,
    ) -> Self {
        Self {
            feed,
            vendor,
            frontier,
            attempts: 0,
            dry: 0,
            backoff: 0,
            rereads: CREDENTIAL_REREADS,
            halted: None,
            stalls: Vec::new(),
            last_reason: None,
            months_done: 0,
        }
    }

    /// Forget everything that is about one month, keeping what is about the
    /// feed.
    fn clear_month(&mut self) {
        self.attempts = 0;
        self.dry = 0;
        self.backoff = 0;
    }

    /// The wait after this many consecutive backoffs: 30 s, 60 s, 120 s …
    /// capped at fifteen minutes.
    ///
    /// Shifting rather than multiplying, and saturating at the cap before the
    /// shift can overflow.
    #[must_use]
    pub const fn backoff_secs(steps: u32) -> u64 {
        if steps >= 8 {
            return BACKOFF_CEILING_SECS;
        }
        let secs = BACKOFF_FLOOR_SECS << steps;
        if secs > BACKOFF_CEILING_SECS {
            BACKOFF_CEILING_SECS
        } else {
            secs
        }
    }

    /// What to do after one tick, and the state that follows from it.
    ///
    /// Pure with respect to the world: it reads only `self` and the outcome,
    /// so every branch — halt, stall, backoff, dry, progress — is drivable
    /// from a test with no vendor, no store and no clock.
    ///
    /// The order of the arms is the policy:
    ///
    /// 1. **A credential failure outranks everything.** `CLAUDE.md` §8 — one
    ///    automatic re-read, then halt. It is checked before progress because a
    ///    token that dies mid-sweep still stores the instruments it reached,
    ///    and treating that as progress would retry forever against a dead
    ///    credential.
    /// 2. **Stopped by the operator is not a failure** and costs no attempt.
    /// 3. **Complete advances.**
    /// 4. **Progress resets the bound.** A tick that stored bars moved the
    ///    resume point forward, so the next window is strictly smaller; the
    ///    month cannot loop, and counting it against the bound would stall a
    ///    month that is simply large.
    /// 5. **The same store refusal twice halts.** Hammering a full disk is
    ///    waste.
    /// 6. **Anything else is a transport failure**: backoff, bounded by
    ///    [`MAX_MONTH_ATTEMPTS`], then stalled and passed.
    /// 7. **Nothing stored and nothing wrong is dry.** Two of those retire the
    ///    month, which is `docs/07-plan.md` §9.2's rule.
    pub fn observe(&mut self, out: &TickOutcome) -> Next {
        if let Some(reason) = out.reason.clone() {
            self.last_reason = Some(reason.clone());
            if classify(&reason) == Trouble::Credential {
                if self.rereads > 0 {
                    self.rereads = self.rereads.saturating_sub(1);
                    return Next::Retry;
                }
                let why = format!(
                    "the broker credential is dead and re-reading it returned the same \
                     value. CLAUDE.md §8: this repository never mints a token, so nothing \
                     further is attempted. Refresh it in AWS Parameter Store and press \
                     Resume. The reason, verbatim: {reason}"
                );
                self.halted = Some(why.clone());
                return Next::Halt { reason: why };
            }
        }
        if out.stopped {
            return Next::Retry;
        }
        if out.complete {
            self.clear_month();
            self.rereads = CREDENTIAL_REREADS;
            self.months_done = self.months_done.saturating_add(1);
            return Next::Advance;
        }
        if out.stored > 0 {
            self.clear_month();
            self.rereads = CREDENTIAL_REREADS;
            return Next::Retry;
        }
        if let Some(reason) = out.reason.clone() {
            let repeat = classify(&reason) == Trouble::Store && self.attempts > 0;
            if repeat {
                let why = format!(
                    "the store refused the same write twice: {reason}. This is not \
                     retryable — nothing further is attempted until the disk is dealt \
                     with."
                );
                self.halted = Some(why.clone());
                return Next::Halt { reason: why };
            }
            self.attempts = self.attempts.saturating_add(1);
            if self.attempts >= MAX_MONTH_ATTEMPTS {
                let why = format!(
                    "attempted {} times and never completed: {reason}",
                    self.attempts
                );
                self.stalls.push(Stall {
                    month: self.frontier,
                    attempts: self.attempts,
                    reason: why.clone(),
                });
                self.clear_month();
                return Next::Stall { reason: why };
            }
            let secs = Self::backoff_secs(self.backoff);
            self.backoff = self.backoff.saturating_add(1);
            return Next::Wait { secs };
        }
        self.dry = self.dry.saturating_add(1);
        if self.dry >= DRY_ROUNDS {
            self.clear_month();
            self.months_done = self.months_done.saturating_add(1);
            return Next::Advance;
        }
        Next::Retry
    }
}

/// Everything the status endpoint reports, held in memory.
///
/// **Never rendered from `census_now`.** The counters below are either the
/// manifest's own field reads — one each — taken at the top of a tick, or
/// counts the tick itself produced. A status page that re-read and
/// CRC-verified 248,000 entries per request would be the O(entries) cost
/// D-0039 exists to remove, on the one page an operator refreshes most.
#[derive(Debug, Clone)]
pub struct Status {
    /// What the autopilot as a whole is doing.
    pub phase: Phase,
    /// One sentence naming what it is doing and why.
    pub detail: String,
    /// When this state began, in epoch seconds.
    pub since_unix: i64,
    /// Epoch seconds the next action is due, or 0 when there is no countdown.
    pub due_unix: i64,
    /// How many ticks have run since the process started.
    pub ticks: u64,
    /// Bars this process has stored, summed across every tick.
    pub bars_stored: u64,
    /// The whole span being aimed at.
    pub target: Target,
    /// The cell on the wire right now, if any.
    pub now: Option<InFlight>,
    /// The month the ladder has reached.
    pub cursor: String,
    /// Instruments that did not answer, newest first.
    ///
    /// **Bounded** — see [`MAX_FAILURES`]. An unbounded list on a twelve-hour
    /// run is a payload that grows without limit on the one route the page
    /// polls every two seconds.
    pub failures: Vec<Failed>,
    /// Where the run journal is, so an operator can go and read it.
    pub journal: String,
    /// Milliseconds spent waiting between units, this process.
    pub waiting_ms: u64,
    /// Milliseconds of vendor throttling the governor absorbed, this process.
    pub absorbed_ms: u64,
    /// One report per feed.
    pub feeds: Vec<FeedReport>,
}

/// How many failures the status carries.
///
/// Forty. The page shows the recent ones and the journal holds every one of
/// them for ever — `~/.brutex/store/audit/pull.journal`, whose path travels on
/// the payload precisely so a truncated list is never mistaken for the whole
/// history.
pub const MAX_FAILURES: usize = 40;

impl Default for Status {
    fn default() -> Self {
        Self {
            phase: Phase::Starting,
            detail: "the autopilot has not started yet".to_owned(),
            since_unix: 0,
            due_unix: 0,
            ticks: 0,
            bars_stored: 0,
            target: Target::default(),
            now: None,
            cursor: String::new(),
            failures: Vec::new(),
            journal: String::new(),
            waiting_ms: 0,
            absorbed_ms: 0,
            feeds: Vec::new(),
        }
    }
}

/// What one feed is doing, as the status reports it.
#[derive(Debug, Clone, Default)]
pub struct FeedReport {
    /// The feed's own name.
    pub feed: String,
    /// The store prefix its bars are filed under.
    pub vendor: String,
    /// The month it is working on.
    pub month: String,
    /// The days it asked for, as `from..=to`.
    pub window: String,
    /// Tracked series short of that month's last askable day.
    pub behind: usize,
    /// Tracked series that already hold it.
    pub done: usize,
    /// Attempts spent on the current month.
    pub attempts: u8,
    /// Months retired since this process started.
    pub months_done: u32,
    /// Months between the feed's floor and yesterday.
    pub months_total: u32,
    /// Month files the store holds for this vendor. One field read.
    pub store_months: u64,
    /// Rows the store holds for this vendor. One field read.
    pub store_rows: u64,
    /// The last reason this feed reported, verbatim.
    pub last_reason: String,
    /// The reason this feed is terminal, verbatim, or empty.
    pub halted: String,
    /// Months passed with a reason. Permanent for the life of the process.
    pub stalls: Vec<Stall>,
}

impl Status {
    /// The state word this payload will carry, which is not always the phase's
    /// own.
    ///
    /// `Phase::Running` with nothing in flight is published as `waiting`, and
    /// that is honesty rather than a fudge: the page refuses a `running`
    /// payload whose `now` is null because — in its own words — *nothing is in
    /// flight, so it is not running*. Between two units the autopilot genuinely
    /// is waiting, and saying so is the difference this whole route exists to
    /// make visible.
    #[must_use]
    pub fn state(&self) -> &'static str {
        match self.phase {
            Phase::Running if self.now.is_none() => Phase::Backoff.state(),
            phase => phase.state(),
        }
    }

    /// The reason, which D-0057 makes **mandatory** for `paused` and `halted`.
    ///
    /// A stop that does not name its reason is the failure `CLAUDE.md` §4 bans,
    /// and the page rejects the payload rather than rendering a blank. This
    /// never returns empty for those two states: [`Status::detail`] is written
    /// on every transition into them, and if one ever were empty this says so
    /// out loud rather than shipping a contract violation.
    #[must_use]
    pub fn why(&self) -> String {
        if !self.detail.is_empty() {
            return self.detail.clone();
        }
        String::from(
            "the autopilot changed state without recording a reason, which is itself \
             the bug — nothing here should be able to stop without naming why",
        )
    }

    /// The status as JSON, hand-written for the same reason every other JSON
    /// route in this crate is: there is no serialiser in this dependency
    /// graph and one is not worth a decision entry for six objects.
    ///
    /// # The shape is D-0057's, not this module's
    ///
    /// `state`, `why`, `target`, `now`, `failures` and their nullability rules
    /// are fixed by the decision and enforced by
    /// `web/src/routes/autopilot/+page.svelte`, which names the missing field
    /// rather than rendering `undefined`. `failures` is emitted even when
    /// empty, because the page refuses an absent list on the grounds that a
    /// missing list and an empty list are different facts and only the second
    /// is good news. The `phase`/`feeds` fields below are additional detail the
    /// page ignores.
    #[must_use]
    pub fn json(&self) -> String {
        use std::fmt::Write as _;
        let mut out = String::with_capacity(2048);
        let _ = write!(
            out,
            r#"{{"state":{},"why":{},"cursor":{},"journal":{},"waiting_ms":{},"absorbed_ms":{},"target":{{"from":{},"to":{},"instruments":{},"timeframe":{},"feed":{}}},"now":"#,
            render::json_string(self.state()),
            render::json_string(&self.why()),
            render::json_string(&self.cursor),
            render::json_string(&self.journal),
            self.waiting_ms,
            self.absorbed_ms,
            render::json_string(&self.target.from),
            render::json_string(&self.target.to),
            self.target.instruments,
            render::json_string(&self.target.timeframe),
            render::json_string(&self.target.feed),
        );
        match self.now {
            None => out.push_str("null"),
            Some(ref now) => {
                // A DURATION THE SERVER MEASURED, never a start instant.
                let elapsed = u64::try_from(now.since.elapsed().as_millis()).unwrap_or(u64::MAX);
                let _ = write!(
                    out,
                    r#"{{"instrument":{},"month":{},"timeframe":{},"feed":{},"elapsed_ms":{},"index":{},"of":{}}}"#,
                    render::json_string(&now.instrument),
                    render::json_string(&now.month),
                    render::json_string(&now.timeframe),
                    render::json_string(&now.feed),
                    elapsed,
                    now.index,
                    now.of,
                );
            }
        }
        out.push_str(r#","failures":["#);
        for (n, failed) in self.failures.iter().enumerate() {
            if n > 0 {
                out.push(',');
            }
            let _ = write!(
                out,
                r#"{{"instrument":{},"month":{},"why":{},"at":{}}}"#,
                render::json_string(&failed.instrument),
                render::json_string(&failed.month),
                render::json_string(&failed.why),
                render::json_string(&failed.at),
            );
        }
        out.push(']');
        let _ = write!(
            out,
            r#","phase":{},"detail":{},"since_unix":{},"due_unix":{},"ticks":{},"bars_stored":{},"feeds":["#,
            render::json_string(self.phase.state()),
            render::json_string(&self.detail),
            self.since_unix,
            self.due_unix,
            self.ticks,
            self.bars_stored
        );
        for (n, feed) in self.feeds.iter().enumerate() {
            if n > 0 {
                out.push(',');
            }
            let _ = write!(
                out,
                r#"{{"feed":{},"vendor":{},"month":{},"window":{},"behind":{},"done":{},"attempts":{},"attempts_max":{},"months_done":{},"months_total":{},"store_months":{},"store_rows":{},"last_reason":{},"halted":{},"stalls":["#,
                render::json_string(&feed.feed),
                render::json_string(&feed.vendor),
                render::json_string(&feed.month),
                render::json_string(&feed.window),
                feed.behind,
                feed.done,
                feed.attempts,
                MAX_MONTH_ATTEMPTS,
                feed.months_done,
                feed.months_total,
                feed.store_months,
                feed.store_rows,
                render::json_string(&feed.last_reason),
                render::json_string(&feed.halted),
            );
            for (k, stall) in feed.stalls.iter().enumerate() {
                if k > 0 {
                    out.push(',');
                }
                let _ = write!(
                    out,
                    r#"{{"month":{},"attempts":{},"reason":{}}}"#,
                    render::json_string(&stall.month.to_string()),
                    stall.attempts,
                    render::json_string(&stall.reason)
                );
            }
            out.push_str("]}");
        }
        out.push_str("]}");
        out
    }
}

/// The operator's two controls and the one place the state is published.
///
/// Held on `Site`, which is the `Arc` both the handlers and the background
/// task already share. Atomics rather than a lock for the two flags, so the
/// check inside a 773-instrument sweep is one relaxed load and never blocks a
/// request.
#[derive(Debug)]
pub struct Control {
    /// Whether the operator has stopped it.
    paused: AtomicBool,
    /// Bumped every time a run is asked to stop.
    ///
    /// A generation counter rather than a "cancel" flag, because a flag has a
    /// race: a pause that fires just as a sweep ends would leave the flag
    /// raised and abort the operator's *next* manual pull for no reason. A
    /// run captures the epoch when it starts and stops only when the epoch has
    /// moved since — a run that began after the pause captures the new value
    /// and is unaffected.
    epoch: AtomicU64,
    /// The one in-process pull seat.
    ///
    /// `pull::ingest`'s census lock **refuses rather than queues**, and it is
    /// taken and released once per chunk — so a manual pull landing inside an
    /// autopilot tick would see every instrument fail with a lock message.
    /// One seat, taken for the whole tick, turns that into one honest 409 that
    /// names the resolution.
    seat: AtomicBool,
    /// What to report. A lock, because it is a paragraph rather than a word,
    /// and it is written once per tick and read once per page.
    status: std::sync::Mutex<Status>,
}

impl Default for Control {
    fn default() -> Self {
        Self::new()
    }
}

impl Control {
    /// A control that has not started, and is not paused.
    #[must_use]
    pub fn new() -> Self {
        Self {
            paused: AtomicBool::new(false),
            epoch: AtomicU64::new(0),
            seat: AtomicBool::new(false),
            status: std::sync::Mutex::new(Status::default()),
        }
    }

    /// Whether the operator has stopped it. One relaxed load.
    #[must_use]
    pub fn is_paused(&self) -> bool {
        self.paused.load(Ordering::Relaxed)
    }

    /// Stop, and ask any sweep in flight to stop at its next instrument.
    pub fn pause(&self) {
        self.paused.store(true, Ordering::Relaxed);
        self.epoch.fetch_add(1, Ordering::Relaxed);
    }

    /// Start again. Does not bump the epoch: a sweep that is still unwinding
    /// must still unwind.
    pub fn resume(&self) {
        self.paused.store(false, Ordering::Relaxed);
    }

    /// The current stop generation, captured by a run when it starts.
    #[must_use]
    pub fn epoch(&self) -> u64 {
        self.epoch.load(Ordering::Relaxed)
    }

    /// Whether a run that captured `at` has been asked to stop. One relaxed
    /// load and one comparison — this is the check inside the instrument loop.
    #[must_use]
    pub fn stopped(&self, at: u64) -> bool {
        self.epoch.load(Ordering::Relaxed) != at
    }

    /// Take the one pull seat, or `None` when something else holds it.
    ///
    /// A compare-exchange rather than a `Mutex`, because the seat is held
    /// across `await` points and a `std::sync::MutexGuard` is not `Send` —
    /// which would make the whole background task unspawnable.
    #[must_use]
    pub fn take_seat(&self) -> Option<Seat<'_>> {
        self.seat
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .ok()
            .map(|_| Seat { held: &self.seat })
    }

    /// Publish a new status. Never blocks a fetch: on a poisoned lock the
    /// update is dropped and the previous status stands, which is stale rather
    /// than wrong.
    pub fn publish<F: FnOnce(&mut Status)>(&self, edit: F) {
        if let Ok(mut held) = self.status.lock() {
            edit(&mut held);
        }
    }

    /// Record one instrument that did not answer, newest first.
    ///
    /// **Bounded at [`MAX_FAILURES`], and the truncation is not a hidden one:**
    /// the journal path travels on the same payload, and every failure is in it
    /// for ever. An unbounded list would grow without limit on a route the page
    /// polls every two seconds.
    pub fn fail(&self, instrument: &str, month: &str, why: &str) {
        let at = ingest::ist_day(std::time::SystemTime::now())
            .map_or_else(|_| String::new(), |day| day.to_string());
        self.publish(|status| {
            status.failures.insert(
                0,
                Failed {
                    instrument: instrument.to_owned(),
                    month: month.to_owned(),
                    why: why.to_owned(),
                    at,
                },
            );
            status.failures.truncate(MAX_FAILURES);
        });
    }

    /// The status as JSON.
    ///
    /// A poisoned lock answers with a named refusal rather than an empty
    /// object — `CLAUDE.md` §4, degrade loudly and say why.
    #[must_use]
    pub fn json(&self) -> String {
        self.status.lock().map_or_else(
            |_| {
                Status {
                    phase: Phase::Halted,
                    detail:
                        "the autopilot's status lock is poisoned: a previous publisher panicked, \
                         so what it is doing cannot be reported"
                            .to_owned(),
                    ..Status::default()
                }
                .json()
            },
            |held| held.json(),
        )
    }
}

/// The one pull seat, released when this is dropped.
#[derive(Debug)]
pub struct Seat<'a> {
    held: &'a AtomicBool,
}

impl Drop for Seat<'_> {
    fn drop(&mut self) {
        self.held.store(false, Ordering::Release);
    }
}

/// Every series the sweep will actually fetch, as the store keys them.
///
/// The **same** `catalog::tracked` predicate `broker_run` iterates, mapped
/// through the same three fields `land_one` files bars under — so a cell this
/// says is missing is a cell that sweep can fill. A second notion of "the
/// universe" here would be a fourth spelling of it, and the one that is wrong
/// is the one that reports a gap nothing can close.
///
/// Computed once when the autopilot starts, for the reason `Site::series` is:
/// it is O(rows log rows) and that belongs beside the manifest load, not
/// inside a tick.
#[must_use]
pub fn tracked_series(site: &Site, timeframe: Timeframe) -> Vec<Series> {
    let mut out: Vec<Series> = site
        .read
        .merged
        .by_key
        .iter()
        .filter(|(_, entry)| crate::catalog::tracked(entry.universe))
        .map(|(key, _)| Series {
            exchange: key.exchange,
            segment: key.segment,
            symbol: key.underlying,
            timeframe,
        })
        .collect();
    out.sort_unstable();
    out.dedup();
    out
}

/// A month ordinal, so month arithmetic is a subtraction rather than a
/// calendar walk.
#[must_use]
pub const fn ordinal(month: YearMonth) -> u32 {
    (month.year() as u32) * 12 + (month.month() as u32) - 1
}

/// The month `ordinal` names, or `None` outside the store's range.
#[must_use]
pub fn from_ordinal(ordinal: u32) -> Option<YearMonth> {
    let year = u16::try_from(ordinal / 12).ok()?;
    let month = u8::try_from(ordinal % 12 + 1).ok()?;
    YearMonth::new(year, month).ok()
}

/// The next month, or `None` past 9999-12.
#[must_use]
pub fn month_after(month: YearMonth) -> Option<YearMonth> {
    from_ordinal(ordinal(month).checked_add(1)?)
}

/// The manifest one vendor's census holds, if it loaded.
#[must_use]
fn manifest_of(
    censuses: &[VendorCensus],
    vendor: brutex_core::vendor::Vendor,
) -> Option<&Manifest> {
    censuses.iter().find(|c| c.vendor == vendor).and_then(|c| {
        match c.state {
            Census::Held { ref manifest } => Some(&**manifest),
            // AN UNREADABLE CENSUS IS NOT AN EMPTY ONE. Reporting `None` here
            // would make every key look missing and re-fetch a store that is
            // already full. The caller checks the state itself and refuses.
            Census::Absent | Census::Unreadable { .. } => None,
        }
    })
}

use pull::manifest::Manifest;

/// The first month at or after `hint` that this feed still owes work for.
///
/// Scans upward and stops at the first incomplete month. The hint is monotone,
/// so across the life of a process each month is scanned once rather than once
/// per tick: 76 months × 773 probes is roughly 58,700 hash probes at cold
/// start, about a millisecond, and then nothing.
///
/// Returns the month, the unit it needs, and how many months the feed owes in
/// total — so the status can report progress without a second pass.
#[must_use]
pub fn frontier<F>(
    held: F,
    series: &[Series],
    hint: YearMonth,
    floor: Day,
    yesterday: Day,
) -> (YearMonth, Option<Unit>)
where
    F: Fn(&EntryKey) -> Option<i64>,
{
    let last = yesterday.year_month().unwrap_or(hint);
    let mut month = hint;
    while ordinal(month) <= ordinal(last) {
        if let Some(span) = month_span(month, floor, yesterday)
            && let Some(unit) = next_window(&held, series, month, span)
        {
            return (month, Some(unit));
        }
        let Some(next) = month_after(month) else {
            break;
        };
        month = next;
    }
    (month, None)
}

/// How many months lie between a feed's floor and yesterday, inclusive.
#[must_use]
pub fn months_owed(floor: Day, yesterday: Day) -> u32 {
    let (Ok(from), Ok(to)) = (floor.year_month(), yesterday.year_month()) else {
        return 0;
    };
    ordinal(to).saturating_sub(ordinal(from)).saturating_add(1)
}

/// The feed's history floor as a day, taken from the descriptor through the
/// **same** clamp the manual pull uses.
///
/// Recomputed on every tick and never stored: Dhan's floor rolls, and a stored
/// rolling floor is a frozen one.
#[must_use]
pub fn floor_day(feed: pull::vendor::Feed, yesterday: Day) -> Option<Day> {
    let pull::vendor::Transport::Http(spec) = feed.descriptor().transport else {
        return None;
    };
    let epoch = Day::from_days(0).ok()?;
    let whole = Window::new(epoch, yesterday).ok()?;
    crate::server::clamp_to_floor(whole, spec.history_floor)
        .ok()
        .map(Window::from)
}

/// Every feed this process may drive: an HTTP transport, and a store prefix to
/// file under.
///
/// Derived from `pull::vendor::DESCRIPTORS`, not written out — a fifth feed is
/// driven the day its row lands, and a feed with nowhere to file its bars is
/// skipped here rather than refused 773 times inside a sweep.
#[must_use]
pub fn drivable(yesterday: Day) -> Vec<FeedState> {
    pull::vendor::Feed::ALL
        .into_iter()
        .filter_map(|feed| {
            let vendor = feed.store_vendor()?;
            let floor = floor_day(feed, yesterday)?;
            let month = floor.year_month().ok()?;
            Some(FeedState::new(feed, vendor, month))
        })
        .collect()
}

/// Yesterday, in IST, from a moment.
///
/// **Never today.** A session still running yields a partial day the
/// append-only store cannot correct later.
#[must_use]
pub fn yesterday_ist(now: std::time::SystemTime) -> Option<Day> {
    let today = ingest::ist_day(now).ok()?;
    Day::from_days(today.days_from_epoch().checked_sub(1)?).ok()
}

/// The background task: decide, fetch, report, repeat.
///
/// Spawned beside the HTTP server and holding the same `Arc`, so it can never
/// block a request — every wait is an `await`, and the status a page reads is
/// an in-memory field read behind one uncontended lock.
///
/// # Why it does nothing at all unless the process may reach a broker
///
/// `Site::serving` is the only constructor that sets `Broker::Live`, and only
/// `run_in` calls it. A test-constructed `Site` therefore cannot start a
/// backfill even if something spawns this, which is the guard that already
/// exists for exactly this hazard — no `cfg!(test)` is added and none should
/// be.
pub async fn fly(site: Loaded) {
    if site.broker != Broker::Live {
        site.autopilot.publish(|status| {
            status.phase = Phase::Halted;
            status.detail = String::from(
                "this process may not reach a live broker, so the autopilot will not \
                 start. Only the served binary sets Broker::Live.",
            );
        });
        return;
    }
    let Some(yesterday) = yesterday_ist(std::time::SystemTime::now()) else {
        site.autopilot.publish(|status| {
            status.phase = Phase::Halted;
            status.detail = String::from(
                "the clock is unusable, so the newest finished day cannot be established \
                 and nothing can be asked for safely",
            );
        });
        return;
    };
    // ONE CONVERSION FROM RUNG TO DIRECTORY, and it is the store's own. A
    // literal `Timeframe::MINUTE_1` here would be a second answer to "where do
    // these bars go" beside `pull::ingest::Plan::timeframe`.
    let granularity = pull::vendor::Granularity::Minute1;
    let Some(timeframe) = granularity.store_timeframe() else {
        site.autopilot.publish(|status| {
            status.phase = Phase::Halted;
            status.detail =
                String::from("the one-minute rung has no directory in this store build");
        });
        return;
    };
    let series = tracked_series(&site, timeframe);
    let mut feeds = drivable(yesterday);
    // THE TARGET IS PUBLISHED BEFORE THE GRACE WINDOW, NOT AFTER THE FIRST
    // ROUND.
    //
    // D-0057 makes `target.from`, `target.to` and `target.instruments`
    // mandatory, and the page's own reader treats an EMPTY STRING as absent —
    // `str()` returns null for `""`. So a target left blank until the first
    // tick is twenty seconds of contract violation on the one route the page
    // polls, and the page would refuse the payload by name rather than draw the
    // countdown. Measured against the real reader, which is why this is here.
    let opening = Target {
        // The oldest day ANY drivable feed can be asked for — the bottom of the
        // ladder, which is what the span means before a feed is chosen.
        from: feeds
            .iter()
            .filter_map(|f| floor_day(f.feed, yesterday))
            .min()
            .map_or_else(|| yesterday.to_string(), |floor| floor.to_string()),
        to: yesterday.to_string(),
        instruments: series.len(),
        timeframe: granularity.to_string(),
        feed: String::new(),
    };
    let journal = site.journal().path.display().to_string();
    site.autopilot.publish(|status| {
        status.target = opening;
        status.journal = journal;
    });
    grace(&site).await;
    loop {
        if site.autopilot.is_paused() {
            dwell_paused(&site.autopilot).await;
            continue;
        }
        let waited = round(&site, &mut feeds, &series, granularity).await;
        // BETWEEN UNITS, ALWAYS. The sweep itself awaits on every request, so
        // this is belt and braces for the one path that might not — a tick that
        // decides there is nothing to do and loops.
        tokio::task::yield_now().await;
        if waited > 0 {
            nap(&site, waited).await;
        }
    }
}

/// What a paused autopilot does with one second: say so, and then genuinely
/// sleep.
///
/// # Why this is a function, and why it does not call [`nap`]
///
/// `nap` returns the instant it sees the pause flag — which is exactly what
/// makes a fifteen-minute backoff end promptly when the operator presses Pause.
/// Using it *here* means returning without ever awaiting, and the `continue`
/// above it then spins the loop at the speed of the scheduler.
///
/// That is not a hypothetical. Measured on the built binary before this
/// existed: **99% of a core**, and the tight `publish` inside the spin starved
/// the status mutex so badly that `GET /autopilot.json` timed out after five
/// seconds. A paused autopilot that cannot be asked what it is doing is the
/// silent failure `CLAUDE.md` §4 bans — reached, of all ways, through the
/// control that exists to make it visible.
///
/// Taking `&Control` rather than `&Loaded` so the property can be timed in a
/// test without a `Site`, a store, or a masters directory.
async fn dwell_paused(control: &Control) {
    control.publish(|status| {
        status.phase = Phase::Paused;
        status.detail = String::from(
            "stopped by the operator. Nothing is being asked of any vendor. Press \
             Resume to carry on from where the store already reaches.",
        );
        status.due_unix = 0;
    });
    tokio::time::sleep(std::time::Duration::from_secs(PAUSED_POLL_SECS)).await;
}

/// The grace window: the operator's chance to say no before a socket opens.
async fn grace(site: &Loaded) {
    for left in (1..=GRACE_SECS).rev() {
        if site.autopilot.is_paused() {
            return;
        }
        site.autopilot.publish(|status| {
            status.phase = Phase::Starting;
            status.detail = format!(
                "the autopilot starts in {left}s. It will fetch the oldest month the \
                 store is missing, and never today. Press Pause to stop it before \
                 anything is asked of a vendor."
            );
            status.since_unix = ingest::epoch_secs(std::time::SystemTime::now());
            status.due_unix = status
                .since_unix
                .saturating_add(i64::try_from(left).unwrap_or(i64::MAX));
        });
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }
}

/// Sleep `secs`, one second at a time, so a pause bites within a second and
/// the countdown on the page is real.
async fn nap(site: &Loaded, secs: u64) {
    for left in (1..=secs).rev() {
        if site.autopilot.is_paused() {
            return;
        }
        site.autopilot.publish(|status| {
            status.due_unix = ingest::epoch_secs(std::time::SystemTime::now())
                .saturating_add(i64::try_from(left).unwrap_or(i64::MAX));
        });
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }
}

/// Where every feed stands, and which one owes the oldest month.
///
/// Reads the store and writes nothing to it. Separated from [`round`] because
/// deciding and doing are different jobs: this one is pure enough to reason
/// about — it touches no socket and no clock beyond the `yesterday` handed to
/// it — while `round` is the part that spends the operator's rate budget.
///
/// # Cost
///
/// One probe per (series, month) considered, and the frontier hint is monotone,
/// so across the life of a process each month is examined once rather than once
/// per pass.
fn survey(
    feeds: &mut [FeedState],
    series: &[Series],
    censuses: &[VendorCensus],
    yesterday: Day,
) -> (Vec<FeedReport>, Option<(usize, Unit)>) {
    // OLDEST FIRST, ACROSS FEEDS. Not "finish Groww then start Dhan": the
    // ladder is climbed globally, so an interruption leaves every feed
    // uniformly deep rather than one complete and one empty.
    let mut chosen: Option<(usize, Unit)> = None;
    let mut reports: Vec<FeedReport> = Vec::with_capacity(feeds.len());
    for (slot, state) in feeds.iter_mut().enumerate() {
        let mut report = FeedReport {
            feed: state.feed.display().to_owned(),
            vendor: state.vendor.as_str().to_owned(),
            attempts: state.attempts,
            months_done: state.months_done,
            last_reason: state.last_reason.clone().unwrap_or_default(),
            halted: state.halted.clone().unwrap_or_default(),
            stalls: state.stalls.clone(),
            ..FeedReport::default()
        };
        let floor = floor_day(state.feed, yesterday);
        if let Some(floor) = floor {
            report.months_total = months_owed(floor, yesterday);
        }
        let manifest = manifest_of(censuses, state.vendor);
        if let Some(manifest) = manifest {
            // ONE FIELD READ EACH, never a walk. `docs/04-invariants.md`
            // M-series.
            report.store_months = manifest.keys();
            report.store_rows = manifest.total_rows();
        }
        if state.halted.is_none()
            && let Some(floor) = floor
        {
            // A CENSUS THAT EXISTS AND WILL NOT LOAD IS NOT AN EMPTY ONE.
            // Treating it as empty would re-fetch a store that is already full;
            // treating it as full would hide a real gap. It is named and this
            // feed does nothing until an operator deals with it.
            let unreadable =
                censuses
                    .iter()
                    .find(|c| c.vendor == state.vendor)
                    .and_then(|c| match c.state {
                        Census::Unreadable { ref reason } => Some(reason.clone()),
                        Census::Absent | Census::Held { .. } => None,
                    });
            if let Some(reason) = unreadable {
                let why = format!(
                    "{}'s manifest exists and will not load, so what the store holds \
                     cannot be established and nothing is fetched against a guess: {reason}",
                    state.vendor.as_str()
                );
                state.halted = Some(why.clone());
                report.halted = why;
            } else {
                let (at, unit) = frontier(
                    |key| {
                        manifest
                            .and_then(|m| m.entry(key))
                            .map(|e| e.last_ts_micros)
                    },
                    series,
                    state.frontier,
                    floor,
                    yesterday,
                );
                state.frontier = at;
                report.month = at.to_string();
                if let Some(ref unit) = unit {
                    report.behind = unit.behind;
                    report.done = unit.done;
                    report.window = format!("{}..={}", unit.window.from(), unit.window.to());
                    let older = chosen
                        .as_ref()
                        .is_none_or(|(_, held)| ordinal(unit.month) < ordinal(held.month));
                    if older {
                        chosen = Some((slot, unit.clone()));
                    }
                }
            }
        }
        reports.push(report);
    }
    (reports, chosen)
}

/// One pass over every drivable feed: pick the oldest month anything owes, do
/// it, and record what happened.
///
/// Returns how many seconds to wait before the next pass. Zero means carry on.
async fn round(
    site: &Loaded,
    feeds: &mut [FeedState],
    series: &[Series],
    granularity: pull::vendor::Granularity,
) -> u64 {
    // THE SEAT FIRST, BEFORE THE CENSUS IS READ.
    //
    // A hand-made pull holds it for the minutes its own month takes. Finding
    // that out *after* reading every manifest would re-read ~16 MB and
    // CRC-verify every entry on each attempt, and returning "carry on" from it
    // would spin that at the speed of the loop — which is the one way this task
    // could starve the requests it is supposed to stay out of the way of. So it
    // is checked before any work, and it waits rather than spinning.
    let Some(_seat) = site.autopilot.take_seat() else {
        site.autopilot.publish(|status| {
            status.phase = Phase::Paused;
            status.detail = String::from(
                "a hand-made pull holds the pull seat, so the backfill is standing off \
                 until it finishes. Nothing is lost: the next unit is re-derived from \
                 whatever the store holds by then.",
            );
            status.due_unix = ingest::epoch_secs(std::time::SystemTime::now())
                .saturating_add(i64::try_from(SEAT_WAIT_SECS).unwrap_or(i64::MAX));
        });
        return SEAT_WAIT_SECS;
    };
    let Some(yesterday) = yesterday_ist(std::time::SystemTime::now()) else {
        return IDLE_POLL_SECS;
    };
    // ONE CENSUS READ PER PASS, not per cell. O(entries) once, against a pass
    // that takes minutes — the same bargain D-0039 struck for the site.
    let censuses = census::read_all(&site.store_root);

    let (reports, chosen) = survey(feeds, series, &censuses, yesterday);

    let Some((slot, unit)) = chosen else {
        let terminal = feeds.iter().all(|f| f.halted.is_some());
        site.autopilot.publish(move |status| {
            status.phase = if terminal { Phase::Halted } else { Phase::Idle };
            status.detail = String::from(if terminal {
                "every feed is halted. The reasons are below and nothing further is \
                 attempted until they are dealt with."
            } else {
                "nothing is missing that any feed can still be asked for. The store is \
                 complete through the newest finished day; this re-checks once a minute \
                 so a new day is picked up on its own."
            });
            status.since_unix = ingest::epoch_secs(std::time::SystemTime::now());
            status.feeds = reports;
        });
        return IDLE_POLL_SECS;
    };

    let Some(state) = feeds.get_mut(slot) else {
        return IDLE_POLL_SECS;
    };
    let label = format!("{} · {}", state.feed.display(), unit.month);
    // THE WHOLE SPAN BEING AIMED AT — `docs/07-plan.md` R-2, recomputed rather
    // than remembered so a rolling floor is never a frozen one.
    let target = Target {
        from: floor_day(state.feed, yesterday).map_or_else(String::new, |floor| floor.to_string()),
        to: yesterday.to_string(),
        instruments: series.len(),
        timeframe: granularity.to_string(),
        feed: state.feed.display().to_owned(),
    };
    let cursor = unit.month.to_string();
    let journal = site.journal().path.display().to_string();
    site.autopilot.publish(|status| {
        status.phase = Phase::Running;
        status.detail = format!(
            "fetching {label}, days {}..={} — {} of {} tracked series still short of \
             this month's last finished day.",
            unit.window.from(),
            unit.window.to(),
            unit.behind,
            unit.behind.saturating_add(unit.done)
        );
        status.target = target;
        status.cursor = cursor;
        status.journal = journal;
        status.since_unix = ingest::epoch_secs(std::time::SystemTime::now());
        status.due_unix = 0;
        status.ticks = status.ticks.saturating_add(1);
        status.feeds = reports;
    });

    let out = tick(site, state, &unit, granularity, series).await;
    let stored = u64::try_from(out.stored).unwrap_or(u64::MAX);
    site.autopilot.publish(move |status| {
        status.bars_stored = status.bars_stored.saturating_add(stored);
    });

    settle(site, state, &out)
}

/// Act on what a tick did: move the frontier, publish the reason, and say how
/// long to wait.
///
/// Split out of [`round`] because it is the half that has no I/O in it at all —
/// [`FeedState::observe`] decides, and this only writes the decision down where
/// an operator can read it.
fn settle(site: &Loaded, state: &mut FeedState, out: &TickOutcome) -> u64 {
    match state.observe(out) {
        Next::Advance => {
            if let Some(next) = month_after(state.frontier) {
                state.frontier = next;
            }
            0
        }
        Next::Retry => 0,
        Next::Wait { secs } => {
            let why = state.last_reason.clone().unwrap_or_default();
            let month = state.frontier;
            let attempts = state.attempts;
            site.autopilot.publish(move |status| {
                status.phase = Phase::Backoff;
                status.detail = format!(
                    "{month} did not complete. Attempt {attempts} of {MAX_MONTH_ATTEMPTS}; \
                     retrying the same month in {secs}s. The reason, verbatim: {why}"
                );
                status.due_unix = ingest::epoch_secs(std::time::SystemTime::now())
                    .saturating_add(i64::try_from(secs).unwrap_or(i64::MAX));
            });
            secs
        }
        Next::Stall { reason } => {
            let month = state.frontier;
            if let Some(next) = month_after(state.frontier) {
                state.frontier = next;
            }
            site.autopilot.publish(move |status| {
                status.detail = format!(
                    "{month} was passed over after {MAX_MONTH_ATTEMPTS} attempts and stays \
                     on the list below until it succeeds: {reason}"
                );
            });
            0
        }
        Next::Halt { reason } => {
            site.autopilot.publish(move |status| {
                status.phase = Phase::Halted;
                status.detail = reason;
                status.due_unix = 0;
            });
            IDLE_POLL_SECS
        }
    }
}

/// One unit of work: run the existing sweep, journal it, and re-probe the store
/// for the answer.
///
/// The pull seat is already held by [`round`] for the whole pass, so this is
/// never reached while a hand-made pull is running.
async fn tick(
    site: &Loaded,
    state: &FeedState,
    unit: &Unit,
    granularity: pull::vendor::Granularity,
    series: &[Series],
) -> TickOutcome {
    let asked = ingest::SpotRequest {
        // SWEPT, AND IT MEANS ALL OF THEM. `broker_run` iterates
        // `catalog::tracked`; `broker_window` refuses any other target
        // outright. Anything else here fetches nothing at all.
        target: ingest::SpotTarget::Swept,
        window: unit.window,
        feed: state.feed,
        granularity,
    };
    let now = std::time::SystemTime::now();
    let started = std::time::Instant::now();
    // THE EXISTING PULL PATH, UNCHANGED. The month split, the floor clamp, the
    // AIMD governor's waiting, the retry backoff, the credential read and the
    // store append are all the ones `/pull/spot` runs.
    let run = crate::server::broker_run(&asked, site).await;
    let took = u64::try_from(started.elapsed().as_micros()).unwrap_or(u64::MAX);
    let source = format!("autopilot {} {}", state.feed.display(), unit.month);

    // ONE JOURNAL RECORD PER TICK, through the same constructors the HTTP
    // receipt uses — so `/audit` cannot tell an autopilot run from a hand-made
    // one except by its source, which is the point.
    let record = if run.reached == 0 {
        let why = run
            .blocked
            .clone()
            .or_else(|| run.refused.first().cloned())
            .unwrap_or_else(|| "no instrument in the universe could be reached".to_owned());
        audit::Record::refused(
            audit::Scope::Spot,
            audit::Outcome::NotStarted,
            now,
            &source,
            &why,
        )
        .with_window(unit.window)
    } else {
        audit::Record::of_run(
            audit::Scope::Spot,
            now,
            took,
            &source,
            unit.window,
            &run.total,
        )
    };
    let _ignored = site.journal().append(&record);

    // THE STORE IS THE AUTHORITY ON WHETHER THE MONTH IS DONE, not the
    // counters this run happens to hold. One census read and one probe per
    // series.
    let censuses = census::read_all(&site.store_root);
    let manifest = manifest_of(&censuses, state.vendor);
    let complete = yesterday_ist(std::time::SystemTime::now())
        .and_then(|yesterday| {
            let floor = floor_day(state.feed, yesterday)?;
            let span = month_span(unit.month, floor, yesterday)?;
            Some(
                next_window(
                    |key| {
                        manifest
                            .and_then(|m| m.entry(key))
                            .map(|e| e.last_ts_micros)
                    },
                    series,
                    unit.month,
                    span,
                )
                .is_none(),
            )
        })
        .unwrap_or(false);

    let reason = run
        .blocked
        .clone()
        .or_else(|| {
            run.total
                .failures
                .first()
                .map(|f| format!("{} — {}", f.instrument, f.why))
        })
        .or_else(|| run.refused.first().cloned());

    TickOutcome {
        attempted: run.attempted,
        reached: run.reached,
        stored: run.total.bars_stored,
        reason,
        complete,
        stopped: run.stopped.is_some(),
    }
}

/// `GET /autopilot.json` — what it is doing, right now.
///
/// Served from the in-memory status and **never** from `census_now`: this is
/// the page an operator refreshes every few seconds during a twelve-hour run,
/// and re-reading every manifest per request is the O(entries) cost D-0039
/// exists to remove.
pub async fn status_json(
    axum::extract::State(site): axum::extract::State<Loaded>,
) -> ([(axum::http::HeaderName, &'static str); 1], String) {
    (
        [(
            axum::http::header::CONTENT_TYPE,
            "application/json; charset=utf-8",
        )],
        site.autopilot.json(),
    )
}

/// `POST /autopilot/pause` — stop, within a second, not within a month.
///
/// The sweep in flight stops at its next instrument: one relaxed atomic load
/// per instrument, so pausing a 773-instrument month does not mean waiting out
/// the other 700.
pub async fn pause(
    axum::extract::State(site): axum::extract::State<Loaded>,
) -> ([(axum::http::HeaderName, &'static str); 1], String) {
    site.autopilot.pause();
    site.autopilot.publish(|status| {
        status.phase = Phase::Paused;
        status.detail = String::from(
            "pause requested. The sweep stops at its next instrument; the partial month \
             is refilled automatically on resume, because the resume point is the \
             store's own.",
        );
    });
    status_json(axum::extract::State(site)).await
}

/// `POST /autopilot/resume` — carry on from wherever the store reaches.
///
/// Nothing is replayed and nothing is lost: the next round re-derives the
/// frontier from the manifest, so a month interrupted half way is picked up at
/// the day after its last stored bar.
pub async fn resume(
    axum::extract::State(site): axum::extract::State<Loaded>,
) -> ([(axum::http::HeaderName, &'static str); 1], String) {
    site.autopilot.resume();
    site.autopilot.publish(|status| {
        status.phase = Phase::Running;
        status.detail =
            String::from("resumed. The next unit is whatever the store is missing, oldest first.");
    });
    status_json(axum::extract::State(site)).await
}

#[cfg(test)]
#[allow(
    clippy::indexing_slicing,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic
)]
mod tests {
    use super::*;
    use brutex_core::instrument::{Exchange, Segment};
    use brutex_core::symbol::Symbol;
    use std::collections::HashMap;

    fn day(y: u16, m: u8, d: u8) -> Day {
        Day::new(y, m, d).unwrap()
    }

    fn month(y: u16, m: u8) -> YearMonth {
        YearMonth::new(y, m).unwrap()
    }

    fn series(name: &str) -> Series {
        Series {
            exchange: Exchange::Nse,
            segment: Segment::Index,
            symbol: Symbol::new(name).unwrap(),
            timeframe: Timeframe::MINUTE_1,
        }
    }

    /// The last microsecond of 15:29 IST on a day, which is what a full
    /// one-minute session leaves behind.
    fn close_of(d: Day) -> i64 {
        // 09:15..15:29 IST; the last bar opens at 15:29. IST is UTC+5:30, so
        // 15:29 IST is 09:59 UTC.
        let days = i64::from(d.days_from_epoch());
        (days * 86_400 + 9 * 3_600 + 59 * 60) * 1_000_000
    }

    fn holdings(entries: &[(Series, YearMonth, Day)]) -> HashMap<EntryKey, i64> {
        entries
            .iter()
            .map(|(s, m, d)| (s.at(*m), close_of(*d)))
            .collect()
    }

    // ---------------------------------------------------------------- oldest

    /// **The oldest missing month is the one chosen, never a newer one.**
    ///
    /// Reverting `frontier`'s upward scan to pick the newest — or `round`'s
    /// `ordinal(unit.month) < ordinal(held.month)` to `>` — makes this fail:
    /// it asserts the month, not merely that some month was picked.
    ///
    /// This is the rule the append-only store makes non-negotiable: the file
    /// holding 2026-08-06 refuses 08-03..=08-07 outright, so a newer month
    /// fetched first blocks the earlier days permanently.
    #[test]
    fn the_oldest_missing_month_is_chosen_and_never_a_newer_one() {
        let axis = [series("NIFTY"), series("BANKNIFTY")];
        // 2020-01 and 2020-02 complete; 2020-03 missing; 2020-04 missing too.
        let held = holdings(&[
            (axis[0], month(2020, 1), day(2020, 1, 31)),
            (axis[1], month(2020, 1), day(2020, 1, 31)),
            (axis[0], month(2020, 2), day(2020, 2, 29)),
            (axis[1], month(2020, 2), day(2020, 2, 29)),
        ]);
        let floor = day(2020, 1, 1);
        let yesterday = day(2020, 5, 20);
        let (at, unit) = frontier(
            |k| held.get(k).copied(),
            &axis,
            month(2020, 1),
            floor,
            yesterday,
        );
        assert_eq!(
            at,
            month(2020, 3),
            "the first incomplete month, not the last"
        );
        let unit = unit.expect("2020-03 is missing entirely");
        assert_eq!(unit.month, month(2020, 3));
        assert_eq!(unit.window.from(), day(2020, 3, 1));
        assert_eq!(unit.window.to(), day(2020, 3, 31));
        assert_eq!(unit.behind, 2);
        assert_eq!(unit.done, 0);
    }

    /// A complete store owes nothing, contacts nothing, and says which month
    /// it reached.
    #[test]
    fn a_complete_store_asks_for_nothing() {
        let axis = [series("NIFTY")];
        let held = holdings(&[
            (axis[0], month(2020, 1), day(2020, 1, 31)),
            (axis[0], month(2020, 2), day(2020, 2, 20)),
        ]);
        let (_, unit) = frontier(
            |k| held.get(k).copied(),
            &axis,
            month(2020, 1),
            day(2020, 1, 1),
            day(2020, 2, 20),
        );
        assert_eq!(unit, None, "nothing is missing, so nothing is asked for");
    }

    // ------------------------------------------------------- never today etc.

    /// **Never today, and never below the feed's history floor.**
    ///
    /// Reverting `month_span`'s `to` clamp to the month's own last day makes
    /// the first assertion fail; reverting the `from` clamp to the month's
    /// first day makes the second fail.
    #[test]
    fn the_span_never_reaches_today_and_never_precedes_the_floor() {
        let floor = day(2020, 1, 1);
        // Mid-month: the span stops at yesterday, not at the 31st.
        let yesterday = day(2026, 8, 6);
        let (from, to) = month_span(month(2026, 8), floor, yesterday).unwrap();
        assert_eq!(from, day(2026, 8, 1));
        assert_eq!(to, yesterday, "today is never asked for");

        // The floor's own month: the span starts at the floor, not the 1st.
        let floor = day(2020, 1, 17);
        let (from, to) = month_span(month(2020, 1), floor, day(2020, 3, 1)).unwrap();
        assert_eq!(from, floor, "the vendor has nothing before its floor");
        assert_eq!(to, day(2020, 1, 31));

        // Entirely below the floor, and entirely in the future: neither is due.
        assert_eq!(month_span(month(2019, 12), floor, day(2020, 3, 1)), None);
        assert_eq!(month_span(month(2020, 4), floor, day(2020, 3, 1)), None);
    }

    /// A feed's floor comes from its own descriptor, and a rolling one is
    /// derived from the clock rather than stored.
    ///
    /// Reverting `floor_day` to a literal — or `clamp_to_floor`'s `Rolling` arm
    /// to a constant — makes the Dhan assertion fail, because the expected
    /// value here is recomputed from the descriptor's own `years` against the
    /// clock at the moment the test runs.
    ///
    /// # The limit, stated rather than implied
    ///
    /// That a rolling floor *moves between two days* is **not** asserted, and
    /// the earlier version of this test that claimed to was wrong: `floor_day`
    /// takes `yesterday` to bound the window it clamps, but `clamp_to_floor`
    /// reads `SystemTime::now()` for the rolling arm, so passing two different
    /// days returns the same floor. Proving movement needs an injectable clock,
    /// which this build does not have. `CLAUDE.md` §3 rule 6 — never claim a
    /// measurement not taken. What IS proven is that the value tracks the clock
    /// rather than a stored constant, which is the bug `HistoryFloor::Rolling`
    /// exists to name.
    #[test]
    fn the_floor_comes_from_the_descriptor_and_not_from_a_literal() {
        let yesterday = day(2026, 8, 6);
        let groww = floor_day(pull::vendor::Feed::Groww, yesterday).unwrap();
        assert_eq!(groww, day(2020, 1, 1), "the charter's fixed floor");

        // DHAN'S IS ROLLING. The expected value is rebuilt from the descriptor
        // and the clock, so this test restates no vendor fact of its own.
        let pull::vendor::Transport::Http(spec) = pull::vendor::Feed::Dhan.descriptor().transport
        else {
            panic!("Dhan is an HTTP feed");
        };
        let pull::vendor::HistoryFloor::Rolling { years } = spec.history_floor else {
            panic!("Dhan's floor is the rolling one");
        };
        let today = ingest::ist_day(std::time::SystemTime::now()).unwrap();
        let back = u32::try_from(u64::from(years) * 36_525 / 100).unwrap();
        let expected = Day::from_days(today.days_from_epoch() - back).unwrap();
        let dhan = floor_day(pull::vendor::Feed::Dhan, yesterday).unwrap();
        assert_eq!(
            dhan, expected,
            "a rolling floor is recomputed from the clock, never stored"
        );
        assert_ne!(dhan, groww, "the two feeds do not share one floor");

        // An archive feed has no HTTP transport and is therefore not drivable.
        assert_eq!(floor_day(pull::vendor::Feed::Gdfl, yesterday), None);
        assert!(
            drivable(yesterday).iter().all(|f| matches!(
                f.feed.descriptor().transport,
                pull::vendor::Transport::Http(_)
            )),
            "only HTTP feeds are driven"
        );
    }

    // ------------------------------------------------------------- resume

    /// **Restart resumes; it does not start over.**
    ///
    /// A file holding one interior day is the live shape on the operator's
    /// disk — `2026-08.bin` holds 375 bars, all of 2026-08-06 — and offering
    /// 08-01..=08-07 against it is refused wholesale by `BarFile::append`,
    /// including the 08-07 that strictly follows. The window must therefore
    /// begin the day AFTER what is held.
    ///
    /// Reverting `next_window`'s `day.succ()` to `from` makes this fail.
    #[test]
    fn a_partly_held_month_resumes_the_day_after_what_is_held() {
        let axis = [series("NIFTY")];
        let held = holdings(&[(axis[0], month(2026, 8), day(2026, 8, 6))]);
        let span = month_span(month(2026, 8), day(2020, 1, 1), day(2026, 8, 7)).unwrap();
        let unit = next_window(|k| held.get(k).copied(), &axis, month(2026, 8), span).unwrap();
        assert_eq!(
            unit.window.from(),
            day(2026, 8, 7),
            "the day after the last held one, never the month's first"
        );
        assert_eq!(unit.window.to(), day(2026, 8, 7));

        // A FRESH PROCESS — no cursor, no state — lands on the same answer,
        // because the answer is the store's.
        let (at, again) = frontier(
            |k| held.get(k).copied(),
            &axis,
            month(2020, 1),
            day(2020, 1, 1),
            day(2026, 8, 7),
        );
        assert_eq!(at, month(2020, 1), "2020-01 is still missing entirely");
        assert_eq!(again.unwrap().window.from(), day(2020, 1, 1));
    }

    /// The window starts at the EARLIEST resume point across the universe, so
    /// an instrument that fell behind is not skipped past.
    #[test]
    fn the_window_starts_at_the_earliest_resume_point_not_the_latest() {
        let axis = [series("NIFTY"), series("BANKNIFTY")];
        let held = holdings(&[
            (axis[0], month(2020, 5), day(2020, 5, 20)),
            (axis[1], month(2020, 5), day(2020, 5, 8)),
        ]);
        let span = month_span(month(2020, 5), day(2020, 1, 1), day(2020, 6, 30)).unwrap();
        let unit = next_window(|k| held.get(k).copied(), &axis, month(2020, 5), span).unwrap();
        assert_eq!(
            unit.window.from(),
            day(2020, 5, 9),
            "the laggard's resume point, not the leader's"
        );
        assert_eq!(unit.behind, 2);
    }

    // ------------------------------------------------------------ failure

    /// **A retryable failure retries with backoff; a credential failure
    /// halts and names it.**
    ///
    /// Reverting the `Trouble::Credential` arm in `observe` to fall through to
    /// the transport arm makes the halt assertions fail.
    #[test]
    fn a_transport_failure_backs_off_and_a_credential_failure_halts() {
        let mut state = FeedState::new(
            pull::vendor::Feed::Groww,
            brutex_core::vendor::Vendor::Groww,
            month(2020, 1),
        );
        let failed = TickOutcome {
            attempted: 773,
            reached: 0,
            stored: 0,
            reason: Some("connection reset by peer".to_owned()),
            complete: false,
            stopped: false,
        };
        assert_eq!(
            state.observe(&failed),
            Next::Wait {
                secs: BACKOFF_FLOOR_SECS
            }
        );
        assert_eq!(
            state.observe(&failed),
            Next::Wait {
                secs: BACKOFF_FLOOR_SECS * 2
            },
            "the second wait is longer than the first"
        );

        // The credential. §8: one automatic re-read, then halt — never mint.
        let mut state = FeedState::new(
            pull::vendor::Feed::Groww,
            brutex_core::vendor::Vendor::Groww,
            month(2020, 1),
        );
        let dead = TickOutcome {
            reason: Some(
                "RELIANCE: the broker credential could not be read: AccessDenied".to_owned(),
            ),
            ..failed.clone()
        };
        assert_eq!(
            state.observe(&dead),
            Next::Retry,
            "the token is re-read once, automatically"
        );
        let Next::Halt { reason } = state.observe(&dead) else {
            panic!("the second dead credential must halt");
        };
        assert!(
            reason.contains("never mints"),
            "the halt names §8 rather than trying again: {reason}"
        );
        assert!(
            reason.contains("AccessDenied"),
            "the vendor's own words survive onto the page: {reason}"
        );
        assert!(state.halted.is_some(), "halted is terminal for this feed");
    }

    /// Every reason this build produces lands in the right bucket, and a
    /// dead token is not read as a full disk because it says "Parameter
    /// Store".
    #[test]
    fn every_reason_this_build_produces_is_classified() {
        assert_eq!(
            classify("the broker credential could not be read: x"),
            Trouble::Credential
        );
        assert_eq!(classify("no AWS identity: x"), Trouble::Credential);
        assert_eq!(
            classify("the parameter path could not be built: x"),
            Trouble::Credential
        );
        assert_eq!(
            classify(
                "refused with status 401 — the access token expired mid-run. This \
                 repository never mints one (§8): the refreshed value is read from \
                 Parameter Store on the next pull"
            ),
            Trouble::Credential,
            "the word Store in Parameter Store must not read as a disk failure"
        );
        assert_eq!(
            classify("disk full writing /x/y/2020-01.bin"),
            Trouble::Store
        );
        assert_eq!(classify("permission denied opening /x/y"), Trouble::Store);
        assert_eq!(classify("refused with status 500"), Trouble::Transport);
        assert_eq!(classify("something nobody has seen"), Trouble::Transport);
    }

    /// **The same unit failing repeatedly stops after the bound, and the stop
    /// is reported rather than silent.**
    ///
    /// Reverting the `self.attempts >= MAX_MONTH_ATTEMPTS` arm to fall through
    /// to `Next::Wait` makes this loop forever and fail.
    #[test]
    fn a_month_that_keeps_failing_is_stalled_after_the_bound_and_says_so() {
        let mut state = FeedState::new(
            pull::vendor::Feed::Groww,
            brutex_core::vendor::Vendor::Groww,
            month(2020, 5),
        );
        let failed = TickOutcome {
            attempted: 773,
            reached: 0,
            stored: 0,
            reason: Some("the transport is not blipping, it is down".to_owned()),
            complete: false,
            stopped: false,
        };
        let mut verdicts = Vec::new();
        for _ in 0..MAX_MONTH_ATTEMPTS {
            verdicts.push(state.observe(&failed));
        }
        let Some(Next::Stall { reason }) = verdicts.last() else {
            panic!("attempt {MAX_MONTH_ATTEMPTS} must stall, got {verdicts:?}");
        };
        assert!(reason.contains("it is down"), "the reason is verbatim");
        assert_eq!(state.stalls.len(), 1, "the stall is recorded, not dropped");
        assert_eq!(state.stalls[0].month, month(2020, 5));
        assert_eq!(state.stalls[0].attempts, MAX_MONTH_ATTEMPTS);
        // AND IT IS VISIBLE. A stall nobody can see is the silent skip
        // CLAUDE.md §4 bans.
        let status = Status {
            feeds: vec![FeedReport {
                stalls: state.stalls.clone(),
                ..FeedReport::default()
            }],
            ..Status::default()
        };
        assert!(
            status.json().contains("it is down"),
            "the stall reaches the JSON: {}",
            status.json()
        );
    }

    // --------------------------------------------------------------- survey

    /// A census for a vendor, in whichever of the three states.
    fn census_of(vendor: brutex_core::vendor::Vendor, state: Census) -> VendorCensus {
        VendorCensus {
            vendor,
            path: std::path::PathBuf::from("/nowhere/manifest.man"),
            state,
        }
    }

    /// An empty site on a store root of its own.
    ///
    /// **`name` must differ per test.** `scratch::path` stamps the process id,
    /// not the test, so two tests sharing a name share a store — and one that
    /// asserts "no journal was written" then reads the journal a previous test
    /// wrote. That is exactly how this helper was wrong the first time.
    fn empty_site(name: &str) -> Loaded {
        let dir = crate::scratch::path(&format!("autopilot-{name}"));
        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::create_dir_all(&dir);
        Loaded::new(Site::load(&dir, &dir))
    }

    /// **Oldest first ACROSS feeds, not just within one.**
    ///
    /// Groww's floor is fixed at 2020-01-01 and Dhan's rolls ~5 years back, so
    /// Groww owes the older month and must be the one chosen. Reverting
    /// `survey`'s `ordinal(unit.month) < ordinal(held.month)` to `>` picks Dhan
    /// and fails this.
    ///
    /// It matters for the same reason the within-feed rule does: the ladder is
    /// climbed globally, so an interruption leaves every feed uniformly deep
    /// rather than one complete and one empty.
    #[test]
    fn survey_chooses_the_oldest_month_any_feed_owes_across_all_of_them() {
        let yesterday = day(2026, 8, 6);
        let axis = [series("NIFTY")];
        let mut feeds = drivable(yesterday);
        assert!(
            feeds.len() >= 2,
            "this test needs two HTTP feeds to choose between"
        );
        // Absent censuses: nothing is held, so every feed owes its whole span.
        let censuses: Vec<VendorCensus> = feeds
            .iter()
            .map(|f| census_of(f.vendor, Census::Absent))
            .collect();

        let (reports, chosen) = survey(&mut feeds, &axis, &censuses, yesterday);
        let (slot, unit) = chosen.expect("an empty store owes every month");
        let picked = feeds.get(slot).expect("the slot survey returned");
        let oldest = feeds
            .iter()
            .filter_map(|f| floor_day(f.feed, yesterday))
            .min()
            .expect("at least one floor");
        assert_eq!(
            unit.month,
            oldest.year_month().expect("a real month"),
            "the month chosen is the oldest any feed can be asked for"
        );
        assert_eq!(
            floor_day(picked.feed, yesterday),
            Some(oldest),
            "and it is the feed whose floor reaches that month"
        );
        assert_eq!(
            reports.len(),
            feeds.len(),
            "every feed is reported, chosen or not"
        );
    }

    /// **An unreadable census halts that feed by name — it is never read as an
    /// empty one.**
    ///
    /// Treating it as empty would re-fetch a store that is already full;
    /// treating it as full would hide a real gap. Reverting the `Unreadable`
    /// arm in `survey` to fall through makes the feed look empty and fails
    /// this.
    #[test]
    fn a_census_that_will_not_load_halts_its_feed_and_names_why() {
        let yesterday = day(2026, 8, 6);
        let axis = [series("NIFTY")];
        let mut feeds = drivable(yesterday);
        let censuses: Vec<VendorCensus> = feeds
            .iter()
            .map(|f| {
                census_of(
                    f.vendor,
                    Census::Unreadable {
                        reason: String::from("entry 41 fails its own checksum"),
                    },
                )
            })
            .collect();

        let (reports, chosen) = survey(&mut feeds, &axis, &censuses, yesterday);
        assert!(
            chosen.is_none(),
            "nothing is fetched against a guess about what the store holds"
        );
        assert!(
            feeds.iter().all(|f| f.halted.is_some()),
            "every feed with an unreadable census is halted, not skipped"
        );
        for report in &reports {
            assert!(
                report.halted.contains("entry 41 fails its own checksum"),
                "the refusal's own words survive onto the page: {}",
                report.halted
            );
        }
    }

    // ------------------------------------------------------------- handlers

    /// The three routes an operator drives: read, stop, start.
    ///
    /// Pause must be visible in the very payload the pause request answers with
    /// — an operator who clicks Pause and is handed a payload still saying
    /// `running` has been told a lie by the control itself.
    #[tokio::test]
    async fn the_routes_report_stop_and_start() {
        let site = empty_site("routes");
        let (headers, body) = status_json(axum::extract::State(Loaded::clone(&site))).await;
        assert_eq!(headers[0].1, "application/json; charset=utf-8");
        assert!(body.contains(r#""state":"starting""#), "{body}");

        let (_, paused) = pause(axum::extract::State(Loaded::clone(&site))).await;
        assert!(site.autopilot.is_paused(), "the flag is actually set");
        assert!(
            paused.contains(r#""state":"paused""#),
            "the answer to Pause says paused: {paused}"
        );
        assert!(
            paused.contains("refilled automatically on resume"),
            "and it says what happens to the partial month: {paused}"
        );

        let (_, resumed) = resume(axum::extract::State(Loaded::clone(&site))).await;
        assert!(!site.autopilot.is_paused(), "resume clears the flag");
        assert!(resumed.contains("oldest"), "{resumed}");
    }

    /// `settle` writes the decision where an operator can read it: a backoff
    /// names its reason and its countdown, a stall names the bound it hit, and
    /// a halt is terminal.
    #[tokio::test]
    async fn settle_publishes_every_verdict_where_it_can_be_read() {
        let site = empty_site("settle");
        let mut state = FeedState::new(
            pull::vendor::Feed::Groww,
            brutex_core::vendor::Vendor::Groww,
            month(2020, 5),
        );
        let failed = TickOutcome {
            attempted: 785,
            reached: 0,
            stored: 0,
            reason: Some(String::from("connection reset by peer")),
            complete: false,
            stopped: false,
        };

        // BACKOFF: waits, and says how long and why.
        let waited = settle(&site, &mut state, &failed);
        assert_eq!(waited, BACKOFF_FLOOR_SECS);
        let json = site.autopilot.json();
        assert!(json.contains(r#""state":"waiting""#), "{json}");
        assert!(json.contains("connection reset by peer"), "{json}");

        // STALL: passes the month, and the bound is on the page.
        let mut stalling = FeedState {
            attempts: MAX_MONTH_ATTEMPTS.saturating_sub(1),
            ..FeedState::new(
                pull::vendor::Feed::Groww,
                brutex_core::vendor::Vendor::Groww,
                month(2020, 5),
            )
        };
        assert_eq!(settle(&site, &mut stalling, &failed), 0);
        assert_eq!(
            stalling.frontier,
            month(2020, 6),
            "it moved past the bad month"
        );
        assert!(site.autopilot.json().contains("was passed over after"));

        // ADVANCE: a completed month moves the frontier on.
        let mut done = FeedState::new(
            pull::vendor::Feed::Groww,
            brutex_core::vendor::Vendor::Groww,
            month(2020, 5),
        );
        let complete = TickOutcome {
            reached: 785,
            stored: 3_000_000,
            reason: None,
            complete: true,
            ..failed.clone()
        };
        assert_eq!(settle(&site, &mut done, &complete), 0);
        assert_eq!(done.frontier, month(2020, 6));

        // HALT: terminal, and the reason is the whole detail.
        let mut dead = FeedState::new(
            pull::vendor::Feed::Groww,
            brutex_core::vendor::Vendor::Groww,
            month(2020, 5),
        );
        let credential = TickOutcome {
            reason: Some(String::from(
                "the broker credential could not be read: AccessDenied",
            )),
            ..failed.clone()
        };
        assert_eq!(
            settle(&site, &mut dead, &credential),
            0,
            "the re-read is immediate"
        );
        assert_eq!(settle(&site, &mut dead, &credential), IDLE_POLL_SECS);
        let json = site.autopilot.json();
        assert!(json.contains(r#""state":"halted""#), "{json}");
        assert!(json.contains("never mints"), "{json}");
    }

    /// **A whole round runs end to end, and a refused broker degrades loudly.**
    ///
    /// Safe to drive because `broker_run` checks `Broker::Refused` before it
    /// opens anything — the guard that keeps this suite off the vendor is the
    /// same one that makes the loop testable. So this exercises the real path:
    /// take the seat, read the census, survey the feeds, choose the oldest
    /// month, run the tick, journal it, re-probe the store, and settle.
    ///
    /// It asserts the *reason survives* to the page. A backfill that stops
    /// against a refused broker and does not say so is precisely the silent
    /// failure `CLAUDE.md` §4 bans.
    #[tokio::test]
    async fn a_whole_round_runs_and_a_refused_broker_is_named_on_the_page() {
        let site = empty_site("refused-broker");
        let yesterday = yesterday_ist(std::time::SystemTime::now()).expect("a usable clock");
        let axis = [series("NIFTY")];
        let mut feeds = drivable(yesterday);

        let waited = round(&site, &mut feeds, &axis, pull::vendor::Granularity::Minute1).await;

        // A transport-shaped refusal, so it backs off rather than halting.
        assert_eq!(waited, BACKOFF_FLOOR_SECS);
        let json = site.autopilot.json();
        assert!(json.contains(r#""state":"waiting""#), "{json}");
        assert!(
            json.contains("may not reach a live broker"),
            "the refusal reaches the page verbatim: {json}"
        );
        // AND IT PICKED THE OLDEST MONTH, from a store holding nothing.
        assert!(json.contains(r#""cursor":"2020-01""#), "{json}");
        // THE TICK WAS JOURNALLED. One record per tick, in the store's own
        // journal, so a restart can read what this process did.
        assert!(
            site.journal().path.exists(),
            "the run journal was written at {}",
            site.journal().path.display()
        );
    }

    /// **A hand-made pull holds the seat, and the backfill stands off rather
    /// than spinning.**
    ///
    /// `pull::ingest`'s census lock refuses rather than queues, so an autopilot
    /// tick landing inside a manual pull would see every instrument fail with a
    /// lock message and call that a run. One seat turns that into one honest
    /// wait.
    ///
    /// The returned wait is the load-bearing assertion. Reverting it to `0`
    /// makes `fly` re-enter `round` immediately, and `round` re-reads the whole
    /// census — ~16 MB, CRC-verified — before discovering the seat is still
    /// taken. Measured on the paused loop, the same shape cost 99% of a core.
    #[tokio::test]
    async fn a_manual_pull_holding_the_seat_makes_the_backfill_stand_off() {
        let site = empty_site("seat");
        let held = site.autopilot.take_seat().expect("the seat starts free");
        let yesterday = yesterday_ist(std::time::SystemTime::now()).expect("a usable clock");
        let axis = [series("NIFTY")];
        let mut feeds = drivable(yesterday);

        let waited = round(&site, &mut feeds, &axis, pull::vendor::Granularity::Minute1).await;
        assert_eq!(waited, SEAT_WAIT_SECS, "it waits rather than spinning");
        let json = site.autopilot.json();
        assert!(json.contains("holds the pull seat"), "{json}");
        assert!(
            json.contains("Nothing is lost"),
            "and it says the standoff costs nothing: {json}"
        );
        // NOTHING WAS FETCHED AND NOTHING WAS JOURNALLED — the tick never ran.
        assert!(!site.journal().path.exists(), "no run happened");

        // AND THE SEAT COMES BACK. The next round proceeds normally.
        drop(held);
        let after = round(&site, &mut feeds, &axis, pull::vendor::Granularity::Minute1).await;
        assert_ne!(
            after, SEAT_WAIT_SECS,
            "the standoff ended with the manual pull"
        );
    }

    /// **Pausing during the grace window stops it before any socket opens.**
    ///
    /// That is what the countdown is for: the operator's one chance to say no.
    /// Both `grace` and `nap` return the moment they see the flag, which is
    /// also why neither may be used as the paused loop's own sleep — see
    /// [`dwell_paused`].
    #[tokio::test]
    async fn pausing_during_the_grace_window_returns_at_once() {
        let site = empty_site("grace");
        site.autopilot.pause();
        let started = std::time::Instant::now();
        grace(&site).await;
        nap(&site, IDLE_POLL_SECS).await;
        let elapsed = started.elapsed();
        assert!(
            elapsed < std::time::Duration::from_secs(2),
            "a paused countdown waited {elapsed:?} instead of returning — the \
             operator's Pause has to bite before the first fetch, not after it"
        );
    }

    /// A site that may not reach a broker publishes why and starts nothing.
    ///
    /// This is the guard that keeps the whole test suite off the vendor:
    /// `Site::serving` is the only constructor that sets `Broker::Live`, and
    /// only `run_in` calls it.
    #[tokio::test]
    async fn a_site_that_may_not_reach_a_broker_starts_no_backfill() {
        let site = empty_site("no-broker");
        assert_eq!(site.broker, Broker::Refused, "the default, and the guard");
        // Returns rather than looping, so awaiting it cannot hang.
        fly(Loaded::clone(&site)).await;
        let json = site.autopilot.json();
        assert!(json.contains(r#""state":"halted""#), "{json}");
        assert!(json.contains("Broker::Live"), "it names the guard: {json}");
    }

    /// The same store refusal twice is not retried — a full disk is not a
    /// blip.
    #[test]
    fn the_same_store_refusal_twice_halts_rather_than_hammering_the_disk() {
        let mut state = FeedState::new(
            pull::vendor::Feed::Groww,
            brutex_core::vendor::Vendor::Groww,
            month(2020, 5),
        );
        let full = TickOutcome {
            attempted: 773,
            reached: 773,
            stored: 0,
            reason: Some("NIFTY — disk full writing /store/x.bin".to_owned()),
            complete: false,
            stopped: false,
        };
        assert!(matches!(state.observe(&full), Next::Wait { .. }));
        let Next::Halt { reason } = state.observe(&full) else {
            panic!("a repeated disk failure must halt");
        };
        assert!(reason.contains("disk full"));
    }

    /// Progress resets the bound, so a month that needs several passes is not
    /// stalled for being large — and it advances when the store says it is
    /// done.
    #[test]
    fn progress_resets_the_bound_and_completion_advances() {
        let mut state = FeedState::new(
            pull::vendor::Feed::Groww,
            brutex_core::vendor::Vendor::Groww,
            month(2020, 5),
        );
        let failed = TickOutcome {
            attempted: 773,
            reached: 0,
            stored: 0,
            reason: Some("timed out".to_owned()),
            complete: false,
            stopped: false,
        };
        assert!(matches!(state.observe(&failed), Next::Wait { .. }));
        assert_eq!(state.attempts, 1);
        let progressed = TickOutcome {
            reached: 773,
            stored: 2_900_000,
            reason: None,
            ..failed.clone()
        };
        assert_eq!(state.observe(&progressed), Next::Retry);
        assert_eq!(state.attempts, 0, "progress clears the attempt count");
        let done = TickOutcome {
            complete: true,
            ..progressed
        };
        assert_eq!(state.observe(&done), Next::Advance);
        assert_eq!(state.months_done, 1);
    }

    /// A month with no data anywhere is retired after two dry rounds, not
    /// after one — `docs/07-plan.md` §9.2. One dry round may be a vendor that
    /// was down.
    #[test]
    fn a_dry_month_is_retired_only_after_two_dry_rounds() {
        let mut state = FeedState::new(
            pull::vendor::Feed::Groww,
            brutex_core::vendor::Vendor::Groww,
            month(2020, 5),
        );
        let dry = TickOutcome {
            attempted: 773,
            reached: 773,
            stored: 0,
            reason: None,
            complete: false,
            stopped: false,
        };
        assert_eq!(
            state.observe(&dry),
            Next::Retry,
            "one dry round is not proof"
        );
        assert_eq!(state.observe(&dry), Next::Advance);
        assert_eq!(state.months_done, 1);
    }

    /// A pause costs no attempt: stopping is the operator's decision, not a
    /// failure of the month.
    #[test]
    fn being_stopped_by_the_operator_costs_no_attempt() {
        let mut state = FeedState::new(
            pull::vendor::Feed::Groww,
            brutex_core::vendor::Vendor::Groww,
            month(2020, 5),
        );
        let stopped = TickOutcome {
            attempted: 773,
            reached: 20,
            stored: 0,
            reason: None,
            complete: false,
            stopped: true,
        };
        assert_eq!(state.observe(&stopped), Next::Retry);
        assert_eq!(state.attempts, 0);
        assert_eq!(state.dry, 0, "a stop is not a dry round either");
    }

    // ------------------------------------------------------------- status

    /// **The status reflects the store, not a clock.**
    ///
    /// The same instant with two different stores gives two different answers,
    /// and the same store at two different instants gives the same one.
    /// Reverting `frontier` to count months rather than probe them makes this
    /// fail.
    #[test]
    fn the_status_is_derived_from_the_store_and_not_from_a_timer() {
        let axis = [series("NIFTY")];
        let floor = day(2020, 1, 1);
        let yesterday = day(2020, 4, 30);

        let empty: HashMap<EntryKey, i64> = HashMap::new();
        let (_, cold) = frontier(
            |k| empty.get(k).copied(),
            &axis,
            month(2020, 1),
            floor,
            yesterday,
        );
        let cold = cold.expect("an empty store owes every month");

        let filled = holdings(&[
            (axis[0], month(2020, 1), day(2020, 1, 31)),
            (axis[0], month(2020, 2), day(2020, 2, 29)),
        ]);
        let (at, warm) = frontier(
            |k| filled.get(k).copied(),
            &axis,
            month(2020, 1),
            floor,
            yesterday,
        );
        let warm = warm.expect("2020-03 is still owed");

        assert_eq!(cold.month, month(2020, 1));
        assert_eq!(warm.month, month(2020, 3));
        assert_ne!(
            cold, warm,
            "two stores, one clock: the answer follows the store"
        );
        assert_eq!(at, month(2020, 3));

        // AND THE JSON CARRIES IT. Not a spinner, not an elapsed time: the
        // month, the window, and how many series are still short.
        let status = Status {
            phase: Phase::Running,
            detail: "fetching".to_owned(),
            feeds: vec![FeedReport {
                feed: "groww".to_owned(),
                month: warm.month.to_string(),
                window: format!("{}..={}", warm.window.from(), warm.window.to()),
                behind: warm.behind,
                months_done: 2,
                months_total: months_owed(floor, yesterday),
                store_months: 2,
                store_rows: 6_000_000,
                ..FeedReport::default()
            }],
            ..Status::default()
        };
        let json = status.json();
        // `running` with nothing in flight is published as `waiting`, which is
        // the contract's own rule and the honest word for it.
        assert!(json.contains(r#""state":"waiting""#), "{json}");
        assert!(json.contains(r#""month":"2020-03""#), "{json}");
        assert!(json.contains(r#""behind":1"#), "{json}");
        assert!(json.contains(r#""months_done":2"#), "{json}");
        assert!(json.contains(r#""months_total":4"#), "{json}");
        assert!(json.contains(r#""store_rows":6000000"#), "{json}");
        assert!(
            json.contains(&format!(r#""attempts_max":{MAX_MONTH_ATTEMPTS}"#)),
            "the bound is on the page rather than in the source only: {json}"
        );
    }

    /// Months owed is arithmetic on ordinals, inclusive at both ends.
    #[test]
    fn months_owed_counts_both_ends() {
        assert_eq!(months_owed(day(2020, 1, 1), day(2020, 1, 31)), 1);
        assert_eq!(months_owed(day(2020, 1, 1), day(2020, 12, 31)), 12);
        assert_eq!(months_owed(day(2020, 1, 1), day(2026, 8, 6)), 80);
    }

    /// Month arithmetic is an ordinal, not a calendar walk, and it round-trips.
    #[test]
    fn month_arithmetic_is_an_ordinal_and_round_trips() {
        assert_eq!(month_after(month(2020, 12)), Some(month(2021, 1)));
        assert_eq!(month_after(month(2020, 1)), Some(month(2020, 2)));
        assert_eq!(month_after(month(9999, 12)), None);
        for m in [month(1970, 1), month(2020, 5), month(9999, 12)] {
            assert_eq!(from_ordinal(ordinal(m)), Some(m));
        }
    }

    /// The backoff doubles and then stops doubling.
    #[test]
    fn the_backoff_doubles_and_is_capped() {
        assert_eq!(FeedState::backoff_secs(0), BACKOFF_FLOOR_SECS);
        assert_eq!(FeedState::backoff_secs(1), BACKOFF_FLOOR_SECS * 2);
        assert_eq!(FeedState::backoff_secs(2), BACKOFF_FLOOR_SECS * 4);
        assert_eq!(FeedState::backoff_secs(9), BACKOFF_CEILING_SECS);
        assert_eq!(FeedState::backoff_secs(u32::MAX), BACKOFF_CEILING_SECS);
        for steps in 0..64 {
            assert!(FeedState::backoff_secs(steps) <= BACKOFF_CEILING_SECS);
        }
    }

    // -------------------------------------------------------------- control

    /// Pause bites through the epoch, and a run that starts AFTER a pause is
    /// not cancelled by it — which is the race a plain flag has.
    #[test]
    fn the_stop_epoch_only_cancels_runs_that_were_already_in_flight() {
        let control = Control::new();
        assert!(!control.is_paused());
        let in_flight = control.epoch();
        control.pause();
        assert!(control.is_paused());
        assert!(control.stopped(in_flight), "a run in flight is stopped");
        let started_later = control.epoch();
        assert!(
            !control.stopped(started_later),
            "a run that began after the pause is not cancelled by it"
        );
        control.resume();
        assert!(!control.is_paused());
        assert!(
            !control.stopped(started_later),
            "resuming does not cancel anything"
        );
    }

    /// One seat, and it comes back when it is dropped.
    #[test]
    fn the_pull_seat_admits_one_holder_at_a_time() {
        let control = Control::new();
        {
            let held = control.take_seat().expect("the seat is free");
            assert!(
                control.take_seat().is_none(),
                "a second holder is refused rather than queued"
            );
            drop(held);
        }
        assert!(
            control.take_seat().is_some(),
            "the seat is released on drop, including on an early return"
        );
    }

    /// **A paused autopilot sleeps. It does not spin.**
    ///
    /// The paused arm of `fly` loops on the flag, so whatever it awaits has to
    /// actually take time. Reverting `dwell_paused`'s `tokio::time::sleep` to a
    /// `nap`-shaped call — anything that returns early *because* the flag is
    /// set — drops the elapsed time to microseconds and fails this.
    ///
    /// The measurement behind it: with the early-returning version, the built
    /// binary sat at 99% of a core the moment Pause was pressed, and the tight
    /// `publish` in the loop starved the status mutex until
    /// `GET /autopilot.json` timed out after five seconds. The bound below is
    /// deliberately loose — half the interval — so this asserts "it really
    /// waited" without becoming a flaky clock test on a loaded machine.
    #[tokio::test]
    async fn a_paused_autopilot_sleeps_rather_than_spinning_the_status_lock() {
        let control = Control::new();
        control.pause();
        let started = std::time::Instant::now();
        dwell_paused(&control).await;
        let elapsed = started.elapsed();
        assert!(
            elapsed >= std::time::Duration::from_millis(PAUSED_POLL_SECS * 500),
            "a paused dwell returned in {elapsed:?}, which is a spin rather than a wait"
        );
        // AND IT STILL SAYS WHAT IT IS DOING. A wait that went quiet would be
        // the other half of the same failure.
        let json = control.json();
        assert!(json.contains(r#""state":"paused""#), "{json}");
        assert!(json.contains("Press Resume"), "{json}");
    }

    /// The status starts by saying it has not started, and publishing changes
    /// what is read back.
    #[test]
    fn the_control_publishes_what_is_read_back() {
        let control = Control::new();
        assert!(control.json().contains(r#""state":"starting""#));
        control.publish(|status| {
            status.phase = Phase::Idle;
            status.detail = String::from("nothing is missing");
        });
        let json = control.json();
        assert!(json.contains(r#""state":"complete""#), "{json}");
        assert!(json.contains("nothing is missing"), "{json}");
    }

    /// Every phase has a word, no two share one, and every word is one the
    /// contract defines.
    ///
    /// The second half is the load-bearing one:
    /// `web/src/routes/autopilot/+page.svelte` rejects the whole payload when
    /// `state` is outside its list, so inventing a seventh word here would take
    /// the page down rather than show it. D-0057.
    #[test]
    fn every_phase_has_its_own_word_and_the_contract_defines_it() {
        // The exact list the page validates against.
        const CONTRACT: [&str; 6] = [
            "starting", "running", "waiting", "paused", "halted", "complete",
        ];
        let all = [
            Phase::Starting,
            Phase::Running,
            Phase::Backoff,
            Phase::Paused,
            Phase::Halted,
            Phase::Idle,
        ];
        let mut seen: Vec<&str> = all.iter().map(|p| p.state()).collect();
        for word in &seen {
            assert!(
                CONTRACT.contains(word),
                "`{word}` is not one of {CONTRACT:?} — the page refuses the payload"
            );
        }
        seen.sort_unstable();
        let before = seen.len();
        seen.dedup();
        assert_eq!(before, seen.len(), "two phases share a word");
    }

    /// **The payload satisfies every rule the page enforces.**
    ///
    /// This is the contract test: `state` in the set, `why` non-empty wherever
    /// the state demands it, `target` with its three required fields, `now`
    /// null-or-complete, and `failures` present even when empty. Dropping any
    /// one of them from [`Status::json`] fails here rather than at the browser,
    /// which is the whole point of having it.
    #[test]
    fn the_payload_satisfies_the_contract_the_page_enforces() {
        // `failures` IS EMITTED EVEN WHEN EMPTY. The page refuses an absent
        // list, because a missing list and an empty list are different facts
        // and only the second is good news.
        let bare = Status::default().json();
        assert!(bare.contains(r#""failures":[]"#), "{bare}");
        assert!(bare.contains(r#""now":null"#), "{bare}");
        assert!(bare.contains(r#""target":{"#), "{bare}");
        for field in ["\"from\"", "\"to\"", "\"instruments\""] {
            assert!(bare.contains(field), "target is missing {field}: {bare}");
        }

        // `why` IS MANDATORY FOR paused AND halted, and is never empty.
        for phase in [Phase::Paused, Phase::Halted] {
            let status = Status {
                phase,
                detail: String::new(),
                ..Status::default()
            };
            assert!(
                !status.why().is_empty(),
                "{} published no reason, which is the failure §4 bans",
                phase.state()
            );
            let json = status.json();
            assert!(json.contains(r#""why":""#), "{json}");
            assert!(!json.contains(r#""why":"""#), "an empty why: {json}");
        }

        // A `running` PAYLOAD ALWAYS CARRIES A `now`. The page treats running
        // with nothing in flight as the hang it exists to expose, so the only
        // two legal shapes are running-with-now and waiting-without.
        let idle_running = Status {
            phase: Phase::Running,
            now: None,
            ..Status::default()
        };
        assert!(
            idle_running.json().contains(r#""state":"waiting""#),
            "running with nothing in flight must not claim to be running"
        );
        let busy = Status {
            phase: Phase::Running,
            now: Some(InFlight {
                instrument: String::from("RELIANCE"),
                month: String::from("2020-05"),
                timeframe: String::from("1min"),
                feed: String::from("groww"),
                index: 431,
                of: 773,
                since: std::time::Instant::now(),
            }),
            ..Status::default()
        };
        let json = busy.json();
        assert!(json.contains(r#""state":"running""#), "{json}");
        assert!(json.contains(r#""instrument":"RELIANCE""#), "{json}");
        assert!(json.contains(r#""index":431"#), "{json}");
        assert!(json.contains(r#""of":773"#), "{json}");
        // A DURATION, NOT AN INSTANT. Compared against the browser's clock, a
        // start instant renders a cell running for minus forty-seven seconds.
        assert!(json.contains(r#""elapsed_ms":"#), "{json}");
        assert!(
            !json.contains(r#""started_at""#),
            "a start instant escaped onto the payload: {json}"
        );
    }

    /// The failure list is bounded, and the journal that holds the rest travels
    /// with it — so a truncated list is never mistaken for the whole history.
    #[test]
    fn the_failure_list_is_bounded_and_names_where_the_rest_are() {
        let control = Control::new();
        for n in 0..(MAX_FAILURES * 2) {
            control.fail(&format!("SYM{n}"), "2020-05", "refused with status 500");
        }
        control.publish(|status| {
            assert_eq!(
                status.failures.len(),
                MAX_FAILURES,
                "the list grew without bound on a route polled every two seconds"
            );
            // NEWEST FIRST.
            assert_eq!(
                status.failures.first().map(|f| f.instrument.as_str()),
                Some(format!("SYM{}", MAX_FAILURES * 2 - 1).as_str())
            );
            status.journal = String::from("/x/audit/pull.journal");
        });
        let json = control.json();
        assert!(
            json.contains(r#""journal":"/x/audit/pull.journal""#),
            "{json}"
        );
        assert!(json.contains("refused with status 500"), "{json}");
    }

    /// A timestamp becomes the day it falls on in IST, and 15:29 IST does not
    /// slip into the next day.
    #[test]
    fn a_stored_timestamp_becomes_the_ist_day_it_falls_on() {
        let d = day(2026, 8, 6);
        assert_eq!(day_of(close_of(d)), Some(d));
    }
}
