//! `/calendar.json` answers built off the request path, and the kept answers a
//! request is handed without reading the census (WP-36, DB-N19).
//!
//! # What was measured, and why this exists
//!
//! The first `/calendar.json?feed=gdfl&symbol=NIFTY` after a server start took
//! 6.55 s on a shadow of the operator's store (feat/db-fix 3cd0623c, 10 Oct
//! 2026; 5.11 s and 3.28 s on this package's own shadow): the census read and
//! the month counts of 194 one-second index months, 142 of them without a
//! `.tix` and so bisected, were all paid by the one click that asked first.
//! Zerodha's first answer took 0.11-0.47 s. Every later request took about a
//! millisecond. A route whose maximum is a whole build is not O(1) at its
//! maximum (DB-N19).
//!
//! # What a request costs now
//!
//! * **A confirmed answer** costs the asked feed's census stamp (the five
//!   manifest `stat` calls `manifest_stamps` makes, no read), one lock, one
//!   probe and the copy of a body of about 19 KB. No census is read and no
//!   bar file is opened. An answer is confirmed under the asked feed's stamp
//!   whenever the route keeps it or finds it kept, so it is handed out only
//!   while that feed's manifest is the one it was read from (D-0686, D-0695).
//! * **Anything else** joins or leads a build. A build runs on the calendar
//!   pool behind its admission (`crate::detail::run_calendar`, D-1443), and is
//!   exactly the reading the route always did. A request waits for it at most
//!   [`Builds::budget`], and not at all for a start-up build still queued
//!   behind others; a build not done by then is answered `202` with
//!   `"status": "building"`, what it is doing, how far it has got and when to
//!   ask again ([`RETRY_AFTER_MS`]). The build goes on, and its answer is kept
//!   for whoever asks next.
//! * **At start**, the served process reads the census and builds the answers
//!   the pages ask first, in the background (`calendar_route::warm_at_start`),
//!   so the first click after a restart usually finds its answer kept.
//!
//! Without a budget -- every caller but the served process, the tests among
//! them -- a request waits for its build, so it is answered byte for byte as
//! it was before this module existed.
//!
//! # Bounded
//!
//! At most [`MAX_BUILDS`] builds are in the map, running, queued or finished
//! and waiting for the request that was told to come back; a finished one is
//! evicted to make room, and a full map of running builds is refused by name
//! (`429`). A finished answer is handed to a later request only within
//! [`PARKED_FOR`] and only while the feed's stamp is the one the build began
//! under, so a refusal is never served over a census that has moved.
//!
//! UNVERIFIED: this cost names no test or bench that proves it (docs/06-limits.md,
//! "Cost claims of the /db fix round that name no proof").

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock, PoisonError};
use std::time::{Duration, Instant};

use brutex_core::vendor::Vendor;

use super::{AnswerKey, KeptAnswer};
use crate::server::CensusStamps;

/// How long the served process lets a request wait for a build before it is
/// answered `"building"`; a start-up build still queued behind others is not
/// waited for at all.
///
/// A fifth of the 50 ms the operator's first click is held to (WP-36's
/// verification): measured on a shadow of the operator's store under the
/// start's census read and journal walk, a 25 ms wait was answered in 32 to
/// 57 ms, the rest being the machine's own load. A build of a key whose month
/// counts are already kept takes about 1 to 20 ms, so most of those are still
/// answered in full.
pub const REQUEST_WAIT: Duration = Duration::from_millis(10);

/// When a `"building"` answer tells its reader to ask again, in milliseconds.
pub const RETRY_AFTER_MS: u64 = 250;

/// How long a finished build's answer is kept for a request that was told to
/// come back. A request after that leads a build of its own.
pub const PARKED_FOR: Duration = Duration::from_mins(1);

/// The most builds the map holds at once: running, queued, or finished and
/// parked. The start-up queue is about forty keys on the operator's store.
pub const MAX_BUILDS: usize = 256;

/// Who asked for a build.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Origin {
    /// The served process, at start (`calendar_route::warm_at_start`).
    StartUp,
    /// A request that found no answer.
    Request,
}

impl Origin {
    /// The word on the wire.
    #[must_use]
    pub const fn word(self) -> &'static str {
        match self {
            Self::StartUp => "start-up",
            Self::Request => "request",
        }
    }
}

/// Where one [`Build`] has got to, under its lock.
#[derive(Debug)]
struct State<A> {
    /// When it finished, and its answer.
    done: Option<(Instant, A)>,
    /// A request waiting on it ran out of budget and was told to ask again,
    /// so the answer is parked for the next request rather than dropped.
    told: bool,
}

