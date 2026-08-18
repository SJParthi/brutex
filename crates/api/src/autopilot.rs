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

/// The whole weight of "nothing is contacted before somebody could say no" now
/// rests on that window, because D-0108 made the boot default FLY. A window of
/// zero, or of one second, would remove the consent gate without removing the
/// sentence that promises it — so the floor is a **compile-time** check rather
/// than a test, and shrinking it fails the build with this line as the reason.
const _: () = assert!(GRACE_SECS >= 5);

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

/// How many times one stalled month is put back on the ladder, per process.
///
/// Two, and the number is a **bound on the operator's rate budget**, not a
/// preference. Every stall this build can record is a transport-class reason by
/// construction — the credential class halts in [`FeedState::observe`] before
/// the stall path is reached, and a repeated store refusal halts there too — so
/// a later attempt is justified, and an *unbounded* later attempt would be a
/// quota attack on the owner dressed as resilience.
///
/// The upper bound, stated as a number rather than as a feeling:
/// `MAX_MONTH_ATTEMPTS × (1 + STALL_RETRIES)` = **9** attempts per stalled month
/// per process, spread over at least `2 × STALL_RECHECK_SECS` = twelve hours,
/// and only ever while the ladder has nothing else to do. When the allowance is
/// spent the month stays on the stall list saying so, and this process never
/// asks for it again — `CLAUDE.md` §4, it gives up loudly rather than carrying
/// on quietly.
pub const STALL_RETRIES: u8 = 2;

/// How long a stalled month waits before it may be reconsidered, in seconds.
///
/// Six hours. Long enough that a vendor outage is over and a re-attempt is
/// information rather than noise; short enough that a five-minute blip does not
/// cost that month for the life of the process.
///
/// **The clock starts when the ladder first goes idle with that stall on the
/// list, not at the moment of the stall** — [`FeedState::observe`] is pure with
/// respect to the world and takes no clock, so the first idle pass that sees an
/// unstamped stall stamps it and does not retry it. The wait is therefore
/// always at least this long and sometimes longer, which is the safe direction.
pub const STALL_RECHECK_SECS: i64 = 6 * 3600;

/// How many times a store-halted feed probes its own disk before giving up.
///
/// Eight. A full disk that gets cleared and a volume that gets remounted
/// read-write are real transients, and unlike a dead token the system can
/// **measure** whether the condition still holds — locally, with no vendor, no
/// credential and no rate budget. The passage of time is not evidence; a
/// successful write is.
///
/// Eight probes means seven gaps, and on the doubling schedule
/// [`probe_secs`] gives those are `60 + 120 + 240 + 480 + 960 + 1920 + 3600` =
/// **7,380 seconds, two hours and three minutes** from the first probe to the
/// eighth. After that the feed stays halted with its original reason and a
/// sentence saying the allowance is spent, and nothing touches the disk again
/// until the process is restarted.
pub const STORE_PROBES: u32 = 8;

/// The first wait between store probes, in seconds.
pub const STORE_PROBE_FLOOR_SECS: u64 = 60;

/// The longest wait between store probes, in seconds. One hour.
pub const STORE_PROBE_CEILING_SECS: u64 = 3600;

/// The file name a store probe writes and removes, per vendor.
///
/// A dot-prefixed name directly under the store root, which nothing in this
/// build reads: `census::read_all` opens one known path per vendor and lists no
/// directory, and the two `read_dir` sites under `crates/api` are `assets.rs`
/// (the front-end bundle) and `render.rs` (a bar-file folder). Per vendor
/// rather than shared, so two feeds probing in the same pass cannot remove each
/// other's file and read the removal as a failure.
pub const STORE_PROBE_PREFIX: &str = ".brutex-write-probe-";

/// How many times [`fly`] waits for a usable clock before it gives up.
///
/// Twenty, at [`IDLE_POLL_SECS`] apart — twenty minutes. A machine that boots
/// before NTP has corrected its clock is the case this exists for, and it used
/// to be terminal: `fly` returned, so no backfill ever started for the life of
/// that process and every later resume was refused. Twenty minutes is longer
/// than any sane NTP settle and short enough that a genuinely broken clock is
/// reported rather than waited on for ever.
pub const CLOCK_WAITS: u32 = 20;

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
    // `status 403` AND `tokenexception` ARE THE THIRD BROKER'S SESSION DEATH,
    // AND THEY WERE FILED AS A TRANSPORT BLIP.
    //
    // docs/00-charter.md §4z records it as the row to read twice: Kite answers
    // **403 TokenException** when a session expires, when the user logs out, or
    // **when the user logs into another Kite instance** — so a human opening
    // kite.zerodha.com kills a running backfill. That is a credential fact, and
    // the only cure is a new token; nothing about it improves by waiting.
    //
    // Classified as `Transport`, it fell to the retry ladder: nine attempts per
    // month, backing off, forever, on a session that will never come back
    // without a human — while holding the oldest-month slot away from the feeds
    // that could still run. The charter calls this vendor's expiry its headline
    // failure mode and this build was treating it as a network hiccup.
    //
    // `401` was already here for the other two brokers. `403` is the same fact
    // at a vendor that spells it differently, which is precisely what this
    // table is for.
    const CREDENTIAL: [&str; 8] = [
        "credential",
        "access token expired",
        "aws identity",
        "parameter path",
        "status 401",
        "status 403",
        "tokenexception",
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

/// Whether a credential-shaped reason is about the TOKEN or about one
/// instrument.
///
/// # The defect this removes
///
/// [`tick`] builds its reason from `run.blocked`, else the first entry in
/// `run.total.failures`, else the first refusal. One instrument out of 773
/// answering `status 401` — an entitlement gap on one symbol, which is a
/// vendor's contract with the account and not a fact about the token — used to
/// halt the entire feed for the life of the process, because the reason it put
/// on the page carried a credential marker.
///
/// A genuinely dead token fails **every** instrument. So the halt requires that
/// none was reached, and a credential-shaped reason from a sweep that did reach
/// somebody falls through to the transport arm: bounded backoff, then the
/// stall, both of which are visible and neither of which is terminal.
///
/// # Why `reached == 0` alone, and not `reached == 0 && attempted > 0`
///
/// A run that was BLOCKED before it attempted anything reports
/// `attempted == 0`, and `broker_window`'s credential refusals — an unusable
/// `credentials.toml`, no AWS identity, a parameter path that will not build —
/// are exactly the shape that can produce one. Requiring `attempted > 0` would
/// downgrade a real dead credential into a transport retry, which is the
/// direction that costs the owner rate budget against a fault nothing here can
/// fix. Halting is the direction that costs nothing outside this machine, so
/// the ambiguous case takes it.
#[must_use]
pub const fn credential_is_feedwide(out: &TickOutcome) -> bool {
    out.reached == 0
}

/// The wait before store probe number `made + 1`: 60 s, 120 s, 240 s … capped
/// at one hour.
///
/// Shifting rather than multiplying, and saturating at the cap before the shift
/// can overflow — the same shape [`FeedState::backoff_secs`] uses, for the same
/// reason.
#[must_use]
pub const fn probe_secs(made: u32) -> u64 {
    if made >= 8 {
        return STORE_PROBE_CEILING_SECS;
    }
    let secs = STORE_PROBE_FLOOR_SECS << made;
    if secs > STORE_PROBE_CEILING_SECS {
        STORE_PROBE_CEILING_SECS
    } else {
        secs
    }
}

/// Whether a store-halted feed may probe its disk again, right now.
///
/// Pure over `(probe, now_unix)`. `None` for the probe is a feed that never
/// armed one — a feed halted for a class that has no probe, or one not halted
/// at all — and it answers [`Due::Spent`] with a sentence saying exactly that,
/// so a caller cannot read "no probe" as "probe now".
///
/// # Cost
///
/// Two integer comparisons. Nothing here opens a file.
#[must_use]
pub fn store_due(probe: Option<&Probe>, now_unix: i64) -> Due {
    let Some(probe) = probe else {
        return Due::Spent {
            saying: String::from(
                "no write probe is armed for this feed, so nothing is being re-checked",
            ),
        };
    };
    if probe.made >= STORE_PROBES {
        return Due::Spent {
            saying: format!(
                "the disk was re-checked {STORE_PROBES} times over two hours and three \
                 minutes and refused a few bytes every time. THAT ALLOWANCE IS NOW SPENT: \
                 nothing further is written, nothing further is read, and this feed stays \
                 halted with the reason above until the disk is dealt with and the server is \
                 restarted."
            ),
        };
    }
    if now_unix < probe.due_unix {
        return Due::Later {
            due_unix: probe.due_unix,
        };
    }
    Due::Now { made: probe.made }
}

/// Whether the store root will accept a write, right now.
///
/// # What this is, and what it is emphatically not
///
/// It is a **measurement**, taken locally: create the directory if it is not
/// there, write a few bytes to a per-vendor dot-file directly under the store
/// root, `sync_all` so the answer is the disk's and not the page cache's, and
/// remove it. It contacts no vendor, opens no socket, reads no credential and
/// spends no rate budget, so it can be taken on a schedule without any of it
/// costing the owner anything outside this machine.
///
/// It is **not** a retry of the write that failed. Nothing is re-fetched and no
/// bar file and no manifest is touched: a fault that only affects one bar file
/// while the root stays writable is a fault this probe will report as fine, and
/// the very next tick will halt again on the real refusal. That is the honest
/// failure mode and it is bounded — see [`STORE_PROBES`] — rather than a loop.
///
/// # Idempotence
///
/// `CLAUDE.md` §3 rule 5. The same path, the same bytes, removed each time.
/// Running it twice leaves the store byte-for-byte where running it once did,
/// and running it zero times leaves it there too.
///
/// # Errors
///
/// The host's own words, prefixed with the path, so a `permission denied` on a
/// root owned by somebody else says which path and which vendor.
///
/// **One error arm, not five.** The four steps below can each fail, and only two
/// of those failures can be produced without a full disk or a hostile
/// filesystem — so five separately formatted arms would be three regions no test
/// in this repository could ever enter. They funnel through [`probe_io`]'s
/// `io::Result` into the single `map_err` here, which carries the host's own
/// words whichever step produced them.
pub fn store_writable(root: &std::path::Path, vendor: &str) -> Result<(), String> {
    let path = root.join(format!("{STORE_PROBE_PREFIX}{vendor}"));
    probe_io(root, &path)
        .map_err(|e| format!("the store refused a write probe at {}: {e}", path.display()))
}

/// The four filesystem calls one probe makes, with the host's errors intact.
///
/// Separated from [`store_writable`] purely so there is one place that turns an
/// `io::Error` into a sentence. Nothing here is public and nothing here decides
/// anything.
fn probe_io(root: &std::path::Path, path: &std::path::Path) -> std::io::Result<()> {
    use std::io::Write as _;
    std::fs::create_dir_all(root)?;
    let mut file = std::fs::File::create(path)?;
    file.write_all(b"brutex write probe\n")?;
    // THE BYTES HAVE TO REACH THE DEVICE. A write that only reached the page
    // cache answers "the disk is fine" on a disk that is full, which is the one
    // answer this probe exists to refuse to give.
    file.sync_all()?;
    std::fs::remove_file(path)
}

/// Whether this vendor's manifest loads, right now.
///
/// **`Census::Held` and nothing else.** `Census::Absent` deliberately does NOT
/// clear a census halt: absent means the store is reporting that it holds
/// nothing, and a feed revived on that reading would re-offer months whose bar
/// files are still on disk. Those offers are refused by `BarFile::append`
/// wholesale — the overlap is not a suffix — so the feed would fail, stall and
/// walk the whole ladder for nothing. A manifest that loads is the only reading
/// that is evidence.
///
/// # Cost
///
/// One pass over the four censuses already read this round. No file is opened.
#[must_use]
pub fn manifest_loads(censuses: &[VendorCensus], vendor: brutex_core::vendor::Vendor) -> bool {
    censuses
        .iter()
        .any(|c| c.vendor == vendor && matches!(c.state, Census::Held { .. }))
}

/// What to say while waiting for a usable clock, or `None` when the allowance
/// is spent.
///
/// Split out of [`fly`] because it is the only half of that wait a test can
/// reach: `yesterday_ist` reads `SystemTime::now()` and on any machine this
/// suite runs on it succeeds, so the loop body is unreachable and the decision
/// is not.
///
/// The bound is [`CLOCK_WAITS`], and when it is spent this answers `None` and
/// the caller returns — loudly, with the reason on the page — rather than
/// waiting for ever on a clock that is not going to be fixed.
#[must_use]
pub fn clock_wait(waits: u32) -> Option<String> {
    if waits >= CLOCK_WAITS {
        return None;
    }
    Some(format!(
        "the clock is unusable, so the newest finished day cannot be established and \
         nothing can be asked for safely. Nothing is being contacted. This re-derives it \
         every {IDLE_POLL_SECS}s — check {} of {CLOCK_WAITS} — because a machine that boots \
         before its time is corrected has a clock that fixes itself, and a backfill that \
         gave up on the first reading would need a restart it should not need.",
        waits.saturating_add(1)
    ))
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
    /// Why this tick's record did not reach the journal, verbatim. `None` when
    /// it did.
    ///
    /// It does NOT change what the tick decides — a pull that stored bars stored
    /// them whether or not the audit line landed — and it may not be silent
    /// either, so it travels to [`Status::journal_error`] and onto
    /// `/autopilot.json`.
    pub journal_error: Option<String>,
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
    /// How many times this month has been put back on the ladder since.
    ///
    /// Bounded by [`STALL_RETRIES`]. When it reaches the bound the month is
    /// never reconsidered again by this process, and [`stall_note`] says so on
    /// the page rather than leaving the list looking as though something is
    /// still going to happen.
    pub retried: u8,
    /// Epoch seconds this stall was last acted on: stamped when the idle ladder
    /// first sees it, and re-stamped on every reconsideration.
    ///
    /// **Zero means never stamped**, which is the state a fresh stall is pushed
    /// in — [`FeedState::observe`] reads no clock, deliberately, so that every
    /// arm of it is drivable from a test with no vendor, no store and no clock.
    pub at_unix: i64,
}

/// Which class of fault made a feed terminal, and therefore what — if anything
/// — could ever clear it without a restart.
///
/// This is not a second copy of the halt reason: the reason is the sentence an
/// operator reads and it stays verbatim on the page. This is the machine's own
/// answer to "is there anything I could *measure* that would tell me this has
/// been fixed", and there are exactly three answers because there are exactly
/// three halt sites.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Halt {
    /// The broker credential is dead and re-reading it returned the same value.
    ///
    /// **Nothing in this process can clear this, and nothing here pretends
    /// otherwise.** `CLAUDE.md` §8 forbids minting a token, so the only cure is
    /// a human rotating it in AWS Parameter Store. A timer-driven retry against
    /// an unchanged dead value is the auto-retry that hides a permanent fault —
    /// exactly the shape §4 bans — so this class is never re-checked here. It
    /// is named, and left named, until the process is restarted.
    Credential,
    /// The store refused the same write twice.
    ///
    /// Re-checkable, and the check costs nothing outside this machine: a few
    /// bytes written under the store root, `sync_all`-ed and removed. See
    /// [`store_writable`] and [`STORE_PROBES`].
    Store,
    /// The vendor's manifest exists and will not load.
    ///
    /// Re-checkable for **free**: [`round`] already re-reads every census once
    /// per pass, so noticing that the file now loads is a comparison on data
    /// already in hand rather than a new operation. See [`survey`].
    Census,
}