/// How far a reading has got: what it has to count, and what it has counted.
#[derive(Debug, Default)]
struct Counters {
    /// Files (or series) the reading has to count.
    owed: AtomicU64,
    /// Counted so far, from memory or from disk.
    counted: AtomicU64,
}

/// One answer being built: a `/calendar.json` answer ([`KeptAnswer`]), or a
/// peer vote for `/gaps.json` ([`Voted`]).
#[derive(Debug)]
pub struct Build<A = KeptAnswer> {
    /// Who asked for it.
    origin: Origin,
    /// When it was asked for.
    queued: Instant,
    /// When its reading began, and the census stamp at that moment.
    began: Mutex<Option<(Instant, Option<CensusStamps>)>>,
    /// How far the reading has got.
    counters: Arc<Counters>,
    /// Its answer, once it has one.
    state: Mutex<State<A>>,
    /// Woken when the answer is set.
    ready: tokio::sync::Notify,
}

/// How far one [`Build`] has got, as a `"building"` answer says it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Progress {
    /// Who asked for it.
    pub origin: Origin,
    /// How long ago it was asked for.
    pub queued_for: Duration,
    /// How long it has been reading, or `None` while it waits its turn.
    pub reading_for: Option<Duration>,
    /// Month files its reading has to count, as far as it knows yet.
    pub owed: u64,
    /// Month files counted so far.
    pub counted: u64,
}

impl<A: Clone> Build<A> {
    fn new(origin: Origin) -> Self {
        Self {
            origin,
            queued: Instant::now(),
            began: Mutex::new(None),
            counters: Arc::new(Counters::default()),
            state: Mutex::new(State {
                done: None,
                told: false,
            }),
            ready: tokio::sync::Notify::new(),
        }
    }

    /// How far it has got.
    #[must_use]
    pub fn progress(&self) -> Progress {
        let began = self
            .began
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .as_ref()
            .map(|(at, _)| at.elapsed());
        Progress {
            origin: self.origin,
            queued_for: self.queued.elapsed(),
            reading_for: began,
            owed: self.counters.owed.load(Ordering::Acquire),
            counted: self.counters.counted.load(Ordering::Acquire),
        }
    }

    /// Its answer, if it has one.
    #[must_use]
    pub fn answer(&self) -> Option<A> {
        self.state
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .done
            .as_ref()
            .map(|(_, answer)| answer.clone())
    }

    /// Mark the reading begun on this thread, under the asked feed's stamp
    /// `stamps`, until the guard drops: the month counts the reading makes on
    /// this thread are counted against this build ([`owe`], [`counted`]).
    #[must_use]
    pub fn enter(self: &Arc<Self>, stamps: Option<CensusStamps>) -> Entered {
        *self.began.lock().unwrap_or_else(PoisonError::into_inner) = Some((Instant::now(), stamps));
        CURRENT.with(|current| *current.borrow_mut() = Some(Arc::clone(&self.counters)));
        Entered(())
    }

    /// Wait for the answer, at most `budget`, or until there is one when
    /// `budget` is `None`.
    ///
    /// `None` when the budget ran out first. The build is then marked as
    /// owing its answer to a later request, under the same lock its answer is
    /// set under, so an answer set at that very moment is handed back here
    /// rather than lost between the two.
    pub async fn wait(&self, budget: Option<Duration>) -> Option<A> {
        let deadline = budget.map(|budget| tokio::time::Instant::now() + budget);
        loop {
            let notified = self.ready.notified();
            tokio::pin!(notified);
            // ENABLED BEFORE THE ANSWER IS LOOKED AT, so an answer set
            // between the look and the wait wakes this waiter.
            notified.as_mut().enable();
            if let Some(answer) = self.answer() {
                return Some(answer);
            }
            match deadline {
                None => notified.await,
                Some(at) => {
                    if tokio::time::timeout_at(at, notified).await.is_err() {
                        let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
                        if let Some((_, answer)) = state.done.as_ref() {
                            return Some(answer.clone());
                        }
                        state.told = true;
                        return None;
                    }
                }
            }
        }
    }

    /// Whether this finished build's answer may be handed to a request whose
    /// asked feed is stamped `stamps` now: finished within [`PARKED_FOR`],
    /// under exactly that stamp.
    fn deliverable(&self, stamps: Option<&CensusStamps>) -> Option<A> {
        let state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        let (at, answer) = state.done.as_ref()?;
        let began = self.began.lock().unwrap_or_else(PoisonError::into_inner);
        let same_stamp = began
            .as_ref()
            .is_some_and(|(_, under)| under.as_ref() == stamps && under.is_some());
        (at.elapsed() <= PARKED_FOR && same_stamp).then(|| answer.clone())
    }

    /// Whether it has an answer.
    fn finished(&self) -> bool {
        self.state
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .done
            .is_some()
    }
}

/// What one start-up warm did, as its one log line says it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Warmed {
    /// The feeds warmed.
    pub feeds: Vec<Vendor>,
    /// Milliseconds the census read took.
    pub census_ms: u64,
    /// Answers built.
    pub built: u64,
    /// Of them, answers that were not `200`: a refusal is an answer too, and
    /// the next request is handed it or builds again.
    pub refused: u64,
    /// Answers that could not be queued: the build map was full.
    pub unqueued: u64,
    /// Milliseconds the answers took, all of them.
    pub building_ms: u64,
    /// The pull journal's walk: records read and milliseconds, or why not.
    pub journal: Result<(u64, u64), String>,
    /// Milliseconds the whole warm took.
    pub total_ms: u64,
}

impl Warmed {
    /// Its one line in the log: `api.calendar warmed`.
    #[must_use]
    pub fn note(&self) -> telemetry::Emitted {
        let feeds = self
            .feeds
            .iter()
            .map(|feed| feed.as_str())
            .collect::<Vec<_>>()
            .join(",");
        let (records, journal_ms) = self.journal.as_ref().map_or((0, 0), |walk| *walk);
        let journal_error = self.journal.as_ref().err().map_or("", String::as_str);
        telemetry::emit_if!(
            telemetry::Level::Info,
            "api.calendar",
            "warmed",
            "feeds" => telemetry::Value::Str(&feeds),
            "census_ms" => telemetry::Value::Uint(self.census_ms),
            "built" => telemetry::Value::Uint(self.built),
            "refused" => telemetry::Value::Uint(self.refused),
            "unqueued" => telemetry::Value::Uint(self.unqueued),
            "building_ms" => telemetry::Value::Uint(self.building_ms),
            "journal_records" => telemetry::Value::Uint(records),
            "journal_ms" => telemetry::Value::Uint(journal_ms),
            "journal_error" => telemetry::Value::Str(journal_error),
            "total_ms" => telemetry::Value::Uint(self.total_ms),
        )
    }
}

thread_local! {
    /// The build whose reading runs on this thread, if any: see
    /// [`Build::enter`].
    static CURRENT: std::cell::RefCell<Option<Arc<Counters>>> = const { std::cell::RefCell::new(None) };
}

/// The guard [`Build::enter`] hands back. Dropping it ends the build's claim
/// on this thread; the build itself is finished by [`Builds::finish`].
#[derive(Debug)]
pub struct Entered(());

impl Drop for Entered {
    fn drop(&mut self) {
        CURRENT.with(|current| *current.borrow_mut() = None);
    }
}

/// The reading on this thread has `months` more month files to count. A
/// thread no build has entered counts nothing.
pub fn owe(months: usize) {
    CURRENT.with(|current| {
        if let Some(counters) = current.borrow().as_ref() {
            counters
                .owed
                .fetch_add(u64::try_from(months).unwrap_or(u64::MAX), Ordering::AcqRel);
        }
    });
}

/// The reading on this thread has counted one more month file.
pub fn counted() {
    CURRENT.with(|current| {
        if let Some(counters) = current.borrow().as_ref() {
            counters.counted.fetch_add(1, Ordering::AcqRel);
        }
    });
}

/// What [`Builds::join`] found for a request with no confirmed answer.
#[derive(Debug)]
pub enum Joined<A = KeptAnswer> {
    /// No build for the key: the request leads this one, and must run it.
    Lead(Arc<Build<A>>),
    /// A build for the key is queued or running: wait on it.
    Follow(Arc<Build<A>>),
    /// A finished build's answer, parked for this request.
    Parked(A),
    /// The served process is reading its census at start, and no build for
    /// this key is queued: how long it has been reading. A request does not
    /// read the census beside it.
    CensusAtStart(Duration),
    /// [`MAX_BUILDS`] builds are queued or running.
    Full,
}

/// One build per key: the `/calendar.json` answers' and the peer votes'.
#[derive(Debug)]
pub struct Flights<K, A> {
    /// One build per key, running, queued, or finished and parked.
    map: Mutex<HashMap<K, Arc<Build<A>>>>,
}

impl<K, A> Default for Flights<K, A> {
    fn default() -> Self {
        Self {
            map: Mutex::new(HashMap::new()),
        }
    }
}