impl Halt {
    /// The one word this class carries into a sentence.
    #[must_use]
    pub const fn word(self) -> &'static str {
        match self {
            Self::Credential => "credential",
            Self::Store => "store",
            Self::Census => "census",
        }
    }
}

/// The state of one store-halted feed's write probe.
///
/// Bounded by [`STORE_PROBES`] and scheduled by [`probe_secs`]. It holds a
/// count and a due time and nothing else — no handle, no descriptor, nothing
/// that could keep a file open across a pass.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Probe {
    /// How many probes have been made. Never exceeds [`STORE_PROBES`].
    pub made: u32,
    /// Epoch seconds the next probe is due. Zero on the pass that arms it,
    /// which makes the first probe due immediately.
    pub due_unix: i64,
}

/// Whether a store-halted feed is due for another probe.
///
/// Pure over `(probe, now)`, so all three arms are drivable from a test with no
/// disk and no clock — which is the point of splitting it out of [`round`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Due {
    /// Probe now. This is probe number `made + 1` of [`STORE_PROBES`].
    Now {
        /// How many probes have already been made.
        made: u32,
    },
    /// Not yet. The next probe is due at this epoch second.
    Later {
        /// When.
        due_unix: i64,
    },
    /// The allowance is spent and the reason says so.
    ///
    /// Nothing further is written, nothing further is read, and the feed stays
    /// halted with the reason it halted for. `CLAUDE.md` §4: it gives up out
    /// loud rather than carrying on quietly.
    Spent {
        /// The sentence, ready for the page.
        saying: String,
    },
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
    ///
    /// **No route can clear it, and that has not changed.** The feed table is a
    /// local of [`fly`] — `let mut feeds = drivable(yesterday)` — so nothing an
    /// HTTP handler holds has a reference to this field. A resume clears
    /// [`Control::paused`], which this does not read. That is why
    /// [`admit_resume`] refuses a resume that would change nothing instead of
    /// publishing "resumed" over a halt: see [`RESUME_CANNOT_CLEAR`], every word
    /// of which is still true.
    ///
    /// **What HAS changed is that two of the three classes can now be cleared by
    /// EVIDENCE**, which is not a control and cannot be pressed. A census that
    /// loads and a store that accepts a write are measurements this process
    /// takes anyway or can take locally; when one of them says the fault is
    /// gone, the backfill carries on and says when and why. The third class —
    /// a dead broker credential — is never re-checked here at all, because
    /// `CLAUDE.md` §8 forbids minting a token and a timer-driven retry against
    /// an unchanged dead value is precisely the auto-retry §4 bans. See
    /// [`Halt`].
    ///
    /// A restart still clears every class, and a restart is safe by
    /// construction — `fly` rebuilds every feed at its floor and re-derives the
    /// frontier from the store, which is the module doc's "what is NOT
    /// persisted" rule.
    pub halted: Option<String>,
    /// Which class of fault made it terminal, when it is.
    ///
    /// Set beside [`Self::halted`] at all three halt sites and cleared with it.
    /// It carries no reason of its own — the reason is the string above,
    /// verbatim — only the machine's answer to what could be measured.
    pub halt_kind: Option<Halt>,
    /// The store-halt write probe, when one is armed.
    ///
    /// `Some` only while [`Self::halt_kind`] is [`Halt::Store`] and the
    /// allowance is not spent.
    pub probe: Option<Probe>,
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
            halt_kind: None,
            probe: None,
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

    /// Go terminal: record the reason verbatim, record the class, and arm
    /// whatever re-check that class permits.
    ///
    /// One function for all three halt sites, so a site cannot set a reason
    /// without a class — a halt whose class is unknown is one nothing could
    /// ever honestly re-check, and it would silently become permanent.
    ///
    /// [`Halt::Store`] arms a probe due immediately; the other two arm nothing.
    /// [`Halt::Credential`] deliberately arms nothing: `CLAUDE.md` §8.
    fn halt(&mut self, kind: Halt, why: String) {
        self.halted = Some(why);
        self.halt_kind = Some(kind);
        self.probe = match kind {
            Halt::Store => Some(Probe {
                made: 0,
                due_unix: 0,
            }),
            Halt::Credential | Halt::Census => None,
        };
    }

    /// Come back from terminal, because something MEASURED said the fault is
    /// gone.
    ///
    /// Never called by a route and never called on a timer. The two callers are
    /// [`survey`], when the vendor's manifest loads again, and [`round`], when a
    /// write probe succeeds. Both reset the credential re-read allowance too:
    /// a feed that is being driven again is owed its one §8 re-read the same as
    /// a fresh one.
    fn revive(&mut self) {
        self.halted = None;
        self.halt_kind = None;
        self.probe = None;
        self.rereads = CREDENTIAL_REREADS;
        self.clear_month();
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
    /// 1. **A credential failure that reached NOBODY outranks everything.**
    ///    `CLAUDE.md` §8 — one automatic re-read, then halt. It is checked
    ///    before progress because a token that dies mid-sweep still stores the
    ///    instruments it reached, and treating that as progress would retry
    ///    forever against a dead credential. The "reached nobody" clause is
    ///    [`credential_is_feedwide`] and it is what stops ONE instrument's 401
    ///    killing a whole feed for the life of the process.
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
            if classify(&reason) == Trouble::Credential && credential_is_feedwide(out) {
                if self.rereads > 0 {
                    self.rereads = self.rereads.saturating_sub(1);
                    return Next::Retry;
                }
                let why = format!(
                    "the broker credential is dead and re-reading it returned the same \
                     value, and NOT ONE of the {} instruments asked answered. CLAUDE.md §8: \
                     this repository never mints a token, so nothing further is attempted \
                     and nothing is re-tried on a timer — a retry against an unchanged dead \
                     value would hide a permanent fault, which §4 bans. Refresh it in AWS \
                     Parameter Store and RESTART the server; a resume does not clear a halt, \
                     and POST /autopilot/control refuses one that would change nothing rather \
                     than pretending to. The reason, verbatim: {reason}",
                    out.attempted
                );
                self.halt(Halt::Credential, why.clone());
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
                     retryable — nothing further is FETCHED until the disk is dealt with. \
                     The disk itself is re-checked up to {STORE_PROBES} times with a local \
                     write probe that contacts nothing and spends no rate budget; if one \
                     succeeds the backfill carries on by itself and says when."
                );
                self.halt(Halt::Store, why.clone());
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
                    // NEITHER FIELD IS STAMPED HERE. This function reads no
                    // clock, deliberately — see its doc comment — so the idle
                    // ladder stamps `at_unix` the first time it sees the stall
                    // and `reconsider` is what moves `retried`.
                    retried: 0,
                    at_unix: 0,
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

/// Put ONE stalled month back on the ladder, if one has earned it.
///
/// # Why this exists
///
/// A month that failed [`MAX_MONTH_ATTEMPTS`] times used to be skipped for the
/// **life of the process** — no later attempt, no reconsideration, nothing. A
/// five-minute vendor outage in 2021-03 cost that month permanently, even while
/// the process sat idle afterwards with nothing else to do. Every stall this
/// build can record is transport-class by construction, which is exactly the
/// class where a later attempt is justified.
///
/// # Why it is bounded, and where the bound is
///
/// [`STALL_RETRIES`] per month per process, at least [`STALL_RECHECK_SECS`]
/// apart, and **only from the idle branch of [`round`]** — the branch that
/// already means "nothing is missing that any feed can still be asked for". So
/// it can never delay forward progress and it can never become a quota attack:
/// at most `MAX_MONTH_ATTEMPTS × (1 + STALL_RETRIES)` = 9 attempts per
/// stalled month per process. When a month's allowance is spent it stays on the
/// list, [`stall_note`] says so, and nothing asks for it again.
///
/// # Why moving the frontier BACKWARD is safe
///
/// The store is one file per month, so re-attempting month M writes `M.bin` and
/// cannot reach M+1. Within M the window is re-derived from the manifest by
/// [`next_window`], which starts at the earliest resume point across the series
/// that are behind, and `BarFile::append` verifies the overlap byte for byte and
/// appends only the suffix. A re-attempt therefore costs vendor budget and
/// cannot corrupt. `survey` re-derives the frontier upward from wherever this
/// leaves it, so monotonicity is restored on the next pass by the same scan that
/// always establishes it.
///
/// # First sighting stamps, and never retries
///
/// A fresh stall carries `at_unix == 0` because [`FeedState::observe`] reads no
/// clock. The first idle pass that sees one stamps it with `now_unix` and does
/// **not** reconsider it — `now - now` is zero, which is below the threshold. So
/// the wait is always at least [`STALL_RECHECK_SECS`] and never less.
///
/// # Cost
///
/// One pass over the stall list of every non-terminal feed. Nothing here reads
/// the store, opens a socket or takes a lock.
pub fn reconsider(feeds: &mut [FeedState], now_unix: i64) -> Option<String> {
    // FIRST SIGHTING. Stamping is separate from choosing so that a stall
    // recorded this second cannot also be retried this second.
    for state in feeds.iter_mut() {
        for stall in &mut state.stalls {
            if stall.at_unix == 0 {
                stall.at_unix = now_unix;
            }
        }
    }
    // THE OLDEST MONTH ANY FEED MAY BE ASKED FOR AGAIN — the same rule the
    // ladder itself climbs by, so a reconsideration cannot jump the queue.
    let mut best: Option<(usize, usize, u32)> = None;
    for (slot, state) in feeds.iter().enumerate() {
        if state.halted.is_some() {
            continue;
        }
        for (nth, stall) in state.stalls.iter().enumerate() {
            let due = now_unix.saturating_sub(stall.at_unix) >= STALL_RECHECK_SECS;
            if stall.retried >= STALL_RETRIES || !due {
                continue;
            }
            let rung = ordinal(stall.month);
            if best.is_none_or(|(_, _, held)| rung < held) {
                best = Some((slot, nth, rung));
            }
        }
    }
    let (slot, nth, _) = best?;
    let state = feeds.get_mut(slot)?;
    let feed = state.feed.display().to_owned();
    let stall = state.stalls.get_mut(nth)?;
    stall.retried = stall.retried.saturating_add(1);
    stall.at_unix = now_unix;
    let month = stall.month;
    let retried = stall.retried;
    let reason = stall.reason.clone();
    // THE FRONTIER GOES BACK, AND ONLY HERE. Everywhere else it is monotone.
    state.frontier = month;
    state.clear_month();
    Some(format!(
        "nothing else is missing, so {feed}'s stalled month {month} is being reconsidered — \
         attempt {retried} of {STALL_RETRIES} allowed after the stall, at least \
         {STALL_RECHECK_SECS}s since the last one. Nothing is replayed: the window is \
         re-derived from what the store already holds, so a day already stored is not \
         asked for twice. It was stalled for: {reason}"
    ))
}

/// What the stall lists across every feed add up to, as one sentence.
///
/// Empty when there are no stalls at all, which is the ordinary case and must
/// not put a reassuring sentence on the page for a fact nobody asserted.
///
/// Otherwise it separates the two states that look identical on a list and are
/// not: months that are still going to be tried again, and months whose
/// allowance is spent and which **this process will never ask for again**. The
/// second half is the loud half — `CLAUDE.md` §4, a recovery that gives up says
/// so rather than leaving a list that reads as though something is pending.
#[must_use]
pub fn stall_note(feeds: &[FeedState]) -> String {
    let mut waiting = 0usize;
    let mut spent = 0usize;
    for state in feeds {
        for stall in &state.stalls {
            if stall.retried >= STALL_RETRIES {
                spent = spent.saturating_add(1);
            } else {
                waiting = waiting.saturating_add(1);
            }
        }
    }
    if waiting == 0 && spent == 0 {
        return String::new();
    }
    format!(
        " {} stalled month(s) below: {waiting} still to be reconsidered (at most \
         {STALL_RETRIES} more attempts each, at least {STALL_RECHECK_SECS}s apart, and only \
         while nothing else is missing) and {spent} whose allowance is SPENT — this process \
         will not ask for those again, and the reasons are on each one.",
        waiting.saturating_add(spent)
    )
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
    /// Why the last tick's record did not reach that journal. Empty when every
    /// record this process wrote landed.
    ///
    /// **A path is not a proof.** `journal` said where the file is and every
    /// page read it as evidence the runs were being recorded; the append's
    /// answer was discarded at the one site that produces it. This is that
    /// answer, and `/autopilot.json` carries it as `journal_error`.
    pub journal_error: String,
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
            journal_error: String::new(),
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
            r#"{{"state":{},"why":{},"cursor":{},"journal":{},"journal_error":{},"waiting_ms":{},"absorbed_ms":{},"target":{{"from":{},"to":{},"instruments":{},"timeframe":{},"feed":{}}},"now":"#,
            render::json_string(self.state()),
            render::json_string(&self.why()),
            render::json_string(&self.cursor),
            render::json_string(&self.journal),
            render::json_string(&self.journal_error),
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
                    r#"{{"month":{},"attempts":{},"retried":{},"retries_max":{},"reason":{}}}"#,
                    render::json_string(&stall.month.to_string()),
                    stall.attempts,
                    stall.retried,
                    STALL_RETRIES,
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

/// The environment variable that holds the autopilot on the ground.
///
/// # The default is PAUSED, and this block used to say the opposite
///
/// **What is true, and what [`stays_paused_from`] a hundred lines below
/// enforces:** the autopilot flies only when this variable reads exactly
/// [`AUTOPILOT_RUN`]. Absent, empty, mistyped, or not UTF-8 all stay on the
/// ground, because every one of them is "the operator did not ask".
///
/// **What this block used to claim.** That the default had been changed to FLY,
/// on the reasoning that a Run configuration setting no environment would
/// otherwise come up paused and do nothing. That change was made and then
/// REVERSED, and the reversal is documented at [`stays_paused_from`] — one
/// session flying by default wrote 23,695 `pull.run` events before anyone
/// looked. Fetching is the irreversible half of this program: it spends a shared
/// vendor quota and writes to an append-only store, so the default must be the
/// safe side of it.
///
/// The prose was left behind by that reversal and is the reason this section now
/// leads with the polarity rather than the history. A reader who stopped here
/// would have concluded that pressing Run contacts a vendor.
///
/// **The controls, unchanged:** the twenty-second grace window
/// ([`GRACE_SECS`], counted down by [`grace`]) before any socket opens;
/// `POST /autopilot/pause` and `POST /autopilot/control action=stop`, which bite
/// within one instrument; and this variable, set to [`AUTOPILOT_RUN`], which is
/// the only thing that starts it at all.
pub const AUTOPILOT_ENV: &str = "BRUTEX_AUTOPILOT";

/// The one value that still starts it explicitly.
///
/// Kept accepted although it is no longer required: an existing shell alias or
/// launcher that exports `BRUTEX_AUTOPILOT=run` must not silently start meaning
/// something else. It flies, exactly as it always did — it is simply no longer
/// the only way to.
pub const AUTOPILOT_RUN: &str = "run";

/// A spelling kept accepted for an operator who already sets it.
///
/// **This doc used to say `PAUSE`, `paused`, `stop`, `false`, `0` and `no` all
/// FLY.** They do not, and have not since the polarity was inverted: the switch
/// is positive, so [`stays_paused_from`] grounds every value that is not exactly
/// [`AUTOPILOT_RUN`] — this one included, along with an absent variable, an
/// empty one, a typo and a value that is not UTF-8.
/// `the_boot_default_pulls_nothing_and_only_the_exact_word_run_lets_it_fly`
/// asserts `stays_paused_from(Some("pause"))` for this exact string, so the
/// sentence above was already contradicted by a test in its own file.
///
/// The constant survives because it is harmless and because a value that reads
/// as an instruction should mean what it says. It grants nothing that the
/// variable's absence does not already grant.
pub const AUTOPILOT_PAUSE: &str = "pause";

/// Whether a value read from [`AUTOPILOT_ENV`] holds the autopilot on the
/// ground.
///
/// # Why the value is a parameter rather than a read
///
/// The same reason `server::masters_dir_from` is split from
/// `server::default_masters_dir_from`: `set_var` is `unsafe` under edition 2024,
/// this crate forbids `unsafe`, and mutating process-wide state would race every
/// other test in the binary. A function that reads the environment inline has an
/// arm no test can enter, and `CLAUDE.md` §9's coverage floor is not something
/// to work around with a comment. This half takes the value; [`flies_on_startup`]
/// is the one line that fetches it.
///
/// `None` — the variable is absent — **stays on the ground.**
///
/// **THIS DEFAULT WAS REVERSED, AND THE REVERSAL IS THE POINT.** It used to fly
/// when the variable was absent, so starting the server was itself enough to
/// begin fetching from a vendor. Nobody clicked anything; the operator pressed
/// Run in an IDE and a backfill started twenty seconds later. One such session
/// wrote 23,695 `pull.run` events before anyone looked.
///
/// The owner's instruction is that **no data is pulled unless they ask for it**,
/// and a default that fetches is the opposite of that however loudly the
/// terminal announces it. Fetching is the irreversible half of this program: it
/// spends a shared vendor quota and writes to an append-only store. A default
/// must be the safe side of an irreversible action.
///
/// So the switch is now positive: only [`AUTOPILOT_RUN`] flies. An absent
/// variable, an empty one, a typo, a value that is not UTF-8 — all stay on the
/// ground, because every one of them is "the operator did not ask".
#[must_use]
pub fn stays_paused_from(value: Option<&std::ffi::OsStr>) -> bool {
    value.is_none_or(|v| v != std::ffi::OsStr::new(AUTOPILOT_RUN))
}

/// Whether the environment lets the autopilot fly. It does **only** when the
/// variable says [`AUTOPILOT_RUN`] exactly.
///
/// Read once, at construction. Changing the variable on a running process does
/// nothing — the operator pauses and resumes through `/autopilot/pause` and
/// `/autopilot/resume`, which are the controls the page already uses, and a
/// second source of truth for the same switch is how the two disagree.
///
/// `var_os` rather than `var`: a value that is not UTF-8 is not the byte string
/// `run`, so it stays paused, which is the same answer every other unrecognised
/// value gets. Under the old polarity that reasoning ran the other way and an
/// unreadable value FLEW, which is the case this inversion most needed to fix.
#[must_use]
pub fn flies_on_startup() -> bool {
    !stays_paused_from(std::env::var_os(AUTOPILOT_ENV).as_deref())
}

impl Control {
    /// A control that has not started, and is not paused.
    ///
    /// A pure constructor: it reads no environment and makes no policy. The
    /// SERVER decides whether to fly — see [`serving`] — because a `Control`
    /// built inside a test must mean exactly what the test says it means, and
    /// a constructor that consults the environment would make every test's
    /// meaning depend on the shell that ran it.
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
        let epoch = self.epoch.fetch_add(1, Ordering::Relaxed).saturating_add(1);
        // A PAUSE IS AN OPERATOR DECISION AND IT STOPS A BACKFILL. Left
        // unlogged, a run that halted because somebody pressed pause and one
        // that halted because the vendor stopped answering read the same on
        // every surface this process keeps.
        let _dropped_when_filtered = telemetry::emit(
            &telemetry::Event::warn("autopilot", "paused")
                .with("epoch", telemetry::Value::Uint(epoch)),
        );
    }

    /// Start again. Does not bump the epoch: a sweep that is still unwinding
    /// must still unwind.
    pub fn resume(&self) {
        self.paused.store(false, Ordering::Relaxed);
        let _dropped_when_filtered =
            telemetry::emit(&telemetry::Event::info("autopilot", "resumed").with(
                "epoch",
                telemetry::Value::Uint(self.epoch.load(Ordering::Relaxed)),
            ));
    }

    /// The control a SERVING process gets: **flying**, unless the environment
    /// says [`AUTOPILOT_PAUSE`].
    ///
    /// This is the one place the default lives. `Control::new` stays a pure
    /// constructor so tests mean what they say; policy belongs here, where a
    /// reader looking for "does starting the binary contact a vendor" finds
    /// the answer in one function rather than inferring it from a spawn site.
    ///
    /// The line below is byte-for-byte what it was before the default was
    /// inverted. [`flies_on_startup`] is where the meaning changed, and the
    /// whole argument is in its doc comment — a policy flip that leaves the
    /// call site looking untouched is one a reader can audit in one place.
    #[must_use]
    pub fn serving() -> Self {
        let control = Self::new();
        if !flies_on_startup() {
            control.pause();
        }
        control
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

    /// Read the published status without cloning it, or `None` when the lock is
    /// poisoned.
    ///
    /// **`None` is a refusal, not an empty status**, and every caller must
    /// treat it as one: a poisoned lock means a previous publisher panicked and
    /// what the backfill is doing cannot be established. `CLAUDE.md` §4 —
    /// answering "nothing is halted" from a lock that cannot be read is exactly
    /// the fallback that hides a failure. [`Control::json`] already takes this
    /// position for the status route; this is the same position for the two
    /// routes that have to DECIDE on what they read.
    ///
    /// The closure returns rather than the `Status` being cloned, so a caller
    /// that wants one number does not copy a paragraph and a vector of feed
    /// reports. One uncontended lock, O(what the closure reads).
    pub fn inspect<T, F: FnOnce(&Status) -> T>(&self, read: F) -> Option<T> {
        self.status.lock().ok().map(|held| read(&held))
    }

    /// Whether the one pull seat is taken, right now. One acquire load.
    ///
    /// **A held seat does not mean a hand-made pull is running.** [`round`]
    /// takes it for the whole of every pass, so the backfill's own tick holds it
    /// too. It is reported as the fact it is — held or free — and nothing infers
    /// a blocker from it alone; what is standing off, and why, is
    /// [`Status::detail`]'s job.
    #[must_use]
    pub fn seat_held(&self) -> bool {
        self.seat.load(Ordering::Acquire)
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
    // THE CLOCK IS READ HERE rather than inside the clamp, which used to read
    // it and therefore could not be tested against a stated day. `yesterday` is
    // the window's end and is NOT today: a rolling floor resolves against
    // today, so today is what is asked for.
    let today = crate::ingest::ist_day(std::time::SystemTime::now()).ok()?;
    crate::server::clamp_to_floor(whole, spec.history_floor, today)
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
    // A CLOCK THAT IS NOT READY YET IS NOT A CLOCK THAT IS BROKEN.
    //
    // This used to `return`, which made an NTP-shaped fault terminal: a machine
    // that boots before its time is corrected never started a backfill for the
    // life of the process, and `admit_resume` then refused every resume for ever
    // because the task had gone. `round` already handles the same failure
    // correctly at its own site by returning `IDLE_POLL_SECS` and being called
    // again; this is the same treatment for the pre-loop copy, and it is bounded
    // by `CLOCK_WAITS` so a clock that will never be fixed is reported rather
    // than waited on for ever.
    let mut clock_waits = 0u32;
    let yesterday = loop {
        if let Some(day) = yesterday_ist(std::time::SystemTime::now()) {
            break day;
        }
        let Some(saying) = clock_wait(clock_waits) else {
            site.autopilot.publish(|status| {
                status.phase = Phase::Halted;
                status.detail = format!(
                    "the clock is unusable, so the newest finished day cannot be \
                     established and nothing can be asked for safely. It was re-derived \
                     {CLOCK_WAITS} times over {} minutes and never became usable. THAT \
                     ALLOWANCE IS SPENT: this backfill task has stopped and only a restart \
                     starts another.",
                    u64::from(CLOCK_WAITS) * IDLE_POLL_SECS / 60
                );
                status.due_unix = 0;
            });
            return;
        };
        site.autopilot.publish(move |status| {
            status.phase = Phase::Halted;
            status.detail = saying;
            status.since_unix = ingest::epoch_secs(std::time::SystemTime::now());
            status.due_unix = 0;
        });
        clock_waits = clock_waits.saturating_add(1);
        tokio::time::sleep(std::time::Duration::from_secs(IDLE_POLL_SECS)).await;
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
        // A REPAIRED MANIFEST CLEARS ITS OWN HALT, AND IT COSTS NOTHING.
        //
        // `round` re-reads every census once per pass already, so this is a
        // comparison on data that is in hand rather than a new operation —
        // there is no loop here to bound because there is no work here to
        // repeat. Before this existed, a manifest an operator had repaired
        // still required a full server restart, and the process spent
        // `IDLE_POLL_SECS` re-reading and CRC-verifying every entry once a
        // minute for ever, learning the answer and throwing it away.
        //
        // `Census::Held` and nothing else — see `manifest_loads` for why an
        // ABSENT manifest must not revive a feed.
        if state.halt_kind == Some(Halt::Census) && manifest_loads(censuses, state.vendor) {
            let vendor = state.vendor.as_str();
            state.revive();
            state.last_reason = Some(format!(
                "{vendor}'s manifest loads again, so the halt it caused is cleared and this \
                 feed carries on from wherever the store now reaches. Nothing was guessed \
                 and nothing was rebuilt: the file was re-read on this pass, as it is on \
                 every pass, and it verified."
            ));
        }
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
                     cannot be established and nothing is fetched against a guess: {reason}. \
                     This build has no manifest reconstructor and will not invent one — \
                     rebuilding a census that has already failed a checksum is the wrong \
                     instinct. The file IS re-read on every pass, so a repair clears this by \
                     itself and no restart is needed for it.",
                    state.vendor.as_str()
                );
                state.halt(Halt::Census, why.clone());
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

/// Re-check the disk for every feed a store refusal made terminal, and revive
/// the ones it accepts.
///
/// # The one class of halt that can honestly heal itself
///
/// A full disk that logrotate or the operator clears, and a volume that gets
/// remounted read-write, are real transients — and unlike a dead credential the
/// system can **measure** whether the condition still holds, locally, with no
/// vendor, no credential, no rate budget and no bar request. That is the whole
/// distinction: the clear is keyed on a successful write, never on the passage
/// of time. A `permission denied` on a root owned by somebody else simply keeps
/// failing the probe, which is the truthful answer.
///
/// # What it is not
///
/// It is not a retry of the fetch. Nothing is asked of any vendor here, the
/// phase stays `Halted` for as long as the probe fails, the original reason
/// stays on the page verbatim, and the check count and the next check time are
/// published beside it. `CLAUDE.md` §4's banned shape is a retry that HIDES a
/// permanent fault; nothing is hidden and nothing is claimed that was not
/// measured.
///
/// # Bound
///
/// [`STORE_PROBES`] probes per halt, scheduled by [`probe_secs`], after which
/// [`store_due`] answers [`Due::Spent`] and this stops touching the disk for
/// that feed entirely — and says so, every pass, rather than going quiet.
///
/// # Cost
///
/// At most one `create`, one `write_all`, one `sync_all` and one `unlink` of
/// nineteen bytes per store-halted feed per pass, and only when a probe is due.
/// A feed that is not store-halted costs one enum comparison.
fn probe_store_halts(site: &Loaded, feeds: &mut [FeedState]) -> String {
    use std::fmt::Write as _;
    let now = ingest::epoch_secs(std::time::SystemTime::now());
    let mut said = String::new();
    for state in feeds.iter_mut() {
        if state.halt_kind != Some(Halt::Store) {
            continue;
        }
        let feed = state.feed.display().to_owned();
        let vendor = state.vendor.as_str();
        match store_due(state.probe.as_ref(), now) {
            Due::Now { made } => {
                let attempt = made.saturating_add(1);
                match store_writable(&site.store_root, vendor) {
                    Ok(()) => {
                        state.revive();
                        let saying = format!(
                            "{feed}'s store halt is CLEARED: write probe {attempt} of \
                             {STORE_PROBES} under {} succeeded — a few bytes written, synced \
                             and removed — so the disk now accepts writes and the backfill \
                             carries on. Nothing was asked of any vendor to establish this.",
                            site.store_root.display()
                        );
                        state.last_reason = Some(saying.clone());
                        let _ = write!(said, " {saying}");
                    }
                    Err(why) => {
                        let wait = probe_secs(made);
                        state.probe = Some(Probe {
                            made: attempt,
                            due_unix: now.saturating_add(i64::try_from(wait).unwrap_or(i64::MAX)),
                        });
                        let _ = write!(
                            said,
                            " {feed} stays halted: write probe {attempt} of {STORE_PROBES} \
                             failed too — {why}. The next probe is in {wait}s and nothing is \
                             being asked of any vendor meanwhile."
                        );
                    }
                }
            }
            Due::Later { due_unix } => {
                let left = due_unix.saturating_sub(now).max(0);
                let made = state.probe.map_or(0, |p| p.made);
                let _ = write!(
                    said,
                    " {feed} stays halted: {made} of {STORE_PROBES} write probes made, the \
                     next in {left}s."
                );
            }
            // ONLY WHEN A PROBE WAS ACTUALLY ARMED. `store_due` also answers
            // `Spent` for a feed that armed none, and printing that sentence for
            // a feed that never had an allowance would be a refusal about a
            // thing that never happened.
            Due::Spent { saying } => {
                if state.probe.is_some() {
                    let _ = write!(said, " {feed}: {saying}");
                }
            }
        }
    }
    said
}

/// A "nothing is missing" verdict and the universe it was measured over, bound
/// into one value.
///
/// # The lie this type makes unrepresentable
///
/// With the masters absent, `tracked_series` derives its work list from
/// `site.read.merged.by_key`, which is empty; `next_window` then answers `None`
/// for every feed because it has no series to accumulate a window from; `survey`
/// chooses nothing; and the no-work branch below published
///
/// > nothing is missing that any feed can still be asked for. The store is
/// > complete through the newest finished day
///
/// over a store holding nothing, at phase `idle`, once a minute for the life of
/// the process. Every step was individually correct. **"Nothing is missing" is
/// vacuously true over an empty work list**, and the sentence a human reads from
/// it is not.
///
/// A guard — `if series.is_empty() { … }` beside the `format!` — would have
/// fixed today's path and left the shape intact: the claim and the evidence
/// would still be two separate things, and the next writer to add an arm gets
/// the same defect back. So the completeness sentence is not reachable from a
/// count at all. It is reachable only from [`Self::Complete`], which holds a
/// [`std::num::NonZeroUsize`] and therefore **cannot be constructed over an
/// empty universe**. There is no code path from zero instruments to the word
/// "complete". D-0124.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Settled {
    /// Nothing is missing, across this many tracked instruments. The number is
    /// the evidence and it travels with the claim into the sentence.
    Complete {
        /// How many instruments the verdict was measured over. Never zero — the
        /// type says so.
        instruments: std::num::NonZeroUsize,
    },
    /// There is no universe for anything to be missing from.
    ///
    /// Not "complete", not "idle", not "up to date": the masters produced no
    /// instrument, so no month was ever a candidate and no feed was ever asked.
    NoUniverse,
}

impl Settled {
    /// The verdict the work list itself supports.
    ///
    /// The ONLY constructor. `Complete` is private to this impl in effect,
    /// because every caller reaches it through here and here refuses zero.
    fn over(series: &[Series]) -> Self {
        std::num::NonZeroUsize::new(series.len()).map_or(Self::NoUniverse, |instruments| {
            Self::Complete { instruments }
        })
    }

    /// Whether this verdict stops the backfill rather than idling it.
    ///
    /// An empty universe cannot change while the process runs — the masters are
    /// read once, at startup — so idling on it would be a countdown to an event
    /// that cannot occur. `Halted` is the honest phase and the page already
    /// draws it loudly.
    const fn halts(self) -> bool {
        matches!(self, Self::NoUniverse)
    }

    /// The sentence, which cannot be assembled without the evidence.
    fn say(self, read: &crate::server::Read, note: &str, probed: &str) -> String {
        match self {
            Self::Complete { instruments } => format!(
                "nothing is missing that any feed can still be asked for, across \
                 {instruments} tracked instrument(s). The store is complete through the \
                 newest finished day; this re-checks once a minute so a new day is \
                 picked up on its own.{note}{probed}"
            ),
            // THE READ'S OWN WORDS, NOT A GUESS AT WHY. `Read::notes` already
            // holds `"groww: UNAVAILABLE — <path>: No such file or directory"`,
            // which names the file and the directory an operator has to fix.
            // The autopilot touched `site.read` at exactly one line before this
            // — inside `tracked_series` — and never asked it anything.
            Self::NoUniverse => {
                let why = read
                    .notes
                    .iter()
                    .filter(|n| n.contains("UNAVAILABLE"))
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(" · ");
                let why = if why.is_empty() {
                    String::from("no vendor named a reason")
                } else {
                    why
                };
                format!(
                    "NOT COMPLETE — NOTHING IS TRACKED. No instrument reached the work \
                     list, so no month was ever a candidate and no feed was ever asked. \
                     This is NOT an up-to-date store: the universe is empty because the \
                     masters did not load. Universe status: {}. {why}. The masters are \
                     read once, at startup, so this cannot resolve itself — fix the \
                     masters directory and restart.{probed}",
                    read.status()
                )
            }
        }
    }
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

    // THE DISK IS RE-CHECKED BEFORE THE FEEDS ARE SURVEYED, so a feed the probe
    // revives is surveyed on this pass rather than the next one.
    let probed = probe_store_halts(site, feeds);

    let (mut reports, chosen) = survey(feeds, series, &censuses, yesterday);

    let Some((slot, unit)) = chosen else {
        let terminal = feeds.iter().all(|f| f.halted.is_some());
        // WHAT "NOTHING WAS CHOSEN" ACTUALLY MEANS, decided from the work list
        // rather than assumed. See `Settled`.
        let settled = Settled::over(series);
        // ONLY WHEN NOTHING IS MISSING. A reconsideration cannot delay forward
        // progress because it is only ever reached from the branch that means
        // there is none to delay, and it is bounded per month per process by
        // `STALL_RETRIES`. See `reconsider`.
        //
        // NOT gated on `settled` — a stall is a month that WAS asked for and
        // did not land, so it is real work whatever the universe looks like
        // now, and swallowing it would trade one silent state for another.
        let retrying = if terminal {
            None
        } else {
            reconsider(feeds, ingest::epoch_secs(std::time::SystemTime::now()))
        };
        let note = stall_note(feeds);
        let carry_on = retrying.is_some();
        // THE REPORTS WERE TAKEN BEFORE THE RECONSIDERATION, so they still hold
        // the frontier and the retry counters as they stood a moment ago. Left
        // alone, the page would carry `"retried":0` on the very stall whose
        // detail says "attempt 1 of 2" — two surfaces disagreeing about the one
        // fact an operator opens the page for, which is the defect class this
        // module keeps catching itself with. `reports` is parallel to `feeds` by
        // index: `survey` pushes exactly one per feed, in order.
        if carry_on {
            for (report, state) in reports.iter_mut().zip(feeds.iter()) {
                report.month = state.frontier.to_string();
                report.attempts = state.attempts;
                report.stalls.clone_from(&state.stalls);
            }
        }
        // BUILT BEFORE THE LOCK IS TAKEN, and built from `site.read`, which the
        // publish closure must not borrow.
        let detail = match retrying {
            Some(saying) => saying,
            None if terminal => format!(
                "every feed is halted. The reasons are below and nothing further is \
                 attempted until they are dealt with.{probed}"
            ),
            None => settled.say(&site.read, &note, &probed),
        };
        site.autopilot.publish(move |status| {
            // AN EMPTY UNIVERSE IS HALTED, NOT IDLE. `idle` beside a countdown
            // is what an operator reads as "it is working"; the masters cannot
            // load without a restart, so there is nothing to wait for.
            //
            // `carry_on` outranks both: a reconsidered month is about to be
            // asked for on the next pass, and a task that is about to do work
            // is not halted. With nothing to carry on to, `!carry_on &&
            // terminal` is exactly the condition this line carried before.
            status.phase = if !carry_on && (terminal || settled.halts()) {
                Phase::Halted
            } else {
                Phase::Idle
            };
            status.detail = detail;
            status.since_unix = ingest::epoch_secs(std::time::SystemTime::now());
            status.feeds = reports;
        });
        // A RECONSIDERED MONTH IS WORK, so the next pass happens at once rather
        // than a minute later. It cannot spin: `reconsider` stamps the stall it
        // moved, so the same month cannot be chosen again for
        // `STALL_RECHECK_SECS`, and every month's allowance is finite.
        return if carry_on { 0 } else { IDLE_POLL_SECS };
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
    // THE JOURNAL'S ANSWER TRAVELS WITH THE BARS. Published beside
    // `bars_stored` because it is a fact about the same tick, and cleared on a
    // tick whose record landed so the page shows the CURRENT state of the file
    // rather than the worst one this process ever saw.
    let journal_error = out.journal_error.clone().unwrap_or_default();
    site.autopilot.publish(move |status| {
        status.bars_stored = status.bars_stored.saturating_add(stored);
        status.journal_error = journal_error;
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
        // THE AUTOPILOT NAMES NO MEMBER, which is how it asks for the whole
        // target. It backfills a set rather than a selection, and an empty set
        // is that request rather than a narrowing of it.
        members: std::collections::HashSet::new(),
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
    // A FRESH CENSUS FOR THE GATE. This function already reads one either side
    // of this call; the pull order must see the same store those reads do, and
    // not the one `Site::load` froze at process start.
    let run =
        crate::server::broker_run(&asked, site, &crate::census::read_all(&site.store_root)).await;
    let took = u64::try_from(started.elapsed().as_micros()).unwrap_or(u64::MAX);
    let source = format!("autopilot {} {}", state.feed.display(), unit.month);

    // ONE JOURNAL RECORD PER TICK, through the same constructors the HTTP
    // receipt uses — so `/audit` cannot tell an autopilot run from a hand-made
    // one except by its source, which is the point.
    let record = if run.reached == 0 {
        let why = run
            .blocked
            .as_ref()
            .map(|b| b.why.clone())
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
    // THE APPEND'S OWN ANSWER, CARRIED OUT OF HERE. This was
    // `let _ignored = ...`: the ONE record per tick, and the one write whose
    // failure nobody could see. `/audit` reads the journal, so a journal that
    // cannot be WRITTEN shows as a quiet page — and this page's own footnote
    // tells the operator that a failure missing from `/audit` "was never
    // written down, and that is a defect in the journal, not in this page".
    // It was a defect in this line. `CLAUDE.md` §4.
    let journal_error = site.journal().append(&record).err();

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
        .as_ref()
        .map(|b| b.why.clone())
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
        journal_error,
    }
}

/// What one control asks for.
///
/// Three words and no fourth. `start` and `resume` are the same act — there is
/// one flag — and they are kept as two words because an operator who has never
/// started this process and one who paused it are asking different questions of
/// the same switch, and the answer says which was asked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// Begin, from the state the process came up in.
    Start,
    /// Stop after the instrument in flight.
    Stop,
    /// Carry on after a stop.
    Resume,
}

impl Action {
    /// Every action, in the order the refusal lists them.
    pub const ALL: [Self; 3] = [Self::Start, Self::Stop, Self::Resume];

    /// The three words, ready to be quoted in a refusal.
    ///
    /// A literal rather than a join over [`Self::ALL`], because it is used
    /// inside `const`-friendly format strings and because a fourth action must
    /// fail to compile here — [`Self::from_slug`] and this sentence going out of
    /// step is precisely how a control ends up refusing a word it accepts.
    pub const WORDS: &'static str = "start, stop, resume";

    /// The value this action carries on the wire.
    #[must_use]
    pub const fn slug(self) -> &'static str {
        match self {
            Self::Start => "start",
            Self::Stop => "stop",
            Self::Resume => "resume",
        }
    }

    /// The action a form field names, or `None` for anything else.
    ///
    /// Exact, lower case, no trimming and no synonyms: `START`, `pause`, `go`
    /// and `1` are all refused by name. The same rule as [`AUTOPILOT_RUN`], for
    /// the same reason — a control that half-matches is how a machine ends up
    /// pulling when somebody thought they had turned it off.
    #[must_use]
    pub fn from_slug(raw: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|a| a.slug() == raw)
    }
}

/// The sentence every refused resume opens with.
///
/// One constant so the claim is made once and can be checked once. Every word
/// of it is a fact about this module: [`FeedState::halted`] is set at three
/// sites — twice in [`FeedState::observe`] and once in [`survey`] — and the
/// `Vec<FeedState>` holding them is a local of [`fly`]. No handler is given a
/// reference to it, so no route can clear a halt however it answers.
///
/// **The last sentence is new and it is load-bearing.** Two of the three halt
/// classes now clear themselves on EVIDENCE — a manifest that loads, a disk that
/// accepts a write — and a refusal that did not say so would send an operator to
/// restart a server that was about to recover by itself. That is not a
/// contradiction of what precedes it: a control still cannot clear a halt, and a
/// measurement is not a control. See [`Halt`].
pub const RESUME_CANNOT_CLEAR: &str = "REFUSED · a resume does not clear a halt. \
     The feed table is a local of the backfill task (fly's own `feeds`), so nothing \
     an HTTP route holds can reach it — a resume sets the pause flag, which a halted \
     feed does not read. Fix what is named below and RESTART the server: every feed \
     is then rebuilt at its floor and the frontier is re-derived from the store, so \
     nothing is lost and nothing is re-fetched. A restart is not always necessary, \
     though it is always sufficient: a halt caused by a manifest that would not load \
     clears itself on the next pass once the file loads, and one caused by a disk \
     that refused a write clears itself when a local write probe succeeds. A dead \
     broker credential clears on neither — CLAUDE.md §8 forbids minting a token and \
     nothing here retries against an unchanged dead value — so for that one, rotating \
     it in AWS Parameter Store and restarting is the whole cure.";

/// Whether a resume can do anything, and what to say when it cannot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Admission {
    /// Nothing is terminal. The resume carries on.
    Clear,
    /// At least one feed is terminal and at least one is not.
    ///
    /// The resume is honoured — the feeds that can still be driven are driven —
    /// and the ones that cannot are named, because a resume that silently left
    /// half the backfill dead reads exactly like one that worked.
    Partial {
        /// One line per terminal feed: its name, then its reason, verbatim.
        halted: Vec<String>,
    },
    /// The resume would change nothing, and this is why.
    Refused {
        /// The reason, naming what has to happen instead.
        why: String,
    },
}

/// Whether a resume would change anything, read from the published status.
///
/// # The three answers, and the state each one is read from
///
/// * **Every reported feed is halted** → refused. The loop is still running and
///   `survey` will choose nothing for as long as that holds, so "resumed" would
///   be a claim about work that cannot start.
/// * **No feed has reported and the phase is `Halted`** → refused. That is
///   [`fly`]'s three pre-loop exits — no live broker, an unusable clock, a rung
///   the store cannot file — and in every one of them the task has *returned*.
///   Nothing is left to read the flag.
/// * **Anything else** → honoured, with any terminal feeds named.
///
/// An empty feed list with a phase that is not `Halted` is the ordinary state
/// before the first round finishes, and it is admitted: the loop is alive and
/// the pause flag is exactly what it reads.
///
/// # Cost
///
/// One uncontended lock and one pass over the feed reports — two of them on
/// this build. Nothing here reads the store.
#[must_use]
pub fn admit_resume(control: &Control) -> Admission {
    control
        .inspect(|status| {
            let halted: Vec<String> = status
                .feeds
                .iter()
                .filter(|feed| !feed.halted.is_empty())
                .map(|feed| format!("{} — {}", feed.feed, feed.halted))
                .collect();
            if status.feeds.is_empty() {
                if status.phase == Phase::Halted {
                    return Admission::Refused {
                        why: format!(
                            "{RESUME_CANNOT_CLEAR} No feed has reported at all and the \
                             autopilot is halted, which is the state it takes when the \
                             backfill task stopped before its first round — it has \
                             returned, so nothing is left to read the flag this would \
                             clear. The reason it stopped, verbatim: {}",
                            status.why()
                        ),
                    };
                }
                return Admission::Clear;
            }
            if halted.len() == status.feeds.len() {
                return Admission::Refused {
                    why: format!(
                        "{RESUME_CANNOT_CLEAR} Every feed is terminal, so there is nothing \
                         left for a resume to drive. The reasons, verbatim: {}",
                        halted.join(" · ")
                    ),
                };
            }
            if halted.is_empty() {
                Admission::Clear
            } else {
                Admission::Partial { halted }
            }
        })
        .unwrap_or_else(|| Admission::Refused {
            why: String::from(
                "REFUSED · the autopilot's status lock is poisoned: a previous publisher \
                 panicked, so whether any feed is halted cannot be established. Nothing was \
                 started against a guess — a resume admitted on an unreadable state is the \
                 fallback CLAUDE.md §4 bans. Restart the server.",
            ),
        })
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
///
/// **This one cannot be refused, which is why it keeps the plain status body.**
/// Its sibling [`resume`] gained a status code and a wrapper because it CAN be
/// refused and the refusal has to travel; a stop has nothing to carry. Stopping
/// a backfill that is already halted is not a lie either — the flag is set, and
/// the phase the payload carries is still the halt's own.
pub async fn pause(
    axum::extract::State(site): axum::extract::State<Loaded>,
) -> ([(axum::http::HeaderName, &'static str); 1], String) {
    stop(&site.autopilot);
    status_json(axum::extract::State(site)).await
}

/// `POST /autopilot/resume` — carry on from wherever the store reaches, or
/// refuse and say why.
///
/// Nothing is replayed and nothing is lost: the next round re-derives the
/// frontier from the manifest, so a month interrupted half way is picked up at
/// the day after its last stored bar.
///
/// # What this used to do, and why it was a lie
///
/// It published `phase = Running` and *"resumed. The next unit is whatever the
/// store is missing"* unconditionally. A feed halts at three sites in
/// [`FeedState::observe`] and [`survey`], and **nothing outside the backfill
/// task can clear that** — the feed table is a local of [`fly`]. So pressing
/// Resume against a halted backfill wrote "resumed" over the halt reason on the
/// one page an operator reads, changed nothing, and the halt reappeared at the
/// next round. `CLAUDE.md` §4: degrade loudly and name the reason, or refuse —
/// never both silently. This refuses, and [`RESUME_CANNOT_CLEAR`] is the
/// reason it gives.
///
/// The status body is unchanged in kind and wrapped in kind: see [`answer`].
pub async fn resume(
    axum::extract::State(site): axum::extract::State<Loaded>,
) -> (
    axum::http::StatusCode,
    [(axum::http::HeaderName, &'static str); 1],
    String,
) {
    act(Action::Resume, &site.autopilot)
}

/// `POST /autopilot/control` — one route for start, stop and resume.
///
/// # Why the path is `/autopilot/control` and not `/autopilot`
///
/// `/autopilot` is the front end's own page, served by the router's fallback.
/// A route registered at that exact path with `post` and nothing else makes
/// `GET /autopilot` answer **405** — `axum`'s method router answers a matched
/// path with an unmatched method itself and never reaches `Router::fallback` —
/// which would take the operator's autopilot page off the air. The sibling
/// shape `/autopilot/pause` and `/autopilot/resume` already use is free of
/// that, so this joins it. Registering the bare path needs a `get` arm that
/// hands the request to the asset fallback, and that is a decision about the
/// front end's front door rather than about this module.
///
/// # The three words, and the one that is a synonym
///
/// `start`, `stop`, `resume`. There is exactly one flag —
/// [`Control::paused`] — so `start` and `resume` do the same thing, and the
/// answer says which word was used rather than pretending they are different
/// states. `stop` is [`pause`]'s own body. Anything else is refused **by name**
/// listing the three; a control that guessed which word an operator meant is a
/// control that can start a vendor conversation nobody asked for.
///
/// # Cost
///
/// One form-body scan bounded by the server's body cap, one atomic store, and
/// one read of the published status under an uncontended lock. Nothing here
/// reads the store.
pub async fn control(
    axum::extract::State(site): axum::extract::State<Loaded>,
    body: String,
) -> (
    axum::http::StatusCode,
    [(axum::http::HeaderName, &'static str); 1],
    String,
) {
    let asked = crate::server::param(&body, "action");
    let Some(action) = Action::from_slug(&asked) else {
        let why = if asked.is_empty() {
            format!(
                "REFUSED · no action was named. This control takes exactly one field, \
                 `action`, and exactly three values: {}. Nothing was changed.",
                Action::WORDS
            )
        } else {
            format!(
                "REFUSED · {asked:?} is not an action this control takes. The three are: \
                 {}. Nothing was changed — a control that guessed which one was meant \
                 could start a vendor conversation nobody asked for.",
                Action::WORDS
            )
        };
        return (
            axum::http::StatusCode::BAD_REQUEST,
            json_headers(),
            answer(&asked, false, &why, &site.autopilot),
        );
    };
    act(action, &site.autopilot)
}

/// The JSON content type both control routes answer with.
fn json_headers() -> [(axum::http::HeaderName, &'static str); 1] {
    [(
        axum::http::header::CONTENT_TYPE,
        "application/json; charset=utf-8",
    )]
}

/// One action, performed and answered for. Shared by [`resume`] and
/// [`control`] so the two routes cannot decide differently about the same
/// backfill.
fn act(
    action: Action,
    control: &Control,
) -> (
    axum::http::StatusCode,
    [(axum::http::HeaderName, &'static str); 1],
    String,
) {
    if action == Action::Stop {
        stop(control);
        // Stopping still works with the lock down — the flag is an atomic — and
        // saying so is not the same as pretending the page will show it.
        let why = if control.inspect(|_| ()).is_some() {
            "stopped. The sweep stops at its next instrument and nothing further is asked \
             of any vendor."
        } else {
            "stopped: the pause flag is set and no vendor is contacted. The status lock is \
             poisoned, so the reason could not be published where the page reads it — what \
             this process is doing cannot be reported until it is restarted."
        };
        return (
            axum::http::StatusCode::OK,
            json_headers(),
            answer(action.slug(), true, why, control),
        );
    }
    match admit_resume(control) {
        // NOTHING IS PUBLISHED ON THIS PATH. The halt's own reason is what the
        // page must keep showing; writing "resumed" over it is the defect this
        // whole admission check exists to remove.
        Admission::Refused { why } => (
            axum::http::StatusCode::CONFLICT,
            json_headers(),
            answer(action.slug(), false, &why, control),
        ),
        Admission::Clear => {
            start(control, &[]);
            (
                axum::http::StatusCode::OK,
                json_headers(),
                answer(
                    action.slug(),
                    true,
                    "started. The next unit is whatever the store is missing, oldest first.",
                    control,
                ),
            )
        }
        Admission::Partial { halted } => {
            start(control, &halted);
            let why = format!(
                "started, and it is NOT a full recovery: {} of the feeds below is terminal \
                 for the life of this process and this did not clear it. {RESUME_CANNOT_CLEAR} \
                 The feeds that carry on are the rest. The reasons, verbatim: {}",
                halted.len(),
                halted.join(" · ")
            );
            (
                axum::http::StatusCode::OK,
                json_headers(),
                answer(action.slug(), true, &why, control),
            )
        }
    }
}

/// The body every control route answers with: what was asked, whether it was
/// done, why, and the status itself.
///
/// The status is **embedded rather than fetched separately** because the two
/// would otherwise be a race — an operator's client reading `/autopilot.json`
/// after a stop can see a round that landed in between and conclude the stop
/// did not take. One payload, one moment.
fn answer(action: &str, accepted: bool, why: &str, control: &Control) -> String {
    format!(
        r#"{{"action":{},"accepted":{},"why":{},"status":{}}}"#,
        render::json_string(action),
        accepted,
        render::json_string(why),
        control.json()
    )
}

/// Stop, and say so where the page reads it.
///
/// One function for both routes, so the sentence an operator is shown cannot
/// depend on which control they pressed.
fn stop(control: &Control) {
    control.pause();
    control.publish(|status| {
        status.phase = Phase::Paused;
        status.detail = String::from(
            "pause requested. The sweep stops at its next instrument; the partial month \
             is refilled automatically on resume, because the resume point is the \
             store's own.",
        );
    });
}

/// Start again, naming any feed that will not come back with it.
///
/// `halted` is empty in the ordinary case. When it is not, the sentence says so
/// — a resume that silently left half the backfill terminal would be the same
/// lie in a smaller size.
fn start(control: &Control, halted: &[String]) {
    control.resume();
    let note = if halted.is_empty() {
        String::new()
    } else {
        format!(
            " {} feed(s) stayed terminal and this resume did not clear them: {}",
            halted.len(),
            halted.join(" · ")
        )
    };
    control.publish(move |status| {
        status.phase = Phase::Running;
        status.detail =
            format!("resumed. The next unit is whatever the store is missing, oldest first.{note}");
    });
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
            journal_error: None,
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
        // TERMINAL, AND FOR THIS CLASS IT STAYS TERMINAL. D-0108 made a census
        // halt and a store halt clearable by evidence; the credential class is
        // deliberately not, because CLAUDE.md §8 forbids minting a token and
        // §4 forbids a retry that hides a permanent fault. Nothing is armed.
        assert!(state.halted.is_some(), "halted is terminal for this feed");
        assert_eq!(state.halt_kind, Some(Halt::Credential));
        assert!(
            state.probe.is_none(),
            "a dead credential must arm no re-check of any kind"
        );
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
            journal_error: None,
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

        let (code, _, resumed) = resume(axum::extract::State(Loaded::clone(&site))).await;
        assert_eq!(code, axum::http::StatusCode::OK);
        assert!(!site.autopilot.is_paused(), "resume clears the flag");
        assert!(resumed.contains("oldest"), "{resumed}");
        // AND THE ANSWER SAYS WHAT IT DID, not only what the backfill is. The
        // wrapper is what carries a refusal's reason on the other path.
        assert!(resumed.contains(r#""accepted":true"#), "{resumed}");
        assert!(resumed.contains(r#""action":"resume""#), "{resumed}");
    }

    // ------------------------------------------------- the one control route

    /// One body, three words, and every other word refused **by name**.
    #[tokio::test]
    async fn the_control_takes_three_words_and_refuses_the_rest() {
        let site = empty_site("control-words");
        for action in Action::ALL {
            let (code, headers, body) = control(
                axum::extract::State(Loaded::clone(&site)),
                format!("action={}", action.slug()),
            )
            .await;
            assert_eq!(code, axum::http::StatusCode::OK, "{body}");
            assert_eq!(headers[0].1, "application/json; charset=utf-8");
            assert!(body.contains(r#""accepted":true"#), "{body}");
            assert!(
                body.contains(&format!(r#""action":"{}""#, action.slug())),
                "the answer names the word that was used: {body}"
            );
            assert_eq!(
                site.autopilot.is_paused(),
                action == Action::Stop,
                "{} moved the flag the wrong way",
                action.slug()
            );
        }

        // A WORD THIS CONTROL DOES NOT TAKE IS REFUSED, AND THE FLAG DOES NOT
        // MOVE. `pause` is the tempting one: it is what the sibling route is
        // called, and guessing it here would stop a backfill on a typo.
        let before = site.autopilot.is_paused();
        for body in ["action=pause", "action=START", "action=", "", "actio=stop"] {
            let (code, _, answer) =
                control(axum::extract::State(Loaded::clone(&site)), body.to_owned()).await;
            assert_eq!(
                code,
                axum::http::StatusCode::BAD_REQUEST,
                "{body} was not refused: {answer}"
            );
            assert!(answer.contains(r#""accepted":false"#), "{answer}");
            assert!(
                answer.contains("start, stop, resume"),
                "the refusal names the three: {answer}"
            );
            assert_eq!(
                site.autopilot.is_paused(),
                before,
                "{body} moved the flag anyway"
            );
        }
    }

    /// **A resume that would change nothing is refused, and the halt reason
    /// survives it.**
    ///
    /// This is the defect the admission check exists for: `resume` used to
    /// publish `phase = Running` and "resumed" over a halted backfill, which
    /// changed nothing, said the opposite, and wiped the one sentence naming
    /// what an operator has to go and fix.
    #[tokio::test]
    async fn a_resume_against_a_wholly_halted_backfill_is_refused_and_keeps_the_reason() {
        let site = empty_site("control-halted");
        site.autopilot.publish(|status| {
            status.phase = Phase::Halted;
            status.detail = String::from("every feed is halted");
            status.feeds = vec![
                FeedReport {
                    feed: String::from("Dhan"),
                    halted: String::from("the broker credential is dead"),
                    ..FeedReport::default()
                },
                FeedReport {
                    feed: String::from("Groww"),
                    halted: String::from("the store refused the same write twice"),
                    ..FeedReport::default()
                },
            ];
        });
        let (code, _, body) = control(
            axum::extract::State(Loaded::clone(&site)),
            String::from("action=resume"),
        )
        .await;
        assert_eq!(code, axum::http::StatusCode::CONFLICT, "{body}");
        assert!(body.contains(r#""accepted":false"#), "{body}");
        assert!(
            body.contains("RESTART the server"),
            "the refusal names what actually clears a halt: {body}"
        );
        assert!(
            body.contains("the broker credential is dead")
                && body.contains("the store refused the same write twice"),
            "and it names every reason, verbatim: {body}"
        );
        // NOTHING WAS PUBLISHED OVER THE HALT, and the flag did not move.
        assert!(body.contains(r#""state":"halted""#), "{body}");
        assert!(
            body.contains("every feed is halted"),
            "the halt's own sentence is still what the page reads: {body}"
        );
    }

    /// One feed terminal and one not: the resume is honoured **and says which
    /// half of the backfill it did not bring back**.
    #[tokio::test]
    async fn a_partly_halted_backfill_resumes_and_names_what_stayed_dead() {
        let site = empty_site("control-partial");
        site.autopilot.pause();
        site.autopilot.publish(|status| {
            status.phase = Phase::Paused;
            status.feeds = vec![
                FeedReport {
                    feed: String::from("Dhan"),
                    halted: String::from("the broker credential is dead"),
                    ..FeedReport::default()
                },
                FeedReport {
                    feed: String::from("Groww"),
                    ..FeedReport::default()
                },
            ];
        });
        let (code, _, body) = control(
            axum::extract::State(Loaded::clone(&site)),
            String::from("action=start"),
        )
        .await;
        assert_eq!(code, axum::http::StatusCode::OK, "{body}");
        assert!(!site.autopilot.is_paused(), "the flag is cleared");
        assert!(body.contains(r#""accepted":true"#), "{body}");
        assert!(
            body.contains("NOT a full recovery"),
            "a partial recovery does not read like a whole one: {body}"
        );
        assert!(
            body.contains("Dhan — the broker credential is dead"),
            "and it names the feed that stayed terminal: {body}"
        );
        assert!(
            body.contains("stayed terminal and this resume did not clear them"),
            "the published detail says it too, not only the answer: {body}"
        );
    }

    /// A halted autopilot that never surveyed anything is refused too: those
    /// are `fly`'s three pre-loop exits, and in every one of them the task has
    /// returned.
    #[tokio::test]
    async fn a_resume_before_the_first_round_of_a_halted_task_is_refused() {
        let site = empty_site("control-preloop");
        site.autopilot.publish(|status| {
            status.phase = Phase::Halted;
            status.detail = String::from("this process may not reach a live broker");
        });
        assert_eq!(
            admit_resume(&site.autopilot),
            Admission::Refused {
                why: format!(
                    "{RESUME_CANNOT_CLEAR} No feed has reported at all and the autopilot \
                     is halted, which is the state it takes when the backfill task \
                     stopped before its first round — it has returned, so nothing is \
                     left to read the flag this would clear. The reason it stopped, \
                     verbatim: this process may not reach a live broker"
                ),
            }
        );
        let (code, _, body) = resume(axum::extract::State(Loaded::clone(&site))).await;
        assert_eq!(code, axum::http::StatusCode::CONFLICT, "{body}");
        assert!(
            body.contains("may not reach a live broker"),
            "the reason it stopped is what the refusal carries: {body}"
        );
    }

    /// The ordinary pre-round state — no feed has reported and nothing is
    /// halted — is admitted. The loop is alive and the flag is what it reads.
    #[test]
    fn a_resume_before_the_first_round_of_a_live_task_is_admitted() {
        let control = Control::new();
        assert_eq!(admit_resume(&control), Admission::Clear);
        control.publish(|status| status.phase = Phase::Backoff);
        assert_eq!(admit_resume(&control), Admission::Clear);
        // AND SO IS THE ORDINARY CASE: feeds that have reported and none of
        // them terminal. This is the arm every working resume takes.
        control.publish(|status| {
            status.feeds = vec![
                FeedReport {
                    feed: String::from("Dhan"),
                    ..FeedReport::default()
                },
                FeedReport {
                    feed: String::from("Groww"),
                    ..FeedReport::default()
                },
            ];
        });
        assert_eq!(admit_resume(&control), Admission::Clear);
    }

    /// **An unreadable state is refused rather than guessed.** A poisoned lock
    /// cannot say whether a feed is halted, and "probably not" is the fallback
    /// `CLAUDE.md` §4 bans.
    #[tokio::test]
    async fn a_control_over_a_poisoned_status_refuses_to_start_and_still_stops() {
        let site = empty_site("control-poison");
        let poisoned = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            site.autopilot.publish(|_| panic!("a publisher panicked"));
        }));
        assert!(poisoned.is_err(), "the panic has to reach the lock");

        // THE PREMISE IS "THE FLAG DID NOT MOVE", NOT "THE FLAG IS SET".
        //
        // This used to assert `is_paused()` outright, on the grounds that
        // `Control::serving` came up paused. That is no longer the boot default
        // — see `flies_on_startup` — and asserting the boot default here was
        // asserting the wrong thing anyway: what a refused resume owes is that
        // it changed nothing, whichever state it found. Read before, compare
        // after, and the test now says what it means on either default.
        let before = site.autopilot.is_paused();
        let (code, _, body) = control(
            axum::extract::State(Loaded::clone(&site)),
            String::from("action=resume"),
        )
        .await;
        assert_eq!(code, axum::http::StatusCode::CONFLICT, "{body}");
        assert!(body.contains("poisoned"), "{body}");
        assert_eq!(
            site.autopilot.is_paused(),
            before,
            "a refused resume must not have moved the flag anyway"
        );

        // STOPPING STILL WORKS: the flag is an atomic and the safe direction is
        // never blocked by a status nobody can read. It says the status could
        // not be republished rather than pretending it was.
        let (code, _, body) = control(
            axum::extract::State(Loaded::clone(&site)),
            String::from("action=stop"),
        )
        .await;
        assert_eq!(code, axum::http::StatusCode::OK, "{body}");
        assert!(site.autopilot.is_paused(), "the stop took");
        assert!(body.contains("status lock is poisoned"), "{body}");
    }

    /// The seat is reported as the fact it is, and it is not free while
    /// something holds it.
    #[test]
    fn the_seat_is_readable_without_taking_it() {
        let control = Control::new();
        assert!(!control.seat_held());
        {
            let _seat = control.take_seat().expect("a free seat");
            assert!(control.seat_held());
        }
        assert!(!control.seat_held(), "the seat is released on drop");
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
            journal_error: None,
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
        //
        // ASSERTED AGAINST THE FEED ROWS IN THE SAME ANSWER, never against a
        // literal. It was `"cursor":"2020-01"` — Groww's fixed floor, which was
        // the oldest until a feed claiming a ROLLING ten years joined the table
        // and reached further back. A rolling floor moves with the clock, so a
        // literal here would assert a different thing every month it ran, which
        // is the failure `pull::vendor`'s own floor test names in as many
        // words.
        //
        // The property that actually matters is unchanged and is what is
        // checked: the cursor is the OLDEST month any feed is behind on.
        let months: Vec<&str> = json
            .match_indices(r#""month":""#)
            .filter_map(|(at, key)| {
                let rest = json.get(at.saturating_add(key.len())..)?;
                rest.get(..rest.find('"')?)
            })
            .collect();
        let oldest = months.iter().min().expect("at least one feed row");
        assert!(
            json.contains(&format!(r#""cursor":"{oldest}""#)),
            "the cursor is the oldest month any feed is behind on, and the feed \
             rows in this same answer are {months:?}: {json}"
        );
        // THE TICK WAS JOURNALLED. One record per tick, in the store's own
        // journal, so a restart can read what this process did.
        assert!(
            site.journal().path.exists(),
            "the run journal was written at {}",
            site.journal().path.display()
        );
    }

    /// **A tick whose record cannot be journalled says so on `/autopilot.json`.**
    ///
    /// # What this holds up
    ///
    /// `tick` builds exactly one `Record` per pass and appended it as
    /// `let _ignored = site.journal().append(&record);`. On a read-only store
    /// root, a full disk, or an `audit` path that is a file — the state this
    /// test creates, and the same one `emitted.rs` drives on purpose — every
    /// record this process produced went nowhere and no surface said so. The
    /// page beside it tells the operator the opposite in as many words: *"A
    /// failure that appears here and not there is a failure that was never
    /// written down, and that is a defect in the journal, not in this page."*
    ///
    /// Before the fix this asserted nothing, because there was no field: the
    /// payload carried `journal` — the PATH — and a path is not a proof that
    /// anything reached it.
    #[tokio::test]
    async fn a_tick_that_cannot_be_journalled_carries_the_reason_to_the_page() {
        let site = empty_site("journal-refused");
        // A FILE WHERE THE `audit` DIRECTORY HAS TO BE, so `create_dir_all`
        // refuses and the append refuses with it. Nothing is mocked: this is
        // the shipped `Journal::append` meeting a disk it cannot use.
        std::fs::write(site.store_root.join("audit"), b"not a directory")
            .expect("a file in the way");
        let yesterday = yesterday_ist(std::time::SystemTime::now()).expect("a usable clock");
        let axis = [series("NIFTY")];
        let mut feeds = drivable(yesterday);

        let _waited = round(&site, &mut feeds, &axis, pull::vendor::Granularity::Minute1).await;

        let json = site.autopilot.json();
        assert!(
            json.contains(r#""journal_error":"#),
            "the payload carries the field: {json}"
        );
        assert!(
            json.contains("audit directory"),
            "and it carries the journal's own reason, verbatim: {json}"
        );
        assert!(
            !site.journal().path.exists(),
            "nothing was recorded, which is the fact the page now states"
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

    /// **The grace window is a real window on a site that is NOT paused, which
    /// is what the new boot default made load-bearing.**
    ///
    /// Under the old default a served `Control` came up paused, so `grace`
    /// returned at once and "nothing is contacted before somebody says so" was
    /// guaranteed by the pause rather than by the countdown. Flying by default
    /// moves the whole weight of that guarantee onto this window, so it is
    /// asserted rather than assumed.
    ///
    /// It is also what keeps `cargo test` off the vendor on the one test that
    /// drives `run` end to end: `server::run_in` spawns `fly` and calls
    /// `flying.abort()` the moment `serve` returns, which for an
    /// already-resolved shutdown future is microseconds — three orders of
    /// magnitude inside this window. Reverting `grace` to return immediately
    /// would remove that margin silently, and this fails instead.
    #[tokio::test]
    async fn an_unpaused_grace_window_really_waits_before_anything_is_contacted() {
        let site = empty_site("grace-flying");
        site.autopilot.resume();
        assert!(
            !site.autopilot.is_paused(),
            "the premise: this is the state pressing Run now produces"
        );
        // The floor on GRACE_SECS is asserted where it is defined, at compile
        // time — a runtime assertion on a constant proves nothing a reader
        // could not read.
        let raced = tokio::time::timeout(std::time::Duration::from_millis(300), grace(&site)).await;
        assert!(
            raced.is_err(),
            "grace returned inside 300ms on an unpaused site — the twenty-second \
             chance to say no is the ONLY thing standing between pressing Run and a \
             socket, now that the default flies"
        );
        // AND IT SAYS SO WHILE IT WAITS, with a countdown rather than a silence.
        let json = site.autopilot.json();
        assert!(json.contains(r#""state":"starting""#), "{json}");
        assert!(json.contains("Press Pause to stop it"), "{json}");
        // NOTHING WAS FETCHED AND NOTHING WAS JOURNALLED.
        assert!(
            !site.journal().path.exists(),
            "the grace window contacted something"
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
            journal_error: None,
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
            journal_error: None,
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
            journal_error: None,
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
            journal_error: None,
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

    // ------------------------------------------------------- the boot default

    /// **Pressing Run pulls NOTHING. Exactly one spelling lets it fly.**
    ///
    /// **This test was reversed, and so was the behaviour it pins.** It used to
    /// assert that an unset variable FLIES, citing the owner's first blocker: a
    /// Run button that reliably did nothing. That default then did something
    /// worse than nothing — starting the server was itself enough to begin
    /// fetching from a vendor, and one session wrote 23,695 `pull.run` events
    /// with nobody having clicked anything.
    ///
    /// The owner's instruction is now explicit: **no data is pulled unless they
    /// ask for it.** Fetching spends a shared vendor quota and appends to a
    /// store, so it is the irreversible half of this program, and a default must
    /// sit on the safe side of an irreversible action. A Run button that starts a
    /// server and waits is a working Run button; a Run button that quietly
    /// spends quota is not.
    ///
    /// Restoring the old polarity fails the first assertion. Loosening the
    /// comparison to a prefix, a case-fold or a trim fails the near-miss block —
    /// which now protects the direction that matters, because a near miss that
    /// FLIES spends money the operator never authorised.
    #[test]
    fn the_boot_default_pulls_nothing_and_only_the_exact_word_run_lets_it_fly() {
        use std::ffi::OsStr;

        // ABSENCE STAYS ON THE GROUND. This is the tracked Run configuration's
        // own state: press Run, get a server, fetch nothing.
        assert!(
            stays_paused_from(None),
            "an unset {AUTOPILOT_ENV} must NOT fly — pressing Run must never spend \
             a vendor quota the operator did not ask to spend"
        );

        // THE ONE SPELLING THAT LETS IT FLY.
        assert!(!stays_paused_from(Some(OsStr::new(AUTOPILOT_RUN))));
        assert_eq!(AUTOPILOT_RUN, "run", "the opt-IN is one documented word");

        // EVERY NEAR MISS STAYS GROUNDED, INCLUDING THE TEMPTING ONES. Under the
        // old polarity these all flew; each one is now a request that was not
        // made clearly enough to spend money on.
        for spelling in [
            "RUN",
            "Run",
            "runs",
            " run",
            "run ",
            "run\n",
            "start",
            "true",
            "1",
            "yes",
            "on",
            "",
            "pause",
            AUTOPILOT_PAUSE,
        ] {
            assert!(
                stays_paused_from(Some(OsStr::new(spelling))),
                "{spelling:?} is not the word `run` and must therefore stay on the ground"
            );
        }

        // `pause` IS STILL UNDERSTOOD AS A GROUNDING VALUE, so a machine that
        // already exports it keeps meaning exactly what it meant.
        assert!(stays_paused_from(Some(OsStr::new(AUTOPILOT_PAUSE))));

        // AND THE ENVIRONMENT READER AGREES WITH THE PURE HALF. Whatever this
        // machine's variable says, the two answers are one answer — the split
        // exists so the arms are testable, not so they can disagree.
        assert_eq!(
            flies_on_startup(),
            !stays_paused_from(std::env::var_os(AUTOPILOT_ENV).as_deref()),
            "the environment reader and the pure decision must not diverge"
        );
    }

    // -------------------------------------------------- the credential gate

    /// **One instrument's 401 does not kill a whole feed.**
    ///
    /// A dead token fails EVERY instrument. A sweep that reached 772 of 773 and
    /// saw one credential-shaped refusal has an entitlement gap on one symbol,
    /// which is a fact about the account's contract and not about the token —
    /// and it used to halt the feed for the life of the process, because the
    /// reason carried a credential marker.
    ///
    /// Reverting [`credential_is_feedwide`] to `true` puts the halt back and
    /// fails this.
    #[test]
    fn a_credential_reason_from_a_sweep_that_reached_somebody_backs_off_instead_of_halting() {
        let mut state = FeedState::new(
            pull::vendor::Feed::Groww,
            brutex_core::vendor::Vendor::Groww,
            month(2020, 1),
        );
        let partial = TickOutcome {
            attempted: 773,
            reached: 772,
            stored: 0,
            reason: Some("SOMEBOND — refused with status 401".to_owned()),
            complete: false,
            stopped: false,
            journal_error: None,
        };
        assert_eq!(
            classify("SOMEBOND — refused with status 401"),
            Trouble::Credential,
            "the premise: the reason IS credential-shaped"
        );
        assert!(!credential_is_feedwide(&partial));
        for expected in 1..=2u8 {
            assert!(
                matches!(state.observe(&partial), Next::Wait { .. }),
                "a partial credential failure is a transport failure, not a halt"
            );
            assert_eq!(state.attempts, expected);
        }
        assert!(
            state.halted.is_none() && state.halt_kind.is_none(),
            "one instrument must not make a feed terminal: {:?}",
            state.halted
        );
        // AND IT IS STILL BOUNDED. The third attempt stalls the month, which is
        // visible and is not terminal for the feed.
        assert!(matches!(state.observe(&partial), Next::Stall { .. }));
        assert!(state.halted.is_none());

        // THE FEED-WIDE CASE IS UNCHANGED: nobody answered, so it is the token.
        let dead = TickOutcome {
            reached: 0,
            ..partial
        };
        assert!(credential_is_feedwide(&dead));
        let mut token = FeedState::new(
            pull::vendor::Feed::Groww,
            brutex_core::vendor::Vendor::Groww,
            month(2020, 1),
        );
        assert_eq!(token.observe(&dead), Next::Retry, "the one §8 re-read");
        let Next::Halt { reason } = token.observe(&dead) else {
            panic!("a feed-wide dead credential must still halt");
        };
        assert!(reason.contains("never mints"), "{reason}");
        assert_eq!(token.halt_kind, Some(Halt::Credential));
        assert!(
            token.probe.is_none(),
            "CLAUDE.md §8: a dead credential arms NOTHING here. A timer-driven retry \
             against an unchanged dead value is the auto-retry §4 bans."
        );

        // A RUN BLOCKED BEFORE IT ATTEMPTED ANYTHING IS AMBIGUOUS, and the
        // ambiguous case halts — the direction that costs nothing outside this
        // machine.
        let blocked = TickOutcome {
            attempted: 0,
            reached: 0,
            stored: 0,
            reason: Some("the credential configuration at ~/.brutex is not usable".to_owned()),
            complete: false,
            stopped: false,
            journal_error: None,
        };
        assert!(credential_is_feedwide(&blocked));
    }

    // ------------------------------------------------------ census probation

    /// A census in whichever state, for a vendor.
    fn held_census(vendor: brutex_core::vendor::Vendor) -> VendorCensus {
        let manifest =
            pull::manifest::Manifest::open(vendor, &[], &[]).expect("a genesis manifest");
        census_of(
            vendor,
            Census::Held {
                manifest: Box::new(manifest),
            },
        )
    }

    /// **A manifest an operator repaired clears its own halt, and no restart is
    /// needed for it.**
    ///
    /// `round` re-reads every census once per pass already, so before this the
    /// process spent a minute of O(entries) reading and CRC-verifying ~248,000
    /// entries, learned the answer, and threw it away — for ever. Reverting the
    /// clear at the top of [`survey`] makes the second half of this fail.
    ///
    /// The third block is the load-bearing narrow rule: **`Census::Absent` does
    /// NOT revive a feed.** Absent means the store reports it holds nothing, and
    /// a feed revived on that reading would re-offer months whose bar files are
    /// still on disk — offers `BarFile::append` refuses wholesale, because the
    /// overlap is not a suffix.
    #[test]
    fn a_repaired_manifest_clears_its_own_halt_and_an_absent_one_does_not() {
        let yesterday = day(2026, 8, 6);
        let axis = [series("NIFTY")];
        let mut feeds = drivable(yesterday);
        let broken: Vec<VendorCensus> = feeds
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

        let (_, chosen) = survey(&mut feeds, &axis, &broken, yesterday);
        assert!(
            chosen.is_none(),
            "the premise: nothing is fetched on a guess"
        );
        assert!(
            feeds
                .iter()
                .all(|f| f.halt_kind == Some(Halt::Census) && f.halted.is_some()),
            "the premise: every feed is terminal on its census"
        );

        // AN ABSENT MANIFEST IS NOT A REPAIRED ONE. Nothing revives.
        let absent: Vec<VendorCensus> = feeds
            .iter()
            .map(|f| census_of(f.vendor, Census::Absent))
            .collect();
        let (_, still) = survey(&mut feeds, &axis, &absent, yesterday);
        assert!(still.is_none(), "an absent census must not restart a feed");
        assert!(
            feeds.iter().all(|f| f.halted.is_some()),
            "a census that reports NOTHING HELD is not evidence the fault is gone"
        );
        assert!(!manifest_loads(&absent, feeds[0].vendor));

        // A MANIFEST THAT LOADS IS. The feed carries on, on this pass.
        let repaired: Vec<VendorCensus> = feeds.iter().map(|f| held_census(f.vendor)).collect();
        assert!(manifest_loads(&repaired, feeds[0].vendor));
        let (reports, back) = survey(&mut feeds, &axis, &repaired, yesterday);
        assert!(
            back.is_some(),
            "a repaired manifest puts the feed back on the ladder without a restart"
        );
        assert!(
            feeds
                .iter()
                .all(|f| f.halted.is_none() && f.halt_kind.is_none()),
            "the halt is cleared, not merely skipped"
        );
        assert!(
            reports
                .iter()
                .all(|r| r.halted.is_empty() && r.last_reason.contains("manifest loads again")),
            "and the page says WHEN and WHY it cleared: {:?}",
            reports.iter().map(|r| &r.last_reason).collect::<Vec<_>>()
        );
    }

    // ------------------------------------------------------- store probation

    /// **The store halt is cleared by a successful write and by nothing else,
    /// and the allowance is bounded.**
    ///
    /// The distinguishing property, and the reason this class may be re-checked
    /// at all while the credential class may not: the system can MEASURE whether
    /// the condition still holds, locally, with no vendor, no credential, no
    /// rate budget and no bar request. Time passing is not evidence.
    ///
    /// Reverting [`store_due`]'s `made >= STORE_PROBES` arm to fall through
    /// makes the probe unbounded and fails the exhaustion block, which is the
    /// half `CLAUDE.md` §4 is about: when it gives up it says so and stays
    /// stuck.
    #[test]
    fn the_store_probe_is_bounded_measures_the_disk_and_says_so_when_it_is_spent() {
        // THE SCHEDULE: doubling, capped, and never past the ceiling.
        assert_eq!(probe_secs(0), STORE_PROBE_FLOOR_SECS);
        assert_eq!(probe_secs(1), STORE_PROBE_FLOOR_SECS * 2);
        assert_eq!(probe_secs(5), 1920);
        assert_eq!(probe_secs(6), STORE_PROBE_CEILING_SECS);
        assert_eq!(probe_secs(u32::MAX), STORE_PROBE_CEILING_SECS);
        for made in 0..64 {
            assert!(probe_secs(made) <= STORE_PROBE_CEILING_SECS);
        }
        // THE DOCUMENTED SPAN, READ BACK FROM THE FUNCTION rather than trusted
        // from the comment: seven gaps between eight probes.
        let span: u64 = (0..STORE_PROBES.saturating_sub(1)).map(probe_secs).sum();
        assert_eq!(span, 7_380, "eight probes span two hours and three minutes");

        // NO PROBE ARMED IS NOT "PROBE NOW". A caller must not be able to read
        // an absent allowance as a fresh one.
        let Due::Spent { saying } = store_due(None, 0) else {
            panic!("an unarmed feed must not answer Now");
        };
        assert!(saying.contains("no write probe is armed"), "{saying}");

        // NOT YET DUE SAYS WHEN, rather than probing early.
        assert_eq!(
            store_due(
                Some(&Probe {
                    made: 3,
                    due_unix: 1_000
                }),
                999
            ),
            Due::Later { due_unix: 1_000 }
        );

        // AND THE ALLOWANCE IS EXHAUSTED, one probe at a time, to the bound.
        let mut probe = Probe {
            made: 0,
            due_unix: 0,
        };
        let mut now = 0i64;
        for expected in 0..STORE_PROBES {
            let Due::Now { made } = store_due(Some(&probe), now) else {
                panic!("probe {expected} of {STORE_PROBES} must be due, at {now}");
            };
            assert_eq!(made, expected);
            let wait = probe_secs(made);
            now = now.saturating_add(i64::try_from(wait).expect("a small wait"));
            probe = Probe {
                made: made.saturating_add(1),
                due_unix: now,
            };
        }
        // THE (STORE_PROBES + 1)th IS REFUSED, AND LOUDLY.
        let Due::Spent { saying } = store_due(Some(&probe), now.saturating_add(1_000_000)) else {
            panic!("the {STORE_PROBES}-probe allowance must be spent, not renewed by time");
        };
        assert!(saying.contains("ALLOWANCE IS NOW SPENT"), "{saying}");
        assert!(
            saying.contains("nothing further is written"),
            "it names what it has stopped doing: {saying}"
        );
        assert_eq!(probe.made, STORE_PROBES, "exactly the bound, never past it");
    }

    /// The probe is a real measurement of a real disk: it succeeds on a
    /// writable root and reports the host's own refusal on one that cannot
    /// exist.
    ///
    /// It also leaves nothing behind, which is `CLAUDE.md` §3 rule 5 — a
    /// recovery that re-runs must not accumulate.
    #[test]
    fn the_write_probe_measures_the_disk_and_leaves_nothing_behind() {
        let root = crate::scratch::path("autopilot-probe-ok");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("a writable root");
        let before = std::fs::read_dir(&root).expect("the root exists").count();
        for _ in 0..3 {
            store_writable(&root, "groww").expect("a writable root accepts a few bytes");
        }
        assert_eq!(
            std::fs::read_dir(&root).expect("the root exists").count(),
            before,
            "three probes left three files behind — the probe must remove its own"
        );

        // A ROOT THAT CANNOT EXIST, because its parent is a regular file. The
        // host's own words travel, so `permission denied` on somebody else's
        // volume reads as itself rather than as "the disk is fine".
        let blocker = crate::scratch::path("autopilot-probe-blocker");
        let _ = std::fs::remove_dir_all(&blocker);
        std::fs::create_dir_all(&blocker).expect("a scratch dir");
        let file = blocker.join("not-a-directory");
        std::fs::write(&file, b"x").expect("a regular file");
        let why = store_writable(&file.join("under"), "dhan")
            .expect_err("a path under a regular file cannot be created");
        assert!(
            why.contains("not-a-directory"),
            "the refusal names the path: {why}"
        );
        assert!(why.contains("refused a write probe"), "{why}");

        // A ROOT THAT EXISTS BUT WHOSE PROBE PATH CANNOT BE OPENED. The root is
        // fine and `create_dir_all` succeeds, so this drives the SECOND step
        // rather than the first — the two failures a test can produce without a
        // full disk, and the reason `probe_io` funnels all four steps through
        // one arm rather than formatting five.
        let occupied = crate::scratch::path("autopilot-probe-occupied");
        let _ = std::fs::remove_dir_all(&occupied);
        std::fs::create_dir_all(occupied.join(format!("{STORE_PROBE_PREFIX}groww")))
            .expect("a directory sitting exactly where the probe file goes");
        let why =
            store_writable(&occupied, "groww").expect_err("a directory cannot be opened as a file");
        assert!(
            why.contains(STORE_PROBE_PREFIX),
            "the refusal names the probe path: {why}"
        );

        // AND A FEED WHOSE PROBE PATH IS FREE IS UNAFFECTED BY ITS NEIGHBOUR'S:
        // the file name carries the vendor precisely so two feeds probing in one
        // pass cannot remove each other's file and read that as a failure.
        store_writable(&occupied, "dhan").expect("a different vendor, a different path");
    }

    /// **A store-halted feed probes its own disk inside a real round, and a
    /// disk that accepts the write puts it back on the ladder — with no
    /// restart, no route, and nothing asked of any vendor.**
    ///
    /// The series axis is deliberately empty so that `survey` chooses nothing
    /// and the round reaches its idle branch: the assertion is about the probe,
    /// not about a fetch, and nothing here may contact anything.
    #[tokio::test]
    async fn a_store_halted_feed_probes_the_disk_inside_a_round_and_comes_back() {
        let site = empty_site("store-probation");
        let yesterday = yesterday_ist(std::time::SystemTime::now()).expect("a usable clock");
        let mut feeds = drivable(yesterday);
        let disk = TickOutcome {
            attempted: 773,
            reached: 773,
            stored: 0,
            reason: Some(String::from("NIFTY — disk full writing /store/x.bin")),
            complete: false,
            stopped: false,
            journal_error: None,
        };
        let first = feeds.first_mut().expect("a drivable feed");
        assert!(matches!(first.observe(&disk), Next::Wait { .. }));
        let Next::Halt { .. } = first.observe(&disk) else {
            panic!("the same store refusal twice must halt");
        };
        assert_eq!(first.halt_kind, Some(Halt::Store));
        assert_eq!(
            first.probe,
            Some(Probe {
                made: 0,
                due_unix: 0
            }),
            "a store halt arms a probe due immediately"
        );

        let waited = round(&site, &mut feeds, &[], pull::vendor::Granularity::Minute1).await;
        assert_eq!(
            waited, IDLE_POLL_SECS,
            "nothing was fetched: the axis is empty"
        );
        let back = feeds.first().expect("a drivable feed");
        assert!(
            back.halted.is_none() && back.halt_kind.is_none() && back.probe.is_none(),
            "the scratch store root accepts a write, so the halt is cleared: {:?}",
            back.halted
        );
        let json = site.autopilot.json();
        assert!(json.contains("store halt is CLEARED"), "{json}");
        assert!(
            json.contains("Nothing was asked of any vendor"),
            "it names what the recovery did NOT do: {json}"
        );
    }

    // ------------------------------------------------- stall reconsideration

    /// A feed carrying one stalled month, stamped as though it happened
    /// `age` seconds ago.
    fn stalled(month_at: YearMonth, retried: u8, at_unix: i64) -> FeedState {
        FeedState {
            stalls: vec![Stall {
                month: month_at,
                attempts: MAX_MONTH_ATTEMPTS,
                reason: String::from("attempted 3 times and never completed: timed out"),
                retried,
                at_unix,
            }],
            ..FeedState::new(
                pull::vendor::Feed::Groww,
                brutex_core::vendor::Vendor::Groww,
                month(2026, 1),
            )
        }
    }

    /// **A stalled month is reconsidered at most [`STALL_RETRIES`] times per
    /// process, and then never again — and the page says which of the two it
    /// is.**
    ///
    /// This is the bound made checkable. The loop is exhausted here rather than
    /// described: after `STALL_RETRIES` reconsiderations the function answers
    /// `None` however long is waited, which is `CLAUDE.md` §4's "give up loudly
    /// and stay stuck" rather than an unbounded retry that would be a quota
    /// attack on the owner.
    ///
    /// Reverting the `stall.retried >= STALL_RETRIES` guard makes the final
    /// block loop for ever and fail.
    #[test]
    fn a_stalled_month_is_reconsidered_twice_at_the_earliest_and_then_never_again() {
        let mut feeds = vec![stalled(month(2021, 3), 0, 0)];

        // FIRST SIGHTING STAMPS AND DOES NOT RETRY. A month that stalled this
        // second must not be re-asked this second.
        let now = 1_000_000i64;
        assert_eq!(
            reconsider(&mut feeds, now),
            None,
            "the pass that first sees a stall stamps it and waits"
        );
        assert_eq!(feeds[0].stalls[0].at_unix, now);
        assert_eq!(feeds[0].stalls[0].retried, 0);

        // AND IT STAYS WAITING UNTIL THE FULL INTERVAL HAS PASSED.
        assert_eq!(
            reconsider(&mut feeds, now + STALL_RECHECK_SECS - 1),
            None,
            "one second short of the interval is still short of it"
        );

        // THE ALLOWANCE, EXHAUSTED ONE RECONSIDERATION AT A TIME.
        let mut clock = now;
        for expected in 1..=STALL_RETRIES {
            clock = clock.saturating_add(STALL_RECHECK_SECS);
            let saying = reconsider(&mut feeds, clock)
                .unwrap_or_else(|| panic!("reconsideration {expected} was refused at {clock}"));
            assert!(
                saying.contains("2021-03") && saying.contains("being reconsidered"),
                "it names the month and what it is doing: {saying}"
            );
            assert!(
                saying.contains("timed out"),
                "and it names why it stalled, verbatim: {saying}"
            );
            assert_eq!(feeds[0].stalls[0].retried, expected);
            assert_eq!(
                feeds[0].frontier,
                month(2021, 3),
                "the frontier went back to the stalled month, which is the whole point"
            );
            assert_eq!(feeds[0].attempts, 0, "the month starts its attempts afresh");
            // THE STALL STAYS ON THE LIST. It leaves only when the month
            // actually completes.
            assert_eq!(feeds[0].stalls.len(), 1);
            feeds[0].frontier = month(2026, 1);
        }

        // THE BOUND. However long is waited, it is never asked for again.
        for extra in [1, 10, 1_000, 10_000_000i64] {
            assert_eq!(
                reconsider(&mut feeds, clock.saturating_add(extra * STALL_RECHECK_SECS)),
                None,
                "the allowance is spent and time does not renew it"
            );
        }
        assert_eq!(feeds[0].stalls[0].retried, STALL_RETRIES);

        // AND IT SAYS SO, rather than leaving a list that reads as pending.
        let note = stall_note(&feeds);
        assert!(note.contains("allowance is SPENT"), "{note}");
        assert!(
            note.contains("will not ask for those again"),
            "giving up is stated out loud: {note}"
        );

        // THE WORST CASE, AS THE NUMBER IT IS.
        assert_eq!(
            u32::from(MAX_MONTH_ATTEMPTS) * (1 + u32::from(STALL_RETRIES)),
            9,
            "nine attempts per stalled month per process, and the bound is arithmetic"
        );
    }

    /// The oldest stalled month is the one reconsidered, a terminal feed is
    /// never reconsidered at all, and a list with nothing on it says nothing.
    #[test]
    fn reconsideration_takes_the_oldest_month_and_skips_a_terminal_feed() {
        let old = 1_000_000i64;
        let now = old.saturating_add(STALL_RECHECK_SECS * 2);

        // NOTHING STALLED IS NOT A REASSURING SENTENCE. It is silence.
        let mut clean = drivable(day(2026, 8, 6));
        assert_eq!(reconsider(&mut clean, now), None);
        assert_eq!(
            stall_note(&clean),
            "",
            "a page must not report an empty list"
        );

        // THE OLDEST, ACROSS FEEDS — the same rule the ladder itself climbs by.
        let mut feeds = vec![
            stalled(month(2023, 7), 0, old),
            stalled(month(2021, 3), 0, old),
        ];
        let saying = reconsider(&mut feeds, now).expect("both are due");
        assert!(
            saying.contains("2021-03"),
            "the oldest, not the newest: {saying}"
        );
        assert_eq!(feeds[1].stalls[0].retried, 1);
        assert_eq!(feeds[0].stalls[0].retried, 0, "one per pass, never two");

        // A TERMINAL FEED IS NOT DRIVEN. Reconsidering a month on a feed that
        // cannot be asked for anything would be a claim about work that cannot
        // start.
        let mut dead = vec![stalled(month(2021, 3), 0, old)];
        dead[0].halt(
            Halt::Credential,
            String::from("the broker credential is dead"),
        );
        assert_eq!(
            reconsider(&mut dead, now),
            None,
            "a halted feed owes no reconsideration — there is nothing to drive"
        );
        assert_eq!(dead[0].stalls[0].retried, 0);

        // AND THE NOTE SEPARATES THE TWO STATES THAT LOOK ALIKE ON A LIST.
        let mixed = vec![
            stalled(month(2021, 3), STALL_RETRIES, old),
            stalled(month(2023, 7), 0, old),
        ];
        let note = stall_note(&mixed);
        assert!(note.contains("2 stalled month(s)"), "{note}");
        assert!(note.contains("1 still to be reconsidered"), "{note}");
        assert!(note.contains("1 whose allowance is SPENT"), "{note}");
    }

    /// **THE COMPLETENESS CLAIM CANNOT BE BUILT WITHOUT THE UNIVERSE THAT
    /// JUSTIFIES IT.**
    ///
    /// The whole of the fix is that [`Settled::Complete`] holds a
    /// `NonZeroUsize` and [`Settled::over`] is the only way in, so there is no
    /// value of `series` that produces the word "complete" over an empty work
    /// list. Asserted three ways: the mapping refuses zero, the sentence for
    /// zero contains none of the old claim, and the sentence for one carries
    /// the count that justifies it.
    #[test]
    fn the_completeness_claim_cannot_be_built_without_the_universe_behind_it() {
        assert_eq!(Settled::over(&[]), Settled::NoUniverse);
        assert!(
            Settled::over(&[]).halts(),
            "an empty universe cannot change while the process runs, so idling \
             on it is a countdown to an event that cannot occur"
        );

        let one = [series("NIFTY")];
        let Settled::Complete { instruments } = Settled::over(&one) else {
            panic!("one tracked instrument is a universe");
        };
        assert_eq!(instruments.get(), 1);
        assert!(!Settled::over(&one).halts());

        // AND THE SENTENCES. The evidence travels into the claim, and the
        // claim is absent where the evidence is.
        let site = empty_site("settled-say");
        let complete = Settled::over(&one).say(&site.read, "", "");
        assert!(complete.contains("The store is complete"), "{complete}");
        assert!(
            complete.contains("1 tracked instrument(s)"),
            "the count that justifies it travels with it: {complete}"
        );

        let empty = Settled::over(&[]).say(&site.read, "", "");
        assert!(
            !empty.contains("The store is complete"),
            "an empty universe is never complete: {empty}"
        );
        assert!(empty.contains("NOTHING IS TRACKED"), "{empty}");
        assert!(
            empty.contains("UNAVAILABLE"),
            "and it carries the read's own reason: {empty}"
        );
    }

    /// **The same claim, through a whole round, which is where it was
    /// published.**
    ///
    /// With the masters absent, `tracked_series` is empty, `next_window` owes
    /// nothing for any feed, `survey` chooses nothing, no feed is halted, and
    /// the no-work branch published *"nothing is missing … The store is
    /// complete through the newest finished day"* at phase `idle`, once a
    /// minute, over a store holding nothing. Every step was correct and the
    /// sentence was a lie.
    ///
    /// Nothing is fetched and nothing is contacted: `empty_site` is
    /// `Site::load`, so the broker is `Refused`, and no month is ever chosen.
    #[tokio::test]
    async fn an_empty_universe_is_published_as_halted_and_never_as_complete() {
        let site = empty_site("no-universe");
        let yesterday = yesterday_ist(std::time::SystemTime::now()).expect("a usable clock");
        let mut feeds = drivable(yesterday);

        let waited = round(&site, &mut feeds, &[], pull::vendor::Granularity::Minute1).await;
        assert_eq!(waited, IDLE_POLL_SECS, "nothing to carry on to");

        let (phase, detail) = site
            .autopilot
            .inspect(|status| (status.phase, status.detail.clone()))
            .expect("the status lock");
        assert_eq!(
            phase,
            Phase::Halted,
            "an empty universe is halted, not idle: {detail}"
        );
        assert!(
            !detail.contains("The store is complete"),
            "this is the sentence that was published over an empty store: {detail}"
        );
        assert!(
            detail.contains("NOT COMPLETE") && detail.contains("NOTHING IS TRACKED"),
            "{detail}"
        );
        assert!(
            detail.contains("masters"),
            "and it names what to fix: {detail}"
        );
        assert!(
            detail.contains("UNAVAILABLE"),
            "in the read's own words, which name the file: {detail}"
        );
    }

    /// **A whole round reconsiders a stalled month end to end**, from the idle
    /// branch and only from it, and asks for it by moving the frontier back.
    ///
    /// The empty series axis is what puts the round in its idle branch without
    /// a complete store: `next_window` over no series owes nothing, so `survey`
    /// chooses nothing and no feed is halted. Nothing is fetched and nothing is
    /// contacted.
    #[tokio::test]
    async fn a_round_with_nothing_missing_reconsiders_a_stalled_month_and_says_which_attempt() {
        let site = empty_site("stall-reconsider");
        let yesterday = yesterday_ist(std::time::SystemTime::now()).expect("a usable clock");
        let mut feeds = drivable(yesterday);
        let now = ingest::epoch_secs(std::time::SystemTime::now());
        let feed = feeds.first_mut().expect("a drivable feed");
        let frontier_before = feed.frontier;
        feed.stalls.push(Stall {
            month: month(2021, 3),
            attempts: MAX_MONTH_ATTEMPTS,
            reason: String::from("attempted 3 times and never completed: connection reset"),
            retried: 0,
            at_unix: now.saturating_sub(STALL_RECHECK_SECS * 2),
        });

        let waited = round(&site, &mut feeds, &[], pull::vendor::Granularity::Minute1).await;
        assert_eq!(
            waited, 0,
            "a reconsidered month is work, so the next pass is immediate rather than idle"
        );
        let feed = feeds.first().expect("a drivable feed");
        assert_eq!(feed.frontier, month(2021, 3), "the ladder went back to it");
        assert_ne!(feed.frontier, frontier_before);
        assert_eq!(feed.stalls[0].retried, 1);
        let json = site.autopilot.json();
        assert!(json.contains("being reconsidered"), "{json}");
        assert!(json.contains("attempt 1 of 2"), "{json}");
        assert!(
            json.contains("connection reset"),
            "the original reason survives verbatim: {json}"
        );
        // THE BOUND IS ON THE PAGE, not in the source only.
        assert!(json.contains(r#""retried":1"#), "{json}");
        assert!(
            json.contains(&format!(r#""retries_max":{STALL_RETRIES}"#)),
            "{json}"
        );

        // AND A SECOND ROUND DOES NOT ASK AGAIN, because the stall was stamped.
        feeds.first_mut().expect("a feed").frontier = frontier_before;
        let again = round(&site, &mut feeds, &[], pull::vendor::Granularity::Minute1).await;
        assert_eq!(
            again, IDLE_POLL_SECS,
            "the same month must not be reconsidered twice in one interval"
        );
        assert_eq!(feeds.first().expect("a feed").stalls[0].retried, 1);
    }

    // ------------------------------------------------------------ the clock

    /// **An unusable clock is waited on, bounded, and then given up on
    /// loudly.**
    ///
    /// This used to be a `return`: a machine that booted before NTP corrected
    /// its clock never started a backfill for the life of the process, and
    /// `admit_resume` then refused every resume for ever because the task had
    /// gone. Reverting [`clock_wait`] to answer `None` at zero puts that back.
    #[test]
    fn the_clock_is_waited_on_a_bounded_number_of_times_and_then_named() {
        for waits in 0..CLOCK_WAITS {
            let saying = clock_wait(waits).unwrap_or_else(|| panic!("check {waits} was refused"));
            assert!(
                saying.contains(&format!("check {} of {CLOCK_WAITS}", waits + 1)),
                "every wait says which one it is: {saying}"
            );
            assert!(
                saying.contains("Nothing is being contacted"),
                "and that it is not asking for anything meanwhile: {saying}"
            );
        }
        for spent in [CLOCK_WAITS, CLOCK_WAITS + 1, u32::MAX] {
            assert_eq!(
                clock_wait(spent),
                None,
                "the allowance is spent and waiting longer does not renew it"
            );
        }
    }

    /// Every halt class carries its own word, and no two share one — the words
    /// travel into sentences an operator reads.
    #[test]
    fn every_halt_class_names_itself_and_only_the_store_class_arms_a_probe() {
        let all = [Halt::Credential, Halt::Store, Halt::Census];
        let mut words: Vec<&str> = all.iter().map(|h| h.word()).collect();
        words.sort_unstable();
        let before = words.len();
        words.dedup();
        assert_eq!(before, words.len(), "two halt classes share a word");

        for kind in all {
            let mut state = FeedState::new(
                pull::vendor::Feed::Groww,
                brutex_core::vendor::Vendor::Groww,
                month(2020, 1),
            );
            state.halt(kind, format!("halted on the {} class", kind.word()));
            assert_eq!(state.halt_kind, Some(kind));
            assert_eq!(
                state.probe.is_some(),
                kind == Halt::Store,
                "{} armed the wrong thing: only a disk can be measured locally, and \
                 CLAUDE.md §8 forbids re-trying a credential at all",
                kind.word()
            );
            state.revive();
            assert!(state.halted.is_none() && state.halt_kind.is_none());
            assert!(state.probe.is_none());
            assert_eq!(
                state.rereads, CREDENTIAL_REREADS,
                "a revived feed is owed its one §8 re-read again"
            );
        }
    }
}