impl<K: std::hash::Hash + Eq + Clone, A: Clone> Flights<K, A> {
    /// What a request for `key` finds, its census stamped `stamps` now
    /// (`None` when no stamp could be taken), while the served process has
    /// been reading its census at start for `census_at_start`: see
    /// [`Joined`].
    fn join(
        &self,
        key: &K,
        stamps: Option<&CensusStamps>,
        census_at_start: Option<Duration>,
    ) -> Joined<A> {
        let mut flights = self.map.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(build) = flights.get(key) {
            if !build.finished() {
                return Joined::Follow(Arc::clone(build));
            }
            let parked = build.deliverable(stamps);
            flights.remove(key);
            if let Some(answer) = parked {
                return Joined::Parked(answer);
            }
        }
        if let Some(since) = census_at_start {
            return Joined::CensusAtStart(since);
        }
        if !room(&mut flights) {
            return Joined::Full;
        }
        let build = Arc::new(Build::new(Origin::Request));
        flights.insert(key.clone(), Arc::clone(&build));
        Joined::Lead(build)
    }

    /// Queue a start-up build for `key`, or hand back the one already there.
    /// `None` when [`MAX_BUILDS`] are queued or running.
    fn queue_at_start(&self, key: &K) -> Option<Arc<Build<A>>> {
        let mut flights = self.map.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(build) = flights.get(key)
            && !build.finished()
        {
            return Some(Arc::clone(build));
        }
        flights.remove(key);
        if !room(&mut flights) {
            return None;
        }
        let build = Arc::new(Build::new(Origin::StartUp));
        flights.insert(key.clone(), Arc::clone(&build));
        Some(build)
    }

    /// Set `build`'s answer and wake everyone waiting on it. The build stays
    /// in the map, parked for the next request, only when a request waiting
    /// on it was told to come back.
    pub fn finish(&self, key: &K, build: &Arc<Build<A>>, answer: A) {
        let told = {
            let mut state = build.state.lock().unwrap_or_else(PoisonError::into_inner);
            state.done = Some((Instant::now(), answer));
            state.told
        };
        build.ready.notify_waiters();
        if !told {
            self.drop_build(key, build);
        }
    }

    /// Set `build`'s answer, wake everyone waiting on it, and take it out of
    /// the map whatever anyone was told: a build that stopped before it was
    /// built. The next request leads a build of its own.
    pub fn abandon(&self, key: &K, build: &Arc<Build<A>>, answer: A) {
        build
            .state
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .done = Some((Instant::now(), answer));
        build.ready.notify_waiters();
        self.drop_build(key, build);
    }

    /// Take `build` out of the map, when it is still the one there.
    fn drop_build(&self, key: &K, build: &Arc<Build<A>>) {
        let mut flights = self.map.lock().unwrap_or_else(PoisonError::into_inner);
        if flights
            .get(key)
            .is_some_and(|held| Arc::ptr_eq(held, build))
        {
            flights.remove(key);
        }
    }

    /// How many builds the map holds: for a test.
    #[cfg(test)]
    fn len(&self) -> usize {
        self.map
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .len()
    }
}

/// Make room for one more build: evict finished builds when the map is full.
/// `false` when every build in it is queued or running.
fn room<K, A: Clone>(flights: &mut HashMap<K, Arc<Build<A>>>) -> bool {
    if flights.len() < MAX_BUILDS {
        return true;
    }
    flights.retain(|_, build| !build.finished());
    flights.len() < MAX_BUILDS
}

/// The builds in progress, the confirmed answers, the peer votes, and the
/// served budget.
#[derive(Debug, Default)]
pub struct Builds {
    /// One `/calendar.json` build per answer key.
    flights: Flights<AnswerKey, KeptAnswer>,
    /// Kept answers, each under the asked feed's stamp it was kept or found
    /// kept under: see [`Self::confirmed`].
    confirmed: Mutex<HashMap<AnswerKey, (CensusStamps, Arc<KeptAnswer>)>>,
    /// `/gaps.json`'s peer votes: see [`Votes`].
    pub votes: Votes,
    /// How long a request waits for a build; unset, it waits for the answer.
    budget: OnceLock<Duration>,
    /// When the served process began reading its census at start, while it
    /// reads.
    census_at_start: Mutex<Option<Instant>>,
}

impl Builds {
    /// How long a request waits for a build before it is answered
    /// `"building"`: [`REQUEST_WAIT`] in the served process, and `None` --
    /// until the answer -- everywhere else.
    #[must_use]
    pub fn budget(&self) -> Option<Duration> {
        self.budget.get().copied()
    }

    /// Bound every request's wait to `budget` from now on. Set once, by the
    /// served process; a second call keeps the first budget.
    pub fn bound_waits(&self, budget: Duration) {
        let _first_wins = self.budget.set(budget);
    }

    /// How long the served process has been reading its census at start, or
    /// `None` when it is not reading it.
    #[must_use]
    pub fn census_at_start(&self) -> Option<Duration> {
        self.census_at_start
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .map(|since| since.elapsed())
    }

    /// The kept answer for `key`, when it was confirmed under exactly
    /// `stamps`, the asked feed's stamp now. One lock and one probe.
    #[must_use]
    pub fn confirmed(&self, key: &AnswerKey, stamps: &CensusStamps) -> Option<Arc<KeptAnswer>> {
        self.confirmed
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get(key)
            .filter(|(under, _)| under == stamps)
            .map(|(_, kept)| Arc::clone(kept))
    }

    /// Confirm `kept`, the answer kept for `key`, under `stamps`: the asked
    /// feed's stamp the census it was kept or found under was read at.
    ///
    /// Bounded as the kept answers are: the route confirms only an answer it
    /// keeps, and keeps only one whose key the census bounds (D-0695).
    pub fn confirm(&self, key: AnswerKey, stamps: CensusStamps, kept: Arc<KeptAnswer>) {
        self.confirmed
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(key, (stamps, kept));
    }

    /// Forget every confirmed answer: for a test that wants a cold request.
    #[cfg(test)]
    pub(crate) fn forget_confirmed(&self) {
        self.confirmed
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clear();
    }

    /// How many confirmed answers there are: for a test.
    #[cfg(test)]
    pub(crate) fn confirmed_answers(&self) -> usize {
        self.confirmed
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .len()
    }

    /// How many `/calendar.json` builds the map holds: for a test.
    #[cfg(test)]
    pub(crate) fn in_flight(&self) -> usize {
        self.flights.len()
    }

    /// What a request for `key`, whose asked feed is stamped `stamps` now
    /// (`None` when no stamp could be taken), finds: see [`Joined`].
    #[must_use]
    pub fn join(&self, key: &AnswerKey, stamps: Option<&CensusStamps>) -> Joined {
        self.flights.join(key, stamps, self.census_at_start())
    }

    /// Queue a start-up build for `key`, or hand back the one already there.
    /// `None` when [`MAX_BUILDS`] are queued or running.
    #[must_use]
    pub fn queue_at_start(&self, key: &AnswerKey) -> Option<Arc<Build>> {
        self.flights.queue_at_start(key)
    }

    /// See [`Flights::finish`].
    pub fn finish(&self, key: &AnswerKey, build: &Arc<Build>, answer: KeptAnswer) {
        self.flights.finish(key, build, answer);
    }

    /// See [`Flights::abandon`].
    pub fn abandon(&self, key: &AnswerKey, build: &Arc<Build>, answer: KeptAnswer) {
        self.flights.abandon(key, build, answer);
    }

    /// Mark the served process's census read at start, until the guard drops.
    #[must_use]
    pub fn reading_census_at_start(&self) -> CensusAtStart<'_> {
        *self
            .census_at_start
            .lock()
            .unwrap_or_else(PoisonError::into_inner) = Some(Instant::now());
        CensusAtStart(self)
    }
}

/// The guard [`Builds::reading_census_at_start`] hands back: the census read
/// at start is over when it drops, read or not.
#[derive(Debug)]
pub struct CensusAtStart<'a>(&'a Builds);

impl Drop for CensusAtStart<'_> {
    fn drop(&mut self) {
        *self
            .0
            .census_at_start
            .lock()
            .unwrap_or_else(PoisonError::into_inner) = None;
    }
}

// ══ /gaps.json's peer vote (calendar=peers) ═════════════════════════════════

/// A calendar for auditing one series, built from everything EXCEPT that
/// series.
///
/// # Why the audited series may not vote
///
/// `calendar_of` reads a session's windows off contiguous runs of minute bars,
/// so a series with a hole derives a calendar in which that hole is a window
/// boundary. Auditing a series against its own derivation therefore answers
/// "no losses" for every possible input — measured, in one test run: one minute
/// deleted mid-session produced `expected 8,249, held 8,249, lost 0` with the
/// missing minute reported as `outside-window`.
///
/// `agree` is a UNION — a day is a session if ANY reading saw one, and the
/// session taken is the LONGEST any reading measured — so a peer's own hole
/// cannot shorten the day for everybody, and one honest peer is enough to
/// establish what the exchange did.
///
/// # When there are no peers
///
/// A store holding one feed's copy of one symbol has nothing independent to
/// compare against. That returns `None`, `gaps::classify_against` falls back to
/// the typed table, and `source` says `table` so the answer names what decided
/// it. Substituting the series' own derivation there would be the §4 fallback
/// that hides a failure: it would answer, confidently, zero.
#[derive(Debug, Clone)]
pub struct PeerCalendar {
    /// The agreed calendar, or `None` when nothing independent could vote.
    pub calendar: Option<pull::calendar::Calendar>,
    /// The `feed:symbol` readings that voted, sorted, for the answer to name.
    pub from: Vec<String>,
    /// Every feed whose census could not be read, in `Vendor::ALL` order.
    ///
    /// Such a census contributes no series, so without this list a damaged
    /// counter fell back to the table with `voted_by: []` -- the same answer
    /// as a store with no peers at all, which is the failure `CLAUDE.md` §4
    /// bans. A peer that might have voted and could not be read is named.
    ///
    /// Then, as `feed:symbol` in the order they were walked, every peer series
    /// with a bar file its census holds and its derivation could not open. Its
    /// calendar lacks what that file would have proved, so it does not vote,
    /// and `calendar_of::cached` does not keep it. D-0695.
    pub unreadable: Vec<String>,
}

/// Which peer vote: the audited series' feed, exchange, segment and symbol,
/// the only parts of its address the vote reads.
pub type VoteKey = (Vendor, String, String, String);

/// What a peer-vote build answers: the vote, or the status and body of the
/// refusal its admission met (`crate::detail::run_calendar`, D-1443).
pub type Voted = Result<Arc<PeerCalendar>, (u16, String)>;

/// The most peer votes kept at once. A vote is kept under its audited
/// series' address, which a request names, so the map is bounded here rather
/// than by the census: past it a vote is answered and not kept.
pub const MAX_KEPT_VOTES: usize = 1_024;

/// `/gaps.json`'s peer votes: kept under the census stamps they were read
/// under, and built once per series off the request path (DB-N19).
///
/// The first `calendar=peers` audit after a start derived every peer series'
/// calendar (343 on the operator's Zerodha store): 17.9 s on WP-36's shadow.
/// Every later one walked and sorted every vendor's census entries and agreed
/// every peer again: about 0.26-0.40 s. A vote kept under the stamps of every
/// vendor's manifest -- the vote reads every feed's census -- is now handed
/// out with one `stat` per vendor and one probe.
#[derive(Debug, Default)]
pub struct Votes {
    /// One vote build per audited series.
    pub flights: Flights<VoteKey, Voted>,
    /// Votes, each under the census stamps it was read under.
    kept: Mutex<HashMap<VoteKey, (CensusStamps, Arc<PeerCalendar>)>>,
}

impl Votes {
    /// The vote kept for `key` under exactly `stamps`: one lock, one probe.
    #[must_use]
    pub fn kept(&self, key: &VoteKey, stamps: &CensusStamps) -> Option<Arc<PeerCalendar>> {
        self.kept
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get(key)
            .filter(|(under, _)| under == stamps)
            .map(|(_, vote)| Arc::clone(vote))
    }

    /// Keep `vote` for `key` under `stamps`, replacing one kept under older
    /// stamps. Not kept once [`MAX_KEPT_VOTES`] other votes are: `false`.
    pub fn keep(&self, key: VoteKey, stamps: CensusStamps, vote: Arc<PeerCalendar>) -> bool {
        let mut kept = self.kept.lock().unwrap_or_else(PoisonError::into_inner);
        if kept.len() >= MAX_KEPT_VOTES && !kept.contains_key(&key) {
            return false;
        }
        kept.insert(key, (stamps, vote));
        true
    }

    /// What a request for `key`, the census stamped `stamps` now, finds.
    #[must_use]
    pub fn join(
        &self,
        key: &VoteKey,
        stamps: Option<&CensusStamps>,
        census_at_start: Option<Duration>,
    ) -> Joined<Voted> {
        self.flights.join(key, stamps, census_at_start)
    }
}

#[cfg(test)]
#[expect(
    clippy::expect_used,
    clippy::panic,
    reason = "test fixtures and exact assertions"
)]
mod tests {
    use super::*;

    fn key(symbol: &str) -> AnswerKey {
        (Vendor::Gdfl, None, symbol.to_owned())
    }

    fn answer(body: &str) -> KeptAnswer {
        KeptAnswer {
            status: 200,
            body: body.to_owned(),
        }
    }

    /// Two stamps that differ, of one store root: before and after a GDFL
    /// manifest appears in it.
    fn two_stamps(name: &str) -> (CensusStamps, CensusStamps) {
        let root = crate::scratch::path(name);
        let _ = std::fs::remove_dir_all(&root);
        let masters = root.join("masters");
        std::fs::create_dir_all(&masters).expect("empty offline masters");
        std::fs::create_dir_all(root.join("manifest")).expect("a manifest directory");
        let site = crate::server::Site::load(&masters, &root);
        let empty = crate::server::census_now_stamped(&site).0;
        std::fs::write(root.join("manifest").join("gdfl.man"), b"x").expect("a manifest");
        let held = crate::server::census_now_stamped(&site).0;
        let _ = std::fs::remove_dir_all(&root);
        assert_ne!(empty, held, "a manifest that appeared moves the stamp");
        (empty, held)
    }

    /// **A CONFIRMED ANSWER IS HANDED OUT ONLY UNDER THE STAMP IT WAS
    /// CONFIRMED UNDER.** DB-N19.
    #[test]
    fn a_confirmed_answer_is_found_only_under_its_own_stamp() {
        let (before, after) = two_stamps("calendar-build-confirmed");
        let builds = Builds::default();
        assert!(builds.confirmed(&key("NIFTY"), &before).is_none());
        builds.confirm(key("NIFTY"), before.clone(), Arc::new(answer("kept")));
        assert_eq!(
            builds
                .confirmed(&key("NIFTY"), &before)
                .map(|kept| kept.body.clone()),
            Some("kept".to_owned())
        );
        assert!(
            builds.confirmed(&key("NIFTY"), &after).is_none(),
            "a moved stamp is not the one it was confirmed under"
        );
        assert!(
            builds.confirmed(&key("BANKNIFTY"), &before).is_none(),
            "another key is another answer"
        );
        assert_eq!(builds.confirmed_answers(), 1);
        builds.forget_confirmed();
        assert_eq!(builds.confirmed_answers(), 0);
    }

    /// **ONE BUILD PER KEY: THE FIRST REQUEST LEADS, THE NEXT FOLLOWS, AND A
    /// BUILD NOBODY WAS TOLD TO COME BACK FOR LEAVES THE MAP WHEN IT
    /// FINISHES.** DB-N19.
    #[tokio::test]
    async fn the_first_request_leads_and_the_next_follows_one_build() {
        let builds = Builds::default();
        let Joined::Lead(lead) = builds.join(&key("NIFTY"), None) else {
            panic!("the first request leads");
        };
        let Joined::Follow(follow) = builds.join(&key("NIFTY"), None) else {
            panic!("the second follows");
        };
        assert!(Arc::ptr_eq(&lead, &follow), "the same build");
        assert!(
            matches!(builds.join(&key("BANKNIFTY"), None), Joined::Lead(_)),
            "another key leads its own"
        );
        let waiting = {
            let follow = Arc::clone(&follow);
            tokio::spawn(async move { follow.wait(None).await })
        };
        builds.finish(&key("NIFTY"), &lead, answer("built"));
        assert_eq!(
            waiting.await.expect("the waiter ends"),
            Some(answer("built")),
            "an unbounded wait is answered what the build answered"
        );
        assert_eq!(builds.in_flight(), 1, "only BANKNIFTY's build is left");
        assert!(
            matches!(builds.join(&key("NIFTY"), None), Joined::Lead(_)),
            "a finished build nobody was told to come back for is gone"
        );
    }

    /// **A REQUEST THAT RUNS OUT OF BUDGET IS TOLD TO COME BACK, AND THE NEXT
    /// REQUEST UNDER THE SAME STAMP IS HANDED THE ANSWER, ONCE.** DB-N19.
    #[tokio::test]
    async fn a_request_told_to_come_back_is_handed_the_parked_answer_once() {
        let (stamp, moved) = two_stamps("calendar-build-parked");
        let builds = Builds::default();
        let Joined::Lead(lead) = builds.join(&key("NIFTY"), Some(&stamp)) else {
            panic!("the first request leads");
        };
        let _entered = lead.enter(Some(stamp.clone()));
        let started = Instant::now();
        assert_eq!(
            lead.wait(Some(Duration::from_millis(20))).await,
            None,
            "nothing answered within the budget"
        );
        assert!(
            started.elapsed() < Duration::from_secs(5),
            "the wait is bounded by its budget"
        );
        builds.finish(&key("NIFTY"), &lead, answer("built"));
        assert_eq!(
            builds.in_flight(),
            1,
            "parked for the request told to return"
        );
        assert!(
            matches!(builds.join(&key("NIFTY"), Some(&moved)), Joined::Lead(_)),
            "a moved stamp is not handed an answer read under the old one"
        );

        let Joined::Lead(second) = builds.join(&key("BANKNIFTY"), Some(&stamp)) else {
            panic!("leads");
        };
        let _again = second.enter(Some(stamp.clone()));
        assert_eq!(second.wait(Some(Duration::from_millis(1))).await, None);
        builds.finish(&key("BANKNIFTY"), &second, answer("parked"));
        match builds.join(&key("BANKNIFTY"), Some(&stamp)) {
            Joined::Parked(parked) => assert_eq!(parked, answer("parked")),
            other => panic!("the parked answer is handed over: {other:?}"),
        }
        assert!(
            matches!(
                builds.join(&key("BANKNIFTY"), Some(&stamp)),
                Joined::Lead(_)
            ),
            "once"
        );
    }

    /// **AN ANSWER SET AS THE BUDGET RUNS OUT IS HANDED BACK, NOT LOST.**
    #[tokio::test]
    async fn an_answer_already_set_is_returned_without_waiting() {
        let builds = Builds::default();
        let Joined::Lead(lead) = builds.join(&key("NIFTY"), None) else {
            panic!("leads");
        };
        builds.finish(&key("NIFTY"), &lead, answer("done"));
        assert_eq!(
            lead.wait(Some(Duration::ZERO)).await,
            Some(answer("done")),
            "a zero budget still sees an answer already set"
        );
    }

    /// **A START-UP BUILD IS JOINED, NOT DUPLICATED, AND WHILE THE CENSUS IS
    /// READ AT START A REQUEST FOR ANY OTHER KEY READS NOTHING.** DB-N19.
    #[test]
    fn a_start_up_build_is_joined_and_the_census_at_start_is_not_read_twice() {
        let builds = Builds::default();
        let queued = builds
            .queue_at_start(&key("NIFTY"))
            .expect("room for one build");
        let again = builds.queue_at_start(&key("NIFTY")).expect("room");
        assert!(Arc::ptr_eq(&queued, &again), "queued once");
        assert_eq!(queued.progress().origin, Origin::StartUp);
        assert_eq!(queued.progress().reading_for, None, "waiting its turn");
        {
            let _reading = builds.reading_census_at_start();
            match builds.join(&key("NIFTY"), None) {
                Joined::Follow(build) => assert!(Arc::ptr_eq(&build, &queued)),
                other => panic!("a queued key is followed: {other:?}"),
            }
            assert!(
                matches!(
                    builds.join(&key("RELIANCE"), None),
                    Joined::CensusAtStart(_)
                ),
                "any other key waits for the census read at start"
            );
        }
        assert!(
            matches!(builds.join(&key("RELIANCE"), None), Joined::Lead(_)),
            "and leads once it is read"
        );
        builds.abandon(&key("NIFTY"), &queued, answer("stopped"));
        assert_eq!(queued.answer(), Some(answer("stopped")));
        assert!(
            matches!(builds.join(&key("NIFTY"), None), Joined::Lead(_)),
            "an abandoned build leaves the map whatever anyone was told"
        );
    }

    /// **THE MONTH COUNTS OF A READING ARE COUNTED AGAINST THE BUILD THAT
    /// ENTERED ITS THREAD, AND ONLY WHILE IT HAS.**
    #[test]
    fn progress_is_counted_against_the_build_on_this_thread() {
        let build = Arc::new(Build::<KeptAnswer>::new(Origin::Request));
        owe(3);
        counted();
        assert_eq!(
            build.progress().owed,
            0,
            "no build entered: nothing counted"
        );
        {
            let _entered = build.enter(None);
            owe(3);
            counted();
            counted();
            let progress = build.progress();
            assert_eq!((progress.owed, progress.counted), (3, 2));
            assert!(progress.reading_for.is_some());
        }
        counted();
        assert_eq!(build.progress().counted, 2, "the guard ended the claim");
    }

    /// **THE MAP IS BOUNDED: FINISHED BUILDS ARE EVICTED FOR ROOM, AND A MAP
    /// FULL OF RUNNING BUILDS REFUSES THE NEXT.**
    #[test]
    fn the_map_evicts_finished_builds_and_refuses_when_every_build_runs() {
        let builds = Builds::default();
        let mut running = Vec::new();
        for n in 0..MAX_BUILDS {
            match builds.join(&key(&format!("S{n}")), None) {
                Joined::Lead(build) => running.push(build),
                other => panic!("room for build {n}: {other:?}"),
            }
        }
        assert!(matches!(builds.join(&key("ONEMORE"), None), Joined::Full));
        assert!(builds.queue_at_start(&key("ONEMORE")).is_none());
        let first = running.first().expect("one build").clone();
        // TOLD, so it stays parked in the map after it finishes.
        first
            .state
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .told = true;
        builds.finish(&key("S0"), &first, answer("parked"));
        assert_eq!(builds.in_flight(), MAX_BUILDS, "parked, still in the map");
        assert!(
            matches!(builds.join(&key("ONEMORE"), None), Joined::Lead(_)),
            "the finished build is evicted to make room"
        );
        assert_eq!(builds.in_flight(), MAX_BUILDS);
    }
}
